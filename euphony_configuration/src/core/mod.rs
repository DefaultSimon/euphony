//! Contains the core `euphony` configuration.

mod aggregated_library;
mod library;
mod logging;
mod paths;
mod tools;
mod ui;
mod validation;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use miette::Result;
use serde::Deserialize;
use thiserror::Error;

pub use self::aggregated_library::*;
pub use self::library::*;
pub use self::logging::*;
pub use self::paths::*;
pub use self::tools::*;
pub use self::ui::*;
pub use self::validation::*;
use crate::traits::{Resolve, TryResolve, TryResolveWithContext};
use crate::utilities::get_default_configuration_file_path;
use crate::ConfigurationError;


/// An error that can occurr during validation and resolution of the `configuration.toml` file.
#[derive(Debug, Error)]
pub enum ConfigurationResolutionError {
    #[error(transparent)]
    InPaths {
        #[from]
        error: PathsConfigurationError,
    },

    #[error(transparent)]
    InLogging {
        #[from]
        error: LoggingConfigurationError,
    },

    #[error(transparent)]
    InTools {
        #[from]
        error: ToolsConfigurationError,
    },

    #[error(
        "library display name conflict: \
        two libraries with the display name \"{library_display_name}\" are present in the configuration",
    )]
    LibraryDisplayNameConflict { library_display_name: String },

    #[error(
        "failed to validate and resolve library configuration for \"{library_key}\": {error}"
    )]
    InLibrary {
        library_key: String,
        error: LibraryConfigurationError,
    },

    #[error(transparent)]
    InAggregatedLibrary {
        #[from]
        error: AggregatedLibraryConfigurationError,
    },
}



#[derive(Deserialize, Clone)]
struct UnresolvedConfiguration {
    paths: UnresolvedPathsConfiguration,

    logging: UnresolvedLoggingConfiguration,

    ui: UnresolvedUiConfiguration,

    validation: UnresolvedValidationConfiguration,

    tools: UnresolvedToolsConfiguration,

    libraries: BTreeMap<String, UnresolvedLibraryConfiguration>,

    aggregated_library: UnresolvedAggregatedLibraryConfiguration,
}

/// This struct contains the entire `euphony` configuration,
/// from tool paths to libraries and so forth.
#[derive(Clone)]
pub struct Configuration {
    /// Path to the file from which this configuration was loaded.
    pub configuration_file_path: PathBuf,

    pub paths: PathsConfiguration,

    pub logging: LoggingConfiguration,

    pub ui: UiConfiguration,

    pub validation: ValidationConfiguration,

    pub tools: ToolsConfiguration,

    pub libraries: HashMap<String, LibraryConfiguration>,

    // TODO Should I rename "aggregated library" to something else, like "transcoded library"?
    pub aggregated_library: AggregatedLibraryConfiguration,
}


impl TryResolveWithContext for UnresolvedConfiguration {
    type Resolved = Configuration;
    type Error = ConfigurationResolutionError;
    type Context = PathBuf;

    fn try_resolve(
        self,
        configuration_file_path: PathBuf,
    ) -> Result<Self::Resolved, Self::Error> {
        let paths = self.paths.try_resolve()?;

        let logging = self.logging.try_resolve(paths.clone())?;

        let ui = self.ui.resolve();

        let validation = self.validation.resolve();

        let tools = self.tools.try_resolve(paths.clone())?;


        let mut libraries: HashMap<String, LibraryConfiguration> =
            HashMap::with_capacity(self.libraries.len());
        let mut library_names: HashSet<String> =
            HashSet::with_capacity(self.libraries.len());

        for (key, unresolved_library) in self.libraries {
            let resolved_library =
                unresolved_library.try_resolve(paths.clone()).map_err(
                    |library_error| ConfigurationResolutionError::InLibrary {
                        library_key: key.clone(),
                        error: library_error,
                    },
                )?;

            if library_names.contains(&resolved_library.name) {
                return Err(
                    ConfigurationResolutionError::LibraryDisplayNameConflict {
                        library_display_name: resolved_library.name,
                    },
                );
            }

            library_names.insert(resolved_library.name.clone());
            libraries.insert(key, resolved_library);
        }


        let aggregated_library =
            self.aggregated_library.try_resolve(paths.clone())?;



        Ok(Configuration {
            paths,
            logging,
            ui,
            validation,
            tools,
            libraries,
            aggregated_library,
            configuration_file_path,
        })
    }
}


impl Configuration {
    pub fn load_from_path<S: Into<PathBuf>>(
        configuration_filepath: S,
    ) -> Result<Configuration, ConfigurationError> {
        let configuration_file_path: PathBuf = configuration_filepath.into();

        // Read the configuration file into memory.
        let configuration_string = fs::read_to_string(&configuration_file_path)
            .map_err(|io_error| ConfigurationError::FileLoadError {
                file_path: configuration_file_path.clone(),
                error: Box::new(io_error),
            })?;


        // Parse the string into the [`UnresolvedConfiguration`] struct,
        // then resolve it into the final [`Configuration`] struct.
        let unresolved_configuration: UnresolvedConfiguration =
            toml::from_str(&configuration_string).map_err(|toml_error| {
                ConfigurationError::FileFormatError {
                    file_path: configuration_file_path.clone(),
                    error: Box::new(toml_error),
                }
            })?;

        let resolved_configuration = unresolved_configuration
            .try_resolve(configuration_file_path)
            .map_err(
                |validation_error| ConfigurationError::InvalidContent {
                    error: validation_error,
                },
            )?;


        Ok(resolved_configuration)
    }

    pub fn load_default_path() -> Result<Configuration, ConfigurationError> {
        let default_configuration_file_path =
            get_default_configuration_file_path()?;

        Configuration::load_from_path(default_configuration_file_path)
    }

    pub fn is_library<P: AsRef<Path>>(&self, library_path: P) -> bool {
        for library in self.libraries.values() {
            let current_path = Path::new(&library.path);
            if current_path.eq(library_path.as_ref()) {
                return true;
            }
        }

        false
    }

    pub fn get_library_name_from_path<P: AsRef<Path>>(
        &self,
        library_path: P,
    ) -> Option<String> {
        for library in self.libraries.values() {
            let current_path = Path::new(&library.path);
            if current_path.eq(library_path.as_ref()) {
                return Some(library.name.clone());
            }
        }

        None
    }

    pub fn get_library_by_full_name<S: AsRef<str>>(
        &self,
        library_name: S,
    ) -> Option<&LibraryConfiguration> {
        self.libraries
            .values()
            .find(|library| library.name.eq(library_name.as_ref()))
    }
}
