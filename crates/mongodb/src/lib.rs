pub use mongodb::*;

mod config;
mod db;

pub use crate::config::Config as DbConfig;
pub use crate::db::{insert_always_opts, parse_vec, DB};
