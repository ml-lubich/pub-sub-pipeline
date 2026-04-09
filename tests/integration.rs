//! Integration tests: bus fan-out, map pipeline, fan-in merge.

use std::time::Duration;

use pub_sub_pipeline::{
    CancellationToken, EventBus, channel_pair, collect_with_timeout, spawn_broadcast_stage,
    spawn_map_stage, spawn_map_stage_cancellable, spawn_merge_stage,
};

#[tokio::test]
async fn bus_delivers_same_message_to_all_subscribers() {
    let bus: EventBus<i32> = EventBus::new(16);
    let mut s1 = bus.subscribe("orders").await;
    let mut s2 = bus.subscribe("orders").await;

    let n = bus.publish("orders", 42).await.expect("publish");
    assert!(n >= 2, "expected at least two receivers");

    let a = tokio::time::timeout(Duration::from_secs(1), s1.recv())
        .await
        .expect("timeout")
        .expect("recv");
    let b = tokio::time::timeout(Duration::from_secs(1), s2.recv())
        .await
        .expect("timeout")
        .expect("recv");
    assert_eq!(a, 42);
    assert_eq!(b, 42);
}

#[tokio::test]
async fn bus_try_new_rejects_zero_capacity() {
    let err = EventBus::<()>::try_new(0).expect_err("zero capacity");
    assert_eq!(err, pub_sub_pipeline::InvalidCapacity);
}

#[tokio::test]
async fn bus_publish_errors_without_subscribers() {
    let bus: EventBus<u8> = EventBus::new(4);
    let err = bus
        .publish("empty", 1)
        .await
        .expect_err("should fail without receivers");
    assert!(matches!(
        err,
        pub_sub_pipeline::PublishError::NoReceivers { .. }
    ));
}

#[tokio::test]
async fn map_stage_cancellable_exits_cleanly_when_token_fires() {
    let (in_tx, in_rx) = channel_pair::<i32>(4);
    let (out_tx, mut out_rx) = channel_pair::<i32>(4);
    let shutdown = CancellationToken::new();
    let stage = spawn_map_stage_cancellable(in_rx, out_tx, shutdown.clone(), |x| x * 2);
    shutdown.cancel();
    drop(in_tx);
    stage.await.unwrap().unwrap();
    assert!(out_rx.recv().await.is_none());
}

#[tokio::test]
async fn map_pipeline_transforms_end_to_end() {
    let (in_tx, in_rx) = channel_pair::<String>(8);
    let (out_tx, out_rx) = channel_pair::<i32>(8);

    let stage = spawn_map_stage(in_rx, out_tx, |s| s.parse::<i32>().unwrap_or(-1));

    in_tx.send("7".into()).await.unwrap();
    drop(in_tx);

    stage.await.unwrap().unwrap();
    let got = collect_with_timeout(out_rx, 1, Duration::from_secs(1)).await;
    assert_eq!(got, vec![7]);
}

#[tokio::test]
async fn broadcast_stage_with_no_outputs_drains_without_panic() {
    let (in_tx, in_rx) = channel_pair::<i32>(4);
    let fan = spawn_broadcast_stage(in_rx, vec![]);
    in_tx.send(1).await.unwrap();
    drop(in_tx);
    fan.await.unwrap().unwrap();
}

#[tokio::test]
async fn broadcast_stage_duplicates_to_all_outputs() {
    let (in_tx, in_rx) = channel_pair::<i32>(4);
    let (o1_tx, mut o1_rx) = channel_pair::<i32>(4);
    let (o2_tx, mut o2_rx) = channel_pair::<i32>(4);

    let fan = spawn_broadcast_stage(in_rx, vec![o1_tx, o2_tx]);
    in_tx.send(99).await.unwrap();
    drop(in_tx);

    fan.await.unwrap().unwrap();

    let a = o1_rx.recv().await.expect("o1");
    let b = o2_rx.recv().await.expect("o2");
    assert_eq!(a, 99);
    assert_eq!(b, 99);
}

#[tokio::test]
async fn merge_stage_multiplexes_two_inputs() {
    let (a_tx, a_rx) = channel_pair::<i32>(4);
    let (b_tx, b_rx) = channel_pair::<i32>(4);
    let (out_tx, out_rx) = channel_pair::<i32>(4);

    let merge = spawn_merge_stage(vec![a_rx, b_rx], out_tx);

    a_tx.send(1).await.unwrap();
    b_tx.send(2).await.unwrap();
    drop(a_tx);
    drop(b_tx);

    merge.await.unwrap().unwrap();

    let mut vals = collect_with_timeout(out_rx, 2, Duration::from_secs(1)).await;
    vals.sort_unstable();
    assert_eq!(vals, vec![1, 2]);
}
