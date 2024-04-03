use std::sync::Arc;

use qm_pg::DB;

use crate::{
    cache::{
        update::{Op, Payload},
        RealmUpdate,
    },
    query::fetch_realm_info,
};

pub struct Realm {
    name: Arc<str>,
    id: Option<Arc<str>>,
}

impl Realm {
    pub async fn new(db: &DB, name: &str) -> anyhow::Result<Self> {
        let realm_query = fetch_realm_info(db, name).await?;
        Ok(Self {
            name: Arc::from(name.to_string()),
            id: realm_query.and_then(|r| r.id).map(Arc::from),
        })
    }

    pub fn update(&mut self, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<RealmUpdate> = serde_json::from_str(payload)?;
        if let (Op::Insert, Some(new)) = (payload.op, payload.new) {
            if new.name == self.name {
                self.id = Some(new.id);
            }
        }
        Ok(())
    }

    pub fn equals(&self, id: Option<&str>) -> bool {
        self.id.is_some() && self.id.as_deref() == id
    }
}
