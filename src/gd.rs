mod build_timestamp;
mod catalog;
mod catalog_entry;
mod catalog_error;
mod mac_build;
mod macho_uuid;
mod manifest;
mod pe_timestamp;
mod platform;
mod steam_app;
mod version;

pub use build_timestamp::version_for_timestamp;
pub use catalog::Catalog;
pub use catalog_entry::CatalogEntry;
pub use catalog_error::CatalogError;
pub use mac_build::{uuid_text, version_for_uuid};
pub use macho_uuid::{MACHO_HEAD_BYTES, macho_slices, macho_uuid};
pub use manifest::ManifestId;
pub use pe_timestamp::{PE_HEADER_BYTES, pe_timestamp};
pub use platform::GdPlatform;
pub use steam_app::GD_APP_ID;
pub use version::GdVersion;

pub const WINDOWS_PROGRAM: &str = "GeometryDash.exe";

// drm wrapped: 1.9, 1.92, 2.113. not from 2.2074 on
pub(crate) fn is_encrypted_mac_build(version: &GdVersion) -> bool {
    *version < "2.2".parse().expect("a fixed version parses")
}
