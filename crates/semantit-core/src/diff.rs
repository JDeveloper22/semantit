use std::collections::{BTreeMap, BTreeSet};

use similar::TextDiff;
use strsim::jaro_winkler;

use crate::model::{
    ContextDiff, DiffSummary, FileSemanticIndex, SemanticChange, SemanticChangeKind, SemanticDiff,
    SemanticEntity,
};

pub fn diff_indices(old_index: &FileSemanticIndex, new_index: &FileSemanticIndex) -> SemanticDiff {
    let mut changes = Vec::new();
    let mut summary = DiffSummary::default();

    let new_by_id = new_index
        .entities
        .iter()
        .map(|entity| (entity.id.as_str(), entity))
        .collect::<BTreeMap<_, _>>();

    let mut matched_old = BTreeSet::new();
    let mut matched_new = BTreeSet::new();

    for old_entity in &old_index.entities {
        if let Some(new_entity) = new_by_id.get(old_entity.id.as_str()) {
            matched_old.insert(old_entity.id.clone());
            matched_new.insert(new_entity.id.clone());
            let change = classify_matched_entities(old_entity, new_entity, false);
            for kind in &change.kinds {
                summary.record(kind);
            }
            changes.push(change);
        }
    }

    let unmatched_old = old_index
        .entities
        .iter()
        .filter(|entity| !matched_old.contains(&entity.id))
        .collect::<Vec<_>>();
    let unmatched_new = new_index
        .entities
        .iter()
        .filter(|entity| !matched_new.contains(&entity.id))
        .collect::<Vec<_>>();

    for (old_entity, new_entity) in detect_same_name_matches(&unmatched_old, &unmatched_new) {
        matched_old.insert(old_entity.id.clone());
        matched_new.insert(new_entity.id.clone());
        let change = classify_matched_entities(old_entity, new_entity, false);
        for kind in &change.kinds {
            summary.record(kind);
        }
        changes.push(change);
    }

    let unmatched_old = old_index
        .entities
        .iter()
        .filter(|entity| !matched_old.contains(&entity.id))
        .collect::<Vec<_>>();
    let unmatched_new = new_index
        .entities
        .iter()
        .filter(|entity| !matched_new.contains(&entity.id))
        .collect::<Vec<_>>();

    for (old_entity, new_entity) in detect_renames(&unmatched_old, &unmatched_new) {
        matched_old.insert(old_entity.id.clone());
        matched_new.insert(new_entity.id.clone());
        let change = classify_matched_entities(old_entity, new_entity, true);
        for kind in &change.kinds {
            summary.record(kind);
        }
        changes.push(change);
    }

    let remaining_old = old_index
        .entities
        .iter()
        .filter(|entity| !matched_old.contains(&entity.id))
        .collect::<Vec<_>>();
    let remaining_new = new_index
        .entities
        .iter()
        .filter(|entity| !matched_new.contains(&entity.id))
        .collect::<Vec<_>>();

    let mut split_anchors = BTreeSet::new();
    for old_entity in &remaining_old {
        let candidates = remaining_new
            .iter()
            .copied()
            .filter(|candidate| split_merge_candidate(old_entity, candidate))
            .collect::<Vec<_>>();
        if candidates.len() > 1 {
            split_anchors.insert(old_entity.id.clone());
            let change = SemanticChange {
                anchor_id: old_entity.id.clone(),
                kinds: vec![SemanticChangeKind::Split],
                previous: Some(old_entity.snapshot()),
                current: None,
                related_entities: candidates
                    .into_iter()
                    .map(SemanticEntity::snapshot)
                    .collect::<Vec<_>>(),
                context: None,
                explanation: format!(
                    "Entity `{}` may have been split into multiple semantic entities.",
                    old_entity.path
                ),
            };
            summary.record(&SemanticChangeKind::Split);
            changes.push(change);
        }
    }

    let mut merge_anchors = BTreeSet::new();
    for new_entity in &remaining_new {
        let candidates = remaining_old
            .iter()
            .copied()
            .filter(|candidate| split_merge_candidate(candidate, new_entity))
            .collect::<Vec<_>>();
        if candidates.len() > 1 {
            merge_anchors.insert(new_entity.id.clone());
            let change = SemanticChange {
                anchor_id: new_entity.id.clone(),
                kinds: vec![SemanticChangeKind::Merged],
                previous: None,
                current: Some(new_entity.snapshot()),
                related_entities: candidates
                    .into_iter()
                    .map(SemanticEntity::snapshot)
                    .collect::<Vec<_>>(),
                context: None,
                explanation: format!(
                    "Entity `{}` may be the result of a semantic merge of multiple entities.",
                    new_entity.path
                ),
            };
            summary.record(&SemanticChangeKind::Merged);
            changes.push(change);
        }
    }

    for old_entity in remaining_old {
        if split_anchors.contains(&old_entity.id) {
            continue;
        }

        let change = SemanticChange {
            anchor_id: old_entity.id.clone(),
            kinds: vec![SemanticChangeKind::Deleted],
            previous: Some(old_entity.snapshot()),
            current: None,
            related_entities: Vec::new(),
            context: build_context(old_entity.snippet.as_deref(), None),
            explanation: format!("Entity `{}` was deleted.", old_entity.path),
        };
        summary.record(&SemanticChangeKind::Deleted);
        changes.push(change);
    }

    for new_entity in remaining_new {
        if merge_anchors.contains(&new_entity.id) {
            continue;
        }

        let change = SemanticChange {
            anchor_id: new_entity.id.clone(),
            kinds: vec![SemanticChangeKind::Inserted],
            previous: None,
            current: Some(new_entity.snapshot()),
            related_entities: Vec::new(),
            context: build_context(None, new_entity.snippet.as_deref()),
            explanation: format!("Entity `{}` was inserted.", new_entity.path),
        };
        summary.record(&SemanticChangeKind::Inserted);
        changes.push(change);
    }

    changes.sort_by(|left, right| left.anchor_id.cmp(&right.anchor_id));

    SemanticDiff {
        old_path: old_index.path.clone(),
        new_path: new_index.path.clone(),
        matching_strategy: "id, then conservative shape/body similarity".to_string(),
        summary,
        changes,
    }
}

