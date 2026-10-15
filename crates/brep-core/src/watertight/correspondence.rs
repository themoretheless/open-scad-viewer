//! Whole original free boundaries with independently checked parameter maps.
use super::BoundaryAddress;
use crate::{Model, Result};
use nurbs_core::{
    Error,
    curve_surface_agreement::{self, CorrespondenceReport, Status},
};
#[derive(Clone, Debug)]
pub struct Correspondence {
    /// Source/target coedge traversal fractions, not edge-curve fractions.
    pub nodes: Vec<[f64; 2]>,
    pub agreements: Vec<CorrespondenceReport>,
    pub cells: usize,
    pub within_tolerance: bool,
    /// Each lift is within this distance of the same source edge; the lift to
    /// lift distance is bounded by twice this value.
    pub edge_tolerance_mm: f64,
}
fn invalid() -> Error {
    Error::new(
        "BREP_INVALID_INPUT",
        "Correspondence needs distinct free boundaries, equal tolerances, monotone full traversal and bounded work",
    )
}
fn select(
    models: &[Model],
    address: BoundaryAddress,
) -> Result<(&Model, &crate::Coedge, &crate::Face, bool)> {
    let m = models.get(address.model).ok_or_else(invalid)?;
    m.validate_boundary_diagnostic_inputs()?;
    let f = m.faces.get(address.face).ok_or_else(invalid)?;
    if f.outer != address.wire && !f.holes.contains(&address.wire) {
        return Err(invalid());
    }
    let c = m
        .loops
        .get(address.wire)
        .and_then(|l| l.coedges.get(address.coedge))
        .ok_or_else(invalid)?;
    if m.edges[c.edge].degenerate
        || m.loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|other| other.edge == c.edge)
            .count()
            != 1
    {
        return Err(invalid());
    }
    let reversed = m
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .find(|u| u.face == address.face)
        .ok_or_else(invalid)?
        .reversed;
    Ok((m, c, f, reversed))
}
/// Covers the complete original source edge and both original face lifts.
/// Supplied nodes form a continuous monotone piecewise affine relation. Whole
/// boundary endpoints and opposite face traversal are mandatory. Inputs are
/// immutable; no topology or global solid certificate is authored. A failed
/// relation does not disprove the existence of another nonlinear relation.
pub fn inspect_boundary_correspondence(
    models: &[Model],
    addresses: [BoundaryAddress; 2],
    nodes: &[[f64; 2]],
    max_cells: usize,
) -> Result<Correspondence> {
    if addresses[0].model == addresses[1].model
        || !(1..=1000000).contains(&max_cells)
        || !(2..=1025).contains(&nodes.len())
    {
        return Err(invalid());
    }
    let (sm, sc, sf, sr) = select(models, addresses[0])?;
    let (tm, tc, tf, tr) = select(models, addresses[1])?;
    let target_increases = sr != tr;
    if sm.tolerance_mm != tm.tolerance_mm
        || nodes[0] != [0., if target_increases { 0. } else { 1. }]
        || *nodes.last().unwrap() != [1., if target_increases { 1. } else { 0. }]
        || !nodes
            .iter()
            .flatten()
            .all(|x| x.is_finite() && (0. ..=1.).contains(x))
        || !nodes.windows(2).all(|w| {
            w[1][0] > w[0][0] && w[1][1] != w[0][1] && (w[1][1] > w[0][1]) == target_increases
        })
    {
        return Err(invalid());
    }
    let source_nodes: Vec<_> = nodes.iter().map(|n| [n[0], n[0]]).collect();
    let target_nodes = nodes.to_vec();
    let mut out = Correspondence {
        nodes: nodes.to_vec(),
        agreements: vec![],
        cells: 0,
        within_tolerance: false,
        edge_tolerance_mm: sm.tolerance_mm,
    };
    for (p, s, map) in [
        (&sc.pcurve, &sf.surface, &source_nodes),
        (&tc.pcurve, &tf.surface, &target_nodes),
    ] {
        if out.cells == max_cells {
            break;
        }
        let proof = curve_surface_agreement::verify_oriented_correspondence(
            &sm.edges[sc.edge].curve,
            p,
            s,
            map,
            sc.reversed,
            sm.tolerance_mm,
            (max_cells - out.cells).min(100000),
        )?;
        out.cells += proof.cells;
        out.agreements.push(proof);
    }
    out.within_tolerance = out.agreements.len() == 2
        && out
            .agreements
            .iter()
            .all(|r| r.status == Status::WithinTolerance);
    Ok(out)
}
fn natural(c: &nurbs_core::curve::Curve, t: f64) -> f64 {
    let [a, b] = c.domain();
    if t == 0. {
        a
    } else if t == 1. {
        b
    } else {
        a + (b - a) * t
    }
}
/// Bounded nearest-point proposals on a target face lift. The search is sampled
/// and never grants admission. The returned relation is independently verified
/// against original curves/surfaces over its complete continuous domain.
/// This handles open full-boundary relations; phase search, partial overlaps,
/// uniqueness among other boundaries and topology sewing remain separate.
pub fn infer_boundary_correspondence(
    models: &[Model],
    addresses: [BoundaryAddress; 2],
    segments: usize,
    max_cells: usize,
) -> Result<Correspondence> {
    if !(1..=1024).contains(&segments)
        || !(1..=1000000).contains(&max_cells)
        || addresses[0].model == addresses[1].model
    {
        return Err(invalid());
    }
    let (sm, sc, _, sr) = select(models, addresses[0])?;
    let (tm, tc, tf, tr) = select(models, addresses[1])?;
    if sm.tolerance_mm != tm.tolerance_mm {
        return Err(invalid());
    }
    let increasing = sr != tr;
    let mut nodes = Vec::with_capacity(segments + 1);
    for i in 0..=segments {
        let x = i as f64 / segments as f64;
        let y = if i == 0 {
            if increasing { 0. } else { 1. }
        } else if i == segments {
            if increasing { 1. } else { 0. }
        } else {
            let edge = &sm.edges[sc.edge].curve;
            let point = edge
                .evaluate(natural(edge, if sc.reversed { 1. - x } else { x }))?
                .point;
            let loss = |t: f64| -> Result<f64> {
                let uv = tc.pcurve.evaluate(natural(&tc.pcurve, t))?.point;
                let q = tf.surface.evaluate(uv[0], uv[1])?.point;
                Ok((0..3).map(|k| (q[k] - point[k]).powi(2)).sum())
            };
            let mut best = 0usize;
            let mut score = f64::INFINITY;
            for j in 0..=128 {
                let value = loss(j as f64 / 128.)?;
                if value < score {
                    score = value;
                    best = j;
                }
            }
            let mut lo = best.saturating_sub(1) as f64 / 128.;
            let mut hi = (best + 1).min(128) as f64 / 128.;
            for _ in 0..40 {
                let a = lo + (hi - lo) / 3.;
                let b = hi - (hi - lo) / 3.;
                if loss(a)? < loss(b)? { hi = b } else { lo = a }
            }
            lo + (hi - lo) * 0.5
        };
        nodes.push([x, y]);
    }
    // Nonmonotone nearest proposals carry no evidence. Return an explicit
    // unresolved full relation rather than treating the failed search as proof
    // that the boundaries do not match.
    if !nodes
        .windows(2)
        .all(|w| w[0][1] != w[1][1] && (w[1][1] > w[0][1]) == increasing)
    {
        return Ok(Correspondence {
            nodes,
            agreements: vec![],
            cells: 0,
            within_tolerance: false,
            edge_tolerance_mm: sm.tolerance_mm,
        });
    }
    inspect_boundary_correspondence(models, addresses, &nodes, max_cells)
}
