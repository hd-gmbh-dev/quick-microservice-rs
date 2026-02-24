use std::{collections::HashSet, sync::Arc};

use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Resource access from JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceAccess {
    /// Account access.
    pub account: RealmAccess,
}

/// Realm access from JWT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealmAccess {
    /// Roles.
    pub roles: Vec<Arc<str>>,
}

/// Partial claims for JWT validation.
#[derive(Serialize, Clone, Deserialize, Default)]
pub struct PartialClaims {
    /// Issuer.
    pub iss: String,
    /// Authorized party.
    pub azp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
/// Full claims from JWT.
pub struct Claims {
    /// Expiration time.
    pub exp: i64,
    /// Issued at time.
    pub iat: i64,
    /// Authentication time.
    pub auth_time: Option<i64>,
    /// JWT ID.
    pub jti: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: serde_json::Value,
    /// Subject.
    pub sub: Arc<str>,
    /// Token type.
    pub typ: String,
    /// Authorized party.
    pub azp: String,
    /// Authentication context class reference.
    pub acr: String,
    /// Allowed origins.
    #[serde(rename = "allowed-origins")]
    pub allowed_origins: Option<Vec<Arc<str>>>,
    /// Realm access.
    pub realm_access: RealmAccess,
    /// Resource access.
    pub resource_access: ResourceAccess,
    /// Scope.
    #[serde(default)]
    pub scope: String,
    /// Session ID.
    #[serde(default)]
    pub sid: String,
    /// Email verified.
    #[serde(default)]
    pub email_verified: bool,
    /// Name.
    #[serde(default)]
    pub name: String,
    /// Preferred username.
    #[serde(default)]
    pub preferred_username: String,
    /// Given name.
    #[serde(default)]
    pub given_name: String,
    /// Family name.
    #[serde(default)]
    pub family_name: String,
    /// Email.
    #[serde(default)]
    pub email: String,
    /// Whether this is an API test.
    #[serde(skip)]
    pub is_api_test: bool,
}

impl Default for Claims {
    fn default() -> Self {
        Self {
            exp: 0,
            iat: 0,
            auth_time: None,
            jti: "".to_string(),
            iss: "".to_string(),
            is_api_test: true,
            sub: Arc::from("user-id"),
            typ: "".to_string(),
            azp: "".to_string(),
            acr: "".to_string(),
            allowed_origins: None,
            realm_access: RealmAccess { roles: vec![] },
            resource_access: ResourceAccess {
                account: RealmAccess { roles: vec![] },
            },
            scope: "".to_string(),
            sid: "".to_string(),
            email_verified: false,
            name: "".to_string(),
            preferred_username: "".to_string(),
            given_name: "".to_string(),
            family_name: "".to_string(),
            aud: Default::default(),
            email: "".to_string(),
        }
    }
}

/// Claims from logout token.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutClaims {
    /// Issued at.
    pub iat: i64,
    /// JWT ID.
    pub jti: String,
    /// Issuer.
    pub iss: String,
    /// Audience.
    pub aud: serde_json::Value,
    /// Subject.
    pub sub: String,
    /// Token type.
    pub typ: String,
    /// Session ID.
    pub sid: String,
}

/// JWT token holder.
#[derive(Clone)]
pub struct Jwt {
    /// Key ID.
    pub kid: String,
    validation: Validation,
    logout_validation: Validation,
    decoding_key: DecodingKey,
}

impl Jwt {
    /// Creates a new JWT.
    pub fn new(
        alg: Algorithm,
        kid: String,
        public_key: &str,
        client_id: &str,
    ) -> anyhow::Result<Self> {
        let mut validation = Validation::new(alg);
        validation.set_audience(&[client_id, "account"]);
        // needed workaround to validate logout tokens (they contain no exp field)
        let mut logout_validation = Validation::new(alg);
        logout_validation.validate_exp = false;
        logout_validation.required_spec_claims = HashSet::new();
        logout_validation
            .required_spec_claims
            .insert("sub".to_string());
        logout_validation
            .required_spec_claims
            .insert("iss".to_string());
        logout_validation
            .required_spec_claims
            .insert("aud".to_string());
        Ok(Self {
            kid,
            validation,
            logout_validation,
            decoding_key: DecodingKey::from_rsa_pem(
                format!("-----BEGIN PUBLIC KEY-----\n{public_key}\n-----END PUBLIC KEY-----")
                    .as_bytes(),
            )?,
        })
    }

    /// Decodes a token to Claims.
    pub fn decode(&self, token: &str) -> anyhow::Result<Claims> {
        self.decode_custom(token)
    }

    /// Decodes a token to custom claims.
    pub fn decode_custom<C: DeserializeOwned + Clone>(&self, token: &str) -> anyhow::Result<C> {
        let result = decode(token, &self.decoding_key, &self.validation)?;
        Ok(result.claims)
    }

    /// Decodes a logout token.
    pub fn decode_logout_token(&self, token: &str) -> anyhow::Result<LogoutClaims> {
        self.decode_logout_token_custom(token)
    }

    /// Decodes a logout token to custom claims.
    pub fn decode_logout_token_custom<C: DeserializeOwned + Clone>(
        &self,
        token: &str,
    ) -> anyhow::Result<C> {
        let result = decode(token, &self.decoding_key, &self.logout_validation)?;
        Ok(result.claims)
    }
}
