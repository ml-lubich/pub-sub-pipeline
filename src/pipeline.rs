//! Linear async pipeline: bounded channels between stages for backpressure.

use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use tokio::task::{JoinHandle, JoinSet};
use tokio_util::sync::CancellationToken;

use crate::error::PipelineError;

/// Process items from `rx` and forward transformed output to `tx`, stop when `rx` closes.
#[must_use]
pub fn spawn_map_stage<T, U, F>(
    mut rx: mpsc::Receiver<T>,
    tx: mpsc::Sender<U>,
    mut map: F,
) -> JoinHandle<Result<(), PipelineError>>
where
    T: Send + 'static,
    U: Send + 'static,
    F: FnMut(T) -> U + Send + 'static,
{
    tokio::spawn(async move {
        while let Some(item) = rx.recv().await {
            let out = map(item);
            tx.send(out)
                .await
                .map_err(|_| PipelineError::ChannelClosed)?;
        }
        Ok(())
    })
}

/// Like [`spawn_map_stage`], but stops cleanly when `cancel` is fired (graceful shutdown pattern).
#[must_use]
pub fn spawn_map_stage_cancellable<T, U, F>(
    mut rx: mpsc::Receiver<T>,
    tx: mpsc::Sender<U>,
    cancel: CancellationToken,
    mut map: F,
) -> JoinHandle<Result<(), PipelineError>>
where
    T: Send + 'static,
    U: Send + 'static,
    F: FnMut(T) -> U + Send + 'static,
{
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(()),
                item = rx.recv() => match item {
                    None => return Ok(()),
                    Some(item) => {
                        let out = map(item);
                        tx.send(out)
                            .await
                            .map_err(|_| PipelineError::ChannelClosed)?;
                    }
                },
            }
        }
    })
}

/// Fan-out: dispatch each item to all `outputs`. Stops when input closes after draining.
///
/// An empty `outputs` list is accepted: items are drained from `rx` and dropped.
#[must_use]
pub fn spawn_broadcast_stage<T: Clone + Send + 'static>(
    mut rx: mpsc::Receiver<T>,
    outputs: Vec<mpsc::Sender<T>>,
) -> JoinHandle<Result<(), PipelineError>> {
    tokio::spawn(async move {
        if outputs.is_empty() {
            while rx.recv().await.is_some() {}
            return Ok(());
        }
        'outer: while let Some(item) = rx.recv().await {
            for out in &outputs {
                if out.send(item.clone()).await.is_err() {
                    break 'outer;
                }
            }
        }
        Ok(())
    })
}

/// Like [`spawn_broadcast_stage`], but stops when `cancel` is fired.
#[must_use]
pub fn spawn_broadcast_stage_cancellable<T: Clone + Send + 'static>(
    mut rx: mpsc::Receiver<T>,
    outputs: Vec<mpsc::Sender<T>>,
    cancel: CancellationToken,
) -> JoinHandle<Result<(), PipelineError>> {
    tokio::spawn(async move {
        if outputs.is_empty() {
            loop {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => return Ok(()),
                    item = rx.recv() => {
                        if item.is_none() {
                            return Ok(());
                        }
                    }
                }
            }
        }
        loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => return Ok(()),
                item = rx.recv() => {
                    let Some(item) = item else { return Ok(()); };
                    for out in &outputs {
                        if out.send(item.clone()).await.is_err() {
                            return Ok(());
                        }
                    }
                },
            }
        }
    })
}

/// Merge multiple receivers into one sender (fan-in). Completes when **all** inputs finish.
#[must_use]
pub fn spawn_merge_stage<T: Send + 'static>(
    inputs: Vec<mpsc::Receiver<T>>,
    tx: mpsc::Sender<T>,
) -> JoinHandle<Result<(), PipelineError>> {
    tokio::spawn(async move {
        let tx = Arc::new(tx);
        let mut tasks = JoinSet::new();
        for mut rx in inputs {
            let downstream = Arc::clone(&tx);
            tasks.spawn(async move {
                while let Some(item) = rx.recv().await {
                    downstream
                        .send(item)
                        .await
                        .map_err(|_| PipelineError::ChannelClosed)?;
                }
                Ok::<(), PipelineError>(())
            });
        }
        while let Some(joined) = tasks.join_next().await {
            joined??;
        }
        Ok(())
    })
}

/// Like [`spawn_merge_stage`], but all workers observe the same `cancel` and exit cooperatively.
#[must_use]
pub fn spawn_merge_stage_cancellable<T: Send + 'static>(
    inputs: Vec<mpsc::Receiver<T>>,
    tx: mpsc::Sender<T>,
    cancel: CancellationToken,
) -> JoinHandle<Result<(), PipelineError>> {
    tokio::spawn(async move {
        let tx = Arc::new(tx);
        let mut tasks: JoinSet<Result<(), PipelineError>> = JoinSet::new();
        for mut rx in inputs {
            let downstream = Arc::clone(&tx);
            let child_cancel = cancel.child_token();
            tasks.spawn(async move {
                loop {
                    tokio::select! {
                        biased;
                        () = child_cancel.cancelled() => return Ok(()),
                        item = rx.recv() => match item {
                            None => return Ok(()),
                            Some(item) => {
                                downstream
                                    .send(item)
                                    .await
                                    .map_err(|_| PipelineError::ChannelClosed)?;
                            }
                        },
                    }
                }
            });
        }
        while let Some(joined) = tasks.join_next().await {
            joined??;
        }
        Ok(())
    })
}

/// Bounded channel pair sized for `buffer` in-flight items between stages.
#[must_use]
pub fn channel_pair<T>(buffer: usize) -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
    mpsc::channel(buffer)
}

/// Collect up to `max_items` messages, stopping on channel close or `idle` with no data.
pub async fn collect_with_timeout<T: Send + 'static>(
    mut rx: mpsc::Receiver<T>,
    max_items: usize,
    idle: Duration,
) -> Vec<T> {
    let mut out = Vec::with_capacity(max_items);
    while out.len() < max_items {
        match tokio::time::timeout(idle, rx.recv()).await {
            Ok(Some(v)) => out.push(v),
            Ok(None) | Err(_) => break,
        }
    }
    out
}
