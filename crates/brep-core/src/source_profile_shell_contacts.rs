//! Conservative support-pair exclusion. Overlap remains explicitly pending.
use super::*;
/// Parameter product for two supporting expressions, not a contact witness.
pub type SupportProduct = [[[f64; 2]; 2]; 2];
pub struct SupportPairReport {
    pub separated: Vec<SupportProduct>,
    pub pending: Vec<SupportProduct>,
    pub outside: Vec<SupportProduct>,
    pub domain_cells: usize,
    pub cells: usize,
    pub spans: usize,
}
impl SupportPairReport {
    pub fn complete_excluded(&self) -> bool {
        self.pending.is_empty()
    }
    pub fn complete_separated(&self) -> bool {
        self.pending.is_empty() && self.outside.is_empty()
    }
}
pub struct ShellSupportPair {
    pub faces: [usize; 2],
    pub shared_edges: Vec<usize>,
    pub inspection: SupportPairReport,
}
pub struct ShellSupportReport {
    pub domain_cells: usize,
    pub pairs: Vec<ShellSupportPair>,
    pub cells: usize,
    pub spans: usize,
}
impl ShellSupportReport {
    pub fn pending_pairs(&self) -> usize {
        self.pairs
            .iter()
            .filter(|p| !p.inspection.complete_excluded())
            .count()
    }
}
fn inspect<F>(
    domain: SupportProduct,
    max_cells: usize,
    max_spans: usize,
    bounds: F,
) -> Result<SupportPairReport>
where
    F: FnMut(usize, [[f64; 2]; 2], usize) -> Result<(Option<[[f64; 2]; 3]>, usize)>,
{
    inspect_regions(domain, max_cells, max_spans, 0, bounds, |_, _, _| {
        Ok((false, 0))
    })
}
pub(super) fn inspect_regions<F, G>(
    domain: SupportProduct,
    max_cells: usize,
    max_spans: usize,
    max_domain_cells: usize,
    mut bounds: F,
    mut outside: G,
) -> Result<SupportPairReport>
where
    F: FnMut(usize, [[f64; 2]; 2], usize) -> Result<(Option<[[f64; 2]; 3]>, usize)>,
    G: FnMut(usize, [[f64; 2]; 2], usize) -> Result<(bool, usize)>,
{
    if max_cells > 100000
        || max_spans > 100000
        || max_domain_cells > 100000
        || domain
            .iter()
            .flatten()
            .any(|r| !r.iter().all(|x| x.is_finite()) || r[0] > r[1] || !(r[1] - r[0]).is_finite())
    {
        return Err(Error::new(
            "BREP_PROFILE_CONTACTS",
            "Bound support pair domain and work",
        ));
    }
    let mut out = SupportPairReport {
        separated: Vec::new(),
        pending: Vec::new(),
        cells: 0,
        spans: 0,
        outside: Vec::new(),
        domain_cells: 0,
    };
    let mut stack = vec![domain];
    while let Some(cell) = stack.pop() {
        if out.cells == max_cells {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        }
        out.cells += 1;
        let (a, spent) = bounds(0, cell[0], max_spans - out.spans)?;
        out.spans += spent;
        let Some(a) = a else {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        };
        let (b, spent) = bounds(1, cell[1], max_spans - out.spans)?;
        out.spans += spent;
        let Some(b) = b else {
            out.pending.push(cell);
            out.pending.extend(stack);
            break;
        };
        if (0..3).any(|k| a[k][1] < b[k][0] || b[k][1] < a[k][0]) {
            out.separated.push(cell);
            continue;
        }
        let mut excluded = false;
        for side in 0..2 {
            if out.domain_cells == max_domain_cells {
                break;
            }
            let (is_outside, used) =
                outside(side, cell[side], max_domain_cells - out.domain_cells)?;
            out.domain_cells += used;
            if is_outside {
                excluded = true;
                break;
            }
        }
        if excluded {
            out.outside.push(cell);
            continue;
        }
        let mut choice = None;
        let mut width = -1.;
        for side in 0..2 {
            for axis in 0..2 {
                let r = cell[side][axis];
                let midpoint = r[0] + (r[1] - r[0]) / 2.;
                let original = domain[side][axis];
                let scale = original[1] - original[0];
                if midpoint > r[0] && midpoint < r[1] && scale > 0. {
                    let normalized = (r[1] - r[0]) / scale;
                    if normalized > width {
                        choice = Some((side, axis, midpoint));
                        width = normalized;
                    }
                }
            }
        }
        let Some((side, axis, midpoint)) = choice else {
            out.pending.push(cell);
            continue;
        };
        let mut left = cell;
        let mut right = cell;
        left[side][axis][1] = midpoint;
        right[side][axis][0] = midpoint;
        stack.push(right);
        stack.push(left);
    }
    Ok(out)
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Complete containing support rectangle. Trimmed-out regions are retained
    /// conservatively until a separate region classifier excludes them.
    pub fn support_domain(&self, face: usize) -> Result<[[f64; 2]; 2]> {
        let kind = self
            .faces
            .get(face)
            .ok_or_else(|| Error::new("BREP_PROFILE_CONTACTS", "Unknown expression face"))?
            .kind;
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let proposal = root.restricted.proposal;
        let natural = |surface: &nurbs_core::surface::Surface| {
            [
                [
                    surface.knots_u[surface.degree_u],
                    surface.knots_u[surface.control_points.len()],
                ],
                [
                    surface.knots_v[surface.degree_v],
                    surface.knots_v[surface.control_points[0].len()],
                ],
            ]
        };
        match kind {
            ShellFaceKind::Original(index) => Ok(natural(
                proposal.body.geometry().shell().faces()[index][0].edges()[0].surface(),
            )),
            ShellFaceKind::Retained(end) => {
                let mut domain = natural(proposal.profile(root.face).unwrap().source());
                domain[root.axis][1 - end] = root.parameters[end];
                Ok(domain)
            }
            ShellFaceKind::Transition => Ok([
                proposal
                    .profile(root.face)
                    .unwrap()
                    .section(root.axis, root.parameters[0])?
                    .range(),
                [0., 1.],
            ]),
        }
    }
    /// Inspect every unordered face pair, including adjacent faces. Resource
    /// exhaustion records every remaining pair with its full pending product.
    pub fn inspect_shell_supports(
        &self,
        max_cells: usize,
        cells_per_pair: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
    ) -> Result<ShellSupportReport> {
        self.inspect_shell_regions(
            max_cells,
            cells_per_pair,
            max_controls_per_enclosure,
            max_spans,
            None,
            0,
            0,
        )
    }
    pub fn inspect_shell_trimmed(
        &self,
        max_cells: usize,
        cells_per_pair: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
        tolerance_uv: f64,
        max_domain_cells: usize,
        domain_cells_per_pair: usize,
    ) -> Result<ShellSupportReport> {
        if !tolerance_uv.is_finite() || tolerance_uv <= 0. {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require positive finite UV tolerance",
            ));
        }
        self.inspect_shell_regions(
            max_cells,
            cells_per_pair,
            max_controls_per_enclosure,
            max_spans,
            Some(tolerance_uv),
            max_domain_cells,
            domain_cells_per_pair,
        )
    }
    fn inspect_shell_regions(
        &self,
        max_cells: usize,
        cells_per_pair: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
        tolerance_uv: Option<f64>,
        max_domain_cells: usize,
        domain_cells_per_pair: usize,
    ) -> Result<ShellSupportReport> {
        if max_cells > 100000
            || cells_per_pair > 100000
            || max_domain_cells > 100000
            || domain_cells_per_pair > 100000
            || max_spans > 100000
            || max_controls_per_enclosure > 1000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound shell support inspection work",
            ));
        }
        let pair_count = self
            .faces
            .len()
            .checked_mul(self.faces.len().saturating_sub(1))
            .and_then(|n| n.checked_div(2));
        if pair_count.is_none_or(|n| n > 100000) {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound complete shell pair report to 100000 pairs",
            ));
        }
        let domains = (0..self.faces.len())
            .map(|i| self.support_domain(i))
            .collect::<Result<Vec<_>>>()?;
        let mut out = ShellSupportReport {
            domain_cells: 0,
            pairs: Vec::new(),
            cells: 0,
            spans: 0,
        };
        for a in 0..self.faces.len() {
            for b in a + 1..self.faces.len() {
                let faces = [a, b];
                let domain = [domains[a], domains[b]];
                let pair_cells = cells_per_pair.min(max_cells - out.cells);
                let inspection = if let Some(tolerance) = tolerance_uv {
                    self.inspect_trimmed_pair(
                        faces,
                        domain,
                        tolerance,
                        pair_cells,
                        max_controls_per_enclosure,
                        max_spans - out.spans,
                        domain_cells_per_pair.min(max_domain_cells - out.domain_cells),
                    )?
                } else {
                    self.inspect_support_pair(
                        faces,
                        domain,
                        pair_cells,
                        max_controls_per_enclosure,
                        max_spans - out.spans,
                    )?
                };
                out.domain_cells += inspection.domain_cells;
                out.cells += inspection.cells;
                out.spans += inspection.spans;
                let shared_edges = self
                    .pairs
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| {
                        (p.uses[0].face == a && p.uses[1].face == b)
                            || (p.uses[0].face == b && p.uses[1].face == a)
                    })
                    .map(|(i, _)| i)
                    .collect();
                out.pairs.push(ShellSupportPair {
                    faces,
                    shared_edges,
                    inspection,
                });
            }
        }
        Ok(out)
    }
    /// Exclude products either by strict world separation or by certified
    /// exclusion from one exact source contour. Boundary bands stay pending.
    pub fn inspect_trimmed_pair(
        &self,
        faces: [usize; 2],
        domain: SupportProduct,
        tolerance_uv: f64,
        max_cells: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
        max_domain_cells: usize,
    ) -> Result<SupportPairReport> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_controls_per_enclosure > 1000000
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require distinct owned faces and bounded region work",
            ));
        }
        let loops = faces.map(|face| {
            self.faces[face]
                .loops
                .iter()
                .map(|wire| {
                    wire.iter()
                        .filter_map(|u| {
                            if let ShellUse::Source(f) = u {
                                Some(f.clone())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        });
        inspect_regions(
            domain,
            max_cells,
            max_spans,
            max_domain_cells,
            |side, rect, budget| {
                self.enclose_support(faces[side], rect, max_controls_per_enclosure, budget)
            },
            |side, rect, budget| {
                if self.faces[faces[side]].kind == ShellFaceKind::Transition {
                    return Ok((false, 0));
                }
                let report = crate::source_contour_winding::classify(
                    &loops[side],
                    rect,
                    tolerance_uv,
                    budget,
                )?;
                Ok((
                    report.location == nurbs_core::trim_domain::Location::Outside,
                    report.cells,
                ))
            },
        )
    }
    /// Excludes intersections only on the supplied support rectangles. Shared
    /// seams and uncertain overlap remain pending; no sampled contact admission.
    pub fn inspect_support_pair(
        &self,
        faces: [usize; 2],
        domain: SupportProduct,
        max_cells: usize,
        max_controls_per_enclosure: usize,
        max_spans: usize,
    ) -> Result<SupportPairReport> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_controls_per_enclosure > 1000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require two distinct owned faces and bounded controls",
            ));
        }
        inspect(domain, max_cells, max_spans, |side, rect, budget| {
            self.enclose_support(faces[side], rect, max_controls_per_enclosure, budget)
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn volume(r: SupportProduct) -> f64 {
        r.iter().flatten().map(|x| x[1] - x[0]).product()
    }
    #[test]
    fn support_pair_covers_budget_remainder_and_real_crossing() {
        let domain = [[[0., 1.]; 2]; 2];
        // Two perpendicular planes sharing a line. Bounds cannot exclude it.
        let report = inspect(domain, 31, 100, |side, r, _| {
            Ok((
                Some(if side == 0 {
                    [r[0], r[1], [0.; 2]]
                } else {
                    [r[0], [0.5; 2], r[1]]
                }),
                1,
            ))
        })
        .unwrap();
        assert!(!report.complete_separated());
        assert_eq!(report.cells, 31);
        let covered = report
            .pending
            .iter()
            .chain(&report.separated)
            .copied()
            .map(volume)
            .sum::<f64>();
        assert_eq!(covered, 1.);
        assert!(report
            .pending
            .iter()
            .any(|r| r[0][1][0] <= 0.5 && r[0][1][1] >= 0.5 && r[1][1][0] == 0.));
        let no_work = inspect(domain, 0, 100, |_, _, _| panic!("must not evaluate")).unwrap();
        assert_eq!(no_work.pending, vec![domain]);
        let no_spans = inspect(domain, 10, 0, |_, _, _| Ok((None, 0))).unwrap();
        assert_eq!(no_spans.pending, vec![domain]);
    }
    #[test]
    fn support_pair_region_exclusion_keeps_material_and_budget_cover() {
        let domain = [[[0., 1.]; 2]; 2];
        let report = inspect_regions(
            domain,
            31,
            100,
            100,
            |_, r, _| Ok((Some([r[0], r[1], [0.; 2]]), 1)),
            |_, r, budget| Ok((r[0][1] < 0.75, usize::from(budget > 0))),
        )
        .unwrap();
        assert!(!report.outside.is_empty());
        assert!(!report.complete_excluded());
        let covered = report
            .pending
            .iter()
            .chain(&report.separated)
            .chain(&report.outside)
            .copied()
            .map(volume)
            .sum::<f64>();
        assert_eq!(covered, 1.);
        assert!(report
            .outside
            .iter()
            .all(|r| r.iter().any(|side| side[0][1] < 0.75)));
        let none = inspect_regions(
            domain,
            1,
            2,
            0,
            |_, r, _| Ok((Some([r[0], r[1], [0.; 2]]), 1)),
            |_, _, _| panic!("zero domain allowance must not classify"),
        )
        .unwrap();
        assert!(none.outside.is_empty());
        assert!(!none.complete_excluded());
    }
    #[test]
    fn support_pair_proves_disjoint_planes_and_retains_touch() {
        let domain = [[[0., 1.]; 2]; 2];
        let separated = inspect(domain, 1, 2, |side, r, _| {
            Ok((Some([r[0], r[1], [side as f64; 2]]), 1))
        })
        .unwrap();
        assert!(separated.complete_separated());
        assert_eq!(separated.separated, vec![domain]);
        let touch = inspect(domain, 1, 2, |_, r, _| Ok((Some([r[0], r[1], [0.; 2]]), 1))).unwrap();
        assert!(!touch.complete_separated());
        assert_eq!(touch.pending.iter().copied().map(volume).sum::<f64>(), 1.);
    }
}
