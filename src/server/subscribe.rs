use actix_msgpack::MsgPack;
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
	core::Core,
	server::{self, Auth},
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Request {
	client_id: u32,
	name: String,
	/// Editors that predate the place check do not send these, and are let
	/// through: refusing them would break every plugin in the wild
	#[serde(default)]
	place_id: Option<u64>,
	#[serde(default)]
	game_id: Option<u64>,
}

#[post("/subscribe")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<Request>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: subscribe");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	// The plugin chooses what to apply, but a project that is bound to another
	// place must never be served to this editor, not even by mistake
	{
		let project = core.project();

		if let Err(reason) = project.accepts_place(request.place_id, request.game_id) {
			trace!("Rejected subscription from {}: {reason}", request.name);

			return HttpResponse::Conflict().body(reason);
		}
	}

	if let Some(place_id) = request.place_id {
		core.note_place(place_id);
	}

	let subscribed = core.queue().subscribe(request.client_id, &request.name);

	if subscribed.is_ok() {
		HttpResponse::Ok().body("Subscribed successfully")
	} else {
		HttpResponse::BadRequest().body("Already subscribed")
	}
}
