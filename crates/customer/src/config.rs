use async_graphql::Context;

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
        let cfg: Config = if let Some(prefix) = self.prefix {
            envy::prefixed(prefix)
        } else {
            envy::prefixed("QM_CUSTOMER_")
        }
        .from_env()?;
        Ok(cfg)
    }
}

#[derive(Clone, serde::Deserialize, Debug)]
pub struct Config {
    #[serde(default)]
    allow_multiple_admin_users: bool,
}

impl Config {
    pub fn new() -> envy::Result<Self> {
        ConfigBuilder::default().build()
    }

    pub fn builder<'a>() -> ConfigBuilder<'a> {
        ConfigBuilder::default()
    }
}

pub struct SchemaConfig<'a>(Option<&'a Config>);

impl<'a> SchemaConfig<'a> {
    pub fn new(ctx: &Context<'a>) -> Self {
        SchemaConfig(ctx.data_opt())
    }

    pub fn allow_multiple_admin_users(&self) -> bool {
        self.0
            .map(|v| v.allow_multiple_admin_users)
            .unwrap_or(false)
    }
}
