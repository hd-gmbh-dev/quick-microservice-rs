use std::sync::Arc;

use qm_pg::DB;
use qm_role::AccessLevel;

use crate::cache::update::{Op, Payload};
use crate::cache::{GroupAttributeUpdate, GroupDetail, GroupDetailsMap, KcGroupDetailsQuery};
use crate::query::fetch_group_attributes;

use super::groups::Groups;

fn parse_access_level(s: &str) -> Arc<[AccessLevel]> {
    s.split(',').filter_map(|s| s.trim().parse().ok()).collect()
}

pub struct GroupAttributes {
    group_attribute_map: GroupDetailsMap,
}

impl GroupAttributes {
    pub async fn new(db: &DB, realm: &str) -> anyhow::Result<Self> {
        let group_attribute_map = fetch_group_attributes(db, realm)
            .await?
            .into_iter()
            .filter(KcGroupDetailsQuery::has_all_fields)
            .fold(GroupDetailsMap::default(), |mut state, row| {
                let group_id: Arc<str> = Arc::from(row.group_id.unwrap());
                state.insert(
                    group_id,
                    Arc::new(GroupDetail {
                        allowed_access_levels: row
                            .allowed_access_levels
                            .as_ref()
                            .map(|s| parse_access_level(s)),
                        built_in: row.built_in.map(|s| s == "1").unwrap_or(false),
                        display_name: Some(Arc::from(row.display_name.unwrap())),
                        context: row.context.and_then(|r| r.parse().ok()),
                    }),
                );
                state
            });
        Ok(Self {
            group_attribute_map,
        })
    }

    pub fn get(&self, id: &str) -> Option<&Arc<GroupDetail>> {
        self.group_attribute_map.get(id)
    }

    pub fn update(&mut self, groups: &Groups, payload: &str) -> anyhow::Result<()> {
        let payload: Payload<GroupAttributeUpdate> = serde_json::from_str(payload)?;
        match (payload.op, payload.new, payload.old) {
            (Op::Insert, Some(new), None) => {
                if groups.contains(&new.group_id) {
                    let mut group_detail =
                        if let Some(group_detail) = self.group_attribute_map.get(&new.group_id) {
                            group_detail.as_ref().to_owned()
                        } else {
                            GroupDetail {
                                built_in: false,
                                display_name: None,
                                allowed_access_levels: None,
                                context: None,
                            }
                        };
                    if let Some((name, value)) = new.name.zip(new.value.as_ref()) {
                        match name.as_str() {
                            "built_in" => {
                                group_detail.built_in = value == "1";
                            }
                            "allowed_access_levels" => {
                                group_detail.allowed_access_levels =
                                    Some(parse_access_level(value));
                            }
                            "display_name" => {
                                group_detail.display_name = Some(Arc::from(value.to_string()));
                            }
                            "context" => {
                                group_detail.context = value.as_str().parse().ok();
                            }
                            _ => {}
                        }
                        self.group_attribute_map
                            .insert(new.group_id.clone(), Arc::new(group_detail));
                    }
                }
            }
            (Op::Delete, None, Some(old)) => if groups.contains(&old.group_id) {},
            _ => {}
        }
        Ok(())
    }
}
