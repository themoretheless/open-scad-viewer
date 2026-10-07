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
/// Byte ceiling of the export staging allocator (`abi_export_alloc`).
const EXPORT_LIMIT: usize = 128 * 1024 * 1024;
/// Element ceiling for export vertex buffers: 16M f32 = 64 MiB (legacy),
/// 16M f64 = 128 MiB — the dual-format widening doubles bytes, not vertices.
const EXPORT_VERTEX_LIMIT: usize = EXPORT_LIMIT / 8;

pub fn abi_alloc(len: usize) -> usize {
    if len > LIMIT {
        return 0;
    }
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8 as usize
}
/// Export-only staging: 750,000 expanded triangles need 108 MB of stride-6
/// f64 input (54 MB in the legacy f32 format).
/// Keeps the general request allocator's 32 MiB ceiling unchanged.
pub fn abi_export_alloc(len: usize) -> usize {
    if len > EXPORT_LIMIT {
        return 0;
    }
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8 as usize
}
/// # Safety
/// Pointers must reference live buffers allocated by this module, with their exact lengths.
/// Mesh pointers must come from operation 9; freeing consumes them exactly once.
pub unsafe fn abi_free(ptr: usize, len: usize) {
    if ptr != 0 {
        // A packed response Vec may have capacity > length (amortized growth);
        // its true capacity is recorded by `packed`. Everything else allocated
        // here (`abi_alloc`, `abi_export_alloc`, boxed slices) has
        // capacity == length, which is the fallback.
        let cap = LAST_RESPONSE_CAP.with(|c| {
            let (p, cap) = c.get();
            if p == ptr {
                c.set((0, 0));
                cap
            } else {
                len
            }
        });
        unsafe { drop(Vec::from_raw_parts(ptr as *mut u8, len, cap)) }
    }
}
thread_local! {static LAST_RESPONSE: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };}
/// Capacity of the last packed response allocation. `encode_binary` grows its
/// Vec amortized, so capacity usually exceeds length; the response is freed
/// before the next request (see `decodePacked` on the host), so a single slot
/// suffices. Buffers not matching the slot (request/staging allocations from
/// `abi_alloc`, boxed slices) have capacity == length and take the fallback.
thread_local! {static LAST_RESPONSE_CAP: std::cell::Cell<(usize, usize)> = const { std::cell::Cell::new((0, 0)) };}
/// Native hosts fetch responses through these (the packed u64 return truncates
/// the pointer to 32 bits, which only wasm32 linear memory can promise).
pub fn abi_response_ptr() -> usize {
    LAST_RESPONSE.with(|last| last.get().0)
}
pub fn abi_response_len() -> usize {
    LAST_RESPONSE.with(|last| last.get().1)
}
fn packed(v: Value) -> u64 {
    let mut bytes=value_codec::encode_binary(&v).unwrap_or_else(|_|value_codec::encode_binary(&json!({"ok":false,"error":{"code":"GEOMETRY_RESOURCE_LIMIT","message":"Response exceeds transport limit"}})).unwrap());
    let len = bytes.len();
    let cap = bytes.capacity();
    let ptr = bytes.as_mut_ptr() as usize;
    // Hand the Vec allocation over without the boxed-slice shrink (realloc +
    // copy of the whole megabyte-sized response). `abi_free` reconstructs it
    // via `Vec::from_raw_parts(ptr, len, cap)` using the recorded capacity.
    std::mem::forget(bytes);
    LAST_RESPONSE.with(|last| last.set((ptr, len)));
    LAST_RESPONSE_CAP.with(|last| last.set((ptr, cap)));
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
/// Vertex transport formats for the dual-format ingestion ABI: `0` is the
/// legacy f32 layout, `1` passes f64 coordinates without an f32 round-trip.
const FMT_F32: u32 = 0;
const FMT_F64: u32 = 1;

/// Per-format element ceiling for a vertex buffer that must fit the general
/// request allocator (`LIMIT` bytes).
const fn vertex_limit(fmt: u32, bytes_limit: usize) -> usize {
    if fmt == FMT_F64 { bytes_limit / 8 } else { bytes_limit / 4 }
}

/// Copy an uploaded vertex/matrix buffer into an owned f64 Vec. Legacy f32
/// uploads are widened element-by-element (exact); f64 uploads are copied
/// without rounding. Rejects unknown format tags.
unsafe fn read_vertices_f64(fmt: u32, ptr: usize, len: usize) -> Result<Vec<f64>> {
    match fmt {
        FMT_F32 => Ok(unsafe { read_f32(ptr, len) }
            .into_iter()
            .map(|v| v as f64)
            .collect()),
        FMT_F64 => Ok(unsafe { read_pod(ptr, len) }),
        _ => Err(input("Unknown vertex transport format")),
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
    fmt: u32,
) -> u64 {
    if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = match unsafe { read_vertices_f64(fmt, vp, vl) } {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(e))),
    };
    let indices = unsafe { read_u32(ip, il) };
    packed(geometry(
        import_cad_mesh(stride, &vertices, &indices).and_then(encode),
    ))
}

