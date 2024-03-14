use async_graphql::{EmptySubscription, MergedObject, Object};
use qm::{
    customer::schema::{QmCustomerMutationRoot, QmCustomerQueryRoot},
    entity::ids::InstitutionResourceId,
    role::AuthContainer,
};

use qm_example_auth::{
    roles::{AccessLevel, BuiltInGroup, Permission, Resource},
    Authorization,
};
use qm_example_ctx::Storage;

pub type Schema = async_graphql::Schema<QueryRoot, MutationRoot, EmptySubscription>;

#[derive(Default)]
pub struct DomainQueryRoot {}

#[Object]
impl DomainQueryRoot {
    async fn employee_by_id(
        &self,
        _id: InstitutionResourceId,
    ) -> async_graphql::FieldResult<qm_example_model::Employee> {
        unimplemented!()
    }

    async fn employees(&self) -> async_graphql::FieldResult<qm_example_model::EmployeeList> {
        unimplemented!()
    }
}

#[derive(Default)]
pub struct DomainMutationRoot {}

#[Object]
impl DomainMutationRoot {
    async fn create_employee(
        &self,
        _input: qm_example_model::CreateEmployeeInput,
    ) -> async_graphql::FieldResult<qm_example_model::Employee> {
        unimplemented!()
    }

    async fn update_employee(
        &self,
        _input: qm_example_model::UpdateEmployeeInput,
    ) -> async_graphql::FieldResult<qm_example_model::Employee> {
        unimplemented!()
    }

    async fn remove_employees(
        &self,
        _ids: Vec<InstitutionResourceId>,
    ) -> async_graphql::FieldResult<usize> {
        Ok(0)
    }
}

#[derive(MergedObject, Default)]
pub struct QueryRoot(
    QmCustomerQueryRoot<Authorization, Storage, AccessLevel, Resource, Permission, BuiltInGroup>,
    DomainQueryRoot,
);

#[derive(MergedObject, Default)]
pub struct MutationRoot(
    QmCustomerMutationRoot<Authorization, Storage, AccessLevel, Resource, Permission, BuiltInGroup>,
    DomainMutationRoot,
);

#[derive(Default)]
pub struct SchemaBuilder {
    access_token: Option<String>,
}

impl SchemaBuilder {
    pub fn with_access_token(mut self, access_token: &str) -> Self {
        self.access_token = Some(access_token.to_string());
        self
    }

    pub fn build(self, store: Storage) -> Schema {
        let mut s = async_graphql::Schema::build(
            QueryRoot::default(),
            MutationRoot::default(),
            EmptySubscription,
        )
        .data(store);
        if let Some(access_token) = self.access_token {
            s = s.data(AuthContainer::<Authorization>::new(&access_token));
        } else {
            s = s.data(AuthContainer::<Authorization>::default());
        }
        s.finish()
    }
}
