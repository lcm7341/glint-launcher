use std::fs::{self, File};
use std::io::{self, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use sha2::{Digest, Sha256};

// winehq 9 steam window bug (0x3008)
pub const WINE_VERSION: &str = "11.21";

const WINE_URL: &str = "https://github.com/Gcenx/macOS_Wine_builds/releases/download/11.21/wine-devel-11.21-osx64.tar.xz";

const WINE_SHA256: &str = "9909b486657e10ab66c7279371ae1f990345b596b3b1b3a0cf21f28af24fa0ce";

const WINE_DIR: &str = "wine-11.21";

const WINE_PROGRAM: &str = "Wine Devel.app/Contents/Resources/wine/bin/wine";

pub const INSTALL_ROSETTA: &str = "sudo softwareupdate --install-rosetta --agree-to-license";

pub fn needs_rosetta() -> bool {
    cfg!(all(target_os = "macos", target_arch = "aarch64"))
        && !Path::new("/Library/Apple/usr/share/rosetta/rosetta").exists()
}

pub fn glint_wine(tools_dir: &Path) -> Option<PathBuf> {
    let program = tools_dir.join(WINE_DIR).join(WINE_PROGRAM);
    program.is_file().then_some(program)
}

pub(crate) fn fetch_wine(tools_dir: &Path) -> Result<PathBuf, String> {
    fs::create_dir_all(tools_dir).map_err(|error| error.to_string())?;
    let archive = tools_dir.join(format!("{WINE_DIR}.tar.xz.part"));
    let downloaded = download_checked(&archive);
    if let Err(error) = downloaded {
        let _ = fs::remove_file(&archive);
        return Err(error);
    }
    let unpacking = tools_dir.join(format!("{WINE_DIR}.part"));
    let _ = fs::remove_dir_all(&unpacking);
    let unpacked = unpack(&archive, &unpacking);
    let _ = fs::remove_file(&archive);
    unpacked?;
    let wine_dir = tools_dir.join(WINE_DIR);
    let _ = fs::remove_dir_all(&wine_dir);
    fs::rename(&unpacking, &wine_dir).map_err(|error| error.to_string())?;
    glint_wine(tools_dir).ok_or_else(|| format!("the download has no {WINE_PROGRAM}"))
}

fn download_checked(path: &Path) -> Result<(), String> {
    let agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30 * 60)))
        .build()
        .new_agent();
    let mut response = agent
        .get(WINE_URL)
        .header("User-Agent", "glint")
        .call()
        .map_err(|error| error.to_string())?;
    let mut body = response
        .body_mut()
        .with_config()
        .limit(512 * 1024 * 1024)
        .reader();
    let mut file = BufWriter::new(File::create(path).map_err(|error| error.to_string())?);
    let mut hasher = Sha256::new();
    let mut buffer = vec![0; 1 << 16];
    loop {
        let read = match body.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.to_string()),
        };
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
    }
    file.flush().map_err(|error| error.to_string())?;
    let hash = to_hex(&hasher.finalize());
    if hash != WINE_SHA256 {
        return Err(format!("checksum mismatch ({hash})"));
    }
    Ok(())
}

fn unpack(archive: &Path, into: &Path) -> Result<(), String> {
    fs::create_dir_all(into).map_err(|error| error.to_string())?;
    let status = Command::new("/usr/bin/tar")
        .arg("-xJf")
        .arg(archive)
        .arg("-C")
        .arg(into)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(())
    } else {
        let _ = fs::remove_dir_all(into);
        Err(format!("unpacking it stopped with {status}"))
    }
}

fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
