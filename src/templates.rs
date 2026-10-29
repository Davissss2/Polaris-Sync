use include_dir::{include_dir, Dir};

const GAME: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/game");
const PLACE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/place");
const QUICK: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/quick");
const MODEL: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/model");
const PLUGIN: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/plugin");
const PACKAGE: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/package");
const EMPTY: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/assets/templates/empty");

/// Templates bundled into the binary
pub const NAMES: [&str; 7] = ["game", "place", "quick", "model", "plugin", "package", "empty"];

/// Returns a bundled template by name
pub fn get(name: &str) -> Option<&'static Dir<'static>> {
	match name {
		"game" => Some(&GAME),
		"place" => Some(&PLACE),
		"quick" => Some(&QUICK),
		"model" => Some(&MODEL),
		"plugin" => Some(&PLUGIN),
		"package" => Some(&PACKAGE),
		"empty" => Some(&EMPTY),
		_ => None,
	}
}
