//! Multi-topic pub/sub using Tokio broadcast channels (fan-out to many subscribers).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use thiserror::Error;
use tokio::sync::{RwLock, broadcast};

use crate::error::PublishError;

type Topic = Arc<str>;

/// `EventBus` was constructed with `capacity == 0`, which Tokio broadcast rejects.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("broadcast capacity must be non-zero")]
pub struct InvalidCapacity;

/// Handles fan-out delivery: each subscriber gets a clone of every message (best-effort).
///
/// Lagging subscribers may drop messages depending on `broadcast` capacity and load.
pub struct EventBus<M: Clone + Send + 'static> {
    topics: RwLock<HashMap<Topic, broadcast::Sender<M>>>,
    capacity: usize,
}

impl<M: Clone + Send + 'static> fmt::Debug for EventBus<M> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventBus")
            .field("capacity", &self.capacity)
            .finish_non_exhaustive()
    }
}

impl<M: Clone + Send + 'static> EventBus<M> {
    /// Constructs a bus with the given per-topic lag buffer size for [`broadcast`].
    ///
    /// # Errors
    ///
    /// Returns [`InvalidCapacity`] when `capacity == 0`.
    pub fn try_new(capacity: usize) -> Result<Self, InvalidCapacity> {
        if capacity == 0 {
            return Err(InvalidCapacity);
        }
        Ok(Self {
            topics: RwLock::new(HashMap::new()),
            capacity,
        })
    }

    /// Like [`Self::try_new`], but panics when `capacity == 0` with a concise message.
    ///
    /// # Panics
    ///
    /// When `capacity == 0`.
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self::try_new(capacity).expect("EventBus::new requires non-zero broadcast capacity")
    }

    /// `capacity` is the lag buffer per topic; when full, slow subscribers lag behind.
    #[must_use]
    pub const fn capacity(&self) -> usize {
        self.capacity
    }

    /// Ensure a topic exists (idempotent). Useful for setup before subscribers connect.
    pub async fn ensure_topic(&self, topic: impl AsRef<str>) {
        let topic_key = Self::intern_topic(topic.as_ref());
        let mut map = self.topics.write().await;
        map.entry(topic_key)
            .or_insert_with(|| broadcast::channel(self.capacity).0);
    }

    fn intern_topic(raw: &str) -> Topic {
        Arc::from(raw)
    }

    async fn sender_for(&self, topic: &str) -> broadcast::Sender<M> {
        let topic_key = Self::intern_topic(topic);
        let mut map = self.topics.write().await;
        map.entry(topic_key)
            .or_insert_with(|| broadcast::channel(self.capacity).0)
            .clone()
    }

    /// Publish one message to all current subscribers on `topic`.
    ///
    /// Creates the topic lazily. Returns [`PublishError`] when there are no active receivers
    /// (Tokio broadcast returns an error in that case).
    pub async fn publish(&self, topic: &str, message: M) -> Result<usize, PublishError> {
        let topic_owned = Self::intern_topic(topic);
        let sender = {
            let mut map = self.topics.write().await;
            map.entry(topic_owned.clone())
                .or_insert_with(|| broadcast::channel(self.capacity).0)
                .clone()
        };
        sender.send(message).map_err(|_| PublishError::NoReceivers {
            topic: topic_owned,
            reason: "no active broadcast receivers for this topic".into(),
        })
    }

    /// Subscribe to `topic`, receiving later publishes. Misses messages sent before subscribe.
    pub async fn subscribe(&self, topic: &str) -> broadcast::Receiver<M> {
        self.sender_for(topic).await.subscribe()
    }
}
