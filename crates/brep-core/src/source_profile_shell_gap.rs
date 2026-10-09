//! Whole-support clearance for exact expression faces. Trim supersets are
//! conservative; clearance alone does not prove material wall thickness.
use super::*;
/// A fresh whole-support lower bound tied to the immutable source shell.
/// This certificate establishes separation, not a material wall chord.
pub struct ExpressionFaceClearance<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    shell: &'g ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    faces: [usize; 2],
    lower_mm: f64,
}
impl ExpressionFaceClearance<'_, '_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn shell(&self) -> &ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
        self.shell
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn lower_mm(&self) -> f64 {
        self.lower_mm
    }
}
impl<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    /// Recompute coverage; caller-provided diagnostic reports cannot issue proof.
    pub fn qualify_face_clearance<'g>(
        &'g self,
        faces: [usize; 2],
        minimum_mm: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<(
        ExpressionGapReport,
        Option<ExpressionFaceClearance<'g, 'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>>,
    )> {
        let report =
            self.inspect_face_clearance(faces, minimum_mm, controls, max_cells, max_spans)?;
        let certificate = report
            .lower_mm
            .filter(|lower| {
                report.complete
                    && report.pending.is_empty()
                    && lower.is_finite()
                    && *lower >= minimum_mm
            })
            .map(|lower_mm| ExpressionFaceClearance {
                shell: self,
                faces,
                lower_mm,
            });
        Ok((report, certificate))
    }
}
pub struct ExpressionGapReport {
    pub faces: [usize; 2],
    pub requested_lower_mm: f64,
    pub complete: bool,
    pub lower_mm: Option<f64>,
    pub cells: usize,
    pub spans: usize,
    pub pending: Vec<[[[f64; 2]; 2]; 2]>,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn inspect_face_clearance(
        &self,
        faces: [usize; 2],
        minimum_mm: f64,
        controls: usize,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<ExpressionGapReport> {
        if faces[0] == faces[1]
            || !minimum_mm.is_finite()
            || minimum_mm <= 0.
            || controls > 1000000
            || max_cells > 100000
            || max_spans > 1000000
        {
            return Err(Error::new(
                "BREP_EXPRESSION_GAP",
                "Bound distinct face clearance and work",
            ));
        }
        let natural = [
            self.support_domain(faces[0])?,
            self.support_domain(faces[1])?,
        ];
        let mut out = ExpressionGapReport {
            faces,
            requested_lower_mm: minimum_mm,
            complete: false,
            lower_mm: None,
            cells: 0,
            spans: 0,
            pending: vec![natural],
            reason: "expression-gap-work-limit",
        };
        let mut lower = f64::INFINITY;
        while let Some(pair) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(pair);
                return Ok(out);
            }
            out.cells += 1;
            let mut boxes = Vec::new();
            for side in 0..2 {
                let (bounds, used) = self.enclose_support(
                    faces[side],
                    pair[side],
                    controls,
                    (max_spans - out.spans).min(100000),
                )?;
                out.spans += used;
                let Some(bounds) = bounds else {
                    out.pending.push(pair);
                    return Ok(out);
                };
                boxes.push(bounds);
            }
            let bound = nurbs_core::surface_distance::enclosure_distance(&boxes[0], &boxes[1])?.0;
            if bound >= minimum_mm {
                lower = lower.min(bound);
                continue;
            }
            let mut choice = (0, 0);
            let mut widest = 0.;
            for side in 0..2 {
                for axis in 0..2 {
                    let width = (pair[side][axis][1] - pair[side][axis][0])
                        / (natural[side][axis][1] - natural[side][axis][0]);
                    if width > widest {
                        widest = width;
                        choice = (side, axis);
                    }
                }
            }
            let (side, axis) = choice;
            let range = pair[side][axis];
            let mid = range[0] * 0.5 + range[1] * 0.5;
            if mid <= range[0] || mid >= range[1] {
                out.pending.push(pair);
                out.reason = "expression-gap-resolution-limit";
                return Ok(out);
            }
            let mut left = pair;
            left[side][axis][1] = mid;
            let mut right = pair;
            right[side][axis][0] = mid;
            out.pending.push(right);
            out.pending.push(left);
        }
        out.complete = true;
        out.lower_mm = Some(lower);
        out.reason = "expression-face-clearance-qualified";
        Ok(out)
    }
}
