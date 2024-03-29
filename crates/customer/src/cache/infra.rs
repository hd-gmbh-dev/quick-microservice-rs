use crate::model::*;
use crate::query::fetch_customers;
use crate::query::fetch_institutions;
use crate::query::fetch_organization_units;
use crate::query::fetch_organizations;
use prometheus_client::metrics::gauge::Gauge;
use qm_entity::ids::InfraId;
use qm_pg::DB;
use sqlx::postgres::PgListener;
use std::collections::HashMap;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use time::macros::format_description;
use time::PrimitiveDateTime;
use tokio::sync::RwLock;

use super::update::Op;
use super::update::Payload;

pub type CustomerMap = HashMap<Arc<str>, Arc<Customer>>;
pub type CustomerIdMap = HashMap<InfraId, Arc<Customer>>;
pub type OrganizationMap = HashMap<(Arc<str>, InfraId), Arc<Organization>>;
pub type OrganizationIdMap = HashMap<InfraId, Arc<Organization>>;
pub type OrganizationUnitMap = HashMap<(Arc<str>, InfraId, Option<InfraId>), Arc<OrganizationUnit>>;
pub type OrganizationUnitIdMap = HashMap<InfraId, Arc<OrganizationUnit>>;
pub type InstitutionMap = HashMap<(Arc<str>, InfraId, InfraId), Arc<Institution>>;
pub type InstitutionIdMap = HashMap<InfraId, Arc<Institution>>;

fn parse_date_time(s: &str) -> Option<PrimitiveDateTime> {
    let format = format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond]");
    PrimitiveDateTime::parse(s, format).ok()
}

pub struct InfraDB {
    pub customers: RwLock<CustomerMap>,
    pub customer_id_map: RwLock<CustomerIdMap>,
    pub customers_total: Gauge<i64, AtomicI64>,
    pub organizations: RwLock<OrganizationMap>,
    pub organization_id_map: RwLock<OrganizationIdMap>,
    pub organizations_total: Gauge<i64, AtomicI64>,
    pub organization_units: RwLock<OrganizationUnitMap>,
    pub organization_unit_id_map: RwLock<OrganizationUnitIdMap>,
    pub organization_units_total: Gauge<i64, AtomicI64>,
    pub institutions: RwLock<InstitutionMap>,
    pub institution_id_map: RwLock<InstitutionIdMap>,
    pub institutions_total: Gauge<i64, AtomicI64>,
}

impl InfraDB {
    pub async fn cleanup(db: &DB) -> anyhow::Result<()> {
        let mut migrator = sqlx::migrate!("./migrations/customer");
        migrator.set_ignore_missing(true);
        migrator.undo(db.pool(), 0).await?;
        Ok(())
    }

    pub async fn new(db: &DB) -> anyhow::Result<Self> {
        let customers_total = Gauge::default();
        let organizations_total = Gauge::default();
        let organization_units_total = Gauge::default();
        let institutions_total = Gauge::default();
        let mut migrator = sqlx::migrate!("./migrations/customer");
        migrator.set_ignore_missing(true);
        migrator.run(db.pool()).await?;
        let result = Self {
            customers: Default::default(),
            customer_id_map: Default::default(),
            customers_total,
            organizations: Default::default(),
            organization_id_map: Default::default(),
            organizations_total,
            organization_units: Default::default(),
            organization_unit_id_map: Default::default(),
            organization_units_total,
            institutions: Default::default(),
            institution_id_map: Default::default(),
            institutions_total,
        };
        Ok(result)
    }

    async fn load_customers(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_customers(db).await? {
            self.new_customer(Arc::new(v)).await;
        }
        Ok(())
    }

    async fn load_organizations(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_organizations(db).await? {
            self.new_organization(Arc::new(v)).await;
        }
        Ok(())
    }

    async fn load_institutions(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_institutions(db).await? {
            self.new_institution(Arc::new(v)).await;
        }
        Ok(())
    }

    async fn load_organization_units(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_organization_units(db).await? {
            self.new_organization_unit(Arc::new(v)).await;
        }
        Ok(())
    }

