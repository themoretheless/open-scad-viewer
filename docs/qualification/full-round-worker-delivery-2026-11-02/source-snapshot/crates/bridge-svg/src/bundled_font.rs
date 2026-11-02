//! The existing font, byte for byte. WASM stores a compressed constant; native
//! builds retain their original zero-decode path. No glyph/table is removed.
#[cfg(any(target_arch = "wasm32", test))]
const LENGTH: usize = 569_208;
#[cfg(any(target_arch = "wasm32", test))]
fn decode() -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let compressed = include_bytes!("../assets/NotoSans-Regular.ttf.zlib");
    let mut decoded = Vec::with_capacity(LENGTH);
    flate2::read::ZlibDecoder::new(compressed.as_slice())
        .take((LENGTH + 1) as u64)
        .read_to_end(&mut decoded)?;
    if decoded.len() != LENGTH {
        return Err(std::io::Error::other("Invalid bundled font length"));
    }
    Ok(decoded)
}
pub fn bytes() -> &'static [u8] {
    #[cfg(target_arch = "wasm32")]
    {
        static FONT: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
        FONT.get_or_init(|| decode().expect("Invalid compiled-in font"))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        include_bytes!("../assets/NotoSans-Regular.ttf")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compressed_font_restores_every_original_byte() {
        assert_eq!(
            decode().unwrap(),
            include_bytes!("../assets/NotoSans-Regular.ttf")
        );
    }
}
