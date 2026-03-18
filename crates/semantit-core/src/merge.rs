use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use crate::diff::diff_indices;
use crate::hash::build_full_hash;
use crate::model::{
    ConflictCause, FileSemanticIndex, MergeResult, MergeStatus, SemanticChange, SemanticChangeKind,
    SemanticConflict, SemanticEntity,
};

pub fn merge_indices(
    base: &FileSemanticIndex,
    ours: &FileSemanticIndex,
    theirs: &FileSemanticIndex,
) -> MergeResult {
    let ours_diff = diff_indices(base, ours);
    let theirs_diff = diff_indices(base, theirs);

    let ours_changes = anchor_changes(&ours_diff.changes);
    let theirs_changes = anchor_changes(&theirs_diff.changes);

    let mut merged_entities = Vec::new();
    let mut resolved_changes = Vec::new();
    let mut conflicts = Vec::new();
    let mut applied_ids = BTreeSet::new();

    for base_entity in &base.entities {
        let ours_change = ours_changes.get(base_entity.id.as_str()).copied();
        let theirs_change = theirs_changes.get(base_entity.id.as_str()).copied();

        match resolve_base_entity(base_entity, ours_change, theirs_change) {
            Resolution::Take(entity, change) => {
                applied_ids.insert(entity.id.clone());
                merged_entities.push(entity);
                resolved_changes.push(change);
            }
            Resolution::Drop(change) => {
                resolved_changes.push(change);
            }
            Resolution::Conflict(conflict) => conflicts.push(conflict),
        }
    }

    merge_insertions(
        ours,
        theirs,
        &ours_changes,
        &theirs_changes,
        &mut applied_ids,
        &mut merged_entities,
        &mut resolved_changes,
        &mut conflicts,
    );

    merged_entities.sort_by(|left, right| left.path.cmp(&right.path));

    let merged_index = conflicts.is_empty().then(|| FileSemanticIndex {
        language: base.language,
        path: format!("{} (semantic merge)", base.path),
        root_entities: merged_entities
            .iter()
            .filter(|entity| entity.parent.is_none())
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>(),
        exports: merge_exports(ours, theirs, &merged_entities),
        entities: merged_entities,
        parser_version: format!("semantic-merge<{}>", base.parser_version),
    });

    MergeResult {
        status: if conflicts.is_empty() {
            MergeStatus::Clean
        } else {
            MergeStatus::Conflicted
        },
        resolved_changes,
        conflicts,
        merged_index,
        notes: vec![
            "Merged index is semantic-only and may contain synthetic hashes for auto-resolved rename+implementation cases."
                .to_string(),
        ],
    }
}

#[expect(
    clippy::large_enum_variant,
    reason = "Each resolution variant intentionally carries full semantic payloads for reporting."
)]
enum Resolution {
    Take(SemanticEntity, SemanticChange),
    Drop(SemanticChange),
    Conflict(SemanticConflict),
}

fn resolve_base_entity(
    base_entity: &SemanticEntity,
    ours_change: Option<&SemanticChange>,
    theirs_change: Option<&SemanticChange>,
) -> Resolution {
    match (ours_change, theirs_change) {
        (Some(ours_change), Some(theirs_change)) => {
            resolve_concurrent_change(base_entity, ours_change, theirs_change)
        }
        (Some(change), None) => apply_single_change(base_entity, change),
        (None, Some(change)) => apply_single_change(base_entity, change),
        (None, None) => Resolution::Take(
            base_entity.clone(),
            SemanticChange {
                anchor_id: base_entity.id.clone(),
                kinds: vec![SemanticChangeKind::Unchanged],
                previous: Some(base_entity.snapshot()),
                current: Some(base_entity.snapshot()),
                related_entities: Vec::new(),
                context: None,
                explanation: format!("Entity `{}` remains unchanged.", base_entity.path),
            },
        ),
    }
}

fn apply_single_change(base_entity: &SemanticEntity, change: &SemanticChange) -> Resolution {
    if change.kinds.contains(&SemanticChangeKind::Deleted) {
        return Resolution::Drop(change.clone());
    }

    if change.kinds.contains(&SemanticChangeKind::Split) {
        return Resolution::Drop(change.clone());
    }

    let entity = change
        .current
        .as_ref()
        .and_then(|current| entity_from_snapshot(base_entity, current))
        .unwrap_or_else(|| base_entity.clone());

    Resolution::Take(entity, change.clone())
}