    pub async fn reload(&self, db: &DB) -> anyhow::Result<()> {
        self.load_customers(db).await?;
        self.load_organizations(db).await?;
        self.load_institutions(db).await?;
        self.load_organization_units(db).await?;
        Ok(())
    }

    pub async fn new_customer(&self, customer: Arc<Customer>) {
        self.customers
            .write()
            .await
            .insert(customer.name.clone(), customer.clone());
        self.customer_id_map
            .write()
            .await
            .insert(customer.id, customer);
        self.customers_total
            .set(self.customers.read().await.len() as i64);
    }

    pub async fn new_organization(&self, organization: Arc<Organization>) {
        self.organizations.write().await.insert(
            (organization.name.clone(), organization.customer_id),
            organization.clone(),
        );
        self.organization_id_map
            .write()
            .await
            .insert(organization.id, organization);
        self.organizations_total
            .set(self.organizations.read().await.len() as i64);
    }

    pub async fn new_organization_unit(&self, organization_unit: Arc<OrganizationUnit>) {
        self.organization_units.write().await.insert(
            (
                organization_unit.name.clone(),
                organization_unit.customer_id,
                organization_unit.organization_id,
            ),
            organization_unit.clone(),
        );
        self.organization_unit_id_map
            .write()
            .await
            .insert(organization_unit.id, organization_unit);
        self.organization_units_total
            .set(self.organization_units.read().await.len() as i64);
    }

    pub async fn new_institution(&self, institution: Arc<Institution>) {
        self.institutions.write().await.insert(
            (
                institution.name.clone(),
                institution.customer_id,
                institution.organization_id,
            ),
            institution.clone(),
        );
        self.institution_id_map
            .write()
            .await
            .insert(institution.id, institution);
        self.institutions_total
            .set(self.institutions.read().await.len() as i64);
    }

    pub async fn remove_customer(&self, v: CustomerUpdate) {
        self.customers.write().await.remove(&v.name);
        self.customer_id_map.write().await.remove(&v.id);
        self.customers_total
            .set(self.customers.read().await.len() as i64);
    }

    pub async fn update_customer(&self, new: Arc<Customer>, old: RemoveCustomerPayload) {
        let mut w1 = self.customers.write().await;
        let mut w2 = self.customer_id_map.write().await;
        w1.remove(&old.name);
        w2.remove(&old.id);
        w1.insert(new.name.clone(), new.clone());
        w2.insert(new.id, new);
        self.customers_total
            .set(self.customers.read().await.len() as i64);
    }

    pub async fn update_organization(
        &self,
        new: Arc<Organization>,
        old: RemoveOrganizationPayload,
    ) {
        let mut w1 = self.organizations.write().await;
        let mut w2 = self.organization_id_map.write().await;
        w1.remove(&(old.name.clone(), old.customer_id));
        w2.remove(&old.id);
        w1.insert((new.name.clone(), new.customer_id), new.clone());
        w2.insert(new.id, new);
        self.organizations_total
            .set(self.organizations.read().await.len() as i64);
    }

    pub async fn update_institution(&self, new: Arc<Institution>, old: RemoveInstitutionPayload) {
        let mut w1 = self.institutions.write().await;
        let mut w2 = self.institution_id_map.write().await;
        w1.remove(&(old.name.clone(), old.customer_id, old.organization_id));
        w2.remove(&old.id);
        w1.insert(
            (new.name.clone(), new.customer_id, new.organization_id),
            new.clone(),
        );
        w2.insert(new.id, new);
        self.institutions_total
            .set(self.institutions.read().await.len() as i64);
    }

    pub async fn update_organization_unit(
        &self,
        new: Arc<OrganizationUnit>,
        old: RemoveOrganizationUnitPayload,
    ) {
        let mut w1 = self.organization_units.write().await;
        let mut w2 = self.organization_unit_id_map.write().await;
        w1.remove(&(old.name.clone(), old.customer_id, old.organization_id));
        w2.remove(&old.id);
        w1.insert(
            (new.name.clone(), new.customer_id, new.organization_id),
            new.clone(),
        );
        w2.insert(new.id, new);
        self.organization_units_total
            .set(self.organization_units.read().await.len() as i64);
    }

