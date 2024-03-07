use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct Config {
    app_name: Option<Arc<str>>,
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

    pub fn app_name(&self) -> &str {
        self.app_name.as_deref().unwrap()
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(3000)
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
        let prefix = self.prefix.unwrap_or("SERVER_");
        let mut cfg: Config = envy::prefixed(prefix).from_env()?;
        if cfg.app_name.is_none() {
            cfg.app_name = Some(Arc::from("quick-microservice"));
        }
        let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
        let port = cfg.port.unwrap_or(3000);
        cfg.address = Some(Arc::from(format!("{}:{}", host, port)));
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse_builtin_config_test() -> envy::Result<()> {
        let cfg = super::Config::builder()
            .with_prefix("DEFAULT_SERVER_NOT_SET_IN_SHELL_")
            .build()?;
        assert_eq!(cfg.address(), "127.0.0.1:3000");
        Ok(())
    }

    #[test]
    fn parse_default_config_test() -> envy::Result<()> {
        std::env::set_var("SERVER_HOST", "localhost");
        std::env::set_var("SERVER_PORT", "3000");
        let cfg = super::Config::new()?;
        assert_eq!(cfg.address(), "localhost:3000");
        Ok(())
    }

    #[test]
    fn parse_prefixed_config_test() -> envy::Result<()> {
        std::env::set_var("SERVER_CUSTOM_HOST", "localhost");
        std::env::set_var("SERVER_CUSTOM_PORT", "3000");
        let cfg = super::Config::builder()
            .with_prefix("SERVER_CUSTOM_")
            .build()?;
        assert_eq!(cfg.address(), "localhost:3000");
        Ok(())
    }
}