fn resolve_concurrent_change(
    base_entity: &SemanticEntity,
    ours_change: &SemanticChange,
    theirs_change: &SemanticChange,
) -> Resolution {
    let ours_deleted = ours_change.kinds.contains(&SemanticChangeKind::Deleted);
    let theirs_deleted = theirs_change.kinds.contains(&SemanticChangeKind::Deleted);

    if ours_deleted && theirs_deleted {
        return Resolution::Drop(ours_change.clone());
    }

    if ours_deleted || theirs_deleted {
        let cause = ConflictCause::DeleteVsModification;
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            cause,
            "One branch deleted the entity while the other modified it.",
        ));
    }

    let ours_touches_signature = ours_change
        .kinds
        .contains(&SemanticChangeKind::SignatureChanged)
        || ours_change.kinds.contains(&SemanticChangeKind::Renamed);
    let theirs_touches_signature = theirs_change
        .kinds
        .contains(&SemanticChangeKind::SignatureChanged)
        || theirs_change.kinds.contains(&SemanticChangeKind::Renamed);
    let ours_touches_impl = ours_change
        .kinds
        .contains(&SemanticChangeKind::ImplementationChanged);
    let theirs_touches_impl = theirs_change
        .kinds
        .contains(&SemanticChangeKind::ImplementationChanged);

    let ours_renamed = ours_change.kinds.contains(&SemanticChangeKind::Renamed);
    let theirs_renamed = theirs_change.kinds.contains(&SemanticChangeKind::Renamed);

    if ours_renamed && theirs_renamed {
        let ours_name = ours_change
            .current
            .as_ref()
            .map(|entity| entity.name.as_str());
        let theirs_name = theirs_change
            .current
            .as_ref()
            .map(|entity| entity.name.as_str());
        if ours_name != theirs_name {
            return Resolution::Conflict(build_conflict(
                base_entity,
                ours_change,
                theirs_change,
                ConflictCause::DivergentRename,
                "Both branches renamed the same entity differently.",
            ));
        }
    }

    if ours_touches_impl && theirs_touches_impl {
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            ConflictCause::ConcurrentModification,
            "Both branches changed the same implementation.",
        ));
    }

    if ours_touches_signature && theirs_touches_signature && !(ours_renamed && theirs_renamed) {
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            ConflictCause::ConcurrentModification,
            "Both branches changed the same entity signature.",
        ));
    }

    if (ours_renamed && theirs_touches_impl) || (theirs_renamed && ours_touches_impl) {
        let merged = synthesize_rename_plus_impl(base_entity, ours_change, theirs_change);
        return Resolution::Take(
            merged,
            combine_changes(base_entity, ours_change, theirs_change),
        );
    }

    if ours_touches_signature && theirs_touches_impl
        || theirs_touches_signature && ours_touches_impl
    {
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            ConflictCause::RenameVsModification,
            "Signature and implementation changed concurrently; manual review is safer for this MVP.",
        ));
    }

    if ours_change.kinds == vec![SemanticChangeKind::Moved]
        && theirs_change.kinds == vec![SemanticChangeKind::Moved]
    {
        return apply_single_change(base_entity, ours_change);
    }

    if ours_change.kinds == vec![SemanticChangeKind::Unchanged] {
        return apply_single_change(base_entity, theirs_change);
    }

    if theirs_change.kinds == vec![SemanticChangeKind::Unchanged] {
        return apply_single_change(base_entity, ours_change);
    }

    if ours_change.kinds == vec![SemanticChangeKind::Moved] {
        return apply_single_change(base_entity, theirs_change);
    }

    if theirs_change.kinds == vec![SemanticChangeKind::Moved] {
        return apply_single_change(base_entity, ours_change);
    }

    if ours_change.kinds.contains(&SemanticChangeKind::Split)
        || theirs_change.kinds.contains(&SemanticChangeKind::Split)
    {
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            ConflictCause::AmbiguousSplit,
            "A semantic split overlaps with another change and needs manual review.",
        ));
    }

    if ours_change.kinds.contains(&SemanticChangeKind::Merged)
        || theirs_change.kinds.contains(&SemanticChangeKind::Merged)
    {
        return Resolution::Conflict(build_conflict(
            base_entity,
            ours_change,
            theirs_change,
            ConflictCause::AmbiguousMerge,
            "A semantic merge/fusion overlaps with another change and needs manual review.",
        ));
    }

    apply_single_change(base_entity, ours_change)
}

