use async_graphql::{Context, ErrorExtensions, FieldResult, Object, ResultExt};
use qm_entity::ctx::ContextFilterInput;
use qm_entity::list::ListCtx;
use qm_entity::model::ListFilter;
use qm_role::Access;
use std::collections::HashMap;
use std::sync::Arc;

use qm_entity::error::EntityError;
use qm_entity::error::EntityResult;
use qm_entity::{err, Create};
use qm_keycloak::CredentialRepresentation;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::UserRepresentation;
use qm_mongodb::bson::Uuid;
use qm_mongodb::DB;

use crate::config::SchemaConfig;
use crate::groups::RelatedBuiltInGroup;
use crate::marker::Marker;
use crate::model::User;
use crate::model::{CreateUserInput, CreateUserPayload, UserList};
use crate::model::{RequiredUserAction, UserData, UserDetails};
use crate::schema::auth::AuthCtx;
use crate::schema::RelatedAccessLevel;
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

pub const DEFAULT_COLLECTION: &str = "users";

pub trait UserDB: AsRef<DB> {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn users(&self) -> qm_entity::Collection<User> {
        let collection = self.collection();
        qm_entity::Collection(self.as_ref().get().collection::<User>(collection))
    }
}

fn set_attributes(attributes: HashMap<&str, Option<String>>, u: &mut UserRepresentation) {
    if u.attributes.is_none() {
        u.attributes = Some(HashMap::new());
    }

    if let Some(a) = u.attributes.as_mut() {
        // Loop all attributes possible
        for (key, value) in attributes.into_iter() {
            if let Some(v) = value {
                a.insert(
                    key.to_string(),
                    serde_json::Value::Array(
                        v.split(',')
                            .map(|v| v.trim())
                            .map(|v| serde_json::Value::String(v.to_string()))
                            .collect(),
                    ),
                );
            } else {
                a.remove(key);
            }
        }
    }
}

