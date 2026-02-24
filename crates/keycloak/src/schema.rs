/// Required user action enum.
#[derive(Debug, PartialEq, Eq)]
pub enum RequiredUserAction {
    /// User must update password.
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

/// User input for creating users.
#[derive(Debug)]
pub struct UserInput {
    /// Username.
    pub username: String,
    /// First name.
    pub firstname: String,
    /// Last name.
    pub lastname: String,
    /// Password.
    pub password: String,
    /// Email.
    pub email: String,
    /// Phone number.
    pub phone: Option<String>,
    /// Salutation.
    pub salutation: Option<String>,
    /// Fax number.
    pub fax: Option<String>,
    /// Room number.
    pub room_number: Option<String>,
    /// Job title.
    pub job_title: Option<String>,
    /// Whether user is enabled.
    pub enabled: Option<bool>,
    /// Required user actions.
    pub required_actions: Option<Vec<RequiredUserAction>>,
}
