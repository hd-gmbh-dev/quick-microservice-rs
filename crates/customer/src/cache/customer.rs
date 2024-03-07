use futures::stream::TryStreamExt;
use std::collections::BTreeMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::RwLock;

use qm_entity::ids::CustomerResourceId;
use qm_entity::ids::InstitutionId;
use qm_entity::ids::MemberId;
use qm_entity::ids::OrganizationId;
use qm_entity::ids::OrganizationResourceId;
use qm_entity::ids::OrganizationUnitId;
use qm_entity::ids::ID;
use qm_mongodb::bson::doc;
use qm_mongodb::bson::oid::ObjectId;
use qm_mongodb::bson::Uuid;
use qm_redis::redis::{AsyncCommands, Msg};

use crate::model::Customer;
use crate::model::Institution;
use crate::model::Organization;
use crate::model::OrganizationUnit;
use crate::schema::customer::CustomerDB;
use crate::schema::institution::InstitutionDB;
use crate::schema::organization::OrganizationDB;
use crate::schema::organization_unit::OrganizationUnitDB;

pub type CustomerMap = BTreeMap<ID, Arc<Customer>>;
pub type OrganizationMap = BTreeMap<CustomerResourceId, Arc<Organization>>;
pub type OrganizationUnitMap = BTreeMap<OrganizationUnitId, Arc<OrganizationUnit>>;
pub type InstitutionMap = BTreeMap<OrganizationResourceId, Arc<Institution>>;

use prometheus_client::metrics::gauge::Gauge;

pub trait CustomerCacheDB:
    CustomerDB + OrganizationDB + OrganizationUnitDB + InstitutionDB
{
}

async fn load_customers(db: &impl CustomerDB) -> anyhow::Result<CustomerMap> {
    let items: Vec<Customer> = db
        .customers()
        .as_ref()
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?;
    Ok(BTreeMap::from_iter(
        items
            .into_iter()
            .map(|c| (c.id.id.as_ref().unwrap().clone(), Arc::new(c))),
    ))
}

async fn load_organizations(db: &impl OrganizationDB) -> anyhow::Result<OrganizationMap> {
    let items: Vec<Organization> = db
        .organizations()
        .as_ref()
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?;
    Ok(BTreeMap::from_iter(
        items
            .into_iter()
            .map(|c| ((&c).try_into().unwrap(), Arc::new(c))),
    ))
}

async fn load_organization_units(
    db: &impl OrganizationUnitDB,
) -> anyhow::Result<OrganizationUnitMap> {
    let items: Vec<OrganizationUnit> = db
        .organization_units()
        .as_ref()
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?;
    Ok(BTreeMap::from_iter(
        items
            .into_iter()
            .map(|c| ((&c).try_into().unwrap(), Arc::new(c))),
    ))
}

