use serde::Deserialize;

use crate::traits::Resolve;



#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedUiConfiguration {
    transcoding: UnresolvedTranscodingUiConfiguration,
}

#[derive(Clone)]
pub struct UiConfiguration {
    pub transcoding: TranscodingUiConfiguration,
}


impl Resolve for UnresolvedUiConfiguration {
    type Resolved = UiConfiguration;

    fn resolve(self) -> Self::Resolved {
        UiConfiguration {
            transcoding: self.transcoding.resolve(),
        }
    }
}



#[derive(Deserialize, Clone)]
pub(crate) struct UnresolvedTranscodingUiConfiguration {
    show_logs_tab_on_exit: bool,
}

#[derive(Clone)]
pub struct TranscodingUiConfiguration {
    pub show_logs_tab_on_exit: bool,
}


impl Resolve for UnresolvedTranscodingUiConfiguration {
    type Resolved = TranscodingUiConfiguration;

    fn resolve(self) -> Self::Resolved {
        TranscodingUiConfiguration {
            show_logs_tab_on_exit: self.show_logs_tab_on_exit,
        }
    }
}
