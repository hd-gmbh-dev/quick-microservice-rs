use async_graphql::ComplexObject;
use async_graphql::{Context, ErrorExtensions, FieldResult, Object, ResultExt};
use qm_entity::{exerr, IsAdmin};
use qm_entity::ids::InfraContext;

use qm_entity::model::ListFilter;
use qm_keycloak::{GroupRepresentation, RoleRepresentation};
use qm_role::Access;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

use qm_entity::err;
use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_keycloak::CredentialRepresentation;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::UserRepresentation;
use sqlx::types::Uuid;

use crate::cache::CacheDB;
use crate::config::SchemaConfig;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::{CreateUserPayload, Group, GroupList};
use crate::model::RequiredUserAction;
use crate::model::User;
use crate::model::UserList;
use crate::model::{CreateUserInput, Customer, Institution, Organization, OrganizationUnit};

// use crate::model::User;
// use crate::model::{CreateUserInput, CreateUserPayload, UserList};
// use crate::model::{RequiredUserAction, UserData, UserDetails};
use crate::schema::auth::AuthCtx;
use crate::schema::RelatedAccessLevel;
use crate::schema::RelatedAuth;
use crate::schema::RelatedPermission;
use crate::schema::RelatedResource;
use crate::schema::RelatedStorage;

#[ComplexObject]
impl Group {
    async fn customer(&self, ctx: &Context<'_>) -> Option<Arc<Customer>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self.context.as_ref().map(InfraContext::customer_id) {
            return cache.customer_by_id(&id).await;
        }
        None
    }

    async fn organization(&self, ctx: &Context<'_>) -> Option<Arc<Organization>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .context
            .as_ref()
            .and_then(InfraContext::organization_id)
        {
            return cache.organization_by_id(&id).await;
        }
        None
    }

    async fn organization_unit(&self, ctx: &Context<'_>) -> Option<Arc<OrganizationUnit>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self
            .context
            .as_ref()
            .and_then(InfraContext::organization_unit_id)
        {
            return cache.organization_unit_by_id(&id).await;
        }
        None
    }

    async fn institution(&self, ctx: &Context<'_>) -> Option<Arc<Institution>> {
        let cache = ctx.data::<CacheDB>().ok();
        if cache.is_none() {
            log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
            return None;
        }
        let cache = cache.unwrap();
        if let Some(id) = self.context.as_ref().and_then(InfraContext::institution_id) {
            return cache.institution_by_id(&id).await;
        }
        None
    }
}

fn map_kc_error(name: &str, err: KeycloakError) -> EntityError {
    match err {
        KeycloakError::HttpFailure {
            status: 409,
            ..
        } => {
            EntityError::fields_conflict::<Group>(name, &["name"][..])
        },
        KeycloakError::HttpFailure {
            status: 400,
            body: Some(e),
            ..
        } => {
            let mut err_type = String::new();
            if err_type.is_empty() {
                err_type.push_str("unknown");
            }
            // bad_request_name(&err_type, &err_msg)
            EntityError::bad_request(err_type, e.error_message.unwrap_or_default())
        }
        _ => {
            EntityError::internal()
        }
    }
}

pub struct Ctx<'a, Auth, Store, AccessLevel, Resource, Permission>(
    pub AuthCtx<'a, Auth, Store, AccessLevel, Resource, Permission>,
)
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission;
impl<'a, Auth, Store, AccessLevel, Resource, Permission>
    Ctx<'a, Auth, Store, AccessLevel, Resource, Permission>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    pub async fn list(
        &self,
        mut context: Option<InfraContext>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<GroupList> {
        context = self.0.enforce_current_context(context).await?;
        Ok(self.0.store.cache_db().group_list(context, filter).await)
    }

    pub async fn create(&self, name: String, context: InfraContext) -> async_graphql::FieldResult<Arc<Group>> {
        let ctx = context.to_string();
        let group_name = format!("{name}@{ctx}");
        self.0.store.keycloak().create_group(
            self.0.store.keycloak().config().realm(), 
            GroupRepresentation {
                name: Some(group_name.clone()),
                attributes: Some(HashMap::from_iter([
                    ("context".to_string(), json!([context.to_string()])),
                ])),
                ..Default::default()
            }
        )
        .await
        .map_err(|err| map_kc_error(&name, err))
        .extend()?;
        let result = self.0.store.keycloak()
            .group_by_path(self.0.store.keycloak().config().realm(), &name)
            .await?;
        let group = Arc::new(Group {
            id: Arc::from(result.id.unwrap()),
            name: Arc::from(result.path.unwrap()),
            built_in: false,
            context: Some(context),
        });
        self.0.store.cache_db().user().new_group(group.clone()).await;        
        Ok(group)
    }
}

