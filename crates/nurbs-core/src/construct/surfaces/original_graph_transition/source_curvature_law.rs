//! Profile-independent contact-radius reference from original support/width
//! inputs. This does not turn a variable runout into a constant-radius fillet.
use super::*;
use crate::distance_bounds::Interval as I;
pub struct SourceContactCurvatureLaw {
    source: Surface,
    width: Curve,
}
impl SourceContactCurvatureLaw {
    /// Reference inverse radius |h|/(w*sqrt(1+h^2+(q'-h'w)^2)).
    /// No output surface coefficient or transverse profile is read here.
    pub fn new(source: &Surface, width: &Curve, b: &mut Budget) -> Result<Self> {
        source.validate()?;
        width.validate()?;
        check(
            !source.periodic_u
                && !source.periodic_v
                && (1..=12).contains(&source.degree_u)
                && source.degree_v == 1
                && source.control_points.len() == source.degree_u + 1
                && source.control_points[0].len() == 2
                && source.control_points.iter().flatten().all(|p| p.len() == 3)
                && source
                    .weights
                    .iter()
                    .flatten()
                    .all(|w| *w == source.weights[0][0])
                && !width.periodic
                && width.degree <= 8
                && width.control_points.len() == width.degree + 1
                && width
                    .control_points
                    .iter()
                    .all(|p| p.len() == 3 && p[0] > 0. && p[0] <= 1. && p[1] == 0. && p[2] == 0.)
                && width.weights.iter().all(|w| *w == width.weights[0]),
            "Source contact-curvature law requires original polynomial graph inputs",
        )?;
        for p in source
            .control_points
            .iter()
            .flatten()
            .chain(width.control_points.iter())
        {
            for x in p {
                b.authored(*x)?;
            }
        }
        Ok(Self {
            source: source.clone(),
            width: width.clone(),
        })
    }
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn width(&self) -> &Curve {
        &self.width
    }
    pub fn matches(&self, p: &Patch) -> bool {
        self.source == p.sources[0] && self.width == p.width
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureLawDecision {
    Within,
    Exceeds,
    Unresolved,
}
pub struct SourceCurvatureLawReport {
    pub decision: CurvatureLawDecision,
    pub normalized_domain: [[f64; 2]; 2],
    pub tolerance_inverse_mm: f64,
    pub radial_distance_tolerance_mm: f64,
    pub radial_distance_error_upper_mm: Option<f64>,
    pub normal_chord_tolerance: f64,
    pub normal_chord_error_upper: Option<f64>,
    pub source_inverse_radius_range: Option<[f64; 2]>,
    pub error_inverse_mm: Option<[f64; 2]>,
    pub coefficient_work: usize,
    pub reason: &'static str,
}
fn sqrt(x: I) -> Result<I> {
    check(x.hi >= 0., "Negative squared-curvature enclosure")?;
    I::new(
        x.lo.max(0.).sqrt().next_down().max(0.),
        x.hi.sqrt().next_up(),
    )
}
impl Patch {
    /// Complete-domain comparison to a reference owned by ORIGINAL inputs.
    /// The explicit tolerance has inverse-mm units, not mm radius units.
    /// Inconclusive bounds remain unresolved; mismatched source owners refuse.
    pub fn inspect_source_curvature_law(
        &self,
        law: &SourceContactCurvatureLaw,
        tolerance_inverse_mm: f64,
        radial_distance_tolerance_mm: f64,
        normal_chord_tolerance: f64,
        domain: [[f64; 2]; 2],
        b: &mut Budget,
    ) -> Result<SourceCurvatureLawReport> {
        self.inspect_source_law_domain(law, tolerance_inverse_mm,
            radial_distance_tolerance_mm, normal_chord_tolerance, domain, false, b)
    }
    /// The two complete source-contact seams. G2 to a ruled source forces
    /// zero transverse normal curvature here; a positive constant inverse
    /// radius law may therefore be incompatible with the prescribed contact.
    pub fn inspect_source_curvature_seams(
        &self,
        law: &SourceContactCurvatureLaw,
        tolerance_inverse_mm: f64,
        radial_distance_tolerance_mm: f64,
        normal_chord_tolerance: f64,
        b: &mut Budget,
    ) -> Result<[SourceCurvatureLawReport; 2]> {
        let mut reports = Vec::with_capacity(2);
        for v in [0., 1.] {
            reports.push(self.inspect_source_law_domain(law, tolerance_inverse_mm,
                radial_distance_tolerance_mm, normal_chord_tolerance,
                [[0., 1.], [v, v]], true, b)?);
        }
        Ok(reports.try_into().ok().unwrap())
    }
    fn inspect_source_law_domain(
        &self,
        law: &SourceContactCurvatureLaw,
        tolerance_inverse_mm: f64,
        radial_distance_tolerance_mm: f64,
        normal_chord_tolerance: f64,
        domain: [[f64; 2]; 2],
        seam: bool,
        b: &mut Budget,
    ) -> Result<SourceCurvatureLawReport> {
        check(
            law.matches(self),
            "Contact-curvature reference belongs to different source inputs",
        )?;
        check(
            tolerance_inverse_mm.is_finite()
                && tolerance_inverse_mm >= 0.
                && radial_distance_tolerance_mm.is_finite()
                && radial_distance_tolerance_mm >= 0.
                && normal_chord_tolerance.is_finite()
                && normal_chord_tolerance >= 0.
                && domain.iter().enumerate().all(|(axis, r)| {
                    r[0].is_finite() && r[1].is_finite() && 0. <= r[0]
                        && (r[0] < r[1] || (seam && axis == 1 && r[0] == r[1]
                            && (r[0] == 0. || r[0] == 1.))) && r[1] <= 1.
                }),
            "Invalid source-curvature law domain or inverse-mm tolerance",
        )?;
        let start = b.used();
        let mut report = SourceCurvatureLawReport {
            decision: CurvatureLawDecision::Unresolved,
            normalized_domain: domain,
            tolerance_inverse_mm,
            radial_distance_tolerance_mm,
            radial_distance_error_upper_mm: None,
            normal_chord_tolerance,
            normal_chord_error_upper: None,
            source_inverse_radius_range: None,
            error_inverse_mm: None,
            coefficient_work: 0,
            reason: "coefficient-bit-or-numeric-limit",
        };
        let result = (|| -> Result<Option<_>> {
            let Some(f) = self.graph_curvature_factors(domain, false, b)? else {
                return Ok(None);
            };
            if f.d_ratio.lo <= 0. || f.e.lo <= 0. {
                return Ok(None);
            }
            let source = sqrt(f.source_curvature_squared)?;
            let p = f.phi_second;
            let abs_p = I::new(
                if p.lo <= 0. && p.hi >= 0. {
                    0.
                } else {
                    p.lo.abs().min(p.hi.abs())
                },
                p.lo.abs().max(p.hi.abs()),
            )?;
            let multiplier = abs_p.div(f.e.mul(sqrt(f.d_ratio)?)?.mul(I::point(4.))?)?;
            let delta = multiplier.sub(I::point(1.))?;
            let distance = if delta.lo > 0. {
                delta.lo
            } else if delta.hi < 0. {
                -delta.hi
            } else {
                0.
            };
            let error = [
                (source.lo * distance).next_down().max(0.),
                (source.hi * delta.lo.abs().max(delta.hi.abs())).next_up(),
            ];
            check(
                error.iter().all(|x| x.is_finite()),
                "Source curvature-law bound overflow",
            )?;
            Ok(Some((
                source,
                error,
                f.radial_distance_error_upper,
                f.radial_normal_chord_error_upper,
            )))
        })();
        if let Ok(Some((source, error, radial_error, normal_error))) = result {
            report.radial_distance_error_upper_mm = Some(radial_error);
            report.normal_chord_error_upper = normal_error;
            report.source_inverse_radius_range = Some([source.lo, source.hi]);
            report.error_inverse_mm = Some(error);
            if error[1] <= tolerance_inverse_mm
                && radial_error <= radial_distance_tolerance_mm
                && normal_error.is_some_and(|x| x <= normal_chord_tolerance)
            {
                report.decision = CurvatureLawDecision::Within;
                report.reason = "complete-source-curvature-law-domain";
            } else if error[0] > tolerance_inverse_mm {
                report.decision = CurvatureLawDecision::Exceeds;
                report.reason = "whole-domain-source-curvature-law-violation";
            } else {
                report.reason = "source-curvature-law-bound-unresolved";
            }
        }
        report.coefficient_work = b.used() - start;
        Ok(report)
    }
}

/// A partition of the ENTIRE normalized patch, including both source seams.
/// Every leaf remains visible, including budget-limited/unresolved regions.
pub struct SourceCurvatureLawCoverage {
    pub cells: Vec<SourceCurvatureLawReport>,
}
impl SourceCurvatureLawCoverage {
    pub fn decision(&self) -> CurvatureLawDecision {
        if self.cells.iter().any(|r| r.decision == CurvatureLawDecision::Exceeds) {
            CurvatureLawDecision::Exceeds
        } else if !self.cells.is_empty()
            && self.cells.iter().all(|r| r.decision == CurvatureLawDecision::Within)
        {
            CurvatureLawDecision::Within
        } else {
            CurvatureLawDecision::Unresolved
        }
    }
}
impl Patch {
    /// Adaptive complete-patch source-law qualification. No transverse band
    /// or seam is omitted, and subdivision shares the caller's work budget.
    pub fn inspect_complete_source_curvature_law(
        &self,
        law: &SourceContactCurvatureLaw,
        tolerance_inverse_mm: f64,
        radial_distance_tolerance_mm: f64,
        normal_chord_tolerance: f64,
        max_cells: usize,
        b: &mut Budget,
    ) -> Result<SourceCurvatureLawCoverage> {
        check((1..=65536).contains(&max_cells), "Invalid complete-law cell limit")?;
        let mut pending = vec![[[0., 1.], [0., 1.]]];
        let mut cells = Vec::new();
        while let Some(domain) = pending.pop() {
            let report = self.inspect_source_curvature_law(
                law, tolerance_inverse_mm, radial_distance_tolerance_mm,
                normal_chord_tolerance, domain, b,
            )?;
            if report.decision == CurvatureLawDecision::Unresolved
                && report.coefficient_work > 0
                && cells.len() + pending.len() + 2 <= max_cells
            {
                let axis = if domain[0][1] - domain[0][0] > domain[1][1] - domain[1][0] { 0 } else { 1 };
                let mid = domain[axis][0] + (domain[axis][1] - domain[axis][0]) * 0.5;
                if domain[axis][0] < mid && mid < domain[axis][1] {
                    let mut left = domain;
                    let mut right = domain;
                    left[axis][1] = mid;
                    right[axis][0] = mid;
                    pending.push(right);
                    pending.push(left);
                    continue;
                }
            }
            cells.push(report);
        }
        Ok(SourceCurvatureLawCoverage { cells })
    }
}
