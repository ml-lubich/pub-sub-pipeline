//! Deterministic async scenarios shared by the CLI, examples, and tests.

use std::fmt::Write as _;
use std::time::Duration;

use tokio::sync::broadcast::error::RecvError;
use tokio::task::JoinError;

use crate::bus::EventBus;
use crate::error::{PipelineError, PublishError};
use crate::pipeline::{channel_pair, collect_with_timeout, spawn_map_stage, spawn_merge_stage};

/// Errors surfaced by canned demos (`demo`, `fanout`) and IO helpers.
#[derive(Debug, thiserror::Error)]
pub enum ScenarioError {
    /// Publish failed (typically no subscribers yet).
    #[error(transparent)]
    Publish(#[from] PublishError),
    /// A pipeline stage failed (channel closed or join error).
    #[error(transparent)]
    Pipeline(#[from] PipelineError),
    /// Broadcast receive failed.
    #[error(transparent)]
    Recv(#[from] RecvError),
    /// Async stdin read failed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Line could not be parsed as `i32`.
    #[error("expected an integer, got {0:?}")]
    BadInteger(String),
    /// Subscriber / message counts are inconsistent with the harness (internal).
    #[error("invariant violated: expected {expected} deliveries, saw {actual}")]
    Invariant {
        /// Expected deliveries across all subscriber drains.
        expected: usize,
        /// Observed deliveries.
        actual: usize,
    },
}

impl From<JoinError> for ScenarioError {
    fn from(value: JoinError) -> Self {
        Self::Pipeline(PipelineError::from(value))
    }
}

/// Full demo report: broadcast + two-stage pipeline + merge (used by CLI / tests).
pub async fn run_demo_report() -> Result<String, ScenarioError> {
    let mut out = String::new();

    let bus: EventBus<i32> = EventBus::new(64);
    let mut sub_a = bus.subscribe("metrics").await;
    let mut sub_b = bus.subscribe("metrics").await;

    bus.publish("metrics", 2).await?;
    bus.publish("metrics", 3).await?;

    let a = sub_a.recv().await?;
    let b = sub_b.recv().await?;
    writeln!(
        out,
        "broadcast sample (two subscribers, first message): a={a} b={b}"
    )
    .expect("writing to a `String` is infallible");

    let (in_tx, in_rx) = channel_pair::<String>(8);
    let (mid_tx, mid_rx) = channel_pair::<i32>(8);
    let (out_tx, out_rx) = channel_pair::<i32>(8);

    let parse = spawn_map_stage(in_rx, mid_tx, |s| s.parse::<i32>().unwrap_or(0));
    let square = spawn_map_stage(mid_rx, out_tx, |n| n * n);

    let _feed = tokio::spawn(async move {
        let _ = in_tx.send("4".into()).await;
        let _ = in_tx.send("5".into()).await;
    });

    let (pr, sq) = tokio::join!(parse, square);
    pr??;
    sq??;

    let results = collect_with_timeout(out_rx, 2, Duration::from_secs(1)).await;
    writeln!(out, "pipeline squares: {results:?}").expect("writing to a `String` is infallible");

    let (t1_tx, t1_rx) = channel_pair::<i32>(4);
    let (t2_tx, t2_rx) = channel_pair::<i32>(4);
    let (merge_out_tx, merge_out_rx) = channel_pair::<i32>(4);

    let merge = spawn_merge_stage(vec![t1_rx, t2_rx], merge_out_tx);
    let _p1 = tokio::spawn(async move {
        let _ = t1_tx.send(10).await;
    });
    let _p2 = tokio::spawn(async move {
        let _ = t2_tx.send(20).await;
    });

    let mut merged = collect_with_timeout(merge_out_rx, 2, Duration::from_secs(1)).await;
    merged.sort_unstable();
    merge.await??;
    writeln!(out, "fan-in merged (sorted): {merged:?}")
        .expect("writing to a `String` is infallible");

    Ok(out)
}

/// Fan-out scenario: `subscribers` each receives every publish (`0..count`).
pub async fn run_fanout_report(
    subscribers: usize,
    count: u32,
    topic: &str,
) -> Result<String, ScenarioError> {
    if subscribers == 0 {
        return Ok("subscribers=0: nothing to drain.\n".to_string());
    }

    let bus: EventBus<u32> = EventBus::new(256);
    let mut receivers = Vec::with_capacity(subscribers);
    for _ in 0..subscribers {
        receivers.push(bus.subscribe(topic).await);
    }

    for i in 0..count {
        bus.publish(topic, i).await?;
    }

    let expected = subscribers.saturating_mul(count as usize);

    let mut actual = 0usize;
    let mut checksum = 0u64;
    for mut rx in receivers {
        for _ in 0..count {
            let v = u64::from(rx.recv().await?);
            checksum = checksum.wrapping_add(v);
            actual += 1;
        }
    }

    if actual != expected {
        return Err(ScenarioError::Invariant { expected, actual });
    }

    let mut out = String::new();
    writeln!(
        out,
        "fanout topic={topic} subscribers={subscribers} publishes={count} checksum={checksum}"
    )
    .expect("writing to a `String` is infallible");
    Ok(out)
}

/// Pure helper: square a numeric token (trimmed). Tests the happy path without async IO.
pub fn square_token(line: &str) -> Result<i32, ScenarioError> {
    let t = line.trim();
    if t.is_empty() {
        return Err(ScenarioError::BadInteger(String::new()));
    }
    let v: i32 = t
        .parse()
        .map_err(|_| ScenarioError::BadInteger(t.to_string()))?;
    Ok(v.saturating_mul(v))
}

/// Stdin → square → stdout (one line per non-empty input line).
pub async fn run_squares_stdin_stdout() -> Result<(), ScenarioError> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut buf = String::new();
    loop {
        buf.clear();
        let n = reader.read_line(&mut buf).await?;
        if n == 0 {
            break;
        }
        if buf.trim().is_empty() {
            continue;
        }
        let y = square_token(&buf)?;
        println!("{y}");
    }
    Ok(())
}
