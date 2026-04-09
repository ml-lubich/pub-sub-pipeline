//! Pub/sub hub (`EventBus`) and Tokio-native pipeline stages (`pipeline` module).
//!
//! ## Patterns demonstrated
//! - **Fan-out**: `tokio::sync::broadcast` for many subscribers per topic.
//! - **Backpressure**: bounded `mpsc` between pipeline stages.
//! - **Fan-in**: `spawn_merge_stage` multiplexes multiple inputs into one sink (Tokio `JoinSet`).
//! - **Cooperative shutdown**: `*_cancellable` stages combine a `CancellationToken` with `select!`.

#![forbid(unsafe_code)]
#![warn(missing_docs, rust_2018_idioms)]

mod error;

pub mod art;
pub mod bus;
pub mod pipeline;
pub mod scenarios;

pub use bus::{EventBus, InvalidCapacity};
pub use error::{PipelineError, PublishError};
pub use pipeline::{
    channel_pair, collect_with_timeout, spawn_broadcast_stage, spawn_broadcast_stage_cancellable,
    spawn_map_stage, spawn_map_stage_cancellable, spawn_merge_stage, spawn_merge_stage_cancellable,
};
pub use scenarios::{
    ScenarioError, run_demo_report, run_fanout_report, run_squares_stdin_stdout, square_token,
};
pub use tokio_util::sync::CancellationToken;
