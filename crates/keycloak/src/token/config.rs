use std::sync::Arc;

/// Builder for token config.
#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    /// Sets environment variable prefix.
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Builds the config.
    pub fn build(self) -> envy::Result<Config> {
        let mut cfg: Config = if let Some(prefix) = self.prefix {
            envy::prefixed(prefix)
        } else {
            envy::prefixed("KEYCLOAK_")
        }
        .from_env()?;
        if cfg.address.is_none() {
            let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
            let port = cfg.port.unwrap_or(42210);
            cfg.address = Some(Arc::from(format!("http://{}:{}/", host, port)));
        }
        Ok(cfg)
    }
}

/// Token configuration.
#[derive(Clone, serde::Deserialize, Debug)]
pub struct Config {
    /// Port.
    port: Option<u16>,
    /// Host.
    host: Option<Arc<str>>,
    /// Address.
    address: Option<Arc<str>>,
    /// Public URL.
    public_url: Option<Arc<str>>,
}

impl Config {
    /// Creates a new config.
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    /// Creates a new config builder.
    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    /// Gets the address.
    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap_or("http://127.0.0.1:42210")
    }

    /// Gets the public URL.
    pub fn public_url(&self) -> &str {
        self.public_url.as_deref().unwrap_or("http://127.0.0.1:80")
    }
}
