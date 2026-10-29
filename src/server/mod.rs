use actix_msgpack::MsgPackConfig;
use actix_web::{
	http::header,
	web::{self, Data},
	App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use derive_from_one::FromOne;
use serde::{Deserialize, Serialize};
use std::{io::Result, net::TcpListener, sync::Arc};

use crate::{
	constants::MAX_PAYLOAD_SIZE,
	core::{changes::Changes, Core},
	project::ProjectDetails,
};

mod details;
mod exec;
mod home;
mod open;
mod read;
mod snapshot;
mod stop;
mod subscribe;
mod unsubscribe;
mod write;

#[derive(Debug, Clone, Serialize, FromOne)]
pub enum Message {
	SyncChanges(SyncChanges),
	SyncbackChanges(SyncbackChanges),
	SyncDetails(SyncDetails),
	ExecuteCode(ExecuteCode),
	Disconnect(Disconnect),
}

impl Message {
	pub fn is_change(&self) -> bool {
		matches!(self, Message::SyncChanges(_) | Message::SyncbackChanges(_))
	}
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncChanges(pub Changes);

#[derive(Debug, Clone, Serialize)]
pub struct SyncbackChanges();

#[derive(Debug, Clone, Serialize)]
pub struct SyncDetails(pub ProjectDetails);

#[derive(Debug, Clone, Serialize)]
pub struct ExecuteCode {
	pub code: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Disconnect {
	pub message: String,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AuthRequest {
	client_id: u32,
}

pub struct Server {
	core: Arc<Core>,
	host: String,
	port: u16,
	token: Option<String>,
}

impl Server {
	pub fn new(core: Arc<Core>, host: &str, port: u16, token: Option<String>) -> Self {
		Self {
			core,
			host: host.to_owned(),
			port,
			token,
		}
	}

	#[actix_web::main]
	pub async fn start(&self) -> Result<()> {
		let core = self.core.clone();
		let auth = Data::new(Auth::new(self.token.clone()));

		HttpServer::new(move || {
			let mut msgpack_config = MsgPackConfig::default();
			msgpack_config.limit(MAX_PAYLOAD_SIZE);

			App::new()
				.app_data(Data::new(core.clone()))
				.app_data(auth.clone())
				.app_data(msgpack_config)
				.service(details::main)
				.service(subscribe::main)
				.service(unsubscribe::main)
				.service(snapshot::main)
				.service(read::main)
				.service(write::main)
				.service(exec::main)
				.service(open::main)
				.service(stop::main)
				.service(home::main)
				.default_service(web::to(Self::default_redirect))
		})
		.backlog(0)
		.disable_signals()
		.bind((self.host.clone(), self.port))?
		.run()
		.await
	}

	async fn default_redirect() -> impl Responder {
		web::Redirect::to("/")
	}
}

pub fn is_port_free(host: &str, port: u16) -> bool {
	TcpListener::bind((host, port)).is_ok()
}

/// Session token used to authenticate clients that can modify the project
#[derive(Clone, Debug)]
pub struct Auth {
	token: Option<String>,
}

impl Auth {
	pub fn new(token: Option<String>) -> Self {
		Self { token }
	}

	/// Whether a token is required. Without one, any local process can
	/// talk to the server, which is why `serve --secure` exists
	pub fn is_enabled(&self) -> bool {
		self.token.is_some()
	}

	pub fn check(&self, request: &HttpRequest) -> bool {
		let Some(expected) = &self.token else {
			return true;
		};

		request
			.headers()
			.get(header::AUTHORIZATION)
			.and_then(|value| value.to_str().ok())
			.map(|value| value.strip_prefix("Bearer ").unwrap_or(value))
			.is_some_and(|value| value == expected)
	}
}

/// Returns a `401` response when the request is not authorized
pub fn unauthorized(request: &HttpRequest, auth: &Auth) -> Option<HttpResponse> {
	if auth.check(request) {
		return None;
	}

	Some(HttpResponse::Unauthorized().body(
		"Missing or invalid session token. Copy the token printed by `polaris serve --secure` into the Polaris plugin settings",
	))
}

pub fn get_free_port(host: &str, port: u16) -> u16 {
	let mut port = port;

	while !is_port_free(host, port) {
		port += 1;

		// This should never happen, but just in case
		if port == 65535 {
			break;
		}
	}

	port
}

pub fn format_address(host: &str, port: u16) -> String {
	format!("http://{host}:{port}")
}