pub async fn create_keycloak_user(
    realm: &str,
    keycloak: &Keycloak,
    user: CreateUserInput,
) -> FieldResult<UserRepresentation> {
    let username = user.username;
    let email = Some(user.email);
    let first_name = Some(user.firstname);
    let last_name = Some(user.lastname);
    let enabled = user.enabled;

    let mut keycloak_user: UserRepresentation = UserRepresentation {
        access: None,
        attributes: None,
        client_consents: None,
        client_roles: None,
        created_timestamp: None,
        credentials: None,
        disableable_credential_types: None,
        email: email.clone(),
        email_verified: None,
        enabled,
        federated_identities: None,
        federation_link: None,
        first_name,
        groups: None,
        id: None,
        last_name,
        not_before: None,
        origin: None,
        realm_roles: None,
        // Some(vec!["UPDATE_PASSWORD".to_string()]),
        required_actions: user
            .required_actions
            .as_ref()
            .map(|actions| actions.iter().map(|action| action.to_string()).collect()),
        self_: None,
        service_account_client_id: None,
        username: Some(username.clone()),
    };

    set_attributes(
        HashMap::from([
            ("phone", user.phone),
            ("salutation", user.salutation),
            ("room-number", user.room_number),
            ("job-title", user.job_title),
        ]),
        &mut keycloak_user,
    );

    // Set the credential
    keycloak_user.credentials = Some(vec![CredentialRepresentation {
        created_date: None,
        credential_data: None,
        id: None,
        priority: None,
        secret_data: None,
        temporary: user
            .required_actions
            .as_ref()
            .map(|actions| actions.contains(&RequiredUserAction::UpdatePassword)),
        type_: Some("password".to_string()),
        user_label: None,
        value: Some(user.password),
    }]);

    let result = keycloak.create_user(realm, keycloak_user).await;
    let exists = match result {
        Ok(_) => Ok(false),
        Err(err) => match err {
            KeycloakError::ReqwestFailure(err) => {
                log::error!("KeycloakError::ReqwestFailure: unable to get user");
                Err(EntityError::from(err))
            }
            KeycloakError::HttpFailure {
                status: 409,
                body: Some(e),
                ..
            } => {
                let err_msg = e
                    .error_message
                    .ok_or(anyhow::format_err!("Unknown Error"))?;
                if err_msg.contains("username") {
                    // conflicting_name("Benutzername", "username")
                    err!(fields_conflict::<User>(&username, &["username"][..]))
                } else if err_msg.contains("email") {
                    err!(fields_conflict::<User>(&username, &["email"][..]))
                } else {
                    err!(internal())
                }
            }
            KeycloakError::HttpFailure {
                status: 400,
                body: Some(e),
                ..
            } => {
                let mut err_type = String::new();
                let err_msg = match e.error_message {
                    Some(e) => {
                        let mut err = String::new();
                        if e.eq("Password policy not met") {
                            err_type.push_str("password_policy");
                            err.push_str("Passwortrichtlinie nicht erfüllt");
                        }

                        err
                    }
                    None => "Unknown error".to_string(),
                };

                if err_type.is_empty() {
                    err_type.push_str("unknown");
                }

                // bad_request_name(&err_type, &err_msg)
                err!(bad_request(err_type, err_msg))
            }
            KeycloakError::HttpFailure { .. } => {
                log::error!("KeycloakError::HttpFailure: unable to get user");
                err!(internal())
            }
        },
    };

    if let Err(err) = exists {
        return Err(err.extend());
    }

    keycloak
        .user_by_username(realm, username.clone())
        .await?
        .ok_or(EntityError::not_found_by_field::<User>(
            "username", &username,
        ))
        .extend()
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
        context: Option<ContextFilterInput>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<UserList> {
        ListCtx::new(self.0.store.users())
            .with_query(
                self.0
                    .build_context_query(context.as_ref())
                    .await
                    .extend()?,
            )
            .list(filter)
            .await
            .extend()
    }

    pub async fn by_id(&self, id: Uuid) -> Option<Arc<User>> {
        self.0.store.cache().user().db_user_by_uid(&id).await
    }

    pub async fn create(&self, input: CreateUserPayload) -> FieldResult<Arc<User>> {
        let CreateUserPayload {
            user: mut user_input,
            access,
            group,
            context,
        } = input;
        let mut conflict_fields = Vec::new();

        let user_exists_by_username = self
            .0
            .store
            .users()
            .by_field("username", &user_input.username)
            .await?;
        if user_exists_by_username.is_some() {
            conflict_fields.push("username");
        }

        let user_exists_by_email = self
            .0
            .store
            .users()
            .by_field("email", &user_input.username)
            .await?;
        if user_exists_by_email.is_some() {
            conflict_fields.push("email");
        }

        if !conflict_fields.is_empty() {
            return err!(fields_conflict::<User>(
                user_input.username.as_str(),
                &conflict_fields[..]
            )
            .extend());
        }

        if user_input.enabled.is_none() {
            user_input.enabled = Some(true);
        }

        let keycloak = self.0.store.keycloak();
        let realm = keycloak.config().realm();
        let k_user = create_keycloak_user(realm, keycloak, user_input.clone()).await?;
        let user_id = k_user.id.as_ref().unwrap().clone();
        let user_uuid = Uuid::parse_str(&user_id).map_err(|err| {
            log::error!("Unable to parse user id to Uuid: {err:#?}");
            EntityError::Internal
        })?;

        let cache = self.0.store.cache();
        if let Some(group_id) = cache.user().get_group_id(&group).await {
            keycloak
                .add_user_to_group(realm, &user_id, &group_id)
                .await?;
        }
        if let Some(role) = cache.user().get_role(&access).await {
            keycloak.add_user_role(realm, &user_id, role).await?;
        }

        let db_user = Arc::new(
            self.0
                .store
                .users()
                .save(
                    UserData {
                        owner: context.into(),
                        groups: vec![group],
                        access,
                        details: UserDetails {
                            email: Arc::from(user_input.email),
                            firstname: Arc::from(user_input.firstname),
                            lastname: Arc::from(user_input.lastname),
                            username: Arc::from(user_input.username),
                            user_id: Arc::new(user_uuid),
                            job_title: user_input.job_title.map(Arc::from),
                            phone: user_input.phone.map(Arc::from),
                            salutation: user_input.salutation.map(Arc::from),
                            enabled: user_input.enabled.unwrap_or(false),
                        },
                    }
                    .create(&self.0.auth)?,
                )
                .await?,
        );

        self.0
            .store
            .cache()
            .user()
            .new_user(self.0.store.redis().as_ref(), k_user, db_user.clone())
            .await?;
        Ok(db_user)
    }

    pub async fn remove(&self, ids: Arc<[Arc<Uuid>]>) -> EntityResult<u64> {
        let keycloak = self.0.store.keycloak();
        let mut user_ids = Vec::default();
        for id in ids.iter() {
            let user_id = id.as_ref().to_string();
            match keycloak
                .remove_user(keycloak.config().realm(), &user_id)
                .await
            {
                Ok(_) => user_ids.push(user_id),
                Err(err) => {
                    log::error!("{err:#?}");
                }
            }
        }
        if !user_ids.is_empty() {
            let result = self.0.store.users().remove_all("_id", &user_ids).await?;
            self.0
                .store
                .cache()
                .user()
                .reload_users(keycloak, self.0.store, Some(self.0.store.redis().as_ref()))
                .await?;
            return Ok(result.deleted_count);
        }
        Ok(0)
    }
}

