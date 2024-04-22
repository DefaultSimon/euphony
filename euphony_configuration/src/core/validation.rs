use serde::Deserialize;

use crate::traits::Resolve;



#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedValidationConfiguration {
    extensions_considered_audio_files: Vec<String>,
}



#[derive(Clone)]
pub struct ValidationConfiguration {
    pub extensions_considered_audio_files: Vec<String>,
}


impl Resolve for UnresolvedValidationConfiguration {
    type Resolved = ValidationConfiguration;

    fn resolve(self) -> Self::Resolved {
        let extensions_considered_audio_files = self
            .extensions_considered_audio_files
            .into_iter()
            .map(|mut extension| {
                extension.make_ascii_lowercase();
                extension
            })
            .collect();

        ValidationConfiguration {
            extensions_considered_audio_files,
        }
    }
}
