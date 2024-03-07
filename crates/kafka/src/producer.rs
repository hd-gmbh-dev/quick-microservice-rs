use rdkafka::config::ClientConfig;
use rdkafka::producer::{FutureProducer, FutureRecord};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;

pub enum EventNs {
    Customer,
    Organization,
    OrganizationUnit,
    Institution,
    User,
    Entity,
    RcObject,
    Role,
}

impl AsRef<str> for EventNs {
    fn as_ref(&self) -> &str {
        match self {
            EventNs::Customer => "customer",
            EventNs::Organization => "organization",
            EventNs::OrganizationUnit => "organization_unit",
            EventNs::Institution => "institution",
            EventNs::User => "user",
            EventNs::Entity => "entity",
            EventNs::RcObject => "rc_object",
            EventNs::Role => "role",
        }
    }
}

impl FromStr for EventNs {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "customer" => Ok(EventNs::Customer),
            "organization" => Ok(EventNs::Organization),
            "organization_unit" => Ok(EventNs::OrganizationUnit),
            "institution" => Ok(EventNs::Institution),
            "user" => Ok(EventNs::User),
            "entity" => Ok(EventNs::Entity),
            "rc_object" => Ok(EventNs::RcObject),
            "role" => Ok(EventNs::Role),
            _ => Err(anyhow::anyhow!(
                "variant not found '{s}' for Event namespace"
            )),
        }
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum EventType {
    Create,
    Update,
    Delete,
    Link,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub event: EventType,
    pub ty: String,
    pub object: serde_json::Value,
}

pub struct Inner {
    config: Config,
    producer: FutureProducer,
}

#[derive(Default)]
pub struct ProducerBuilder {
    env_prefix: Option<&'static str>,
}

impl ProducerBuilder {
    pub fn with_env_prefix(mut self, prefix: &'static str) -> Self {
        self.env_prefix = Some(prefix);
        self
    }

    pub fn build(self) -> anyhow::Result<Producer> {
        let mut config_builder = Config::builder();
        if let Some(prefix) = self.env_prefix {
            config_builder = config_builder.with_prefix(prefix);
        }
        let config = config_builder.build()?;
        let producer: FutureProducer = ClientConfig::new()
            .set("bootstrap.servers", config.address())
            .set("message.timeout.ms", "5000")
            .create()?;

        Ok(Producer {
            inner: Arc::new(Inner { producer, config }),
        })
    }
}

#[derive(Clone)]
pub struct Producer {
    inner: Arc<Inner>,
}

impl Producer {
    pub fn new() -> anyhow::Result<Self> {
        ProducerBuilder::default().build()
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    pub async fn create_event<O>(
        &self,
        event_ns: &EventNs,
        ty: &str,
        object: O,
    ) -> anyhow::Result<()>
    where
        O: serde::ser::Serialize,
    {
        self.produce_event("create", EventType::Create, event_ns, ty, object)
            .await
    }

    pub async fn update_event<O>(
        &self,
        event_ns: &EventNs,
        ty: &str,
        object: O,
    ) -> anyhow::Result<()>
    where
        O: serde::ser::Serialize,
    {
        self.produce_event("update", EventType::Update, event_ns, ty, object)
            .await
    }

    pub async fn delete_event<O>(
        &self,
        event_ns: &EventNs,
        ty: &str,
        object: O,
    ) -> anyhow::Result<()>
    where
        O: serde::ser::Serialize,
    {
        self.produce_event("delete", EventType::Delete, event_ns, ty, object)
            .await
    }

    pub async fn link_event<O>(&self, event_ns: &EventNs, ty: &str, object: O) -> anyhow::Result<()>
    where
        O: serde::ser::Serialize,
    {
        self.produce_event("link", EventType::Link, event_ns, ty, object)
            .await
    }

    async fn produce_event<O>(
        &self,
        event_name: &'static str,
        event: EventType,
        event_ns: &EventNs,
        ty: &str,
        object: O,
    ) -> anyhow::Result<()>
    where
        O: serde::ser::Serialize,
    {
        log::debug!("{event_name} event for type: {ty}");
        let object = serde_json::to_value(object)?;
        let event = Event {
            event,
            ty: ty.to_string(),
            object,
        };
        let event = serde_json::to_string(&event)?;
        let (a, b) = self
            .inner
            .producer
            .send_result(
                FutureRecord::to(self.inner.config.topic_mutation_events())
                    .key(event_ns.as_ref())
                    .payload(&event)
                    .timestamp(now()),
            )
            .map_err(|e| anyhow::anyhow!("{e:#?}"))?
            .await?
            .map_err(|e| anyhow::anyhow!("{e:#?}"))?;
        log::debug!("produced {event_name} event for type {ty} with partition {a} and offset {b}");
        Ok(())
    }
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .try_into()
        .unwrap()
}
