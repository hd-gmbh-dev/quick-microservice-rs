//! Procedural macros for entity relationships.
//!
//! This crate provides procedural macros for generating many-to-many,
//! one-to-many, and one-to-one relationship code for entities.
//!
//! ## Macros
//!
//! - [`m2m`] - Generate many-to-many relationship code
//! - [`o2m`] - Generate one-to-many relationship code
//! - [`o2o`] - Generate one-to-one relationship code
//!
//! ## Usage
//!
//! \```ignore
//! use qm_entity_derive::{m2m, o2m, o2o};
//!
//! // Many-to-many relationship
//! m2m!(User, Group);
//!
//! // One-to-many relationship
//! o2m!(Parent, Child);
//!
//! // One-to-one relationship
//! o2o!(User, Profile);
//! \```

use proc_macro::TokenStream;

mod m2m;
mod o2m;
mod o2o;

/// Generate many-to-many relationship code.
///
/// Takes two entity types as input and generates the boilerplate
/// for managing a many-to-many relationship between them.
#[proc_macro]
pub fn m2m(item: TokenStream) -> TokenStream {
    m2m::expand(item)
}

/// Generate one-to-many relationship code.
///
/// Takes two entity types as input and generates the boilerplate
/// for managing a one-to-many relationship between them.
#[proc_macro]
pub fn o2m(item: TokenStream) -> TokenStream {
    o2m::expand(item)
}

/// Generate one-to-one relationship code.
///
/// Takes two entity types as input and generates the boilerplate
/// for managing a one-to-one relationship between them.
#[proc_macro]
pub fn o2o(item: TokenStream) -> TokenStream {
    o2o::expand(item)
}
