//! Import-free WASM host using the repository's binary Value codec.
mod response;
mod session;
use value_codec::{json, Deserialize, Map, Value};
type Result<T> = std::result::Result<T, String>;
fn input(message: impl Into<String>) -> String {
    message.into()
}
fn field<T: for<'a> Deserialize<'a>>(v: &Value, key: &str) -> Result<T> {
    value_codec::from_value(v[key].clone()).map_err(|e| e.to_string())
}
// Keep these equal to the existing MGV1 decoders (Rust and valueBinaryCodec.ts).
// The photo adapter checks all three before encoding; the shared codec is unchanged.
const LIMIT: usize = 32 * 1024 * 1024;
const ITEM_LIMIT: usize = 4_000_000;
const DEPTH_LIMIT: usize = 128;
const TRANSPORT_ERROR: &str =
    "Photo result exceeds transport limits; use fewer photos or lower depth resolution";

struct ResponseBudget {
    bytes: usize,
    items: usize,
}
impl ResponseBudget {
    fn reserve(&mut self, depth: usize, bytes: usize) -> Result<()> {
        if depth > DEPTH_LIMIT
            || self.items >= ITEM_LIMIT
            || bytes > LIMIT.saturating_sub(self.bytes)
        {
            return Err(input(TRANSPORT_ERROR));
        }
        self.items += 1;
        self.bytes += bytes;
        Ok(())
    }

    fn visit(&mut self, value: &Value, depth: usize) -> Result<()> {
        let shallow_bytes = match value {
            Value::Null | Value::Bool(_) => 1,
            Value::Number(_) => 9,
            Value::String(text) => 5usize.checked_add(text.len()).ok_or(TRANSPORT_ERROR)?,
            Value::Array(_) | Value::Object(_) => 5,
        };
        self.reserve(depth, shallow_bytes)?;
        match value {
            Value::Array(values) => {
                for value in values {
                    self.visit(value, depth + 1)?;
                }
            }
            Value::Object(fields) => {
                for (key, value) in fields {
                    // Object keys are full string values in MGV1, including their item/depth budget.
                    self.reserve(
                        depth + 1,
                        5usize.checked_add(key.len()).ok_or(TRANSPORT_ERROR)?,
                    )?;
                    self.visit(value, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn response_value(result: Result<Value>) -> Value {
    // Consume the serialized payload: json! would clone its entire Value tree.
    let mut fields = Map::new();
    match result {
        Ok(value) => {
            fields.insert("ok".into(), Value::Bool(true));
            fields.insert("value".into(), value);
        }
        Err(message) => {
            fields.insert("ok".into(), Value::Bool(false));
            fields.insert("message".into(), Value::String(message));
        }
    }
    Value::Object(fields)
}

fn response_bytes(result: Result<Value>) -> Vec<u8> {
    let value = response_value(result);
    // Four bytes are reserved for the MGV1 header; visit includes the complete
    // response envelope and diagnostics rather than only the surface arrays.
    let mut budget = ResponseBudget { bytes: 4, items: 0 };
    match budget
        .visit(&value, 0)
        .and_then(|()| value_codec::encode_binary(&value).map_err(|e| e.to_string()))
    {
        Ok(bytes) => bytes,
        Err(_) => value_codec::encode_binary(&response_value(Err(input(TRANSPORT_ERROR))))
            .expect("The constant transport error fits the MGV1 limits"),
    }
}
#[no_mangle]
pub extern "C" fn photo_alloc(len: usize) -> usize {
    if len == 0 || len > LIMIT {
        return 0;
    }
    // The JS host overwrites all `len` bytes through a fresh Uint8Array view
    // before photo_add/photo_add_calibrated may read them, and u8 has no
    // invalid bit patterns, so the zero-fill would be dead work. No Rust code
    // reads the allocation before that overwrite (closed ABI of this module).
    let mut buffer: Vec<u8> = Vec::with_capacity(len);
    #[allow(clippy::uninit_vec)]
    unsafe {
        buffer.set_len(len);
    }
    Box::into_raw(buffer.into_boxed_slice()) as *mut u8 as usize
}
/// # Safety
/// ptr/len must be a live allocation returned by this module; consumed once.
#[no_mangle]
pub unsafe extern "C" fn photo_free(ptr: usize, len: usize) {
    if ptr != 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut u8,
            len,
        )));
    }
}
fn packed(result: Result<Value>) -> u64 {
    packed_bytes(Ok(response_bytes(result)))
}

thread_local! {
    static LAST_RESPONSE: std::cell::Cell<(usize, usize)> = std::cell::Cell::new((0, 0));
}
/// Native hosts fetch responses through these (the packed u64 return truncates
/// the pointer to 32 bits, which only wasm32 linear memory can promise).
#[no_mangle]
pub extern "C" fn photo_response_ptr() -> usize {
    LAST_RESPONSE.with(|last| last.get().0)
}
#[no_mangle]
pub extern "C" fn photo_response_len() -> usize {
    LAST_RESPONSE.with(|last| last.get().1)
}
fn packed_bytes(result: Result<Vec<u8>>) -> u64 {
    let bytes = result.unwrap_or_else(|message| response_bytes(Err(message)));
    let len = bytes.len();
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8 as usize;
    LAST_RESPONSE.with(|last| last.set((ptr, len)));
    // wasm32 pointers fit u32; native callers use photo_response_ptr/len.
    ((len as u64) << 32) | (ptr as u64 & 0xffff_ffff)
}
/// # Safety
/// ptr/len must reference a live caller-owned allocation returned by photo_alloc.
/// The buffer is consumed on every path: moved into the session image on success
/// and freed on failure, so the caller must not photo_free it afterwards.
#[no_mangle]
pub unsafe extern "C" fn photo_add(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
) -> u64 {
    if ptr == 0
        || len > 3 * 2048 * 2048
        || width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(len)
    {
        return packed(Err(input("Invalid photo buffer")));
    }
    let rgb = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len));
    packed(session::add(width, height, focal, rgb).map(|n| json!(n)))
}
/// Add measured calibration without altering the legacy photo_add ABI.
/// # Safety
/// ptr/len must reference a live caller-owned photo_alloc buffer and is consumed
/// like in photo_add; the calibration buffer stays caller-owned and is only read.
#[no_mangle]
pub unsafe extern "C" fn photo_add_calibrated(
    width: usize,
    height: usize,
    focal: f64,
    ptr: usize,
    len: usize,
    calibration_ptr: usize,
    calibration_len: usize,
) -> u64 {
    if ptr == 0
        || calibration_ptr == 0
        || calibration_len == 0
        || calibration_len > 16 * 1024
        || len > 3 * 2048 * 2048
        || width.checked_mul(height).and_then(|n| n.checked_mul(3)) != Some(len)
    {
        return packed(Err(input("Invalid calibrated photo buffer")));
    }
    let rgb = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len));
    packed(
        session::add_calibrated(
            width,
            height,
            focal,
            rgb,
            std::slice::from_raw_parts(calibration_ptr as *const u8, calibration_len),
        )
        .map(|n| json!(n)),
    )
}

