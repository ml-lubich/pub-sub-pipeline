//! Tests aimed at exercising easy-to-miss branches (coverage helpers).

use std::time::Duration;

use pub_sub_pipeline::{
    CancellationToken, EventBus, channel_pair, collect_with_timeout,
    spawn_broadcast_stage_cancellable, spawn_map_stage_cancellable, spawn_merge_stage_cancellable,
};

#[tokio::test]
async fn bus_reports_capacity_and_ensure_topic() {
    let bus = EventBus::<u8>::new(12);
    assert_eq!(bus.capacity(), 12);
    bus.ensure_topic("ready").await;
    let mut sub = bus.subscribe("ready").await;
    bus.publish("ready", 9).await.expect("publish");
    assert_eq!(sub.recv().await.expect("recv"), 9);
}

#[tokio::test]
async fn map_cancellable_processes_items_before_shutdown() {
    let (in_tx, in_rx) = channel_pair::<i32>(4);
    let (out_tx, mut out_rx) = channel_pair::<i32>(4);
    let shutdown = CancellationToken::new();
    let stage = spawn_map_stage_cancellable(in_rx, out_tx, shutdown.clone(), |x| x + 10);

    in_tx.send(1).await.unwrap();
    assert_eq!(out_rx.recv().await.expect("out"), 11);

    shutdown.cancel();
    drop(in_tx);

    stage.await.unwrap().unwrap();
    assert!(out_rx.recv().await.is_none());
}

#[tokio::test]
async fn broadcast_cancellable_stops_even_with_outputs() {
    let (in_tx, in_rx) = channel_pair::<i32>(4);
    let (o1_tx, _o1_rx) = channel_pair::<i32>(4);
    let cancel = CancellationToken::new();
    let fan = spawn_broadcast_stage_cancellable(in_rx, vec![o1_tx], cancel.clone());
    cancel.cancel();
    drop(in_tx);
    fan.await.unwrap().unwrap();
}

#[tokio::test]
async fn merge_cancellable_stops_cleanly() {
    let cancel = CancellationToken::new();
    let (a_tx, a_rx) = channel_pair::<i32>(4);
    let (out_tx, out_rx) = channel_pair::<i32>(4);
    let merge = spawn_merge_stage_cancellable(vec![a_rx], out_tx, cancel.clone());
    cancel.cancel();
    drop(a_tx);
    merge.await.unwrap().unwrap();
    assert!(
        collect_with_timeout(out_rx, 1, Duration::from_millis(50))
            .await
            .is_empty()
    );
}
