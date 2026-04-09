//! Error types for the pub/sub hub and pipeline.

use std::sync::Arc;

use thiserror::Error;
use tokio::task::JoinError;

/// Failures when publishing to a topic.
#[derive(Debug, Error)]
pub enum PublishError {
    /// No active receivers (Tokio broadcast requires at least one live subscriber).
    #[error("no receivers for topic {topic:?}: {reason}")]
    NoReceivers {
        /// Topic key used for publish.
        topic: Arc<str>,
        /// Human-readable detail.
        reason: String,
    },
}

/// Subscription or channel failures.
#[derive(Debug, Error)]
pub enum PipelineError {
    /// A downstream stopped accepting items while upstream was still producing.
    #[error("worker channel closed unexpectedly")]
    ChannelClosed,
    /// A spawned task panicked or was cancelled while being awaited.
    #[error("task join failed: {0}")]
    TaskJoin(#[from] JoinError),
}
