use actix_msgpack::MsgPack;
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::{error, trace};
use serde::Deserialize;
use std::sync::Arc;

use crate::{
	core::Core,
	server::{self, Auth},
	studio,
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Request {
	code: String,
	focus: bool,
}

#[post("/exec")]
async fn main(
	http_request: HttpRequest,
	request: MsgPack<Request>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: exec");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	let queue = core.queue();

	let pushed = queue.push(
		server::ExecuteCode {
			code: request.code.clone(),
		},
		None,
	);

	if request.focus {
		if let Some(name) = queue.get_first_non_internal_listener_name() {
			match studio::focus(Some(name)) {
				Ok(()) => (),
				Err(err) => error!("Failed to focus Roblox Studio: {err}"),
			}
		}
	}

	match pushed {
		Ok(()) => HttpResponse::Ok().body("Code executed successfully"),
		Err(err) => HttpResponse::InternalServerError().body(err.to_string()),
	}
}
