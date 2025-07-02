pub mod router;

extern crate conduwuit_core as conduwuit;
extern crate conduwuit_database as database;

pub use router::ServiceApiRouter;

conduwuit::mod_ctor! {}
conduwuit::mod_dtor! {}
conduwuit::rustc_flags_capture! {}
