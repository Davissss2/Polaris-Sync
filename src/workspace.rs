use anyhow::{bail, Result};
use chrono::Datelike;
use colored::Colorize;
use include_dir::{Dir, DirEntry};
use log::{debug, trace};
use reqwest::{blocking::Client, header::USER_AGENT};
use std::{
	fs,
	path::{Path, PathBuf},
};

use crate::{
	argon_info, argon_warn,
	config::Config,
	ext::PathExt,
	program::{Program, ProgramName},
	project::{Project, ProjectNode, ProjectPath},
	templates, util,
};

/// Services mapped by the default `game` template, mirroring what Roblox
/// creates in a new place. Nested services are separated with `/`
pub const DATA_MODEL_SERVICES: [&str; 17] = [
	"Workspace",
	"Players",
	"Lighting",
	"MaterialService",
	"ReplicatedFirst",
	"ReplicatedStorage",
	"ServerScriptService",
	"ServerStorage",
	"StarterGui",
	"StarterPack",
	"StarterPlayer/StarterCharacterScripts",
	"StarterPlayer/StarterPlayerScripts",
	"Teams",
	"SoundService",
	"TextChatService",
	"LocalizationService",
	"TestService",
];

/// Nodes that are removed from a project file when Wally is not used
const PACKAGE_NODES: [&str; 3] = ["Packages", "ServerPackages", "DevPackages"];

#[derive(Debug)]
pub struct WorkspaceLicense<'a> {
	pub inner: &'a str,
	pub force: bool,
}

#[derive(Debug)]
pub struct WorkspaceConfig<'a> {
	pub project: &'a Path,
	pub template: &'a str,
	pub license: WorkspaceLicense<'a>,
	pub git: bool,
	pub wally: bool,
	pub selene: bool,
	pub docs: bool,
	pub rojo_mode: bool,
	pub use_lua: bool,
}

pub fn init(workspace: WorkspaceConfig) -> Result<()> {
	let Some(template) = templates::get(workspace.template) else {
		bail!(
			"Template {} does not exist. Available templates: {}",
			workspace.template.bold(),
			templates::NAMES.join(", "),
		);
	};

	let workspace_dir = workspace.project.get_parent();
	let project_name = workspace_dir.get_name();

	if !workspace_dir.exists() {
		fs::create_dir_all(workspace_dir)?;
	}

	install_template(template, workspace_dir, &workspace, project_name, 0)?;

	if workspace.git {
		initialize_repo(workspace_dir)?;
	}

	Ok(())
}

/// Writes a bundled template to disk, applying the workspace preferences
fn install_template(
	template: &Dir,
	target: &Path,
	workspace: &WorkspaceConfig,
	project_name: &str,
	depth: usize,
) -> Result<()> {
	for entry in template.entries() {
		let name = entry.path().get_name();

		if matches!(name, ".gitkeep" | ".DS_Store") {
			continue;
		}

		let new_path = if name == "project.json" {
			workspace.project.to_owned()
		} else {
			target.join(name)
		};

		if new_path.exists() {
			continue;
		}

		match entry {
			DirEntry::Dir(dir) => {
				if name == ".github" && !workspace.git {
					continue;
				}

				fs::create_dir_all(&new_path)?;
				install_template(dir, &new_path, workspace, project_name, depth + 1)?;
			}
			DirEntry::File(file) => {
				let contents = || String::from_utf8_lossy(file.contents()).into_owned();

				match name {
					"project.json" => {
						let contents = contents().replace("$name", project_name);

						let contents = if workspace.wally {
							contents
						} else {
							remove_package_nodes(&contents)?
						};

						fs::write(new_path, contents)?;
					}
					".gitignore" => {
						if workspace.git {
							fs::write(new_path, file.contents())?;
						}
					}
					"wally.toml" => {
						if workspace.wally || workspace.template == "package" {
							let contents = contents()
								.replace("$name", &project_name.to_lowercase())
								.replace("$author", &util::get_username().to_lowercase())
								.replace("$license", workspace.license.inner);

							fs::write(new_path, contents)?;
						}
					}
					"selene.toml" => {
						if workspace.selene {
							fs::write(new_path, file.contents())?;
						}
					}
					_ => match entry
						.path()
						.file_stem()
						.and_then(|stem| stem.to_str())
						.unwrap_or_default()
					{
						"README" | "CHANGELOG" => {
							if workspace.docs {
								fs::write(new_path, contents().replace("$name", project_name))?;
							}
						}
						"LICENSE" => {
							if workspace.docs || workspace.license.force {
								add_license(&new_path, workspace.license.inner, &contents())?;
							}
						}
						_ => {
							let mut file_name = name.to_owned();

							if depth > 0 {
								if file_name.starts_with(".src") && workspace.rojo_mode {
									file_name = file_name.replace(".src", "init");
								}

								if file_name.ends_with(".luau") && workspace.use_lua {
									file_name = file_name.replace(".luau", ".lua");
								}
							}

							fs::write(target.join(file_name), file.contents())?;
						}
					},
				}
			}
		}
	}

	Ok(())
}

