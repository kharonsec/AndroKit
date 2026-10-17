pub struct DtboInfo {
    pub magic: u32,
    pub total_size: u32,
    pub header_size: u32,
    pub dt_entry_size: u32,
    pub dt_entry_count: u32,
}

pub fn parse_dtbo_header(data: &[u8]) -> Result<DtboInfo, String> {
    if data.len() < 28 {
        return Err("File too small for DTBO header".into());
    }
    let get_u32 =
        |off: usize| u32::from_be_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
    let magic = get_u32(0);
    if magic != 0xd7b7ab1e {
        return Err(format!("Invalid DTBO magic: 0x{:08x}", magic));
    }
    Ok(DtboInfo {
        magic,
        total_size: get_u32(4),
        header_size: get_u32(8),
        dt_entry_size: get_u32(12),
        dt_entry_count: get_u32(20),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_dtbo() -> Vec<u8> {
        let mut data = vec![0u8; 32];
        data[0..4].copy_from_slice(&0xd7b7ab1eu32.to_be_bytes()); // magic
        data[4..8].copy_from_slice(&512u32.to_be_bytes()); // total_size
        data[8..12].copy_from_slice(&32u32.to_be_bytes()); // header_size
        data[12..16].copy_from_slice(&24u32.to_be_bytes()); // dt_entry_size
        data[20..24].copy_from_slice(&3u32.to_be_bytes()); // dt_entry_count
        data
    }

    #[test]
    fn parses_valid_dtbo() {
        let info = parse_dtbo_header(&sample_dtbo()).expect("should parse");
        assert_eq!(info.magic, 0xd7b7ab1e);
        assert_eq!(info.total_size, 512);
        assert_eq!(info.header_size, 32);
        assert_eq!(info.dt_entry_size, 24);
        assert_eq!(info.dt_entry_count, 3);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = sample_dtbo();
        data[0..4].copy_from_slice(&0x12345678u32.to_be_bytes());
        assert!(parse_dtbo_header(&data).is_err());
    }

    #[test]
    fn rejects_too_small() {
        assert!(parse_dtbo_header(&[0u8; 10]).is_err());
    }
}
