use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Config {
    host: Option<Arc<str>>,
    port: Option<u16>,
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

    pub fn username(&self) -> &str {
        self.username.as_deref().unwrap()
    }

    pub fn password(&self) -> &str {
        self.password.as_deref().unwrap()
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    pub fn root_address(&self) -> &str {
        self.root_address.as_deref().unwrap()
    }

    pub fn database(&self) -> &str {
        self.database.as_deref().unwrap()
    }

    pub fn root_database(&self) -> &str {
        self.root_database.as_deref().unwrap()
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

        if cfg.database.is_none() {
            cfg.database = Some(Arc::from("test"));
        }

        let database = cfg.database.as_deref().unwrap();
        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(27017);
        let address = match (cfg.username.as_deref(), cfg.password.as_deref()) {
            (Some(username), Some(password)) => format!(
                "postgresql://{}:{}@{}:{}/{}",
                username, password, host, port, database
            ),
            (Some(username), None) => format!(
                "postgresql://{}@{}:{}/{}",
                username, host, port, database
            ),
            _ => format!("postgresql://{}:{}/{}", host, port, database),
        };
        cfg.address = Some(Arc::from(address));
        let root_address = match (cfg.root_username.as_deref(), cfg.root_password.as_deref()) {
            (Some(username), Some(password)) => format!(
                "postgresql://{}:{}@{}:{}/",
                username, password, host, port,
            ),
            (Some(username), None) => format!(
                "postgresql://{}@{}:{}/",
                username, host, port,
            ),
            _ => format!("postgresql://{}:{}/", host, port),
        };
        cfg.root_address = Some(Arc::from(root_address));
        Ok(cfg)
    }
}
