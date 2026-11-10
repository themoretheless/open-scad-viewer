//! Pointwise tolerance inclusion of actual new arc rims in the source profile.
//! This is not a proof of exact material subtraction or of filled-region inclusion.
use super::CurvedFullRound;
use crate::{Model, Result, feature_ops::error};
use nurbs_core::trim_domain::curve_cover::{self, Certificate, Limits};

fn original_xy_edge(
    model: &Model,
    edge: usize,
    z: f64,
    reverse: bool,
) -> Option<nurbs_core::curve::Curve> {
    let mut curve = model.edges[edge].curve.clone();
    if curve.control_points.iter().any(|p| p[2] != z) {
        return None;
    }
    for p in &mut curve.control_points {
        p.truncate(2);
    }
    if reverse {
        curve.control_points.reverse();
        curve.weights.reverse();
        // t -> -t is an exact sign change even for non-dyadic knot clocks.
        // No normalizing division or rounded a+b-t reversal supplies authority.
        curve.knots = curve.knots.iter().rev().map(|k| -k).collect();
    }
    Some(curve)
}

pub(super) fn original_profile(
    model: &Model,
    z: f64,
) -> Result<Vec<Vec<nurbs_core::curve::Curve>>> {
    let caps = model
        .faces
        .iter()
        .filter(|f| f.surface.control_points.iter().flatten().all(|p| p[2] == z))
        .collect::<Vec<_>>();
    if caps.len() != 1 {
        return Err(error("Original profile needs exactly one stored top cap"));
    }
    let cap = caps[0];
    std::iter::once(cap.outer)
        .chain(cap.holes.iter().copied())
        .map(|w| {
            model.loops[w]
                .coedges
                .iter()
                .map(|c| {
                    original_xy_edge(model, c.edge, z, c.reversed)
                        .ok_or_else(|| error("Original profile rim leaves its stored top plane"))
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect()
}

pub struct SourceArcCover {
    source: Model,
    result: Model,
    arcs: Vec<(usize, Certificate)>,
    work: [usize; 3],
}
impl SourceArcCover {
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn result(&self) -> &Model {
        &self.result
    }
    pub fn arcs(&self) -> &[(usize, Certificate)] {
        &self.arcs
    }
    pub fn work(&self) -> [usize; 3] {
        self.work
    }
}
impl CurvedFullRound {
    /// Every point of each actual arc rim must be inside the original profile
    /// or within source.tolerance_mm of an original boundary point. Same-height
    /// XY extrusion propagates this side-boundary relation along the axis.
    /// It does NOT establish that all filled result material belongs to source.
    pub fn qualify_source_arc_cover(&self, limits: Limits) -> Result<SourceArcCover> {
        let profile = crate::prism::recognize(self.source())?
            .ok_or_else(|| error("Source arc cover needs the original recognized prism"))?;
        let loops = original_profile(self.source(), profile.z_max)?;
        let mut arcs = Vec::new();
        let mut work = [0usize; 3];
        for (face, _) in self.construction().tangency.envelopes() {
            let f = &self.model().faces[*face];
            let mut candidates = Vec::new();
            for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for c in &self.model().loops[w].coedges {
                    if let Some(edge) = original_xy_edge(self.model(), c.edge, profile.z_max, false)
                    {
                        candidates.push(edge);
                    }
                }
            }
            if candidates.len() != 1 {
                return Err(error("Actual arc face must own one complete top rim"));
            }
            if work[0] >= limits.cells
                || work[1] >= limits.classification_work
                || work[2] >= limits.witness_work
            {
                return Err(error("Source arc cover shared work exhausted"));
            }
            let r = curve_cover::inspect(
                &candidates[0],
                &loops,
                self.source().tolerance_mm,
                Limits {
                    cells: limits.cells - work[0],
                    classification_work: limits.classification_work - work[1],
                    witness_work: limits.witness_work - work[2],
                },
            )?;
            work[0] += r.cells;
            work[1] += r.classification_work;
            work[2] += r.witness_work;
            let c=r.certificate.ok_or_else(||error(format!(
                "Original source arc inclusion remains unproven: face={face}, reason={}, pending={}, work={work:?}",r.reason,r.pending.len())))?;
            arcs.push((*face, c));
        }
        Ok(SourceArcCover {
            source: self.source().clone(),
            result: self.model().clone(),
            arcs,
            work,
        })
    }
}
