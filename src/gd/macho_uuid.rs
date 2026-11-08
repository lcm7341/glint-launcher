pub const MACHO_HEAD_BYTES: usize = 64 * 1024;

const FAT_MAGIC: u32 = 0xcafe_babe;
const FAT_MAGIC_64: u32 = 0xcafe_babf;
const MACHO_MAGIC_64: u32 = 0xfeed_facf;
const LOAD_COMMAND_UUID: u32 = 0x1b;

pub fn macho_slices(head: &[u8]) -> Option<Vec<u64>> {
    match read_u32_be(head, 0)? {
        FAT_MAGIC => slice_offsets(head, 20, |entry| read_u32_be(entry, 8).map(u64::from)),
        FAT_MAGIC_64 => slice_offsets(head, 32, |entry| read_u64_be(entry, 8)),
        _ if read_u32_le(head, 0)? == MACHO_MAGIC_64 => Some(vec![0]),
        _ => None,
    }
}

pub fn macho_uuid(head: &[u8]) -> Option<[u8; 16]> {
    if read_u32_le(head, 0)? != MACHO_MAGIC_64 {
        return None;
    }
    let command_count = read_u32_le(head, 16)?;
    let mut offset = 32;
    for _ in 0..command_count {
        let command = read_u32_le(head, offset)?;
        let size = usize::try_from(read_u32_le(head, offset + 4)?).ok()?;
        if command == LOAD_COMMAND_UUID {
            return head.get(offset + 8..offset + 24)?.try_into().ok();
        }
        if size == 0 {
            return None;
        }
        offset += size;
    }
    None
}

fn slice_offsets(
    head: &[u8],
    entry_bytes: usize,
    offset_of: impl Fn(&[u8]) -> Option<u64>,
) -> Option<Vec<u64>> {
    let count = usize::try_from(read_u32_be(head, 4)?).ok()?;
    (0..count)
        .map(|index| {
            let start = 8 + index * entry_bytes;
            offset_of(head.get(start..start + entry_bytes)?)
        })
        .collect()
}

fn read_u32_be(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

fn read_u64_be(bytes: &[u8], offset: usize) -> Option<u64> {
    Some(u64::from_be_bytes(
        bytes.get(offset..offset + 8)?.try_into().ok()?,
    ))
}
