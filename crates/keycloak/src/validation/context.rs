use crate::{Keycloak, KeycloakConfig};

pub struct Config<'a> {
    pub keycloak: &'a KeycloakConfig,
    pub public_url: &'a str,
    pub realm: &'a str,
    pub client_id: &'a str,
}

impl<'a> Config<'a> {
    pub fn keycloak(&self) -> &'a KeycloakConfig {
        self.keycloak
    }
    pub fn public_url(&self) -> &'a str {
        self.public_url
    }
    pub fn realm(&self) -> &'a str {
        self.realm
    }
    pub fn client_id(&self) -> &'a str {
        self.client_id
    }
}

pub struct ValidationContext<'a> {
    pub keycloak: &'a Keycloak,
    pub config: &'a Config<'a>,
}

impl<'a> ValidationContext<'a> {
    pub fn keycloak(&self) -> &'a Keycloak {
        self.keycloak
    }

    pub fn cfg(&self) -> &'a Config<'a> {
        self.config
    }
}
