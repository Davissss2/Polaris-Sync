use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use std::{fs, path::PathBuf, time::Duration};

use crate::{argon_info, argon_warn, ext::PathExt, util};

/// Where the plugin is published
const PLUGIN_URL: &str = "https://github.com/Davissss2/Polaris-Sync/releases/latest/download/Polaris.rbxm";

/// Install or uninstall the Polaris Studio plugin
#[derive(Parser)]
pub struct Plugin {
	/// Whether to `install` or `uninstall` the plugin
	#[arg(hide_possible_values = true)]
	mode: Option<PluginMode>,

	/// Local `.rbxm` to install instead of downloading the latest release
	#[arg()]
	path: Option<PathBuf>,

	/// Installation target (defaults to the Roblox Studio plugins folder)
	#[arg(short, long)]
	output: Option<PathBuf>,
}

#[derive(Clone, Default, ValueEnum)]
enum PluginMode {
	#[default]
	Install,
	Uninstall,
}

impl Plugin {
	pub fn main(self) -> Result<()> {
		let target = match self.output {
			Some(path) => path,
			None => util::get_plugin_path().context("Roblox Studio was not found, pass a target with --output")?,
		};

		match self.mode.unwrap_or_default() {
			PluginMode::Install => {
				fs::create_dir_all(target.get_parent())?;

				match self.path {
					Some(source) => {
						if !source.exists() {
							bail!("Plugin file not found: {}", source.to_string().bold());
						}

						fs::copy(&source, &target)?;
					}
					None => download(&target)?,
				}

				argon_info!(
					"Installed Polaris plugin at {}. Reload plugins in Studio to see it",
					target.to_string().bold()
				);
			}
			PluginMode::Uninstall => {
				if !target.exists() {
					argon_warn!("Polaris plugin is not installed at {}", target.to_string().bold());

					return Ok(());
				}

				fs::remove_file(&target)?;

				argon_info!("Removed Polaris plugin from {}", target.to_string().bold());
			}
		}

		Ok(())
	}
}

/// Downloads the plugin published with the latest release
fn download(target: &PathBuf) -> Result<()> {
	argon_info!("Downloading Polaris plugin from the latest release..");

	let client = reqwest::blocking::Client::builder()
		.timeout(Duration::from_secs(120))
		.build()?;

	let response = client
		.get(PLUGIN_URL)
		.send()
		.with_context(|| format!("Failed to download the plugin from {PLUGIN_URL}"))?;

	if !response.status().is_success() {
		bail!(
			"No plugin published for the latest release ({}). Build it with {} and pass the file instead",
			response.status(),
			"rojo build -o Polaris.rbxm".bold()
		);
	}

	let bytes = response.bytes()?;

	if bytes.is_empty() {
		bail!("The downloaded plugin is empty, try again or build it locally");
	}

	fs::write(target, &bytes)?;

	Ok(())
}
