//! Tests for the state behind a pending confirmation. The endpoints on top of
//! it are thin, and the plugin is the only thing that polls them, so the rules
//! live here: who can answer, and what the answer looks like.

use polaris::core::consent::{Consent, Pending};

fn prompt(client_id: u32) -> Pending {
	Pending {
		client_id,
		kind: String::from("project"),
		message: String::from("Apply this project here?"),
		decision: None,
	}
}

#[test]
fn announcing_a_prompt_makes_it_visible() {
	let consent = Consent::new();
	assert!(consent.get().is_none());

	consent.announce(prompt(7));

	let pending = consent.get().expect("the prompt should be listed");

	assert_eq!(pending.client_id, 7);
	assert_eq!(pending.kind, "project");
	assert!(pending.decision.is_none(), "nobody answered yet");
}

#[test]
fn a_new_prompt_replaces_the_previous_one() {
	let consent = Consent::new();

	consent.announce(prompt(7));
	consent.announce(prompt(9));

	assert_eq!(consent.get().unwrap().client_id, 9);
}

#[test]
fn the_asking_client_sees_the_answer() {
	let consent = Consent::new();
	consent.announce(prompt(7));

	assert!(consent.decision_for(7).is_none(), "still waiting");

	assert!(consent.decide(7, true));
	assert_eq!(consent.decision_for(7).as_deref(), Some("accept"));

	assert!(consent.decide(7, false));
	assert_eq!(consent.decision_for(7).as_deref(), Some("reject"));
}

#[test]
fn another_client_cannot_answer() {
	let consent = Consent::new();
	consent.announce(prompt(7));

	assert!(!consent.decide(9, true), "a stale poller must not answer");
	assert!(consent.decision_for(9).is_none());
	assert!(consent.get().unwrap().decision.is_none());
}

#[test]
fn answering_without_a_prompt_does_nothing() {
	let consent = Consent::new();

	assert!(!consent.decide(7, true));
	assert!(consent.get().is_none());
}

#[test]
fn clearing_forgets_the_prompt() {
	let consent = Consent::new();
	consent.announce(prompt(7));

	assert!(consent.clear(7));
	assert!(consent.get().is_none());
	assert!(consent.decision_for(7).is_none());
}

#[test]
fn another_client_cannot_clear_it() {
	let consent = Consent::new();
	consent.announce(prompt(7));

	assert!(!consent.clear(9));
	assert!(consent.get().is_some());
}
