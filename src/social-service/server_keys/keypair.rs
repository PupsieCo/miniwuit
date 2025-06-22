use std::sync::Arc;

use conduwuit::{Result, debug, debug_info, err, error, utils, utils::string_from_bytes};
use database::Database;
use ruma::{api::federation::discovery::VerifyKey, serde::Base64, signatures::Ed25519KeyPair};

use super::VerifyKeys;

pub(super) fn init(db: &Arc<Database>) -> Result<(Box<Ed25519KeyPair>, VerifyKeys)> {
	let keypair = load(db).inspect_err(|_e| {
		error!("Keypair invalid. Deleting...");
		remove(db);
	})?;

	let verify_key = VerifyKey {
		key: Base64::new(keypair.public_key().to_vec()),
	};

	let id = format!("ed25519:{}", keypair.version());
	let verify_keys: VerifyKeys = [(id.try_into()?, verify_key)].into();

	Ok((keypair, verify_keys))
}

fn load(db: &Arc<Database>) -> Result<Box<Ed25519KeyPair>> {
	let (version, key) = db["global"]
		.get_blocking(b"keypair")
		.map(|ref val| {
			// database deserializer is having trouble with this so it's manual for now
			let mut elems = val.split(|&b| b == b'\xFF');
			let vlen = elems.next().expect("invalid keypair entry").len();
			let ver = string_from_bytes(&val[..vlen]).expect("invalid keypair version");
			let der = val[vlen.saturating_add(1)..].to_vec();
			debug!("Found existing Ed25519 keypair: {ver:?}");
			(ver, der)
		})
		.or_else(|e| {
			assert!(e.is_not_found(), "unexpected error fetching keypair");
			create(db)
		})?;

	let key = Ed25519KeyPair::from_der(&key, version)
		.map_err(|e| err!("Failed to load ed25519 keypair from der: {e:?}"))?;

	Ok(Box::new(key))
}

fn create(db: &Arc<Database>) -> Result<(String, Vec<u8>)> {
	let keypair = Ed25519KeyPair::generate()
		.map_err(|e| err!("Failed to generate new ed25519 keypair: {e:?}"))?;

	let id = utils::rand::string(8);
	debug_info!("Generated new Ed25519 keypair: {id:?}");

	let value: (String, Vec<u8>) = (id, keypair.to_vec());
	db["global"].raw_put(b"keypair", &value);

	Ok(value)
}

#[inline]
fn remove(db: &Arc<Database>) {
	let global = &db["global"];
	global.remove(b"keypair");
}
