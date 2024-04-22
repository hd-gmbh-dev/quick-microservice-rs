use crate::model::MdTables;
use crate::model::{RoleMapping, UserGroupNameMapping};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

fn sorted(v: HashSet<Rc<str>>) -> Rc<[Rc<str>]> {
    let mut v: Vec<Rc<str>> = v.into_iter().collect();
    v.sort();
    Rc::from(v)
}

#[derive(Debug)]
pub struct ParseResult {
    pub user_group_name_mappings: Vec<UserGroupNameMapping>,
    pub role_mappings: Vec<RoleMapping>,
    pub roles: Rc<[Rc<str>]>,
    pub user_groups: Rc<[Rc<str>]>,
    pub permissions: Rc<[Rc<str>]>,
    pub resources: Rc<[Rc<str>]>,
}

impl ParseResult {
    fn new(
        user_group_name_mappings: Vec<UserGroupNameMapping>,
        role_mappings: Vec<RoleMapping>,
    ) -> Self {
        let user_groups: HashSet<Rc<str>> = user_group_name_mappings
            .iter()
            .map(|v| v.user_group.clone())
            .collect();
        let roles: HashSet<Rc<str>> = role_mappings
            .iter()
            .map(|v| v.roles.iter().map(|r| r.clone()))
            .flatten()
            .collect();
        let roles = sorted(roles);
        let resources = sorted(roles.iter().fold(HashSet::default(), |mut state, s| {
            if let Some(resource) = s.split(":").next() {
                state.insert(Rc::from(resource.to_string()));
            }
            state
        }));
        let permissions = sorted(roles.iter().fold(HashSet::default(), |mut state, s| {
            if let Some(permission) = s.split(":").skip(1).next() {
                state.insert(Rc::from(permission.to_string()));
            }
            state
        }));
        Self {
            user_group_name_mappings,
            role_mappings,
            roles,
            user_groups: sorted(user_groups),
            permissions,
            resources,
        }
    }
}

pub fn parse(tables: MdTables) -> anyhow::Result<ParseResult> {
    let user_group_name_mappings: Vec<UserGroupNameMapping> = tables
        .user_groups
        .rows
        .into_iter()
        .filter_map(|mut t| {
            let access_level = t.pop();
            let display_name = t.pop();
            let path = t.pop();
            let user_group = t.pop();
            user_group
                .zip(path.zip(display_name.zip(access_level)))
                .map(
                    |(user_group, (path, (display_name, access_level)))| UserGroupNameMapping {
                        user_group: Rc::from(user_group),
                        display_name: Rc::from(display_name),
                        path: Rc::from(path),
                        access_level: Rc::from(access_level),
                    },
                )
        })
        .collect::<Vec<UserGroupNameMapping>>();

    let role_mappings = tables.roles;
    let role_mapping_headers: Rc<[Rc<str>]> = role_mappings
        .headers
        .into_iter()
        .skip(1)
        .map(|h| Rc::from(h))
        .collect();
    let role_mappings_map: HashMap<Rc<str>, Vec<Rc<str>>> =
        role_mappings
            .rows
            .into_iter()
            .fold(HashMap::default(), |mut state, mut row| {
                if !row.is_empty() {
                    let role: Rc<str> = Rc::from(row.remove(0));
                    for (idx, col) in row.into_iter().enumerate() {
                        if let Some(user_group) = role_mapping_headers.get(idx) {
                            if col.trim() == "x" {
                                state
                                    .entry(user_group.clone())
                                    .or_insert_with(|| vec![])
                                    .push(role.clone())
                            }
                        }
                    }
                }
                state
            });
    let mut role_mappings: Vec<RoleMapping> = role_mappings_map
        .into_iter()
        .map(|(user_group, roles)| RoleMapping {
            user_group,
            roles: Rc::from(roles),
        })
        .collect();
    role_mappings.sort_by_key(|v| v.user_group.clone());
    Ok(ParseResult::new(user_group_name_mappings, role_mappings))
}
