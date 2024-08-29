use std::{borrow::Cow, sync::Arc};

pub use keycloak::{
    types::{
        AuthenticationExecutionInfoRepresentation, AuthenticationFlowRepresentation,
        AuthenticatorConfigRepresentation, ClientRepresentation, CredentialRepresentation,
        GroupRepresentation, RealmRepresentation, RoleRepresentation, TypeMap, UserRepresentation,
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
            .realm_users_get(
                realm,
                None,
                None,
                None,
                None,
                None,
                offset,
                None,
                None,
                None,
                None,
                page_size,
                None,
                search_query,
                None,
            )
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
        self.inner.admin.realm_delete(realm).await
    }

    pub async fn remove_group(&self, realm: &str, id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm_groups_with_group_id_delete(realm, id)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_group_by_path(&self, realm: &str, path: &str) -> Result<(), KeycloakError> {
        let group = self
            .inner
            .admin
            .realm_group_by_path_with_path_get(realm, path)
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
            .realm_roles_with_role_name_delete(realm, role_name)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn remove_role_by_id(&self, realm: &str, role_id: &str) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm_roles_by_id_with_role_id_delete(realm, role_id)
            .await
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
                .realm_clients_get(
                    realm,
                    None,
                    Some(offset),
                    Some(page_offset),
                    None,
                    None,
                    None,
                )
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
        self.inner.admin.realm_get(realm).await.map_err(|e| {
            tracing::error!("{e:#?}");
            e
        })
    }

    pub async fn update_realm_by_name(
        &self,
        realm: &str,
        rep: RealmRepresentation,
    ) -> Result<(), KeycloakError> {
        self.inner.admin.realm_put(realm, rep).await.map_err(|e| {
            tracing::error!("{e:#?}");
            e
        })
    }

    pub async fn roles(&self, realm: &str) -> Result<Vec<RoleRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm_roles_get(realm, Some(true), None, None, None)
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
                .realm_roles_get(realm, Some(true), Some(offset), Some(page_offset), None)
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
            .realm_roles_with_role_name_get(realm, role_name)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn create_role(
        &self,
        realm: &str,
        rep: RoleRepresentation,
    ) -> Result<Option<String>, KeycloakError> {
        self.inner
            .admin
            .realm_roles_post(realm, rep)
            .await
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
            .realm_groups_post(realm, rep)
            .await
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
            .realm_group_by_path_with_path_get(realm, path)
            .await
            .map_err(|e| {
                tracing::error!("{e:#?}");
                e
            })
    }

    pub async fn role_members(
        &self,
        realm: &str,
        role_name: &str,
    ) -> Result<Vec<UserRepresentation>, KeycloakError> {
        self.inner
            .admin
            .realm_roles_with_role_name_users_get(realm, role_name, None, None)
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
            .realm_groups_with_group_id_children_post(realm, parent_id, rep)
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
            .realm_groups_with_group_id_role_mappings_realm_post(realm, id, roles)
            .await
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
            .realm_users_with_user_id_get(realm, id, Some(true))
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
            .realm_roles_with_role_name_users_get(realm, role_name, None, None)
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
            .realm_users_get(
                realm,
                Some(false),
                None,
                None,
                None,
                Some(true),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(username),
            )
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
            .realm_clients_get(
                realm,
                Some("spa".to_owned()),
                None,
                None,
                None,
                Some(true),
                Some(false),
            )
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
            .realm_clients_get(
                realm,
                Some(client_id.to_owned()),
                None,
                None,
                None,
                Some(true),
                Some(false),
            )
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
            .realm_clients_with_client_uuid_service_account_user_get(realm, client_uuid)
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
            .realm_clients_post(realm, rep)
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
            .realm_clients_with_client_uuid_delete(realm, &client.id.unwrap())
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
            .realm_clients_with_client_uuid_delete(realm, client_uuid)
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
            .realm_clients_with_client_uuid_put(realm, id, rep)
            .await
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
            .realm_users_post(realm, user)
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
            .realm_users_with_user_id_reset_password_put(realm, user_id, credential)
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
            .realm_users_with_user_id_put(realm, user_id, user.to_owned())
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
            .realm_users_with_user_id_groups_with_group_id_put(realm, user_id, group_id)
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
            .realm_users_with_user_id_role_mappings_realm_post(realm, user_id, vec![role])
            .await
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
            .realm_users_with_user_id_groups_with_group_id_delete(realm, user_id, group_id)
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
            .realm_users_with_user_id_delete(realm, user_id)
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
        redirect_url: Option<String>,
    ) -> Result<(), KeycloakError> {
        self.inner
            .admin
            .realm_users_with_user_id_send_verify_email_put(
                realm,
                user_id,
                None,
                None,
                redirect_url,
            )
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
            .realm_users_with_user_id_execute_actions_email_put(
                realm,
                user_id,
                None,
                None,
                redirect_url,
                body,
            )
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
        self.inner.admin.realm_authentication_flows_get(realm).await
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
            .realm_authentication_flows_with_flow_alias_copy_post(realm, flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Copied successfully.");
                Ok(())
            }
            Err(e) => {
                tracing::error!("Failed to copy authentication flow: {:?}", e);
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
            .realm_authentication_flows_with_flow_alias_executions_get(realm, flowalias)
            .await;
        match result {
            Ok(response) => {
                tracing::info!("Getted flow executions successfully.");
                return Ok(response);
            }
            Err(e) => {
                tracing::error!("Failed to get flow executions: {:?}", e);
                return Err(e);
            }
        }
    }

    pub async fn remove_execution(&self, realm: &str, id: &str) -> Result<(), KeycloakError> {
        let result = self
            .inner
            .admin
            .realm_authentication_executions_with_execution_id_delete(realm, id)
            .await;
        match result {
            Ok(_) => {
                tracing::info!("Execution deleted successfully.");
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Failed to delete execution: {:?}", e);
                return Err(e);
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
            .realm_authentication_flows_with_flow_alias_executions_flow_post(realm, flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Subflow created successfully.");
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Failed to crete subflow: {:?}", e);
                return Err(e);
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
            .realm_authentication_flows_with_flow_alias_executions_put(realm, flowalias, body)
            .await;
        match response {
            Ok(_) => {
                tracing::info!("PUT flow execution successfully.");
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Failed PUT flow execution: {:?}", e);
                return Err(e);
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
            .realm_authentication_flows_with_flow_alias_executions_execution_post(
                realm, flowalias, body,
            )
            .await;
        match response {
            Ok(_) => {
                tracing::info!("Execution created successfully.");
                return Ok(());
            }
            Err(e) => {
                tracing::error!("Failed to crete execution: {:?}", e);
                return Err(e);
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
            .realm_authentication_executions_with_execution_id_config_post(
                realm,
                execution_id,
                body,
            )
            .await?;
        Ok(())
    }
}