fn classify_matched_entities(
    old_entity: &SemanticEntity,
    new_entity: &SemanticEntity,
    renamed: bool,
) -> SemanticChange {
    let mut kinds = Vec::new();

    if renamed {
        kinds.push(SemanticChangeKind::Renamed);
    }

    if old_entity.span.start != new_entity.span.start {
        kinds.push(SemanticChangeKind::Moved);
    }

    if old_entity.signature_hash != new_entity.signature_hash {
        kinds.push(SemanticChangeKind::SignatureChanged);
    }

    if old_entity.body_hash != new_entity.body_hash {
        kinds.push(SemanticChangeKind::ImplementationChanged);
    }

    if kinds.is_empty() {
        kinds.push(SemanticChangeKind::Unchanged);
    }

    let context = if kinds.contains(&SemanticChangeKind::ImplementationChanged) {
        build_context(old_entity.body.as_deref(), new_entity.body.as_deref())
            .or_else(|| build_context(old_entity.snippet.as_deref(), new_entity.snippet.as_deref()))
    } else {
        build_context(old_entity.snippet.as_deref(), new_entity.snippet.as_deref())
    };

    let explanation = if kinds == [SemanticChangeKind::Unchanged] {
        format!("Entity `{}` is unchanged.", old_entity.path)
    } else {
        describe_change(old_entity, new_entity, &kinds)
    };

    SemanticChange {
        anchor_id: old_entity.id.clone(),
        kinds,
        previous: Some(old_entity.snapshot()),
        current: Some(new_entity.snapshot()),
        related_entities: Vec::new(),
        context,
        explanation,
    }
}

fn describe_change(
    old_entity: &SemanticEntity,
    new_entity: &SemanticEntity,
    kinds: &[SemanticChangeKind],
) -> String {
    let labels = kinds
        .iter()
        .map(|kind| match kind {
            SemanticChangeKind::Moved => "moved",
            SemanticChangeKind::SignatureChanged => "signature changed",
            SemanticChangeKind::ImplementationChanged => "implementation changed",
            SemanticChangeKind::Renamed => "renamed",
            SemanticChangeKind::Unchanged => "unchanged",
            SemanticChangeKind::Inserted => "inserted",
            SemanticChangeKind::Deleted => "deleted",
            SemanticChangeKind::Split => "split",
            SemanticChangeKind::Merged => "merged",
        })
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        "Entity `{}` -> `{}` was {}.",
        old_entity.path, new_entity.path, labels
    )
}

