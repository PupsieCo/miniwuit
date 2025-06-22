#![type_length_limit = "16384"] //TODO: reduce me
#![allow(clippy::toplevel_ref_arg)]

pub mod client;
pub mod router;
pub mod server;

extern crate conduwuit_core as conduwuit;
extern crate conduwuit_service as service;

pub(crate) use self::router::{Ruma, RumaResponse, State};

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
