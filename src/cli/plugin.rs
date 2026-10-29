use anyhow::{bail, Context, Result};
use clap::{Parser, ValueEnum};
use colored::Colorize;
use std::{fs, path::PathBuf};

use crate::{argon_info, argon_warn, ext::PathExt, util};

/// Install or uninstall the Polaris Studio plugin
#[derive(Parser)]
pub struct Plugin {
	/// Whether to `install` or `uninstall` the plugin
	#[arg(hide_possible_values = true)]
	mode: Option<PluginMode>,

	/// Path to a built `Polaris.rbxm`
	#[arg()]
	path: Option<PathBuf>,
}

#[derive(Clone, Default, ValueEnum)]
enum PluginMode {
	#[default]
	Install,
	Uninstall,
}

impl Plugin {
	pub fn main(self) -> Result<()> {
		let target =
			util::get_plugin_path().context("Roblox Studio was not found, pass a custom installation path instead")?;

		match self.mode.unwrap_or_default() {
			PluginMode::Install => {
				let Some(source) = self.path else {
					bail!(
						"Pass the path to a built plugin, e.g. {}",
						"polaris plugin install plugin/Polaris.rbxm".bold()
					);
				};

				if !source.exists() {
					bail!("Plugin file not found: {}", source.to_string().bold());
				}

				fs::create_dir_all(target.get_parent())?;
				fs::copy(&source, &target)?;

				argon_info!(
					"Installed Polaris plugin at {}. Reload plugins in Studio to see it",
					target.to_string().bold()
				);
			}
			PluginMode::Uninstall => {
				if !target.exists() {
					argon_warn!("Polaris plugin is not installed");

					return Ok(());
				}

				fs::remove_file(&target)?;

				argon_info!("Removed Polaris plugin from {}", target.to_string().bold());
			}
		}

		Ok(())
	}
}
