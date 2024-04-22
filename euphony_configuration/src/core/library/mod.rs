use std::{io, path::PathBuf};

use camino::Utf8PathBuf;
use serde::Deserialize;
use thiserror::Error;

use super::PathsConfiguration;
use crate::{
    traits::{Resolve, TryResolveWithContext},
    utilities::replace_placeholders_in_str,
};


mod transcoding;
mod validation;
pub use transcoding::*;
pub use validation::*;


#[derive(Debug, Error)]
pub enum LibraryConfigurationError {
    #[error(
        "library path does not exist: {}",
        .library_path
    )]
    LibraryPathNotFound { library_path: String },

    #[error(
        "library path exists, but is not a directory: {}",
        .library_path
    )]
    LibraryPathNotDirectory { library_path: String },

    #[error(
        "library path could not be canonicalized: \
        \"{}\" (original) -> \"{}\" (final)\n
        reason: {}",
        .original_path,
        .final_path,
        .error
    )]
    FailedToCanonicalizeLibraryPath {
        original_path: String,
        final_path: String,
        error: io::Error,
    },

    #[error("library path is not UTF-8: {}", .path.display())]
    LibraryPathIsNotUtf8 { path: PathBuf },
}


#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedLibraryConfiguration {
    name: String,

    path: String,

    ignored_directories_in_base_directory: Option<Vec<String>>,

    validation: UnresolvedLibraryValidationConfiguration,

    transcoding: UnresolvedLibraryTranscodingConfiguration,
}

impl TryResolveWithContext for UnresolvedLibraryConfiguration {
    type Resolved = LibraryConfiguration;
    type Error = LibraryConfigurationError;
    type Context = PathsConfiguration;

    fn try_resolve(
        self,
        paths: PathsConfiguration,
    ) -> Result<Self::Resolved, Self::Error> {
        let canonical_library_path = {
            let final_library_path =
                replace_placeholders_in_str(&self.path, &paths.placeholders());

            let canonical_library_path = dunce::canonicalize(
                &final_library_path,
            )
            .map_err(|io_error| {
                LibraryConfigurationError::FailedToCanonicalizeLibraryPath {
                    original_path: self.path,
                    final_path: final_library_path,
                    error: io_error,
                }
            })?;

            Utf8PathBuf::try_from(canonical_library_path).map_err(|error| {
                LibraryConfigurationError::LibraryPathIsNotUtf8 {
                    path: error.into_path_buf(),
                }
            })?
        };

        if !canonical_library_path.exists() {
            return Err(LibraryConfigurationError::LibraryPathNotFound {
                library_path: canonical_library_path.into_string(),
            });
        } else if !canonical_library_path.is_dir() {
            return Err(
                LibraryConfigurationError::LibraryPathNotDirectory {
                    library_path: canonical_library_path.into_string(),
                },
            );
        }


        Ok(LibraryConfiguration {
            name: self.name,
            path: canonical_library_path,
            ignored_directories_in_base_directory: self
                .ignored_directories_in_base_directory,
            validation: self.validation.resolve(),
            transcoding: self.transcoding.resolve(),
        })
    }
}



#[derive(Clone, PartialEq, Eq, Debug)]
#[non_exhaustive]
pub struct LibraryConfiguration {
    /// Library display name.
    pub name: String,

    /// Absolute canonical path to the library.
    pub path: Utf8PathBuf,

    pub ignored_directories_in_base_directory: Option<Vec<String>>,

    /// Validation-related configuration for this library.
    pub validation: LibraryValidationConfiguration,

    /// Transcoding-related configuration for this library.
    pub transcoding: LibraryTranscodingConfiguration,
}
