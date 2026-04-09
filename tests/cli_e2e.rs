//! End-to-end CLI tests (`pubsub` binary).

use assert_cmd::Command;

#[test]
fn help_lists_commands_and_ascii_motif() {
    let assert = Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .arg("--help")
        .assert()
        .success();

    let hay = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(hay.contains("P U B / S U B"));
    assert!(hay.contains("demo"));
    assert!(hay.contains("squares"));
    assert!(hay.contains("fanout"));
}

#[test]
fn demo_runs_deterministic_report() {
    let assert = Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .arg("demo")
        .assert()
        .success();

    let hay = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(hay.contains("broadcast sample"));
    assert!(hay.contains("pipeline squares"));
    assert!(hay.contains("fan-in merged"));
}

#[test]
fn squares_transforms_stdin_lines() {
    Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .arg("squares")
        .write_stdin("3\n4\n\n")
        .assert()
        .success()
        .stdout("9\n16\n");
}

#[test]
fn demo_verbose_flag_still_succeeds() {
    Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .args(["--verbose", "demo"])
        .assert()
        .success();
}

#[test]
fn squares_reports_error_on_bad_integer() {
    Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .arg("squares")
        .write_stdin("not-an-int\n")
        .assert()
        .failure();
}

#[test]
fn fanout_drains_all_subscribers() {
    let assert = Command::cargo_bin("pubsub")
        .expect("cargo built `pubsub` binary")
        .args([
            "fanout",
            "--topic",
            "e2e",
            "--subscribers",
            "2",
            "--count",
            "4",
        ])
        .assert()
        .success();

    let hay = std::str::from_utf8(&assert.get_output().stdout).expect("utf8");
    assert!(hay.contains("topic=e2e"));
    assert!(hay.contains("subscribers=2"));
    assert!(hay.contains("publishes=4"));
}
