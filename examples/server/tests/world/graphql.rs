use async_graphql::{Request, Response, Variables};
use qm_example_server::schema::Schema;

pub struct GraphQLQueryBuilder<'s, 'a> {
    schema: &'s Schema,
    query: &'a str,
    variables: Option<serde_json::Value>,
}

impl<'s, 'a> GraphQLQueryBuilder<'s, 'a> {
    pub fn new(schema: &'s Schema, query: &'a str) -> Self {
        Self {
            schema,
            query,
            variables: None,
        }
    }

    // pub fn variables(mut self, variables: serde_json::Value) -> Self {
    //     self.variables = Some(variables);
    //     self
    // }

    pub fn var(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.variables
            .get_or_insert(serde_json::json!({}))
            .as_object_mut()
            .unwrap()
            .insert(key.into(), value);
        self
    }

    pub async fn execute(self) -> Response {
        let mut request = Request::new(self.query);
        if let Some(variables) = self.variables {
            request = request.variables(Variables::from_json(variables));
        }
        self.schema.execute(request).await
    }
}
