use std::path::Path;
use std::sync::Arc;

use crate::error::{RegistryError, Result};
use crate::model::FileSemanticIndex;
use crate::parser::{LanguageParser, ParseInput};

#[derive(Default)]
pub struct ParserRegistry {
    parsers: Vec<Arc<dyn LanguageParser>>,
}

impl ParserRegistry {
    pub fn register<P>(&mut self, parser: P)
    where
        P: LanguageParser + 'static,
    {
        self.parsers.push(Arc::new(parser));
    }

    pub fn parser_for_path(&self, path: &Path) -> Result<Arc<dyn LanguageParser>> {
        self.parsers
            .iter()
            .find(|parser| parser.supports_path(path))
            .cloned()
            .ok_or_else(|| RegistryError::NoParserForPath {
                path: path.display().to_string(),
            })
            .map_err(Into::into)
    }

    pub fn parse(&self, input: ParseInput<'_>) -> Result<FileSemanticIndex> {
        let parser = self.parser_for_path(input.path)?;
        parser.parse(input).map_err(Into::into)
    }
}
