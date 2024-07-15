use async_graphql::ComplexObject;
use async_graphql::{Context, ErrorExtensions, FieldResult, Object, ResultExt};
use qm_entity::exerr;
use qm_entity::ids::InfraContext;

use qm_entity::model::ListFilter;
use qm_keycloak::RoleRepresentation;
use qm_role::{Access, AccessLevel};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use crate::cache::CacheDB;
use crate::config::SchemaConfig;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::ApiClient;
use crate::model::ApiClientList;
use crate::model::{CreateApiClientInput, Customer};
use crate::model::{CreateApiClientPayload, Institution, Organization, ApiClientDetails, /* OrganizationUnit */};
use crate::model::{Group, RequiredApiClientAction, Role, ApiClientGroup};
use qm_entity::err;
use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_entity::IsAdmin;
use qm_keycloak::CredentialRepresentation;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::ClientRepresentation;
use sqlx::types::Uuid;

use crate::schema::auth::AuthCtx;
use crate::schema::RelatedAuth;
use crate::schema::RelatedPermission;
use crate::schema::RelatedResource;
use crate::schema::RelatedStorage;

pub trait KeycloakClient {
    fn keycloak(&self) -> &Keycloak;
}

impl<T> KeycloakClient for T
where
    T: AsRef<Keycloak>,
{
    fn keycloak(&self) -> &Keycloak {
        self.as_ref()
    }
}

fn set_attributes(attributes: HashMap<&str, Option<String>>, u: &mut ClientRepresentation) {
    if u.attributes.is_none() {
        u.attributes = Some(HashMap::new());
    }

    if let Some(a) = u.attributes.as_mut() {
        // Loop all attributes possible
        for (key, value) in attributes.into_iter() {
            if let Some(v) = value {
                a.insert(
                    key.to_string(),
                    v.split(',').map(|v| v.trim().to_string()).collect(),
                );
            } else {
                a.remove(key);
            }
        }
    }
}