pub struct GroupQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for GroupQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    GroupQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn groups(
        &self,
        ctx: &Context<'_>,
        context: Option<InfraContext>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<GroupList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::create()),
            )
            .await?,
        )
        .list(context, filter)
        .await
        .extend()
    }
}

pub struct GroupMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for GroupMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

// fetch("https://keycloak.shapth.local/admin/realms/shapth/groups/b5ee117a-e5e4-4d08-a0f5-896edb893fa3", {
//     "headers": {
//       "accept": "application/json, text/plain, */*",
//       "accept-language": "de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7",
//       "authorization": "Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJIX29FX0J0dzJ2S3NFNHZLSktldFRKS3B2SEl4dFllbFp6YTJGRGtUb0FFIn0.eyJleHAiOjE3MTE3ODIwOTIsImlhdCI6MTcxMTc4MjAzMiwiYXV0aF90aW1lIjoxNzExNzgxMzMxLCJqdGkiOiJhZmFhYTg2MS1jN2M5LTQ1YTctOTc4NC0zNjhiNWIyZGJlYWEiLCJpc3MiOiJodHRwczovL2F1dGguc2hhcHRoLmxvY2FsL3JlYWxtcy9tYXN0ZXIiLCJzdWIiOiIzODdiOWFmZi05MmRlLTQ2ZGMtOTM3My0yZTI3ZGE0NzFlYTkiLCJ0eXAiOiJCZWFyZXIiLCJhenAiOiJzZWN1cml0eS1hZG1pbi1jb25zb2xlIiwibm9uY2UiOiJkYmFlZmIyNC01NmRjLTQ4ZjEtYWM2Yy04YWRmYzY2NDAxYTQiLCJzZXNzaW9uX3N0YXRlIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiYWNyIjoiMSIsImFsbG93ZWQtb3JpZ2lucyI6WyJodHRwczovL2tleWNsb2FrLnNoYXB0aC5sb2NhbCJdLCJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIiwic2lkIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiZW1haWxfdmVyaWZpZWQiOmZhbHNlLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJhZG1pbiJ9.LAnjjs5mEueck1cT8oZtLGMC_z_wSeX7C9qVicZ8F1ggkUXAOx5dZopWWY9ASSc5RtSPApGvpMTRhsVjOyukqsoq0BHy4Lq_ZNJejsuI0eAGulZZH-DpKQ9v7Z3voSmFYQTKLtjMASVeEJbQklHh6VHSlQ2ZRSfoiPjbpHKXtCapdZcoawtTU5hbbWp1QQ41o171L1fAlxh9cxMtud9lLc105BExRIkxFunD17h9pClnLxakngdpZ8n1VvVi3CV_f0uWQj1VcbLlNi1XvzyZumdYhUAr7Gj0SA_B9FRW51czqsQj2HmZXtvK-8OHqTZid4R1_hwFMjpgk3yNFWsCfg",
//       "cache-control": "no-cache",
//       "content-type": "application/json",
//       "pragma": "no-cache",
//       "sec-ch-ua": "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"",
//       "sec-ch-ua-mobile": "?0",
//       "sec-ch-ua-platform": "\"macOS\"",
//       "sec-fetch-dest": "empty",
//       "sec-fetch-mode": "cors",
//       "sec-fetch-site": "same-origin"
//     },
//     "referrerPolicy": "no-referrer",
//     "body": "{\"id\":\"b5ee117a-e5e4-4d08-a0f5-896edb893fa3\",\"name\":\"my_employee_manager\",\"path\":\"/my_employee_manager\",\"subGroupCount\":0,\"subGroups\":[],\"attributes\":{\"context\":[\"V01\"],\"should_not_appair_key\":[\"should_not_appair_value\"],\"built_in\":[\"true\"]},\"realmRoles\":[],\"clientRoles\":{},\"access\":{\"view\":true,\"viewMembers\":true,\"manageMembers\":true,\"manage\":true,\"manageMembership\":true}}",
//     "method": "PUT",
//     "mode": "cors",
//     "credentials": "include"
//   });

