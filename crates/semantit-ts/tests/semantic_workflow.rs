use std::fs;
use std::path::{Path, PathBuf};

use insta::assert_json_snapshot;
use semantit_core::model::{ConflictCause, MergeStatus, SemanticChangeKind};
use semantit_core::parser::{LanguageParser, ParseInput};
use semantit_core::{diff_indices, merge_indices};
use semantit_ts::TypeScriptParser;
use serde_json::json;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/typescript")
        .join(name)
}

fn parse_fixture(name: &str) -> semantit_core::model::FileSemanticIndex {
    let path = fixture_path(name);
    let source = fs::read_to_string(&path).expect("fixture should be readable");
    TypeScriptParser
        .parse(ParseInput {
            path: Path::new(&path),
            source: &source,
        })
        .expect("fixture should parse")
}

fn entity<'a>(
    index: &'a semantit_core::model::FileSemanticIndex,
    path: &str,
) -> &'a semantit_core::model::SemanticEntity {
    index
        .entities
        .iter()
        .find(|entity| entity.path == path)
        .expect("entity should exist")
}

#[test]
fn function_id_should_stay_stable_when_moved() {
    let old_index = parse_fixture("move_old.ts");
    let new_index = parse_fixture("move_new.ts");

    assert_eq!(entity(&old_index, "foo").id, entity(&new_index, "foo").id);
    assert_eq!(entity(&old_index, "bar").id, entity(&new_index, "bar").id);

    let diff = diff_indices(&old_index, &new_index);
    assert_eq!(diff.summary.moved, 2);
    assert_eq!(diff.summary.inserted, 0);
    assert_eq!(diff.summary.deleted, 0);
}

#[test]
fn body_hash_should_ignore_formatting_changes() {
    let old_index = parse_fixture("format_old.ts");
    let new_index = parse_fixture("format_new.ts");

    assert_eq!(
        entity(&old_index, "stable").body_hash,
        entity(&new_index, "stable").body_hash
    );

    let diff = diff_indices(&old_index, &new_index);
    assert_eq!(diff.summary.implementation_changed, 0);
}

#[test]
fn signature_hash_should_change_only_when_signature_changes() {
    let old_index = parse_fixture("signature_old.ts");
    let new_index = parse_fixture("signature_new.ts");

    assert_ne!(
        entity(&old_index, "changeMe").signature_hash,
        entity(&new_index, "changeMe").signature_hash
    );
    assert_eq!(
        entity(&old_index, "changeMe").body_hash,
        entity(&new_index, "changeMe").body_hash
    );

    let diff = diff_indices(&old_index, &new_index);
    let change = diff
        .changes
        .iter()
        .find(|change| change.anchor_id == entity(&old_index, "changeMe").id)
        .expect("signature change should exist");
    assert!(change.kinds.contains(&SemanticChangeKind::SignatureChanged));
    assert!(!change
        .kinds
        .contains(&SemanticChangeKind::ImplementationChanged));
}

#[test]
fn diff_should_scope_small_change_to_function() {
    let base = parse_fixture("disjoint_base.ts");
    let ours = parse_fixture("disjoint_ours.ts");
    let diff = diff_indices(&base, &ours);

    let change = diff
        .changes
        .iter()
        .find(|change| change.anchor_id == entity(&base, "foo").id)
        .expect("foo change should exist");

    assert!(change
        .kinds
        .contains(&SemanticChangeKind::ImplementationChanged));
    assert!(change
        .context
        .as_ref()
        .and_then(|context| context.after.as_deref())
        .is_some_and(|after| after.contains("adjusted")));
}

#[test]
fn merge_should_auto_resolve_disjoint_changes() {
    let base = parse_fixture("disjoint_base.ts");
    let ours = parse_fixture("disjoint_ours.ts");
    let theirs = parse_fixture("disjoint_theirs.ts");
    let merge = merge_indices(&base, &ours, &theirs);

    assert_eq!(merge.status, MergeStatus::Clean);
    assert!(merge.conflicts.is_empty());

    let merged = merge
        .merged_index
        .expect("clean merge should produce index");
    assert!(entity(&merged, "foo")
        .body
        .as_deref()
        .is_some_and(|body| body.contains("adjusted")));
    assert!(entity(&merged, "bar")
        .body
        .as_deref()
        .is_some_and(|body| body.contains("doubled")));
}

#[test]
fn merge_should_conflict_on_same_entity_changes() {
    let base = parse_fixture("same_base.ts");
    let ours = parse_fixture("same_ours.ts");
    let theirs = parse_fixture("same_theirs.ts");
    let merge = merge_indices(&base, &ours, &theirs);

    assert_eq!(merge.status, MergeStatus::Conflicted);
    assert_eq!(merge.conflicts.len(), 1);
    assert_eq!(
        merge.conflicts[0].cause,
        ConflictCause::ConcurrentModification
    );
}

