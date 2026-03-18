use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum LanguageId {
    #[serde(rename = "typescript")]
    TypeScript,
}

impl fmt::Display for LanguageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::TypeScript => "typescript",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EntityKind {
    Function,
    Class,
    Method,
    Interface,
    TypeAlias,
    Constant,
    Variable,
}

impl EntityKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Class => "class",
            Self::Method => "method",
            Self::Interface => "interface",
            Self::TypeAlias => "type_alias",
            Self::Constant => "constant",
            Self::Variable => "variable",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanLocation {
    pub byte: u32,
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpanRange {
    pub start: SpanLocation,
    pub end: SpanLocation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticEntity {
    pub id: String,
    pub kind: EntityKind,
    pub name: String,
    pub signature: String,
    pub path: String,
    pub parent: Option<String>,
    pub body_hash: String,
    pub full_hash: String,
    pub signature_hash: String,
    pub shape_hash: String,
    pub span: SpanRange,
    pub semantic_span: SpanRange,
    pub dependencies: Vec<String>,
    pub children: Vec<String>,
    pub metadata: BTreeMap<String, Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

impl SemanticEntity {
    pub fn snapshot(&self) -> EntitySnapshot {
        EntitySnapshot {
            id: self.id.clone(),
            kind: self.kind,
            name: self.name.clone(),
            path: self.path.clone(),
            signature: self.signature.clone(),
            body_hash: self.body_hash.clone(),
            full_hash: self.full_hash.clone(),
            signature_hash: self.signature_hash.clone(),
            shape_hash: self.shape_hash.clone(),
            snippet: self.snippet.clone(),
            body: self.body.clone(),
        }
    }

    pub fn metadata_str(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).and_then(Value::as_str)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportBinding {
    pub exported_name: String,
    pub local_name: String,
    pub entity_id: Option<String>,
    pub is_default: bool,
    pub type_only: bool,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileSemanticIndex {
    pub language: LanguageId,
    pub path: String,
    pub entities: Vec<SemanticEntity>,
    pub root_entities: Vec<String>,
    pub exports: Vec<ExportBinding>,
    pub parser_version: String,
}

impl FileSemanticIndex {
    pub fn entity_by_id(&self, id: &str) -> Option<&SemanticEntity> {
        self.entities.iter().find(|entity| entity.id == id)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntitySnapshot {
    pub id: String,
    pub kind: EntityKind,
    pub name: String,
    pub path: String,
    pub signature: String,
    pub body_hash: String,
    pub full_hash: String,
    pub signature_hash: String,
    pub shape_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snippet: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SemanticChangeKind {
    Unchanged,
    Moved,
    SignatureChanged,
    ImplementationChanged,
    Renamed,
    Inserted,
    Deleted,
    Split,
    Merged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextDiff {
    pub before: Option<String>,
    pub after: Option<String>,
    pub unified_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct DiffSummary {
    pub unchanged: usize,
    pub moved: usize,
    pub signature_changed: usize,
    pub implementation_changed: usize,
    pub renamed: usize,
    pub inserted: usize,
    pub deleted: usize,
    pub split: usize,
    pub merged: usize,
}

impl DiffSummary {
    pub fn record(&mut self, kind: &SemanticChangeKind) {
        match kind {
            SemanticChangeKind::Unchanged => self.unchanged += 1,
            SemanticChangeKind::Moved => self.moved += 1,
            SemanticChangeKind::SignatureChanged => self.signature_changed += 1,
            SemanticChangeKind::ImplementationChanged => self.implementation_changed += 1,
            SemanticChangeKind::Renamed => self.renamed += 1,
            SemanticChangeKind::Inserted => self.inserted += 1,
            SemanticChangeKind::Deleted => self.deleted += 1,
            SemanticChangeKind::Split => self.split += 1,
            SemanticChangeKind::Merged => self.merged += 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticChange {
    pub anchor_id: String,
    pub kinds: Vec<SemanticChangeKind>,
    pub previous: Option<EntitySnapshot>,
    pub current: Option<EntitySnapshot>,
    pub related_entities: Vec<EntitySnapshot>,
    pub context: Option<ContextDiff>,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticDiff {
    pub old_path: String,
    pub new_path: String,
    pub matching_strategy: String,
    pub summary: DiffSummary,
    pub changes: Vec<SemanticChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConflictCause {
    ConcurrentModification,
    RenameVsModification,
    DeleteVsModification,
    DivergentRename,
    ConcurrentInsertion,
    AmbiguousSplit,
    AmbiguousMerge,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SemanticConflict {
    pub anchor_id: String,
    pub cause: ConflictCause,
    pub base: Option<EntitySnapshot>,
    pub ours: Option<EntitySnapshot>,
    pub theirs: Option<EntitySnapshot>,
    pub context: Option<ContextDiff>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MergeStatus {
    Clean,
    Conflicted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MergeResult {
    pub status: MergeStatus,
    pub resolved_changes: Vec<SemanticChange>,
    pub conflicts: Vec<SemanticConflict>,
    pub merged_index: Option<FileSemanticIndex>,
    pub notes: Vec<String>,
}
