use conduwuit::{Result, implement};
use ruma::{CanonicalJsonObject, RoomVersionId};

#[implement(super::Service)]
pub fn sign_json(&self, object: &mut CanonicalJsonObject) -> Result {
	use ruma::signatures::sign_json;

	let server_name = self.services.globals.server_name().as_str();
	sign_json(server_name, self.keypair(), object).map_err(Into::into)
}

#[implement(super::Service)]
pub fn hash_and_sign_event(
	&self,
	object: &mut CanonicalJsonObject,
	room_version: &RoomVersionId,
) -> Result {
	use ruma::signatures::hash_and_sign_event;

	let server_name = self.services.globals.server_name().as_str();
	hash_and_sign_event(server_name, self.keypair(), object, room_version).map_err(Into::into)
}
