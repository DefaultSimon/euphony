use serde::Deserialize;

use crate::traits::Resolve;


#[derive(Deserialize, Clone, Debug)]
pub(crate) struct UnresolvedLibraryValidationConfiguration {
    allowed_audio_file_extensions: Vec<String>,

    allowed_other_file_extensions: Vec<String>,

    allowed_other_files_by_name: Vec<String>,
}

impl Resolve for UnresolvedLibraryValidationConfiguration {
    type Resolved = LibraryValidationConfiguration;

    fn resolve(self) -> Self::Resolved {
        let allowed_audio_file_extensions = self
            .allowed_audio_file_extensions
            .into_iter()
            .map(|extension| extension.to_ascii_lowercase())
            .collect();

        let allowed_other_file_extensions = self
            .allowed_other_file_extensions
            .into_iter()
            .map(|extension| extension.to_ascii_lowercase())
            .collect();


        LibraryValidationConfiguration {
            allowed_audio_file_extensions,
            allowed_other_file_extensions,
            allowed_other_files_by_name: self.allowed_other_files_by_name,
        }
    }
}



#[derive(Clone, PartialEq, Eq, Debug)]
pub struct LibraryValidationConfiguration {
    /// A list of allowed audio extensions. Any not specified here are forbidden
    /// (flagged when running validation), see configuration template for more information.
    pub allowed_audio_file_extensions: Vec<String>,

    pub allowed_other_file_extensions: Vec<String>,

    pub allowed_other_files_by_name: Vec<String>,
}
