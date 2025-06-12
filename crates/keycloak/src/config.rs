use std::sync::Arc;

#[derive(Default)]
pub struct ConfigBuilder<'a> {
    prefix: Option<&'a str>,
}

impl<'a> ConfigBuilder<'a> {
    pub fn with_prefix(mut self, prefix: &'a str) -> Self {
        self.prefix = Some(prefix);
        self
    }

    pub fn build(self) -> envy::Result<Config> {
        let mut cfg: Config = if let Some(prefix) = self.prefix {
            envy::prefixed(prefix)
        } else {
            envy::prefixed("KEYCLOAK_")
        }
        .from_env()?;
        if cfg.realm.is_none() {
            cfg.realm = Some("rmp".into());
        }
        if cfg.username.is_none() {
            cfg.username = Some("admin".into());
        }
        if cfg.password.is_none() {
            cfg.password = Some("admin".into());
        }
        if cfg.realm_admin_email.is_none() {
            cfg.realm_admin_email = Some("admin@test.local".into());
        }
        if cfg.realm_admin_username.is_none() {
            cfg.realm_admin_username = Some("admin".into());
        }
        if cfg.realm_admin_password.is_none() {
            cfg.realm_admin_password = Some("Admin123!".into());
        }
        if cfg.address.is_none() {
            let host = cfg.host.as_deref().unwrap_or("127.0.0.1");
            let port = cfg.port.unwrap_or(42210);
            cfg.address = Some(Arc::from(format!("http://{}:{}/", host, port)));
        }
        if cfg.smtp_starttls.is_none() {
            cfg.smtp_starttls = Some(false);
        }
        if cfg.smtp_port.is_none() {
            cfg.port = Some(1025);
        }
        if cfg.smtp_host.is_none() {
            cfg.smtp_host = Some("smtp".into());
        }
        if cfg.smtp_from.is_none() {
            cfg.smtp_from = Some("noreply@test.local".into());
        }
        if cfg.smtp_ssl.is_none() {
            cfg.smtp_ssl = Some(false);
        }
        if cfg.browser_flow.is_none() {
            cfg.browser_flow = Some("browser".into());
        }
        if cfg.authenticator_email_subject.is_none() {
            cfg.authenticator_email_subject = Some("Temporary Authentication Code".into());
        }

        Ok(cfg)
    }
}

#[derive(Clone, serde::Deserialize, Debug)]
pub struct Config {
    realm: Option<Arc<str>>,
    username: Option<Arc<str>>,
    password: Option<Arc<str>>,
    theme: Option<Arc<str>>,
    email_theme: Option<Arc<str>>,
    realm_admin_email: Option<Arc<str>>,
    realm_admin_username: Option<Arc<str>>,
    realm_admin_password: Option<Arc<str>>,
    port: Option<u16>,
    host: Option<Arc<str>>,
    address: Option<Arc<str>>,
    public_url: Option<Arc<str>>,
    smtp_reply_to_display_name: Option<Arc<str>>,
    smtp_starttls: Option<bool>,
    smtp_port: Option<u16>,
    smtp_host: Option<Arc<str>>,
    smtp_reply_to: Option<Arc<str>>,
    smtp_from: Option<Arc<str>>,
    smtp_from_display_name: Option<Arc<str>>,
    smtp_ssl: Option<bool>,
    browser_flow: Option<Arc<str>>,
    authenticator_email_subject: Option<Arc<str>>,
    keystore_password: Option<Arc<str>>,
    duplicate_emails_allowed: Option<bool>,
}

impl Config {
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }

    pub fn realm(&self) -> &str {
        self.realm.as_deref().unwrap_or("rmp")
    }

    pub fn theme(&self) -> &str {
        self.theme.as_deref().unwrap_or("qm")
    }

    pub fn email_theme(&self) -> &str {
        self.email_theme.as_deref().unwrap_or("qm")
    }

    pub fn realm_admin_username(&self) -> &str {
        self.realm_admin_username.as_deref().unwrap_or("admin")
    }
    pub fn realm_admin_password(&self) -> &str {
        self.realm_admin_password.as_deref().unwrap_or("Admin123!")
    }
    pub fn realm_admin_email(&self) -> &str {
        self.realm_admin_email
            .as_deref()
            .unwrap_or("admin@test.local")
    }

    pub fn address(&self) -> &str {
        self.address.as_deref().unwrap_or("http://127.0.0.1:42210")
    }

    pub fn public_url(&self) -> &str {
        self.public_url.as_deref().unwrap_or("http://127.0.0.1:80")
    }

    pub fn username(&self) -> &str {
        self.username.as_deref().unwrap_or("admin")
    }

    pub fn password(&self) -> &str {
        self.password.as_deref().unwrap_or("admin")
    }

    pub fn smtp_reply_to_display_name(&self) -> Option<&str> {
        self.smtp_reply_to_display_name.as_deref()
    }

    pub fn smtp_starttls(&self) -> Option<&bool> {
        self.smtp_starttls.as_ref()
    }

    pub fn smtp_port(&self) -> Option<&u16> {
        self.smtp_port.as_ref()
    }

    pub fn smtp_host(&self) -> Option<&str> {
        self.smtp_host.as_deref()
    }

    pub fn smtp_reply_to(&self) -> Option<&str> {
        self.smtp_reply_to.as_deref()
    }

    pub fn smtp_from(&self) -> Option<&str> {
        self.smtp_from.as_deref()
    }

    pub fn smtp_from_display_name(&self) -> Option<&str> {
        self.smtp_from_display_name.as_deref()
    }

    pub fn smtp_ssl(&self) -> Option<&bool> {
        self.smtp_ssl.as_ref()
    }

    pub fn browser_flow(&self) -> &str {
        self.browser_flow.as_deref().unwrap_or("browser")
    }

    pub fn authenticator_email_subject(&self) -> Option<&str> {
        self.authenticator_email_subject.as_deref()
    }

    pub fn keystore_password(&self) -> Option<&str> {
        self.keystore_password.as_deref()
    }

    pub fn duplicate_emails_allowed(&self) -> bool {
        self.duplicate_emails_allowed.unwrap_or_default()
    }
}
