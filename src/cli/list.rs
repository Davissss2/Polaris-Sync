use anyhow::Result;
use clap::Parser;
use serde::Serialize;
use std::path::Path;

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
}

impl List {
	pub fn main(self) -> Result<()> {
		let mut entries = sessions::get_all()?
			.into_iter()
			.map(|(id, session)| SessionEntry {
				id,
				project: session.project.as_ref().map(|path| path.to_string_lossy().to_string()),
				address: session.get_address(),
				pid: session.pid,
				running: util::process_exists(session.pid),
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
		table.set_header(vec!["ID", "Project", "Address", "PID", "Status"]);

		for entry in entries {
			table.add_row(vec![
				entry.id,
				project_name(entry.project.as_deref()),
				entry.address.unwrap_or_else(|| String::from("-")),
				entry.pid.to_string(),
				if entry.running {
					String::from("running")
				} else {
					String::from("stopped")
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
