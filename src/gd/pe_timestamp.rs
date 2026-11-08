pub const PE_HEADER_BYTES: usize = 4096;

pub fn pe_timestamp(bytes: &[u8]) -> Option<u32> {
    if bytes.get(..2)? != b"MZ" {
        return None;
    }
    let header = usize::try_from(read_u32(bytes, 0x3c)?).ok()?;
    if bytes.get(header..header + 4)? != b"PE\0\0" {
        return None;
    }
    read_u32(bytes, header + 8)
}

fn read_u32(bytes: &[u8], offset: usize) -> Option<u32> {
    let field = bytes.get(offset..offset + 4)?;
    Some(u32::from_le_bytes(field.try_into().ok()?))
}
