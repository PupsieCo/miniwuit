#![type_length_limit = "2048"]
#![allow(refining_impl_trait)]

mod manager;
mod migrations;
mod service;
pub mod services;
pub mod config;


extern crate conduwuit_core as conduwuit;
extern crate conduwuit_database as database;

pub(crate) use service::{Args, Dep, Service};

pub use crate::services::Services;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}
