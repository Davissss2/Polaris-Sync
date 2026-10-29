use actix_msgpack::MsgPack;
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use std::sync::Arc;

use crate::{
	core::{processor::WriteRequest, Core},
	server::{self, Auth},
};

#[post("/write")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<WriteRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: write");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	let request = request.0;

	if !core.queue().is_subscribed(request.client_id) {
		return HttpResponse::Unauthorized().body("Not subscribed");
	}

	core.processor().write(request);

	HttpResponse::Ok().body("Written changes successfully")
}
