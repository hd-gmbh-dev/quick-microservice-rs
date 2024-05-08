use async_graphql::{Enum, InputObject};

#[derive(serde::Deserialize, serde::Serialize, Debug, Clone, Enum, Copy, Eq, PartialEq)]
pub enum RequiredUserAction {
    #[graphql(name = "UPDATE_PASSWORD")]
    UpdatePassword,
}

impl std::fmt::Display for RequiredUserAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                RequiredUserAction::UpdatePassword => "UPDATE_PASSWORD",
            }
        )
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, Clone, InputObject)]
#[serde(rename_all = "camelCase")]
pub struct UserInput {
    pub username: String,
    pub firstname: String,
    pub lastname: String,
    pub password: String,
    pub email: String,
    pub phone: Option<String>,
    pub salutation: Option<String>,
    pub fax: Option<String>,
    pub room_number: Option<String>,
    pub job_title: Option<String>,
    pub enabled: Option<bool>,
    pub required_actions: Option<Vec<RequiredUserAction>>,
}
