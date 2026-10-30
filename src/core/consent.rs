use serde::{Deserialize, Serialize};
use std::sync::Mutex;

use crate::lock;

/// A confirmation the Studio plugin is waiting on. The interface can always
/// answer it, and so can `polaris consent`, which is what lets an agent clear
/// it without clicking anything
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pending {
	pub client_id: u32,
	/// Short name of the guard that raised the prompt: `project` or `changes`
	pub kind: String,
	pub message: String,
	/// `None` while nobody answered, `accept` or `reject` once someone did
	#[serde(skip_serializing_if = "Option::is_none")]
	pub decision: Option<String>,
}

#[derive(Debug, Default)]
pub struct Consent {
	inner: Mutex<Option<Pending>>,
}

impl Consent {
	pub fn new() -> Self {
		Self::default()
	}

	/// Registers a prompt the plugin is waiting on. The interface shows one at
	/// a time, so a new one replaces whatever was there
	pub fn announce(&self, pending: Pending) {
		*lock!(self.inner) = Some(pending);
	}

	pub fn get(&self) -> Option<Pending> {
		lock!(self.inner).clone()
	}

	/// Decision waiting for this client. `None` means it is not the one being
	/// asked, so a stale poller cannot answer somebody else's prompt
	pub fn decision_for(&self, client_id: u32) -> Option<String> {
		let inner = lock!(self.inner);
		let pending = inner.as_ref()?;

		if pending.client_id != client_id {
			return None;
		}

		pending.decision.clone()
	}

	pub fn decide(&self, client_id: u32, accept: bool) -> bool {
		let mut inner = lock!(self.inner);

		match inner.as_mut() {
			Some(pending) if pending.client_id == client_id => {
				pending.decision = Some(String::from(if accept { "accept" } else { "reject" }));

				true
			}
			_ => false,
		}
	}

	/// Forgets the prompt once the interface answered it on its own
	pub fn clear(&self, client_id: u32) -> bool {
		let mut inner = lock!(self.inner);

		if inner.as_ref().is_some_and(|pending| pending.client_id == client_id) {
			*inner = None;

			true
		} else {
			false
		}
	}
}