pub async fn create_keycloak_api_client(
    _realm: &str,
    _keycloak: &Keycloak,
    _api_client: CreateApiClientInput,
) -> FieldResult<ClientRepresentation> {
    // let username = api_client.username;
    // let email = Some(api_client.email);
    // let first_name = Some(api_client.firstname);
    // let last_name = Some(api_client.lastname);
    // let enabled = api_client.enabled;

    // let mut keycloak_user: ClientRepresentation = ClientRepresentation {
    //     access: None,
    //     attributes: None,
    //     client_consents: None,
    //     client_roles: None,
    //     created_timestamp: None,
    //     credentials: None,
    //     disableable_credential_types: None,
    //     email: email.clone(),
    //     email_verified: None,
    //     enabled,
    //     federated_identities: None,
    //     federation_link: None,
    //     first_name,
    //     groups: None,
    //     id: None,
    //     last_name,
    //     not_before: None,
    //     origin: None,
    //     realm_roles: None,
    //     // Some(vec!["UPDATE_PASSWORD".to_string()]),
    //     required_actions: api_client
    //         .required_actions
    //         .as_ref()
    //         .map(|actions| actions.iter().map(|action| action.to_string()).collect()),
    //     self_: None,
    //     service_account_client_id: None,
    //     username: Some(username.clone()),
    //     ..Default::default()
    // };

    // set_attributes(
    //     HashMap::from([
    //         ("phone", api_client.phone),
    //         ("salutation", api_client.salutation),
    //         ("room-number", api_client.room_number),
    //         ("job-title", api_client.job_title),
    //     ]),
    //     &mut keycloak_user,
    // );

    // // Set the credential
    // keycloak_user.credentials = Some(vec![CredentialRepresentation {
    //     created_date: None,
    //     credential_data: None,
    //     id: None,
    //     priority: None,
    //     secret_data: None,
    //     temporary: api_client
    //         .required_actions
    //         .as_ref()
    //         .map(|actions| actions.contains(&RequiredApiClientAction::UpdatePassword)),
    //     type_: Some("password".to_string()),
    //     user_label: None,
    //     value: Some(api_client.password),
    //     ..Default::default()
    // }]);

    // let result = keycloak.create_user(realm, keycloak_user).await;
    // let exists = match result {
    //     Ok(_) => Ok(false),
    //     Err(err) => match err {
    //         KeycloakError::ReqwestFailure(err) => {
    //             log::error!("KeycloakError::ReqwestFailure: unable to get api_client");
    //             Err(EntityError::from(err))
    //         }
    //         KeycloakError::HttpFailure {
    //             status: 409,
    //             body: Some(e),
    //             ..
    //         } => {
    //             let err_msg = e
    //                 .error_message
    //                 .ok_or(anyhow::format_err!("Unknown Error"))?;
    //             if err_msg.contains("username") {
    //                 // conflicting_name("Benutzername", "username")
    //                 err!(fields_conflict::<ApiClient>(&username, &["username"][..]))
    //             } else if err_msg.contains("email") {
    //                 err!(fields_conflict::<ApiClient>(&username, &["email"][..]))
    //             } else {
    //                 err!(internal())
    //             }
    //         }
    //         KeycloakError::HttpFailure {
    //             status: 400,
    //             body: Some(e),
    //             ..
    //         } => {
    //             let mut err_type = String::new();
    //             let err_msg = match e.error_message {
    //                 Some(e) => {
    //                     let mut err = String::new();
    //                     if e.eq("Password policy not met") {
    //                         err_type.push_str("password_policy");
    //                         err.push_str("Passwortrichtlinie nicht erfüllt");
    //                     }

    //                     err
    //                 }
    //                 None => "Unknown error".to_string(),
    //             };

    //             if err_type.is_empty() {
    //                 err_type.push_str("unknown");
    //             }

    //             // bad_request_name(&err_type, &err_msg)
    //             err!(bad_request(err_type, err_msg))
    //         }
    //         KeycloakError::HttpFailure { .. } => {
    //             log::error!("KeycloakError::HttpFailure: unable to get api_client");
    //             err!(internal())
    //         }
    //     },
    // };

    // if let Err(err) = exists {
    //     return Err(err.extend());
    // }

    // keycloak
    //     .user_by_username(realm, username.clone())
    //     .await?
    //     .ok_or(EntityError::not_found_by_field::<ApiClient>(
    //         "username", &username,
    //     ))
    //     .extend()
    unimplemented!()
}

#[ComplexObject]
impl ApiClientDetails {
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

    // async fn organization_unit(&self, ctx: &Context<'_>) -> Option<Arc<OrganizationUnit>> {
    //     let cache = ctx.data::<CacheDB>().ok();
    //     if cache.is_none() {
    //         log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
    //         return None;
    //     }
    //     let cache = cache.unwrap();
    //     if let Some(id) = self
    //         .context
    //         .as_ref()
    //         .and_then(InfraContext::organization_unit_id)
    //     {
    //         return cache.organization_unit_by_id(&id).await;
    //     }
    //     None
    // }

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

    async fn roles(&self, _ctx: &Context<'_>) -> Option<Arc<[Arc<Role>]>> {
        // let cache = ctx.data::<CacheDB>().ok();
        // if cache.is_none() {
        //     log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
        //     return None;
        // }
        // let cache = cache.unwrap();
        // cache.roles_by_user_id(&self.api_client.id).await
        unimplemented!()
    }

    // async fn groups(&self, _ctx: &Context<'_>) -> Option<Arc<[ApiClientGroup]>> {
    //     // let cache = ctx.data::<CacheDB>().ok();
    //     // if cache.is_none() {
    //     //     log::warn!("qm::customer::cache::CacheDB is not installed in schema context");
    //     //     return None;
    //     // }
    //     // let cache = cache.unwrap();
    //     // cache.groups_by_user_id(&self.api_client.id).await
    //     unimplemented!()
    // }
}

