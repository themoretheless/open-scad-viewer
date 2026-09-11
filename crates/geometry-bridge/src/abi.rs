//! Linear-memory ABI core. The host owns request buffers and frees every response.
//! The `geometry-wasm` shell adds the extern "C" surface.
use super::*;
use std::cell::RefCell;

#[cfg(feature = "languages")]
fn language_abi(op: u32, value: Value) -> Value {
    crate::languages::abi_language(op, value)
}
#[cfg(not(feature = "languages"))]
fn language_abi(_op: u32, _value: Value) -> Value {
    json!({"ok":false,"error":{"code":"GEOMETRY_FEATURE","message":"Language frontends require the languages feature"}})
}
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
        unsafe {
            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                ptr as *mut u8,
                len,
            )))
        }
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
    let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) };
    let value = match value_codec::decode_binary(bytes) {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(input(e.to_string())))),
    };
    packed(match op {
        0 => geometry(dispatch(value)),
        1 | 2 | 3 | 4 | 5 | 10 | 11 => language_abi(op, value),
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
            let mesh = mesh::export_buffers(id)?;
            encode(Box::into_raw(Box::new(mesh)) as usize)
        })()),
        _ => geometry(Err(input("Unknown ABI operation"))),
    })
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_mesh_field(ptr: usize, field: u32) -> usize {
    let m = unsafe { &*(ptr as *const CadMeshBuffer) };
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
        unsafe { drop(Box::from_raw(ptr as *mut CadMeshBuffer)) }
    }
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.

pub unsafe fn abi_import_mesh(stride: usize, vp: usize, vl: usize, ip: usize, il: usize) -> u64 {
    if vl > LIMIT / 4 || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    // Request buffers are byte-aligned; read unaligned values rather than assuming allocator alignment.
    let vertices = (0..vl)
        .map(|i| unsafe { std::ptr::read_unaligned((vp as *const f32).add(i)) })
        .collect::<Vec<_>>();
    let indices = (0..il)
        .map(|i| unsafe { std::ptr::read_unaligned((ip as *const u32).add(i)) })
        .collect::<Vec<_>>();
    packed(geometry(
        import_cad_mesh(stride, &vertices, &indices).and_then(encode),
    ))
}

unsafe fn read_f32(ptr: usize, len: usize) -> Vec<f32> {
    (0..len)
        .map(|i| unsafe { std::ptr::read_unaligned((ptr as *const f32).add(i)) })
        .collect()
}

unsafe fn read_u32(ptr: usize, len: usize) -> Vec<u32> {
    (0..len)
        .map(|i| unsafe { std::ptr::read_unaligned((ptr as *const u32).add(i)) })
        .collect()
}

/// Build the median-split BVH and return a result handle for `abi_array_field`.
/// # Safety
/// vp/vl (f32 vertices) and ip/il (u32 indices) must reference live caller-owned
/// buffers; they are only read. stride/leaf are pre-clamped by the host.
pub unsafe fn abi_bvh_build(
    stride: usize,
    leaf: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    if !(3..=256).contains(&stride) || !(1..=64).contains(&leaf) {
        return packed(geometry(Err(input("Invalid BVH build parameters"))));
    }
    if vl > LIMIT / 4 || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = unsafe { read_f32(vp, vl) };
    let indices = unsafe { read_u32(ip, il) };
    let bvh = polygon_core::solid::bvh::build_mesh_bvh(&vertices, &indices, stride, leaf);
    let handle = mesh_analysis::store(mesh_analysis::AnalysisBuffers::Bvh {
        bounds: bvh.bounds,
        nodes: bvh.nodes,
        triangles: bvh.triangles,
    });
    packed(geometry(encode(handle)))
}

/// Extract semantic edges and return a result handle for `abi_array_field`.
/// # Safety
/// All buffer pointers must reference live caller-owned buffers; they are only
/// read. Merge arrays are pre-validated by the host and may be empty.
#[allow(clippy::too_many_arguments)]
pub unsafe fn abi_semantic_edges(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    mfp: usize,
    mfl: usize,
    mtp: usize,
    mtl: usize,
    weld: u32,
    crease_dot_threshold: f64,
) -> u64 {
    if vl > LIMIT / 4 || il > LIMIT / 4 || mfl > LIMIT / 4 || mtl > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = unsafe { read_f32(vp, vl) };
    let indices = unsafe { read_u32(ip, il) };
    let merge_from = unsafe { read_u32(mfp, mfl) };
    let merge_to = unsafe { read_u32(mtp, mtl) };
    let edges = polygon_core::solid::edges::extract_semantic_edges(
        &vertices,
        &indices,
        &merge_from,
        &merge_to,
        weld != 0,
        crease_dot_threshold,
    );
    let handle = mesh_analysis::store(mesh_analysis::AnalysisBuffers::Edges {
        indices: edges.indices,
        diagnostics: [
            edges.diagnostics.boundary,
            edges.diagnostics.crease,
            edges.diagnostics.non_manifold,
            edges.diagnostics.degenerate,
        ],
    });
    packed(geometry(encode(handle)))
}

/// Read one pointer/length/diagnostic slot of a stored analysis result.
/// # Safety
/// The handle must reference a live result from `abi_bvh_build` or
/// `abi_semantic_edges`.
pub unsafe fn abi_array_field(handle: usize, slot: u32) -> usize {
    mesh_analysis::field(handle, slot)
}

/// Release a stored analysis result exactly once.
/// # Safety
/// The handle must reference a live result from `abi_bvh_build` or
/// `abi_semantic_edges`; it is consumed by this call.
pub unsafe fn abi_array_free(handle: usize) {
    mesh_analysis::free(handle)
}
