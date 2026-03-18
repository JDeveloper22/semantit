#![forbid(unsafe_code)]

pub mod diff;
pub mod error;
pub mod hash;
pub mod merge;
pub mod model;
pub mod parser;
pub mod registry;

pub use crate::diff::diff_indices;
pub use crate::error::{ParseError, RegistryError, Result, SemantitError};
pub use crate::merge::merge_indices;
pub use crate::model::{
    ConflictCause, ContextDiff, DiffSummary, EntityKind, EntitySnapshot, ExportBinding,
    FileSemanticIndex, LanguageId, MergeResult, MergeStatus, SemanticChange, SemanticChangeKind,
    SemanticConflict, SemanticEntity, SpanLocation, SpanRange,
};
pub use crate::parser::{LanguageParser, ParseInput};
pub use crate::registry::ParserRegistry;