#[test]
fn merge_should_auto_resolve_rename_plus_implementation() {
    let base = parse_fixture("rename_base.ts");
    let ours = parse_fixture("rename_ours.ts");
    let theirs = parse_fixture("rename_theirs.ts");
    let merge = merge_indices(&base, &ours, &theirs);

    assert_eq!(merge.status, MergeStatus::Clean);
    let merged = merge.merged_index.expect("rename merge should be clean");
    let entity = entity(&merged, "compute");
    assert_eq!(entity.name, "compute");
    assert!(entity
        .body
        .as_deref()
        .is_some_and(|body| body.contains("base*2")));
    assert_eq!(merged.exports.len(), 1);
    assert_eq!(merged.exports[0].exported_name, "compute");
}

#[test]
fn merge_should_flag_split_as_ambiguous() {
    let base = parse_fixture("split_base.ts");
    let ours = parse_fixture("split_ours.ts");
    let theirs = parse_fixture("split_theirs.ts");
    let merge = merge_indices(&base, &ours, &theirs);

    assert_eq!(merge.status, MergeStatus::Conflicted);
    assert!(!merge.conflicts.is_empty());
    assert!(merge.merged_index.is_none());
}

#[test]
fn parse_json_snapshot_should_remain_stable() {
    let index = parse_fixture("parse_sample.ts");
    let summary = json!({
        "language": index.language,
        "entity_paths": index.entities.iter().map(|entity| entity.path.clone()).collect::<Vec<_>>(),
        "exports": index.exports.iter().map(|export| export.exported_name.clone()).collect::<Vec<_>>(),
        "callable_constants": index.entities.iter().filter(|entity| entity.kind == semantit_core::model::EntityKind::Constant).map(|entity| entity.path.clone()).collect::<Vec<_>>(),
    });

    assert_json_snapshot!(summary, @r###"
    {
      "callable_constants": [
        "formatUser"
      ],
      "entity_paths": [
        "Greeter",
        "Greeter#constructor",
        "Greeter#greet",
        "Greeter.create",
        "User",
        "UserSummary",
        "foo",
        "formatUser"
      ],
      "exports": [
        "Greeter",
        "User",
        "UserSummary",
        "foo",
        "formatUser"
      ],
      "language": "typescript"
    }
    "###);
}

#[test]
fn diff_json_snapshot_should_capture_move_semantics() {
    let old_index = parse_fixture("move_old.ts");
    let new_index = parse_fixture("move_new.ts");
    let diff = diff_indices(&old_index, &new_index);

    let summary = json!({
        "summary": diff.summary,
        "changes": diff.changes.iter().map(|change| json!({
            "path": change.current.as_ref().or(change.previous.as_ref()).map(|entity| entity.path.clone()),
            "kinds": change.kinds,
        })).collect::<Vec<_>>(),
    });

    assert_json_snapshot!(summary, @r###"
    {
      "changes": [
        {
          "kinds": [
            "moved"
          ],
          "path": "bar"
        },
        {
          "kinds": [
            "moved"
          ],
          "path": "foo"
        }
      ],
      "summary": {
        "deleted": 0,
        "implementation_changed": 0,
        "inserted": 0,
        "merged": 0,
        "moved": 2,
        "renamed": 0,
        "signature_changed": 0,
        "split": 0,
        "unchanged": 0
      }
    }
    "###);
}

#[test]
fn merge_json_snapshot_should_capture_rename_resolution() {
    let base = parse_fixture("rename_base.ts");
    let ours = parse_fixture("rename_ours.ts");
    let theirs = parse_fixture("rename_theirs.ts");
    let merge = merge_indices(&base, &ours, &theirs);
    let merged = merge
        .merged_index
        .clone()
        .expect("clean merge should build index");

    let summary = json!({
        "status": merge.status,
        "resolved_kinds": merge.resolved_changes.iter().map(|change| json!({
            "anchor": change.previous.as_ref().map(|entity| entity.path.clone()),
            "kinds": change.kinds,
        })).collect::<Vec<_>>(),
        "merged_entities": merged.entities.iter().map(|entity| json!({
            "name": entity.name,
            "path": entity.path,
            "body": entity.body,
        })).collect::<Vec<_>>(),
        "exports": merged.exports.iter().map(|export| export.exported_name.clone()).collect::<Vec<_>>(),
    });

    assert_json_snapshot!(summary, @r###"
    {
      "exports": [
        "compute"
      ],
      "merged_entities": [
        {
          "body": "{const base=value+1;return base*2;}",
          "name": "compute",
          "path": "compute"
        }
      ],
      "resolved_kinds": [
        {
          "anchor": "foo",
          "kinds": [
            "signature_changed",
            "implementation_changed",
            "renamed"
          ]
        }
      ],
      "status": "clean"
    }
    "###);
}
