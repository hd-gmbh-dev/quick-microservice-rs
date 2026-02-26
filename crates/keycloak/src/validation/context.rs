use crate::{Keycloak, KeycloakConfig};

/// Configuration for validation.
pub struct Config<'a> {
    /// Keycloak config.
    pub keycloak: &'a KeycloakConfig,
    /// App URLs, first one is used for root URL, and all are used to set redirect URIs.
    pub public_urls: &'a [&'a str],
    /// Realm name.
    pub realm: &'a str,
    /// Client ID.
    pub client_id: &'a str,
}

impl<'a> Config<'a> {
    /// Gets the Keycloak config.
    pub fn keycloak(&self) -> &'a KeycloakConfig {
        self.keycloak
    }
    /// App URLs, first one is used for root URL, and all are used to set redirect URIs.
    pub fn public_urls(&self) -> &'a [&'a str] {
        self.public_urls
    }
    /// Gets the realm name.
    pub fn realm(&self) -> &'a str {
        self.realm
    }
    /// Gets the client ID.
    pub fn client_id(&self) -> &'a str {
        self.client_id
    }
}

/// Validation context.
pub struct ValidationContext<'a> {
    /// Keycloak instance.
    pub keycloak: &'a Keycloak,
    /// Config.
    pub config: &'a Config<'a>,
}

impl<'a> ValidationContext<'a> {
    /// Gets the Keycloak instance.
    pub fn keycloak(&self) -> &'a Keycloak {
        self.keycloak
    }

    /// Gets the config.
    pub fn cfg(&self) -> &'a Config<'a> {
        self.config
    }
}
