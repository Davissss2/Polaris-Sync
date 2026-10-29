use actix_msgpack::{MsgPack, MsgPackResponseBuilder};
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use std::sync::Arc;

use crate::{
	core::Core,
	server::{self, Auth, AuthRequest},
};

#[post("/read")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<AuthRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: read");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	let id = request.client_id;
	let queue = core.queue();

	if !queue.is_subscribed(id) {
		return HttpResponse::Unauthorized().body("Not subscribed");
	}

	match queue.get_timeout(id) {
		Ok(message) => HttpResponse::Ok().msgpack(message),
		Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
	}
}
