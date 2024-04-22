use std::{collections::HashMap, env::current_exe, io, path::PathBuf};

use camino::Utf8PathBuf;
use serde::Deserialize;
use thiserror::Error;

use crate::traits::TryResolve;


#[derive(Debug, Error)]
pub enum PathsConfigurationError {
    #[error(
        "failed to get path to current executable: {:?}", .error
    )]
    FailedToGetCurrentExecutable { error: Option<io::Error> },

    #[error("provided path is not UTF-8: {}", .path.display())]
    PathIsNotUtf8 { path: PathBuf },

    #[error(
        "base library path does not exist on disk: \
        \"{}\" (untransformed path: \"{}\")",
        .final_path,
        .original_path
    )]
    BaseLibraryPathNotFound {
        original_path: String,
        final_path: String,
    },

    #[error(
        "base library path exists, but is not a directory: \
        \"{}\" (untransformed path: \"{}\")",
        .final_path,
        .original_path
    )]
    BaseLibraryPathNotADirectory {
        original_path: String,
        final_path: String,
    },

    #[error(
        "base library path could not be canonicalized: \
        \"{}\" (untransformed path: \"{}\")\n
        reason: {}",
        .final_path,
        .original_path,
        .error
    )]
    FailedToCanonicalizeBaseLibraryPath {
        original_path: String,
        final_path: String,
        error: io::Error,
    },

    #[error(
        "base tools path does not exist on disk: \
        \"{}\" (untransformed path: \"{}\")",
        .final_path,
        .original_path
    )]
    BaseToolsPathNotFound {
        original_path: String,
        final_path: String,
    },

    #[error(
        "base tools path exists, but is not a directory: \
        \"{}\" (untransformed path: \"{}\")",
        .final_path,
        .original_path
    )]
    BaseToolsPathNotADirectory {
        original_path: String,
        final_path: String,
    },

    #[error(
        "base tools path could not be canonicalized: \
        \"{}\" (untransformed path: \"{}\")\n
        reason: {}",
        .final_path,
        .original_path,
        .error
    )]
    FailedToCanonicalizeBaseToolsPath {
        original_path: String,
        final_path: String,
        error: io::Error,
    },
}


#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedPathsConfiguration {
    base_library_path: String,

    base_tools_path: String,
}

/// Base paths - reusable values such as the base library path and base tools path.
#[derive(Clone)]
pub struct PathsConfiguration {
    pub base_library_path: Utf8PathBuf,

    pub base_tools_path: Utf8PathBuf,
}


impl TryResolve for UnresolvedPathsConfiguration {
    type Resolved = PathsConfiguration;
    type Error = PathsConfigurationError;

    fn try_resolve(self) -> Result<Self::Resolved, Self::Error> {
        // Replaces any placeholders and validates the paths.
        let executable_directory = {
            let binary_path = current_exe()
            .map_err(|io_error| {
                PathsConfigurationError::FailedToGetCurrentExecutable {
                    error: Some(io_error),
                }
            })?;

            let binary_path_directory = binary_path
            .parent()
            .ok_or_else(|| {
                PathsConfigurationError::FailedToGetCurrentExecutable {
                    error: None,
                }
            })?;

            binary_path_directory
                .to_str()
                .ok_or_else(|| {
                    PathsConfigurationError::PathIsNotUtf8 { path: binary_path.to_path_buf() }
                })?
                .to_string()
        };


        let base_library_path = {
            let path_string = self
                .base_library_path
                .replace("{BINARY_DIRECTORY_PATH}", &executable_directory);

            let canonical_path = dunce::canonicalize(&path_string)
                .map_err(|io_error| PathsConfigurationError::FailedToCanonicalizeBaseLibraryPath {
                    original_path: self.base_library_path.clone(),
                    final_path: path_string,
                    error: io_error 
                })?;

            Utf8PathBuf::try_from(canonical_path)
                .map_err(|error| PathsConfigurationError::PathIsNotUtf8 {
                    path: error.into_path_buf(),
                })?
        };

        let base_tools_path = {
            let path_string = self
                .base_tools_path
                .replace("{BINARY_DIRECTORY_PATH}", &executable_directory);

            let canonical_path =
                dunce::canonicalize(&path_string).map_err(|io_error| {
                    PathsConfigurationError::FailedToCanonicalizeBaseToolsPath {
                        original_path: self.base_library_path.clone(),
                        final_path: path_string,
                        error: io_error,
                    }
                })?;

            Utf8PathBuf::try_from(canonical_path)
                .map_err(|error| PathsConfigurationError::PathIsNotUtf8 {
                    path: error.into_path_buf(),
                })?
        };


        Ok(PathsConfiguration {
            base_library_path,
            base_tools_path,
        })
    }
}


impl PathsConfiguration {
    pub fn placeholders(&self) -> HashMap<&'static str, String> {
        let mut placeholders_map = HashMap::with_capacity(2);

        placeholders_map.insert(
            "{LIBRARY_DIRECTORY}",
            self.base_library_path.to_string(),
        );
        placeholders_map
            .insert("{TOOLS_DIRECTORY}", self.base_tools_path.to_string());

        placeholders_map
    }
}
