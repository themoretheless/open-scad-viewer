//! Raw-buffer handle registry for mesh analysis results (BVH, semantic edges).
//! Results outlive the call that produced them so the host can copy typed
//! views out of linear memory; `free` consumes the handle exactly once.

pub struct SolidAnalysis {
    mesh: crate::mesh::RenderMesh,
    bvh: mesh_query::MeshBvh,
    edges: mesh_topology::edges::SemanticEdges,
}

#[derive(Clone)]
struct ZRigidAnalysisTemplate {
    origin_id: u32,
    normal_bits: u64,
    edge_bits: u64,
    rot_xy: [f64; 4],
    trans: [f64; 3],
    mesh: crate::mesh::RenderMesh,
    edges: mesh_topology::edges::SemanticEdges,
}

thread_local! {
    static Z_RIGID_TEMPLATE: std::cell::RefCell<Option<ZRigidAnalysisTemplate>> =
        const { std::cell::RefCell::new(None) };
}

fn transform_render_mesh_from_template(
    tpl: &ZRigidAnalysisTemplate,
    target: &crate::mesh::ZRigidProvenance,
) -> crate::mesh::RenderMesh {
    // Relative rotation: R_rel = R_target * R_tpl^T
    let r00 = target.rot_xy[0] * tpl.rot_xy[0] + target.rot_xy[1] * tpl.rot_xy[1];
    let r01 = target.rot_xy[0] * tpl.rot_xy[2] + target.rot_xy[1] * tpl.rot_xy[3];
    let r10 = target.rot_xy[2] * tpl.rot_xy[0] + target.rot_xy[3] * tpl.rot_xy[1];
    let r11 = target.rot_xy[2] * tpl.rot_xy[2] + target.rot_xy[3] * tpl.rot_xy[3];
    let tx = target.trans[0] - (r00 * tpl.trans[0] + r01 * tpl.trans[1]);
    let ty = target.trans[1] - (r10 * tpl.trans[0] + r11 * tpl.trans[1]);
    let tz = target.trans[2] - tpl.trans[2];
    let mut vertices = Vec::with_capacity(tpl.mesh.vertices.len());
    for v in tpl.mesh.vertices.chunks_exact(6) {
        let px = v[0] as f64;
        let py = v[1] as f64;
        let pz = v[2] as f64;
        let nx = v[3] as f64;
        let ny = v[4] as f64;
        let nz = v[5];
        vertices.extend([
            (r00 * px + r01 * py + tx) as f32,
            (r10 * px + r11 * py + ty) as f32,
            (pz + tz) as f32,
            (r00 * nx + r01 * ny) as f32,
            (r10 * nx + r11 * ny) as f32,
            nz,
        ]);
    }
    crate::mesh::RenderMesh {
        vertices,
        indices: tpl.mesh.indices.clone(),
        merge_from: tpl.mesh.merge_from.clone(),
        merge_to: tpl.mesh.merge_to.clone(),
        face_ids: tpl.mesh.face_ids.clone(),
    }
}

/// Analyze the same f32 display buffers the host publishes, while they are
/// still in the kernel. Merge pairs preserve the source solid's topology.
pub fn analyze_solid(
    id: u32,
    normal_cosine: f64,
    edge_cosine: f64,
    leaf_size: usize,
) -> crate::Result<SolidAnalysis> {
    let prov = crate::mesh::solid_z_rigid_provenance(id)?;
    let normal_bits = normal_cosine.to_bits();
    let edge_bits = edge_cosine.to_bits();
    if let Some(target) = prov {
        let cached = Z_RIGID_TEMPLATE.with(|slot| {
            slot.borrow()
                .as_ref()
                .filter(|tpl| {
                    tpl.origin_id == target.origin_id
                        && tpl.normal_bits == normal_bits
                        && tpl.edge_bits == edge_bits
                })
                .map(|tpl| {
                    (
                        transform_render_mesh_from_template(tpl, &target),
                        mesh_topology::edges::SemanticEdges {
                            indices: tpl.edges.indices.clone(),
                            diagnostics: tpl.edges.diagnostics.clone(),
                        },
                    )
                })
        });
        if let Some((mesh, edges)) = cached {
            let bvh = mesh_query::build_mesh_bvh(&mesh.vertices, &mesh.indices, 6, leaf_size);
            return Ok(SolidAnalysis { mesh, bvh, edges });
        }
    }
    let mesh = crate::mesh::render_buffers(id, normal_cosine)?;
    let bvh = mesh_query::build_mesh_bvh(&mesh.vertices, &mesh.indices, 6, leaf_size);
    let edges = mesh_topology::edges::extract_semantic_edges(
        &mesh.vertices,
        &mesh.indices,
        &mesh.merge_from,
        &mesh.merge_to,
        false,
        edge_cosine,
    );
    if let Some(target) = prov {
        Z_RIGID_TEMPLATE.with(|slot| {
            *slot.borrow_mut() = Some(ZRigidAnalysisTemplate {
                origin_id: target.origin_id,
                normal_bits,
                edge_bits,
                rot_xy: target.rot_xy,
                trans: target.trans,
                mesh: mesh.clone(),
                edges: mesh_topology::edges::SemanticEdges {
                    indices: edges.indices.clone(),
                    diagnostics: edges.diagnostics.clone(),
                },
            });
        });
    }
    Ok(SolidAnalysis { mesh, bvh, edges })
}

