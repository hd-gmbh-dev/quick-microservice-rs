use rdkafka::admin::AdminClient;
use rdkafka::admin::ConfigEntry;
use rdkafka::admin::ResourceSpecifier;
use rdkafka::admin::TopicReplication;
use rdkafka::admin::{AdminOptions, NewTopic};
use rdkafka::client::DefaultClientContext;
use rdkafka::groups::GroupList;
use rdkafka::metadata::Metadata;
use rdkafka::ClientConfig;

use crate::config::Config as KafkaConfig;

/// Kafka admin client for topic and consumer group management.
///
/// Provides methods for creating, deleting, and managing Kafka topics
/// and consumer groups.
pub struct Client {
    inner: AdminClient<DefaultClientContext>,
}

impl Client {
    /// Creates a new Kafka admin client.
    pub fn new(cfg: &KafkaConfig) -> anyhow::Result<Self> {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", cfg.address());
        let inner = config.create()?;
        Ok(Self { inner })
    }

    /// Returns a reference to the admin client.
    pub fn admin(&self) -> &AdminClient<DefaultClientContext> {
        &self.inner
    }

    /// Creates a new topic with default configuration.
    pub async fn create_topic(&self, topic_name: &str) -> anyhow::Result<()> {
        self.inner
            .create_topics(
                &[NewTopic {
                    name: topic_name,
                    num_partitions: 1,
                    replication: TopicReplication::Fixed(1),
                    config: vec![("retention.ms", "-1")],
                }],
                &AdminOptions::default(),
            )
            .await?;
        Ok(())
    }

    /// Creates a new topic with custom configuration.
    pub async fn create_topic_with_config(
        &self,
        topic_name: &str,
        config: Vec<(&str, &str)>,
    ) -> anyhow::Result<()> {
        self.inner
            .create_topics(
                &[NewTopic {
                    name: topic_name,
                    num_partitions: 1,
                    replication: TopicReplication::Fixed(1),
                    config,
                }],
                &AdminOptions::default(),
            )
            .await?;
        Ok(())
    }

    /// Deletes a topic.
    pub async fn delete_topic(&self, topic_name: &str) -> anyhow::Result<()> {
        self.inner
            .delete_topics(&[topic_name], &AdminOptions::default())
            .await?;
        Ok(())
    }

    /// Returns cluster metadata.
    pub fn metadata(&self) -> anyhow::Result<Metadata> {
        Ok(self.inner.inner().fetch_metadata(None, None)?)
    }

    /// Returns the list of consumer groups.
    pub fn groups(&self) -> anyhow::Result<GroupList> {
        Ok(self.inner.inner().fetch_group_list(None, None)?)
    }

    /// Ensures all specified topics exist, creating any that don't.
    pub async fn ensure_topics(&self, topic_names: &[&str]) -> anyhow::Result<()> {
        let metadata = self.metadata()?;
        for topic_name in topic_names {
            let exist = metadata.topics().iter().any(|t| &t.name() == topic_name);
            if !exist {
                self.create_topic(topic_name).await?;
            }
        }
        Ok(())
    }

    /// Returns the configuration for a topic.
    pub async fn topic_config(&self, topic_name: &str) -> anyhow::Result<Vec<ConfigEntry>> {
        Ok(self
            .inner
            .describe_configs(
                &[ResourceSpecifier::Topic(topic_name)],
                &AdminOptions::default(),
            )
            .await?
            .pop()
            .transpose()?
            .map(|v| v.entries)
            .unwrap_or_default())
    }

    /// Deletes all topics in the cluster.
    // TODO: only delete topic containing prefix
    pub async fn cleanup_topics(&self) -> anyhow::Result<()> {
        let metadata = self.metadata()?;
        let topics: Vec<&str> = metadata.topics().iter().map(|t| t.name()).collect();
        if !topics.is_empty() {
            self.inner
                .delete_topics(topics.as_ref(), &AdminOptions::default())
                .await?;
        }
        Ok(())
    }

    /// Deletes all consumer groups in the cluster.
    // TODO: only delete group containing prefix
    pub async fn cleanup_groups(&self) -> anyhow::Result<()> {
        let groups = self.groups()?;
        let groups: Vec<&str> = groups.groups().iter().map(|g| g.name()).collect();
        if !groups.is_empty() {
            self.inner
                .delete_groups(groups.as_ref(), &AdminOptions::default())
                .await?;
        }
        Ok(())
    }
}
