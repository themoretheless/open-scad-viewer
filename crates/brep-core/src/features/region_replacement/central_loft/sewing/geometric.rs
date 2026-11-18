//! Actual original-knot normal/tensor joins; raw parameter C2 is not required.
use super::*;
use nurbs_core::{surface_g2_pcurves, surface_join::Decision};
pub struct CornerInternalJoin {
    pub face: usize,
    /// Original face parameter axis (0=U, 1=V), including transposed queries.
    pub axis: usize,
    pub knot: f64,
    pub report: Result<surface_g2_pcurves::SpanJoinReport>,
}
pub struct CornerInternalG2Audit {
    pub joins: Vec<CornerInternalJoin>,
    pub required_joins: usize,
    pub next_join: Option<usize>,
    pub cells: usize,
    pub within_tolerance: bool,
    pub normal_tolerance: f64,
    pub tensor_tolerance: f64,
}
fn transposed(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|j| s.control_points.iter().map(|r| r[j].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|j| s.weights.iter().map(|r| r[j]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
fn inspect(
    model: &Model,
    normal: f64,
    tensor: f64,
    max_cells: usize,
) -> Result<CornerInternalG2Audit> {
    require(
        normal.is_finite()
            && (0. ..=2.).contains(&normal)
            && tensor.is_finite()
            && tensor >= 0.
            && max_cells <= 100000
            && model.faces.len() <= 256,
        "Bound original corner G2 network tolerances and work",
    )?;
    model.validate()?;
    let mut required = Vec::new();
    for (face, f) in model.faces.iter().enumerate() {
        let s = &f.surface;
        for (axis, p, knots, n) in [
            (0, s.degree_u, &s.knots_u, s.control_points.len()),
            (1, s.degree_v, &s.knots_v, s.control_points[0].len()),
        ] {
            let mut nodes: Vec<_> = knots
                .iter()
                .copied()
                .filter(|&k| knots[p] < k && k < knots[n])
                .collect();
            nodes.dedup();
            required.extend(nodes.into_iter().map(|k| (face, axis, k)));
        }
    }
    let mut out = CornerInternalG2Audit {
        required_joins: required.len(),
        joins: vec![],
        next_join: None,
        cells: 0,
        within_tolerance: false,
        normal_tolerance: normal,
        tensor_tolerance: tensor,
    };
    for (i, &(face, axis, knot)) in required.iter().enumerate() {
        if out.cells >= max_cells {
            out.next_join = Some(i);
            break;
        }
        let surface = &model.faces[face].surface;
        let transpose;
        let original = if axis == 0 {
            surface
        } else {
            transpose = transposed(surface);
            &transpose
        };
        // Each later original knot retains an explicit share of the budget.
        // Unsupported bases are recorded as errors, never skipped as smooth.
        let report = surface_g2_pcurves::inspect_original_geometric_span_join(
            original,
            knot,
            normal,
            tensor,
            (max_cells - out.cells) / (required.len() - i),
        );
        if let Ok(r) = &report {
            out.cells += r.cells;
        }
        out.joins.push(CornerInternalJoin {
            face,
            axis,
            knot,
            report,
        });
    }
    out.within_tolerance = !model.faces.is_empty()
        && out.joins.len() == required.len()
        && out.joins.iter().all(|j| {
            j.report.as_ref().is_ok_and(|r| {
                r.decision == Decision::WithinTolerance && r.regularity_proven && r.pending == 0
            })
        });
    Ok(out)
}
/// Only internal original-knot G2. Exterior coedges, pole limits, whole-face
/// curvature, embedding, material, radius laws and thickness remain separate.
pub struct QualifiedCornerInternalG2<'a> {
    candidate: &'a SewnCornerCandidate,
    report: CornerInternalG2Audit,
}
impl SewnCornerCandidate {
    pub fn inspect_internal_g2(
        &self,
        normal: f64,
        tensor: f64,
        max_cells: usize,
    ) -> Result<CornerInternalG2Audit> {
        inspect(&self.model, normal, tensor, max_cells)
    }
    pub fn qualify_internal_g2(
        &self,
        normal: f64,
        tensor: f64,
        max_cells: usize,
    ) -> Result<QualifiedCornerInternalG2<'_>> {
        let report = self.inspect_internal_g2(normal, tensor, max_cells)?;
        require(
            report.within_tolerance,
            "Original corner geometric G2 network remains unproven",
        )?;
        Ok(QualifiedCornerInternalG2 {
            candidate: self,
            report,
        })
    }
}
impl QualifiedCornerInternalG2<'_> {
    pub fn model(&self) -> &Model {
        self.candidate.model()
    }
    /// Complete saved-face limiting curvature fields, using this immutable
    /// network's geometric G2 scope. Natural quotient poles remain separate.
    pub fn inspect_curvature(
        &self,
        tolerance_uv: f64,
        limits: crate::face_curvature::Limits,
    ) -> Result<CornerCurvatureAudit> {
        let mut report =
            super::curvature::inspect_one_sided_model(self.model(), tolerance_uv, limits)?;
        report.internal_g2_tolerances =
            Some([self.report.normal_tolerance, self.report.tensor_tolerance]);
        Ok(report)
    }
    pub fn report(&self) -> &CornerInternalG2Audit {
        &self.report
    }
}
#[cfg(test)]
#[path = "geometric_tests.rs"]
mod tests;
