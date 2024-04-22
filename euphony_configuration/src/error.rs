use std::{io, path::PathBuf};

use miette::Diagnostic;
use thiserror::Error;

use crate::core::ConfigurationResolutionError;

/// A general configuration error, returned from configuration loading functions.
#[derive(Error, Debug, Diagnostic)]
pub enum ConfigurationError {
    /// The file at the provided file path could not be loaded,
    /// e.g. because it does not exist or due to missing read permissions.
    #[error(
        "Failed to load configuration file \"{}\" from disk: {error:?}",
        .file_path.display()
    )]
    FileLoadError {
        file_path: PathBuf,
        error: Box<io::Error>,
    },

    /// The file at the provided file path was read,
    /// but its contents were not valid TOML.
    #[error(
        "Failed to parse configuration file \"{}\" as TOML: {error:?}.",
        .file_path.display()
    )]
    FileFormatError {
        file_path: PathBuf,
        error: Box<toml::de::Error>,
    },

    /// The file was read and parsed as TOML,
    /// but the actual contents (tables and fields) were invalid.
    ///
    /// This can happen when, for example, the user provides a string
    /// in place of an integer field.
    #[error("Failed to validate configuration: {error:?}")]
    InvalidContent { error: ConfigurationResolutionError },

    /// Other uncategorized (and unlikely) errors.
    #[error("Other error: {error:?}")]
    OtherError { error: miette::Report },
}
