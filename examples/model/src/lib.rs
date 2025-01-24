use async_graphql::{ComplexObject, InputObject, MaybeUndefined, SimpleObject};
use qm::entity::ids::{InstitutionResourceId, OrganizationResourceId, Owner, ID};
use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, SimpleObject, InputObject, Serialize, Deserialize)]
#[graphql(input_name = "PersonInput")]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub salutation: Option<String>,
    pub title: Option<String>,
    #[serde(default)]
    pub firstname: String,
    pub middlename: Option<String>,
    #[serde(default)]
    pub lastname: String,
    #[serde(default)]
    pub gender: String,
    #[serde(default)]
    pub citizenships: Option<Vec<String>>,
    pub confession: Option<String>,
    pub denomination: Option<String>,
    pub languages: Option<Vec<String>>,
    pub is_german_first_language: Option<bool>,
}

#[derive(Default, Debug, Clone, SimpleObject, InputObject, Serialize, Deserialize)]
#[graphql(input_name = "SimpleAddressInput")]
#[serde(rename_all = "camelCase")]
pub struct SimpleAddress {
    pub street: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub district: Option<String>,
    pub country: Option<String>,
}

#[derive(Default, Debug, Clone, SimpleObject, Serialize, Deserialize)]
#[graphql(complex)]
pub struct Employee {
    id: ID,
    #[graphql(skip)]
    owner: Owner,
    person: Person,
    address: Option<SimpleAddress>,
}

#[ComplexObject]
impl Employee {}

#[derive(SimpleObject)]
pub struct EmployeeList {
    pub items: Vec<Employee>,
    pub limit: Option<i64>,
    pub total: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, InputObject)]
pub struct UpdatePersonInput {
    pub salutation: MaybeUndefined<String>,
    pub title: MaybeUndefined<String>,
    pub firstname: Option<String>,
    pub middlename: MaybeUndefined<String>,
    pub lastname: Option<String>,
    pub gender: Option<String>,
    pub citizenships: MaybeUndefined<Vec<String>>,
    pub confession: MaybeUndefined<String>,
    pub denomination: MaybeUndefined<String>,
    pub languages: MaybeUndefined<Vec<String>>,
    pub is_german_first_language: MaybeUndefined<bool>,
}

#[derive(Debug, InputObject)]
pub struct CreateEmployeeInput {
    institution: OrganizationResourceId,
    person: Person,
    address: Option<SimpleAddress>,
}

#[derive(Debug, InputObject)]
pub struct UpdateEmployeeInput {
    id: InstitutionResourceId,
    person: Option<UpdatePersonInput>,
    address: MaybeUndefined<SimpleAddress>,
}

// target structure, implemented with proc macros

// use chrono::NaiveDateTime;
// use qm::entity::{entity, member};

// #[member]
// pub struct Person {
//     firstname: String,
//     middlename: Option<String>,
//     lastname: String,
// }

// #[member]
// pub struct Address {
//     street: Option<String>,
//     zip_code: Option<String>,
//     place: Option<String>,
// }

// #[entity]
// pub struct Employee {
//     person: Person,
//     address: Option<Address>,
// }

// #[entity]
// pub struct WorkTime {
//    from: NaiveDateTime,
//    to: Option<NaiveDateTime>,
// }

// #[entity]
// pub struct Office {
//     name: String,
//     address: Option<Address>,
// }

// #[entity]
// pub struct Employee {
//     person: Person,
//     address: Option<Address>,
// }

// #[entity]
// pub struct Appointment {
//     name: String,
// }

// qm::entity::m2m!(Appointment, Employee);
// qm::entity::m2m!(Appointment, Office);
// qm::entity::o2m!(Employee, WorkTime);
// qm::entity::o2o!(Employee, Office);
