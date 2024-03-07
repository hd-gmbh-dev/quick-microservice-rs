use std::collections::HashMap;
use std::sync::Arc;
// use async_graphql::Context;
// use async_graphql::FieldResult;

use async_graphql::{ErrorExtensions, FieldResult, ResultExt};
use qm_entity::error::EntityError;

// use qm_entity::FromGraphQLContext;
// use qm_entity::UserId;
use qm_entity::{err, Create};
use qm_keycloak::CredentialRepresentation;
use qm_keycloak::Keycloak;
use qm_keycloak::KeycloakError;
use qm_keycloak::UserRepresentation;
use qm_mongodb::bson::Uuid;
use qm_mongodb::DB;

use crate::model::CreateUserInput;
use crate::model::User;
use crate::model::UserInput;
use crate::model::{RequiredUserAction, UserData, UserDetails};

// use crate::model::UserInput;

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

pub trait UserDB {
    fn collection(&self) -> &str {
        DEFAULT_COLLECTION
    }
    fn user_db(&self) -> &qm_mongodb::DB;
    fn users(&self) -> qm_entity::Collection<User> {
        let collection = self.collection();
        qm_entity::Collection(self.user_db().get().collection::<User>(collection))
    }
}

impl<T> UserDB for T
where
    T: AsRef<DB>,
{
    fn user_db(&self) -> &DB {
        self.as_ref()
    }
}

use crate::schema::auth::AuthCtx;
use crate::schema::RelatedAccessLevel;
use crate::schema::RelatedAuth;
use crate::schema::RelatedPermission;
use crate::schema::RelatedResource;
use crate::schema::RelatedStorage;

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
    user: UserInput,
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
    pub async fn create(&self, input: CreateUserInput) -> FieldResult<Arc<User>> {
        let CreateUserInput {
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

        if let Some(cache) = self.0.store.cache() {
            if let Some(group_id) = cache.user().get_group_id(&group).await {
                keycloak
                    .add_user_to_group(realm, &user_id, &group_id)
                    .await?;
            }
            if let Some(role) = cache.user().get_role(&access).await {
                keycloak
                    .add_user_role(
                        realm,
                        &k_user
                            .id
                            .clone()
                            .expect("Keycloak must have returned a user"),
                        role,
                    )
                    .await?;
            }
        } else {
            unimplemented!()
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

        if let Some(cache) = self.0.store.cache() {
            cache
                .user()
                .new_user(self.0.store.redis().as_ref(), k_user, db_user.clone())
                .await?;
        }

        Ok(db_user)
    }
}
