use crate::gd::{GdPlatform, GdVersion, ManifestId};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct CatalogEntry {
    pub version: GdVersion,
    pub platform: GdPlatform,
    pub manifest: ManifestId,
}

impl CatalogEntry {
    pub fn new(version: GdVersion, platform: GdPlatform, manifest: ManifestId) -> Self {
        Self {
            version,
            platform,
            manifest,
        }
    }

    pub fn depot_id(&self) -> u32 {
        self.platform.depot_id()
    }
}