fn synthesize_rename_plus_impl(
    base_entity: &SemanticEntity,
    ours_change: &SemanticChange,
    theirs_change: &SemanticChange,
) -> SemanticEntity {
    let (rename_change, impl_change) = if ours_change.kinds.contains(&SemanticChangeKind::Renamed) {
        (ours_change, theirs_change)
    } else {
        (theirs_change, ours_change)
    };

    let mut entity = impl_change
        .current
        .as_ref()
        .and_then(|snapshot| entity_from_snapshot(base_entity, snapshot))
        .unwrap_or_else(|| base_entity.clone());
    let rename_snapshot = rename_change
        .current
        .as_ref()
        .expect("rename changes always have current snapshot");

    entity.id = rename_snapshot.id.clone();
    entity.name = rename_snapshot.name.clone();
    entity.path = rename_snapshot.path.clone();
    entity.signature = rename_snapshot.signature.clone();
    entity.signature_hash = rename_snapshot.signature_hash.clone();
    entity.shape_hash = rename_snapshot.shape_hash.clone();
    entity.full_hash = build_full_hash(entity.kind, &entity.signature_hash, &entity.body_hash);
    entity
        .metadata
        .insert("synthetic_merge".to_string(), Value::Bool(true));

    entity
}

fn combine_changes(
    base_entity: &SemanticEntity,
    ours_change: &SemanticChange,
    theirs_change: &SemanticChange,
) -> SemanticChange {
    let mut kinds = ours_change.kinds.clone();
    kinds.extend(theirs_change.kinds.iter().cloned());
    kinds.sort();
    kinds.dedup();

    let current = if ours_change.kinds.contains(&SemanticChangeKind::Renamed) {
        ours_change
            .current
            .clone()
            .or_else(|| theirs_change.current.clone())
    } else {
        theirs_change
            .current
            .clone()
            .or_else(|| ours_change.current.clone())
    };

    SemanticChange {
        anchor_id: base_entity.id.clone(),
        kinds,
        previous: Some(base_entity.snapshot()),
        current,
        related_entities: Vec::new(),
        context: ours_change
            .context
            .clone()
            .or_else(|| theirs_change.context.clone()),
        explanation:
            "Auto-resolved rename in one branch with implementation-only change in the other."
                .to_string(),
    }
}

fn build_conflict(
    base_entity: &SemanticEntity,
    ours_change: &SemanticChange,
    theirs_change: &SemanticChange,
    cause: ConflictCause,
    message: &str,
) -> SemanticConflict {
    SemanticConflict {
        anchor_id: base_entity.id.clone(),
        cause,
        base: Some(base_entity.snapshot()),
        ours: ours_change
            .current
            .clone()
            .or_else(|| ours_change.previous.clone()),
        theirs: theirs_change
            .current
            .clone()
            .or_else(|| theirs_change.previous.clone()),
        context: ours_change
            .context
            .clone()
            .or_else(|| theirs_change.context.clone()),
        message: message.to_string(),
    }
}

fn entity_from_snapshot(
    base_entity: &SemanticEntity,
    snapshot: &crate::model::EntitySnapshot,
) -> Option<SemanticEntity> {
    let mut entity = base_entity.clone();
    entity.id = snapshot.id.clone();
    entity.name = snapshot.name.clone();
    entity.path = snapshot.path.clone();
    entity.signature = snapshot.signature.clone();
    entity.body_hash = snapshot.body_hash.clone();
    entity.full_hash = snapshot.full_hash.clone();
    entity.signature_hash = snapshot.signature_hash.clone();
    entity.shape_hash = snapshot.shape_hash.clone();
    entity.snippet = snapshot.snippet.clone();
    entity.body = snapshot.body.clone();
    Some(entity)
}

fn anchor_changes(changes: &[SemanticChange]) -> BTreeMap<&str, &SemanticChange> {
    changes
        .iter()
        .map(|change| (change.anchor_id.as_str(), change))
        .collect::<BTreeMap<_, _>>()
}

