//! Bounded fallback for native snapshots that do not fit the deflate carrier.
use super::{Result, input};
use brotli::{BrotliDecompressStream, BrotliResult, BrotliState, CompressorReader, HeapAlloc};
use std::io::Read;

pub(super) fn encode(source: &[u8], max_bytes: usize) -> Result<Vec<u8>> {
    let mut reader = CompressorReader::new(source, 16384, 5, 22).take(max_bytes as u64 + 1);
    let mut bytes = Vec::new();
    reader
        .read_to_end(&mut bytes)
        .map_err(|e| input(e.to_string()))?;
    if bytes.len() > max_bytes {
        return Err(input("Native snapshot encoded size limit"));
    }
    Ok(bytes)
}

pub(super) fn decode(bytes: &[u8], output: &mut [u8], expected: usize) -> bool {
    let mut state = BrotliState::new_strict(
        HeapAlloc::default(),
        HeapAlloc::default(),
        HeapAlloc::default(),
    );
    let mut available = bytes.len();
    let mut input_offset = 0;
    let mut remaining = output.len();
    let mut output_offset = 0;
    let mut total = 0;
    let result = BrotliDecompressStream(
        &mut available,
        &mut input_offset,
        bytes,
        &mut remaining,
        &mut output_offset,
        output,
        &mut total,
        &mut state,
    );
    matches!(result, BrotliResult::ResultSuccess) && available == 0 && output_offset == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_only_the_complete_stream_and_exact_output_length() {
        let text = br#"{"geometry":[0.1,1e-38,5,6,7]}"#;
        let bytes = encode(text, 1024).unwrap();
        let mut decoded = vec![0; text.len() + 1];
        assert!(decode(&bytes, &mut decoded, text.len()));
        assert_eq!(&decoded[..text.len()], text);
        assert!(!decode(&bytes, &mut decoded, text.len() - 1));
        assert!(!decode(&bytes[..bytes.len() - 1], &mut decoded, text.len()));
        assert!(!decode(
            &[bytes, vec![0]].concat(),
            &mut decoded,
            text.len()
        ));
        assert!(encode(text, 1).is_err());
    }
}
