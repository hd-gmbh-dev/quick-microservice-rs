use async_graphql::{Enum, InputObject, SimpleObject};
use qm_entity::ids::{InfraContext, InstitutionId, PartialEqual};
use sqlx::types::Uuid;
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::GroupDetail;

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

impl UserEntityUpdate {
    pub fn has_all_fields(&self) -> bool {
        self.email.is_some() && self.first_name.is_some() && self.last_name.is_some()
    }
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
    pub id: Option<String>,
    pub firstname: Option<String>,
    pub lastname: Option<String>,
    pub username: Option<String>,
    pub email: Option<String>,
    pub enabled: bool,
}

impl KcUserQuery {
    pub fn has_all_fields(&self) -> bool {
        [
            self.id.as_ref(),
            self.firstname.as_ref(),
            self.lastname.as_ref(),
            self.username.as_ref(),
            self.email.as_ref(),
        ]
        .iter()
        .all(Option::is_some)
    }
}

#[derive(Debug, FromRow)]
pub struct KcUserGroupQuery {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
}

impl KcUserGroupQuery {
    pub fn has_all_fields(&self) -> bool {
        [self.group_id.as_ref(), self.user_id.as_ref()]
            .iter()
            .all(Option::is_some)
    }
}

#[derive(Debug, FromRow)]
pub struct KcUserRoleQuery {
    pub user_id: Option<String>,
    pub role_id: Option<String>,
}

impl KcUserRoleQuery {
    pub fn has_all_fields(&self) -> bool {
        [self.user_id.as_ref(), self.role_id.as_ref()]
            .iter()
            .all(Option::is_some)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Enum, Copy, Eq, PartialEq)]
pub enum QmRequiredUserAction {
    #[graphql(name = "VERIFY_EMAIL")]
    VerifyEmail,
    #[graphql(name = "UPDATE_PROFILE")]
    UpdateProfile,
    #[graphql(name = "CONFIGURE_TOTP")]
    ConfigureTotp,
    #[graphql(name = "UPDATE_PASSWORD")]
    UpdatePassword,
    #[graphql(name = "TERMS_AND_CONDITIONS")]
    TermsAndConditions,
}

impl std::fmt::Display for QmRequiredUserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            QmRequiredUserAction::VerifyEmail => "VERIFY_EMAIL",
            QmRequiredUserAction::UpdateProfile => "UPDATE_PROFILE",
            QmRequiredUserAction::ConfigureTotp => "CONFIGURE_TOTP",
            QmRequiredUserAction::UpdatePassword => "UPDATE_PASSWORD",
            QmRequiredUserAction::TermsAndConditions => "TERMS_AND_CONDITIONS",
        }
        .to_string();
        write!(f, "{}", str)
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct QmCreateUserInput {
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
    pub required_actions: Option<Vec<QmRequiredUserAction>>,
}

#[derive(Debug)]
pub struct CreateUserPayload {
    pub user: QmCreateUserInput,
    pub group_id: Option<String>,
    pub access: Option<String>,
    pub context: Option<InfraContext>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct QmUser {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub firstname: Arc<str>,
    pub lastname: Arc<str>,
    pub enabled: bool,
}

pub type UserMap = HashMap<Arc<str>, Arc<QmUser>>;
pub type UserUidMap = HashMap<Uuid, Arc<QmUser>>;
pub type UserGroupMap = HashMap<Arc<str>, HashSet<Arc<str>>>;
pub type UserRoleMap = HashMap<Arc<str>, HashSet<Arc<str>>>;

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

#[derive(Debug)]
pub struct UserGroupMembership {
    pub group_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct QmUserList {
    pub items: Arc<[QmUserDetails]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone, SimpleObject)]
#[graphql(complex)]
pub struct QmUserDetails {
    #[graphql(flatten)]
    pub user: Arc<QmUser>,
    #[graphql(skip)]
    pub context: Option<InfraContext>,
    #[graphql(skip)]
    pub access: Option<qm_role::Access>,
    #[graphql(skip)]
    pub group: Option<Arc<GroupDetail>>,
}

impl PartialEqual<'_, InfraContext> for QmUserDetails {
    fn partial_equal(&'_ self, r: &'_ InfraContext) -> bool {
        if let Some(context) = self.context.as_ref() {
            match r {
                InfraContext::Customer(v) => context.has_customer(v),
                InfraContext::Organization(v) => context.has_organization(v),
                InfraContext::Institution(v) => context.has_institution(v),
            }
        } else {
            false
        }
    }
}

impl PartialEqual<'_, InstitutionId> for QmUserDetails {
    fn partial_equal(&'_ self, r: &'_ InstitutionId) -> bool {
        if let Some(context) = self.context.as_ref() {
            context.has_institution(r)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserGroup {
    pub group_id: Arc<str>,
    pub group_detail: Arc<GroupDetail>,
}
