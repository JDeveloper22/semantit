use std::path::PathBuf;

use assert_cmd::Command;
use predicates::str::contains;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/typescript")
        .join(name)
}

#[test]
fn parse_command_should_emit_json() {
    let mut command = Command::cargo_bin("semantit-cli").expect("binary should build");
    command
        .args([
            "parse",
            fixture("parse_sample.ts").to_str().expect("utf-8 path"),
            "--json",
        ])
        .assert()
        .success()
        .stdout(contains("\"entities\""))
        .stdout(contains("\"language\": \"typescript\""));
}

#[test]
fn merge_command_should_emit_human_summary() {
    let mut command = Command::cargo_bin("semantit-cli").expect("binary should build");
    command
        .args([
            "merge",
            fixture("disjoint_base.ts").to_str().expect("utf-8 path"),
            fixture("disjoint_ours.ts").to_str().expect("utf-8 path"),
            fixture("disjoint_theirs.ts").to_str().expect("utf-8 path"),
        ])
        .assert()
        .success()
        .stdout(contains("Merge status"))
        .stdout(contains("Resolved changes"));
}
