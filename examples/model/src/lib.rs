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