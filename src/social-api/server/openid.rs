use axum::extract::State;
use conduwuit::Result;
use ruma::api::federation::openid::get_openid_userinfo;

use crate::Ruma;

/// # `GET /_matrix/federation/v1/openid/userinfo`
///
/// Get information about the user that generated the OpenID token.
pub(crate) async fn get_openid_userinfo_route(
	State(services): State<crate::State>,
	body: Ruma<get_openid_userinfo::v1::Request>,
) -> Result<get_openid_userinfo::v1::Response> {
	Ok(get_openid_userinfo::v1::Response::new(
		services
			.users
			.find_from_openid_token(&body.access_token)
			.await?,
	))
}