pub struct UserQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for UserQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    UserQueryRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn user_by_id(
        &self,
        ctx: &Context<'_>,
        id: Uuid,
    ) -> async_graphql::FieldResult<Option<Arc<User>>> {
        Ok(Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::view()),
            )
            .await
            .extend()?,
        )
        .by_id(id)
        .await)
    }

    async fn users(
        &self,
        ctx: &Context<'_>,
        context: Option<ContextFilterInput>,
        filter: Option<ListFilter>,
    ) -> async_graphql::FieldResult<UserList> {
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::list()),
            )
            .await?,
        )
        .list(context, filter)
        .await
        .extend()
    }
}

pub struct UserMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> {
    _marker: Marker<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>,
}

impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup> Default
    for UserMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
{
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

#[Object]
impl<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
    UserMutationRoot<Auth, Store, AccessLevel, Resource, Permission, BuiltInGroup>
where
    Auth: RelatedAuth<AccessLevel, Resource, Permission>,
    Store: RelatedStorage,
    AccessLevel: RelatedAccessLevel,
    Resource: RelatedResource,
    Permission: RelatedPermission,
    BuiltInGroup: RelatedBuiltInGroup,
{
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        access_level: AccessLevel,
        built_in_group: Option<BuiltInGroup>,
        custom_group: Option<String>, // TODO: implement custom_groups in Cache and schema
        input: CreateUserInput,
        context: ContextFilterInput,
    ) -> async_graphql::FieldResult<Arc<User>> {
        if access_level.is_admin() && !SchemaConfig::new(ctx).allow_multiple_admin_users() {
            return err!(not_allowed("creating multiple admin users").extend());
        }
        let auth_ctx =
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::create()),
            )
            .await?;
        let access_level_u32 = access_level.as_number();
        let access = Access::new(access_level).with_fmt_id(Some(&context));
        if auth_ctx.auth.as_number() < access_level_u32
            || (auth_ctx.auth.as_number() == access_level_u32 && !auth_ctx.auth.has_access(&access))
        {
            return err!(unauthorized(&auth_ctx.auth).extend());
        }
        Ctx(auth_ctx)
            .create(CreateUserPayload {
                access: access.to_string(),
                user: input,
                group: custom_group.unwrap_or_else(|| {
                    built_in_group
                        .map(|v| v.as_ref().to_string())
                        .unwrap_or_default()
                }),
                context,
            })
            .await
            .extend()
    }

    async fn update_user(
        &self,
        _ctx: &Context<'_>,
        _input: String,
    ) -> async_graphql::FieldResult<Option<Arc<User>>> {
        // Ok(InstitutionCtx::<Auth, Store>::from_graphql(ctx)
        //     .await?
        //     .update(&input)
        //     .await?)
        unimplemented!()
    }

    async fn remove_users(
        &self,
        ctx: &Context<'_>,
        ids: Arc<[Arc<Uuid>]>,
    ) -> async_graphql::FieldResult<u64> {
        // TODO: check if user is allowed to remove users by owner field in customer cache
        Ctx(
            AuthCtx::<'_, Auth, Store, AccessLevel, Resource, Permission>::new_with_role(
                ctx,
                (Resource::user(), Permission::delete()),
            )
            .await?,
        )
        .remove(ids)
        .await
        .extend()
    }
}
