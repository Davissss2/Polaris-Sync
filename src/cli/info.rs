use anyhow::{bail, Result};
use clap::Parser;
use colored::Colorize;
use serde::Serialize;
use std::path::PathBuf;

use crate::{
	argon_info,
	cli::Cli,
	config::Config,
	ext::PathExt,
	logger::Table,
	project::{self, Project, ProjectNode},
	sessions,
};

/// Show project information: mapped services, folders and live session
#[derive(Parser)]
pub struct Info {
	/// Project path
	#[arg()]
	project: Option<PathBuf>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProjectInfo {
	name: String,
	project: String,
	workspace: String,
	kind: String,
	typescript: bool,
	wally: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	game_id: Option<u64>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	place_ids: Vec<u64>,
	services: Vec<ServiceEntry>,
	address: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceEntry {
	name: String,
	path: String,
}

impl Info {
	pub fn main(self) -> Result<()> {
		let project_path = project::resolve(self.project.unwrap_or_default())?;

		Config::load_workspace(project_path.get_parent());

		if !project_path.exists() {
			bail!(
				"No project files found in {}",
				project_path.get_parent().to_string().bold()
			);
		}

		let project = Project::load(&project_path)?;
		let mut services = Vec::new();

		collect_services(&project.node, "", &mut services);

		let address = sessions::get_all().ok().and_then(|sessions| {
			sessions
				.into_values()
				.find(|session| session.project.as_ref() == Some(&project_path))
				.and_then(|session| session.get_address())
		});

		let info = ProjectInfo {
			name: project.name.clone(),
			project: project_path.to_string_lossy().to_string(),
			workspace: project.workspace_dir.to_string_lossy().to_string(),
			kind: if project.is_place() { "place" } else { "model" }.to_owned(),
			typescript: project.is_ts(),
			wally: project.is_wally(),
			game_id: project.game_id,
			place_ids: project.place_ids.clone(),
			services,
			address,
		};

		if Cli::json() {
			println!("{}", serde_json::to_string_pretty(&info)?);

			return Ok(());
		}

		// Whether the project is tied to a place decides if the Studio plugin
		// accepts it on sight or has to ask first
		let binding = if info.game_id.is_none() && info.place_ids.is_empty() {
			String::from("nothing, any place is accepted")
		} else {
			let game = info
				.game_id
				.map(|id| id.to_string())
				.unwrap_or_else(|| String::from("any"));

			let places = if info.place_ids.is_empty() {
				String::from("any")
			} else {
				info.place_ids
					.iter()
					.map(u64::to_string)
					.collect::<Vec<String>>()
					.join(", ")
			};

			format!("game {game}, places {places}")
		};

		argon_info!(
			"Project: {}\nType: {}\nBound to: {}\nWorkspace: {}\nServing at: {}",
			info.name.bold(),
			info.kind,
			binding,
			info.workspace,
			info.address.clone().unwrap_or_else(|| String::from("not running"))
		);

		let mut table = Table::new();
		table.set_header(vec!["Service", "Folder"]);

		for service in &info.services {
			table.add_row(vec![service.name.clone(), service.path.clone()]);
		}

		argon_info!("Mapped services:\n\n{}", table);

		Ok(())
	}
}

fn collect_services(node: &ProjectNode, prefix: &str, services: &mut Vec<ServiceEntry>) {
	for (name, child) in &node.tree {
		let path = if prefix.is_empty() {
			name.clone()
		} else {
			format!("{prefix}/{name}")
		};

		if let Some(project_path) = &child.path {
			services.push(ServiceEntry {
				name: path.clone(),
				path: project_path.path().to_string_lossy().to_string(),
			});
		}

		collect_services(child, &path, services);
	}
}
