//! Linear-memory ABI core. The host owns request buffers and frees every response.
//! The `geometry-wasm` shell adds the extern "C" surface.
use super::*;
use std::cell::RefCell;

/// Language frontends ship as their own module (`languages-wasm`); the host routes their operations
/// there. Keeping the numbers reserved here makes a misrouted call explicit instead of silent.
fn language_abi(_op: u32, _value: Value) -> Value {
    json!({"ok":false,"error":{"code":"GEOMETRY_FEATURE","message":"Language frontends live in the language kernel"}})
}
thread_local! {static SAMPLERS:RefCell<Vec<Option<SurfaceSampler>>>=const{RefCell::new(Vec::new())};}
const LIMIT: usize = 32 * 1024 * 1024;

pub fn abi_alloc(len: usize) -> usize {
    if len > LIMIT {
        return 0;
    }
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8 as usize
}
/// Export-only staging: 750,000 expanded triangles need 54 MB of stride-6 input.
/// Keeps the general request allocator's 32 MiB ceiling unchanged.
pub fn abi_export_alloc(len: usize) -> usize {
    if len > 64 * 1024 * 1024 {
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
        Err(error) => json!({"ok":false,"error":crate::error_json(&error)}),
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
        6 => {
            let result: Result<Value> = try {
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
                })?
            };
            geometry(result)
        }
        7 => {
            let result: Result<Value> = try {
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
                })?
            };
            geometry(result)
        }
        8 => {
            let result: Result<Value> = try {
                let id = field::<usize>(&value, "id")?;
                SAMPLERS.with(|s| {
                    if let Some(s) = s.borrow_mut().get_mut(id) {
                        *s = None
                    }
                });
                Value::Null
            };
            geometry(result)
        }
        9 => {
            let result: Result<Value> = try {
                let id = field::<u32>(&value, "id")?;
                let mesh = mesh::export_buffers(id)?;
                encode(Box::into_raw(Box::new(mesh)) as usize)?
            };
            geometry(result)
        }
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
    let vertices = unsafe { read_f32(vp, vl) };
    let indices = unsafe { read_u32(ip, il) };
    packed(geometry(
        import_cad_mesh(stride, &vertices, &indices).and_then(encode),
    ))
}

/// Copy a caller-owned typed buffer into an owned `Vec` with one bulk memcpy.
/// Request buffers are byte-aligned, so the copy goes through `u8` pointers and
/// never assumes `T` alignment; `T` must be a plain-old-data type where every
/// bit pattern is a valid value (`f32`, `u32`).
///
/// # Safety
/// `ptr` must be readable for `len * size_of::<T>()` bytes for the duration of
/// the call, or `len` must be zero.
#[inline]
unsafe fn read_pod<T: Copy>(ptr: usize, len: usize) -> Vec<T> {
    if len == 0 || ptr == 0 {
        return Vec::new();
    }
    let mut out = Vec::<T>::with_capacity(len);
    unsafe {
        std::ptr::copy_nonoverlapping(
            ptr as *const u8,
            out.as_mut_ptr() as *mut u8,
            len * std::mem::size_of::<T>(),
        );
        out.set_len(len);
    }
    out
}

#[inline]
unsafe fn read_f32(ptr: usize, len: usize) -> Vec<f32> {
    unsafe { read_pod(ptr, len) }
}

#[inline]
unsafe fn read_u32(ptr: usize, len: usize) -> Vec<u32> {
    unsafe { read_pod(ptr, len) }
}

/// Bake one display mesh into f64 Solid positions and outward triangle indices.
/// # Safety
/// Buffer ranges must be live caller-owned allocations; they are read only.
pub unsafe fn abi_solid_placement(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    mp: usize,
    ml: usize,
) -> u64 {
    if vl > LIMIT / 4
        || il > LIMIT / 4
        || ml > 16
        || (vl / 6)
            .checked_mul(24)
            .and_then(|v| il.checked_mul(4).and_then(|i| v.checked_add(i)))
            .is_none_or(|n| n > LIMIT)
    {
        return packed(geometry(Err(input(
            "Solid placement exceeds transport limit",
        ))));
    }
    let result = polygon_core::solid::placement::place(
        &unsafe { read_f32(vp, vl) },
        &unsafe { read_u32(ip, il) },
        &unsafe { read_f32(mp, ml) },
    )
    .map(|result| {
        result.map_or(0, |mesh| {
            mesh_analysis::store(mesh_analysis::AnalysisBuffers::Placement {
                positions: mesh.positions,
                indices: mesh.indices,
            })
        })
    })
    .and_then(encode);
    packed(geometry(result))
}

