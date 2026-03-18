use blake3::Hasher;

use crate::model::EntityKind;

pub fn hash_string(value: impl AsRef<str>) -> String {
    let mut hasher = Hasher::new();
    hasher.update(value.as_ref().as_bytes());
    hasher.finalize().to_hex().to_string()
}

pub fn hash_join<'a>(parts: impl IntoIterator<Item = &'a str>) -> String {
    let mut hasher = Hasher::new();

    for part in parts {
        hasher.update(part.as_bytes());
        hasher.update(&[0]);
    }

    hasher.finalize().to_hex().to_string()
}

pub fn normalize_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn build_entity_id(
    kind: EntityKind,
    parent_anchor: &str,
    declared_name: &str,
    normalized_signature_shape: &str,
) -> String {
    hash_join([
        "entity-id-v1",
        kind.as_str(),
        parent_anchor,
        declared_name,
        normalized_signature_shape,
    ])
}

pub fn build_shape_hash(kind: EntityKind, parent_anchor: &str, signature_shape: &str) -> String {
    hash_join([
        "entity-shape-v1",
        kind.as_str(),
        parent_anchor,
        signature_shape,
    ])
}

pub fn build_full_hash(kind: EntityKind, signature_hash: &str, body_hash: &str) -> String {
    hash_join(["entity-full-v1", kind.as_str(), signature_hash, body_hash])
}
