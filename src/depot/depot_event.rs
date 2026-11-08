use crate::gd::ManifestId;

#[derive(Clone, PartialEq)]
pub enum DepotEvent {
    PasswordRequested,
    ApproveInMobileApp,
    SignedIn,
    QrCodeFollows,
    QrCodeRow(Vec<bool>),
    SignedInAs(String),
    ManifestFound { manifest: ManifestId, date: String },
    Progress { percent: f32, file: String },
    Finished { bytes: u64, uncompressed_bytes: u64 },
    Retrying(String),
    Failed(String),
    Other(String),
}
