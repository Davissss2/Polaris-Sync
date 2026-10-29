use actix_msgpack::MsgPack;
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use rbx_dom_weak::types::Ref;
use serde::Deserialize;
use std::sync::Arc;

use crate::{
	core::Core,
	server::{self, Auth},
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Request {
	instance: Ref,
	_line: u32,
}

#[post("/open")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<Request>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: open");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	match core.open(request.instance) {
		Ok(_) => HttpResponse::Ok().body("Opened file successfully"),
		Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
	}
}
