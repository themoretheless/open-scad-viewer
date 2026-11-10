//! Connect freshly isolated cutter roots to shared support-face trimming.
use super::{FaceRetention, OwnedCut, RetentionNetwork, invalid, unproven};
use crate::{Model, edge_partition};
use nurbs_core::{Result, curve::Curve};

#[derive(Clone, Copy)]
pub enum RootEnd {
    Vertex(usize),
    /// Root index among retained Inside roots, in original edge order.
    Crossing {
        edge: usize,
        root: usize,
    },
}
pub struct RootedCut {
    pub endpoints: [RootEnd; 2],
    pub curves: Vec<Curve>,
}
pub struct RootedFace<'a> {
    pub face: usize,
    pub cuts: &'a [RootedCut],
    pub keep_uv: &'a [[f64; 2]],
}
pub struct RootedRetention {
    pub plan: RetentionNetwork,
    /// Original edge and its freshly recomputed complete cutter coverage.
    pub roots: Vec<(usize, crate::face_crossings::Report)>,
    /// Original edge, retained root ordinal, shared authored vertex.
    pub root_vertices: Vec<(usize, usize, usize)>,
    /// Includes all root subdivision and final trim-network work.
    pub cells: usize,
}
/// Bind cut ends to actual trimmed-cutter roots, subdividing each shared edge
/// once for every incident use, then project and classify the face cuts. Root
/// reports and vertices cannot be supplied by the caller. Both stages are
/// private and share one cell ledger. Curve generation and closed replacement
/// admission remain separate; no nearest-point welding is performed.
pub fn plan_rooted_retention(
    source: &Model,
    cutter: &Model,
    cutter_face: usize,
    requests: &[RootedFace<'_>],
    tolerance_uv: f64,
    max_cells: usize,
    max_exact: u64,
    max_domain_cells: usize,
) -> Result<RootedRetention> {
    source.validate()?;
    cutter.validate()?;
    if !(1..=256).contains(&requests.len())
        || !(1..=100000).contains(&max_cells)
        || max_exact > 1000000
        || !(1..=100000).contains(&max_domain_cells)
        || cutter_face >= cutter.faces.len()
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
    {
        return Err(invalid(
            "Need original support requests, a cutter face and bounded shared work",
        ));
    }
    let mut edges = std::collections::BTreeSet::new();
    let mut faces = std::collections::BTreeSet::new();
    for request in requests {
        if request.face >= source.faces.len()
            || !faces.insert(request.face)
            || !(1..=256).contains(&request.cuts.len())
        {
            return Err(invalid("Need distinct original faces and bounded cuts"));
        }
        for end in request.cuts.iter().flat_map(|c| c.endpoints) {
            match end {
                RootEnd::Vertex(v) if v >= source.vertices.len() => {
                    return Err(invalid("Missing original vertex"));
                }
                RootEnd::Crossing { edge, .. } => {
                    if edge >= source.edges.len() {
                        return Err(invalid("Missing original root edge"));
                    }
                    edges.insert(edge);
                }
                _ => {}
            }
        }
    }
    if edges.len() > 256 {
        return Err(invalid("Root-edge work set exceeds its bound"));
    }
    let mut model = source.clone();
    let mut roots = Vec::new();
    let mut root_vertices = Vec::new();
    let mut cells = 0;
    for edge in edges {
        if cells == max_cells {
            return Err(unproven("Shared root subdivision work exhausted"));
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
        root_vertices.extend(
            split
                .vertices
                .iter()
                .enumerate()
                .map(|(r, &v)| (edge, r, v)),
        );
        roots.push((edge, split.coverage));
        model = split.model;
    }
    let owned = requests
        .iter()
        .map(|request| {
            request
                .cuts
                .iter()
                .map(|cut| {
                    let mut endpoints = [0; 2];
                    for (i, end) in cut.endpoints.iter().enumerate() {
                        endpoints[i] = match *end {
                            RootEnd::Vertex(v) => v,
                            RootEnd::Crossing { edge, root } => root_vertices
                                .iter()
                                .find(|&&(e, r, _)| e == edge && r == root)
                                .map(|&(_, _, v)| v)
                                .ok_or_else(|| {
                                    invalid("Requested retained cutter root is missing")
                                })?,
                        };
                    }
                    Ok(OwnedCut {
                        endpoints,
                        curves: cut.curves.clone(),
                    })
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let prepared = requests
        .iter()
        .zip(&owned)
        .map(|(r, c)| FaceRetention {
            face: r.face,
            cuts: c,
            keep_uv: r.keep_uv,
        })
        .collect::<Vec<_>>();
    if cells == max_cells {
        return Err(unproven("No remaining face-cut work"));
    }
    let mut plan = super::plan_retention_network(
        &model,
        &prepared,
        tolerance_uv,
        max_cells - cells,
        max_exact,
        max_domain_cells,
    )?;
    cells += plan.cells;
    plan.model.refresh_change_set(&[source]);
    Ok(RootedRetention {
        plan,
        roots,
        root_vertices,
        cells,
    })
}
