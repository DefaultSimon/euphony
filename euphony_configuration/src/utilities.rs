use std::{
    collections::HashMap,
    env::current_exe,
    path::{Path, PathBuf},
};

use camino::{Utf8Path, Utf8PathBuf};
use miette::{miette, Result};
use thiserror::Error;

use crate::ConfigurationError;


/// A const function returning the same `u16` as its const generic `V`.
///
/// Used for specifying default serde fields, see <https://github.com/serde-rs/serde/issues/368>
/// for discussion and reasoning.
pub const fn default_u16<const V: u16>() -> u16 {
    V
}


/// Returns the default configuration filepath.
///
/// This is at `./data/configuration.toml` relative to the `euphony` binary.
pub fn get_default_configuration_file_path(
) -> Result<PathBuf, ConfigurationError> {
    let configuration_file_path = current_exe()
        .map_err(|io_error| ConfigurationError::OtherError {
            error: miette!("{io_error:?}")
                .wrap_err("Could not get path to current executable."),
        })?
        .parent()
        .ok_or_else(|| ConfigurationError::OtherError {
            error: miette!(
                "Current executable path does not have a parent directory."
            ),
        })?
        .join("data/configuration.toml");

    Ok(configuration_file_path)
}



#[inline]
pub fn replace_placeholders_in_str(
    string: &str,
    placeholders: &HashMap<&'static str, String>,
) -> String {
    let mut replaced_string = string.to_string();

    for (key, value) in placeholders {
        replaced_string = replaced_string.replace(key, value);
    }

    replaced_string
}

#[derive(Debug, Error)]
#[error("provided path is not valid UTF-8")]
pub struct NotUtf8Error;

#[must_use = "function returns the modified path"]
#[allow(dead_code)]
pub fn replace_placeholders_in_path(
    original_path: &Path,
    placeholders: &HashMap<&'static str, String>,
) -> Result<PathBuf, NotUtf8Error> {
    let Some(path_str) = original_path.to_str() else {
        return Err(NotUtf8Error);
    };

    let replaced_path_string =
        replace_placeholders_in_str(path_str, placeholders);

    Ok(PathBuf::from(replaced_path_string))
}


#[must_use = "function returns the modified path"]
pub fn replace_placeholders_in_utf8_path(
    original_path: &Utf8Path,
    placeholders: &HashMap<&'static str, String>,
) -> Utf8PathBuf {
    let path_string = original_path.as_str();

    let replaced_path_string =
        replace_placeholders_in_str(path_string, placeholders);

    Utf8PathBuf::from(replaced_path_string)
}
