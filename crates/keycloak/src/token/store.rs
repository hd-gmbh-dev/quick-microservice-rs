use std::{collections::HashMap, sync::Arc};

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{Algorithm, Header};
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use crate::{
    token::jwt::{Claims, Jwt},
    RealmInfo,
};

use super::{
    config::Config,
    jwt::{LogoutClaims, PartialClaims},
};
pub trait JwtConfig {
    fn address(&self) -> &str;
    fn public_url(&self) -> &str;
}

impl JwtConfig for Config {
    fn address(&self) -> &str {
        Config::address(self)
    }
    fn public_url(&self) -> &str {
        Config::public_url(self)
    }
}

impl JwtConfig for crate::config::Config {
    fn address(&self) -> &str {
        crate::config::Config::address(self)
    }
    fn public_url(&self) -> &str {
        crate::config::Config::public_url(self)
    }
}

struct Inner {
    url: Arc<str>,
    public_url: Arc<str>,
    client: Client,
    keys: RwLock<HashMap<String, Jwt>>,
}

#[derive(Clone)]
pub struct JwtStore {
    inner: Arc<Inner>,
}

impl JwtStore {
    pub fn new(config: &impl JwtConfig) -> Self {
        let client = reqwest::Client::new();
        let url = Arc::from(config.address());
        let public_url = Arc::from(config.public_url());
        Self {
            inner: Arc::new(Inner {
                url,
                client,
                public_url,
                keys: Default::default(),
            }),
        }
    }

    pub async fn info(&self, realm: &str) -> anyhow::Result<RealmInfo> {
        let builder = self
            .inner
            .client
            .get(format!("{}/realms/{realm}", &self.inner.url));
        Ok(builder.send().await?.json().await?)
    }

    async fn get_jwt_from_realm(
        &self,
        realm: &str,
        client_id: &str,
        header: Header,
    ) -> anyhow::Result<Jwt> {
        let info = self.info(realm).await?;
        let public_key = info
            .public_key
            .ok_or(anyhow::anyhow!("unable to get public key"))?;
        match (header.alg, header.kid) {
            (Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512, Some(kid)) => {
                Ok(Jwt::new(header.alg, kid, &public_key, client_id)?)
            }
            _ => anyhow::bail!("Invalid token"),
        }
    }

    async fn get_jwt_from_partial_claims(&self, token: &str) -> anyhow::Result<Jwt> {
        let token_header = jsonwebtoken::decode_header(token)?;
        let mut iter = token.split('.');
        if let Some(payload) = iter.nth(1) {
            let partial_claims = URL_SAFE_NO_PAD
                .decode(payload)
                .map_err(|e| {
                    tracing::error!("Base64 Decode error: {e:#?}");
                    e
                })
                .ok()
                .and_then(|v| {
                    serde_json::from_slice::<PartialClaims>(&v)
                        .map_err(|e| {
                            tracing::error!("Serde JSON Deserialize Error {e:#?}");
                            e
                        })
                        .ok()
                })
                .ok_or(anyhow::anyhow!("Invalid token"))?;

            let public_url = self.inner.public_url.as_ref();
            let issuer_url = &partial_claims.iss[0..public_url.len()];
            if partial_claims.iss.len() > public_url.len() && public_url == issuer_url {
                let s = partial_claims
                    .iss
                    .replace(self.inner.public_url.as_ref(), "");
                let mut u = s.rsplit('/');
                let realm = u.next().ok_or(anyhow::anyhow!("Invalid token"))?;
                let client_id = &partial_claims.azp;
                return self
                    .get_jwt_from_realm(realm, client_id, token_header)
                    .await;
            } else {
                return Err(anyhow::anyhow!("Invalid token - issuer does not match - public_url '{public_url}' issuer url '{issuer_url}'"));
            }
        }
        Err(anyhow::anyhow!("Invalid token"))
    }

    pub async fn decode(&self, token: &str) -> anyhow::Result<Claims> {
        self.decode_custom(token).await
    }

    pub async fn decode_custom<C: DeserializeOwned>(&self, token: &str) -> anyhow::Result<C> {
        let token_header = jsonwebtoken::decode_header(token)?;
        let kid = token_header
            .kid
            .as_ref()
            .ok_or(anyhow::anyhow!("Invalid token"))?;
        {
            if let Some(key) = self.inner.keys.read().await.get(kid) {
                return key.decode_custom(token);
            }
        }
        let jwt = self.get_jwt_from_partial_claims(token).await?;
        let claims = jwt.decode_custom(token)?;
        self.inner.keys.write().await.insert(jwt.kid.clone(), jwt);
        Ok(claims)
    }

    pub async fn decode_logout_token(&self, token: &str) -> anyhow::Result<LogoutClaims> {
        self.decode_logout_token_custom(token).await
    }

    pub async fn decode_logout_token_custom<C: DeserializeOwned>(
        &self,
        token: &str,
    ) -> anyhow::Result<C> {
        let token_header = jsonwebtoken::decode_header(token)?;
        let kid = token_header
            .kid
            .as_ref()
            .ok_or(anyhow::anyhow!("Invalid token"))?;
        {
            if let Some(key) = self.inner.keys.read().await.get(kid) {
                return key.decode_logout_token_custom(token);
            }
        }
        let jwt = self.get_jwt_from_partial_claims(token).await?;
        let logout_claims = jwt.decode_logout_token_custom(token)?;
        self.inner.keys.write().await.insert(jwt.kid.clone(), jwt);
        Ok(logout_claims)
    }
}
