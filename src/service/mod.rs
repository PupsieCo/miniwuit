#![type_length_limit = "2048"]
#![allow(refining_impl_trait)]

pub mod services;
pub mod config;



extern crate conduwuit_core as conduwuit;
extern crate conduwuit_database as database;
extern crate conduwuit_service_core as service;

pub use crate::services::Services;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}
