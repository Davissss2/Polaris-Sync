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

	let subscribed = core.queue().subscribe(request.client_id, &request.name);

	if subscribed.is_ok() {
		HttpResponse::Ok().body("Subscribed successfully")
	} else {
		HttpResponse::BadRequest().body("Already subscribed")
	}
}