/// Explicit bounded dense preset; photo_run(2, resolution) remains the legacy default.
#[no_mangle]
pub extern "C" fn photo_dense(resolution: usize, preset: u32) -> u64 {
    packed_bytes(session::dispatch_bytes(
        json!({"action": "dense", "resolution": resolution, "preset": preset}),
    ))
}

/// Selects the compute backend for subsequent runs: 0 = CPU (default),
/// 1 = GPU (native builds with the `gpu` feature; otherwise a recorded no-op
/// that keeps the CPU reference).
#[no_mangle]
pub extern "C" fn photo_set_acceleration(value: u32) -> u64 {
    packed_bytes(session::set_acceleration_host(value))
}

/// Stage 1 of the browser WebGPU dense sweep; the response value carries the
/// payload pointer/length and the WGSL shader text, or null when the request
/// is ineligible (caller then uses photo_dense).
#[no_mangle]
pub extern "C" fn photo_dense_prepare(resolution: usize, preset: u32) -> u64 {
    packed(session::dense_prepare_host(resolution, preset))
}

/// Stage 2: consumes the score buffer like photo_add consumes rgb.
/// # Safety
/// ptr/len must reference a live caller-owned photo_alloc buffer, which this
/// call takes over and frees on any outcome.
#[no_mangle]
pub unsafe extern "C" fn photo_dense_finish(ptr: usize, len: usize) -> u64 {
    // The score stream is raw f32, not MGV1; the session validates the exact
    // expected length. 128 MiB caps a 24-view run at the default resolution.
    if ptr == 0 || len == 0 || len % 4 != 0 || len > 128 * 1024 * 1024 {
        return packed(Err(input("Invalid host sweep score buffer")));
    }
    let bytes = Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len));
    let scores: Vec<f32> = bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    packed_bytes(session::dense_finish_host(scores))
}

/// Runs in a disposable Worker, so cancellation releases the whole session.
#[no_mangle]
pub extern "C" fn photo_run(action: u32, resolution: usize) -> u64 {
    let action = match action {
        0 => "clear",
        1 => "sparse",
        2 => "dense",
        3 => "compact",
        4 => "report",
        _ => return packed(Err(input("Unknown photo operation"))),
    };
    packed_bytes(session::dispatch_bytes(
        json!({"action":action,"resolution":resolution}),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn budget_counts_the_same_bytes_as_the_shared_encoder() {
        let value = response_value(Ok(json!({
            "positions": [[0., 1., 2.], [-1.5, 3., 4.]],
            "diagnostics": {"reason": "Связан", "value": null},
        })));
        let mut budget = ResponseBudget { bytes: 4, items: 0 };
        budget.visit(&value, 0).unwrap();
        assert_eq!(
            budget.bytes,
            value_codec::encode_binary(&value).unwrap().len()
        );
        assert!(budget.items > 0);
    }

    #[test]
    fn a_byte_small_but_item_oversized_response_becomes_a_readable_error() {
        // Four million null values need only about 4 MiB of binary bytes, but
        // their array and the response envelope exceed the decoder's item limit.
        let payload = Value::Array(vec![Value::Null; ITEM_LIMIT]);
        let bytes = response_bytes(Ok(payload));
        assert!(bytes.len() < 256);
        let decoded = value_codec::decode_binary(&bytes).unwrap();
        assert_eq!(decoded["ok"], Value::Bool(false));
        assert_eq!(decoded["message"], Value::String(TRANSPORT_ERROR.into()));
    }

    #[test]
    fn envelope_and_keys_take_part_in_depth_and_item_limits() {
        let value = response_value(Ok(Value::Null));
        let mut budget = ResponseBudget {
            bytes: 4,
            items: ITEM_LIMIT - 4,
        };
        assert!(budget.visit(&value, 0).is_err());
        let mut budget = ResponseBudget {
            bytes: LIMIT - 1,
            items: 0,
        };
        assert!(budget.visit(&value, 0).is_err());
        let mut nested = Value::Null;
        for _ in 0..DEPTH_LIMIT {
            nested = Value::Array(vec![nested]);
        }
        // Payload alone reaches the allowed depth, but the envelope adds one level.
        let decoded = value_codec::decode_binary(&response_bytes(Ok(nested))).unwrap();
        assert_eq!(decoded["ok"], Value::Bool(false));
    }
}
