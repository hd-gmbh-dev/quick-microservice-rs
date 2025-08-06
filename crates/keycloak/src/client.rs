use std::{borrow::Cow, collections::HashMap, convert::identity, sync::Arc};

use keycloak::types::{
    ClientScopeRepresentation, ComponentRepresentation, IdentityProviderMapperRepresentation,
    ProtocolMapperRepresentation,
};
pub use keycloak::{
    types::{
        self, AuthenticationExecutionInfoRepresentation, AuthenticationFlowRepresentation,
        AuthenticatorConfigRepresentation, ClientRepresentation, CredentialRepresentation,
        GroupRepresentation, IdentityProviderRepresentation, RealmRepresentation,
        RoleRepresentation, TypeMap, UserRepresentation,
    },
    KeycloakAdmin, KeycloakError, KeycloakTokenSupplier,
};
use serde_json::Value;

use crate::session::{KeycloakSession, KeycloakSessionClient};

pub use crate::config::Config as KeycloakConfig;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ServerInfo {
    #[serde(default)]
    pub realm: Option<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RealmInfo {
    #[serde(default)]
    pub realm: Option<String>,
    #[serde(default)]
    pub public_key: Option<String>,
}

async fn error_check(response: reqwest::Response) -> Result<reqwest::Response, KeycloakError> {
    if !response.status().is_success() {
        let status = response.status().into();
        let text = response.text().await.unwrap_or_default();
        return Err(KeycloakError::HttpFailure {
            status,
            body: serde_json::from_str(&text).ok(),
            text,
        });
    }

    Ok(response)
}

struct Inner {
    url: Arc<str>,
    config: KeycloakConfig,
    client: reqwest::Client,
    session: KeycloakSession,
    admin: KeycloakAdmin<KeycloakSession>,
}

#[derive(Default)]
pub struct KeycloakBuilder {
    no_refresh: bool,
    env_prefix: Option<&'static str>,
}

impl KeycloakBuilder {
    pub fn with_no_refresh(mut self) -> Self {
        self.no_refresh = true;
        self
    }

    pub fn with_env_prefix(mut self, prefix: &'static str) -> Self {
        self.env_prefix = Some(prefix);
        self
    }

    pub async fn build(self) -> anyhow::Result<Keycloak> {
        let mut config_builder = KeycloakConfig::builder();
        if let Some(prefix) = self.env_prefix {
            config_builder = config_builder.with_prefix(prefix);
        }
        let config = config_builder.build()?;
        let refresh_token_enabled = !self.no_refresh;
        let url: Arc<str> = Arc::from(config.address().to_string());
        let username: Arc<str> = Arc::from(config.username().to_string());
        let password: Arc<str> = Arc::from(config.password().to_string());
        let client = reqwest::Client::new();
        let session_client = KeycloakSessionClient::new(config.address(), "master", "admin-cli");
        let session =
            KeycloakSession::new(session_client, &username, &password, refresh_token_enabled)
                .await?;
        Ok(Keycloak {
            inner: Arc::new(Inner {
                url: url.clone(),
                config,
                client: client.clone(),
                session: session.clone(),
                admin: KeycloakAdmin::new(&url, session, client),
            }),
        })
    }
}

#[derive(Clone)]
pub struct Keycloak {
    inner: Arc<Inner>,
}

impl Keycloak {
    pub fn builder() -> KeycloakBuilder {
        KeycloakBuilder::default()
    }

    pub fn http_client(&self) -> &reqwest::Client {
        &self.inner.client
    }

    pub async fn new() -> anyhow::Result<Self> {
        KeycloakBuilder::default().build().await
    }

    pub fn public_url(&self) -> &str {
        self.inner.config.public_url()
    }

    pub fn config(&self) -> &KeycloakConfig {
        &self.inner.config
    }

    pub async fn users(
        &self,
        realm: &str,
        offset: Option<i32>,
        page_size: Option<i32>,
        search_query: Option<String>,
    ) -> Result<Vec<UserRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_get()
            .first(offset)
            .max(page_size)
            .search(search_query)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_realm(
        &self,
        realm_representation: RealmRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .post(realm_representation)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;

        Ok(())
    }

    pub async fn remove_realm(&self, realm: &str) -> Result<(), KeycloakError> {
        self.inner.admin.realm(realm).delete().await.map(|_| ())
    }

    pub async fn remove_group(&self, realm: &str, id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .groups_with_group_id_delete(id)
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_group_by_path(&self, realm: &str, path: &str) -> Result<(), KeycloakError> {
        let group = self
            .inner
            .admin
            .realm(realm)
            .group_by_path_with_path_get(path)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        self.remove_group(realm, group.id.as_deref().unwrap())
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_role(&self, realm: &str, role_name: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_with_role_name_delete(role_name)
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_role_by_id(&self, realm: &str, role_id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_by_id_with_role_id_delete(role_id)
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn realms(&self) -> Result<Vec<String>, KeycloakError> {
        let builder = self
            .inner
            .client
            .get(format!("{}admin/realms", &self.inner.url));
        let response = builder
            .bearer_auth(self.inner.session.get(&self.inner.url).await?)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(error_check(response)
            .await?
            .json::<Vec<ServerInfo>>()
            .await?
            .into_iter()
            .filter_map(|r| {
                if let Some(r) = r.realm {
                    match r.as_str() {
                        "master" => None,
                        _ => Some(r),
                    }
                } else {
                    None
                }
            })
            .collect())
    }

    pub async fn clients(&self, realm: &str) -> Result<Vec<ClientRepresentation>, KeycloakError> {
        let page_offset = 1000;
        let mut offset = 0;
        let mut clients = vec![];
        loop {
            let result = self
                .inner
                .admin
                .realm(realm)
                .clients_get()
                .first(offset)
                .max(page_offset)
                .await
                .map_err(|e| {
                    tracing::error!("{e:#?}");
                    e
                })?;
            if result.is_empty() {
                break;
            }
            offset += page_offset;
            clients.extend(result);
        }
        Ok(clients)
    }

    pub async fn realm_by_name(&self, realm: &str) -> Result<RealmRepresentation, KeycloakError> {
        self.inner.admin.realm(realm).get().await.map_err(|e| {
            tracing::error!("{e:#?}");
            e
        })
    }

    pub async fn update_realm_by_name(
        &self,
        realm: &str,
        rep: RealmRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .put(rep)
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn roles(&self, realm: &str) -> Result<Vec<RoleRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_get()
            .brief_representation(true)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn all_roles(&self, realm: &str) -> Result<Vec<RoleRepresentation>, KeycloakError> {
        let page_offset = 1000;
        let mut offset = 0;
        let mut roles = vec![];
        loop {
            let result = self
                .inner
                .admin
                .realm(realm)
                .roles_get()
                .brief_representation(true)
                .first(offset)
                .max(page_offset)
                .await
                .map_err(|e| {
                    tracing::error!("{e:#?}");
                    e
                })?;
            if result.is_empty() {
                break;
            }
            offset += page_offset;
            roles.extend(result);
        }
        Ok(roles)
    }

    pub async fn realm_role_by_name(
        &self,
        realm: &str,
        role_name: &str,
    ) -> Result<RoleRepresentation, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_with_role_name_get(role_name)
            .await
    }

    pub async fn create_role(
        &self,
        realm: &str,
        rep: RoleRepresentation,
    ) -> Result<Option<String>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_post(rep)
            .await
            .map(|response| response.to_id().map(String::from))
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_group(
        &self,
        realm: &str,
        rep: GroupRepresentation,
    ) -> Result<Option<String>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .groups_post(rep)
            .await
            .map(|response| response.to_id().map(String::from))
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn group_by_path(
        &self,
        realm: &str,
        path: &str,
    ) -> Result<GroupRepresentation, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .group_by_path_with_path_get(path)
            .await
    }

    pub async fn group_by_id_with_children(
        &self,
        realm: &str,
        group_id: &str,
        max: i32,
    ) -> Result<Vec<GroupRepresentation>, KeycloakError> {
        let subgroups = self
            .inner
            .admin
            .realm(realm)
            .groups_with_group_id_children_get(group_id)
            .max(max)
            .await?;

        Ok(subgroups.into_iter().filter(|g| g.id.is_some()).collect())
    }

    pub async fn role_members(
        &self,
        realm: &str,
        role_name: &str,
    ) -> Result<Vec<UserRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .roles_with_role_name_users_get(role_name)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_sub_group_with_id(
        &self,
        realm: &str,
        parent_id: &str,
        rep: GroupRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .groups_with_group_id_children_post(parent_id, rep)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn create_realm_role_mappings_by_group_id(
        &self,
        realm: &str,
        id: &str,
        roles: Vec<RoleRepresentation>,
    ) -> Result<Option<String>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .groups_with_group_id_role_mappings_realm_post(id, roles)
            .await
            .map(|response| response.to_id().map(String::from))
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn user_by_id(
        &self,
        realm: &str,
        id: &str,
    ) -> Result<Option<UserRepresentation>, KeycloakError> {
        Ok(self
            .inner
            .admin
            .realm(realm)
            .users_with_user_id_get(id)
            .user_profile_metadata(true)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
            .ok())
    }

    pub async fn user_by_role(
        &self,
        realm: &str,
        role_name: &str,
    ) -> Result<Option<UserRepresentation>, KeycloakError> {
        Ok(self
            .inner
            .admin
            .realm(realm)
            .roles_with_role_name_users_get(role_name)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
            .ok()
            .and_then(|mut v| {
                if !v.is_empty() {
                    Some(v.remove(0))
                } else {
                    None
                }
            }))
    }

    pub async fn user_by_username(
        &self,
        realm: &str,
        username: String,
    ) -> Result<Option<UserRepresentation>, KeycloakError> {
        Ok(self
            .inner
            .admin
            .realm(realm)
            .users_get()
            .brief_representation(false)
            .exact(true)
            .username(username)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
            .ok()
            .and_then(|mut v| {
                if !v.is_empty() {
                    Some(v.remove(0))
                } else {
                    None
                }
            }))
    }

    pub async fn info(&self, realm: &str) -> Result<RealmInfo, KeycloakError> {
        let builder = self
            .inner
            .client
            .get(format!("{}/realms/{realm}", &self.inner.url));
        let response = builder.send().await?;
        Ok(error_check(response)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?
            .json()
            .await?)
    }

    pub async fn get_client(
        &self,
        realm: &str,
    ) -> Result<Option<ClientRepresentation>, KeycloakError> {
        Ok(self
            .inner
            .admin
            .realm(realm)
            .clients_get()
            .client_id(self.config().client_id().to_owned())
            .search(true)
            .viewable_only(false)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?
            .pop())
    }

    pub async fn get_client_by_id(
        &self,
        realm: &str,
        client_id: &str,
    ) -> Result<Option<ClientRepresentation>, KeycloakError> {
        Ok(self
            .inner
            .admin
            .realm(realm)
            .clients_get()
            .client_id(client_id.to_owned())
            .search(true)
            .viewable_only(false)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?
            .pop())
    }

    pub async fn get_client_service_account(
        &self,
        realm: &str,
        client_uuid: &str,
    ) -> Result<UserRepresentation, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .clients_with_client_uuid_service_account_user_get(client_uuid)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_client(
        &self,
        realm: &str,
        rep: ClientRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .clients_post(rep)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn remove_client(&self, realm: &str, client_id: &str) -> Result<(), KeycloakError> {
        let client = self
            .get_client_by_id(realm, client_id)
            .await?
            .ok_or_else(|| KeycloakError::HttpFailure {
                status: 404,
                body: None,
                text: format!("client with id: '{client_id}' not found"),
            })?;
        self.inner
            .admin
            .realm(realm)
            .clients_with_client_uuid_delete(&client.id.unwrap())
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn remove_client_with_uuid(
        &self,
        realm: &str,
        client_uuid: &str,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .clients_with_client_uuid_delete(client_uuid)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn update_client(
        &self,
        realm: &str,
        id: &str,
        rep: ClientRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .clients_with_client_uuid_put(id, rep)
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn get_client_scopes(
        &self,
        realm: &str,
    ) -> Result<Vec<ClientScopeRepresentation>, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .client_scopes_get()
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn get_client_scope_protocol_mapper(
        &self,
        realm: &str,
        client_scope_id: &str,
        id: &str,
    ) -> Result<ProtocolMapperRepresentation, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .client_scopes_with_client_scope_id_protocol_mappers_models_with_id_get(
                client_scope_id,
                id,
            )
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_client_scope_protocol_mapper(
        &self,
        realm: &str,
        client_scope_id: &str,
        rep: ProtocolMapperRepresentation,
    ) -> Result<Option<String>, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .client_scopes_with_client_scope_id_protocol_mappers_models_post(client_scope_id, rep)
            .await
            .map(|response| response.to_id().map(String::from))
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn update_client_scope_protocol_mapper(
        &self,
        realm: &str,
        client_scope_id: &str,
        id: &str,
        rep: ProtocolMapperRepresentation,
    ) -> Result<(), keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .client_scopes_with_client_scope_id_protocol_mappers_models_with_id_put(
                client_scope_id,
                id,
                rep,
            )
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_client_scope_protocol_mapper(
        &self,
        realm: &str,
        client_scope_id: &str,
        id: &str,
    ) -> Result<(), keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .client_scopes_with_client_scope_id_protocol_mappers_models_with_id_delete(
                client_scope_id,
                id,
            )
            .await
            .map(|_| ())
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_user(
        &self,
        realm: &str,
        user: UserRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_post(user)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn update_password(
        &self,
        realm: &str,
        user_id: &str,
        credential: CredentialRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_reset_password_put(user_id, credential)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn update_user(
        &self,
        realm: &str,
        user_id: &str,
        user: &UserRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_put(user_id, user.to_owned())
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn add_user_to_group(
        &self,
        realm: &str,
        user_id: &str,
        group_id: &str,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_groups_with_group_id_put(user_id, group_id)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn add_user_role(
        &self,
        realm: &str,
        user_id: &str,
        role: RoleRepresentation,
    ) -> Result<Option<String>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_role_mappings_realm_post(user_id, vec![role])
            .await
            .map(|response| response.to_id().map(String::from))
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_user_from_group(
        &self,
        realm: &str,
        user_id: &str,
        group_id: &str,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_groups_with_group_id_delete(user_id, group_id)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn remove_user_from_roles(
        &self,
        realm: &str,
        user_id: &str,
        roles: Vec<RoleRepresentation>,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_role_mappings_realm_delete(user_id, roles)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn remove_user(&self, realm: &str, user_id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_delete(user_id)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn send_verify_email_user(
        &self,
        realm: &str,
        user_id: &str,
        client_id: Option<String>,
        redirect_url: Option<String>,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_send_verify_email_put(user_id)
            .client_id(client_id)
            .redirect_uri(redirect_url)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub async fn send_custom_email_user(
        &self,
        realm: &str,
        user_id: &str,
        redirect_url: Option<String>,
        body: Vec<String>,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_execute_actions_email_put(user_id, body)
            .redirect_uri(redirect_url)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })?;
        Ok(())
    }

    pub fn error_message<'e>(&self, err: &'e KeycloakError) -> Cow<'e, str> {
        match err {
            KeycloakError::ReqwestFailure(err) => Cow::Owned(err.to_string()),
            KeycloakError::HttpFailure { status, body, text } => body
                .as_ref()
                .and_then(|e| {
                    e.error
                        .as_deref()
                        .or(e.error_message.as_deref())
                        .map(Cow::Borrowed)
                })
                .unwrap_or_else(|| {
                    if !text.is_empty() {
                        Cow::Borrowed(text.as_str())
                    } else {
                        Cow::Owned(status.to_string())
                    }
                }),
        }
    }

    pub async fn get_authentication_flows(
        &self,
        realm: &str,
    ) -> Result<Vec<AuthenticationFlowRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .authentication_flows_get()
            .await
    }

    pub async fn copy_authentication_flow(
        &self,
        realm: &str,
        flowalias: &str,
        body: TypeMap<String, String>,
    ) -> Result<(), KeycloakError> {
        let response = self
            .inner
            .admin
            .realm(realm)
            .authentication_flows_with_flow_alias_copy_post(flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Copied successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to copy authentication flow: {e}");
                Err(e)
            }
        }
    }

    pub async fn get_flow_executions(
        &self,
        realm: &str,
        flowalias: &str,
    ) -> Result<Vec<AuthenticationExecutionInfoRepresentation>, KeycloakError> {
        let result = self
            .inner
            .admin
            .realm(realm)
            .authentication_flows_with_flow_alias_executions_get(flowalias)
            .await;
        match result {
            Ok(response) => {
                tracing::info!("Getted flow executions successfully.");
                Ok(response)
            }
            Err(e) => {
                tracing::error!("Failed to get flow executions: {e}");
                Err(e)
            }
        }
    }

    pub async fn remove_execution(&self, realm: &str, id: &str) -> Result<(), KeycloakError> {
        let result = self
            .inner
            .admin
            .realm(realm)
            .authentication_executions_with_execution_id_delete(id)
            .await;
        match result {
            Ok(_) => {
                tracing::info!("Execution deleted successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to delete execution: {e}");
                Err(e)
            }
        }
    }

    pub async fn create_subflow(
        &self,
        realm: &str,
        flowalias: &str,
        body: TypeMap<String, Value>,
    ) -> Result<(), KeycloakError> {
        let response = self
            .inner
            .admin
            .realm(realm)
            .authentication_flows_with_flow_alias_executions_flow_post(flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Subflow created successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to crete subflow: {e}");
                Err(e)
            }
        }
    }

    pub async fn modify_flow_execution(
        &self,
        realm: &str,
        flowalias: &str,
        body: AuthenticationExecutionInfoRepresentation,
    ) -> Result<(), KeycloakError> {
        let response = self
            .inner
            .admin
            .realm(realm)
            .authentication_flows_with_flow_alias_executions_put(flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("PUT flow execution successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed PUT flow execution: {e}");
                Err(e)
            }
        }
    }

    pub async fn create_flow_execution(
        &self,
        realm: &str,
        flowalias: &str,
        body: TypeMap<String, Value>,
    ) -> Result<(), KeycloakError> {
        let response = self
            .inner
            .admin
            .realm(realm)
            .authentication_flows_with_flow_alias_executions_execution_post(flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Execution created successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to crete execution: {e}");
                Err(e)
            }
        }
    }

    pub async fn add_authenticator_config(
        &self,
        realm: &str,
        execution_id: &str,
        body: AuthenticatorConfigRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .authentication_executions_with_execution_id_config_post(execution_id, body)
            .await?;
        Ok(())
    }

    pub async fn find_identity_provider(
        &self,
        realm: &str,
        alias: &str,
    ) -> Result<IdentityProviderRepresentation, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .identity_provider_instances_with_alias_get(alias)
            .await
    }

    pub async fn add_saml_identity_provider(
        &self,
        realm: &str,
        alias: &str,
        metainfo_url: &str,
        entity_id: &str,
    ) -> Result<(), KeycloakError> {
        self.add_saml_identity_provider_custom(realm, alias, metainfo_url, entity_id, identity)
            .await
    }

    pub async fn add_saml_identity_provider_custom<T>(
        &self,
        realm: &str,
        alias: &str,
        metainfo_url: &str,
        entity_id: &str,
        idp_representation_transform: T,
    ) -> Result<(), KeycloakError>
    where
        T: Fn(IdentityProviderRepresentation) -> IdentityProviderRepresentation,
    {
        let idp_config = [
            ("alias", alias),
            ("providerId", "saml"),
            ("fromUrl", metainfo_url),
        ]
        .into_iter()
        .map(|(key, value)| {
            (
                key.to_string(),
                serde_json::Value::String(value.to_string()),
            )
        })
        .collect();

        let imported_config = self
            .inner
            .admin
            .realm(realm)
            .identity_provider_import_config_post(idp_config)
            .await?;
        self.add_saml_identity_provider_from_config(
            realm,
            alias,
            imported_config,
            entity_id,
            idp_representation_transform,
        )
        .await
    }

    pub async fn add_saml_identity_provider_from_config<T>(
        &self,
        realm: &str,
        alias: &str,
        mut idp_config: HashMap<String, String>,
        entity_id: &str,
        idp_representation_transform: T,
    ) -> Result<(), KeycloakError>
    where
        T: Fn(IdentityProviderRepresentation) -> IdentityProviderRepresentation,
    {
        if idp_config.get("nameIDPolicyFormat").map(|s| s.as_str())
            == Some("urn:oasis:names:tc:SAML:2.0:nameid-format:transient")
        {
            idp_config.insert("principalType".into(), "ATTRIBUTE".into());
        }

        idp_config.insert("entityId".into(), entity_id.into());

        let idp_representation = IdentityProviderRepresentation {
            alias: Some(alias.to_string()),
            config: Some(idp_config),
            display_name: Some(alias.to_string()),
            enabled: Some(true),
            provider_id: Some("saml".to_string()),
            ..Default::default()
        };

        let realm = self.inner.admin.realm(realm);
        realm
            .identity_provider_instances_post(idp_representation.clone())
            .await?;

        realm
            .identity_provider_instances_with_alias_put(
                alias,
                idp_representation_transform(idp_representation),
            )
            .await?;

        Ok(())
    }

    pub async fn find_identity_provider_mappers(
        &self,
        realm: &str,
        alias: &str,
    ) -> Result<Vec<IdentityProviderMapperRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .identity_provider_instances_with_alias_mappers_get(alias)
            .await
    }

    pub async fn add_identity_provider_mapper(
        &self,
        realm: &str,
        alias: &str,
        mapper: IdentityProviderMapperRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .identity_provider_instances_with_alias_mappers_post(alias, mapper)
            .await?;
        Ok(())
    }

    pub async fn find_key_providers(
        &self,
        realm: &str,
    ) -> Result<Vec<ComponentRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .components_get()
            .type_("org.keycloak.keys.KeyProvider".to_string())
            .await
    }

    pub async fn add_key_provider(
        &self,
        realm: &str,
        mut key_provider: ComponentRepresentation,
    ) -> Result<(), KeycloakError> {
        key_provider.provider_type = Some("org.keycloak.keys.KeyProvider".into());
        self.inner
            .admin
            .realm(realm)
            .components_post(key_provider)
            .await?;
        Ok(())
    }

    pub async fn delete_component(&self, realm: &str, id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .components_with_id_delete(id)
            .await
            .map(|_| ())
    }

    pub async fn modify_component(
        &self,
        realm: &str,
        id: &str,
        component: ComponentRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .components_with_id_put(id, component)
            .await
            .map(|_| ())
    }

    pub async fn user_groups(
        &self,
        realm: &str,
        user_id: &str,
    ) -> Result<Vec<keycloak::types::GroupRepresentation>, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_groups_get(user_id)
            .brief_representation(true)
            .await
    }

    pub async fn user_roles(
        &self,
        realm: &str,
        user_id: &str,
    ) -> Result<Vec<keycloak::types::RoleRepresentation>, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm(realm)
            .users_with_user_id_role_mappings_realm_get(user_id)
            .await
    }

    pub async fn identity_provider_import_config(
        &self,
        realm: &str,
        provider_id: String,
        file: Vec<u8>,
    ) -> Result<HashMap<String, String>, keycloak::KeycloakError> {
        self.inner
            .admin
            .realm_identity_provider_import_config_post_form(realm, provider_id, file)
            .await
    }

    pub async fn identity_provider_import_saml_config(
        &self,
        realm: &str,
        file: Vec<u8>,
    ) -> Result<HashMap<String, String>, keycloak::KeycloakError> {
        self.identity_provider_import_config(realm, "saml".to_string(), file)
            .await
    }
}

pub fn idp_signature_and_encryption(
    mut idp: IdentityProviderRepresentation,
    principal_attribute: &str,
) -> IdentityProviderRepresentation {
    if let Some(config) = &mut idp.config {
        config.extend(
            [
                ("allowCreate", "true"),
                ("allowedClockSkew", "0"),
                ("artifactResolutionServiceUrl", ""),
                ("attributeConsumingServiceIndex", "0"),
                ("attributeConsumingServiceName", ""),
                ("authnContextComparisonType", "exact"),
                ("backchannelSupported", "false"),
                ("caseSensitiveOriginalUsername", "false"),
                ("encryptionAlgorithm", "RSA-OAEP"),
                ("forceAuthn", "false"),
                ("guiOrder", ""),
                ("principalAttribute", principal_attribute),
                ("principalType", "ATTRIBUTE"),
                ("sendClientIdOnLogout", "false"),
                ("sendIdTokenOnLogout", "true"),
                ("signSpMetadata", "false"),
                ("signatureAlgorithm", "RSA_SHA256"),
                ("singleLogoutServiceUrl", ""),
                ("syncMode", "LEGACY"),
                ("useMetadataDescriptorUrl", "true"),
                ("validateSignature", "true"),
                ("wantAssertionsEncrypted", "true"),
                ("wantAssertionsSigned", "true"),
                ("wantAuthnRequestsSigned", "true"),
                ("xmlSigKeyInfoKeyNameTransformer", "KEY_ID"),
            ]
            .map(|(k, v)| (k.to_string(), v.to_string())),
        );
    }
    idp
}