// fetch("https://keycloak.shapth.local/admin/realms/shapth/groups/b5ee117a-e5e4-4d08-a0f5-896edb893fa3/role-mappings/realm", {
//     "headers": {
//       "accept": "application/json, text/plain, */*",
//       "accept-language": "de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7",
//       "authorization": "Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJIX29FX0J0dzJ2S3NFNHZLSktldFRKS3B2SEl4dFllbFp6YTJGRGtUb0FFIn0.eyJleHAiOjE3MTE3ODI1NjYsImlhdCI6MTcxMTc4MjUwNiwiYXV0aF90aW1lIjoxNzExNzgxMzMxLCJqdGkiOiJiMDQ2MGFiZC05YjU1LTQxNmEtYjI5Zi04MWQzYWNjYmFjOGYiLCJpc3MiOiJodHRwczovL2F1dGguc2hhcHRoLmxvY2FsL3JlYWxtcy9tYXN0ZXIiLCJzdWIiOiIzODdiOWFmZi05MmRlLTQ2ZGMtOTM3My0yZTI3ZGE0NzFlYTkiLCJ0eXAiOiJCZWFyZXIiLCJhenAiOiJzZWN1cml0eS1hZG1pbi1jb25zb2xlIiwibm9uY2UiOiJkYmFlZmIyNC01NmRjLTQ4ZjEtYWM2Yy04YWRmYzY2NDAxYTQiLCJzZXNzaW9uX3N0YXRlIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiYWNyIjoiMSIsImFsbG93ZWQtb3JpZ2lucyI6WyJodHRwczovL2tleWNsb2FrLnNoYXB0aC5sb2NhbCJdLCJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIiwic2lkIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiZW1haWxfdmVyaWZpZWQiOmZhbHNlLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJhZG1pbiJ9.L7sBru0h4OGDbJwf4KI2rPaIdm6K6MfKN-q7pmx3fGIuI5yoFRXAOtWCq2HoWuNjYptzS_TAhTidH98yKx-q5bVGzVtmOm5_wjN76kN7dJZJP21hujpE3-R6zdkQR_QTvH5Qw3wIY6CsT8jrxzfLRfaCxCpIRavc0oE_iRo3PpPiGtxoUpcdMvdcWvv2pH1YzSAeh2Cof9_I-zD57jaHU2aYeWzCrRyxix69XA6dISTJK5vt3_cPnSR_KQ7N9LTWjjdrxR_dz1HmDxUaVlBKo_v-jG271zAK9HCXD7mcfakb8cwvJK01ErcfKFWLCR4ckJ409_lQtuRwTm_GpIPjWQ",
//       "cache-control": "no-cache",
//       "content-type": "application/json",
//       "pragma": "no-cache",
//       "sec-ch-ua": "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"",
//       "sec-ch-ua-mobile": "?0",
//       "sec-ch-ua-platform": "\"macOS\"",
//       "sec-fetch-dest": "empty",
//       "sec-fetch-mode": "cors",
//       "sec-fetch-site": "same-origin"
//     },
//     "referrerPolicy": "no-referrer",
//     "body": "[{\"id\":\"3efcd037-7756-4a6a-88d2-4ecf7a7786a9\",\"name\":\"customer:access@V01\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"87207e85-2415-430e-890d-6f84c19c642d\",\"name\":\"institution:create\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"47551343-f8f1-4be6-bb58-ed5ea0b82e2b\",\"name\":\"institution:delete\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"a99f7677-4ca7-4725-8645-18c4183ea7ef\",\"name\":\"institution:list\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"9ec134ec-ce15-4ad0-a700-e6c54a8eb9a9\",\"name\":\"institution:update\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"9f1f01a8-1889-4246-a33f-6292808076f9\",\"name\":\"institution:view\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"cd174526-ef5b-4863-a5ab-19762371aecf\",\"name\":\"organization:create\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"1fee634c-c839-4e26-a427-55d8afcc5947\",\"name\":\"organization:delete\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"5f22519b-ea33-403c-9d4e-2ce955f21bf3\",\"name\":\"organization:list\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"130dc23b-ca59-4422-9a4e-0e3592817e5e\",\"name\":\"organization:update\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"},{\"id\":\"5f5ff8f9-fdab-46b8-822a-9aedcdde8ca1\",\"name\":\"organization:view\",\"composite\":false,\"clientRole\":false,\"containerId\":\"300a2b90-8b25-45ac-b20e-7a00c68881ca\"}]",
//     "method": "POST",
//     "mode": "cors",
//     "credentials": "include"
//   });

