//! Raw-buffer handle registry for mesh analysis results (BVH, semantic edges).
//! Results outlive the call that produced them so the host can copy typed
//! views out of linear memory; `free` consumes the handle exactly once.

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
        }
    })
}

pub fn free(handle: usize) {
    RESULTS.with(|results| {
        let mut results = results.borrow_mut();
        if let Some(slot) = handle.checked_sub(1).and_then(|i| results.get_mut(i)) {
            *slot = None;
        }
    });
}
