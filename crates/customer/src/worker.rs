use crate::cleanup::cleanup_api_clients;
use crate::cleanup::cleanup_roles;
use crate::cleanup::CleanupTaskType;
use crate::context::RelatedAuth;
use crate::context::RelatedPermission;
use crate::context::RelatedResource;
use crate::context::RelatedStorage;
use crate::marker::Marker;

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::cleanup::CleanupTask;
use qm_entity::ids::CustomerId;
use qm_entity::ids::CustomerIds;

use qm_entity::ids::InstitutionId;
use qm_entity::ids::InstitutionIds;

use qm_entity::ids::OrganizationId;
use qm_entity::ids::OrganizationIds;

use qm_entity::ids::OrganizationUnitId;
use qm_entity::ids::OrganizationUnitIds;

use qm_entity::ids::CUSTOMER_UNIT_ID_PREFIX;
use qm_entity::ids::INSTITUTION_ID_PREFIX;
use qm_entity::ids::INSTITUTION_UNIT_ID_PREFIX;
use qm_entity::ids::ORGANIZATION_ID_PREFIX;
use qm_kafka::producer::EventNs;
use qm_mongodb::bson::doc;

use qm_mongodb::bson::Document;
use qm_mongodb::ClientSession;
use qm_mongodb::DB;
use qm_role::AccessLevel;
use sqlx::types::Uuid;

use qm_redis::AsyncWorker;
pub use qm_redis::Producer;
use qm_redis::Work;
use qm_redis::WorkerContext;
use qm_redis::Workers;

lazy_static::lazy_static! {
    static ref PREFIX: String = {
        std::env::var("CUSTOMER_CLEANUP_TASK_PREFIX").unwrap_or("cleanup_tasks".to_string())
    };
}

pub trait CleanupTaskProducer {
    fn cleanup_task_producer(&self) -> &qm_redis::Producer;
}

#[derive(Clone)]
pub struct CleanupProducer {
    inner: Arc<Producer>,
}

impl CleanupProducer {
    pub fn new(redis: Arc<deadpool_redis::Pool>) -> Self {
        Self {
            inner: Arc::new(Producer::new_with_client(redis, PREFIX.as_str())),
        }
    }
}

impl AsRef<Producer> for CleanupProducer {
    fn as_ref(&self) -> &Producer {
        self.inner.as_ref()
    }
}

pub struct CleanupWorkerCtx<Auth, Store, Resource, Permission> {
    pub store: Store,
    _marker: Marker<Auth, Store, Resource, Permission, ()>,
}

impl<Auth, Store, Resource, Permission> CleanupWorkerCtx<Auth, Store, Resource, Permission> {
    pub fn new(store: Store) -> Self {
        Self {
            store,
            _marker: std::marker::PhantomData,
        }
    }
}

impl<Auth, Store, Resource, Permission> Clone
    for CleanupWorkerCtx<Auth, Store, Resource, Permission>
where
    Store: RelatedStorage,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            _marker: self._marker,
        }
    }
}

async fn remove_documents(
    db: &DB,
    session: &mut ClientSession,
    collection: &str,
    query: &Document,
) -> anyhow::Result<u64> {
    let result = db
        .get()
        .collection::<Document>(collection)
        .delete_many(query.clone())
        .session(session)
        .await?;
    Ok(result.deleted_count)
}

async fn cleanup_customers<Auth, Store, Resource, Permission>(
    worker_ctx: WorkerContext<CleanupWorkerCtx<Auth, Store, Resource, Permission>>,
    ty: &str,
    id: Uuid,
    cids: &CustomerIds,
) -> anyhow::Result<()>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    let store: &Store = &worker_ctx.ctx().store;
    let db: &DB = store.as_ref();
    let mut session = db.session().await?;
    let mut roles = BTreeSet::new();
    let existing_roles = store.cache_db().roles().await;
    let access_roles: Vec<&str> = existing_roles
        .iter()
        .filter(|k| k.name.contains("access@"))
        .map(|v| v.name.as_ref())
        .collect();
    let mut client_ids = Vec::with_capacity(cids.len());
    for cid in cids.iter() {
        client_ids.push(cid.to_string());
        roles.insert(
            qm_role::Access::new(AccessLevel::Customer)
                .with_fmt_id(Some(cid))
                .to_string(),
        );
        extend_roles_with_children(
            cid,
            &[
                INSTITUTION_ID_PREFIX,
                INSTITUTION_UNIT_ID_PREFIX,
                ORGANIZATION_ID_PREFIX,
                CUSTOMER_UNIT_ID_PREFIX,
            ],
            &access_roles,
            &mut roles,
        );
    }
    let cids: Vec<i64> = cids.iter().map(CustomerId::unzip).collect();
    let query = doc! {
        "owner.cid": {
            "$in": &cids
        },
    };
    for collection in db
        .get()
        .list_collection_names()
        .session(&mut session)
        .await?
    {
        log::debug!("remove all organization related resources from db {collection}");
        remove_documents(db, &mut session, &collection, &query).await?;
    }
    log::debug!("cleanup api clients");
    cleanup_api_clients(store.keycloak(), client_ids).await?;
    log::debug!("cleanup roles");
    cleanup_roles(store.keycloak(), roles).await?;
    // Emit the Kafka event
    if let Some(producer) = store.mutation_event_producer() {
        producer
            .delete_event(&EventNs::Customer, "customer", cids)
            .await?;
    }
    worker_ctx.complete().await?;
    log::debug!("finished cleanup task '{ty}' with id '{id}'");
    Ok(())
}