// fetch("https://keycloak.shapth.local/admin/realms/shapth/groups/b5ee117a-e5e4-4d08-a0f5-896edb893fa3/role-mappings/realm", {
//     "headers": {
//       "accept": "application/json, text/plain, */*",
//       "accept-language": "de-DE,de;q=0.9,en-US;q=0.8,en;q=0.7",
//       "authorization": "Bearer eyJhbGciOiJSUzI1NiIsInR5cCIgOiAiSldUIiwia2lkIiA6ICJIX29FX0J0dzJ2S3NFNHZLSktldFRKS3B2SEl4dFllbFp6YTJGRGtUb0FFIn0.eyJleHAiOjE3MTE3ODI3MjIsImlhdCI6MTcxMTc4MjY2MiwiYXV0aF90aW1lIjoxNzExNzgxMzMxLCJqdGkiOiJmYjlhYTE4Zi0yZDdjLTRiM2MtOGFhNy1mYWEwZTNlMmM0MGYiLCJpc3MiOiJodHRwczovL2F1dGguc2hhcHRoLmxvY2FsL3JlYWxtcy9tYXN0ZXIiLCJzdWIiOiIzODdiOWFmZi05MmRlLTQ2ZGMtOTM3My0yZTI3ZGE0NzFlYTkiLCJ0eXAiOiJCZWFyZXIiLCJhenAiOiJzZWN1cml0eS1hZG1pbi1jb25zb2xlIiwibm9uY2UiOiJkYmFlZmIyNC01NmRjLTQ4ZjEtYWM2Yy04YWRmYzY2NDAxYTQiLCJzZXNzaW9uX3N0YXRlIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiYWNyIjoiMSIsImFsbG93ZWQtb3JpZ2lucyI6WyJodHRwczovL2tleWNsb2FrLnNoYXB0aC5sb2NhbCJdLCJzY29wZSI6Im9wZW5pZCBwcm9maWxlIGVtYWlsIiwic2lkIjoiZjRjYWNmYmUtNzJkMy00N2I2LTk5NjAtZjNhMmYyYWJmNTdmIiwiZW1haWxfdmVyaWZpZWQiOmZhbHNlLCJwcmVmZXJyZWRfdXNlcm5hbWUiOiJhZG1pbiJ9.G-c_gT_RXjj_xiqCJJBUNN02leN3wy9VEj6PHhwYhoWcHAwS4UmzU_Vw3bXFad1Jc_0F505LOpWNC65kWWeMymlF4bXR2KozcKyt6HeBjEAqgcMJWs62L2rnbAyvr4vOkXCubZDd4JOSLCpF8gPHXurR5qARTgYoo-nNw5ebuZMZ-Bk9A_ukmtZa3LICKcRgQbpHu59f7KwvJqEPptVncX4Gfh_GY-70eiBYPFPx47RTZ4hYwKwMKJ-5JBhpXhffDSEAaqM_Xyvrw7ah1tLo2FTUqVp7Qgvhx_XVrPWT0mWNKTPNstwuTd6AE__iXtZGQqye4U5iHKrV6DHvR8bPwg",
//       "cache-control": "no-cache",
//       "content-type": "application/json",
//       "pragma": "no-cache",
//       "sec-ch-ua": "\"Google Chrome\";v=\"123\", \"Not:A-Brand\";v=\"8\", \"Chromium\";v=\"123\"",
//       "sec-ch-ua-mobile": "?0",
//       "sec-ch-ua-platform": "\"macOS\"",
//       "sec-fetch-dest": "empty",
//       "sec-fetch-mode": "cors",
//       "sec-fetch-site": "same-origin"
//     },
//     "referrerPolicy": "no-referrer",
//     "body": "[{\"id\":\"9f1f01a8-1889-4246-a33f-6292808076f9\",\"name\":\"institution:view\"}]",
//     "method": "DELETE",
//     "mode": "cors",
//     "credentials": "include"
//   });

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    GroupMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_group(
        &self,
        ctx: &Context<'_>,
        context: InfraContext,
        name: String,
    ) -> async_graphql::FieldResult<Arc<Group>> {
        let auth_ctx =
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::create()),
            )
            .await?;
        auth_ctx.can_mutate(Some(&context)).await?;
        Ctx(auth_ctx).create(name, context).await
    }

    async fn update_group(
        &self,
        _ctx: &Context<'_>,
        _input: String,
    ) -> async_graphql::FieldResult<Option<Arc<Group>>> {
        unimplemented!()
    }

    async fn remove_groups(
        &self,
        ctx: &Context<'_>,
        ids: Arc<[Arc<Uuid>]>,
    ) -> async_graphql::FieldResult<u64> {
        unimplemented!()
    }
}
