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

// Jobs own their display buffers and pinned BVH future. No linear-memory view
// or source-solid borrow survives a step on the host. IDs never get recycled:
// cancelling a stale job cannot drop a newer one.
type AnalysisFuture = std::pin::Pin<Box<dyn std::future::Future<Output = SolidAnalysis>>>;
#[derive(Default)]
struct AnalysisJobs {
    next: u32,
    jobs: std::collections::BTreeMap<u32, AnalysisFuture>,
}
thread_local! {
    static JOBS: std::cell::RefCell<AnalysisJobs> = std::cell::RefCell::new(AnalysisJobs::default());
}
const MAX_ANALYSIS_JOBS: usize = 2;
const MAX_ANALYSIS_TRIANGLES: usize = 750_000;

pub fn start_solid_analysis(
    id: u32,
    normal_cosine: f64,
    edge_cosine: f64,
    leaf_size: usize,
) -> crate::Result<u32> {
    if !normal_cosine.is_finite() || !edge_cosine.is_finite() || !(1..=64).contains(&leaf_size) {
        return Err(crate::input("Invalid solid analysis parameters"));
    }
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        if jobs.jobs.len() >= MAX_ANALYSIS_JOBS {
            return Err(crate::input("Solid analysis job budget exceeded"));
        }
        let next = jobs
            .next
            .checked_add(1)
            .ok_or_else(|| crate::input("Solid analysis job IDs exhausted"))?;
        crate::mesh::check_analysis_triangle_budget(id, MAX_ANALYSIS_TRIANGLES)?;
        let mesh = crate::mesh::render_buffers(id, normal_cosine)?;
        jobs.jobs.insert(
            next,
            Box::pin(async move {
                let bvh = polygon_core::solid::bvh::build_mesh_bvh_cooperative(
                    &mesh.vertices,
                    &mesh.indices,
                    6,
                    leaf_size,
                )
                .await;
                // Let the host observe cancellation before entering edge extraction,
                // which also yields internally. No partial result has been published.
                let mut yielded = false;
                std::future::poll_fn(|_| {
                    if std::mem::replace(&mut yielded, true) {
                        std::task::Poll::Ready(())
                    } else {
                        std::task::Poll::Pending
                    }
                })
                .await;
                let edges = polygon_core::solid::edges::extract_semantic_edges_cooperative(
                    &mesh.vertices,
                    &mesh.indices,
                    &mesh.merge_from,
                    &mesh.merge_to,
                    false,
                    edge_cosine,
                )
                .await;
                SolidAnalysis { mesh, bvh, edges }
            }),
        );
        jobs.next = next;
        Ok(next)
    })
}

/// Zero means pending; a nonzero result is an ordinary owned array-result
/// handle. Completion consumes the job. Unknown/completed/cancelled jobs refuse.
pub fn step_solid_analysis(id: u32) -> crate::Result<usize> {
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        let future = jobs
            .jobs
            .get_mut(&id)
            .ok_or_else(|| crate::input("Unknown or completed solid analysis job"))?;
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Pending => Ok(0),
            std::task::Poll::Ready(result) => {
                jobs.jobs.remove(&id);
                Ok(store(AnalysisBuffers::Solid(result)))
            }
        }
    })
}

/// Idempotent cancellation drops all unfinished scratch and owned input data.
pub fn cancel_solid_analysis(id: u32) {
    JOBS.with(|jobs| {
        jobs.borrow_mut().jobs.remove(&id);
    });
}

pub enum AnalysisBuffers {
    SurfaceGroups {
        ids: Vec<u32>,
    },
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
/// SurfaceGroups: 0/1 ids (u32).
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
            AnalysisBuffers::SurfaceGroups { ids } => match slot {
                0 => ids.as_ptr() as usize,
                1 => ids.len(),
                _ => 0,
            },
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

#[cfg(test)]
mod cooperative_tests {
    use super::*;
    use value_codec::json;

    fn sphere() -> u32 {
        crate::mesh::dispatch(json!({"action":"sphere", "radius":3, "segments":128}))
            .unwrap()
            .as_u64()
            .unwrap() as u32
    }
    fn delete(id: u32) {
        crate::mesh::dispatch(json!({"action":"delete", "ids":[id]})).unwrap();
    }
    fn start(id: u32) -> u32 {
        start_solid_analysis(id, 0.6, 0.8, 8).unwrap()
    }
    fn live() -> usize {
        JOBS.with(|jobs| jobs.borrow().jobs.len())
    }

