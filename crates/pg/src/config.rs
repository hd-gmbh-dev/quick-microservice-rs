use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Config {
    host: Option<Arc<str>>,
    port: Option<u16>,
    max_connections: Option<u32>,
    min_connections: Option<u32>,
    acquire_timeout: Option<u64>,
    idle_timeout: Option<u64>,
    max_lifetime: Option<u64>,
    fair: Option<bool>,
    username: Option<Arc<str>>,
    password: Option<Arc<str>>,
    database: Option<Arc<str>>,
    root_username: Option<Arc<str>>,
    root_password: Option<Arc<str>>,
    root_database: Option<Arc<str>>,
    #[serde(skip)]
    address: Option<Arc<str>>,
    #[serde(skip)]
    root_address: Option<Arc<str>>,
}

impl Config {
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    pub fn max_connections(&self) -> u32 {
        self.max_connections.unwrap_or(32)
    }

    pub fn acquire_timeout(&self) -> u64 {
        self.acquire_timeout.unwrap_or(30)
    }

    pub fn database(&self) -> Option<&str> {
        self.database.as_deref()
    }

    pub fn username(&self) -> Option<&str> {
        self.username.as_deref()
    }

    pub fn password(&self) -> Option<&str> {
        self.password.as_deref()
    }

    pub fn root_database(&self) -> Option<&str> {
        self.root_database.as_deref()
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    pub fn root_address(&self) -> &str {
        self.root_address.as_deref().unwrap()
    }
}

#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn build(self) -> envy::Result<Config> {
        let mut cfg: Config = if let Some(prefix) = self.prefix {
            envy::prefixed(prefix)
        } else {
            envy::prefixed("PG_")
        }
        .from_env()?;
        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(27017);
        let mut address = match (cfg.username.as_deref(), cfg.password.as_deref()) {
            (Some(username), Some(password)) => {
                format!("postgresql://{}:{}@{}:{}/", username, password, host, port)
            }
            (Some(username), None) => format!("postgresql://{}@{}:{}/", username, host, port),
            _ => format!("postgresql://{}:{}/", host, port),
        };
        if let Some(database) = cfg.database.as_deref() {
            address.push_str(database);
        }
        cfg.address = Some(Arc::from(address));
        let mut root_address = match (cfg.root_username.as_deref(), cfg.root_password.as_deref()) {
            (Some(root_username), Some(root_password)) => format!(
                "postgresql://{}:{}@{}:{}/",
                root_username, root_password, host, port
            ),
            (Some(root_username), None) => {
                format!("postgresql://{}@{}:{}/", root_username, host, port)
            }
            _ => format!("postgresql://{}:{}/", host, port),
        };
        if let Some(root_database) = cfg.root_database.as_deref() {
            root_address.push_str(root_database);
        }
        cfg.root_address = Some(Arc::from(root_address));
        Ok(cfg)
    }
}
