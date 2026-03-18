use crate::model::LanguageId;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SemantitError>;

#[derive(Debug, Error)]
pub enum SemantitError {
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Registry(#[from] RegistryError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("unsupported language for `{path}`")]
    UnsupportedLanguage { path: String },
    #[error("{language} parser failed for `{path}`: {message}")]
    ParserFailure {
        language: LanguageId,
        path: String,
        message: String,
    },
    #[error("could not build semantic index for `{path}`: {message}")]
    InvalidSource { path: String, message: String },
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("no parser registered for `{path}`")]
    NoParserForPath { path: String },
}
