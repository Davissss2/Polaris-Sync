use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use colored::Colorize;
use std::path::PathBuf;

use crate::{argon_info, cli::Cli, config::Config, ext::PathExt, project, workspace};

/// Manage the project workspace
#[derive(Parser)]
pub struct Workspace {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Map every missing Roblox service and create its folder
	Expand(Expand),
}

impl Workspace {
	pub fn main(self) -> Result<()> {
		match self.command {
			Commands::Expand(command) => command.main(),
		}
	}
}

#[derive(Parser)]
pub struct Expand {
	/// Project path
	#[arg()]
	project: Option<PathBuf>,
}

impl Expand {
	pub fn main(self) -> Result<()> {
		let project_path = project::resolve(self.project.unwrap_or_default())?;

		Config::load_workspace(project_path.get_parent());

		if !project_path.exists() {
			bail!(
				"No project files found in {}. Run {} to create a new one",
				project_path.get_parent().to_string().bold(),
				"polaris init".bold(),
			);
		}

		let added = workspace::expand(&project_path)?;

		if Cli::json() {
			println!(
				"{}",
				serde_json::to_string_pretty(&serde_json::json!({
					"project": project_path.to_string_lossy(),
					"added": added,
				}))?
			);

			return Ok(());
		}

		if added.is_empty() {
			argon_info!("Workspace is already complete, nothing to expand");
			return Ok(());
		}

		for service in &added {
			argon_info!("Added {}", service.bold());
		}

		argon_info!(
			"Workspace expanded with {} service(s) in {}",
			added.len().to_string().bold(),
			project_path.to_string().bold()
		);

		Ok(())
	}
}
