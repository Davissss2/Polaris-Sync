use anyhow::Result;
use clap::{ColorChoice, Parser, Subcommand};
use clap_verbosity_flag::Verbosity;
use env_logger::fmt::WriteStyle;
use lazy_static::lazy_static;
use log::LevelFilter;
use std::{
	env,
	sync::atomic::{AtomicBool, Ordering},
};

use crate::util;

mod build;
mod config;
mod consent;
mod debug;
mod doc;
mod exec;
mod info;
mod init;
mod list;
mod plugin;
mod serve;
mod sourcemap;
mod stop;
mod studio;
mod workspace;

lazy_static! {
	static ref OUTPUT_JSON: AtomicBool = AtomicBool::new(false);
}

macro_rules! about {
	() => {
		concat!("Polaris ", env!("CARGO_PKG_VERSION"))
	};
}

macro_rules! long_about {
	() => {
		concat!(
			"Polaris ",
			env!("CARGO_PKG_VERSION"),
			"\n",
			env!("CARGO_PKG_DESCRIPTION"),
			"\n",
			"Made with <3 by ",
			env!("CARGO_PKG_AUTHORS")
		)
	};
}

#[derive(Parser)]
#[clap(about = about!(), long_about = long_about!(), version)]
pub struct Cli {
	#[command(subcommand)]
	command: Commands,

	#[command(flatten)]
	verbose: Verbosity,

	/// Automatically answer to any prompts
	#[arg(short, long, global = true)]
	yes: bool,

	/// Print machine readable JSON instead of human output
	#[arg(long, global = true)]
	json: bool,

	/// Print full backtrace on panic
	#[arg(short = 'B', long, global = true)]
	backtrace: bool,

	#[arg(long, hide = true, global = true)]
	profile: bool,

	/// Output coloring: auto, always, never
	#[arg(
		long,
		short = 'C',
		global = true,
		value_name = "WHEN",
		default_value = "auto",
		hide_default_value = true,
		hide_possible_values = true
	)]
	pub color: ColorChoice,
}

impl Cli {
	pub fn new() -> Cli {
		let cli = Cli::parse();

		let json = cli.json || env::var("POLARIS_JSON").is_ok_and(|value| value == "1");

		OUTPUT_JSON.store(json, Ordering::Relaxed);

		cli
	}

	/// Whether commands should print machine readable JSON
	pub fn json() -> bool {
		OUTPUT_JSON.load(Ordering::Relaxed)
	}

	pub fn profile(&self) -> bool {
		self.profile
	}

	pub fn yes(&self) -> bool {
		if env::var("RUST_YES").is_ok() {
			return util::env_yes();
		}

		self.yes
	}

	pub fn backtrace(&self) -> bool {
		if env::var("RUST_BACKTRACE").is_ok() {
			return util::env_backtrace();
		}

		self.backtrace
	}

	pub fn verbosity(&self) -> LevelFilter {
		if env::var("RUST_VERBOSE").is_ok() {
			return util::env_verbosity();
		}

		self.verbose.log_level_filter()
	}

	pub fn log_style(&self) -> WriteStyle {
		if env::var("RUST_LOG_STYLE").is_ok() {
			return util::env_log_style();
		}

		match self.color {
			ColorChoice::Always => WriteStyle::Always,
			ColorChoice::Never => WriteStyle::Never,
			_ => WriteStyle::Auto,
		}
	}

	pub fn main(self) -> Result<()> {
		match self.command {
			Commands::Init(command) => command.main(),
			Commands::Serve(command) => command.main(),
			Commands::Build(command) => command.main(),
			Commands::Sourcemap(command) => command.main(),
			Commands::Stop(command) => command.main(),
			Commands::Studio(command) => command.main(),
			Commands::Debug(command) => command.main(),
			Commands::Exec(command) => command.main(),
			Commands::Consent(command) => command.main(),
			Commands::List(command) => command.main(),
			Commands::Info(command) => command.main(),
			Commands::Plugin(command) => command.main(),
			Commands::Workspace(command) => command.main(),
			Commands::Config(command) => command.main(),
			Commands::Doc(command) => command.main(),
		}
	}
}

#[derive(Subcommand)]
pub enum Commands {
	Init(init::Init),
	Serve(serve::Serve),
	Build(build::Build),
	Sourcemap(sourcemap::Sourcemap),
	Stop(stop::Stop),
	Studio(studio::Studio),
	Debug(debug::Debug),
	Exec(exec::Exec),
	Consent(consent::Consent),
	List(list::List),
	Info(info::Info),
	Plugin(plugin::Plugin),
	Workspace(workspace::Workspace),
	Config(config::Config),
	Doc(doc::Doc),
}
