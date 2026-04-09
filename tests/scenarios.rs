//! Scenario helpers: small unit tests for pure logic.

use pub_sub_pipeline::ScenarioError;
use pub_sub_pipeline::scenarios::{run_fanout_report, square_token};

#[test]
fn square_token_parses_and_squares() {
    assert_eq!(square_token(" 7 \n").unwrap(), 49);
}

#[test]
fn square_token_rejects_garbage() {
    let err = square_token("nope").unwrap_err();
    assert!(matches!(err, ScenarioError::BadInteger(_)));
}

#[tokio::test]
async fn fanout_checksum_matches_closed_form() {
    let report = run_fanout_report(2, 4, "stats").await.expect("fanout");
    assert!(report.contains("checksum="));
    // sum_{i=0}^{3} i = 6 per subscriber , 2 subscribers => sum of all received = 12
    assert!(report.contains("checksum=12"));
}
