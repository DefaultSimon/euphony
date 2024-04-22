use std::fs;
use std::path::PathBuf;


mod scan;
use serde::Deserialize;

pub use self::scan::*;
use crate::error::ConfigurationError;
use crate::traits::{Resolve, ResolveWithContext};


/// The file name for the album overrides (see [`AlbumConfiguration`]).
///
/// This file is not required to exist in each album directory,
/// but the user may create it to influence
/// various configuration values per-album.
pub const ALBUM_OVERRIDE_FILE_NAME: &str = ".album.override.euphony";



#[derive(Deserialize, Clone, Debug)]
pub(crate) struct UnresolvedAlbumConfiguration {
    /// Album file scanning options.
    #[serde(default)]
    scan: UnresolvedAlbumScanConfiguration,
}

impl ResolveWithContext for UnresolvedAlbumConfiguration {
    type Resolved = AlbumConfiguration;
    type Context = PathBuf;

    fn resolve(self, context: Self::Context) -> Self::Resolved {
        let scan = self.scan.resolve();

        Self::Resolved {
            configuration_file_path: context,
            scan,
        }
    }
}


/// Album-specific options for `euphony`.
///
/// Usage: create a `.album.override.euphony` file in an album directory.
/// You can look at the structure below or copy a template from
/// `data/.album.override.TEMPLATE.euphony`.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct AlbumConfiguration {
    /// Path to the file from which this album configuration was loaded.
    pub configuration_file_path: PathBuf,

    /// Album file scanning options.
    pub scan: AlbumScanConfiguration,
}

impl AlbumConfiguration {
    /// Given a `directory_path`, load its `.album.override.euphony` file.
    /// If the file does not exist in the given directory, a default [`AlbumConfiguration`] will be returned.
    ///
    /// NOTE: Any optional values will be filled with defaults
    /// (e.g. `scan.depth` will default to `0` -- see [`DEFAULT_SCAN_DEPTH`][self::scan::DEFAULT_SCAN_DEPTH]).
    pub fn load_or_default<P: Into<PathBuf>>(
        album_directory_path: P,
    ) -> Result<AlbumConfiguration, ConfigurationError> {
        let album_configuration_file_path: PathBuf =
            album_directory_path.into().join(ALBUM_OVERRIDE_FILE_NAME);

        // If no override exists, just return the defaults.
        if !album_configuration_file_path.is_file() {
            return Ok(AlbumConfiguration::default());
        }

        // It it exists, load the configuration and resolve its contents.
        let album_override_configuration_string = fs::read_to_string(
            &album_configuration_file_path,
        )
        .map_err(|error| ConfigurationError::FileLoadError {
            file_path: album_configuration_file_path.clone(),
            error: Box::new(error),
        })?;


        let unresolved_album_configuration: UnresolvedAlbumConfiguration =
            toml::from_str(&album_override_configuration_string).map_err(
                |error| ConfigurationError::FileFormatError {
                    file_path: album_configuration_file_path.clone(),
                    error: Box::new(error),
                },
            )?;

        let resolved_album_configuration = unresolved_album_configuration
            .resolve(album_configuration_file_path);


        Ok(resolved_album_configuration)
    }
}
