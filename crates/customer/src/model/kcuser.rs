use crate::model::Group;
use crate::model::Role;
use async_graphql::{Enum, InputObject, SimpleObject};
use qm_entity::ids::{InfraContext, InstitutionId, PartialEqual};
use qm_entity::IsAdmin;
use sqlx::types::Uuid;
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
pub struct UserEntityUpdate {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Option<Arc<str>>,
    pub first_name: Option<Arc<str>>,
    pub last_name: Option<Arc<str>>,
    pub realm_id: Option<Arc<str>>,
    pub enabled: bool,
}

pub struct TmpUser {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub firstname: Arc<str>,
    pub lastname: Arc<str>,
    pub groups: HashSet<Arc<str>>,
    pub roles: HashSet<Arc<str>>,
    pub enabled: bool,
}
pub type TmpUserMap = HashMap<Arc<str>, TmpUser>;

#[derive(Debug, FromRow)]
pub struct KcUserQuery {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
    pub role_id: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub enabled: bool,
}

impl KcUserQuery {
    pub fn has_all_fields(&self) -> bool {
        [
            self.user_id.as_ref(),
            self.group_id.as_ref(),
            self.role_id.as_ref(),
            self.firstname.as_ref(),
            self.lastname.as_ref(),
            self.username.as_ref(),
            self.email.as_ref(),
        ]
        .iter()
        .all(Option::is_some)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Enum, Copy, Eq, PartialEq)]
pub enum RequiredUserAction {
    #[graphql(name = "UPDATE_PASSWORD")]
    UpdatePassword,
}

impl std::fmt::Display for RequiredUserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            RequiredUserAction::UpdatePassword => "UPDATE_PASSWORD",
        }
        .to_string();
        write!(f, "{}", str)
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserInput {
    pub username: String,
    pub firstname: String,
    pub lastname: String,
    pub password: String,
    pub email: String,
    pub phone: Option<String>,
    pub salutation: Option<String>,
    pub room_number: Option<String>,
    pub job_title: Option<String>,
    pub enabled: Option<bool>,
    pub required_actions: Option<Vec<RequiredUserAction>>,
}

#[derive(Debug)]
pub struct CreateUserPayload {
    pub user: CreateUserInput,
    pub group: Option<String>,
    pub access: Option<String>,
    pub context: Option<InfraContext>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct User {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub firstname: Arc<str>,
    pub lastname: Arc<str>,
    pub groups: Arc<[Arc<Group>]>,
    pub roles: Arc<[Arc<Role>]>,
    pub enabled: bool,
    #[graphql(skip)]
    pub context: Option<InfraContext>,
}

pub type UserMap = HashMap<Arc<str>, Arc<User>>;
pub type UserUidMap = HashMap<Uuid, Arc<User>>;

#[derive(Debug, serde::Deserialize)]
pub struct UserRoleMappingUpdate {
    pub role_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct UserGroupMembershipUpdate {
    pub group_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct GroupAttributeUpdate {
    pub group_id: Arc<str>,
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct UserList {
    pub items: Arc<[Arc<User>]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl PartialEqual<'_, InfraContext> for User {
    fn partial_equal(&'_ self, r: &'_ InfraContext) -> bool {
        if let Some(context) = self.context.as_ref() {
            match r {
                InfraContext::Customer(v) => context.has_customer(v),
                InfraContext::Organization(v) => context.has_organization(v),
                InfraContext::OrganizationUnit(v) => context.has_organization_unit(v),
                InfraContext::Institution(v) => context.has_institution(v),
            }
        } else {
            false
        }
    }
}

impl PartialEqual<'_, InstitutionId> for User {
    fn partial_equal(&'_ self, r: &'_ InstitutionId) -> bool {
        if let Some(context) = self.context.as_ref() {
            context.has_institution(r)
        } else {
            false
        }
    }
}

impl IsAdmin for User {
    fn is_admin(&self) -> bool {
        self.roles.iter().any(|r| r.name.as_ref() == "admin" || r.name.as_ref() == "administration")
    }
}