use async_graphql::{Enum, InputObject, SimpleObject};
use qm_entity::ids::{InfraContext, InstitutionId, PartialEqual};
use qm_entity::IsAdmin;
use sqlx::types::Uuid;
use sqlx::FromRow;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::GroupDetail;

#[derive(Debug, serde::Deserialize)]
pub struct ApiClientEntityUpdate {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Option<Arc<str>>,
    pub first_name: Option<Arc<str>>,
    pub last_name: Option<Arc<str>>,
    pub realm_id: Option<Arc<str>>,
    pub enabled: bool,
}

impl ApiClientEntityUpdate {
    pub fn has_all_fields(&self) -> bool {
        self.email.is_some() && self.first_name.is_some() && self.last_name.is_some()
    }
}

pub struct TmpApiClient {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub firstname: Arc<str>,
    pub lastname: Arc<str>,
    pub groups: HashSet<Arc<str>>,
    pub roles: HashSet<Arc<str>>,
    pub enabled: bool,
}
pub type TmpApiClientMap = HashMap<Arc<str>, TmpApiClient>;

#[derive(Debug, FromRow)]
pub struct KcApiClientQuery {
    pub id: Option<String>,
    pub enabled: bool,
    pub full_scope_allowed: bool,
    pub client_id: Option<String>,
    pub not_before: Option<i32>,
    pub public_client: bool,
    pub secret: Option<String>,
    pub base_url: Option<String>,
    pub bearer_only: bool,
    pub management_url: Option<String>,
    pub surrogate_auth_required: bool,
    pub realm_id: Option<String>,
    pub protocol: Option<String>,
    pub node_rereg_timeout: Option<i32>,
    pub frontchannel_logout: bool,
    pub consent_required: bool,
    pub name: Option<String>,
    pub service_accounts_enabled: bool,
    pub client_authenticator_type: Option<String>,
    pub root_url: Option<String>,
    pub description: Option<String>,
    pub registration_token: Option<String>,
    pub standard_flow_enabled: bool,
    pub implicit_flow_enabled: bool,
    pub direct_access_grants_enabled: bool,
    pub always_display_in_console: bool,
}

impl KcApiClientQuery {
    pub fn has_all_fields(&self) -> bool {
        [
            self.id.as_ref(),
        ]
        .iter()
        .all(Option::is_some)
    }
}

#[derive(Debug, FromRow)]
pub struct KcApiClientGroupQuery {
    pub user_id: Option<String>,
    pub group_id: Option<String>,
}

impl KcApiClientGroupQuery {
    pub fn has_all_fields(&self) -> bool {
        [self.group_id.as_ref(), self.user_id.as_ref()]
            .iter()
            .all(Option::is_some)
    }
}

#[derive(Debug, FromRow)]
pub struct KcApiClientRoleQuery {
    pub user_id: Option<String>,
    pub role_id: Option<String>,
}

impl KcApiClientRoleQuery {
    pub fn has_all_fields(&self) -> bool {
        [self.user_id.as_ref(), self.role_id.as_ref()]
            .iter()
            .all(Option::is_some)
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Enum, Copy, Eq, PartialEq)]
pub enum RequiredApiClientAction {
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

impl std::fmt::Display for RequiredApiClientAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            RequiredApiClientAction::VerifyEmail => "VERIFY_EMAIL",
            RequiredApiClientAction::UpdateProfile => "UPDATE_PROFILE",
            RequiredApiClientAction::ConfigureTotp => "CONFIGURE_TOTP",
            RequiredApiClientAction::UpdatePassword => "UPDATE_PASSWORD",
            RequiredApiClientAction::TermsAndConditions => "TERMS_AND_CONDITIONS",
        }
        .to_string();
        write!(f, "{}", str)
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiClientInput {
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
    pub required_actions: Option<Vec<RequiredApiClientAction>>,
}

#[derive(Debug)]
pub struct CreateApiClientPayload {
    pub user: CreateApiClientInput,
    pub group_id: Option<String>,
    pub access: Option<String>,
    pub context: Option<InfraContext>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct ApiClient {
    pub id: Arc<str>,
    pub username: Arc<str>,
    pub email: Arc<str>,
    pub firstname: Arc<str>,
    pub lastname: Arc<str>,
    pub enabled: bool,
}

pub type ApiClientMap = HashMap<Arc<str>, Arc<ApiClient>>;
pub type ApiClientUidMap = HashMap<Uuid, Arc<ApiClient>>;
pub type ApiClientGroupMap = HashMap<Arc<str>, HashSet<Arc<str>>>;
pub type ApiClientRoleMap = HashMap<Arc<str>, HashSet<Arc<str>>>;

#[derive(Debug, serde::Deserialize)]
pub struct ApiClientRoleMappingUpdate {
    pub role_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug, serde::Deserialize)]
pub struct ApiClientGroupMembershipUpdate {
    pub group_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug)]
pub struct ApiClientGroupMembership {
    pub group_id: Arc<str>,
    pub user_id: Arc<str>,
}

#[derive(Debug, Clone, SimpleObject)]
pub struct ApiClientList {
    pub items: Arc<[ApiClientDetails]>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

impl IsAdmin for ApiClientDetails {
    fn is_admin(&self) -> bool {
        self.access
            .as_ref()
            .map(|a| a.ty().is_admin())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, SimpleObject)]
// #[graphql(complex)]
pub struct ApiClientDetails {
    #[graphql(flatten)]
    pub user: Arc<ApiClient>,
    #[graphql(skip)]
    pub context: Option<InfraContext>,
    #[graphql(skip)]
    pub access: Option<qm_role::Access>,
    #[graphql(skip)]
    pub group: Option<Arc<GroupDetail>>,
}

impl PartialEqual<'_, InfraContext> for ApiClientDetails {
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

impl PartialEqual<'_, InstitutionId> for ApiClientDetails {
    fn partial_equal(&'_ self, r: &'_ InstitutionId) -> bool {
        if let Some(context) = self.context.as_ref() {
            context.has_institution(r)
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct ApiClientGroup {
    pub group_id: Arc<str>,
    pub group_detail: Arc<GroupDetail>,
}