    pub async fn remove_organization(&self, v: OrganizationUpdate) {
        self.organizations
            .write()
            .await
            .remove(&(v.name.clone(), v.customer_id));
        self.organization_id_map.write().await.remove(&v.id);
        self.organizations_total
            .set(self.organizations.read().await.len() as i64);
    }

    pub async fn remove_organization_unit(&self, v: OrganizationUnitUpdate) {
        self.organization_units.write().await.remove(&(
            v.name.clone(),
            v.customer_id,
            v.organization_id,
        ));
        self.organization_unit_id_map.write().await.remove(&v.id);
        self.organization_units_total
            .set(self.organization_units.read().await.len() as i64);
    }

    pub async fn remove_institution(&self, v: InstitutionUpdate) {
        self.institutions
            .write()
            .await
            .remove(&(v.name.clone(), v.customer_id, v.organization_id));
        self.institution_id_map.write().await.remove(&v.id);
        self.institutions_total
            .set(self.institutions.read().await.len() as i64);
    }

    pub async fn listen(&self, db: &DB) -> anyhow::Result<()> {
        let mut listener = PgListener::connect_with(db.pool()).await?;
        listener
            .listen_all([
                "customers_update",
                "organizations_update",
                "organization_units_update",
                "institutions_update",
            ])
            .await?;

        while let Some(notification) = listener.try_recv().await? {
            match notification.channel() {
                "customers_update" => {
                    self.customers_update(notification.payload()).await?;
                }
                "organizations_update" => {
                    self.organizations_update(notification.payload()).await?;
                }
                "organization_units_update" => {
                    self.organization_units_update(notification.payload())
                        .await?;
                }
                "institutions_update" => {
                    self.institutions_update(notification.payload()).await?;
                }
                _ => {}
            }
        }
        log::error!("postgresql listener disconnected");
        std::process::exit(1);
    }

    async fn customers_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<CustomerUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if let Some(created_at) = parse_date_time(&new.created_at) {
                    let customer = Arc::new(Customer {
                        id: new.id,
                        name: new.name,
                        created_at,
                        created_by: new.created_by,
                        updated_at: new.updated_at.and_then(|s| parse_date_time(&s)),
                        updated_by: new.updated_by,
                    });
                    self.new_customer(customer).await;
                }
            }
            (Op::Delete, None, Some(old)) => {
                self.remove_customer(old).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn organizations_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<OrganizationUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if let Some(created_at) = parse_date_time(&new.created_at) {
                    let organization = Arc::new(Organization {
                        id: new.id,
                        customer_id: new.customer_id,
                        name: new.name,
                        created_at,
                        created_by: new.created_by,
                        updated_at: new.updated_at.and_then(|s| parse_date_time(&s)),
                        updated_by: new.updated_by,
                    });
                    self.new_organization(organization).await;
                }
            }
            (Op::Delete, None, Some(old)) => {
                self.remove_organization(old).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn organization_units_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<OrganizationUnitUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if let Some(created_at) = parse_date_time(&new.created_at) {
                    let organization_unit = Arc::new(OrganizationUnit {
                        id: new.id,
                        customer_id: new.customer_id,
                        organization_id: new.organization_id,
                        name: new.name,
                        created_at,
                        created_by: new.created_by,
                        updated_at: new.updated_at.and_then(|s| parse_date_time(&s)),
                        updated_by: new.updated_by,
                        members: Arc::from(vec![]),
                    });
                    self.new_organization_unit(organization_unit).await;
                }
            }
            (Op::Delete, None, Some(old)) => {
                self.remove_organization_unit(old).await;
            }
            _ => {}
        }
        Ok(())
    }

    async fn institutions_update(&self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<InstitutionUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if let Some(created_at) = parse_date_time(&new.created_at) {
                    let organization = Arc::new(Institution {
                        id: new.id,
                        customer_id: new.customer_id,
                        organization_id: new.organization_id,
                        name: new.name,
                        created_at,
                        created_by: new.created_by,
                        updated_at: new.updated_at.and_then(|s| parse_date_time(&s)),
                        updated_by: new.updated_by,
                    });
                    self.new_institution(organization).await;
                }
            }
            (Op::Delete, None, Some(old)) => {
                self.remove_institution(old).await;
            }
            _ => {}
        }
        Ok(())
    }
}
