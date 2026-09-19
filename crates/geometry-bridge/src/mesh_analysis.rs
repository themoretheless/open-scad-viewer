//! Raw-buffer handle registry for mesh analysis results (BVH, semantic edges).
//! Results outlive the call that produced them so the host can copy typed
//! views out of linear memory; `free` consumes the handle exactly once.

pub struct SolidAnalysis {
    mesh: crate::mesh::RenderMesh,
    bvh: polygon_core::solid::bvh::MeshBvh,
    edges: polygon_core::solid::edges::SemanticEdges,
}

/// Analyze the same f32 display buffers the host publishes, while they are
/// still in the kernel. Merge pairs preserve the source solid's topology.
pub fn analyze_solid(
    id: u32,
    normal_cosine: f64,
    edge_cosine: f64,
    leaf_size: usize,
) -> crate::Result<SolidAnalysis> {
    let mesh = crate::mesh::render_buffers(id, normal_cosine)?;
    let bvh = polygon_core::solid::bvh::build_mesh_bvh(&mesh.vertices, &mesh.indices, 6, leaf_size);
    let edges = polygon_core::solid::edges::extract_semantic_edges(
        &mesh.vertices,
        &mesh.indices,
        &mesh.merge_from,
        &mesh.merge_to,
        false,
        edge_cosine,
    );
    Ok(SolidAnalysis { mesh, bvh, edges })
}

pub enum AnalysisBuffers {
    Bytes {
        bytes: Vec<u8>,
    },
    Export {
        positions: Vec<f64>,
        indices: Vec<u32>,
        normals: Vec<f64>,
    },
    Placement {
        positions: Vec<f64>,
        indices: Vec<u32>,
    },
    Bvh {
        bounds: Vec<f32>,
        nodes: Vec<u32>,
        triangles: Vec<u32>,
    },
    Edges {
        indices: Vec<u32>,
        diagnostics: [u32; 4],
    },
    Render(crate::mesh::RenderMesh),
    Solid(SolidAnalysis),
}

thread_local! {
    static RESULTS: std::cell::RefCell<Vec<Option<AnalysisBuffers>>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

pub fn store(result: AnalysisBuffers) -> usize {
    RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        let slot = results
            .iter()
            .position(Option::is_none)
            .unwrap_or(results.len());
        if slot == results.len() {
            results.push(Some(result));
        } else {
            results[slot] = Some(result);
        }
        // Handle 0 is reserved for the invalid handle.
        slot + 1
    })
}

/// Even slots return a buffer pointer, odd slots its element length.
/// BVH: 0/1 bounds (f32), 2/3 nodes (u32), 4/5 triangles (u32).
/// Placement: 0/1 positions (f64), 2/3 indices (u32).
/// Edges: 0/1 indices (u32); slots 2..=5 return the diagnostic counters
/// (boundary, crease, non-manifold, degenerate) directly.
/// Render: 0/1 vertices (f32, stride 6), 2/3 indices, 4/5 merge-from,
/// 6/7 merge-to, 8/9 face ids (all u32).
/// Solid: Render slots 0..=9, 10/11 BVH bounds (f32), 12/13 BVH nodes,
/// 14/15 BVH triangles, 16/17 edge indices (u32), 18..=21 diagnostic counters.
pub fn field(handle: usize, slot: u32) -> usize {
    RESULTS.with(|results| {
        let results = results.borrow();
        let Some(Some(result)) = handle.checked_sub(1).and_then(|i| results.get(i)) else {
            return 0;
        };
        match result {
            AnalysisBuffers::Bytes { bytes } => match slot {
                0 => bytes.as_ptr() as usize,
                1 => bytes.len(),
                _ => 0,
            },
            AnalysisBuffers::Export {
                positions,
                indices,
                normals,
            } => match slot {
                0 => positions.as_ptr() as usize,
                1 => positions.len(),
                2 => indices.as_ptr() as usize,
                3 => indices.len(),
                4 => normals.as_ptr() as usize,
                5 => normals.len(),
                _ => 0,
            },
            AnalysisBuffers::Placement { positions, indices } => match slot {
                0 => positions.as_ptr() as usize,
                1 => positions.len(),
                2 => indices.as_ptr() as usize,
                3 => indices.len(),
                _ => 0,
            },
            AnalysisBuffers::Bvh {
                bounds,
                nodes,
                triangles,
            } => match slot {
                0 => bounds.as_ptr() as usize,
                1 => bounds.len(),
                2 => nodes.as_ptr() as usize,
                3 => nodes.len(),
                4 => triangles.as_ptr() as usize,
                5 => triangles.len(),
                _ => 0,
            },
            AnalysisBuffers::Edges {
                indices,
                diagnostics,
            } => match slot {
                0 => indices.as_ptr() as usize,
                1 => indices.len(),
                2..=5 => diagnostics[slot as usize - 2] as usize,
                _ => 0,
            },
            AnalysisBuffers::Render(mesh) => render_field(mesh, slot),
            AnalysisBuffers::Solid(result) => match slot {
                0..=9 => render_field(&result.mesh, slot),
                10 => result.bvh.bounds.as_ptr() as usize,
                11 => result.bvh.bounds.len(),
                12 => result.bvh.nodes.as_ptr() as usize,
                13 => result.bvh.nodes.len(),
                14 => result.bvh.triangles.as_ptr() as usize,
                15 => result.bvh.triangles.len(),
                16 => result.edges.indices.as_ptr() as usize,
                17 => result.edges.indices.len(),
                18 => result.edges.diagnostics.boundary as usize,
                19 => result.edges.diagnostics.crease as usize,
                20 => result.edges.diagnostics.non_manifold as usize,
                21 => result.edges.diagnostics.degenerate as usize,
                _ => 0,
            },
        }
    })
}

fn render_field(mesh: &crate::mesh::RenderMesh, slot: u32) -> usize {
    match slot {
        0 => mesh.vertices.as_ptr() as usize,
        1 => mesh.vertices.len(),
        2 => mesh.indices.as_ptr() as usize,
        3 => mesh.indices.len(),
        4 => mesh.merge_from.as_ptr() as usize,
        5 => mesh.merge_from.len(),
        6 => mesh.merge_to.as_ptr() as usize,
        7 => mesh.merge_to.len(),
        8 => mesh.face_ids.as_ptr() as usize,
        9 => mesh.face_ids.len(),
        _ => 0,
    }
}

pub fn free(handle: usize) {
    RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        if let Some(slot) = handle.checked_sub(1).and_then(|i| results.get_mut(i)) {
            *slot = None;
        }
    });
}