pub fn init_ts(workspace: WorkspaceConfig) -> Result<Option<PathBuf>> {
	let package_manager = &Config::new().package_manager;

	argon_info!("Waiting for {}..", package_manager.bold());

	let template = workspace.template;
	let mut project = workspace.project.to_owned();

	let env_yes = util::env_yes();
	let mut command = match template {
		"place" => "game",
		"plugin" => template,
		"package" => template,
		"model" => template,
		_ => "init",
	};

	if project.get_name().ends_with(".project.json") {
		project = workspace
			.project
			.parent()
			.map_or(PathBuf::new(), |path| path.to_owned());
	}

	if command == "init" && env_yes {
		command = "game";
	}

	let child = Program::new(ProgramName::Npm)
		.message("Failed to initialize roblox-ts project")
		.arg("create")
		.arg("roblox-ts")
		.arg(command)
		.arg("--skipBuild")
		.arg(format!("--git={}", workspace.git))
		.arg(format!("--packageManager={package_manager}"))
		.args(["--dir", &project.to_string()])
		.arg(if env_yes { "--yes" } else { "" })
		.spawn()?;

	if let Some(child) = child {
		let output = child.wait_with_output()?;

		if let Some(code) = output.status.code() {
			if code != 0 {
				return Ok(None);
			}
		} else {
			return Ok(None);
		}
	} else {
		return Ok(None);
	}

	let Some(template) = templates::get(template) else {
		argon_warn!(
			"Template {} does not exist, additional files won't be added!",
			workspace.template.bold()
		);

		return Ok(Some(project));
	};

	let project_name = project.get_name();

	for entry in template.entries() {
		let name = entry.path().get_name();
		let new_path = project.join(name);

		if new_path.exists() {
			continue;
		}

		let DirEntry::File(file) = entry else {
			continue;
		};

		let contents = || String::from_utf8_lossy(file.contents()).into_owned();

		match entry
			.path()
			.file_stem()
			.and_then(|stem| stem.to_str())
			.unwrap_or_default()
		{
			"wally" if workspace.wally || workspace.template == "package" => {
				let contents = contents()
					.replace("$name", &project_name.to_lowercase())
					.replace("$author", &util::get_username().to_lowercase());

				fs::write(new_path, contents)?;
			}
			"README" | "CHANGELOG" if workspace.docs => {
				fs::write(new_path, contents().replace("$name", project_name))?;
			}
			"LICENSE" if workspace.docs || workspace.license.force => {
				add_license(&new_path, workspace.license.inner, &contents())?;
			}

			_ => {}
		}
	}

	Ok(Some(project))
}

