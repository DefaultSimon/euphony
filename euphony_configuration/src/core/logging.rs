use std::{env::current_exe, io, path::PathBuf};

use camino::Utf8PathBuf;
use chrono::Local;
use serde::Deserialize;
use thiserror::Error;

use super::PathsConfiguration;
use crate::{
    traits::TryResolveWithContext,
    utilities::replace_placeholders_in_utf8_path,
};


#[derive(Debug, Error)]
pub enum LoggingConfigurationError {
    #[error(
        "failed to get path to current executable: {:?}", .error
    )]
    FailedToGetCurrentExecutable { error: Option<io::Error> },

    #[error("provided path is not UTF-8: {}", .path.display())]
    PathIsNotUtf8 { path: PathBuf },
}


#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedLoggingConfiguration {
    log_output_path: Option<String>,
}

#[derive(Clone)]
pub struct LoggingConfiguration {
    pub log_output_path: Option<Utf8PathBuf>,
}


impl TryResolveWithContext for UnresolvedLoggingConfiguration {
    type Resolved = LoggingConfiguration;
    type Error = LoggingConfigurationError;
    type Context = PathsConfiguration;

    fn try_resolve(
        self,
        paths: PathsConfiguration,
    ) -> Result<Self::Resolved, Self::Error> {
        let executable_directory = {
            let binary_path = current_exe().map_err(|io_error| {
                LoggingConfigurationError::FailedToGetCurrentExecutable {
                    error: Some(io_error),
                }
            })?;

            let binary_path_directory =
                binary_path.parent().ok_or_else(|| {
                    LoggingConfigurationError::FailedToGetCurrentExecutable {
                        error: None,
                    }
                })?;

            binary_path_directory
                .to_str()
                .ok_or_else(|| LoggingConfigurationError::PathIsNotUtf8 {
                    path: binary_path_directory.to_path_buf(),
                })?
                .to_string()
        };


        let time_now = Local::now();
        let formatted_time_now = time_now.format("%Y-%m-%d_%H-%M-%S");


        let log_output_path = if let Some(log_output_path) = self.log_output_path
        {
            let log_output_path = Utf8PathBuf::from(log_output_path);

            let mut placeholders = paths.placeholders();
            placeholders.insert(
                "{BINARY_DIRECTORY_PATH}",
                executable_directory.to_string(),
            );
            placeholders.insert(
                "{STARTUP_DATE_TIME}",
                formatted_time_now.to_string(),
            );


            let final_log_output_path = replace_placeholders_in_utf8_path(
                &log_output_path,
                &placeholders,
            );

            Some(final_log_output_path)
        } else {
            None
        };


        Ok(LoggingConfiguration { log_output_path })
    }
}
