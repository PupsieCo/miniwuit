pub mod manager;
pub mod service;
pub mod services;

extern crate conduwuit_core as conduwuit;
extern crate conduwuit_database as database;

pub use crate::service::Service;
pub use crate::service::Args;
pub use crate::service::Dep;
pub use crate::service::Map;
pub use crate::service::MapType;
pub use crate::service::MapKey;
pub use crate::service::MapVal;
pub use crate::services::ServicesTrait;
pub use crate::manager::Manager;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}
