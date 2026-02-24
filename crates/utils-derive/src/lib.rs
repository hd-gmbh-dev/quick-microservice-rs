#![deny(missing_docs)]

//! Procedural macros for utility functions.
//!
//! This crate provides procedural macros for generating utility code.
//!
//! ## Macros
//!
//! - [`cheap_clone`] - Derive macro for cheap clone operations
//!
//! ## Usage
//!
//! \```ignore
//! use qm_utils_derive::CheapClone;
//!
//! #[derive(CheapClone)]
//! struct MyStruct {
//!     data: Arc<Data>,
//! }
//! \```

use proc_macro::TokenStream;

mod cheap_clone;

/// Derive macro for generating cheap clone implementations.
///
/// This macro generates a `cheap_clone` method that clones internal `Arc`
/// fields without cloning the entire struct. Useful for types that are
/// cheap to share via `Arc` but expensive to clone directly.
#[proc_macro_derive(CheapClone)]
pub fn cheap_clone(item: TokenStream) -> TokenStream {
    cheap_clone::expand(item)
}
