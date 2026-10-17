pub struct BootImageInfo {
    pub kernel_size: u32,
    pub kernel_addr: u32,
    pub ramdisk_size: u32,
    pub ramdisk_addr: u32,
    pub second_size: u32,
    pub second_addr: u32,
    pub tags_addr: u32,
    pub page_size: u32,
    pub cmdline: String,
    pub has_dtb: bool,
}

pub fn parse_boot_header(data: &[u8]) -> Result<BootImageInfo, String> {
    if data.len() < 2048 {
        return Err("File too small for boot image header".into());
    }
    if &data[0..8] != b"ANDROID!" {
        return Err("Not a valid Android boot image (missing ANDROID! magic)".into());
    }
    let get_u32 =
        |off: usize| u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]]);
    let cmdline = std::str::from_utf8(&data[64..576])
        .unwrap_or("")
        .trim_end_matches('\0')
        .to_string();
    Ok(BootImageInfo {
        kernel_size: get_u32(8),
        kernel_addr: get_u32(12),
        ramdisk_size: get_u32(16),
        ramdisk_addr: get_u32(20),
        second_size: get_u32(24),
        second_addr: get_u32(28),
        tags_addr: get_u32(32),
        page_size: get_u32(36),
        cmdline,
        has_dtb: get_u32(40) > 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_boot() -> Vec<u8> {
        let mut data = vec![0u8; 2048];
        data[0..8].copy_from_slice(b"ANDROID!");
        data[8..12].copy_from_slice(&1024u32.to_le_bytes()); // kernel_size
        data[12..16].copy_from_slice(&0x8000u32.to_le_bytes()); // kernel_addr
        data[16..20].copy_from_slice(&2048u32.to_le_bytes()); // ramdisk_size
        data[36..40].copy_from_slice(&2048u32.to_le_bytes()); // page_size
        data[40..44].copy_from_slice(&1u32.to_le_bytes()); // dt size -> has_dtb
        let cmdline = b"console=ttyMSM0";
        data[64..64 + cmdline.len()].copy_from_slice(cmdline);
        data
    }

    #[test]
    fn parses_valid_header() {
        let info = parse_boot_header(&sample_boot()).expect("should parse");
        assert_eq!(info.kernel_size, 1024);
        assert_eq!(info.kernel_addr, 0x8000);
        assert_eq!(info.ramdisk_size, 2048);
        assert_eq!(info.page_size, 2048);
        assert!(info.has_dtb);
        assert_eq!(info.cmdline, "console=ttyMSM0");
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = sample_boot();
        data[0] = b'X';
        assert!(parse_boot_header(&data).is_err());
    }

    #[test]
    fn rejects_too_small() {
        assert!(parse_boot_header(&[0u8; 100]).is_err());
    }
}
