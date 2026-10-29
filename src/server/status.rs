use actix_web::{get, web::Data, HttpResponse, Responder};
use serde::Serialize;
use std::sync::Arc;

use crate::core::Core;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Status {
	clients: Vec<String>,
}

/// Reports which editors are attached to this session, so `list` and the
/// editor extension can tell "server up" from "Studio actually connected"
#[get("/status")]
async fn main(core: Data<Arc<Core>>) -> impl Responder {
	let clients = core.queue().clients();

	HttpResponse::Ok().json(Status { clients })
}