    #[test]
    fn cancelled_jobs_drop_scratch_and_never_alias_new_jobs() {
        let id = sphere();
        for _ in 0..8 {
            let cancelled = start(id);
            assert_eq!(step_solid_analysis(cancelled).unwrap(), 0);
            assert_eq!(live(), 1);
            cancel_solid_analysis(cancelled);
            assert_eq!(live(), 0);
            let next = start(id);
            assert!(next > cancelled);
            cancel_solid_analysis(cancelled);
            assert!(step_solid_analysis(cancelled).is_err());
            assert_eq!(step_solid_analysis(next).unwrap(), 0);
            cancel_solid_analysis(next);
        }
        delete(id);
    }

    #[test]
    fn cancellation_after_bvh_drops_edge_scratch_and_releases_admission() {
        use std::future::Future;
        let id = sphere();
        let mesh = crate::mesh::render_buffers(id, 0.6).unwrap();
        let mut bvh = std::pin::pin!(polygon_core::solid::bvh::build_mesh_bvh_cooperative(
            &mesh.vertices,
            &mesh.indices,
            6,
            8
        ));
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        let mut bvh_polls = 1;
        while bvh.as_mut().poll(&mut cx).is_pending() {
            bvh_polls += 1;
        }
        for edge_steps in [1, 20, 80] {
            let job = start(id);
            // The poll completing BVH also reaches the pre-edge yield.
            for _ in 0..bvh_polls + edge_steps {
                assert_eq!(step_solid_analysis(job).unwrap(), 0);
            }
            assert_eq!(live(), 1);
            cancel_solid_analysis(job);
            assert_eq!(live(), 0);
            assert!(step_solid_analysis(job).is_err());
        }
        let a = start(id);
        let b = start(id);
        cancel_solid_analysis(a);
        cancel_solid_analysis(b);
        delete(id);
    }

    #[test]
    fn completion_matches_sync_and_owns_snapshot_after_source_deletion() {
        let id = sphere();
        let expected = analyze_solid(id, 0.6, 0.8, 8).unwrap();
        let job = start(id);
        delete(id);
        let mut polls = 0;
        let handle = loop {
            polls += 1;
            assert!(polls < 10_000);
            let result = step_solid_analysis(job).unwrap();
            if result != 0 {
                break result;
            }
        };
        assert!(polls > 2);
        assert_eq!(live(), 0);
        assert!(step_solid_analysis(job).is_err());
        cancel_solid_analysis(job);
        RESULTS.with(|results| {
            let results = results.borrow();
            let Some(AnalysisBuffers::Solid(actual)) = &results[handle - 1] else {
                panic!("missing result")
            };
            assert_eq!(actual.mesh.vertices, expected.mesh.vertices);
            assert_eq!(actual.mesh.indices, expected.mesh.indices);
            assert_eq!(
                actual
                    .bvh
                    .bounds
                    .iter()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>(),
                expected
                    .bvh
                    .bounds
                    .iter()
                    .map(|x| x.to_bits())
                    .collect::<Vec<_>>()
            );
            assert_eq!(actual.bvh.nodes, expected.bvh.nodes);
            assert_eq!(actual.bvh.triangles, expected.bvh.triangles);
            assert_eq!(actual.edges.indices, expected.edges.indices);
            assert_eq!(actual.edges.diagnostics, expected.edges.diagnostics);
        });
        free(handle);
    }

    #[test]
    fn admission_is_bounded_and_failure_does_not_poison_registry() {
        let id = sphere();
        assert!(start_solid_analysis(id, f64::NAN, 0.8, 8).is_err());
        assert!(start_solid_analysis(id, 0.6, 0.8, 0).is_err());
        let a = start(id);
        let b = start(id);
        assert!(start_solid_analysis(id, 0.6, 0.8, 8).is_err());
        assert_eq!(live(), 2);
        cancel_solid_analysis(a);
        let c = start(id);
        cancel_solid_analysis(b);
        cancel_solid_analysis(c);
        delete(id);
        assert!(start_solid_analysis(id, 0.6, 0.8, 8).is_err());
        assert_eq!(live(), 0);
    }
}
