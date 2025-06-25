#![type_length_limit = "2048"]
#![allow(refining_impl_trait)]

pub mod manager;
pub mod service;
pub mod services;
pub mod config;



extern crate conduwuit_core as conduwuit;
extern crate conduwuit_database as database;

pub use crate::service::Service;
pub use crate::services::Services;
pub use crate::services::ServicesTrait;
pub use crate::manager::Manager;
pub use crate::service::Args;
pub use crate::service::Dep;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}