// Jobs own their display buffers and pinned BVH future. No linear-memory view
// or source-solid borrow survives a step on the host. IDs never get recycled:
// cancelling a stale job cannot drop a newer one.
type AnalysisFuture = std::pin::Pin<Box<dyn std::future::Future<Output = SolidAnalysis>>>;
#[derive(Default)]
struct AnalysisJobs {
    next: u32,
    jobs: [Option<(u32, AnalysisFuture)>; MAX_ANALYSIS_JOBS],
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
        let slot_idx = jobs
            .jobs
            .iter()
            .position(|slot| slot.is_none())
            .ok_or_else(|| crate::input("Solid analysis job budget exceeded"))?;
        let next = jobs
            .next
            .checked_add(1)
            .ok_or_else(|| crate::input("Solid analysis job IDs exhausted"))?;
        crate::mesh::check_analysis_triangle_budget(id, MAX_ANALYSIS_TRIANGLES)?;
        let prov = crate::mesh::solid_z_rigid_provenance(id)?;
        let normal_bits = normal_cosine.to_bits();
        let edge_bits = edge_cosine.to_bits();
        let cached = prov.and_then(|target| {
            Z_RIGID_TEMPLATE.with(|slot| {
                slot.borrow()
                    .as_ref()
                    .filter(|tpl| {
                        tpl.origin_id == target.origin_id
                            && tpl.normal_bits == normal_bits
                            && tpl.edge_bits == edge_bits
                    })
                    .map(|tpl| {
                        (
                            transform_render_mesh_from_template(tpl, &target),
                            mesh_topology::edges::SemanticEdges {
                                indices: tpl.edges.indices.clone(),
                                diagnostics: tpl.edges.diagnostics.clone(),
                            },
                        )
                    })
            })
        });
        let (mesh, cached_edges) = match cached {
            Some((mesh, edges)) => (mesh, Some(edges)),
            None => (crate::mesh::render_buffers(id, normal_cosine)?, None),
        };
        jobs.jobs[slot_idx] = Some((
            next,
            Box::pin(async move {
                let bvh = mesh_query::build_mesh_bvh_cooperative(
                    &mesh.vertices,
                    &mesh.indices,
                    6,
                    leaf_size,
                )
                .await;
                if let Some(edges) = cached_edges {
                    return SolidAnalysis { mesh, bvh, edges };
                }
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
                let edges = mesh_topology::edges::extract_semantic_edges_cooperative(
                    &mesh.vertices,
                    &mesh.indices,
                    &mesh.merge_from,
                    &mesh.merge_to,
                    false,
                    edge_cosine,
                )
                .await;
                if let Some(target) = prov {
                    Z_RIGID_TEMPLATE.with(|slot| {
                        *slot.borrow_mut() = Some(ZRigidAnalysisTemplate {
                            origin_id: target.origin_id,
                            normal_bits,
                            edge_bits,
                            rot_xy: target.rot_xy,
                            trans: target.trans,
                            mesh: mesh.clone(),
                            edges: mesh_topology::edges::SemanticEdges {
                                indices: edges.indices.clone(),
                                diagnostics: edges.diagnostics.clone(),
                            },
                        });
                    });
                }
                SolidAnalysis { mesh, bvh, edges }
            }),
        ));
        jobs.next = next;
        Ok(next)
    })
}