/// Adds every missing DataModel service to an existing project and creates
/// its directory. Returns the list of services that were added
pub fn expand(project_path: &Path) -> Result<Vec<String>> {
	let mut project = Project::load(project_path)?;

	if !project.is_place() {
		bail!("Cannot expand a project that is not a place!");
	}

	let workspace_dir = project_path.get_parent();
	let mut added = Vec::new();

	for service in DATA_MODEL_SERVICES {
		let path = PathBuf::from(format!("src/{service}"));

		if set_service_path(&mut project.node, service, &path) {
			added.push(service.to_owned());
		}
	}

	if added.is_empty() {
		return Ok(added);
	}

	project.save(project_path)?;

	for service in &added {
		let dir = workspace_dir.join("src").join(service);

		fs::create_dir_all(&dir)?;

		if fs::read_dir(&dir)?.next().is_none() {
			fs::write(dir.join(".gitkeep"), "")?;
		}
	}

	Ok(added)
}

/// Ensures that a (possibly nested) service node points at `path` on disk.
/// Returns `true` when a `$path` had to be added
fn set_service_path(node: &mut ProjectNode, service: &str, path: &Path) -> bool {
	let mut segments = service.split('/').peekable();
	let mut current = node;

	while let Some(segment) = segments.next() {
		let entry = current.tree.entry(segment.to_owned()).or_default();

		if segments.peek().is_none() {
			if entry.path.is_some() {
				return false;
			}

			entry.path = Some(ProjectPath::Required(path.to_owned()));

			return true;
		}

		current = entry;
	}

	false
}

/// Removes Wally package nodes from a project file when Wally is not used
fn remove_package_nodes(contents: &str) -> Result<String> {
	let mut project: serde_json::Value = serde_json::from_str(contents)?;

	if let Some(tree) = project.get_mut("tree").and_then(|tree| tree.as_object_mut()) {
		for node in tree.values_mut() {
			if let Some(node) = node.as_object_mut() {
				for key in PACKAGE_NODES {
					node.remove(key);
				}
			}
		}
	}

	let contents = util::serialize_json(&project)?;

	Ok(String::from_utf8(contents)?)
}

pub fn initialize_repo(directory: &Path) -> Result<()> {
	let output = Program::new(ProgramName::Git)
		.message("Failed to initialize repository")
		.arg("init")
		.arg(directory.to_string())
		.output()?;

	if output.is_some() {
		debug!("Initialized Git repository");
	}

	Ok(())
}

fn add_license(path: &Path, license: &str, fallback: &str) -> Result<()> {
	trace!("Getting {license} license template..");

	let url = format!("https://api.github.com/licenses/{license}");

	let license_template = || -> Result<String> {
		match Client::new().get(url).header(USER_AGENT, "Polaris").send() {
			Ok(response) => {
				let json = response.json::<serde_json::Value>()?;

				if let Some(body) = json["body"].as_str() {
					Ok(body.to_owned())
				} else {
					bail!("Bad SPDX License identifier")
				}
			}
			Err(_) => {
				bail!("No internet connection");
			}
		}
	}();

	let name = util::get_username();
	let year = chrono::Utc::now().year();

	match license_template {
		Ok(license) => {
			// Apache
			let license = license.replace("[yyyy]", &year.to_string());
			let license = license.replace("[name of copyright owner]", &name);

			// MIT & BSD
			let license = license.replace("[year]", &year.to_string());
			let license = license.replace("[fullname]", &name);

			// GNU
			let license = license.replace("<year>", &year.to_string());
			let license = license.replace("<name of author>", &name);

			fs::write(path, license)?;
		}
		Err(err) => {
			let license = fallback.replace("$license", license);
			let license = license.replace("$year", &year.to_string());
			let license = license.replace("$owner", &name);

			fs::write(path, license)?;

			argon_warn!("Failed to add license: {}. Using basic fallback instead!", err);

			return Ok(());
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::DATA_MODEL_SERVICES;
	use crate::util;

	#[test]
	fn data_model_services_are_valid() {
		for service in DATA_MODEL_SERVICES {
			let name = service.rsplit('/').next().unwrap();

			assert!(util::is_service(name), "`{service}` is not a Roblox service");
		}
	}
}
