use actix_msgpack::{MsgPack, MsgPackResponseBuilder};
use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::trace;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
	core::{consent::Pending, Core},
	server::{self, Auth},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct AnnounceRequest {
	client_id: u32,
	kind: String,
	message: String,
}

/// The plugin tells the server which confirmation it is waiting on, so the
/// prompt stops being invisible from outside Studio
#[post("/consent")]
pub async fn announce(
	http_request: HttpRequest,
	request: MsgPack<AnnounceRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: consent");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	core.consent().announce(Pending {
		client_id: request.client_id,
		kind: request.kind.clone(),
		message: request.message.clone(),
		decision: None,
	});

	HttpResponse::Ok().finish()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClientRequest {
	client_id: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecisionReply {
	decision: Option<String>,
}

/// The plugin polls this while the prompt is open. An answer from the interface
/// is not visible here, which is what the clear endpoint is for
#[post("/consent/read")]
pub async fn read_pending(
	http_request: HttpRequest,
	request: MsgPack<ClientRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: consent read");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	HttpResponse::Ok().msgpack(DecisionReply {
		decision: core.consent().decision_for(request.client_id),
	})
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DecideRequest {
	client_id: u32,
	accept: bool,
}

/// Answers the pending prompt, which is how `polaris consent` unblocks a
/// session without a click
#[post("/consent/decide")]
pub async fn decide(
	http_request: HttpRequest,
	request: MsgPack<DecideRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: consent decide");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	if core.consent().decide(request.client_id, request.accept) {
		HttpResponse::Ok().finish()
	} else {
		HttpResponse::NotFound().body("There is no pending prompt for this client")
	}
}

/// Forgets the prompt after the interface answered it, so `polaris consent`
/// does not report a question that is already gone
#[post("/consent/clear")]
pub async fn clear(
	http_request: HttpRequest,
	request: MsgPack<ClientRequest>,
	core: Data<Arc<Core>>,
	auth: Data<Auth>,
) -> HttpResponse {
	trace!("Received request: consent clear");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	if core.consent().clear(request.client_id) {
		HttpResponse::Ok().finish()
	} else {
		HttpResponse::NotFound().body("There is no pending prompt for this client")
	}
}
