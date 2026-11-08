use crate::gd::GdVersion;

// info.plist says 2.208 for both 2.208 and 2.2081
const BUILDS: &[(&str, &str)] = &[
    ("7D7A5372-349C-38A6-B108-032666E979F2", "1.900"),
    ("DB5CADC0-E533-3123-8A63-5A434FE391ED", "2.2074"),
    ("27044C8B-76BD-303C-A035-5314AF1D9E6E", "2.2074"),
    ("3FBF9F72-5A83-3918-A82A-37BD3C019D7A", "2.2081"),
    ("ABCA9EC8-DDEE-317A-BC32-3F41A842311D", "2.2081"),
    ("06795129-83C4-37DD-9A90-F41302886C58", "2.2091"),
    ("2906A39D-2B5E-393C-ABD8-3C08F2547455", "2.2091"),
];

pub fn version_for_uuid(uuid: [u8; 16]) -> Option<GdVersion> {
    let text = uuid_text(uuid);
    BUILDS
        .iter()
        .find(|(build, _)| *build == text)
        .map(|(_, version)| version.parse().expect("the build table's versions parse"))
}

pub fn uuid_text(uuid: [u8; 16]) -> String {
    let hex: String = uuid.iter().map(|byte| format!("{byte:02X}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}
