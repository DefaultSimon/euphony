use std::{io, path::PathBuf};

use camino::Utf8PathBuf;
use serde::Deserialize;
use thiserror::Error;

use super::paths::PathsConfiguration;
use crate::{
    traits::TryResolveWithContext,
    utilities::replace_placeholders_in_str,
};


#[derive(Debug, Error)]
pub enum AggregatedLibraryConfigurationError {
    #[error("invalid number of transcode threads: expected 1 or more")]
    ZeroTranscodeThreads,

    #[error(
        "aggregated library path could not be canonicalized: \
        \"{final_path}\" (untransformed path: \"{original_path}\")\n
        reason: {error}"
    )]
    FailedToCanonicalizePath {
        original_path: String,
        final_path: String,
        error: io::Error,
    },

    #[error("aggregated library path is not UTF-8: {}", .path.display())]
    PathIsNotUtf8 { path: PathBuf },
}



#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedAggregatedLibraryConfiguration {
    path: String,

    transcode_threads: usize,

    failure_max_retries: u16,

    failure_delay_seconds: u16,
}

impl TryResolveWithContext for UnresolvedAggregatedLibraryConfiguration {
    type Resolved = AggregatedLibraryConfiguration;
    type Error = AggregatedLibraryConfigurationError;
    type Context = PathsConfiguration;

    fn try_resolve(
        self,
        paths: PathsConfiguration,
    ) -> Result<Self::Resolved, Self::Error> {
        let canonical_aggregated_library_path = {
            let final_aggregated_library_path =
                replace_placeholders_in_str(&self.path, &paths.placeholders());

            let canonical_aggregated_library_path = dunce::canonicalize(
                &final_aggregated_library_path,
            )
            .map_err(|io_error| {
                AggregatedLibraryConfigurationError::FailedToCanonicalizePath {
                    original_path: self.path,
                    final_path: final_aggregated_library_path,
                    error: io_error,
                }
            })?;

            Utf8PathBuf::try_from(canonical_aggregated_library_path).map_err(
                |error| AggregatedLibraryConfigurationError::PathIsNotUtf8 {
                    path: error.into_path_buf(),
                },
            )?
        };

        if self.transcode_threads == 0 {
            return Err(
                AggregatedLibraryConfigurationError::ZeroTranscodeThreads,
            );
        }


        Ok(AggregatedLibraryConfiguration {
            path: canonical_aggregated_library_path,
            transcode_threads: self.transcode_threads,
            failure_max_retries: self.failure_max_retries,
            failure_delay_seconds: self.failure_delay_seconds,
        })
    }
}



#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AggregatedLibraryConfiguration {
    pub path: Utf8PathBuf,

    pub transcode_threads: usize,

    pub failure_max_retries: u16,

    pub failure_delay_seconds: u16,
}
