use rdkafka::admin::AdminClient;
use rdkafka::admin::TopicReplication;
use rdkafka::admin::{AdminOptions, NewTopic};
use rdkafka::client::DefaultClientContext;
use rdkafka::groups::GroupList;
use rdkafka::metadata::Metadata;
use rdkafka::ClientConfig;

use crate::config::Config as KafkaConfig;

pub struct Client {
    inner: AdminClient<DefaultClientContext>,
}

impl Client {
    pub fn new(cfg: &KafkaConfig) -> anyhow::Result<Self> {
        let mut config = ClientConfig::new();
        config.set("bootstrap.servers", cfg.address());
        let inner = config.create()?;
        Ok(Self { inner })
    }

    pub async fn create_topic(&self, topic_name: &str) -> anyhow::Result<()> {
        self.inner
            .create_topics(
                &[NewTopic {
                    name: topic_name,
                    num_partitions: 1,
                    replication: TopicReplication::Fixed(1),
                    config: vec![],
                }],
                &AdminOptions::default(),
            )
            .await?;
        Ok(())
    }

    pub async fn delete_topic(&self, topic_name: &str) -> anyhow::Result<()> {
        self.inner
            .delete_topics(&[topic_name], &AdminOptions::default())
            .await?;
        Ok(())
    }

    pub fn metadata(&self) -> anyhow::Result<Metadata> {
        Ok(self.inner.inner().fetch_metadata(None, None)?)
    }

    pub fn groups(&self) -> anyhow::Result<GroupList> {
        Ok(self.inner.inner().fetch_group_list(None, None)?)
    }

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
