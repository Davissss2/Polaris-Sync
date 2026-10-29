use anyhow::{bail, Result};
use colored::Colorize;
use crossbeam_channel::{Receiver, Sender};
use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicU64, Ordering},
		RwLock,
	},
	time::{SystemTime, UNIX_EPOCH},
};

use crate::{
	argon_warn,
	config::Config,
	constants::{MAX_QUEUED_MESSAGES, QUEUE_TIMEOUT},
	server::{self, Message},
};

/// A client that has not polled in this long is gone: Studio was closed or
/// reloaded without unsubscribing
const CLIENT_TIMEOUT_MS: u64 = 15_000;

fn now_ms() -> u64 {
	SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|elapsed| elapsed.as_millis() as u64)
		.unwrap_or(0)
}

macro_rules! read {
	($rwlock:expr) => {
		$rwlock.write().unwrap()
	};
}

macro_rules! write {
	($rwlock:expr) => {
		$rwlock.write().unwrap()
	};
}

#[derive(Debug)]
struct Listener {
	pub id: u32,
	pub name: String,
	pub is_internal: bool,
	pub last_seen: AtomicU64,
}

#[derive(Debug)]
struct Channel {
	sender: Sender<Message>,
	receiver: Receiver<Message>,
}

#[derive(Debug)]
pub struct Queue {
	queues: RwLock<HashMap<u32, Channel>>,
	listeners: RwLock<Vec<Listener>>,
	unsynced_changes: RwLock<usize>,
}

impl Queue {
	pub fn new() -> Self {
		Self {
			queues: RwLock::new(HashMap::new()),
			listeners: RwLock::new(Vec::new()),
			unsynced_changes: RwLock::new(0),
		}
	}

	pub fn push<M>(&self, message: M, id: Option<u32>) -> Result<()>
	where
		M: Into<Message>,
	{
		if let Some(id) = id {
			if !self.is_subscribed(id) {
				bail!("Not subscribed")
			}

			let queues = read!(self.queues);
			let sender = queues.get(&id).unwrap().sender.clone();

			sender.send(message.into())?;

			return Ok(());
		}

		let message: Message = message.into();
		let mut did_push = false;
		let mut failed = Vec::new();

		for listener in read!(self.listeners).iter() {
			let queues = read!(self.queues);

			if let Some(channel) = queues.get(&listener.id) {
				// Never block the sync loop on a client that stopped reading
				if channel.sender.try_send(message.clone()).is_err() {
					failed.push(listener.id);
				}

				did_push = true;
			}
		}

		for id in failed {
			argon_warn!("Client {id} is not consuming changes, disconnecting it");

			let _ = self.unsubscribe(id);
		}

		if !did_push {
			let max_unsynced_changes = Config::new().max_unsynced_changes;
			let mut unsynced_changes = write!(self.unsynced_changes);

			*unsynced_changes += 1;

			if max_unsynced_changes > 0 && *unsynced_changes >= max_unsynced_changes {
				argon_warn!(
					"There are {} unsynced changes. Connect at least one client to this server or increase max_unsynced_changes setting to suppress this warning",
					unsynced_changes.to_string().bold()
				);
			}
		}

		Ok(())
	}

	pub fn get(&self, id: u32) -> Result<Option<Message>> {
		if !self.is_subscribed(id) {
			bail!("Not subscribed")
		}

		self.touch(id);

		let queues = read!(self.queues);
		let receiver = queues.get(&id).unwrap().receiver.clone();

		drop(queues);

		Ok(receiver.recv().ok())
	}

	pub fn get_timeout(&self, id: u32) -> Result<Option<Message>> {
		if !self.is_subscribed(id) {
			bail!("Not subscribed")
		}

		self.touch(id);

		let queues = read!(self.queues);
		let receiver = queues.get(&id).unwrap().receiver.clone();

		drop(queues);

		Ok(receiver.recv_timeout(QUEUE_TIMEOUT).ok())
	}

