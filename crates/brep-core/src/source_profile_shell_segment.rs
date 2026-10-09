//! Finite-segment exclusion on complete exact expression support domains.
//! Possible contacts are not isolated roots; material parity remains separate.
use super::*;
pub struct ExpressionSegmentCandidate {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub parameter: [f64; 2],
}
pub struct ExpressionSegmentReport {
    pub boundary_excluded: bool,
    pub candidates: Vec<ExpressionSegmentCandidate>,
    pub pending: Vec<(usize, [[f64; 2]; 2])>,
    pub cells: usize,
    pub spans: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Trim supersets conservatively retain possible contacts. No interior
    /// material or thickness certificate can be issued from this report.
    pub fn inspect_segment_supports(
        &self,
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance_uv: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionSegmentReport> {
        if !origin.iter().chain(&direction).all(|v| v.is_finite())
            || direction.iter().all(|v| *v == 0.)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
            || tolerance_uv > 1.
            || controls > 1000000
            || max_cells > 100000
            || max_spans > 1000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_SEGMENT",
                "Choose a finite segment and bounded coverage work",
            ));
        }
        let natural = (0..self.faces.len())
            .map(|face| self.support_domain(face))
            .collect::<Result<Vec<_>>>()?;
        let mut out = ExpressionSegmentReport {
            boundary_excluded: false,
            candidates: Vec::new(),
            pending: natural.iter().copied().enumerate().rev().collect(),
            cells: 0,
            spans: 0,
            reason: "expression-segment-work-limit",
        };
        while let Some((face, uv)) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push((face, uv));
                return Ok(out);
            }
            out.cells += 1;
            let (position, used) =
                self.enclose_support(face, uv, controls, (max_spans - out.spans).min(100000))?;
            out.spans += used;
            let Some(position) = position else {
                out.pending.push((face, uv));
                return Ok(out);
            };
            let Some(parameter) = nurbs_core::ray_surface::finite_line_enclosure_parameters(
                position, origin, direction,
            )?
            else {
                continue;
            };
            let width = std::array::from_fn::<_, 2, _>(|axis| {
                (uv[axis][1] - uv[axis][0]) / (natural[face][axis][1] - natural[face][axis][0])
            });
            if width.iter().all(|w| *w <= tolerance_uv) {
                out.candidates.push(ExpressionSegmentCandidate {
                    face,
                    uv,
                    parameter,
                });
                continue;
            }
            let axis = usize::from(width[1] > width[0]);
            let midpoint = uv[axis][0] * 0.5 + uv[axis][1] * 0.5;
            if midpoint <= uv[axis][0] || midpoint >= uv[axis][1] {
                out.pending.push((face, uv));
                out.reason = "expression-segment-resolution-limit";
                return Ok(out);
            }
            let mut left = uv;
            let mut right = uv;
            left[axis][1] = midpoint;
            right[axis][0] = midpoint;
            out.pending.push((face, right));
            out.pending.push((face, left));
        }
        out.boundary_excluded = out.candidates.is_empty();
        out.reason = if out.boundary_excluded {
            "expression-segment-boundary-excluded"
        } else {
            "expression-segment-contacts-not-isolated"
        };
        Ok(out)
    }
}