#[expect(
    clippy::too_many_arguments,
    reason = "Insertion resolution coordinates several merge accumulators in one place."
)]
fn merge_insertions(
    ours: &FileSemanticIndex,
    theirs: &FileSemanticIndex,
    ours_changes: &BTreeMap<&str, &SemanticChange>,
    theirs_changes: &BTreeMap<&str, &SemanticChange>,
    applied_ids: &mut BTreeSet<String>,
    merged_entities: &mut Vec<SemanticEntity>,
    resolved_changes: &mut Vec<SemanticChange>,
    conflicts: &mut Vec<SemanticConflict>,
) {
    let ours_inserted = inserted_entities(ours, ours_changes);
    let theirs_inserted = inserted_entities(theirs, theirs_changes);

    let mut theirs_by_path = theirs_inserted
        .iter()
        .map(|entity| (entity.path.as_str(), *entity))
        .collect::<BTreeMap<_, _>>();

    for ours_entity in ours_inserted {
        if applied_ids.contains(&ours_entity.id) {
            continue;
        }

        if let Some(theirs_entity) = theirs_by_path.remove(ours_entity.path.as_str()) {
            if ours_entity.full_hash == theirs_entity.full_hash {
                applied_ids.insert(ours_entity.id.clone());
                merged_entities.push(ours_entity.clone());
                resolved_changes.push(SemanticChange {
                    anchor_id: ours_entity.id.clone(),
                    kinds: vec![SemanticChangeKind::Inserted],
                    previous: None,
                    current: Some(ours_entity.snapshot()),
                    related_entities: Vec::new(),
                    context: None,
                    explanation: "Both branches inserted the same semantic entity.".to_string(),
                });
            } else {
                conflicts.push(SemanticConflict {
                    anchor_id: ours_entity.path.clone(),
                    cause: ConflictCause::ConcurrentInsertion,
                    base: None,
                    ours: Some(ours_entity.snapshot()),
                    theirs: Some(theirs_entity.snapshot()),
                    context: None,
                    message: "Both branches inserted different entities at the same semantic path."
                        .to_string(),
                });
            }

            continue;
        }

        applied_ids.insert(ours_entity.id.clone());
        merged_entities.push(ours_entity.clone());
        resolved_changes.push(SemanticChange {
            anchor_id: ours_entity.id.clone(),
            kinds: vec![SemanticChangeKind::Inserted],
            previous: None,
            current: Some(ours_entity.snapshot()),
            related_entities: Vec::new(),
            context: None,
            explanation: "Entity inserted only in ours branch.".to_string(),
        });
    }

    for theirs_entity in theirs_by_path.into_values() {
        if applied_ids.contains(&theirs_entity.id) {
            continue;
        }

        applied_ids.insert(theirs_entity.id.clone());
        merged_entities.push(theirs_entity.clone());
        resolved_changes.push(SemanticChange {
            anchor_id: theirs_entity.id.clone(),
            kinds: vec![SemanticChangeKind::Inserted],
            previous: None,
            current: Some(theirs_entity.snapshot()),
            related_entities: Vec::new(),
            context: None,
            explanation: "Entity inserted only in theirs branch.".to_string(),
        });
    }
}

fn inserted_entities<'a>(
    index: &'a FileSemanticIndex,
    changes: &BTreeMap<&str, &SemanticChange>,
) -> Vec<&'a SemanticEntity> {
    index
        .entities
        .iter()
        .filter(|entity| {
            changes
                .get(entity.id.as_str())
                .map(|change| change.kinds.contains(&SemanticChangeKind::Inserted))
                .unwrap_or(false)
        })
        .collect::<Vec<_>>()
}

fn merge_exports(
    ours: &FileSemanticIndex,
    theirs: &FileSemanticIndex,
    merged_entities: &[SemanticEntity],
) -> Vec<crate::model::ExportBinding> {
    let mut exports = BTreeMap::new();
    let live_ids = merged_entities
        .iter()
        .map(|entity| entity.id.as_str())
        .collect::<BTreeSet<_>>();
    let live_names = merged_entities
        .iter()
        .map(|entity| entity.name.as_str())
        .collect::<BTreeSet<_>>();

    for export in ours.exports.iter().chain(theirs.exports.iter()) {
        let keep = export.source.is_some()
            || export
                .entity_id
                .as_deref()
                .map(|value| live_ids.contains(value))
                .unwrap_or(false)
            || live_names.contains(export.local_name.as_str());
        if !keep {
            continue;
        }

        let key = format!(
            "{}:{}:{}",
            export.exported_name,
            export.local_name,
            export.source.clone().unwrap_or_default()
        );
        exports.entry(key).or_insert_with(|| export.clone());
    }

    exports.into_values().collect::<Vec<_>>()
}
