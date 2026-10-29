use actix_web::{post, web::Data, HttpRequest, HttpResponse};
use log::{info, trace};
use std::process;

use crate::{
	server::{self, Auth},
	util,
};

#[post("/stop")]
async fn main(http_request: HttpRequest, auth: Data<Auth>) -> HttpResponse {
	trace!("Received request: stop");

	if let Some(response) = server::unauthorized(&http_request, &auth) {
		return response;
	}

	info!("Stopping Polaris!");

	util::kill_process(process::id());

	HttpResponse::Ok().body("Polaris stopped successfully")
}