/// Zero means pending; a nonzero result is an ordinary owned array-result
/// handle. Completion consumes the job. Unknown/completed/cancelled jobs refuse.
pub fn step_solid_analysis(id: u32) -> crate::Result<usize> {
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        let slot_idx = jobs
            .jobs
            .iter()
            .position(|slot| matches!(slot, Some((job_id, _)) if *job_id == id))
            .ok_or_else(|| crate::input("Unknown or completed solid analysis job"))?;
        let future = &mut jobs.jobs[slot_idx].as_mut().unwrap().1;
        let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut cx) {
            std::task::Poll::Pending => Ok(0),
            std::task::Poll::Ready(result) => {
                jobs.jobs[slot_idx] = None;
                Ok(store(AnalysisBuffers::Solid(result)))
            }
        }
    })
}

/// Idempotent cancellation drops all unfinished scratch and owned input data.
pub fn cancel_solid_analysis(id: u32) {
    JOBS.with(|jobs| {
        let mut jobs = jobs.borrow_mut();
        for slot in &mut jobs.jobs {
            if matches!(slot, Some((job_id, _)) if *job_id == id) {
                *slot = None;
            }
        }
    });
}

pub enum AnalysisBuffers {
    Transparency(crate::transparent_bsp::Buffers),
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
    /// manifold-core check report: flat u32 edge pairs / index lists plus
    /// summary counts.
    ManifoldCheck {
        boundary_edges: Vec<u32>,
        non_manifold_edges: Vec<u32>,
        orientation_edges: Vec<u32>,
        degenerate_triangles: Vec<u32>,
        non_manifold_vertices: Vec<u32>,
        isolated_vertices: Vec<u32>,
        /// [vertex_count, triangle_count, component_count, flags]
        /// flags bit 0: strict manifold; bit 1: manifold with boundary.
        summary: [u32; 4],
    },
    /// manifold-core repair output mesh plus stats
    /// [welded_vertices, removed_degenerates, flipped_triangles, residual_flags,
    ///  split_vertices, filled_holes, filled_triangles, mode].
    ManifoldRepair {
        positions: Vec<f64>,
        indices: Vec<u32>,
        stats: [u32; 8],
    },
    /// manifold-core metrics: flat per-component records plus summary counts.
    ManifoldMetrics {
        /// 6 u32 per component: [triangles, vertices, edges, boundary_edges,
        /// euler_characteristic (i32 bits), genus + 1 (0 = undefined)].
        component_stats: Vec<u32>,
        /// 2 f64 per component: [signed_volume, surface_area].
        component_floats: Vec<f64>,
        /// Whole-mesh [signed_volume, surface_area].
        floats: Vec<f64>,
        /// [vertex_count, triangle_count, edge_count, euler_characteristic
        /// (i32 bits), component_count, flags, 0, 0]
        /// flags bit 0: watertight.
        summary: [u32; 8],
    },
    /// manifold-ops boolean output mesh plus kernel stats.
    ManifoldBoolean {
        positions: Vec<f64>,
        indices: Vec<u32>,
        /// [tolerance_mm].
        floats: Vec<f64>,
        /// [op, operand_a_repaired, operand_b_repaired, fragments, work,
        ///  input_triangles_a, input_triangles_b, flags]
        /// flags bit 0: strict manifold output (always set on success).
        stats: [u32; 8],
    },
    SmoothTriangleList {
        positions: Vec<f32>,
        normals: Vec<f32>,
    },
    VertexNormals {
        vertices: Vec<f32>,
    },
    ExpandedDisplayMesh {
        vertices: Vec<f32>,
        indices: Vec<u32>,
        floats: Vec<f64>,
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
/// SurfaceGroups: 0/1 ids (u32).
/// Placement: 0/1 positions (f64), 2/3 indices (u32).
/// Edges: 0/1 indices (u32); slots 2..=5 return the diagnostic counters
/// (boundary, crease, non-manifold, degenerate) directly.
/// Render: 0/1 vertices (f32, stride 6), 2/3 indices, 4/5 merge-from,
/// 6/7 merge-to, 8/9 face ids (all u32).
/// Solid: Render slots 0..=9, 10/11 BVH bounds (f32), 12/13 BVH nodes,
/// 14/15 BVH triangles, 16/17 edge indices (u32), 18..=21 diagnostic counters.
fn ptr_len<T>(slice: &[T]) -> [usize; 2] {
    [slice.as_ptr() as usize, slice.len()]
}

fn pair_slot(pairs: &[[usize; 2]], slot: u32) -> Option<usize> {
    pairs.as_flattened().get(slot as usize).copied()
}

pub fn field(handle: usize, slot: u32) -> usize {
    RESULTS.with(|results| {
        let results = results.borrow();
        let Some(Some(result)) = handle.checked_sub(1).and_then(|i| results.get(i)) else {
            return 0;
        };
        match result {
            AnalysisBuffers::Transparency(t) => pair_slot(
                &[
                    ptr_len(&t.planes),
                    ptr_len(&t.links),
                    ptr_len(&t.owners),
                    ptr_len(&t.vertices),
                ],
                slot,
            )
            .or_else(|| t.summary.get(slot.checked_sub(8)? as usize).copied())
            .unwrap_or(0),
            AnalysisBuffers::SurfaceGroups { ids } => pair_slot(&[ptr_len(ids)], slot).unwrap_or(0),
            AnalysisBuffers::Bytes { bytes } => pair_slot(&[ptr_len(bytes)], slot).unwrap_or(0),
            AnalysisBuffers::Export {
                positions,
                indices,
                normals,
            } => pair_slot(
                &[ptr_len(positions), ptr_len(indices), ptr_len(normals)],
                slot,
            )
            .unwrap_or(0),
            AnalysisBuffers::Placement { positions, indices } => {
                pair_slot(&[ptr_len(positions), ptr_len(indices)], slot).unwrap_or(0)
            }
            AnalysisBuffers::Bvh {
                bounds,
                nodes,
                triangles,
            } => pair_slot(
                &[ptr_len(bounds), ptr_len(nodes), ptr_len(triangles)],
                slot,
            )
            .unwrap_or(0),
            AnalysisBuffers::Edges {
                indices,
                diagnostics,
            } => pair_slot(&[ptr_len(indices)], slot)
                .or_else(|| {
                    diagnostics
                        .get(slot.checked_sub(2)? as usize)
                        .map(|&v| v as usize)
                })
                .unwrap_or(0),
            AnalysisBuffers::Render(mesh) => render_field(mesh, slot),
            AnalysisBuffers::ManifoldCheck {
                boundary_edges,
                non_manifold_edges,
                orientation_edges,
                degenerate_triangles,
                non_manifold_vertices,
                isolated_vertices,
                summary,
            } => pair_slot(
                &[
                    ptr_len(boundary_edges),
                    ptr_len(non_manifold_edges),
                    ptr_len(orientation_edges),
                    ptr_len(degenerate_triangles),
                    ptr_len(non_manifold_vertices),
                    ptr_len(isolated_vertices),
                ],
                slot,
            )
            .or_else(|| {
                summary
                    .get(slot.checked_sub(12)? as usize)
                    .map(|&v| v as usize)
            })
            .unwrap_or(0),
            AnalysisBuffers::ManifoldRepair {
                positions,
                indices,
                stats,
            } => pair_slot(&[ptr_len(positions), ptr_len(indices)], slot)
                .or_else(|| {
                    stats
                        .get(slot.checked_sub(4)? as usize)
                        .map(|&v| v as usize)
                })
                .unwrap_or(0),
            AnalysisBuffers::ManifoldMetrics {
                component_stats,
                component_floats,
                floats,
                summary,
            } => pair_slot(
                &[
                    ptr_len(component_stats),
                    ptr_len(component_floats),
                    ptr_len(floats),
                ],
                slot,
            )
            .or_else(|| {
                summary
                    .get(slot.checked_sub(6)? as usize)
                    .map(|&v| v as usize)
            })
            .unwrap_or(0),
            AnalysisBuffers::ManifoldBoolean {
                positions,
                indices,
                floats,
                stats,
            } => pair_slot(&[ptr_len(positions), ptr_len(indices), ptr_len(floats)], slot)
                .or_else(|| {
                    stats
                        .get(slot.checked_sub(6)? as usize)
                        .map(|&v| v as usize)
                })
                .unwrap_or(0),
            AnalysisBuffers::Solid(result) => solid_field(result, slot),
            AnalysisBuffers::SmoothTriangleList { positions, normals } => {
                pair_slot(&[ptr_len(positions), ptr_len(normals)], slot).unwrap_or(0)
            }
            AnalysisBuffers::VertexNormals { vertices } => {
                pair_slot(&[ptr_len(vertices)], slot).unwrap_or(0)
            }
            AnalysisBuffers::ExpandedDisplayMesh {
                vertices,
                indices,
                floats,
            } => pair_slot(&[ptr_len(vertices), ptr_len(indices), ptr_len(floats)], slot)
                .unwrap_or(0),
        }
    })
}

fn solid_field(result: &SolidAnalysis, slot: u32) -> usize {
    match slot {
        0..=9 => render_field(&result.mesh, slot),
        10..=17 => pair_slot(
            &[
                ptr_len(&result.bvh.bounds),
                ptr_len(&result.bvh.nodes),
                ptr_len(&result.bvh.triangles),
                ptr_len(&result.edges.indices),
            ],
            slot - 10,
        )
        .unwrap_or(0),
        18 => result.edges.diagnostics.boundary as usize,
        19 => result.edges.diagnostics.crease as usize,
        20 => result.edges.diagnostics.non_manifold as usize,
        21 => result.edges.diagnostics.degenerate as usize,
        _ => 0,
    }
}

fn render_field(mesh: &crate::mesh::RenderMesh, slot: u32) -> usize {
    pair_slot(
        &[
            ptr_len(&mesh.vertices),
            ptr_len(&mesh.indices),
            ptr_len(&mesh.merge_from),
            ptr_len(&mesh.merge_to),
            ptr_len(&mesh.face_ids),
        ],
        slot,
    )
    .unwrap_or(0)
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
pub fn live_result_count() -> usize {
    RESULTS.with(|results| {
        results
            .borrow()
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    })
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
        Z_RIGID_TEMPLATE.with(|slot| *slot.borrow_mut() = None);
        start_solid_analysis(id, 0.6, 0.8, 8).unwrap()
    }
    fn live() -> usize {
        JOBS.with(|jobs| jobs.borrow().jobs.iter().flatten().count())
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
    fn cancellation_at_every_cooperative_step_leaves_no_jobs_or_results() {
        let id = sphere();
        let result_baseline = live_result_count();
        let completed = start(id);
        let mut steps = 0;
        let handle = loop {
            steps += 1;
            assert!(
                steps < 10_000,
                "analysis did not finish within its step budget"
            );
            let result = step_solid_analysis(completed).unwrap();
            if result != 0 {
                break result;
            }
        };
        assert!(steps > 2, "test requires distinct BVH and edge phases");
        free(handle);
        assert_eq!(live_result_count(), result_baseline);

        // Every prefix ends before the completing poll. This covers admission,
        // every BVH slice, the inter-phase yield and every edge-extraction slice.
        for completed_steps in 0..steps {
            let job = start(id);
            for _ in 0..completed_steps {
                assert_eq!(step_solid_analysis(job).unwrap(), 0);
            }
            assert_eq!(live(), 1);
            cancel_solid_analysis(job);
            cancel_solid_analysis(job); // idempotent, including stale IDs
            assert_eq!(live(), 0, "cancel after {completed_steps} steps");
            assert_eq!(live_result_count(), result_baseline);
            assert!(step_solid_analysis(job).is_err());
            let next = start(id);
            assert!(next > job, "job IDs must not be recycled");
            cancel_solid_analysis(job);
            assert_eq!(live(), 1, "stale cancellation removed a newer job");
            cancel_solid_analysis(next);
            assert_eq!(live(), 0);
            assert_eq!(live_result_count(), result_baseline);
        }
        eprintln!("verified cancellation at all {steps} cooperative-step prefixes");
        delete(id);
    }

    #[test]
    fn cancellation_after_bvh_drops_edge_scratch_and_releases_admission() {
        use std::future::Future;
        let id = sphere();
        let mesh = crate::mesh::render_buffers(id, 0.6).unwrap();
        let mut bvh = std::pin::pin!(mesh_query::build_mesh_bvh_cooperative(
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
            let actual_bits: Vec<_> = actual.bvh.bounds.iter().map(|x| x.to_bits()).collect();
            let expected_bits: Vec<_> = expected.bvh.bounds.iter().map(|x| x.to_bits()).collect();
            assert_eq!(actual_bits, expected_bits);
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
