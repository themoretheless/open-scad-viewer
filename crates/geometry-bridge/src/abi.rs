//! Linear-memory ABI core. The host owns request buffers and frees every response.
//! The `geometry-wasm` and `geometry-native` shells add the extern "C" surface.
use super::*;
use std::cell::RefCell;
thread_local! {static SAMPLERS:RefCell<Vec<Option<SurfaceSampler>>>=const{RefCell::new(Vec::new())};}
const LIMIT: usize = 32 * 1024 * 1024;

pub fn abi_alloc(len: usize) -> usize {
    if len > LIMIT {
        return 0;
    }
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8 as usize
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_free(ptr: usize, len: usize) {
    if ptr != 0 {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            ptr as *mut u8,
            len,
        )))
    }
}
thread_local! {static LAST_RESPONSE: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };}
/// Native hosts fetch responses through these (the packed u64 return truncates
/// the pointer to 32 bits, which only wasm32 linear memory can promise).
pub fn abi_response_ptr() -> usize {
    LAST_RESPONSE.with(|last| last.get().0)
}
pub fn abi_response_len() -> usize {
    LAST_RESPONSE.with(|last| last.get().1)
}
fn packed(v: Value) -> u64 {
    let bytes=value_codec::encode_binary(&v).unwrap_or_else(|_|value_codec::encode_binary(&json!({"ok":false,"error":{"code":"GEOMETRY_RESOURCE_LIMIT","message":"Response exceeds transport limit"}})).unwrap());
    let len = bytes.len();
    let ptr = Box::into_raw(bytes.into_boxed_slice()) as *mut u8 as usize;
    LAST_RESPONSE.with(|last| last.set((ptr, len)));
    ((len as u64) << 32) | ptr as u64
}
fn geometry(result: Result<Value>) -> Value {
    match result {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
    }
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_request(op: u32, ptr: usize, len: usize) -> u64 {
    if len > LIMIT {
        return packed(geometry(Err(input("Request exceeds transport limit"))));
    }
    let bytes = std::slice::from_raw_parts(ptr as *const u8, len);
    let value = match value_codec::decode_binary(bytes) {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(input(e.to_string())))),
    };
    packed(match op {
        0 => geometry(dispatch(value)),
        1 => match value.as_str() {
            Some(s) => match modelgraph_text::compile(s) {
                Ok(value) => json!({"ok":true,"value":value}),
                Err(message) => json!({"ok":false,"message":message}),
            },
            None => json!({"ok":false,"message":"Expected source string"}),
        },
        2 => runtime_value(modelgraph_runtime::compile(value), None),
        3 => runtime_value(modelgraph_runtime::nurbs::compile(value), None),
        4 => match value.as_str() {
            Some(s) => execute_text_value(s),
            None => runtime_value(
                Err(modelgraph_runtime::Error::new(
                    "text_error",
                    "",
                    "Expected source string",
                )),
                None,
            ),
        },
        5 => runtime_value(
            (|| {
                let nodes = value["nodes"].as_array().ok_or_else(|| {
                    modelgraph_runtime::Error::new("invalid_document", "/nodes", "Expected nodes")
                })?;
                let parameters = value["parameters"].as_array().ok_or_else(|| {
                    modelgraph_runtime::Error::new(
                        "invalid_document",
                        "/parameters",
                        "Expected parameters",
                    )
                })?;
                let root = value["root"].as_str().ok_or_else(|| {
                    modelgraph_runtime::Error::new("invalid_document", "/root", "Expected root")
                })?;
                modelgraph_runtime::nurbs::compile_text(nodes.clone(), parameters, root.into())
            })(),
            None,
        ),
        6 => geometry((|| {
            let surface: Surface =
                value_codec::from_value(value).map_err(|e| input(e.to_string()))?;
            let sampler = SurfaceSampler::new(&surface)?;
            SAMPLERS.with(|s| {
                let mut s = s.borrow_mut();
                let id = s.iter().position(Option::is_none).unwrap_or(s.len());
                if id == s.len() {
                    s.push(Some(sampler));
                } else {
                    s[id] = Some(sampler);
                }
                encode(id)
            })
        })()),
        7 => geometry((|| {
            let id = field::<usize>(&value, "id")?;
            let u = field(&value, "u")?;
            let v = field(&value, "v")?;
            SAMPLERS.with(|s| {
                let s = s.borrow();
                let sampler = s
                    .get(id)
                    .and_then(Option::as_ref)
                    .ok_or_else(|| input("Surface evaluator is disposed"))?;
                encode(sampler.evaluate(u, v)?)
            })
        })()),
        8 => geometry((|| {
            let id = field::<usize>(&value, "id")?;
            SAMPLERS.with(|s| {
                if let Some(s) = s.borrow_mut().get_mut(id) {
                    *s = None
                }
            });
            Ok(Value::Null)
        })()),
        9 => geometry((|| {
            let id = field::<u32>(&value, "id")?;
            let mesh = cad::export_buffers(id)?;
            encode(Box::into_raw(Box::new(mesh)) as usize)
        })()),
        _ => geometry(Err(input("Unknown ABI operation"))),
    })
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_mesh_field(ptr: usize, field: u32) -> usize {
    let m = &*(ptr as *const CadMeshBuffer);
    match field {
        0 => m.positions_ptr(),
        1 => m.positions_len(),
        2 => m.indices_ptr(),
        3 => m.indices_len(),
        4 => m.face_ids_ptr(),
        5 => m.face_ids_len(),
        _ => 0,
    }
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_mesh_free(ptr: usize) {
    if ptr != 0 {
        drop(Box::from_raw(ptr as *mut CadMeshBuffer))
    }
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_import_mesh(
    stride: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    if vl > LIMIT / 4 || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    // Request buffers are byte-aligned; read unaligned values rather than assuming allocator alignment.
    let vertices = (0..vl)
        .map(|i| std::ptr::read_unaligned((vp as *const f32).add(i)))
        .collect::<Vec<_>>();
    let indices = (0..il)
        .map(|i| std::ptr::read_unaligned((ip as *const u32).add(i)))
        .collect::<Vec<_>>();
    packed(geometry(
        import_cad_mesh(stride, &vertices, &indices).and_then(encode),
    ))
}
