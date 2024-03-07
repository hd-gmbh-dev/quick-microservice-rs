use std::sync::Arc;
pub mod graphql;
pub mod test_realm;

use std::fmt;
use std::fmt::{Debug, Formatter};

use crate::world::graphql::GraphQLQueryBuilder;
use async_graphql::Response;
use qm::customer::schema::customer::CustomerDB;
use qm_example_ctx::Storage;
use qm_example_server::schema::Schema;

#[derive(Clone)]
pub struct Ctx {
    pub store: Storage,
    pub schema: Schema,
    pub username: Option<Arc<str>>,
}

impl Debug for Ctx {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "Ctx(",)?;
        writeln!(
            f,
            "\tusername: \"{}\",",
            self.username.as_deref().unwrap_or("")
        )?;
        write!(f, ")")
    }
}

pub async fn init_context() -> anyhow::Result<Ctx> {
    let cleanup = std::env::var("CLEANUP_INFRA_BEFORE").as_deref() == Ok("true");
    let store = Storage::new().await?;
    if cleanup {
        store.customer_db().cleanup().await?;
    }
    test_realm::ensure(&store, cleanup).await?;
    let schema = qm_example_server::schema::SchemaBuilder::default().build(store.clone());
    Ok(Ctx {
        store,
        schema,
        username: None,
    })
}

pub struct DataSelector<'a> {
    val: &'a serde_json::Value,
}

impl<'a> DataSelector<'a> {
    pub fn new(val: &'a serde_json::Value) -> Self {
        Self { val }
    }

    pub fn get<T>(self, idx: T) -> Self
    where
        T: serde_json::value::Index,
    {
        if let Some(val) = self.val.get(idx) {
            DataSelector { val }
        } else {
            DataSelector { val: self.val }
        }
    }

    pub fn as_str(&self) -> &str {
        self.val.as_str().unwrap_or("")
    }

    pub fn into_inner(self) -> &'a serde_json::Value {
        self.val
    }
}

#[derive(cucumber::World, Debug, Default)]
pub struct World {
    pub ctx: Option<Ctx>,
    pub state: serde_json::Value,
    last_response: async_graphql::Response,
    last_response_data: serde_json::Value,
    pub last_query_name: &'static str,
}

impl World {
    pub async fn init(&mut self, ctx: Ctx) -> anyhow::Result<()> {
        self.ctx = Some(ctx);
        self.state = serde_json::Value::Null;
        self.last_response = async_graphql::Response::default();
        Ok(())
    }

    pub fn store(&self) -> &Storage {
        &self.ctx.as_ref().unwrap().store
    }

    pub fn schema(&self) -> &Schema {
        &self.ctx.as_ref().unwrap().schema
    }

    pub fn graphql(
        &mut self,
        query_name: &'static str,
        query: &'static str,
    ) -> GraphQLQueryBuilder<'_, 'static> {
        self.last_query_name = query_name;
        GraphQLQueryBuilder::new(self.schema(), query)
    }

    pub fn save_last_response(&mut self, response: Response) {
        self.last_response_data = response
            .data
            .clone()
            .into_json()
            .ok()
            .unwrap_or(serde_json::Value::Null);
        self.last_response = response;
    }

    pub fn last_response_data_field<'a, T>(&'a self, idx: T) -> DataSelector<'a>
    where
        T: serde_json::value::Index,
    {
        self.last_response_data_query_root().get(idx)
    }

    pub fn last_response_data_query_root<'a>(&'a self) -> DataSelector<'a> {
        self.last_response_data().get(&self.last_query_name)
    }

    pub fn last_response_data<'a>(&'a self) -> DataSelector<'a> {
        DataSelector::new(&self.last_response_data)
    }

    pub fn last_response_error_extensions(
        &mut self,
    ) -> anyhow::Result<&async_graphql::ErrorExtensionValues> {
        Ok(self
            .last_response
            .errors
            .first()
            .ok_or(anyhow::anyhow!("last response has no errors"))?
            .extensions
            .as_ref()
            .ok_or(anyhow::anyhow!(
                "last response does not have error extensions"
            ))?)
    }

    pub async fn switch_user(&mut self, username: String, password: String) -> anyhow::Result<()> {
        let Ctx { store, .. } = self.ctx.take().unwrap();
        let kc = store.keycloak().config();
        let admin_session = qm::keycloak::KeycloakSession::acquire_custom_realm(
            kc.address(),
            &username,
            &password,
            kc.realm(),
            "spa",
            "password",
            store.keycloak().http_client(),
        )
        .await?;
        let schema = qm_example_server::schema::SchemaBuilder::default()
            .with_access_token(admin_session.access_token())
            .build(store.clone());
        self.ctx = Some(Ctx {
            schema,
            store,
            username: Some(Arc::from(username)),
        });
        Ok(())
    }

    pub async fn clear_user(&mut self) -> anyhow::Result<()> {
        let Ctx { store, .. } = self.ctx.take().unwrap();
        let schema = qm_example_server::schema::SchemaBuilder::default().build(store.clone());
        self.ctx = Some(Ctx {
            schema,
            store,
            username: None,
        });
        Ok(())
    }
}
