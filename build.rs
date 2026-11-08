use std::env;
use std::fs;
use std::path::PathBuf;

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

const GROUP_ID: u16 = 1;

const LANGUAGE: u16 = 0x0409;

const MEMORY_FLAGS: u16 = 0x1030;

fn main() {
    let icon = PathBuf::from("assets/icons/glint.ico");
    println!("cargo:rerun-if-changed={}", icon.display());
    let is_windows = env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows");
    let is_msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    if !is_windows {
        return;
    }
    if !is_msvc {
        println!("cargo:warning=no exe icon on the gnu toolchain");
        return;
    }
    let ico = fs::read(&icon).expect("missing glint.ico");
    let res = res_from_ico(&ico).expect("bad glint.ico");
    let out = PathBuf::from(env::var("OUT_DIR").expect("cargo sets OUT_DIR")).join("glint.res");
    fs::write(&out, res).expect("OUT_DIR is writable");
    println!("cargo:rustc-link-arg-bins={}", out.display());
}

fn res_from_ico(ico: &[u8]) -> Option<Vec<u8>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(ico.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(ico.get(at..at + 4)?.try_into().ok()?));
    let count = u16_at(4)?;
    // empty first entry marks the .res as 32-bit
    let mut res = Vec::new();
    push_entry(&mut res, 0, 0, &[]);
    let mut group = Vec::new();
    group.extend(0u16.to_le_bytes());
    group.extend(1u16.to_le_bytes());
    group.extend(count.to_le_bytes());
    for index in 0..count {
        let entry = 6 + 16 * usize::from(index);
        let size = u32_at(entry + 8)? as usize;
        let offset = u32_at(entry + 12)? as usize;
        let image = ico.get(offset..offset + size)?;
        let id = index + 1;
        push_entry(&mut res, RT_ICON, id, image);
        group.extend(ico.get(entry..entry + 12)?);
        group.extend(id.to_le_bytes());
    }
    push_entry(&mut res, RT_GROUP_ICON, GROUP_ID, &group);
    Some(res)
}

fn push_entry(res: &mut Vec<u8>, kind: u16, id: u16, data: &[u8]) {
    let header_size: u32 = 32;
    let (flags, language) = if kind == 0 {
        (0, 0)
    } else {
        (MEMORY_FLAGS, LANGUAGE)
    };
    res.extend((data.len() as u32).to_le_bytes());
    res.extend(header_size.to_le_bytes());
    for name in [kind, id] {
        res.extend(0xFFFFu16.to_le_bytes());
        res.extend(name.to_le_bytes());
    }
    res.extend(0u32.to_le_bytes());
    res.extend(flags.to_le_bytes());
    res.extend(language.to_le_bytes());
    res.extend(0u32.to_le_bytes());
    res.extend(0u32.to_le_bytes());
    res.extend(data);
    while !res.len().is_multiple_of(4) {
        res.push(0);
    }
}
