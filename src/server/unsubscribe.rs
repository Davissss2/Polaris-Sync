use actix_msgpack::MsgPack;
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use std::sync::Arc;

use crate::{
	core::Core,
	server::{self, Auth, AuthRequest},
};

#[post("/unsubscribe")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<AuthRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: unsubscribe");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	let unsubscribed = core.queue().unsubscribe(request.client_id);

	if unsubscribed.is_ok() {
		HttpResponse::Ok().body("Unsubscribed successfully")
	} else {
		HttpResponse::BadRequest().body("Not subscribed")
	}
}
