use anyhow::Result;
use clap::Parser;
use serde::{Deserialize, Serialize};
use std::{path::Path, time::Duration};

use crate::{argon_info, argon_warn, cli::Cli, logger::Table, sessions, util};

/// List every Polaris session with its project and port
#[derive(Parser)]
pub struct List {
	/// Hide sessions whose process is no longer running
	#[arg(short, long)]
	alive: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionEntry {
	id: String,
	project: Option<String>,
	address: Option<String>,
	pid: u32,
	running: bool,
	healthy: bool,
	/// Editors attached to the session. None when the server is too old to
	/// answer, an empty list when nobody is listening
	clients: Option<Vec<String>>,
}

impl List {
	pub fn main(self) -> Result<()> {
		let mut entries = sessions::get_all()?
			.into_iter()
			.map(|(id, session)| {
				let address = session.get_address();
				let running = util::process_exists(session.pid);
				let healthy = running && is_responsive(address.as_deref());
				let clients = if healthy {
					fetch_clients(address.as_deref())
				} else {
					None
				};

				SessionEntry {
					healthy,
					clients,
					id,
					project: session.project.as_ref().map(|path| path.to_string_lossy().to_string()),
					address,
					pid: session.pid,
					running,
				}
			})
			.filter(|entry| !self.alive || entry.running)
			.collect::<Vec<_>>();

		entries.sort_by(|a, b| a.id.cmp(&b.id));

		if Cli::json() {
			println!("{}", serde_json::to_string_pretty(&entries)?);

			return Ok(());
		}

		if entries.is_empty() {
			argon_warn!("There are no running sessions");

			return Ok(());
		}

		let mut table = Table::new();
		table.set_header(vec!["ID", "Project", "Address", "PID", "Status", "Clients"]);

		for entry in entries {
			table.add_row(vec![
				entry.id,
				project_name(entry.project.as_deref()),
				entry.address.unwrap_or_else(|| String::from("-")),
				entry.pid.to_string(),
				if entry.healthy {
					String::from("running")
				} else if entry.running {
					String::from("unresponsive")
				} else {
					String::from("stopped")
				},
				match entry.clients {
					Some(ref clients) if clients.is_empty() => String::from("none"),
					Some(ref clients) => clients.len().to_string(),
					None => String::from("-"),
				},
			]);
		}

		argon_info!("All Polaris sessions:\n\n{}", table);

		Ok(())
	}
}

fn project_name(path: Option<&str>) -> String {
	path.and_then(|path| Path::new(path).parent())
		.and_then(|path| path.file_name())
		.map(|name| name.to_string_lossy().to_string())
		.unwrap_or_else(|| String::from("-"))
}

/// Asks the server for its project details with a short timeout,
/// so a process that is alive but hung is not reported as healthy
fn is_responsive(address: Option<&str>) -> bool {
	let Some(address) = address else {
		return false;
	};

	let client = match reqwest::blocking::Client::builder()
		.timeout(Duration::from_millis(800))
		.build()
	{
		Ok(client) => client,
		Err(_) => return false,
	};

	client
		.get(format!("{address}/details"))
		.send()
		.is_ok_and(|response| response.status().is_success())
}

#[derive(Deserialize)]
struct StatusResponse {
	clients: Vec<String>,
}

/// Asks the session which editors are attached to it. None means the server is
/// too old to answer, which is not the same as nobody being connected
fn fetch_clients(address: Option<&str>) -> Option<Vec<String>> {
	let address = address?;

	let client = reqwest::blocking::Client::builder()
		.timeout(Duration::from_millis(800))
		.build()
		.ok()?;

	let response = client.get(format!("{address}/status")).send().ok()?;

	if !response.status().is_success() {
		return None;
	}

	response.json::<StatusResponse>().ok().map(|status| status.clients)
}
