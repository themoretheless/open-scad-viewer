//! Bounded native support enclosures for diagnostic display.
//! These rectangles include trimmed-out material. They are neither triangles
//! nor an admission of the represented shell as a solid.
use super::*;
use nurbs_core::trim_domain::Location;

pub struct ExpressionDisplayLimits {
    pub divisions: usize,
    pub cells: usize,
    pub controls_per_enclosure: usize,
    pub spans: usize,
    pub tolerance_uv: f64,
    pub domain_cells: usize,
    pub domain_cells_per_cell: usize,
}
pub struct ExpressionDisplayCell {
    pub face: usize,
    pub domain: [[f64; 2]; 2],
    pub bounds: Option<[[f64; 2]; 3]>,
    pub location: Location,
}
pub struct ExpressionDisplayReport {
    pub cells: Vec<ExpressionDisplayCell>,
    pub enclosed: usize,
    pub pending: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub inside: usize,
    pub outside: usize,
    pub unresolved_trim: usize,
}
fn grid_domain(domain: [[f64; 2]; 2], divisions: usize, i: usize, j: usize) -> [[f64; 2]; 2] {
    let coordinate = |axis: usize, k: usize| {
        if k == 0 {
            domain[axis][0]
        } else if k == divisions {
            domain[axis][1]
        } else {
            let t = k as f64 / divisions as f64;
            (domain[axis][0] * (1. - t) + domain[axis][1] * t)
                .clamp(domain[axis][0], domain[axis][1])
        }
    };
    [
        [coordinate(0, i), coordinate(0, i + 1)],
        [coordinate(1, j), coordinate(1, j + 1)],
    ]
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Classify complete UV rectangles against the current root-ended loops.
    /// Supporting-surface identity alone does not authorize a whole-chart trim.
    pub(super) fn display_location(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        tolerance: f64,
        cells: usize,
    ) -> Result<(Location, usize)> {
        let (location, used, _, _, _) =
            self.display_query_location(face, domain, None, tolerance, cells, None)?;
        Ok((location, used))
    }
    pub(super) fn display_triangle_location(
        &self,
        face: usize,
        triangle: [[f64; 2]; 3],
        tolerance: f64,
        cells: usize,
        prepared: Option<&mut crate::source_contour_winding::Prepared<'_>>,
    ) -> Result<(Location, usize, bool, usize, usize)> {
        let domain = std::array::from_fn(|k| {
            [
                triangle.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min),
                triangle
                    .iter()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        });
        self.display_query_location(face, domain, Some(triangle), tolerance, cells, prepared)
    }
    fn display_query_location(
        &self,
        face: usize,
        domain: [[f64; 2]; 2],
        triangle: Option<[[f64; 2]; 3]>,
        tolerance: f64,
        cells: usize,
        prepared: Option<&mut crate::source_contour_winding::Prepared<'_>>,
    ) -> Result<(Location, usize, bool, usize, usize)> {
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let source = root.restricted.proposal.body.geometry().shell();
        let original = match self.faces[face].kind() {
            ShellFaceKind::Transition => return Ok((Location::Inside, 0, true, 0, 0)),
            ShellFaceKind::Original(i) => i,
            ShellFaceKind::Retained(_) => root.face,
        };
        let region = &source.regions().unwrap()[original];
        if self.equivalent_original_face(face) == Some(original) && region.whole_chart_material() {
            return Ok((Location::Inside, 0, true, 0, 0));
        }
        if cells == 0 {
            return Ok((Location::Unresolved, 0, false, 0, 0));
        }
        let report = if let (Some(prepared), Some(triangle)) = (prepared, triangle) {
            prepared.classify_triangle(triangle, tolerance, cells)?
        } else {
            let mut loops = Vec::with_capacity(self.faces[face].loops.len());
            for wire in &self.faces[face].loops {
                let mut fragments = Vec::with_capacity(wire.len());
                for edge in wire {
                    let ShellUse::Source(fragment) = edge else {
                        return Ok((Location::Unresolved, 0, false, 0, 0));
                    };
                    fragments.push(fragment);
                }
                loops.push(fragments);
            }
            match triangle {
                Some(t) => {
                    crate::source_contour_winding::classify_triangle(&loops, t, tolerance, cells)?
                }
                None => crate::source_contour_winding::classify(&loops, domain, tolerance, cells)?,
            }
        };
        let location = match report.location {
            Location::Outside => Location::Outside,
            Location::Inside if report.winding == Some(region.chart_winding()) => Location::Inside,
            _ => Location::Unresolved,
        };
        Ok((
            location,
            report.cells,
            report.rectangle_proven && location != Location::Unresolved,
            report.enclosure_queries,
            report.enclosure_cache_hits,
        ))
    }
    /// Every face is represented even when no enclosure work is allowed.
    /// Unspent or exhausted faces retain their complete support rectangle.
    pub fn inspect_display_enclosures(
        &self,
        limits: ExpressionDisplayLimits,
    ) -> Result<ExpressionDisplayReport> {
        if !(1..=64).contains(&limits.divisions)
            || limits.cells > 100_000
            || limits.controls_per_enclosure > 1_000_000
            || limits.spans > 100_000
            || limits.domain_cells > 100_000
            || limits.domain_cells_per_cell > 100_000
            || !limits.tolerance_uv.is_finite()
            || limits.tolerance_uv <= 0.
        {
            return Err(Error::new(
                "BREP_PROFILE_DISPLAY",
                "Bound display divisions, cells, controls and spans",
            ));
        }
        let mut report = ExpressionDisplayReport {
            cells: Vec::new(),
            enclosed: 0,
            pending: 0,
            spans: 0,
            domain_cells: 0,
            inside: 0,
            outside: 0,
            unresolved_trim: 0,
        };
        let per_face = limits.divisions * limits.divisions;
        let mut evaluated = 0;
        for face in 0..self.faces().len() {
            let domain = self.support_domain(face)?;
            if per_face > limits.cells.saturating_sub(evaluated)
                || report.spans == limits.spans
                || limits.controls_per_enclosure == 0
            {
                report.cells.push(ExpressionDisplayCell {
                    face,
                    domain,
                    bounds: None,
                    location: Location::Unresolved,
                });
                report.pending += 1;
                report.unresolved_trim += 1;
                continue;
            }
            for i in 0..limits.divisions {
                for j in 0..limits.divisions {
                    let domain = grid_domain(domain, limits.divisions, i, j);
                    let (location, domain_cells) = self.display_location(
                        face,
                        domain,
                        limits.tolerance_uv,
                        limits
                            .domain_cells_per_cell
                            .min(limits.domain_cells - report.domain_cells),
                    )?;
                    report.domain_cells += domain_cells;
                    match location {
                        Location::Inside => report.inside += 1,
                        Location::Outside => report.outside += 1,
                        _ => report.unresolved_trim += 1,
                    }
                    let (bounds, used) = self.enclose_support(
                        face,
                        domain,
                        limits.controls_per_enclosure,
                        limits.spans - report.spans,
                    )?;
                    report.spans += used;
                    evaluated += 1;
                    if bounds.is_some() {
                        report.enclosed += 1;
                    } else {
                        report.pending += 1;
                    }
                    report.cells.push(ExpressionDisplayCell {
                        face,
                        domain,
                        bounds,
                        location,
                    });
                }
            }
        }
        Ok(report)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partition_preserves_endpoints_and_shared_grid_coordinates() {
        let domain = [[-1e100, 1e100], [0.75, 0.9]];
        for n in [1, 3, 64] {
            assert_eq!(grid_domain(domain, n, 0, 0)[0][0], domain[0][0]);
            assert_eq!(grid_domain(domain, n, n - 1, n - 1)[1][1], domain[1][1]);
            for i in 0..n - 1 {
                assert_eq!(
                    grid_domain(domain, n, i, 0)[0][1].to_bits(),
                    grid_domain(domain, n, i + 1, 0)[0][0].to_bits()
                );
            }
        }
    }
}