/// Copy a caller-owned typed buffer into an owned `Vec` with one bulk memcpy.
/// Request buffers are byte-aligned, so the copy goes through `u8` pointers and
/// never assumes `T` alignment; `T` must be a plain-old-data type where every
/// bit pattern is a valid value (`f32`, `f64`, `u32`).
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
    fmt: u32,
) -> u64 {
    if fmt > FMT_F64
        || vl > vertex_limit(fmt, LIMIT)
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
    let (vertices, matrix) = match (
        unsafe { read_vertices_f64(fmt, vp, vl) },
        unsafe { read_vertices_f64(fmt, mp, ml) },
    ) {
        (Ok(v), Ok(m)) => (v, m),
        (Err(e), _) | (_, Err(e)) => return packed(geometry(Err(e))),
    };
    let result = polygon_core::solid::placement::place(
        &vertices,
        &unsafe { read_u32(ip, il) },
        &matrix,
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
    fmt: u32,
) -> u64 {
    if fmt > FMT_F64 || vl > EXPORT_VERTEX_LIMIT || il > 2_250_000 || ml > 16 || float32 > 1 {
        return packed(geometry(Err(input("Mesh export exceeds transport limit"))));
    }
    let (vertices, matrix) = match (
        unsafe { read_vertices_f64(fmt, vp, vl) },
        unsafe { read_vertices_f64(fmt, mp, ml) },
    ) {
        (Ok(v), Ok(m)) => (v, m),
        (Err(e), _) | (_, Err(e)) => return packed(geometry(Err(e))),
    };
    let result = mesh_io::export_prepare::prepare(
        &vertices,
        &unsafe { read_u32(ip, il) },
        &matrix,
        float32 == 1,
    )
    .map_err(legacy_mesh_error)
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
    fmt: u32,
) -> u64 {
    if il > 2_250_000 {
        mesh_export_file::poison(handle);
        return packed(geometry(Err(Error::new(
            "MESH_EXPORT_TOO_MANY_TRIANGLES",
            "Export exceeds 750000 triangles",
        ))));
    }
    if fmt > FMT_F64 || vl > EXPORT_VERTEX_LIMIT || ml > 16 {
        mesh_export_file::poison(handle);
        return packed(geometry(Err(input("Mesh export exceeds transport limit"))));
    }
    let (vertices, matrix) = match (
        unsafe { read_vertices_f64(fmt, vp, vl) },
        unsafe { read_vertices_f64(fmt, mp, ml) },
    ) {
        (Ok(v), Ok(m)) => (v, m),
        (Err(e), _) | (_, Err(e)) => {
            mesh_export_file::poison(handle);
            return packed(geometry(Err(e)));
        }
    };
    let result = mesh_export_file::append(
        handle,
        &vertices,
        &unsafe { read_u32(ip, il) },
        &matrix,
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
    let bvh = mesh_query::build_mesh_bvh(&vertices, &indices, stride, leaf);
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
    let edges = mesh_topology::edges::extract_semantic_edges(
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

/// Start an owned cooperative solid analysis; BVH traversal resumes via step.
pub fn abi_solid_analysis_start(id: u32, normal_cosine: f64, edge_cosine: f64, leaf: usize) -> u64 {
    if !normal_cosine.is_finite() || !edge_cosine.is_finite() || !(1..=64).contains(&leaf) {
        return packed(geometry(Err(input("Invalid solid analysis parameters"))));
    }
    packed(geometry(
        mesh_analysis::start_solid_analysis(id, normal_cosine, edge_cosine, leaf).and_then(encode),
    ))
}

pub fn abi_solid_analysis_step(job: u32) -> u64 {
    packed(geometry(
        mesh_analysis::step_solid_analysis(job).and_then(encode),
    ))
}

pub fn abi_solid_analysis_cancel(job: u32) {
    mesh_analysis::cancel_solid_analysis(job);
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

/// Run manifold-core `check` on an uploaded mesh and return a result handle
/// for `abi_array_field` (`ManifoldCheck` slots).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read.
pub unsafe fn abi_manifold_check(vp: usize, vl: usize, ip: usize, il: usize, fmt: u32) -> u64 {
    if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = match unsafe { read_vertices_f64(fmt, vp, vl) } {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(e))),
    };
    let indices: Vec<usize> = unsafe { read_u32(ip, il) }
        .into_iter()
        .map(|i| i as usize)
        .collect();
    let report = manifold_core::check(&vertices, &indices);
    let flat = |edges: &[manifold_core::EdgeKey]| -> Vec<u32> {
        edges
            .iter()
            .flat_map(|&(a, b)| [a as u32, b as u32])
            .collect()
    };
    let flags = (report.is_manifold() as u32) | ((report.is_manifold_with_boundary() as u32) << 1);
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldCheck {
        boundary_edges: flat(&report.boundary_edges),
        non_manifold_edges: flat(&report.non_manifold_edges),
        orientation_edges: flat(&report.orientation_edges),
        degenerate_triangles: report
            .degenerate_triangles
            .iter()
            .map(|&i| i as u32)
            .collect(),
        non_manifold_vertices: report
            .non_manifold_vertices
            .iter()
            .map(|&i| i as u32)
            .collect(),
        isolated_vertices: report.isolated_vertices.iter().map(|&i| i as u32).collect(),
        summary: [
            report.vertex_count as u32,
            report.triangle_count as u32,
            report.component_count as u32,
            flags,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run manifold-core `repair` and return the repaired mesh plus stats
/// (`ManifoldRepair` slots). `mode`: 0 = Conservative (weld + de-degenerate +
/// orientation unify), 1 = Full (additionally splits non-manifold
/// edges/vertices and fills simple boundary loops).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read. `epsilon` must be finite.
pub unsafe fn abi_manifold_repair(
    vp: usize,
    vl: usize,
    ip: usize,
    il: usize,
    fmt: u32,
    epsilon: f64,
    mode: u32,
) -> u64 {
    if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    if !epsilon.is_finite() {
        return packed(geometry(Err(input("Invalid weld epsilon"))));
    }
    let mode = match mode {
        0 => manifold_core::RepairMode::Conservative,
        1 => manifold_core::RepairMode::Full,
        _ => return packed(geometry(Err(input("Invalid repair mode")))),
    };
    let vertices = match unsafe { read_vertices_f64(fmt, vp, vl) } {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(e))),
    };
    let indices: Vec<usize> = unsafe { read_u32(ip, il) }
        .into_iter()
        .map(|i| i as usize)
        .collect();
    let out = manifold_core::repair_with_mode(&vertices, &indices, epsilon, mode);
    let flags = (out.report.residual.is_manifold() as u32)
        | ((out.report.residual.is_manifold_with_boundary() as u32) << 1);
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldRepair {
        positions: out.positions,
        indices: out.indices.iter().map(|&i| i as u32).collect(),
        stats: [
            out.report.welded_vertices as u32,
            out.report.removed_degenerate_triangles as u32,
            out.report.flipped_triangles as u32,
            flags,
            out.report.split_vertices as u32,
            out.report.filled_holes as u32,
            out.report.filled_triangles as u32,
            mode as u32,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run manifold-core `metrics` on an uploaded mesh and return a result handle
/// for `abi_array_field` (`ManifoldMetrics` slots).
/// # Safety
/// vp/vl (vertices, format `fmt`) and ip/il (u32 indices) must reference live
/// caller-owned buffers; they are only read.
pub unsafe fn abi_manifold_metrics(vp: usize, vl: usize, ip: usize, il: usize, fmt: u32) -> u64 {
    if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
        return packed(geometry(Err(input("Mesh exceeds transport limit"))));
    }
    let vertices = match unsafe { read_vertices_f64(fmt, vp, vl) } {
        Ok(v) => v,
        Err(e) => return packed(geometry(Err(e))),
    };
    let indices: Vec<usize> = unsafe { read_u32(ip, il) }
        .into_iter()
        .map(|i| i as usize)
        .collect();
    let m = manifold_core::metrics(&vertices, &indices);
    let mut component_stats = Vec::with_capacity(m.components.len() * 6);
    let mut component_floats = Vec::with_capacity(m.components.len() * 2);
    for c in &m.components {
        component_stats.extend_from_slice(&[
            c.triangle_count as u32,
            c.vertex_count as u32,
            c.edge_count as u32,
            c.boundary_edges as u32,
            c.euler_characteristic as i32 as u32,
            c.genus.map_or(0, |g| g as u32 + 1),
        ]);
        component_floats.extend_from_slice(&[c.signed_volume, c.surface_area]);
    }
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldMetrics {
        component_stats,
        component_floats,
        floats: vec![m.signed_volume, m.surface_area],
        summary: [
            m.vertex_count as u32,
            m.triangle_count as u32,
            m.edge_count as u32,
            m.euler_characteristic as i32 as u32,
            m.components.len() as u32,
            m.watertight as u32,
            0,
            0,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

/// Run a manifold-ops boolean `a OP b` (`op`: 0 = union, 1 = intersection,
/// 2 = difference) with guaranteed strictly manifold output and return the
/// result mesh plus kernel stats (`ManifoldBoolean` slots).
/// # Safety
/// All vertex/index buffers must reference live caller-owned memory; read only.
#[allow(clippy::too_many_arguments)]
pub unsafe fn abi_manifold_boolean(
    a_vp: usize,
    a_vl: usize,
    a_ip: usize,
    a_il: usize,
    a_fmt: u32,
    b_vp: usize,
    b_vl: usize,
    b_ip: usize,
    b_il: usize,
    b_fmt: u32,
    op: u32,
) -> u64 {
    for (fmt, vl, il) in [(a_fmt, a_vl, a_il), (b_fmt, b_vl, b_il)] {
        if fmt > FMT_F64 || vl > vertex_limit(fmt, LIMIT) || il > LIMIT / 4 {
            return packed(geometry(Err(input("Mesh exceeds transport limit"))));
        }
    }
    let op = match op {
        0 => manifold_ops::BooleanOp::Union,
        1 => manifold_ops::BooleanOp::Intersection,
        2 => manifold_ops::BooleanOp::Difference,
        _ => return packed(geometry(Err(input("Invalid boolean operation")))),
    };
    let read = |fmt: u32, vp: usize, vl: usize, ip: usize, il: usize| -> Result<(Vec<f64>, Vec<usize>)> {
        let vertices = unsafe { read_vertices_f64(fmt, vp, vl) }?;
        let indices = unsafe { read_u32(ip, il) }
            .into_iter()
            .map(|i| i as usize)
            .collect();
        Ok((vertices, indices))
    };
    let (a_positions, a_indices) = match read(a_fmt, a_vp, a_vl, a_ip, a_il) {
        Ok(mesh) => mesh,
        Err(e) => return packed(geometry(Err(e))),
    };
    let (b_positions, b_indices) = match read(b_fmt, b_vp, b_vl, b_ip, b_il) {
        Ok(mesh) => mesh,
        Err(e) => return packed(geometry(Err(e))),
    };
    let out = match manifold_ops::boolean_manifold(
        &a_positions,
        &a_indices,
        &b_positions,
        &b_indices,
        op,
    ) {
        Ok(out) => out,
        Err(manifold_ops::BooleanError::Kernel(e)) => return packed(geometry(Err(e))),
        Err(e) => {
            let code = match e {
                manifold_ops::BooleanError::UnrepairableInput { .. } => "GEOMETRY_INVALID_INPUT",
                _ => "GEOMETRY_KERNEL",
            };
            return packed(geometry(Err(Error::new(code, e.to_string()))));
        }
    };
    let repaired = |slot: usize| out.operand_repairs[slot].is_some() as u32;
    let buffers = mesh_analysis::AnalysisBuffers::ManifoldBoolean {
        positions: out.positions,
        indices: out.indices.iter().map(|&i| i as u32).collect(),
        floats: vec![out.kernel_report.tolerance_mm],
        stats: [
            op as u32,
            repaired(0),
            repaired(1),
            out.kernel_report.fragments as u32,
            out.kernel_report.work as u32,
            out.kernel_report.input_triangles[0] as u32,
            out.kernel_report.input_triangles[1] as u32,
            out.output_report.is_manifold() as u32,
        ],
    };
    packed(geometry(encode(mesh_analysis::store(buffers))))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Leak a copy of `data` into a raw buffer, mimicking a host upload.
    /// `read_pod` copies out of it; the buffer is reclaimed by `reclaim`.
    fn upload<T: Copy>(data: &[T]) -> (usize, usize) {
        let v = data.to_vec();
        let ptr = v.as_ptr() as usize;
        std::mem::forget(v);
        (ptr, data.len())
    }

    unsafe fn reclaim<T: Copy>(ptr: usize, len: usize) {
        if ptr != 0 {
            drop(unsafe { Vec::from_raw_parts(ptr as *mut T, len, len) });
        }
    }

    /// Decode the last packed response and free its allocation.
    unsafe fn take_response() -> Value {
        let ptr = abi_response_ptr();
        let len = abi_response_len();
        let bytes = unsafe { std::slice::from_raw_parts(ptr as *const u8, len) }.to_vec();
        unsafe { abi_free(ptr, len) };
        value_codec::decode_binary(&bytes).unwrap()
    }

    fn response_handle(v: &Value) -> usize {
        assert_eq!(v["ok"].as_bool(), Some(true), "{v:?}");
        v["value"].as_u64().unwrap() as usize
    }

    #[test]
    fn manifold_check_reports_open_quad() {
        // Single quad (two triangles): manifold with boundary, one component.
        let positions: [f64; 12] = [0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0.];
        let indices: [u32; 6] = [0, 1, 2, 0, 2, 3];
        let (vp, vl) = upload(&positions);
        let (ip, il) = upload(&indices);
        let packed = unsafe { abi_manifold_check(vp, vl, ip, il, FMT_F64) };
        let _ = packed;
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            // Boundary edges: 4 edges * 2 indices.
            assert_eq!(abi_array_field(handle, 1), 8);
            // No non-manifold / orientation / degenerate issues.
            assert_eq!(abi_array_field(handle, 3), 0);
            assert_eq!(abi_array_field(handle, 5), 0);
            assert_eq!(abi_array_field(handle, 7), 0);
            // Summary: 4 vertices, 2 triangles, 1 component, boundary flag.
            assert_eq!(abi_array_field(handle, 12), 4);
            assert_eq!(abi_array_field(handle, 13), 2);
            assert_eq!(abi_array_field(handle, 14), 1);
            let flags = abi_array_field(handle, 15);
            assert_eq!(flags & 1, 0, "open quad is not strictly manifold");
            assert_ne!(flags & 2, 0, "open quad is manifold with boundary");
            abi_array_free(handle);
            reclaim::<f64>(vp, vl);
            reclaim::<u32>(ip, il);
        }
    }

    #[test]
    fn manifold_check_flags_triple_edge() {
        let positions: [f64; 15] = [
            0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., -1., 0.,
        ];
        let indices: [u32; 9] = [0, 1, 2, 0, 4, 1, 0, 1, 3];
        let (vp, vl) = upload(&positions);
        let (ip, il) = upload(&indices);
        unsafe { abi_manifold_check(vp, vl, ip, il, FMT_F64) };
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            // One non-manifold edge (0, 1) with 3 faces.
            assert_eq!(abi_array_field(handle, 3), 2);
            assert_ne!(abi_array_field(handle, 15) & 1, 1);
            abi_array_free(handle);
            reclaim::<f64>(vp, vl);
            reclaim::<u32>(ip, il);
        }
    }

    #[test]
    fn manifold_repair_welds_unwelded_cube() {
        // 24 duplicated corner vertices (face-local), 12 triangles.
        let corners: [[f64; 3]; 8] = [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ];
        // Face-local quads (bottom, top, front, back, right, left), each CCW
        // seen from outside, referencing cube corners 0..8.
        let faces: [[usize; 4]; 6] = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [1, 2, 6, 5],
            [3, 0, 4, 7],
        ];
        let mut positions: Vec<f64> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        for face in faces {
            let base = (positions.len() / 3) as u32;
            for c in face {
                positions.extend_from_slice(&corners[c]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let (vp, vl) = upload(&positions);
        let (ip, il) = upload(&indices);
        unsafe { abi_manifold_repair(vp, vl, ip, il, FMT_F64, 0.0, 0) };
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            // Welded 24 -> 8 vertices, nothing removed or flipped.
            assert_eq!(abi_array_field(handle, 1), 24); // 8 vertices * 3
            assert_eq!(abi_array_field(handle, 3), 36); // 12 triangles * 3
            assert_eq!(abi_array_field(handle, 4), 16); // welded
            assert_eq!(abi_array_field(handle, 5), 0); // degenerates
            let flags = abi_array_field(handle, 7);
            assert_eq!(flags & 1, 1, "repaired cube must be strictly manifold");
            abi_array_free(handle);
            reclaim::<f64>(vp, vl);
            reclaim::<u32>(ip, il);
        }
    }

    #[test]
    fn manifold_check_rejects_unknown_format() {
        let positions: [f64; 3] = [0., 0., 0.];
        let (vp, vl) = upload(&positions);
        unsafe { abi_manifold_check(vp, vl, 0, 0, 7) };
        let response = unsafe { take_response() };
        assert_eq!(response["ok"].as_bool(), Some(false));
        unsafe { reclaim::<f64>(vp, vl) };
    }

    /// Unit tetrahedron, outward CCW: closed, genus 0, volume 1/6.
    fn tetrahedron() -> (Vec<f64>, Vec<u32>) {
        let positions = vec![0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.];
        let indices = vec![0, 2, 1, 0, 1, 3, 1, 2, 3, 2, 0, 3];
        (positions, indices)
    }

    #[test]
    fn manifold_metrics_reports_tetrahedron() {
        let (positions, indices) = tetrahedron();
        let (vp, vl) = upload(&positions);
        let (ip, il) = upload(&indices);
        unsafe { abi_manifold_metrics(vp, vl, ip, il, FMT_F64) };
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            // Summary: 4 vertices, 4 triangles, 6 edges, euler 2, 1 component.
            assert_eq!(abi_array_field(handle, 6), 4);
            assert_eq!(abi_array_field(handle, 7), 4);
            assert_eq!(abi_array_field(handle, 8), 6);
            assert_eq!(abi_array_field(handle, 9) as i32, 2);
            assert_eq!(abi_array_field(handle, 10), 1);
            assert_eq!(abi_array_field(handle, 11) & 1, 1, "closed tetra is watertight");
            // One component record: 4 triangles, genus 0 (stored as genus + 1).
            assert_eq!(abi_array_field(handle, 1), 6);
            let stats_ptr = abi_array_field(handle, 0) as *const u32;
            let stats = std::slice::from_raw_parts(stats_ptr, 6);
            assert_eq!(stats[0], 4);
            assert_eq!(stats[3], 0, "no boundary edges");
            assert_eq!(stats[5], 1, "genus 0 stored as genus + 1");
            // Whole-mesh floats: signed volume 1/6, surface area > 0.
            assert_eq!(abi_array_field(handle, 5), 2);
            let floats = std::slice::from_raw_parts(abi_array_field(handle, 4) as *const f64, 2);
            assert!((floats[0] - 1.0 / 6.0).abs() < 1e-12, "volume: {}", floats[0]);
            assert!(floats[1] > 1.0, "area: {}", floats[1]);
            abi_array_free(handle);
            reclaim::<f64>(vp, vl);
            reclaim::<u32>(ip, il);
        }
    }

    #[test]
    fn manifold_boolean_unions_overlapping_boxes() {
        // Two unit cubes offset by half an edge; union must be strictly
        // manifold with 26 triangles... rather: just assert the contract —
        // success, manifold flag set, nonempty output.
        let box_mesh = |ox: f64| -> (Vec<f64>, Vec<u32>) {
            let corners: [[f64; 3]; 8] = [
                [ox, 0., 0.],
                [ox + 1., 0., 0.],
                [ox + 1., 1., 0.],
                [ox, 1., 0.],
                [ox, 0., 1.],
                [ox + 1., 0., 1.],
                [ox + 1., 1., 1.],
                [ox, 1., 1.],
            ];
            let faces: [[usize; 4]; 6] = [
                [0, 3, 2, 1],
                [4, 5, 6, 7],
                [0, 1, 5, 4],
                [2, 3, 7, 6],
                [1, 2, 6, 5],
                [3, 0, 4, 7],
            ];
            let mut positions = Vec::new();
            let mut indices = Vec::new();
            for face in faces {
                let base = (positions.len() / 3) as u32;
                for c in face {
                    positions.extend_from_slice(&corners[c]);
                }
                indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
            }
            // Weld into a strictly manifold box first (the ABI checks input).
            let out = manifold_core::repair_with_mode(
                &positions,
                &indices.iter().map(|&i| i as usize).collect::<Vec<_>>(),
                0.0,
                manifold_core::RepairMode::Conservative,
            );
            assert!(out.is_manifold());
            (out.positions, out.indices.iter().map(|&i| i as u32).collect())
        };
        let (ap, ai) = box_mesh(0.0);
        let (bp, bi) = box_mesh(0.5);
        let (avp, avl) = upload(&ap);
        let (aip, ail) = upload(&ai);
        let (bvp, bvl) = upload(&bp);
        let (bip, bil) = upload(&bi);
        unsafe { abi_manifold_boolean(avp, avl, aip, ail, FMT_F64, bvp, bvl, bip, bil, FMT_F64, 0) };
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            assert!(abi_array_field(handle, 1) > 0, "union produced vertices");
            assert!(abi_array_field(handle, 3) > 0, "union produced triangles");
            // Stats slots 6..=13: op, repaired a/b, fragments, work, inputs, flags.
            assert_eq!(abi_array_field(handle, 6), 0, "union op tag");
            assert_eq!(abi_array_field(handle, 11), 12, "operand a triangles");
            assert_eq!(abi_array_field(handle, 12), 12, "operand b triangles");
            assert_eq!(abi_array_field(handle, 13) & 1, 1, "strictly manifold output");
            abi_array_free(handle);
            reclaim::<f64>(avp, avl);
            reclaim::<u32>(aip, ail);
            reclaim::<f64>(bvp, bvl);
            reclaim::<u32>(bip, bil);
        }
    }

    #[test]
    fn manifold_repair_full_fills_open_box_top() {
        // Closed box minus its top face: conservative repair leaves the
        // boundary; Full mode fills the loop.
        let corners: [[f64; 3]; 8] = [
            [0., 0., 0.],
            [1., 0., 0.],
            [1., 1., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [1., 0., 1.],
            [1., 1., 1.],
            [0., 1., 1.],
        ];
        let faces: [[usize; 4]; 5] = [
            [0, 3, 2, 1],
            [0, 1, 5, 4],
            [2, 3, 7, 6],
            [1, 2, 6, 5],
            [3, 0, 4, 7],
        ];
        let mut positions = Vec::new();
        let mut indices = Vec::new();
        for face in faces {
            let base = (positions.len() / 3) as u32;
            for c in face {
                positions.extend_from_slice(&corners[c]);
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        }
        let (vp, vl) = upload(&positions);
        let (ip, il) = upload(&indices);
        unsafe { abi_manifold_repair(vp, vl, ip, il, FMT_F64, 0.0, 1) };
        let response = unsafe { take_response() };
        let handle = response_handle(&response);
        unsafe {
            assert_eq!(abi_array_field(handle, 8), 0, "no splits needed");
            assert_eq!(abi_array_field(handle, 9), 1, "one boundary loop filled");
            assert_eq!(abi_array_field(handle, 10), 2, "quad filled with 2 triangles");
            let flags = abi_array_field(handle, 7);
            assert_eq!(flags & 1, 1, "full repair closes the box");
            assert_eq!(abi_array_field(handle, 11), 1, "mode tag is Full");
            abi_array_free(handle);
            reclaim::<f64>(vp, vl);
            reclaim::<u32>(ip, il);
        }
    }
}

/// Decode bounded mesh bytes without expanding each byte through the value codec.
/// # Safety
/// `ptr..ptr+len` must be a live allocation owned by this kernel.
pub unsafe fn abi_mesh_decode(format:u32,ptr:usize,len:usize)->u64{
 if len>20_000_000{return packed(geometry(Err(input("Mesh input exceeds byte budget"))));}
 let bytes=if len==0{&[]}else{unsafe{std::slice::from_raw_parts(ptr as *const u8,len)}};
 let result=(||{use mesh_io::import as import;let mesh=match format{0=>import::obj(bytes)?,1=>import::ply(bytes)?,2=>import::stl(bytes)?,3=>import::off(bytes)?,4=>{let positions=import::binary_stl(bytes)?.into_iter().map(f64::from).collect::<Vec<_>>();let indices=(0..positions.len()/3).collect();import::RawMesh{positions,indices}},_=>return Err(input("Unknown mesh decoder format"))};let indices=mesh.indices.into_iter().map(|i|u32::try_from(i).map_err(|_|input("Import index exceeds u32"))).collect::<Result<Vec<_>>>()?;encode(mesh_analysis::store(mesh_analysis::AnalysisBuffers::Placement{positions:mesh.positions,indices}))})();packed(geometry(result))
}
/// Prepare display triangle soup into retained typed result buffers.
/// # Safety
/// `ptr` references `len` live f32 components owned by this kernel.
pub unsafe fn abi_mesh_soup_render(ptr:usize,len:usize)->u64{
 if len>250_000*9{return packed(geometry(Err(input("Triangle soup exceeds budget"))));}
 let positions=unsafe{read_f32(ptr,len)};let result=(||{let soup=mesh_topology::soup::render(&positions)?;let discarded=soup.discarded;let handle=mesh_analysis::store(mesh_analysis::AnalysisBuffers::Render(crate::mesh::RenderMesh{vertices:soup.vertices,indices:soup.indices,merge_from:Vec::new(),merge_to:Vec::new(),face_ids:soup.face_ids}));encode(json!({"handle":handle,"discarded":discarded}))})();packed(geometry(result))
}

/// Build an owned transparency tree from packed f64 triangle attributes.
/// # Safety
/// vp/vl must reference a live caller-owned f64 buffer, read only.
pub unsafe fn abi_transparent_bsp(width:usize,vp:usize,vl:usize,limit:usize,operations:usize,tolerance:f64)->u64 {
    if vl>LIMIT/8 || width>LIMIT/24 || limit>u32::MAX as usize || operations>u32::MAX as usize {
        return packed(geometry(Err(input("Transparency input exceeds transport limit"))));
    }
    let result=crate::transparent_bsp::buffers(width,unsafe{read_pod::<f64>(vp,vl)},geometry_ops::transparency::Limits{fragments:limit,operations,tolerance})
        .map(|v|mesh_analysis::store(mesh_analysis::AnalysisBuffers::Transparency(v))).and_then(encode);
    packed(geometry(result))
}
