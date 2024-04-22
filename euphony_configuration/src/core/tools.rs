use std::{
    io,
    path::{Path, PathBuf},
};

use camino::Utf8PathBuf;
use miette::Result;
use serde::Deserialize;
use thiserror::Error;

use super::PathsConfiguration;
use crate::{
    filesystem::get_path_extension_or_empty,
    traits::TryResolveWithContext,
    utilities::replace_placeholders_in_str,
};



#[derive(Debug, Error)]
pub enum ToolsConfigurationError {
    #[error("{tool_name} binary not found on disk: provided path \"{tool_path}\" does not exist")]
    BinaryNotFound {
        tool_name: String,
        tool_path: String,
    },

    #[error("path to {tool_name} binary is not UTF-8: \"{}\"", .tool_path.display())]
    BinaryPathIsNotUtf8 {
        tool_name: String,
        tool_path: PathBuf,
    },

    #[error("path to {tool_name} binary found on disk, but it is not a file: \"{tool_path}\"")]
    BinaryNotFile {
        tool_name: String,
        tool_path: String,
    },

    #[error(
        "path to {tool_name} binary not be canonicalized: \
        \"{final_path}\" (untransformed path: \"{original_path}\")\n
        reason: {error}"
    )]
    FailedToCanonicalizeBinaryPath {
        tool_name: String,
        original_path: String,
        final_path: String,
        error: io::Error,
    },
}


#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedToolsConfiguration {
    ffmpeg: UnresolvedFfmpegToolsConfiguration,
}

#[derive(Clone)]
pub struct ToolsConfiguration {
    pub ffmpeg: FfmpegToolsConfiguration,
}


impl TryResolveWithContext for UnresolvedToolsConfiguration {
    type Resolved = ToolsConfiguration;
    type Error = ToolsConfigurationError;
    type Context = PathsConfiguration;

    fn try_resolve(
        self,
        paths: PathsConfiguration,
    ) -> Result<Self::Resolved, Self::Error> {
        Ok(ToolsConfiguration {
            ffmpeg: self.ffmpeg.try_resolve(paths)?,
        })
    }
}



#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedFfmpegToolsConfiguration {
    binary_path: String,

    audio_transcoding_args: Vec<String>,

    audio_transcoding_output_extension: String,
}

#[derive(Clone)]
pub struct FfmpegToolsConfiguration {
    /// Configures the ffmpeg binary location.
    /// The {TOOLS_BASE} placeholder is available (see `base_tools_path` in the `essentials` table)
    pub binary_path: Utf8PathBuf,

    /// These are the arguments passed to ffmpeg when converting an audio file into MP3 V0.
    /// The placeholders {INPUT_FILE} and {OUTPUT_FILE} will be replaced with the absolute path to those files.
    pub audio_transcoding_args: Vec<String>,

    /// This setting should be the extension of the audio files after transcoding.
    /// The default conversion is to MP3, but the user may set any ffmpeg conversion above, which is why this exists.
    pub audio_transcoding_output_extension: String,
}


impl FfmpegToolsConfiguration {
    /// Returns `Ok(true)` if the given path's extension matches
    /// the ffmpeg transcoding output path.
    ///
    /// Returns `Err` if the extension is not valid UTF-8.
    pub fn is_path_transcoding_output_by_extension<P: AsRef<Path>>(
        &self,
        file_path: P,
    ) -> Result<bool> {
        let extension = get_path_extension_or_empty(file_path)?;

        Ok(self.audio_transcoding_output_extension.eq(&extension))
    }
}

impl TryResolveWithContext for UnresolvedFfmpegToolsConfiguration {
    type Resolved = FfmpegToolsConfiguration;
    type Error = ToolsConfigurationError;
    type Context = PathsConfiguration;

    fn try_resolve(
        self,
        paths: PathsConfiguration,
    ) -> Result<Self::Resolved, Self::Error> {
        let canonical_ffmpeg_binary_path = {
            let final_ffmpeg_path = replace_placeholders_in_str(
                &self.binary_path,
                &paths.placeholders(),
            );

            let canonical_ffmpeg_path = dunce::canonicalize(&final_ffmpeg_path)
                .map_err(|io_error| {
                    ToolsConfigurationError::FailedToCanonicalizeBinaryPath {
                        tool_name: "ffmpeg".to_string(),
                        original_path: self.binary_path,
                        final_path: final_ffmpeg_path.to_string(),
                        error: io_error,
                    }
                })?;

            Utf8PathBuf::try_from(canonical_ffmpeg_path).map_err(|error| {
                ToolsConfigurationError::BinaryPathIsNotUtf8 {
                    tool_name: "ffmpeg".to_string(),
                    tool_path: error.into_path_buf(),
                }
            })?
        };

        if !canonical_ffmpeg_binary_path.exists() {
            return Err(ToolsConfigurationError::BinaryNotFound {
                tool_name: "ffmpeg".to_string(),
                tool_path: canonical_ffmpeg_binary_path.to_string(),
            });
        } else if !canonical_ffmpeg_binary_path.is_file() {
            return Err(ToolsConfigurationError::BinaryNotFile {
                tool_name: "ffmpeg".to_string(),
                tool_path: canonical_ffmpeg_binary_path.to_string(),
            });
        }

        let audio_transcoding_output_extension =
            self.audio_transcoding_output_extension.to_ascii_lowercase();


        Ok(FfmpegToolsConfiguration {
            binary_path: canonical_ffmpeg_binary_path,
            audio_transcoding_args: self.audio_transcoding_args,
            audio_transcoding_output_extension,
        })
    }
}
