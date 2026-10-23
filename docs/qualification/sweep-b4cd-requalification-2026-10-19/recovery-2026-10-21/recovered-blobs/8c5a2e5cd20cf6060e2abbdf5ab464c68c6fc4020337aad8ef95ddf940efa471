//! Multiple boundary-connected fibers, with original roots queried once.
use super::{automatic_fiber, invalid, unproven};
use crate::{Model, edge_partition, face_crossings};
use nurbs_core::Result;
use std::collections::BTreeSet;

pub struct FiberBranch {
    pub endpoints: [usize; 2],
    pub edges: Vec<usize>,
    pub root_fibers: usize,
}
pub struct FiberBranchRefusal {
    pub endpoints: [usize; 2],
    pub code: &'static str,
    pub message: String,
}
pub struct FiberBranches {
    pub model: Model,
    pub faces: Vec<usize>,
    pub branches: Vec<FiberBranch>,
    pub boundary_coverage: Vec<(usize, face_crossings::Report)>,
    /// Includes reported proposal work and a conservative full reservation for
    /// construction refusals whose internal counters are unavailable.
    pub charged_cells: usize,
    pub charged_exact_work: u64,
    pub charged_evaluations: usize,
    pub rejected_pairs: usize,
    pub refusals: Vec<FiberBranchRefusal>,
}

/// Match up to sixteen original transverse boundary roots by constructing and
/// proving complete candidate fibers. Pair order never grants connectivity.
/// Each actual trim partition must pass; a refusal returns no partial model.
/// All sectors remain. Closed hidden branches and source-hole cuts are separate.
pub fn partition_fiber_branches_at_face(
    source: &Model,
    face: usize,
    cutter: &Model,
    cutter_face: usize,
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<FiberBranches> {
    source.validate()?;
    cutter.validate()?;
    if !(32..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid("Need bounded multi-branch cut work"));
    }
    let original_box = automatic_fiber::original_parameter_box(source, face, cutter, cutter_face)?;
    let ring = &source.loops[source.faces[face].outer].coedges;
    let edges = ring.iter().map(|c| c.edge).collect::<BTreeSet<_>>();
    if !(3..=256).contains(&ring.len()) || edges.len() != ring.len() {
        return Err(unproven("Need distinct bounded original outer edges"));
    }
    let mut model = source.clone();
    let mut cells = 0;
    let mut exact = 0;
    let mut evaluations = 0;
    let mut rejected = 0;
    let mut refusals = Vec::new();
    let mut coverage = Vec::new();
    let mut roots = Vec::new();
    for edge in edges {
        if cells == max_cells {
            return Err(unproven("Multi-branch original-root work exhausted"));
        }
        let split = edge_partition::partition_all_at_face(
            &model,
            edge,
            cutter,
            cutter_face,
            tolerance_uv,
            max_cells - cells,
        )?;
        cells += split.cells;
        for (&index, &vertex) in split.coverage.eligible.iter().zip(&split.vertices) {
            let root = &split.coverage.roots[index].root;
            roots.push((
                vertex,
                root.position_bounds,
                [root.parameter_box[1], root.parameter_box[2]],
            ));
        }
        if roots.len() > 16 {
            return Err(unproven(
                "More than sixteen roots need a larger branch planner",
            ));
        }
        coverage.push((edge, split.coverage));
        model = split.model;
    }
    if roots.len() < 2 || roots.len() % 2 != 0 {
        return Err(unproven("Need an even number of transverse boundary roots"));
    }
    let mut faces = vec![face];
    let mut branches = Vec::new();
    while !roots.is_empty() {
        let mut accepted = None;
        for other in 1..roots.len() {
            let pair = [roots[0], roots[other]];
            let owners = faces
                .iter()
                .copied()
                .filter(|&f| {
                    let ring = &model.loops[model.faces[f].outer].coedges;
                    pair.iter().all(|r| {
                        ring.iter()
                            .any(|c| model.edges[c.edge].vertices[usize::from(c.reversed)] == r.0)
                    })
                })
                .collect::<Vec<_>>();
            if owners.len() != 1 {
                continue;
            }
            let remaining_pairs = roots.len() - other;
            let offered_cells = (max_cells - cells) / remaining_pairs;
            let offered_exact = (max_exact - exact) / remaining_pairs as u64;
            let offered_evaluations = (4096 - evaluations) / remaining_pairs;
            if offered_cells < 64 || offered_evaluations == 0 {
                return Err(unproven("Multi-branch candidate work exhausted"));
            }
            let mut used = 0;
            let mut construction_started = false;
            let attempt = (|| {
                let mut box4 = original_box;
                let owned = pair.map(|r| (r.0, r.1));
                let cutter_uv = pair.map(|r| r.2);
                let axis = automatic_fiber::qualify_automatic_search(
                    &model,
                    owners[0],
                    cutter,
                    cutter_face,
                    &owned,
                    &cutter_uv,
                    &mut box4,
                    &mut used,
                    offered_cells,
                )?;
                construction_started = true;
                let mut out = automatic_fiber::partition_fiber_prebound(
                    &model,
                    owners[0],
                    cutter,
                    cutter_face,
                    &owned,
                    box4,
                    axis,
                    tolerance_uv,
                    offered_cells - used,
                    offered_exact,
                    offered_evaluations,
                )?;
                out.cells += used;
                Ok::<_, nurbs_core::Error>(out)
            })();
            match attempt {
                Ok(out) => {
                    cells += out.cells;
                    exact += out.partition.exact_work;
                    evaluations += out.evaluations;
                    accepted = Some((other, owners[0], out));
                    break;
                }
                Err(error) => {
                    // Proposal proofs expose all their rectangle work and use
                    // no trace evaluations or trim exact work. A construction
                    // refusal can stop inside those gates: reserve the complete
                    // offered counters there rather than hide unknown work.
                    if construction_started {
                        cells += offered_cells;
                        exact += offered_exact;
                        evaluations += offered_evaluations;
                    } else {
                        cells += if error.code == "BREP_FACE_CUT_UNPROVEN" {
                            used
                        } else {
                            offered_cells
                        };
                    }
                    rejected += 1;
                    refusals.push(FiberBranchRefusal {
                        endpoints: pair.map(|r| r.0),
                        code: error.code,
                        message: error.message,
                    });
                }
            }
        }
        let (other, owner, out) = accepted.ok_or_else(|| {
            unproven(&format!(
                "No complete owned fiber pairs original root vertex {}: {}",
                roots[0].0,
                refusals
                    .iter()
                    .map(|r| format!("{:?}: {}", r.endpoints, r.message))
                    .collect::<Vec<_>>()
                    .join("; "),
            ))
        })?;
        faces.retain(|&f| f != owner);
        faces.extend(out.partition.faces);
        branches.push(FiberBranch {
            endpoints: out.endpoints,
            edges: out.partition.edges,
            root_fibers: out.root_fibers,
        });
        model = out.partition.model;
        roots.remove(other);
        roots.remove(0);
    }
    model.refresh_change_set(&[source]);
    Ok(FiberBranches {
        model,
        faces,
        branches,
        boundary_coverage: coverage,
        charged_cells: cells,
        charged_exact_work: exact,
        charged_evaluations: evaluations,
        rejected_pairs: rejected,
        refusals,
    })
}