/// # Safety
/// All ranges must be live caller-owned buffers. Returns copied result views.
pub unsafe fn abi_export_prepare(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    mp: usize,
    ml: usize,
    float32: u32,
) -> u64 {
    if vl > 64 * 1024 * 1024 / 4 || il > 2_250_000 || ml > 16 || float32 > 1 {
        return packed(geometry(Err(input("Mesh export exceeds transport limit"))));
    }
    let result = polygon_core::solid::export_prepare::prepare(
        &unsafe { read_f32(vp, vl) },
        &unsafe { read_u32(ip, il) },
        &unsafe { read_f32(mp, ml) },
        float32 == 1,
    )
    .map(|mesh| {
        mesh_analysis::store(mesh_analysis::AnalysisBuffers::Export {
            positions: mesh.positions,
            indices: mesh.indices,
            normals: mesh.normals,
        })
    })
    .and_then(encode);
    packed(geometry(result))
}

/// # Safety
/// All ranges must be live caller-owned export staging buffers.
pub unsafe fn abi_export_append(
    handle: u32,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    mp: usize,
    ml: usize,
) -> u64 {
    if il > 2_250_000 {
        mesh_export_file::poison(handle);
        return packed(geometry(Err(Error::new(
            "MESH_EXPORT_TOO_MANY_TRIANGLES",
            "Export exceeds 750000 triangles",
        ))));
    }
    if vl > 64 * 1024 * 1024 / 4 || ml > 16 {
        mesh_export_file::poison(handle);
        return packed(geometry(Err(input("Mesh export exceeds transport limit"))));
    }
    let result = mesh_export_file::append(
        handle,
        &unsafe { read_f32(vp, vl) },
        &unsafe { read_u32(ip, il) },
        &unsafe { read_f32(mp, ml) },
    )
    .map(|()| Value::Null);
    packed(geometry(result))
}

/// Build the median-split BVH and return a result handle for `abi_array_field`.
/// Upload an immutable native picking snapshot using raw typed buffers.
/// # Safety
/// vp/vl and ip/il must reference live caller-owned buffers.
pub unsafe fn abi_picking_create(
    stride: usize,
    leaf: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
) -> u64 {
    if vl > LIMIT / 4 || il > LIMIT / 4 {
        return packed(geometry(Err(input(
            "Picking upload exceeds transport limit",
        ))));
    }
    let vertices = unsafe { read_f32(vp, vl) };
    let indices = unsafe { read_u32(ip, il) };
    packed(geometry(
        super::mesh_picking::create(vertices, indices, stride, leaf).and_then(encode),
    ))
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

/// Compute connected surface ids in caller-owned mesh buffers.
/// # Safety
/// Buffer ranges must reference live caller-owned allocations.
pub unsafe fn abi_surface_groups(
    stride: usize,
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    angle: f64,
) -> u64 {
    if vl > LIMIT / 4 || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = unsafe { read_f32(vp, vl) };
    let indices = unsafe { read_u32(ip, il) };
    packed(geometry(
        crate::mesh_surface_groups::surface_group_ids(stride, &vertices, &indices, angle)
            .map(|ids| mesh_analysis::store(mesh_analysis::AnalysisBuffers::SurfaceGroups { ids }))
            .and_then(encode),
    ))
}

/// Build the display mesh of a retained solid (crease-split normals, merge
/// pairs, face ids) inside the kernel and return a result handle for
/// `abi_array_field`. Reads no host memory; the handle is validated by the
/// solid store.
pub fn abi_render_mesh(id: u32, crease_cosine: f64) -> u64 {
    if !crease_cosine.is_finite() {
        return packed(geometry(Err(input("Invalid crease cosine"))));
    }
    packed(geometry(
        mesh::render_buffers(id, crease_cosine)
            .map(|mesh| mesh_analysis::store(mesh_analysis::AnalysisBuffers::Render(mesh)))
            .and_then(encode),
    ))
}

/// Display mesh, BVH and semantic edges from a retained solid, without host uploads.
pub fn abi_analyze_solid(id: u32, normal_cosine: f64, edge_cosine: f64, leaf: usize) -> u64 {
    if !normal_cosine.is_finite() || !edge_cosine.is_finite() || !(1..=64).contains(&leaf) {
        return packed(geometry(Err(input("Invalid solid analysis parameters"))));
    }
    packed(geometry(
        mesh_analysis::analyze_solid(id, normal_cosine, edge_cosine, leaf)
            .map(|analysis| mesh_analysis::store(mesh_analysis::AnalysisBuffers::Solid(analysis)))
            .and_then(encode),
    ))
}

/// Read one pointer/length/diagnostic slot of a stored analysis result.
/// # Safety
/// The handle must reference a live array result (BVH, edges, placement, or export).
pub unsafe fn abi_array_field(handle: usize, slot: u32) -> usize {
    mesh_analysis::field(handle, slot)
}

/// Release a stored analysis result exactly once.
/// # Safety
/// The handle must reference a live array result; it is consumed by this call.
pub unsafe fn abi_array_free(handle: usize) {
    mesh_analysis::free(handle)
}
