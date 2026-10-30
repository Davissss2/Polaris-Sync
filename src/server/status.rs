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
}

/// Reports which editors are attached to this session, so `list` and the
/// editor extension can tell "server up" from "Studio actually connected"
#[get("/status")]
async fn main(core: Data<Arc<Core>>) -> impl Responder {
	let clients = core.queue().clients();
	let pending = core.consent().get();

	HttpResponse::Ok().json(Status { clients, pending })
}
