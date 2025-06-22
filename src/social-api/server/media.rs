use axum::extract::State;
use axum_client_ip::InsecureClientIp;
use conduwuit::{Err, Result, utils::content_disposition::make_content_disposition};
use conduwuit_service::media::{Dim, FileMeta};
use ruma::{
	Mxc,
	api::federation::authenticated_media::{
		Content, ContentMetadata, FileOrLocation, get_content, get_content_thumbnail,
	},
};

use crate::Ruma;

/// # `GET /_matrix/federation/v1/media/download/{mediaId}`
///
/// Load media from our server.
#[tracing::instrument(
	name = "media_get",
	level = "debug",
	skip_all,
	fields(%client)
)]
pub(crate) async fn get_content_route(
	State(services): State<crate::State>,
	InsecureClientIp(client): InsecureClientIp,
	body: Ruma<get_content::v1::Request>,
) -> Result<get_content::v1::Response> {
	let mxc = Mxc {
		server_name: services.globals.server_name(),
		media_id: &body.media_id,
	};

	let Some(FileMeta {
		content,
		content_type,
		content_disposition,
	}) = services.media.get(&mxc).await?
	else {
		return Err!(Request(NotFound("Media not found.")));
	};

	let content_disposition =
		make_content_disposition(content_disposition.as_ref(), content_type.as_deref(), None);
	let content = Content {
		file: content.expect("entire file contents"),
		content_type: content_type.map(Into::into),
		content_disposition: Some(content_disposition),
	};

	Ok(get_content::v1::Response {
		content: FileOrLocation::File(content),
		metadata: ContentMetadata::new(),
	})
}

/// # `GET /_matrix/federation/v1/media/thumbnail/{mediaId}`
///
/// Load media thumbnail from our server.
#[tracing::instrument(
	name = "media_thumbnail_get",
	level = "debug",
	skip_all,
	fields(%client)
)]
pub(crate) async fn get_content_thumbnail_route(
	State(services): State<crate::State>,
	InsecureClientIp(client): InsecureClientIp,
	body: Ruma<get_content_thumbnail::v1::Request>,
) -> Result<get_content_thumbnail::v1::Response> {
	let dim = Dim::from_ruma(body.width, body.height, body.method.clone())?;
	let mxc = Mxc {
		server_name: services.globals.server_name(),
		media_id: &body.media_id,
	};

	let Some(FileMeta {
		content,
		content_type,
		content_disposition,
	}) = services.media.get_thumbnail(&mxc, &dim).await?
	else {
		return Err!(Request(NotFound("Media not found.")));
	};

	let content_disposition =
		make_content_disposition(content_disposition.as_ref(), content_type.as_deref(), None);
	let content = Content {
		file: content.expect("entire file contents"),
		content_type: content_type.map(Into::into),
		content_disposition: Some(content_disposition),
	};

	Ok(get_content_thumbnail::v1::Response {
		content: FileOrLocation::File(content),
		metadata: ContentMetadata::new(),
	})
}
