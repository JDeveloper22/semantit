use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

fn write_case(root: &Path, relative_path: &str, source: &str) -> PathBuf {
    let path = root.join(relative_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directories should be created");
    }
    fs::write(&path, source).expect("fixture should be writable");
    path
}

fn run_json(current_dir: &Path, args: &[&str]) -> Value {
    let output = Command::cargo_bin("semantit-cli")
        .expect("binary should build")
        .current_dir(current_dir)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    serde_json::from_slice(&output).expect("stdout should be valid JSON")
}

#[test]
fn parse_should_work_from_nested_directory_with_relative_paths() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let project_root = temp_dir.path().join("workspace");
    let nested_dir = project_root.join("apps/demo");
    fs::create_dir_all(&nested_dir).expect("nested directory should exist");

    write_case(
        &project_root,
        "src/parse_sample.ts",
        r#"
export interface User {
  id: string;
}

export function greet(user: User): string {
  return `hola ${user.id}`;
}
"#,
    );

    let relative_path = PathBuf::from("..")
        .join("..")
        .join("src")
        .join("parse_sample.ts")
        .to_string_lossy()
        .into_owned();
    let parsed = run_json(&nested_dir, &["parse", &relative_path, "--json"]);

    assert_eq!(parsed["language"], "typescript");
    assert_eq!(parsed["entities"].as_array().map(Vec::len), Some(2));
    assert_eq!(parsed["exports"].as_array().map(Vec::len), Some(2));
}

#[test]
fn diff_should_detect_function_moves_without_fake_insertions() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let root = temp_dir.path();

    let old_path = write_case(
        root,
        "cases/move_old.ts",
        r#"
export function alpha(): number {
  return 1;
}

export function beta(): number {
  return 2;
}
"#,
    );
    let new_path = write_case(
        root,
        "cases/move_new.ts",
        r#"
export function beta(): number {
  return 2;
}

export function alpha(): number {
  return 1;
}
"#,
    );

    let diff = run_json(
        root,
        &[
            "diff",
            old_path.to_str().expect("utf-8 path"),
            new_path.to_str().expect("utf-8 path"),
            "--json",
        ],
    );

    assert_eq!(diff["summary"]["moved"], 2);
    assert_eq!(diff["summary"]["inserted"], 0);
    assert_eq!(diff["summary"]["deleted"], 0);
}

#[test]
fn merge_should_auto_resolve_disjoint_function_changes() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let root = temp_dir.path();

    let base = write_case(
        root,
        "merge/base.ts",
        r#"
export function foo(value: number): number {
  return value + 1;
}

export function bar(value: number): number {
  return value * 2;
}
"#,
    );
    let ours = write_case(
        root,
        "merge/ours.ts",
        r#"
export function foo(value: number): number {
  const adjusted = value + 10;
  return adjusted + 1;
}

export function bar(value: number): number {
  return value * 2;
}
"#,
    );
    let theirs = write_case(
        root,
        "merge/theirs.ts",
        r#"
export function bar(value: number): number {
  const doubled = value * 2;
  return doubled + 5;
}

export function foo(value: number): number {
  return value + 1;
}
"#,
    );

    let merge = run_json(
        root,
        &[
            "merge",
            base.to_str().expect("utf-8 path"),
            ours.to_str().expect("utf-8 path"),
            theirs.to_str().expect("utf-8 path"),
            "--json",
        ],
    );

    assert_eq!(merge["status"], "clean");
    assert_eq!(merge["conflicts"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        merge["merged_index"]["entities"].as_array().map(Vec::len),
        Some(2)
    );

    let merged_entities = merge["merged_index"]["entities"]
        .as_array()
        .expect("entities should be present");
    let foo = merged_entities
        .iter()
        .find(|entity| entity["path"] == "foo")
        .expect("foo should exist");
    let bar = merged_entities
        .iter()
        .find(|entity| entity["path"] == "bar")
        .expect("bar should exist");

    assert!(foo["body"]
        .as_str()
        .is_some_and(|body| body.contains("adjusted")));
    assert!(bar["body"]
        .as_str()
        .is_some_and(|body| body.contains("doubled")));
}

