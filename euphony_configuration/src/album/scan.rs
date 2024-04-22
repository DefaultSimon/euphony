use serde::Deserialize;

use crate::traits::Resolve;
use crate::utilities::default_u16;


/// Default album file scan depth.
pub const DEFAULT_SCAN_DEPTH: u16 = 0;



#[derive(Deserialize, Clone, Debug)]
pub(crate) struct UnresolvedAlbumScanConfiguration {
    /// Maximum album scanning depth. Zero (the default) means no subdirectories are scanned.
    #[serde(default = "default_u16::<DEFAULT_SCAN_DEPTH>")]
    depth: u16,
}

impl Default for UnresolvedAlbumScanConfiguration {
    fn default() -> Self {
        Self {
            depth: DEFAULT_SCAN_DEPTH,
        }
    }
}

impl Resolve for UnresolvedAlbumScanConfiguration {
    type Resolved = AlbumScanConfiguration;

    fn resolve(self) -> Self::Resolved {
        Self::Resolved { depth: self.depth }
    }
}



#[derive(Clone, PartialEq, Eq, Debug)]
pub struct AlbumScanConfiguration {
    /// Maximum album scanning depth. Zero (the default) means no subdirectories are scanned.
    pub depth: u16,
}

impl Default for AlbumScanConfiguration {
    fn default() -> Self {
        Self {
            depth: DEFAULT_SCAN_DEPTH,
        }
    }
}
