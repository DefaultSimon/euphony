use std::path::Path;

use miette::Result;
use serde::Deserialize;

use crate::{get_path_extension_or_empty, traits::Resolve};


#[derive(Deserialize, Clone, Debug)]
pub(crate) struct UnresolvedLibraryTranscodingConfiguration {
    audio_file_extensions: Vec<String>,

    other_file_extensions: Vec<String>,
}

impl Resolve for UnresolvedLibraryTranscodingConfiguration {
    type Resolved = LibraryTranscodingConfiguration;

    fn resolve(self) -> Self::Resolved {
        let audio_file_extensions: Vec<String> = self
            .audio_file_extensions
            .into_iter()
            .map(|extention| extention.to_ascii_lowercase())
            .collect();

        let other_file_extensions: Vec<String> = self
            .other_file_extensions
            .into_iter()
            .map(|extention| extention.to_ascii_lowercase())
            .collect();


        let mut all_tracked_extensions = Vec::with_capacity(
            audio_file_extensions.len() + other_file_extensions.len(),
        );

        all_tracked_extensions.extend(audio_file_extensions.iter().cloned());
        all_tracked_extensions.extend(other_file_extensions.iter().cloned());


        LibraryTranscodingConfiguration {
            audio_file_extensions,
            other_file_extensions,
            all_tracked_extensions,
        }
    }
}



#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LibraryTranscodingConfiguration {
    /// A list of audio file extensions (e.g. "mp3", "flac" - don't include ".").
    /// Files with these extensions are considered audio files and are transcoded using ffmpeg
    /// (see `tools.ffmpeg`).
    pub audio_file_extensions: Vec<String>,

    /// A list of other tracked file extensions (e.g. `jpg`, `png` - don't include ".").
    /// Files with these extensions are considered data files and are copied when transcoding.
    pub other_file_extensions: Vec<String>,

    /// Dynamically contains extensions from both `audio_file_extensions` and `other_file_extensions`.
    pub all_tracked_extensions: Vec<String>,
}

impl LibraryTranscodingConfiguration {
    /// Returns a boolean indicating whether the extension of the given file path is considered an audio file
    /// (based on this transcoding configuration).
    ///
    /// Returns `Err` if the extension is invalid UTF-8.
    pub fn is_audio_file_by_extension<P>(&self, file_path: P) -> Result<bool>
    where
        P: AsRef<Path>,
    {
        let extension = get_path_extension_or_empty(file_path)?;

        Ok(self.audio_file_extensions.contains(&extension))
    }

    /// Returns a boolean indicating whether the extension of the given file path is considered a data (non-audio) file
    /// (based on this transcoding configuration).
    ///
    /// Returns `Err` if the extension is invalid UTF-8.
    pub fn is_data_file_by_extension<P>(&self, file_path: P) -> Result<bool>
    where
        P: AsRef<Path>,
    {
        let extension = get_path_extension_or_empty(file_path)?;

        Ok(self.other_file_extensions.contains(&extension))
    }
}
