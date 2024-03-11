use keycloak::KeycloakError;
use keycloak::KeycloakTokenSupplier;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Builder;
use tokio::sync::RwLock;
use tokio::task::LocalSet;

#[derive(Debug, Clone)]
pub enum KeycloakSessionError {
    ReqwestFailure(Arc<reqwest::Error>),
    HttpFailure { status: u16, text: Arc<str> },
    Decode(Arc<serde_json::Error>),
}

impl From<reqwest::Error> for KeycloakSessionError {
    fn from(value: reqwest::Error) -> Self {
        KeycloakSessionError::ReqwestFailure(Arc::new(value))
    }
}

impl std::error::Error for KeycloakSessionError {}
impl std::fmt::Display for KeycloakSessionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            KeycloakSessionError::HttpFailure { text, .. } => {
                writeln!(f, "keycloak error: {}", text.as_ref())
            }
            KeycloakSessionError::ReqwestFailure(e) => e.fmt(f),
            KeycloakSessionError::Decode(e) => e.fmt(f),
        }
    }
}

async fn error(response: reqwest::Response) -> Result<reqwest::Response, KeycloakSessionError> {
    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await;
        return match text {
            Ok(text) => Err(KeycloakSessionError::HttpFailure {
                status: status.as_u16(),
                text: Arc::from(text),
            }),
            Err(e) => Err(KeycloakSessionError::ReqwestFailure(Arc::new(e))),
        };
    }

    Ok(response)
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ParsedAccessToken {
    exp: usize,
    //:1677048774,
    iat: usize,
    //:1677048714,
    // auth_time: usize, //:1677047319,
    jti: Option<String>,
    //:"48ef7bc9-1a42-4e4f-b136-5fd74d4d6033",
    iss: Option<String>,
    //:"https://id.qm-example.local/realms/master",
    sub: Option<String>,
    //:"fe487690-8c65-4106-95a5-5b1dbb8e6bbd",
    typ: Option<String>,
    //:"Bearer",
    azp: Option<String>,
    //:"security-admin-console",
    nonce: Option<String>,
    //:"86e7e8a2-5af5-4fed-80e7-1da412e51070",
    session_state: Option<String>,
    //:"cdfaa367-5c30-4142-b31a-f770073e2051",
    acr: Option<String>,
    //:"0",
    allowed: Option<Vec<String>>,
    //origins":["https://keycloak.qm-example.local"],
    scope: Option<String>,
    //:"openid profile email",
    sid: Option<String>,
    //:"cdfaa367-5c30-4142-b31a-f770073e2051",
    email_verified: bool,
    //:false,
    preferred_username: Option<String>, //:"admin"
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct KeycloakSessionToken {
    access_token: Arc<str>,
    expires_in: usize,
    #[serde(rename = "not-before-policy")]
    not_before_policy: Option<usize>,
    refresh_expires_in: Option<usize>,
    refresh_token: Arc<str>,
    scope: String,
    session_state: Option<String>,
    token_type: String,
    #[serde(skip)]
    parsed_access_token: Option<ParsedAccessToken>,
    #[serde(skip)]
    client_token: Option<Arc<str>>,
}

impl KeycloakSessionToken {
    fn parse_access_token(mut token: Self) -> Self {
        use base64::engine::{general_purpose::STANDARD_NO_PAD, Engine};
        if let Some(parsed_access_token) = token
            .access_token
            .split('.')
            .nth(1)
            .and_then(|s| {
                STANDARD_NO_PAD
                    .decode(s)
                    .map_err(|e| {
                        log::error!("{e:#?}");
                        e
                    })
                    .ok()
            })
            .and_then(|b| {
                serde_json::from_slice::<ParsedAccessToken>(&b)
                    .map_err(|e| {
                        log::error!("{e:#?}");
                        e
                    })
                    .ok()
            })
        {
            token.parsed_access_token = Some(parsed_access_token);
        }
        token.client_token = Some(Arc::from(format!(
            "{} {}",
            &token.token_type, &token.access_token
        )));
        token
    }
}

struct KeycloakSessionClientInner {
    url: Arc<str>,
    realm: Arc<str>,
    client_id: Arc<str>,
    client: reqwest::Client,
}

#[derive(Clone)]
pub struct KeycloakSessionClient {
    inner: Arc<KeycloakSessionClientInner>,
}

impl KeycloakSessionClient {
    pub fn new<T>(url: T, realm: T, client_id: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            inner: Arc::new(KeycloakSessionClientInner {
                url: Arc::from(url.into()),
                realm: Arc::from(realm.into()),
                client_id: Arc::from(client_id.into()),
                client: reqwest::Client::default(),
            }),
        }
    }

    async fn acquire(
        &self,
        username: &str,
        password: &str,
    ) -> Result<KeycloakSessionToken, KeycloakSessionError> {
        let url = self.inner.url.as_ref();
        let realm = self.inner.realm.as_ref();
        let client_id = self.inner.client_id.as_ref();
        let result = error(
            self.inner
                .client
                .post(&format!(
                    "{url}/realms/{realm}/protocol/openid-connect/token",
                ))
                .form(&serde_json::json!({
                    "username": username,
                    "password": password,
                    "client_id": client_id,
                    "grant_type": "password"
                }))
                .send()
                .await?,
        )
        .await?
        .json::<serde_json::Value>()
        .await?;
        log::debug!(
            "Acquire result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );
        serde_json::from_value(result).map_err(|err| KeycloakSessionError::Decode(Arc::new(err)))
    }

    async fn refresh(
        &self,
        refresh_token: &str,
    ) -> Result<KeycloakSessionToken, KeycloakSessionError> {
        let url = self.inner.url.as_ref();
        let realm = self.inner.realm.as_ref();
        let client_id = self.inner.client_id.as_ref();
        let result = error(
            self.inner
                .client
                .post(&format!(
                    "{url}/realms/{realm}/protocol/openid-connect/token",
                ))
                .form(&serde_json::json!({
                    "grant_type": "refresh_token",
                    "refresh_token": refresh_token,
                    "client_id": client_id,
                }))
                .send()
                .await?,
        )
        .await?
        .json::<serde_json::Value>()
        .await?;
        log::debug!(
            "Refresh result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );
        serde_json::from_value(result).map_err(|err| KeycloakSessionError::Decode(Arc::new(err)))
    }
}

async fn try_refresh(
    keycloak: &KeycloakSessionClient,
    refresh_token: &str,
    username: &str,
    password: &str,
) -> Result<KeycloakSessionToken, KeycloakSessionError> {
    log::debug!("refresh session for user {username}");
    match keycloak.refresh(refresh_token).await {
        Ok(token) => Ok(KeycloakSessionToken::parse_access_token(token)),
        Err(err) => {
            if let KeycloakSessionError::HttpFailure { status, .. } = &err {
                if *status == 400 {
                    log::error!("refresh token expired try to acquire new token with credentials");
                    log::error!("{:#?}", err);
                    keycloak
                        .acquire(username, password)
                        .await
                        .map(KeycloakSessionToken::parse_access_token)
                } else {
                    Err(err)
                }
            } else {
                Err(err)
            }
        }
    }
}

struct KeycloakSessionInner {
    username: Arc<str>,
    password: Arc<str>,
    token: RwLock<KeycloakSessionToken>,
    stop_tx: tokio::sync::watch::Sender<bool>,
}

#[derive(Clone)]
pub struct KeycloakSession {
    inner: Arc<KeycloakSessionInner>,
}

impl Drop for KeycloakSession {
    fn drop(&mut self) {
        self.inner.stop_tx.send(false).ok();
    }
}

impl KeycloakSession {
    pub async fn new(
        keycloak: KeycloakSessionClient,
        username: &str,
        password: &str,
        refresh_enabled: bool,
    ) -> anyhow::Result<Self> {
        let token = keycloak
            .acquire(username, password)
            .await
            .map(KeycloakSessionToken::parse_access_token)?;
        let username: Arc<str> = Arc::from(username.to_string());
        let password: Arc<str> = Arc::from(password.to_string());
        let (stop_tx, stop_signal) = tokio::sync::watch::channel(true);
        let result = KeycloakSession {
            inner: Arc::new(KeycloakSessionInner {
                username,
                password,
                token: RwLock::new(token),
                stop_tx,
            }),
        };
        if refresh_enabled {
            let keycloak = keycloak.clone();
            let session = result.clone();
            std::thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();
                let local = LocalSet::new();
                local.spawn_local(async move {
                    let username = &session.inner.username;
                    let password = &session.inner.password;
                    loop {
                        let expires_in = session.inner.token.read().await.expires_in;
                        let refresh_future = async {
                            tokio::time::sleep(Duration::from_secs(
                                expires_in
                                    .checked_sub(30)
                                    .ok_or(anyhow::anyhow!("unable to calculate refresh timeout"))?
                                    as u64,
                            ))
                            .await;
                            let next_token = async {
                                try_refresh(
                                    &keycloak,
                                    &session.inner.token.read().await.refresh_token,
                                    username,
                                    password,
                                )
                                .await
                            }
                            .await;
                            match next_token {
                                Ok(next_token) => {
                                    *session.inner.token.write().await = next_token;
                                }
                                Err(err) => {
                                    log::error!("{err:#?}");
                                    std::process::exit(1)
                                }
                            }
                            anyhow::Ok(true)
                        };
                        let stop_future = async {
                            let mut stop_signal = stop_signal.clone();
                            stop_signal.changed().await?;
                            let result = *stop_signal.borrow_and_update();
                            anyhow::Ok(result)
                        };
                        tokio::select! {
                            _ = refresh_future => {}
                            is_logged_in = stop_future => {
                                if !is_logged_in.unwrap_or(false) {
                                    break
                                }
                            }
                        }
                    }
                    log::debug!("session ends for user {username}");
                    anyhow::Ok(())
                });
                rt.block_on(local);
            });
        }
        Ok(result)
    }

    pub fn stop(&self) -> anyhow::Result<()> {
        log::debug!("stop session for {}", self.inner.username);
        self.inner.stop_tx.send(false)?;
        Ok(())
    }

    pub async fn access_token(&self) -> Arc<str> {
        self.inner.token.read().await.access_token.clone()
    }

    pub async fn token(&self) -> Arc<str> {
        self.inner
            .token
            .read()
            .await
            .client_token
            .as_ref()
            .unwrap()
            .clone()
    }
}

#[async_trait::async_trait]
impl KeycloakTokenSupplier for KeycloakSession {
    async fn get(&self, _url: &str) -> Result<String, KeycloakError> {
        Ok(self.inner.token.read().await.access_token.to_string())
    }
}