fn extend_roles_with_children(
    v: &impl std::fmt::Display,
    allowed_prefixes: &[char],
    access_roles: &[&str],
    roles: &mut BTreeSet<String>,
) {
    let id = v.to_string();
    for role in access_roles.iter() {
        if let Some((_, role_id)) = role.rsplit_once("access@") {
            if !role_id.is_empty()
                && !id.is_empty()
                && allowed_prefixes.iter().any(|v| role_id.starts_with(*v))
                && role_id[1..].starts_with(&id[1..])
            {
                roles.insert(role.to_string());
            }
        }
    }
}

async fn cleanup_organizations<Auth, Store, Resource, Permission>(
    worker_ctx: WorkerContext<CleanupWorkerCtx<Auth, Store, Resource, Permission>>,
    ty: &str,
    id: Uuid,
    strict_oids: &OrganizationIds,
) -> anyhow::Result<()>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    let store: &Store = &worker_ctx.ctx().store;
    let db: &DB = store.as_ref();
    let mut session = db.session().await?;
    let mut roles = BTreeSet::new();
    let existing_roles = store.cache_db().roles().await;
    let access_roles: Vec<&str> = existing_roles
        .iter()
        .filter(|k| k.name.contains("access@"))
        .map(|v| v.name.as_ref())
        .collect();
    let mut client_ids = Vec::with_capacity(strict_oids.len());
    for v in strict_oids.iter() {
        client_ids.push(v.to_string());
        roles.insert(
            qm_role::Access::new(AccessLevel::Organization)
                .with_fmt_id(Some(&v))
                .to_string(),
        );
        extend_roles_with_children(
            v,
            &[INSTITUTION_ID_PREFIX, INSTITUTION_UNIT_ID_PREFIX],
            &access_roles,
            &mut roles,
        );
    }
    let (cids, oids): (Vec<i64>, Vec<i64>) = strict_oids.iter().map(OrganizationId::unzip).unzip();
    let query = doc! {
        "owner.cid": {
            "$in": &cids
        },
        "owner.oid": {
            "$in": &oids
        }
    };
    for collection in db
        .get()
        .list_collection_names()
        .session(&mut session)
        .await?
    {
        log::debug!("remove all organization related resources from db {collection}");
        remove_documents(db, &mut session, &collection, &query).await?;
    }
    log::debug!("cleanup api clients");
    cleanup_api_clients(store.keycloak(), client_ids).await?;
    log::debug!("cleanup roles");
    cleanup_roles(store.keycloak(), roles).await?;
    // // Emit the Kafka event
    if let Some(producer) = store.mutation_event_producer() {
        producer
            .delete_event(&EventNs::Organization, "organization", strict_oids)
            .await?;
    }
    worker_ctx.complete().await?;
    log::debug!("finished cleanup task '{ty}' with id '{id}'");
    Ok(())
}

async fn cleanup_institutions<Auth, Store, Resource, Permission>(
    worker_ctx: WorkerContext<CleanupWorkerCtx<Auth, Store, Resource, Permission>>,
    ty: &str,
    id: Uuid,
    strict_iids: &InstitutionIds,
) -> anyhow::Result<()>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    let store: &Store = &worker_ctx.ctx().store;
    let db = store.as_ref();
    let mut session = db.session().await?;
    let mut roles = BTreeSet::new();
    let mut client_ids = Vec::with_capacity(strict_iids.len());
    for id in strict_iids.iter() {
        client_ids.push(id.to_string());
        roles.insert(
            qm_role::Access::new(AccessLevel::Institution)
                .with_fmt_id(Some(&id))
                .to_string(),
        );
    }
    let (cids, (oids, iids)): (Vec<i64>, (Vec<i64>, Vec<i64>)) =
        strict_iids.iter().map(InstitutionId::untuple).unzip();
    let query = doc! {
        "owner.cid": {
            "$in": &cids
        },
        "owner.oid": {
            "$in": &oids
        },
        "owner.iid": {
            "$in": &iids
        }
    };
    for collection in db
        .get()
        .list_collection_names()
        .session(&mut session)
        .await?
    {
        log::debug!("remove all organization related resources from db {collection}");
        remove_documents(db, &mut session, &collection, &query).await?;
    }
    log::debug!("cleanup api clients");
    cleanup_api_clients(store.keycloak(), client_ids).await?;
    log::debug!("cleanup roles");
    cleanup_roles(store.keycloak(), roles).await?;
    // // Emit the Kafka event
    if let Some(producer) = store.mutation_event_producer() {
        producer
            .delete_event(&EventNs::Institution, "institution", strict_iids)
            .await?;
    }
    worker_ctx.complete().await?;
    log::debug!("finished cleanup task '{ty}' with id '{id}'");
    Ok(())
}

