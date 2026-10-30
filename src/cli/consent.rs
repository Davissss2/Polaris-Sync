use anyhow::{bail, Context, Result};
use clap::Parser;
use reqwest::{
	blocking::Client,
	header::{AUTHORIZATION, CONTENT_TYPE},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::{argon_info, cli::Cli, core::consent::Pending, sessions, util};

/// Report or answer the confirmation a session is waiting on
#[derive(Parser)]
pub struct Consent {
	/// Accept the pending prompt
	#[arg(long, conflicts_with = "reject")]
	accept: bool,

	/// Reject the pending prompt
	#[arg(long)]
	reject: bool,

	/// Session identifier
	#[arg()]
	session: Option<String>,

	/// Server host name
	#[arg(short = 'H', long)]
	host: Option<String>,

	/// Server port
	#[arg(short = 'P', long)]
	port: Option<u16>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusReply {
	/// Older servers do not send this field at all
	#[serde(default)]
	pending: Option<Pending>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecideRequest {
	client_id: u32,
	accept: bool,
}

impl Consent {
	pub fn main(self) -> Result<()> {
		let session = sessions::get(self.session, self.host, self.port)?.context("No running session was found")?;
		let address = session
			.get_address()
			.context("The session does not have an address to talk to")?;

		let pending = fetch_pending(&address)?;

		if !self.accept && !self.reject {
			if Cli::json() {
				println!("{}", serde_json::to_string_pretty(&pending)?);

				return Ok(());
			}

			match pending {
				Some(pending) => argon_info!(
					"Waiting on a {} confirmation for client {}:\n\n{}\n\nAnswer it with `polaris consent --accept` or `--reject`",
					pending.kind,
					pending.client_id,
					pending.message
				),
				None => argon_info!("There is no pending confirmation for this session"),
			}

			return Ok(());
		}

		let pending = pending.context("There is no pending confirmation to answer")?;

		decide(&address, pending.client_id, self.accept)?;

		if Cli::json() {
			println!(
				"{}",
				serde_json::to_string_pretty(&serde_json::json!({
					"clientId": pending.client_id,
					"kind": pending.kind,
					"message": pending.message,
					"accepted": self.accept,
				}))?
			);
		} else {
			argon_info!(
				"{} the {} confirmation for client {}",
				if self.accept { "Accepted" } else { "Rejected" },
				pending.kind,
				pending.client_id
			);
		}

		Ok(())
	}
}

fn fetch_pending(address: &str) -> Result<Option<Pending>> {
	let client = Client::builder().timeout(Duration::from_millis(1500)).build()?;
	let response = client.get(format!("{address}/status")).send()?;

	if !response.status().is_success() {
		bail!("The session did not answer: {}", response.status());
	}

	Ok(response.json::<StatusReply>()?.pending)
}

fn decide(address: &str, client_id: u32, accept: bool) -> Result<()> {
	let token = util::read_token().unwrap_or_default();
	let body = rmp_serde::to_vec(&DecideRequest { client_id, accept })?;

	let response = Client::default()
		.post(format!("{address}/consent/decide"))
		.header(CONTENT_TYPE, "application/msgpack")
		.header(AUTHORIZATION, format!("Bearer {}", token))
		.body(body)
		.send()?;

	if !response.status().is_success() {
		let status = response.status();
		let body = response.text().unwrap_or_default();

		bail!("The session refused the answer: {status} {body}");
	}

	Ok(())
}
