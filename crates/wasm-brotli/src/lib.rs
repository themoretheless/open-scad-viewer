//! Synchronous compression bootstrap. This crate contains no geometry code.
//! The host creates a fresh instance, copies the result, then drops the instance.
use brotli_decompressor::{BrotliDecompressStream, BrotliResult, BrotliState, StandardAlloc};
use std::sync::Mutex;

const INPUT_LIMIT: usize = 4 * 1024 * 1024;
const OUTPUT_LIMIT: usize = 16 * 1024 * 1024;
const ENCODED_LIMIT: usize = 5 + (INPUT_LIMIT + 4).div_ceil(4) * 5;
const ALPHABET: &[u8; 85] =
    b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#";
const fn ranks() -> [u8; 128] {
    let mut table = [255; 128];
    let mut i = 0;
    while i < 85 {
        table[ALPHABET[i] as usize] = i as u8;
        i += 1
    }
    table
}
const RANKS: [u8; 128] = ranks();
static INPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());
static OUTPUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Reserve the sole input buffer. No caller-provided pointer is dereferenced.
#[unsafe(no_mangle)]
pub extern "C" fn prepare(size: usize) -> usize {
    if size == 0 || size > INPUT_LIMIT {
        return 0;
    }
    let mut input = INPUT.lock().unwrap();
    input.resize(size, 0);
    input.as_mut_ptr() as usize
}

/// Reserve bounded ASCII input; base85 decoding stays in the Rust bootstrap.
#[unsafe(no_mangle)]
pub extern "C" fn prepare_encoded(size: usize) -> usize {
    if !(5..=ENCODED_LIMIT).contains(&size) {
        return 0;
    }
    let mut input = INPUT.lock().unwrap();
    input.resize(size, 0);
    input.as_mut_ptr() as usize
}
fn word(bytes: &[u8]) -> Option<u32> {
    let mut value = 0u32;
    for &byte in bytes {
        let rank = *RANKS.get(byte as usize)?;
        if rank == 255 {
            return None;
        }
        value = value.checked_mul(85)?.checked_add(rank as u32)?
    }
    Some(value)
}
fn decode_base85(input: &[u8]) -> Option<Vec<u8>> {
    if input.len() < 5 || input.len() > ENCODED_LIMIT {
        return None;
    }
    let size = word(&input[..5])? as usize;
    if size <= 4 || size > INPUT_LIMIT + 4 || input.len() != 5 + size.div_ceil(4) * 5 {
        return None;
    }
    let mut output = Vec::with_capacity(size);
    let full_end = 5 + (size / 4) * 5;
    for group in input[5..full_end].as_chunks::<5>().0 {
        output.extend_from_slice(&word(group)?.to_be_bytes());
    }
    let tail = size % 4;
    if tail != 0 {
        let bytes = word(&input[full_end..])?.to_be_bytes();
        if bytes[tail..].iter().any(|&byte| byte != 0) {
            return None;
        }
        output.extend_from_slice(&bytes[..tail]);
    }
    Some(output)
}

/// Decode the length-prefixed base85 package and its complete Brotli stream.
#[unsafe(no_mangle)]
pub extern "C" fn decode_encoded() -> u64 {
    let input = INPUT.lock().unwrap();
    let Some(package) = decode_base85(&input) else {
        return 0;
    };
    let size = u32::from_le_bytes(package[..4].try_into().unwrap()) as usize;
    if size > OUTPUT_LIMIT {
        return 0;
    }
    let mut output = OUTPUT.lock().unwrap();
    output.resize(size, 0);
    if !decompress(&package[4..], &mut output) {
        return 0;
    }
    ((size as u64) << 32) | output.as_ptr() as usize as u64
}

fn decompress(input: &[u8], output: &mut [u8]) -> bool {
    let mut state = BrotliState::new(
        StandardAlloc::default(),
        StandardAlloc::default(),
        StandardAlloc::default(),
    );
    let mut available_in = input.len();
    let mut input_offset = 0;
    let mut available_out = output.len();
    let mut output_offset = 0;
    let mut total_out = 0;
    let result = BrotliDecompressStream(
        &mut available_in,
        &mut input_offset,
        input,
        &mut available_out,
        &mut output_offset,
        output,
        &mut total_out,
        &mut state,
    );
    matches!(result, BrotliResult::ResultSuccess)
        && available_in == 0
        && output_offset == output.len()
}

/// Return (length << 32 | pointer), or zero on invalid/truncated/trailing input.
/// A fixed output slice prevents expansion beyond the declared package size.
#[unsafe(no_mangle)]
pub extern "C" fn decode(expected_size: usize) -> u64 {
    if expected_size > OUTPUT_LIMIT {
        return 0;
    }
    let input = INPUT.lock().unwrap();
    if input.is_empty() {
        return 0;
    }
    let mut output = OUTPUT.lock().unwrap();
    output.resize(expected_size, 0);
    if !decompress(&input, &mut output) {
        return 0;
    }
    ((expected_size as u64) << 32) | output.as_ptr() as usize as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_lengths_before_allocating() {
        assert_eq!(prepare(0), 0);
        assert_eq!(prepare(INPUT_LIMIT + 1), 0);
        assert_eq!(decode(OUTPUT_LIMIT + 1), 0);
    }

    #[test]
    fn requires_complete_stream_and_exact_output() {
        // RFC 7932 empty stream, produced by the reference Node encoder.
        assert!(decompress(&[0x3b], &mut []));
        assert!(!decompress(&[], &mut []));
        assert!(!decompress(&[0x3b, 0], &mut []));
        assert!(!decompress(&[0x3b], &mut [0]));
        assert!(!decompress(&[0xff, 0xff, 0xff], &mut []));
    }
    #[test]
    fn base85_checks_words_lengths_and_padding() {
        assert_eq!(word(b"00000"), Some(0));
        assert_eq!(word(b"%nSc0"), Some(u32::MAX));
        assert!(word(b"#####").is_none());
        assert!(word(b"000~0").is_none());
        // Five zero bytes: size=5, two zero-padded words.
        assert_eq!(decode_base85(b"000050000000000"), Some(vec![0; 5]));
        assert!(decode_base85(b"000050000000001").is_none());
        assert!(decode_base85(b"0000500000").is_none());
        assert_eq!(prepare_encoded(ENCODED_LIMIT + 1), 0);
    }

    #[test]
    fn base85_full_words_and_every_tail_length_roundtrip() {
        fn encode_word(mut value: u32, output: &mut Vec<u8>) {
            let mut digits = [0; 5];
            for digit in digits.iter_mut().rev() {
                *digit = ALPHABET[(value % 85) as usize];
                value /= 85;
            }
            output.extend_from_slice(&digits);
        }
        for size in 5..=512 {
            let bytes: Vec<_> = (0..size).map(|i| ((i * 37 + size) % 256) as u8).collect();
            let mut encoded = Vec::new();
            encode_word(size as u32, &mut encoded);
            for chunk in bytes.chunks(4) {
                let mut block = [0; 4];
                block[..chunk.len()].copy_from_slice(chunk);
                encode_word(u32::from_be_bytes(block), &mut encoded);
            }
            assert_eq!(decode_base85(&encoded), Some(bytes));
            if size % 4 != 0 {
                let last = encoded.len() - 5;
                let invalid = word(&encoded[last..]).unwrap() | 1;
                encoded.truncate(last);
                encode_word(invalid, &mut encoded);
                assert!(decode_base85(&encoded).is_none());
            }
        }
    }
}