#[test]
fn merge_should_report_conflict_when_both_branches_touch_same_function() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let root = temp_dir.path();

    let base = write_case(
        root,
        "same/base.ts",
        r#"
export function foo(value: number): number {
  return value + 1;
}
"#,
    );
    let ours = write_case(
        root,
        "same/ours.ts",
        r#"
export function foo(value: number): number {
  return value + 10;
}
"#,
    );
    let theirs = write_case(
        root,
        "same/theirs.ts",
        r#"
export function foo(value: number): number {
  return value * 3;
}
"#,
    );

    let merge = run_json(
        root,
        &[
            "merge",
            base.to_str().expect("utf-8 path"),
            ours.to_str().expect("utf-8 path"),
            theirs.to_str().expect("utf-8 path"),
            "--json",
        ],
    );

    assert_eq!(merge["status"], "conflicted");
    assert_eq!(merge["conflicts"].as_array().map(Vec::len), Some(1));
    assert_eq!(merge["conflicts"][0]["cause"], "concurrent_modification");
    assert!(merge["merged_index"].is_null());
}

#[test]
fn merge_should_preserve_rename_plus_implementation_change() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let root = temp_dir.path();

    let base = write_case(
        root,
        "rename/base.ts",
        r#"
export function foo(value: number): number {
  return value + 1;
}
"#,
    );
    let ours = write_case(
        root,
        "rename/ours.ts",
        r#"
export function compute(value: number): number {
  return value + 1;
}
"#,
    );
    let theirs = write_case(
        root,
        "rename/theirs.ts",
        r#"
export function foo(value: number): number {
  const base = value + 1;
  return base * 2;
}
"#,
    );

    let merge = run_json(
        root,
        &[
            "merge",
            base.to_str().expect("utf-8 path"),
            ours.to_str().expect("utf-8 path"),
            theirs.to_str().expect("utf-8 path"),
            "--json",
        ],
    );

    assert_eq!(merge["status"], "clean");

    let merged_entities = merge["merged_index"]["entities"]
        .as_array()
        .expect("entities should be present");
    let compute = merged_entities
        .iter()
        .find(|entity| entity["path"] == "compute")
        .expect("renamed function should exist");
    assert!(compute["body"]
        .as_str()
        .is_some_and(|body| body.contains("base*2")));
    assert_eq!(
        merge["merged_index"]["exports"][0]["exported_name"],
        "compute"
    );
}

#[test]
fn merge_should_flag_ambiguous_split_for_manual_review() {
    let temp_dir = TempDir::new().expect("temp dir should exist");
    let root = temp_dir.path();

    let base = write_case(
        root,
        "split/base.ts",
        r#"
export function pipeline(value: number): number {
  const doubled = value * 2;
  return doubled + 5;
}
"#,
    );
    let ours = write_case(
        root,
        "split/ours.ts",
        r#"
function doubleValue(value: number): number {
  return value * 2;
}

export function pipeline(value: number): number {
  return doubleValue(value) + 5;
}
"#,
    );
    let theirs = write_case(
        root,
        "split/theirs.ts",
        r#"
function addFive(value: number): number {
  return value + 5;
}

export function pipeline(value: number): number {
  return addFive(value * 2);
}
"#,
    );

    let merge = run_json(
        root,
        &[
            "merge",
            base.to_str().expect("utf-8 path"),
            ours.to_str().expect("utf-8 path"),
            theirs.to_str().expect("utf-8 path"),
            "--json",
        ],
    );

    assert_eq!(merge["status"], "conflicted");
    assert!(merge["conflicts"]
        .as_array()
        .is_some_and(|conflicts| !conflicts.is_empty()));
    assert!(merge["merged_index"].is_null());
}
