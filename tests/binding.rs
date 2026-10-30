//! The rule that keeps a project from being served to the wrong place. The
//! plugin asks before applying anything; this is the server-side line, so a
//! mistaken click, or an old prompt accepted by inertia, still cannot land.

use polaris::project::check_place_binding;

#[test]
fn a_project_that_declares_nothing_accepts_any_place() {
	assert!(check_place_binding(None, &[], Some(9), Some(123)).is_ok());
	assert!(check_place_binding(None, &[], None, None).is_ok());
}

#[test]
fn a_bound_place_is_accepted() {
	assert!(check_place_binding(None, &[1, 2], Some(9), Some(2)).is_ok());
}

#[test]
fn another_place_is_refused() {
	let error = check_place_binding(None, &[1, 2], Some(9), Some(7)).unwrap_err();

	assert!(error.contains("bound to place 1, 2"), "got: {error}");
	assert!(error.contains("place 7"), "got: {error}");
}

#[test]
fn another_game_is_refused() {
	let error = check_place_binding(Some(5), &[], Some(6), None).unwrap_err();

	assert!(error.contains("game 5"), "got: {error}");
	assert!(error.contains("game 6"), "got: {error}");
}

#[test]
fn an_editor_that_says_nothing_is_let_through() {
	// A plugin that predates this check must keep working, so the missing
	// side is never treated as a mismatch
	assert!(check_place_binding(Some(5), &[1, 2], None, None).is_ok());
	assert!(check_place_binding(None, &[1], None, Some(1)).is_ok());
}

#[test]
fn an_unsaved_place_does_not_match_a_bound_project() {
	// Studio reports 0 for a place that was never saved, which is not any of
	// the places a real project is bound to
	assert!(check_place_binding(None, &[1234], Some(9), Some(0)).is_err());
}