fn detect_same_name_matches<'a>(
    unmatched_old: &[&'a SemanticEntity],
    unmatched_new: &[&'a SemanticEntity],
) -> Vec<(&'a SemanticEntity, &'a SemanticEntity)> {
    let mut matches = Vec::new();
    let mut used_old = BTreeSet::new();
    let mut used_new = BTreeSet::new();

    for old_entity in unmatched_old {
        let candidates = unmatched_new
            .iter()
            .copied()
            .filter(|new_entity| {
                old_entity.kind == new_entity.kind
                    && old_entity.name == new_entity.name
                    && old_entity.parent == new_entity.parent
                    && !used_new.contains(&new_entity.id)
            })
            .collect::<Vec<_>>();

        if candidates.len() != 1 || used_old.contains(&old_entity.id) {
            continue;
        }

        let candidate = candidates[0];
        let similarity = match (old_entity.body.as_deref(), candidate.body.as_deref()) {
            (Some(left), Some(right)) => jaro_winkler(left, right),
            _ => 0.0,
        };
        if old_entity.body_hash == candidate.body_hash || similarity >= 0.55 {
            used_old.insert(old_entity.id.clone());
            used_new.insert(candidate.id.clone());
            matches.push((*old_entity, candidate));
        }
    }

    matches
}

fn detect_renames<'a>(
    unmatched_old: &[&'a SemanticEntity],
    unmatched_new: &[&'a SemanticEntity],
) -> Vec<(&'a SemanticEntity, &'a SemanticEntity)> {
    let mut candidates = unmatched_old
        .iter()
        .flat_map(|old_entity| {
            unmatched_new.iter().filter_map(move |new_entity| {
                if old_entity.kind != new_entity.kind
                    || old_entity.parent != new_entity.parent
                    || old_entity.shape_hash != new_entity.shape_hash
                {
                    return None;
                }

                let signature_score = jaro_winkler(&old_entity.signature, &new_entity.signature);
                let body_score = match (old_entity.body.as_deref(), new_entity.body.as_deref()) {
                    (Some(left), Some(right)) => jaro_winkler(left, right),
                    _ => 0.0,
                };
                let exact_body = old_entity.body_hash == new_entity.body_hash;
                let score = if exact_body {
                    1.0
                } else {
                    signature_score.max(body_score)
                };

                (score >= 0.92).then_some((score, *old_entity, *new_entity))
            })
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .0
            .partial_cmp(&left.0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut used_old = BTreeSet::new();
    let mut used_new = BTreeSet::new();
    let mut matches = Vec::new();

    for (_, old_entity, new_entity) in candidates {
        if used_old.contains(&old_entity.id) || used_new.contains(&new_entity.id) {
            continue;
        }

        used_old.insert(old_entity.id.clone());
        used_new.insert(new_entity.id.clone());
        matches.push((old_entity, new_entity));
    }

    matches
}

fn split_merge_candidate(left: &SemanticEntity, right: &SemanticEntity) -> bool {
    if left.kind != right.kind || left.parent != right.parent {
        return false;
    }

    let body_similarity = match (left.body.as_deref(), right.body.as_deref()) {
        (Some(lhs), Some(rhs)) => jaro_winkler(lhs, rhs),
        _ => 0.0,
    };

    body_similarity >= 0.70
}

fn build_context(before: Option<&str>, after: Option<&str>) -> Option<ContextDiff> {
    if before.is_none() && after.is_none() {
        return None;
    }

    let before_value = before.unwrap_or_default();
    let after_value = after.unwrap_or_default();
    let diff = TextDiff::from_lines(before_value, after_value);
    let unified = diff
        .unified_diff()
        .context_radius(2)
        .header("before", "after")
        .to_string();

    Some(ContextDiff {
        before: before.map(ToOwned::to_owned),
        after: after.map(ToOwned::to_owned),
        unified_diff: unified,
    })
}
