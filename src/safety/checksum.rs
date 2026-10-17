use sha2::{Digest, Sha256};
use std::path::Path;

pub fn checksum_file(path: &Path) -> Result<String, String> {
    let data = std::fs::read(path).map_err(|e| format!("Read error: {}", e))?;
    let mut h = Sha256::new();
    h.update(&data);
    Ok(format!("{:x}", h.finalize()))
}

pub fn checksum_partition(partition: &str) -> Result<String, String> {
    let path = format!("/dev/block/by-name/{}", partition);
    checksum_file(Path::new(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn sha256_matches_known_vector() {
        let mut path = std::env::temp_dir();
        path.push(format!("androkit-checksum-test-{}.bin", std::process::id()));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(b"abc").unwrap();
        drop(f);
        let sum = checksum_file(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(
            sum,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn missing_file_errors() {
        assert!(checksum_file(std::path::Path::new("/no/such/file/xyz")).is_err());
    }
}