pub struct Ctx<'a, Auth, Store, Resource, Permission>(
    pub &'a AuthCtx<'a, Auth, Store, Resource, Permission>,
)
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,

    Resource: RelatedResource,
    Permission: RelatedPermission;
impl<'a, Auth, Store, Resource, Permission> Ctx<'a, Auth, Store, Resource, Permission>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,

    Resource: RelatedResource,
    Permission: RelatedPermission,
{
    // pub async fn list(
    //     &self,
    //     mut context: Option<InfraContext>,
    //     filter: Option<ListFilter>,
    // ) -> async_graphql::FieldResult<ApiClientList> {
    //     context = self.0.enforce_current_context(context).await?;
    //     Ok(self.0.store.cache_db().user_list(context, filter).await)
    // }

    // pub async fn by_id(&self, id: &str) -> Option<ApiClientDetails> {
    //     self.0.store.cache_db().user_details_by_id(id).await
    // }

    // pub async fn create(&self, input: CreateApiClientPayload) -> FieldResult<Arc<ApiClient>> {
    //     let CreateApiClientPayload {
    //         api_client: mut user_input,
    //         access,
    //         group_id,
    //         context: _,
    //     } = input;
    //     let mut conflict_fields = Vec::new();
    //     let user_exists_by_username = self
    //         .0
    //         .store
    //         .cache_db()
    //         .user_by_username(&user_input.username)
    //         .await;
    //     if user_exists_by_username.is_some() {
    //         conflict_fields.push("username");
    //     }
    //     let user_exists_by_email = self
    //         .0
    //         .store
    //         .cache_db()
    //         .user_by_email(&user_input.username)
    //         .await;
    //     if user_exists_by_email.is_some() {
    //         conflict_fields.push("email");
    //     }

    //     if !conflict_fields.is_empty() {
    //         return err!(fields_conflict::<ApiClient>(
    //             user_input.username.as_str(),
    //             &conflict_fields[..]
    //         )
    //         .extend());
    //     }

    //     if user_input.enabled.is_none() {
    //         user_input.enabled = Some(true);
    //     }

    //     let keycloak = self.0.store.keycloak();
    //     let realm = keycloak.config().realm();
    //     let k_user = create_keycloak_user(realm, keycloak, user_input.clone()).await?;
    //     let user_id = k_user.id.as_ref().unwrap().clone();

    //     if user_input
    //         .required_actions
    //         .map(|actions| actions.contains(&RequiredApiClientAction::VerifyEmail))
    //         .unwrap_or_default()
    //     {
    //         if let Err(err) = keycloak.send_verify_email_user(realm, &user_id, None).await {
    //             log::warn!(
    //                 "Verification email could not be sent: {}",
    //                 keycloak.error_message(&err)
    //             );
    //         }
    //     }

    //     let user_uuid = Uuid::parse_str(&user_id).map_err(|err| {
    //         log::error!("Unable to parse api_client id to Uuid: {err:#?}");
    //         EntityError::Internal
    //     })?;
    //     let mut user_groups = vec![];
    //     let cache = self.0.store.cache_db();
    //     if let Some(group_id) = group_id.as_ref() {
    //         if let Some(group) = cache.group_by_id(group_id).await {
    //             log::info!(
    //                 "add api_client {} to group {group:#?}",
    //                 user_input.username.as_str()
    //             );
    //             keycloak
    //                 .add_user_to_group(realm, &user_id, &group.id)
    //                 .await?;
    //             user_groups.push(group);
    //         }
    //     }
    //     let mut user_roles = vec![];
    //     if let Some(access) = access.as_ref() {
    //         if let Some(role) = cache.role_by_name(access).await {
    //             keycloak
    //                 .add_user_role(
    //                     realm,
    //                     &user_id,
    //                     RoleRepresentation {
    //                         id: Some(role.id.to_string()),
    //                         name: Some(role.name.to_string()),
    //                         ..Default::default()
    //                     },
    //                 )
    //                 .await?;
    //             user_roles.push(role);
    //         }
    //     }
    //     let api_client = Arc::new(ApiClient {
    //         id: Arc::from(user_uuid.to_string()),
    //         username: Arc::from(user_input.username),
    //         firstname: Arc::from(user_input.firstname),
    //         lastname: Arc::from(user_input.lastname),
    //         email: Arc::from(user_input.email),
    //         enabled: user_input.enabled.unwrap(),
    //     });
    //     cache.api_client().new_user(api_client.clone()).await;
    //     Ok(api_client)
    // }

    // pub async fn remove(&self, ids: Arc<[Arc<str>]>) -> EntityResult<u64> {
    //     let keycloak = self.0.store.keycloak();
    //     let mut user_ids = Vec::default();
    //     for id in ids.iter() {
    //         match keycloak
    //             .remove_user(keycloak.config().realm(), id.as_ref())
    //             .await
    //         {
    //             Ok(_) => user_ids.push(id.as_ref()),
    //             Err(err) => {
    //                 log::error!("{err:#?}");
    //             }
    //         }
    //     }
    //     if !user_ids.is_empty() {
    //         return Ok(user_ids.len() as u64);
    //     }
    //     Ok(0)
    // }
}

