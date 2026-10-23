//! Browser shell for the language frontends: exports the linear-memory ABI as an import-free wasm32
//! module. All logic lives in `languages-bridge`; this crate only adds the `extern "C"` surface the
//! TypeScript host (`services/languages`) uses.
#![feature(try_blocks, yeet_expr)]
#![allow(unused_features)]

#[unsafe(no_mangle)]
pub extern "C" fn abi_alloc(len: usize) -> usize {
    languages_bridge::abi::abi_alloc(len)
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_free(ptr: usize, len: usize) {
    unsafe { languages_bridge::abi::abi_free(ptr, len) }
}

/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn abi_request(op: u32, ptr: usize, len: usize) -> u64 {
    unsafe { languages_bridge::abi::abi_request(op, ptr, len) }
}
