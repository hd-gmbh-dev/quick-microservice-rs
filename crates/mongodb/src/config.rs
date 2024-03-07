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
    sharded: Option<bool>,
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

    pub fn sharded(&self) -> bool {
        self.sharded.unwrap_or(false)
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
            envy::prefixed("MONGODB_")
        }
        .from_env()?;

        if cfg.database.is_none() {
            cfg.database = Some(Arc::from("test"));
        }

        if cfg.root_database.is_none() {
            cfg.root_database = Some(Arc::from("admin"));
        }

        let database = cfg.database.as_deref().unwrap();
        let root_database = cfg.root_database.as_deref().unwrap();
        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(27017);
        let address = match (cfg.username.as_deref(), cfg.password.as_deref()) {
            (Some(username), Some(password)) => format!(
                "mongodb://{}:{}@{}:{}/{}",
                username, password, host, port, database
            ),
            _ => format!("mongodb://{}:{}/{}", host, port, database),
        };
        cfg.address = Some(Arc::from(address));
        let root_address = match (cfg.root_username.as_deref(), cfg.root_password.as_deref()) {
            (Some(username), Some(password)) => format!(
                "mongodb://{}:{}@{}:{}/{}",
                username, password, host, port, root_database
            ),
            _ => format!("mongodb://{}:{}/{}", host, port, root_database),
        };
        cfg.root_address = Some(Arc::from(root_address));
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_builtin_config_test() -> envy::Result<()> {
        let cfg = super::Config::builder()
            .with_prefix("DEFAULT_DB_NOT_SET_IN_SHELL_")
            .build()?;
        assert_eq!(cfg.address(), "mongodb://127.0.0.1:27017/test");
        assert_eq!(cfg.root_address(), "mongodb://127.0.0.1:27017/admin");
        Ok(())
    }

    #[test]
    fn parse_default_config_test() -> envy::Result<()> {
        std::env::set_var("MONGODB_HOST", "localhost");
        std::env::set_var("MONGODB_PORT", "27017");
        std::env::set_var("MONGODB_USERNAME", "testuser");
        std::env::set_var("MONGODB_PASSWORD", "userpw");
        std::env::set_var("MONGODB_DATABASE", "testdb");
        std::env::set_var("MONGODB_ROOT_USERNAME", "testadmin");
        std::env::set_var("MONGODB_ROOT_PASSWORD", "adminpw");
        std::env::set_var("MONGODB_ROOT_DATABASE", "admin");
        std::env::set_var("MONGODB_SHARDED", "false");
        let cfg = super::Config::new()?;
        assert_eq!(
            cfg.address(),
            "mongodb://testuser:userpw@localhost:27017/testdb"
        );
        assert_eq!(
            cfg.root_address(),
            "mongodb://testadmin:adminpw@localhost:27017/admin"
        );
        Ok(())
    }

    #[test]
    fn parse_prefixed_config_test() -> envy::Result<()> {
        std::env::set_var("MGMTDB_HOST", "localhost");
        std::env::set_var("MGMTDB_PORT", "27017");
        std::env::set_var("MGMTDB_USERNAME", "testuser");
        std::env::set_var("MGMTDB_PASSWORD", "userpw");
        std::env::set_var("MGMTDB_DATABASE", "testdb");
        std::env::set_var("MGMTDB_ROOT_USERNAME", "testadmin");
        std::env::set_var("MGMTDB_ROOT_PASSWORD", "adminpw");
        std::env::set_var("MGMTDB_ROOT_DATABASE", "admin");
        std::env::set_var("MGMTDB_SHARDED", "false");
        let cfg = super::Config::builder().with_prefix("MGMTDB_").build()?;
        assert_eq!(
            cfg.address(),
            "mongodb://testuser:userpw@localhost:27017/testdb"
        );
        assert_eq!(
            cfg.root_address(),
            "mongodb://testadmin:adminpw@localhost:27017/admin"
        );
        Ok(())
    }
}