	pub fn get_change(&self, id: u32) -> Result<Message> {
		if !self.is_subscribed(id) {
			bail!("Not subscribed")
		}

		let queues = read!(self.queues);
		let receiver = queues.get(&id).unwrap().receiver.clone();

		drop(queues);

		Ok(loop {
			if let Ok(message) = receiver.recv() {
				if message.is_change() {
					break message;
				}
			}
		})
	}

	pub fn subscribe(&self, id: u32, name: &str) -> Result<()> {
		if self.is_subscribed(id) {
			bail!("Already subscribed")
		}

		let (sender, receiver) = crossbeam_channel::bounded(MAX_QUEUED_MESSAGES);
		let channel = Channel { sender, receiver };

		let listener = Listener {
			id,
			name: name.to_owned(),
			is_internal: false,
			last_seen: AtomicU64::new(now_ms()),
		};

		write!(self.listeners).push(listener);
		write!(self.queues).insert(id.to_owned(), channel);

		Ok(())
	}

	pub fn subscribe_internal(&self) -> Result<()> {
		let mut id = 0;

		loop {
			if !self.is_subscribed(id) {
				break;
			}

			id += 1;
		}

		let (sender, receiver) = crossbeam_channel::bounded(MAX_QUEUED_MESSAGES);
		let channel = Channel { sender, receiver };

		let listener = Listener {
			id,
			name: format!("Internal listener #{id}"),
			is_internal: true,
			last_seen: AtomicU64::new(now_ms()),
		};

		write!(self.listeners).push(listener);
		write!(self.queues).insert(id.to_owned(), channel);

		Ok(())
	}

	pub fn unsubscribe(&self, id: u32) -> Result<()> {
		if !self.is_subscribed(id) {
			bail!("Not subscribed")
		}

		write!(self.listeners).retain(|listener| listener.id != id);
		write!(self.queues).remove(&id);

		Ok(())
	}

	pub fn disconnect(&self, message: &str, id: u32) -> Result<()> {
		if !self.is_subscribed(id) {
			bail!("Not subscribed")
		}

		self.push(
			server::Disconnect {
				message: message.to_owned(),
			},
			Some(id),
		)?;

		Ok(())
	}

	pub fn is_subscribed(&self, id: u32) -> bool {
		read!(self.listeners).iter().any(|listener| listener.id == id)
	}

	fn touch(&self, id: u32) {
		let listeners = self.listeners.write().unwrap();

		for listener in listeners.iter() {
			if listener.id == id {
				listener.last_seen.store(now_ms(), Ordering::Relaxed);
			}
		}
	}

	/// Drops clients that stopped polling. Studio closes without unsubscribing,
	/// and the leftover listener would be counted as a live connection
	fn prune(&self) {
		let now = now_ms();
		let mut dead = Vec::new();

		self.listeners.write().unwrap().retain(|listener| {
			let idle = now.saturating_sub(listener.last_seen.load(Ordering::Relaxed));
			let alive = listener.is_internal || idle <= CLIENT_TIMEOUT_MS;

			if !alive {
				dead.push(listener.id);
			}

			alive
		});

		if dead.is_empty() {
			return;
		}

		let mut queues = self.queues.write().unwrap();

		for id in dead {
			queues.remove(&id);
		}
	}

	pub fn get_first_non_internal_listener_name(&self) -> Option<String> {
		read!(self.listeners)
			.iter()
			.find(|listener| !listener.is_internal)
			.map(|listener| listener.name.to_owned())
	}

	/// Every client currently subscribed, internal listeners excluded. A
	/// session can be healthy with an empty list: the server is up but Studio
	/// is not listening.
	pub fn clients(&self) -> Vec<String> {
		self.prune();

		read!(self.listeners)
			.iter()
			.filter(|listener| !listener.is_internal)
			.map(|listener| listener.name.to_owned())
			.collect()
	}
}
