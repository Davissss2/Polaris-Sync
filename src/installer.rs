use anyhow::Result;
use colored::Colorize;
use std::{env, fs};

use crate::{argon_info, argon_warn, ext::PathExt, util};

pub fn is_managed() -> bool {
	let path = match env::current_exe() {
		Ok(path) => path,
		Err(_) => return false,
	};

	!path.contains(&[".polaris", "bin"]) && (path.contains(&["bin"]) || path.contains(&["tool-storage"]))
}

pub fn verify(is_managed: bool) -> Result<()> {
	if !is_managed {
		let bin_dir = util::get_data_dir()?.join("bin");

		if !bin_dir.exists() {
			fs::create_dir_all(&bin_dir)?;
		}

		globenv::set_path(&bin_dir.to_string())?;

		#[cfg(not(target_os = "windows"))]
		let exe_path = bin_dir.join("polaris");

		#[cfg(target_os = "windows")]
		let exe_path = bin_dir.join("polaris.exe");

		if !exe_path.exists() {
			fs::copy(env::current_exe()?, &exe_path)?;

			argon_info!("Installed Polaris to {}", exe_path.to_string().bold());
		}
	}

	if let Ok(plugin_path) = util::get_plugin_path() {
		if !plugin_path.exists() {
			argon_warn!(
				"Polaris Studio plugin not found at {}. Build it from the `plugin` directory and copy it there to enable live syncing",
				plugin_path.to_string().bold()
			);
		}
	}

	Ok(())
}