async fn load_institutions(db: &impl InstitutionDB) -> anyhow::Result<InstitutionMap> {
    let items: Vec<Institution> = db
        .institutions()
        .as_ref()
        .find(doc! {}, None)
        .await?
        .try_collect()
        .await?;
    Ok(BTreeMap::from_iter(
        items
            .into_iter()
            .map(|c| ((&c).try_into().unwrap(), Arc::new(c))),
    ))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum CustomerCacheEventType {
    NewCustomer((ID, Arc<Customer>)),
    NewOrganization((OrganizationId, Arc<Organization>)),
    NewOrganizationUnit((OrganizationUnitId, Arc<OrganizationUnit>)),
    NewInstitution((InstitutionId, Arc<Institution>)),
    ReloadAll,
    ReloadCustomers,
    ReloadOrganizations,
    ReloadInstitutions,
    ReloadOrganizationUnits,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CustomerCacheEvent {
    publisher: Arc<Uuid>,
    event: CustomerCacheEventType,
}

#[derive(Clone)]
struct CustomerCacheInner {
    channel: Arc<str>,
    id: Arc<Uuid>,
    customers: Arc<RwLock<CustomerMap>>,
    organizations: Arc<RwLock<OrganizationMap>>,
    organization_units: Arc<RwLock<OrganizationUnitMap>>,
    institutions: Arc<RwLock<InstitutionMap>>,
    customers_total: Gauge<f64, AtomicU64>,
    organizations_total: Gauge<f64, AtomicU64>,
    organization_units_total: Gauge<f64, AtomicU64>,
    institutions_total: Gauge<f64, AtomicU64>,
}

#[derive(Clone)]
pub struct CustomerCache {
    inner: Arc<CustomerCacheInner>,
}

impl CustomerCache {
    pub async fn new(prefix: &str) -> anyhow::Result<Self> {
        Ok(Self {
            inner: Arc::new(CustomerCacheInner {
                id: Arc::new(Uuid::new()),
                customers_total: Default::default(),
                organizations_total: Default::default(),
                organization_units_total: Default::default(),
                institutions_total: Default::default(),
                channel: Arc::from(format!("{prefix}_customers")),
                customers: Default::default(),
                organizations: Default::default(),
                organization_units: Default::default(),
                institutions: Default::default(),
            }),
        })
    }

    pub fn customers_total(&self) -> &Gauge<f64, AtomicU64> {
        &self.inner.customers_total
    }

    pub fn organizations_total(&self) -> &Gauge<f64, AtomicU64> {
        &self.inner.organizations_total
    }

    pub fn organization_units_total(&self) -> &Gauge<f64, AtomicU64> {
        &self.inner.organization_units_total
    }

    pub fn institutions_total(&self) -> &Gauge<f64, AtomicU64> {
        &self.inner.institutions_total
    }

    pub async fn institutions_by_customer_id(
        &self,
        id: &ObjectId,
    ) -> Vec<(OrganizationResourceId, Arc<Institution>)> {
        self.inner
            .institutions
            .read()
            .await
            .iter()
            .filter(|i| i.1.id.cid.as_deref() == Some(id))
            .map(|(id, v)| (id.clone(), v.clone()))
            .collect()
    }

    pub async fn organization_by_customer_id(
        &self,
        id: &ObjectId,
    ) -> Vec<(Arc<ObjectId>, Arc<Organization>)> {
        self.inner
            .organizations
            .read()
            .await
            .iter()
            .filter(|i| i.1.id.cid.as_deref() == Some(id))
            .map(|(_, v)| (v.id.id.clone().unwrap(), v.clone()))
            .collect()
    }

    pub async fn customer_by_id(&self, id: &ObjectId) -> Option<Arc<Customer>> {
        self.inner.customers.read().await.get(id).cloned()
    }

    pub async fn organization_by_id(&self, id: &CustomerResourceId) -> Option<Arc<Organization>> {
        self.inner.organizations.read().await.get(id).cloned()
    }

    pub async fn organization_unit_by_id(
        &self,
        id: &OrganizationUnitId,
    ) -> Option<Arc<OrganizationUnit>> {
        self.inner.organization_units.read().await.get(id).cloned()
    }

    pub async fn customer_by_organization(
        &self,
        organization: &Organization,
    ) -> Option<Arc<Customer>> {
        self.inner
            .customers
            .read()
            .await
            .iter()
            .find(|(_, c)| c.id.id.as_deref() == organization.id.cid.as_deref())
            .map(|(_, c)| c.clone())
    }

    pub async fn organization_by_institution(
        &self,
        institution: &Institution,
    ) -> Option<Arc<Organization>> {
        self.inner
            .organizations
            .read()
            .await
            .iter()
            .find(|(_, o)| o.id.id.as_deref() == institution.id.oid.as_deref())
            .map(|(_, o)| o.clone())
    }

    pub async fn institution_by_member_id(&self, id: &MemberId) -> Option<Arc<Institution>> {
        self.inner
            .institutions
            .read()
            .await
            .get(&OrganizationResourceId {
                cid: id.cid.clone(),
                oid: id.oid.clone(),
                id: id.iid.clone(),
            })
            .cloned()
    }

    pub async fn institution_by_id(&self, id: &OrganizationResourceId) -> Option<Arc<Institution>> {
        self.inner.institutions.read().await.get(id).cloned()
    }

    pub async fn institutions_by_organization_id(
        &self,
        customer_sharded_id: &CustomerResourceId,
    ) -> Vec<(OrganizationResourceId, Arc<Institution>)> {
        self.inner
            .institutions
            .read()
            .await
            .iter()
            .filter(|i| i.0 == customer_sharded_id)
            .map(|(id, v)| (id.clone(), v.clone()))
            .collect()
    }

    pub async fn find_customer(&self, name: &str) -> Option<Arc<Customer>> {
        self.inner
            .customers
            .read()
            .await
            .iter()
            .find(|i| i.1.name.as_str() == name)
            .map(|(_, c)| c.clone())
    }

    pub async fn find_organization(&self, cid: &ObjectId, name: &str) -> Option<Arc<Organization>> {
        self.inner
            .organizations
            .read()
            .await
            .iter()
            .find(|i| i.0.cid.as_ref() == cid && i.1.name.as_str() == name)
            .map(|(_, c)| c.clone())
    }

    pub async fn find_institution(
        &self,
        cid: &ObjectId,
        oid: &ObjectId,
        name: &str,
    ) -> Option<Arc<Institution>> {
        self.inner
            .institutions
            .read()
            .await
            .iter()
            .find(|i| {
                i.0.cid.as_ref() == cid && i.0.oid.as_ref() == oid && i.1.name.as_str() == name
            })
            .map(|(_, c)| c.clone())
    }

    pub async fn reload(
        &self,
        db: &impl CustomerCacheDB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_customers(db).await?;
        self.inner.customers_total.set(next_items.len() as f64);
        *self.inner.customers.write().await = next_items;
        let next_items = load_organizations(db).await?;
        self.inner.organizations_total.set(next_items.len() as f64);
        *self.inner.organizations.write().await = next_items;
        let next_items = load_organization_units(db).await?;
        self.inner
            .organization_units_total
            .set(next_items.len() as f64);
        *self.inner.organization_units.write().await = next_items;
        let next_items = load_institutions(db).await?;
        self.inner.institutions_total.set(next_items.len() as f64);
        *self.inner.institutions.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&CustomerCacheEvent {
                    publisher,
                    event: CustomerCacheEventType::ReloadAll,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_customers(
        &self,
        db: &impl CustomerDB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_customers(db).await?;
        self.inner.customers_total.set(next_items.len() as f64);
        *self.inner.customers.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&CustomerCacheEvent {
                    publisher,
                    event: CustomerCacheEventType::ReloadCustomers,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_organizations(
        &self,
        db: &impl OrganizationDB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_organizations(db).await?;
        self.inner.organizations_total.set(next_items.len() as f64);
        *self.inner.organizations.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&CustomerCacheEvent {
                    publisher,
                    event: CustomerCacheEventType::ReloadOrganizations,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_institutions(
        &self,
        db: &impl InstitutionDB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_institutions(db).await?;
        self.inner.institutions_total.set(next_items.len() as f64);
        *self.inner.institutions.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&CustomerCacheEvent {
                    publisher,
                    event: CustomerCacheEventType::ReloadInstitutions,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    pub async fn reload_organization_units(
        &self,
        db: &impl OrganizationUnitDB,
        redis: Option<&deadpool_redis::Pool>,
    ) -> anyhow::Result<()> {
        let next_items = load_organization_units(db).await?;
        self.inner
            .organization_units_total
            .set(next_items.len() as f64);
        *self.inner.organization_units.write().await = next_items;
        if let Some(redis) = redis {
            let publisher = self.inner.id.clone();
            let mut con = redis.get().await?;
            con.publish(
                self.inner.channel.as_ref(),
                serde_json::to_string(&CustomerCacheEvent {
                    publisher,
                    event: CustomerCacheEventType::ReloadOrganizationUnits,
                })?,
            )
            .await?;
        }
        Ok(())
    }

    async fn load_customers(&self, id: Arc<ObjectId>, value: Arc<Customer>) {
        self.inner.customers.write().await.insert(id, value);
        self.inner
            .customers_total
            .set(self.inner.customers.read().await.len() as f64);
    }

    pub async fn new_customer(
        &self,
        redis: &deadpool_redis::Pool,
        customer: Customer,
    ) -> anyhow::Result<()> {
        let id = customer.id.id.as_ref().cloned().unwrap();
        let value = Arc::new(customer);
        self.load_customers(id.clone(), value.clone()).await;
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&CustomerCacheEvent {
                publisher,
                event: CustomerCacheEventType::NewCustomer((id, value)),
            })?,
        )
        .await?;
        Ok(())
    }

    async fn load_organization(&self, id: CustomerResourceId, value: Arc<Organization>) {
        self.inner.organizations.write().await.insert(id, value);
        self.inner
            .organizations_total
            .set(self.inner.organizations.read().await.len() as f64);
    }

    async fn load_organization_unit(&self, id: OrganizationUnitId, value: Arc<OrganizationUnit>) {
        self.inner
            .organization_units
            .write()
            .await
            .insert(id, value);
        self.inner
            .organization_units_total
            .set(self.inner.organization_units.read().await.len() as f64);
    }

    pub async fn new_organization(
        &self,
        redis: &deadpool_redis::Pool,
        organization: Organization,
    ) -> anyhow::Result<()> {
        let id: CustomerResourceId = (&organization).try_into()?;
        let value = Arc::new(organization);
        self.load_organization(id.clone(), value.clone()).await;
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&CustomerCacheEvent {
                publisher,
                event: CustomerCacheEventType::NewOrganization((id, value)),
            })?,
        )
        .await?;
        Ok(())
    }

    pub async fn new_organization_unit(
        &self,
        redis: &deadpool_redis::Pool,
        organization_unit: OrganizationUnit,
    ) -> anyhow::Result<()> {
        let id: OrganizationUnitId = (&organization_unit).try_into()?;
        let value = Arc::new(organization_unit);
        self.load_organization_unit(id.clone(), value.clone()).await;
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&CustomerCacheEvent {
                publisher,
                event: CustomerCacheEventType::NewOrganizationUnit((id, value)),
            })?,
        )
        .await?;
        Ok(())
    }

    async fn load_institution(&self, id: OrganizationResourceId, value: Arc<Institution>) {
        self.inner.institutions.write().await.insert(id, value);
        self.inner
            .institutions_total
            .set(self.inner.institutions.read().await.len() as f64);
    }

    pub async fn new_institution(
        &self,
        redis: &deadpool_redis::Pool,
        institution: Institution,
    ) -> anyhow::Result<()> {
        let id: OrganizationResourceId = (&institution).try_into()?;
        let value = Arc::new(institution);
        self.load_institution(id.clone(), value.clone()).await;
        let publisher = self.inner.id.clone();
        let mut con = redis.get().await?;
        con.publish(
            self.inner.channel.as_ref(),
            serde_json::to_string(&CustomerCacheEvent {
                publisher,
                event: CustomerCacheEventType::NewInstitution((id, value)),
            })?,
        )
        .await?;
        Ok(())
    }

    // TODO: implement EntityCache in qm-entity crate
    pub async fn process_event(
        &self,
        db: &impl CustomerCacheDB,
        msg: Msg, /*  cache: &EntityCache */
    ) -> anyhow::Result<()> {
        let CustomerCacheEvent { publisher, event } =
            serde_json::from_slice(msg.get_payload_bytes())?;
        match event {
            CustomerCacheEventType::NewCustomer((id, value)) => {
                if self.inner.id != publisher {
                    log::debug!("new customer with id: {id:?}");
                    self.load_customers(id, value).await;
                }
                // qm_entity::cache::invalidate::<Customer>(cache).await;
            }
            CustomerCacheEventType::NewOrganization((id, value)) => {
                if self.inner.id != publisher {
                    log::debug!("new organization with id: {id:?}");
                    self.load_organization(id, value).await;
                }
                // qm_entity::cache::invalidate::<Organization>(cache).await;
            }
            CustomerCacheEventType::NewOrganizationUnit((id, value)) => {
                if self.inner.id != publisher {
                    log::debug!("new organization unit with id: {id:?}");
                    self.load_organization_unit(id, value).await;
                }
                // qm_entity::cache::invalidate::<OrganizationUnit>(cache).await;
            }
            CustomerCacheEventType::NewInstitution((id, value)) => {
                if self.inner.id != publisher {
                    log::debug!("new institution with id: {id:?}");
                    self.load_institution(id, value).await;
                }
                // qm_entity::cache::invalidate::<Institution>(cache).await;
            }
            CustomerCacheEventType::ReloadAll => {
                self.reload(db, None).await?;
            }
            CustomerCacheEventType::ReloadCustomers => {
                self.reload_customers(db, None).await?;
            }
            CustomerCacheEventType::ReloadOrganizations => {
                self.reload_organizations(db, None).await?;
            }
            CustomerCacheEventType::ReloadInstitutions => {
                self.reload_institutions(db, None).await?;
            }
            CustomerCacheEventType::ReloadOrganizationUnits => {
                self.reload_organization_units(db, None).await?;
            }
        }
        Ok(())
    }
}