pub struct ApiClientQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for ApiClientQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    ApiClientQueryRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,

    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    // async fn me(&self, ctx: &Context<'_>) -> async_graphql::FieldResult<Option<ApiClientDetails>> {
    //     let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new(ctx)
    //         .await
    //         .extend()?;
    //     let id = *auth_ctx.auth.user_id().unwrap();
    //     Ok(Ctx(&auth_ctx).by_id(&id.to_string()).await)
    // }

    // async fn user_by_id(
    //     &self,
    //     ctx: &Context<'_>,
    //     id: Uuid,
    // ) -> async_graphql::FieldResult<Option<ApiClientDetails>> {
    //     Ok(Ctx(
    //         &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
    //             ctx,
    //             (Resource::api_client(), Permission::view()),
    //         )
    //         .await
    //         .extend()?,
    //     )
    //     .by_id(&id.to_string())
    //     .await)
    // }

    async fn api_clients(
        &self,
        ctx: &Context<'_>,
        context: Option<InfraContext>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<ApiClientList> {
        // Ctx(
        //     &AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
        //         ctx,
        //         (Resource::api_client(), Permission::list()),
        //     )
        //     .await?,
        // )
        // .list(context, filter)
        // .await
        // .extend()
        unimplemented!()
    }
}

pub struct ApiClientMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, Resource, Permission, BuiltInGroup> Default
    for ApiClientMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, Resource, Permission, BuiltInGroup>
    ApiClientMutationRoot<Auth, Store, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<Resource, Permission>,
    Store: RelatedStorage,

    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_api_client(
        &self,
        ctx: &Context<'_>,
        access_level: AccessLevel,
        group_id: Option<String>,
        input: CreateApiClientInput,
        context: Option<InfraContext>,
    ) -> async_graphql::FieldResult<Arc<ApiClient>> {
        unimplemented!()
        // let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
        //     ctx,
        //     (Resource::api_client(), Permission::create()),
        // )
        // .await?;
        // if !SchemaConfig::new(ctx).allow_multiple_admin_users() && access_level.is_admin() {
        //     return err!(not_allowed("creating multiple admin users").extend());
        // }
        // if let Some(group_id) = group_id.as_ref() {
        //     let group = auth_ctx
        //         .store
        //         .cache_db()
        //         .group_detail_by_id(group_id)
        //         .await
        //         .ok_or(EntityError::not_found_by_id::<Group>(group_id))
        //         .extend()?;
        //     if group
        //         .allowed_access_levels
        //         .as_ref()
        //         .map(|lvls| !lvls.iter().any(|l| l == &access_level))
        //         .unwrap_or(false)
        //     {
        //         return err!(not_allowed("invalid access level for selected group").extend());
        //     }

        //     let group_roles = auth_ctx
        //         .store
        //         .cache_db()
        //         .roles_by_group_id(group_id)
        //         .await
        //         .ok_or(EntityError::not_found_by_id::<Group>(group_id))
        //         .extend()?;

        //     for role in group_roles.iter() {
        //         if let Ok(role) =
        //             qm_role::Role::<Resource, Permission>::from_str(role.name.as_ref())
        //         {
        //             if role.ty.is_admin() {
        //                 return err!(not_allowed("invalid group selected").extend());
        //             }
        //             if !auth_ctx.is_admin && !auth_ctx.auth.has_role_object(&role) {
        //                 return err!(not_allowed("invalid group selected").extend());
        //             }
        //         } else {
        //             return err!(internal().extend());
        //         }
        //     }
        // }
        // let user_access_level_u32 = auth_ctx.auth.as_number();
        // let access_level_u32 = access_level.as_number();
        // let access = if let Some(context) = context.as_ref() {
        //     let access = Access::new(access_level).with_fmt_id(Some(&context));
        //     if (user_access_level_u32 < access_level_u32)
        //         || (user_access_level_u32 == access_level_u32 && !auth_ctx.auth.has_access(&access))
        //     {
        //         return err!(unauthorized(&auth_ctx.auth).extend());
        //     }
        //     access
        // } else {
        //     let own_access_level_id = auth_ctx
        //         .auth
        //         .session_access()
        //         .ok_or(EntityError::unauthorized(&auth_ctx.auth))?;
        //     if own_access_level_id.id().is_some() {
        //         return err!(unauthorized(&auth_ctx.auth).extend());
        //     }
        //     if access_level.id_required() {
        //         return err!(bad_request(
        //             "InfraContext",
        //             "'context' is required for specified access level"
        //         )
        //         .extend());
        //     }
        //     Access::new(access_level)
        // };
        // Ctx(&auth_ctx)
        //     .create(CreateApiClientPayload {
        //         access: Some(access.to_string()),
        //         api_client: input,
        //         group_id,
        //         context,
        //     })
        //     .await
        //     .extend()
    }

    // async fn update_user(
    //     &self,
    //     _ctx: &Context<'_>,
    //     _input: String,
    // ) -> async_graphql::FieldResult<Option<Arc<ApiClient>>> {
    //     // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
    //     //     .await?
    //     //     .update(&input)
    //     //     .await?)
    //     unimplemented!()
    // }

    // async fn remove_users(
    //     &self,
    //     ctx: &Context<'_>,
    //     ids: Arc<[Arc<Uuid>]>,
    // ) -> async_graphql::FieldResult<u64> {
    //     let auth_ctx = AuthCtx::<'_, Auth, Store, Resource, Permission>::new_with_role(
    //         ctx,
    //         (Resource::api_client(), Permission::delete()),
    //     )
    //     .await?;
    //     let active_user_id = auth_ctx
    //         .auth
    //         .user_id()
    //         .ok_or(EntityError::unauthorized(&auth_ctx.auth))?;
    //     if ids.iter().any(|id| id.as_ref() == active_user_id) {
    //         return exerr!(bad_request("ApiClient", "ApiClient cannot remove himself"));
    //     }
    //     let cache = auth_ctx.store.cache_db();
    //     let mut user_ids = Vec::with_capacity(ids.len());
    //     for id in ids.as_ref().iter() {
    //         let id = id.to_string();
    //         let api_client = cache.user_details_by_id(&id).await;
    //         if let Some(api_client) = api_client {
    //             if api_client.is_admin() {
    //                 return exerr!(unauthorized(&auth_ctx.auth));
    //             }
    //             auth_ctx.can_mutate(api_client.context.as_ref()).await.extend()?;
    //             user_ids.push(Arc::from(id));
    //         } else {
    //             return exerr!(not_found_by_id::<ApiClient>(id.to_string()));
    //         }
    //     }
    //     Ctx(&auth_ctx).remove(Arc::from(user_ids)).await.extend()
    // }
}
