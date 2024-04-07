use super::update::Op;
use super::update::Payload;
use crate::model::*;
use crate::query::fetch_customers;
use crate::query::fetch_institutions;
use crate::query::fetch_organization_units;
use crate::query::fetch_organizations;
use prometheus_client::metrics::gauge::Gauge;
use qm_entity::ids::InfraId;
use qm_pg::DB;
use sha2::{Digest, Sha512};
use sqlx::postgres::PgListener;
use std::collections::HashMap;
use std::sync::atomic::AtomicI64;
use std::sync::Arc;
use time::macros::format_description;
use time::PrimitiveDateTime;
use tokio::sync::RwLock;

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

const EMPTY_SHA: &str = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

pub struct InfraDB {
    pub customers_version: RwLock<Arc<str>>,
    pub customers: RwLock<CustomerMap>,
    pub customer_id_map: RwLock<CustomerIdMap>,
    pub customers_list: RwLock<Arc<[Arc<Customer>]>>,
    pub customers_total: Gauge<i64, AtomicI64>,
    pub organizations_version: RwLock<Arc<str>>,
    pub organizations: RwLock<OrganizationMap>,
    pub organization_id_map: RwLock<OrganizationIdMap>,
    pub organizations_list: RwLock<Arc<[Arc<Organization>]>>,
    pub organizations_total: Gauge<i64, AtomicI64>,
    pub organization_units_version: RwLock<Arc<str>>,
    pub organization_units: RwLock<OrganizationUnitMap>,
    pub organization_unit_id_map: RwLock<OrganizationUnitIdMap>,
    pub organization_units_list: RwLock<Arc<[Arc<OrganizationUnit>]>>,
    pub organization_units_total: Gauge<i64, AtomicI64>,
    pub institutions_version: RwLock<Arc<str>>,
    pub institutions: RwLock<InstitutionMap>,
    pub institution_id_map: RwLock<InstitutionIdMap>,
    pub institutions_list: RwLock<Arc<[Arc<Institution>]>>,
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
        log::info!("start init InfraDB");
        let start = std::time::Instant::now();
        let customers_total = Gauge::default();
        let organizations_total = Gauge::default();
        let organization_units_total = Gauge::default();
        let institutions_total = Gauge::default();
        let mut migrator = sqlx::migrate!("./migrations/customer");
        migrator.set_ignore_missing(true);
        migrator.run(db.pool()).await?;
        let result = Self {
            customers_version: RwLock::new(Arc::from(EMPTY_SHA)),
            customers: Default::default(),
            customer_id_map: Default::default(),
            customers_list: RwLock::new(Arc::from(vec![])),
            customers_total,
            organizations_version: RwLock::new(Arc::from(EMPTY_SHA)),
            organizations: Default::default(),
            organization_id_map: Default::default(),
            organizations_list: RwLock::new(Arc::from(vec![])),
            organizations_total,
            organization_units_version: RwLock::new(Arc::from(EMPTY_SHA)),
            organization_units: Default::default(),
            organization_unit_id_map: Default::default(),
            organization_units_list: RwLock::new(Arc::from(vec![])),
            organization_units_total,
            institutions_version: RwLock::new(Arc::from(EMPTY_SHA)),
            institutions: Default::default(),
            institution_id_map: Default::default(),
            institutions_list: RwLock::new(Arc::from(vec![])),
            institutions_total,
        };
        result.reload(db).await?;
        let duration = start.elapsed();
        log::info!("initialized InfraDB within {duration:?}");
        Ok(result)
    }

    async fn load_customers(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_customers(db).await? {
            self.create_customer(Arc::new(v)).await;
        }
        self.update_total_customers().await;
        Ok(())
    }

    async fn load_organizations(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_organizations(db).await? {
            self.create_organization(Arc::new(v)).await;
        }
        self.update_total_organizations().await;
        Ok(())
    }

    async fn load_institutions(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_institutions(db).await? {
            self.create_institution(Arc::new(v)).await;
        }
        self.update_total_institutions().await;
        Ok(())
    }

    async fn load_organization_units(&self, db: &DB) -> anyhow::Result<()> {
        for v in fetch_organization_units(db).await? {
            self.create_organization_unit(Arc::new(v)).await;
        }
        self.update_total_organization_units().await;
        Ok(())
    }

    pub async fn reload(&self, db: &DB) -> anyhow::Result<()> {
        self.load_customers(db).await?;
        self.load_organizations(db).await?;
        self.load_institutions(db).await?;
        self.load_organization_units(db).await?;
        Ok(())
    }

    pub async fn create_customer(&self, customer: Arc<Customer>) {
        self.customers
            .write()
            .await
            .insert(customer.name.clone(), customer.clone());
        self.customer_id_map
            .write()
            .await
            .insert(customer.id, customer);
    }

    async fn update_total_customers(&self) {
        self.customers_total
            .set(self.customers.read().await.len() as i64);
        let mut hash = Sha512::new();
        let mut list: Vec<Arc<Customer>> = self
            .customer_id_map
            .read()
            .await
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        for v in list.iter() {
            if let Ok(buf) = bincode::serialize(v) {
                hash.update(&buf);
            }
        }
        let hash = hash.finalize();
        *self.customers_version.write().await = Arc::from(hex::encode(hash));
        *self.customers_list.write().await = Arc::from(list);
    }

    pub async fn new_customer(&self, customer: Arc<Customer>) {
        self.create_customer(customer).await;
        self.update_total_customers().await;
    }

    pub async fn create_organization(&self, organization: Arc<Organization>) {
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

    async fn update_total_organizations(&self) {
        self.organizations_total
            .set(self.organizations.read().await.len() as i64);
        let mut hash = Sha512::new();
        let mut list: Vec<Arc<Organization>> = self
            .organization_id_map
            .read()
            .await
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        for v in list.iter() {
            if let Ok(buf) = bincode::serialize(v) {
                hash.update(&buf);
            }
        }
        let hash = hash.finalize();
        *self.organizations_version.write().await = Arc::from(hex::encode(hash));
        *self.organizations_list.write().await = Arc::from(list);
    }

    pub async fn new_organization(&self, organization: Arc<Organization>) {
        self.create_organization(organization).await;
        self.update_total_organizations().await;
    }

    pub async fn create_organization_unit(&self, organization_unit: Arc<OrganizationUnit>) {
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
    }

    async fn update_total_organization_units(&self) {
        self.organization_units_total
            .set(self.organization_units.read().await.len() as i64);
        let mut hash = Sha512::new();
        let mut list: Vec<Arc<OrganizationUnit>> = self
            .organization_unit_id_map
            .read()
            .await
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        for v in list.iter() {
            if let Ok(buf) = bincode::serialize(v) {
                hash.update(&buf);
            }
        }
        let hash = hash.finalize();
        *self.organization_units_version.write().await = Arc::from(hex::encode(hash));
        *self.organization_units_list.write().await = Arc::from(list);
    }

    pub async fn new_organization_unit(&self, organization_unit: Arc<OrganizationUnit>) {
        self.create_organization_unit(organization_unit).await;
        self.update_total_organization_units().await;
    }

    async fn create_institution(&self, institution: Arc<Institution>) {
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
    }

    pub async fn new_institution(&self, institution: Arc<Institution>) {
        self.create_institution(institution).await;
        self.update_total_institutions().await;
    }

    async fn update_total_institutions(&self) {
        self.institutions_total
            .set(self.institutions.read().await.len() as i64);
        let mut hash = Sha512::new();
        let mut list: Vec<Arc<Institution>> = self
            .institution_id_map
            .read()
            .await
            .values()
            .cloned()
            .collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        for v in list.iter() {
            if let Ok(buf) = bincode::serialize(v) {
                hash.update(&buf);
            }
        }
        let hash = hash.finalize();
        *self.institutions_version.write().await = Arc::from(hex::encode(hash));
        *self.institutions_list.write().await = Arc::from(list);
    }

    pub async fn remove_customer(&self, v: CustomerUpdate) {
        self.customers.write().await.remove(&v.name);
        self.customer_id_map.write().await.remove(&v.id);
        self.update_total_customers().await;
    }

    pub async fn update_customer(&self, new: Arc<Customer>, old: RemoveCustomerPayload) {
        let mut w1 = self.customers.write().await;
        let mut w2 = self.customer_id_map.write().await;
        w1.remove(&old.name);
        w2.remove(&old.id);
        w1.insert(new.name.clone(), new.clone());
        w2.insert(new.id, new);
        self.update_total_customers().await;
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
        self.update_total_organizations().await;
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
        self.update_total_institutions().await;
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
        self.update_total_organization_units().await;
    }

    pub async fn remove_organization(&self, v: OrganizationUpdate) {
        self.organizations
            .write()
            .await
            .remove(&(v.name.clone(), v.customer_id));
        self.organization_id_map.write().await.remove(&v.id);
        self.update_total_organizations().await;
    }

    pub async fn remove_organization_unit(&self, v: OrganizationUnitUpdate) {
        self.organization_units.write().await.remove(&(
            v.name.clone(),
            v.customer_id,
            v.organization_id,
        ));
        self.organization_unit_id_map.write().await.remove(&v.id);
        self.update_total_organization_units().await;
    }

    pub async fn remove_institution(&self, v: InstitutionUpdate) {
        self.institutions
            .write()
            .await
            .remove(&(v.name.clone(), v.customer_id, v.organization_id));
        self.institution_id_map.write().await.remove(&v.id);
        self.update_total_institutions().await;
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
                        ty: new.ty,
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
                        ty: new.ty,
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
                        ty: new.ty,
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
                        ty: new.ty,
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
