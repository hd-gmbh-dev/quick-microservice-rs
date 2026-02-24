use keycloak::KeycloakError;
use keycloak::KeycloakTokenSupplier;
use std::{sync::Arc, time::Duration};
use tokio::runtime::Builder;
use tokio::sync::RwLock;
use tokio::task::LocalSet;

/// Errors for Keycloak session operations.
#[derive(Debug, Clone)]
pub enum KeycloakSessionError {
    /// Request failure.
    ReqwestFailure(Arc<reqwest::Error>),
    /// HTTP failure with status and text.
    HttpFailure {
        /// HTTP status code.
        status: u16,
        /// Response text.
        text: Arc<str>,
    },
    /// Decode failure.
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

/// Parsed access token from Keycloak (equivalent to KeycloakAccessTokenResponse).
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ParsedAccessToken {
    /// Expiration time (unix seconds).
    exp: usize,
    /// Issued at time (unix seconds).
    iat: usize,
    /// JWT ID.
    jti: Option<String>,
    /// Issuer.
    iss: Option<String>,
    /// Subject (user ID).
    sub: Option<String>,
    /// Token type.
    typ: Option<String>,
    /// Authorized party (client ID).
    azp: Option<String>,
    /// Nonce.
    nonce: Option<String>,
    /// Session state.
    session_state: Option<String>,
    /// Authentication context class reference.
    acr: Option<String>,
    /// Allowed actions.
    allowed: Option<Vec<String>>,
    /// Scope.
    scope: Option<String>,
    /// Session ID.
    sid: Option<String>,
    /// Whether email is verified.
    #[serde(default)]
    email_verified: bool,
    /// Preferred username.
    preferred_username: Option<String>,
}

/// Session token from Keycloak.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct KeycloakSessionToken {
    /// Access token.
    access_token: Arc<str>,
    /// Time until expiration.
    expires_in: usize,
    /// Not before policy.
    #[serde(rename = "not-before-policy")]
    not_before_policy: Option<usize>,
    /// Time until refresh token expires.
    refresh_expires_in: Option<usize>,
    /// Refresh token.
    refresh_token: Arc<str>,
    /// Scope.
    scope: String,
    /// Session state.
    session_state: Option<String>,
    /// Token type.
    token_type: String,
    /// Parsed access token.
    #[serde(skip)]
    parsed_access_token: Option<ParsedAccessToken>,
    /// Client token (type + access_token).
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
                        tracing::error!("{e:#?}");
                        e
                    })
                    .ok()
            })
            .and_then(|b| {
                serde_json::from_slice::<ParsedAccessToken>(&b)
                    .map_err(|e| {
                        tracing::error!("{e:#?}");
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
/// Keycloak session client.
pub struct KeycloakSessionClient {
    inner: Arc<KeycloakSessionClientInner>,
}

impl KeycloakSessionClient {
    /// Creates a new KeycloakSessionClient.
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
                .post(format!(
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
        tracing::debug!(
            "Acquire result: {}",
            serde_json::to_string_pretty(&result).unwrap()
        );
        serde_json::from_value(result).map_err(|err| KeycloakSessionError::Decode(Arc::new(err)))
    }

    async fn acquire_with_secret(
        &self,
        secret: &str,
    ) -> Result<KeycloakSessionToken, KeycloakSessionError> {
        let url = self.inner.url.as_ref();
        let realm = self.inner.realm.as_ref();
        let client_id = self.inner.client_id.as_ref();

        // curl \
        // -d "client_id=R09219E08" \
        // -d "client_secret=wBdk1Z3GXm2YXRrtbgcEMLrVsbL8jjwn" \
        // -d "grant_type=client_credentials" \
        // "https://id.shapth.homenet/realms/shapth/protocol/openid-connect/token"
        let result = error(
            self.inner
                .client
                .post(format!(
                    "{url}/realms/{realm}/protocol/openid-connect/token",
                ))
                .form(&serde_json::json!({
                    "client_id": client_id,
                    "client_secret": secret,
                    "grant_type": "client_credentials"
                }))
                .send()
                .await?,
        )
        .await?
        .json::<serde_json::Value>()
        .await?;
        tracing::debug!(
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
                .post(format!(
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
        tracing::debug!(
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
    tracing::debug!("refresh session for user {username}");
    match keycloak.refresh(refresh_token).await {
        Ok(token) => Ok(KeycloakSessionToken::parse_access_token(token)),
        Err(err) => {
            if let KeycloakSessionError::HttpFailure { status, .. } = &err {
                if *status == 400 {
                    tracing::error!(
                        "refresh token expired try to acquire new token with credentials"
                    );
                    tracing::error!("{:#?}", err);
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

async fn try_refresh_with_secret(
    keycloak: &KeycloakSessionClient,
    refresh_token: &str,
    secret: &str,
) -> Result<KeycloakSessionToken, KeycloakSessionError> {
    tracing::debug!("refresh session for api client");
    match keycloak.refresh(refresh_token).await {
        Ok(token) => Ok(KeycloakSessionToken::parse_access_token(token)),
        Err(err) => {
            if let KeycloakSessionError::HttpFailure { status, .. } = &err {
                if *status == 400 {
                    tracing::error!(
                        "refresh token expired try to acquire new token with credentials"
                    );
                    tracing::error!("{:#?}", err);
                    keycloak
                        .acquire_with_secret(secret)
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
/// Keycloak session for user authentication.
pub struct KeycloakSession {
    inner: Arc<KeycloakSessionInner>,
}

impl Drop for KeycloakSession {
    fn drop(&mut self) {
        self.inner.stop_tx.send(false).ok();
    }
}

impl KeycloakSession {
    /// Creates a new Keycloak session.
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
                        let (expires_in, refresh_expires_in) = async {
                            let r = session.inner.token.read().await;
                            (r.expires_in, r.refresh_expires_in)
                        }
                        .await;
                        tracing::debug!("{expires_in} -> {refresh_expires_in:#?}");
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
                                    tracing::error!("{err:#?}");
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
                            result = refresh_future => {
                                match result {
                                    Ok(_) => {},
                                    Err(_) => {
                                        tracing::debug!("acquire new session");
                                        match keycloak
                                            .acquire(username, password)
                                            .await
                                            .map(KeycloakSessionToken::parse_access_token) {
                                            Ok(next_token) => {
                                                *session.inner.token.write().await = next_token;
                                            },
                                            Err(err) => {
                                                tracing::error!("{err:#?}");
                                                std::process::exit(1)
                                            }
                                        }
                                    }
                                }
                            }
                            is_logged_in = stop_future => {
                                if !is_logged_in.unwrap_or(false) {
                                    break
                                }
                            }
                        }
                    }
                    tracing::debug!("session ends for user {username}");
                    anyhow::Ok(())
                });
                rt.block_on(local);
            });
        }
        Ok(result)
    }

    /// Stops the session.
    pub fn stop(&self) -> anyhow::Result<()> {
        tracing::debug!("stop session for {}", self.inner.username);
        self.inner.stop_tx.send(false)?;
        Ok(())
    }

    /// Gets the access token.
    pub async fn access_token(&self) -> Arc<str> {
        self.inner.token.read().await.access_token.clone()
    }

    /// Gets the token.
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

struct KeycloakApiClientSessionInner {
    secret: Arc<str>,
    token: RwLock<KeycloakSessionToken>,
    stop_tx: tokio::sync::watch::Sender<bool>,
}

#[derive(Clone)]
/// Keycloak API client session for service accounts.
pub struct KeycloakApiClientSession {
    inner: Arc<KeycloakApiClientSessionInner>,
}

impl Drop for KeycloakApiClientSession {
    fn drop(&mut self) {
        self.inner.stop_tx.send(false).ok();
    }
}

impl KeycloakApiClientSession {
    /// Creates a new KeycloakApiClientSession.
    pub async fn new(
        keycloak: KeycloakSessionClient,
        secret: &str,
        refresh_enabled: bool,
    ) -> anyhow::Result<Self> {
        let token = keycloak
            .acquire_with_secret(secret)
            .await
            .map(KeycloakSessionToken::parse_access_token)?;
        let secret: Arc<str> = Arc::from(secret.to_string());
        let (stop_tx, stop_signal) = tokio::sync::watch::channel(true);
        let result = KeycloakApiClientSession {
            inner: Arc::new(KeycloakApiClientSessionInner {
                secret,
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
                    let secret = &session.inner.secret;
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
                                try_refresh_with_secret(
                                    &keycloak,
                                    &session.inner.token.read().await.refresh_token,
                                    secret,
                                )
                                .await
                            }
                            .await;
                            match next_token {
                                Ok(next_token) => {
                                    *session.inner.token.write().await = next_token;
                                }
                                Err(err) => {
                                    tracing::error!("{err:#?}");
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
                            result = refresh_future => {
                                match result {
                                    Ok(_) => {},
                                    Err(_) => {
                                        tracing::debug!("acquire new session");
                                        match keycloak
                                            .acquire_with_secret(secret)
                                            .await
                                            .map(KeycloakSessionToken::parse_access_token) {
                                            Ok(next_token) => {
                                                *session.inner.token.write().await = next_token;
                                            },
                                            Err(err) => {
                                                tracing::error!("{err:#?}");
                                                std::process::exit(1)
                                            }
                                        }
                                    }
                                }
                            }
                            is_logged_in = stop_future => {
                                if !is_logged_in.unwrap_or(false) {
                                    break
                                }
                            }
                        }
                    }
                    tracing::debug!("session ends for api client");
                    anyhow::Ok(())
                });
                rt.block_on(local);
            });
        }
        Ok(result)
    }

    /// Stops the session.
    pub fn stop(&self) -> anyhow::Result<()> {
        tracing::debug!("stop session for {}", self.inner.secret);
        self.inner.stop_tx.send(false)?;
        Ok(())
    }

    /// Gets the access token.
    pub async fn access_token(&self) -> Arc<str> {
        self.inner.token.read().await.access_token.clone()
    }

    /// Gets the token.
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
impl KeycloakTokenSupplier for KeycloakApiClientSession {
    async fn get(&self, _url: &str) -> Result<String, KeycloakError> {
        Ok(self.inner.token.read().await.access_token.to_string())
    }
}
