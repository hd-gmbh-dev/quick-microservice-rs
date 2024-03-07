use std::collections::HashMap;
use std::sync::Arc;

use base64::engine::{general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::Algorithm;
use jsonwebtoken::Header;
use reqwest::Client;
use tokio::sync::RwLock;

use crate::token::jwt::Claims;
use crate::token::jwt::Jwt;
use crate::RealmInfo;

use super::jwt::LogoutClaims;
use super::jwt::PartialClaims;

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
    pub fn new(config: &crate::KeycloakConfig) -> Self {
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

    async fn get_jwt_from_realm(&self, realm: &str, header: Header) -> anyhow::Result<Jwt> {
        let info = self.info(realm).await?;
        let public_key = info
            .public_key
            .ok_or(anyhow::anyhow!("unable to get public key"))?;
        match (header.alg, header.kid) {
            (Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512, Some(kid)) => {
                Ok(Jwt::new(header.alg, kid, &public_key)?)
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
                    log::error!("Base64 Decode error: {e:#?}");
                    e
                })
                .ok()
                .and_then(|v| {
                    serde_json::from_slice::<PartialClaims>(&v)
                        .map_err(|e| {
                            log::error!("Serde JSON Deserialize Error {e:#?}");
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
                return self.get_jwt_from_realm(realm, token_header).await;
            } else {
                return Err(anyhow::anyhow!("Invalid token - issuer does not match"));
            }
        }
        Err(anyhow::anyhow!("Invalid token"))
    }
    pub async fn decode(&self, token: &str) -> anyhow::Result<Claims> {
        let token_header = jsonwebtoken::decode_header(token)?;
        let kid = token_header
            .kid
            .as_ref()
            .ok_or(anyhow::anyhow!("Invalid token"))?;
        {
            if let Some(key) = self.inner.keys.read().await.get(kid) {
                return key.decode(token);
            }
        }
        let jwt = self.get_jwt_from_partial_claims(token).await?;
        let claims = jwt.decode(token)?;
        self.inner.keys.write().await.insert(jwt.kid.clone(), jwt);
        Ok(claims)
    }
    pub async fn decode_logout_token(&self, token: &str) -> anyhow::Result<LogoutClaims> {
        let token_header = jsonwebtoken::decode_header(token)?;
        let kid = token_header
            .kid
            .as_ref()
            .ok_or(anyhow::anyhow!("Invalid token"))?;
        {
            if let Some(key) = self.inner.keys.read().await.get(kid) {
                return key.decode_logout_token(token);
            }
        }
        let jwt = self.get_jwt_from_partial_claims(token).await?;
        let logout_claims = jwt.decode_logout_token(token)?;
        self.inner.keys.write().await.insert(jwt.kid.clone(), jwt);
        Ok(logout_claims)
    }
}
