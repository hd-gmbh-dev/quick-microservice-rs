use std::rc::Rc;

pub type Column = String;
pub type Row = Vec<Column>;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Table {
    pub headers: Vec<String>,
    pub rows: Vec<Row>,
}

#[derive(Default)]
pub struct OptMdTables {
    pub user_groups: Option<Table>,
    pub roles: Option<Table>,
}

pub struct MdTables {
    pub user_groups: Table,
    pub roles: Table,
}

impl TryFrom<OptMdTables> for MdTables {
    type Error = anyhow::Error;
    fn try_from(value: OptMdTables) -> Result<Self, Self::Error> {
        Ok(Self {
            user_groups: value
                .user_groups
                .ok_or(anyhow::anyhow!("unable to find `user_groups` table"))?,
            roles: value
                .roles
                .ok_or(anyhow::anyhow!("unable to find `roles` table"))?,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct UserGroupNameMapping {
    pub user_group: Rc<str>,
    pub path: Rc<str>,
    pub display_name: Rc<str>,
    pub access_level: Rc<str>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct RoleMapping {
    pub user_group: Rc<str>,
    pub roles: Rc<[Rc<str>]>,
}
