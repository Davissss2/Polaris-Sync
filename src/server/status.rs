use actix_web::{get, web::Data, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Arc;

use crate::core::{consent::Pending, Core};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
	clients: Vec<String>,
	/// Confirmation the plugin is waiting on, so `list` can explain why a
	/// session is not syncing instead of just showing an empty client list
	pending: Option<Pending>,
	/// Place the attached editor is on, which is how `list` tells two sessions
	/// apart when they use the same project name
	#[serde(skip_serializing_if = "Option::is_none")]
	place_id: Option<u64>,
}

/// Reports which editors are attached to this session, so `list` and the
/// editor extension can tell "server up" from "Studio actually connected"
#[get("/status")]
async fn main(core: Data<Arc<Core>>) -> impl Responder {
	let clients = core.queue().clients();
	let pending = core.consent().get();
	let place_id = core.place();

	HttpResponse::Ok().json(Status {
		clients,
		pending,
		place_id,
	})
}
