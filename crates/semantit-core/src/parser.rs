use std::path::Path;

use crate::error::ParseError;
use crate::model::{FileSemanticIndex, LanguageId};

#[derive(Debug, Clone, Copy)]
pub struct ParseInput<'a> {
    pub path: &'a Path,
    pub source: &'a str,
}

pub trait LanguageParser: Send + Sync {
    fn language(&self) -> LanguageId;
    fn parser_version(&self) -> &'static str;
    fn supports_path(&self, path: &Path) -> bool;
    fn parse(&self, input: ParseInput<'_>) -> std::result::Result<FileSemanticIndex, ParseError>;
}
