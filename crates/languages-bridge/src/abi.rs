//! Linear-memory ABI for the language frontends.
//!
//! Mirrors the geometry ABI so the host reuses its transport: the caller owns request buffers and frees
//! every response. Responses are packed as `len << 32 | ptr`, sound because wasm32 pointers fit 32 bits.
use value_codec::{Value, json};

const LIMIT: usize = 32 * 1024 * 1024;

pub fn abi_alloc(len: usize) -> usize {
    if len > LIMIT {
        return 0;
    }
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8 as usize
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
pub unsafe fn abi_free(ptr: usize, len: usize) {
    if ptr == 0 {
        return;
    }
    drop(unsafe { Vec::from_raw_parts(ptr as *mut u8, len, len) });
}

fn packed(value: Value) -> u64 {
    let bytes = value_codec::encode_binary(&value).unwrap_or_else(|_| {
        value_codec::encode_binary(
            &json!({"ok":false,"error":{"code":"LANGUAGE_TRANSPORT","message":"Response exceeds transport limit"}}),
        )
        .unwrap()
    });
    let len = bytes.len();
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8 as usize;
    ((len as u64) << 32) | ptr as u64
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
pub unsafe fn abi_request(op: u32, ptr: usize, len: usize) -> u64 {
    if len > LIMIT {
        return packed(
            json!({"ok":false,"error":{"code":"LANGUAGE_TRANSPORT","message":"Request exceeds transport limit"}}),
        );
    }
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    let value = match value_codec::decode_binary(bytes) {
        Ok(v) => v,
        Err(e) => {
            return packed(
                json!({"ok":false,"error":{"code":"LANGUAGE_TRANSPORT","message":e.to_string()}}),
            );
        }
    };
    packed(crate::abi_language(op, value))
}
