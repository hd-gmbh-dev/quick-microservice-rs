use serde::Deserialize;
use std::sync::Arc;

/// Kafka configuration loaded from environment variables.
#[derive(Deserialize)]
pub struct Config {
    host: Option<Arc<str>>,
    port: Option<u16>,
    address: Option<Arc<str>>,
    topic_mutation_events: Option<Arc<str>>,
    consumer_group_mutation_events_prefix: Option<Arc<str>>,
}

impl Config {
    /// Creates a new Config from environment variables with default KAFKA_ prefix.
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    /// Creates a new ConfigBuilder for custom configuration.
    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    /// Returns the Kafka broker address.
    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap()
    }

    /// Returns the mutation events topic name.
    pub fn topic_mutation_events(&self) -> &str {
        self.topic_mutation_events.as_deref().unwrap()
    }

    /// Returns the consumer group prefix for mutation events.
    pub fn consumer_group_mutation_events_prefix(&self) -> &str {
        self.consumer_group_mutation_events_prefix
            .as_deref()
            .unwrap()
    }
}

/// Builder for Kafka configuration with custom prefix support.
#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    /// Sets a custom environment variable prefix.
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    /// Builds the Config from environment variables.
    pub fn build(self) -> envy::Result<Config> {
        let mut cfg: Config = if let Some(prefix) = self.prefix {
            envy::prefixed(prefix)
        } else {
            envy::prefixed("KAFKA_")
        }
        .from_env()?;

        if cfg.address.is_none() {
            let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
            let port = cfg.port.unwrap_or(9092);
            cfg.address = Some(Arc::from(format!("{}:{}", host, port)));
        }
        if cfg.topic_mutation_events.is_none() {
            cfg.topic_mutation_events = Some(Arc::from("qm_mutation_events"));
        }
        if cfg.consumer_group_mutation_events_prefix.is_none() {
            cfg.consumer_group_mutation_events_prefix = Some(Arc::from("qm_consumer_group"));
        }
        Ok(cfg)
    }
}
