use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Config {
    host: Option<Arc<str>>,
    port: Option<u16>,
    #[serde(skip)]
    address: Option<Arc<str>>,
}

impl Config {
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
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
            envy::prefixed("REDIS_")
        }
        .from_env()?;

        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(6379);
        cfg.address = Some(Arc::from(format!("redis://{}:{}/", host, port)));
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_builtin_config_test() -> envy::Result<()> {
        let cfg = super::Config::builder()
            .with_prefix("DEFAULT_REDIS_NOT_SET_IN_SHELL_")
            .build()?;
        assert_eq!(cfg.address(), "redis://127.0.0.1:6379/");
        Ok(())
    }

    #[test]
    fn parse_default_config_test() -> envy::Result<()> {
        std::env::set_var("REDIS_HOST", "localhost");
        std::env::set_var("REDIS_PORT", "6379");
        let cfg = super::Config::new()?;
        assert_eq!(cfg.address(), "redis://localhost:6379/");
        Ok(())
    }

    #[test]
    fn parse_prefixed_config_test() -> envy::Result<()> {
        std::env::set_var("REDIS_CACHE_HOST", "localhost");
        std::env::set_var("REDIS_CACHE_PORT", "6379");
        let cfg = super::Config::builder()
            .with_prefix("REDIS_CACHE_")
            .build()?;
        assert_eq!(cfg.address(), "redis://localhost:6379/");
        Ok(())
    }
}