async fn cleanup_organization_units<Auth, Store, Resource, Permission>(
    worker_ctx: WorkerContext<CleanupWorkerCtx<Auth, Store, Resource, Permission>>,
    ty: &str,
    id: Uuid,
    strict_uids: &OrganizationUnitIds,
) -> anyhow::Result<()>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    let store: &Store = &worker_ctx.ctx().store;
    let db: &DB = store.as_ref();
    let mut session = db.session().await?;
    let mut roles = BTreeSet::new();
    let mut client_ids = Vec::with_capacity(strict_uids.len());
    for id in strict_uids.iter() {
        client_ids.push(id.to_string());
        match id {
            OrganizationUnitId::Customer(_) => {
                roles.insert(
                    qm_role::Access::new(AccessLevel::CustomerUnit)
                        .with_fmt_id(Some(id))
                        .to_string(),
                );
            }
            OrganizationUnitId::Organization(_) => {
                roles.insert(
                    qm_role::Access::new(AccessLevel::InstitutionUnit)
                        .with_fmt_id(Some(id))
                        .to_string(),
                );
            }
        }
    }
    let (cids, uids): (Vec<i64>, Vec<i64>) =
        strict_uids.iter().map(OrganizationUnitId::untuple).unzip();
    let query = doc! {
        "owner.cid": {
            "$in": &cids
        },
        "owner.uid": {
            "$in": &uids
        }
    };
    for collection in db
        .get()
        .list_collection_names()
        .session(&mut session)
        .await?
    {
        log::debug!("remove all organization unit related resources from db {collection}");
        remove_documents(db, &mut session, &collection, &query).await?;
    }
    log::debug!("cleanup api clients");
    cleanup_api_clients(store.keycloak(), client_ids).await?;
    log::debug!("cleanup roles");
    cleanup_roles(store.keycloak(), roles).await?;
    // Emit the Kafka event
    if let Some(producer) = store.mutation_event_producer() {
        producer
            .delete_event(&EventNs::OrganizationUnit, "organization_unit", strict_uids)
            .await?;
    }
    worker_ctx.complete().await?;
    log::debug!("finished cleanup task '{ty}' with id '{id}'");
    Ok(())
}

pub struct CleanupWorker;

#[async_trait::async_trait]
impl<Auth, Store, Resource, Permission>
    Work<CleanupWorkerCtx<Auth, Store, Resource, Permission>, CleanupTask> for CleanupWorker
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    async fn run(
        &self,
        ctx: WorkerContext<CleanupWorkerCtx<Auth, Store, Resource, Permission>>,
        item: CleanupTask,
    ) -> anyhow::Result<()> {
        log::debug!(
            "start cleanup task '{}' with id '{}'",
            item.ty.as_ref(),
            item.id
        );
        match &item.ty {
            CleanupTaskType::Customers(ids) => {
                cleanup_customers(ctx, item.ty.as_ref(), item.id, ids).await?;
            }
            CleanupTaskType::Organizations(ids) => {
                cleanup_organizations(ctx, item.ty.as_ref(), item.id, ids).await?;
            }
            CleanupTaskType::Institutions(ids) => {
                cleanup_institutions(ctx, item.ty.as_ref(), item.id, ids).await?;
            }
            CleanupTaskType::OrganizationUnits(ids) => {
                cleanup_organization_units(ctx, item.ty.as_ref(), item.id, ids).await?;
            }
            CleanupTaskType::None => {
                ctx.complete().await?;
            }
        }
        Ok(())
    }
}

pub async fn run<Auth, Store, Resource, Permission>(
    workers: &Workers,
    ctx: CleanupWorkerCtx<Auth, Store, Resource, Permission>,
    num_workers: usize,
) -> anyhow::Result<()>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    workers
        .start(
            ctx,
            AsyncWorker::new(PREFIX.as_str())
                .with_num_workers(num_workers)
                .run(CleanupWorker),
        )
        .await?;
    Ok(())
}
