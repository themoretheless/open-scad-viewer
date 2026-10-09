//! Complete radius-law coverage of the exact corrected Hermite expression.
//! Reports carry no B-rep admission or caller-created certificate authority.
use crate::{
    check,
    curve::Curve,
    interval_eval::{self, Interval as I},
    profile_section_bridge::RailConstrainedBridge,
    Result,
};
pub struct Report {
    pub domain: [[f64; 2]; 2],
    pub complete: bool,
    pub cells: usize,
    pub spans: usize,
    pub accepted_cells: usize,
    pub accepted_error_upper: f64,
    pub pending: Vec<[[f64; 2]; 2]>,
    pub reason: &'static str,
    pub violation: Option<Violation>,
}
impl Report {
    /// Diagnostic parameter-area fraction, not physical surface area or proof.
    /// Admission continues to require complete coverage with no pending cells.
    pub fn pending_parameter_fraction(&self) -> f64 {
        self.pending
            .iter()
            .map(|rect| {
                (0..2)
                    .map(|axis| {
                        (rect[axis][1] - rect[axis][0])
                            / (self.domain[axis][1] - self.domain[axis][0])
                    })
                    .product::<f64>()
            })
            .sum::<f64>()
            .clamp(0., 1.)
    }
}
pub struct Violation {
    pub domain: [[f64; 2]; 2],
    pub error_bounds: [f64; 2],
    pub squared_residual: [f64; 2],
}
fn mapped(d: [f64; 2], across: [f64; 2]) -> Result<I> {
    I::point(d[0])
        .add(I::new(across[0], across[1])?.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])
}
fn spans(curve: &Curve, t: I) -> usize {
    (curve.degree..curve.control_points.len())
        .filter(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= t.hi
                && curve.knots[i + 1] >= t.lo
        })
        .count()
}
fn continuous_knots(knots: &[f64], degree: usize, count: usize, domain: [f64; 2]) -> bool {
    !knots[degree + 1..count].iter().any(|&knot| {
        domain[0] <= knot
            && knot <= domain[1]
            && knots.iter().filter(|&&k| k == knot).count() > degree
    })
}
// Original rational jets differentiate the restricted local [0,1] chart.
// Convert to source parameter, then to normalized bridge across parameter.
fn law_derivative(
    curve: &Curve,
    t: I,
    law_domain: [f64; 2],
    budget: usize,
) -> Result<(Option<Vec<I>>, usize)> {
    let mut spent = 0;
    let mut hull: Option<Vec<I>> = None;
    let scale = I::point(law_domain[1]).sub(I::point(law_domain[0]))?;
    for span in curve.degree..curve.control_points.len() {
        let lo = t.lo.max(curve.knots[span]);
        let hi = t.hi.min(curve.knots[span + 1]);
        if lo >= hi {
            continue;
        }
        if spent == budget {
            return Ok((None, spent));
        }
        spent += 1;
        let jet = crate::curve_jets::enclose(curve, span, [lo, hi])?;
        let width = I::point(hi).sub(I::point(lo))?;
        let derivative = jet[1]
            .iter()
            .map(|d| d.div(width)?.mul(scale))
            .collect::<Result<Vec<_>>>()?;
        if let Some(previous) = &mut hull {
            for (a, b) in previous.iter_mut().zip(derivative) {
                *a = I::new(a.lo.min(b.lo), a.hi.max(b.hi))?;
            }
        } else {
            hull = Some(derivative);
        }
    }
    Ok((hull, spent))
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    fn mean_radius_residual(
        &self,
        centers: &Curve,
        radius: &Curve,
        domains: [[f64; 2]; 2],
        rect: [[f64; 2]; 2],
        position: [[f64; 2]; 3],
        c: &[I],
        r: I,
        budget: usize,
    ) -> Result<(Option<([f64; 2], [f64; 2])>, usize)> {
        for (curve, domain) in [centers, radius].into_iter().zip(domains) {
            if !continuous_knots(
                &curve.knots,
                curve.degree,
                curve.control_points.len(),
                domain,
            ) {
                return Ok((None, 0));
            }
        }
        // The exact profile rails participate in the corrected expression.
        // A discontinuous source chart cannot use a whole-cell mean bound.
        for high in [false, true] {
            let section = self.section(high);
            let source = section.profile().source();
            let across = [
                self.section(false).parameter(),
                self.section(true).parameter(),
            ];
            let mut domain = [[across[0].min(across[1]), across[0].max(across[1])]; 2];
            domain[1 - section.axis()] = rect[0];
            for (axis, (knots, degree, count)) in [
                (
                    &source.knots_u,
                    source.degree_u,
                    source.control_points.len(),
                ),
                (
                    &source.knots_v,
                    source.degree_v,
                    source.control_points[0].len(),
                ),
            ]
            .into_iter()
            .enumerate()
            {
                if !continuous_knots(knots, degree, count, domain[axis]) {
                    return Ok((None, 0));
                }
            }
        }
        let midpoint = rect.map(|d| d[0] * 0.5 + d[1] * 0.5);
        let (p, mut spent) = self.enclose_with_transverse_correction(
            [midpoint[0]; 2],
            [midpoint[1]; 2],
            budget.min(100000),
        )?;
        let Some(p) = p else {
            return Ok((None, spent));
        };
        let ct = mapped(domains[0], [midpoint[1]; 2])?;
        let rt = mapped(domains[1], [midpoint[1]; 2])?;
        let used = spans(centers, ct) + spans(radius, rt);
        if used > budget - spent {
            return Ok((None, spent));
        }
        spent += used;
        let center = interval_eval::evaluate_interval(centers, ct)?;
        let law = interval_eval::evaluate_interval(radius, rt)?;
        let point = crate::moving_radius::bound_expression(
            p,
            std::array::from_fn(|k| [center[k].lo, center[k].hi]),
            [law[0].lo.max(0.), law[0].hi],
        )?;
        let (jet, used) = self.derivatives_with_transverse_correction(
            rect[0],
            rect[1],
            (budget - spent).min(100000),
        )?;
        spent += used;
        let Some(jet) = jet else {
            return Ok((None, spent));
        };
        let (cd, used) = law_derivative(
            centers,
            mapped(domains[0], rect[1])?,
            domains[0],
            budget - spent,
        )?;
        spent += used;
        let Some(cd) = cd else {
            return Ok((None, spent));
        };
        let (rd, used) = law_derivative(
            radius,
            mapped(domains[1], rect[1])?,
            domains[1],
            budget - spent,
        )?;
        spent += used;
        let Some(rd) = rd else {
            return Ok((None, spent));
        };
        let mut gradient = [I::point(0.); 2];
        for k in 0..3 {
            let delta = I::new(position[k][0], position[k][1])?.sub(c[k])?;
            gradient[0] = gradient[0].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[0][k][0], jet[0][k][1])?)?,
            )?;
            gradient[1] = gradient[1].add(
                I::point(2.)
                    .mul(delta)?
                    .mul(I::new(jet[1][k][0], jet[1][k][1])?.sub(cd[k])?)?,
            )?;
        }
        gradient[1] = gradient[1].sub(I::point(2.).mul(r)?.mul(rd[0])?)?;
        let mut residual = I::new(point.squared_residual[0], point.squared_residual[1])?;
        let mut contributions = [0.; 2];
        for axis in 0..2 {
            let contribution = gradient[axis]
                .mul(I::new(rect[axis][0], rect[axis][1])?.sub(I::point(midpoint[axis]))?)?;
            contributions[axis] = contribution.lo.abs().max(contribution.hi.abs());
            residual = residual.add(contribution)?;
        }
        Ok((Some(([residual.lo, residual.hi], contributions)), spent))
    }
    /// Center and radius curves are mapped over their complete domains by
    /// normalized across coordinate. Radius uses coordinate zero. All free
    /// coordinates participate; no sample or unchanged source admits a cell.
    pub fn inspect_radius(
        &self,
        centers: &Curve,
        radius: &Curve,
        tolerance: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<Report> {
        centers.validate()?;
        radius.validate()?;
        self.inspect_radius_on_domains(
            centers,
            radius,
            [centers.domain(), radius.domain()],
            tolerance,
            max_cells,
            max_spans,
        )
    }
    /// Restrict the original law parameters without rebuilding either curve.
    /// Each ordered domain maps affinely from the entire across interval.
    pub fn inspect_radius_on_domains(
        &self,
        centers: &Curve,
        radius: &Curve,
        domains: [[f64; 2]; 2],
        tolerance: f64,
        max_cells: usize,
        max_spans: usize,
    ) -> Result<Report> {
        centers.validate()?;
        radius.validate()?;
        for (curve, domain) in [centers, radius].into_iter().zip(domains) {
            let full = curve.domain();
            check(
                domain.iter().all(|x| x.is_finite())
                    && full[0] <= domain[0]
                    && domain[0] < domain[1]
                    && domain[1] <= full[1],
                "Keep original radius-law domain ordered inside its curve",
            )?;
        }
        check(
            centers.control_points[0].len() == 3
                && radius.control_points[0].len() == 2
                && radius.control_points.iter().all(|p| p[0] >= 0.)
                && tolerance.is_finite()
                && tolerance > 0.
                && max_cells <= 100000
                && max_spans <= 1000000,
            "Bound exact expression radius law and work",
        )?;
        let range = self.section(false).range();
        let mut out = Report {
            domain: [range, [0., 1.]],
            complete: false,
            cells: 0,
            spans: 0,
            accepted_cells: 0,
            accepted_error_upper: 0.,
            pending: vec![[range, [0., 1.]]],
            reason: "expression-radius-work-limit",
            violation: None,
        };
        while let Some(rect) = out.pending.pop() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                return Ok(out);
            }
            out.cells += 1;
            let (position, used) = self.enclose_with_transverse_correction(
                rect[0],
                rect[1],
                (max_spans - out.spans).min(100000),
            )?;
            out.spans += used;
            let Some(position) = position else {
                out.pending.push(rect);
                return Ok(out);
            };
            let ct = mapped(domains[0], rect[1])?;
            let rt = mapped(domains[1], rect[1])?;
            let used = spans(centers, ct) + spans(radius, rt);
            if used > max_spans - out.spans {
                out.pending.push(rect);
                return Ok(out);
            }
            out.spans += used;
            let c = interval_eval::evaluate_interval(centers, ct)?;
            let r = interval_eval::evaluate_interval(radius, rt)?;
            let mut bound = crate::moving_radius::bound_expression(
                position,
                std::array::from_fn(|axis| [c[axis].lo, c[axis].hi]),
                [r[0].lo.max(0.), r[0].hi],
            )?;
            let mut split_contributions = None;
            if bound.error_upper > tolerance && max_spans - out.spans >= 100 {
                let (residual, spent) = self.mean_radius_residual(
                    centers,
                    radius,
                    domains,
                    rect,
                    position,
                    &c,
                    r[0],
                    max_spans - out.spans,
                )?;
                out.spans += spent;
                if let Some((residual, contributions)) = residual {
                    split_contributions = Some(contributions);
                    bound = crate::moving_radius::refine_expression_bound(
                        bound,
                        residual,
                        [r[0].lo.max(0.), r[0].hi],
                    )?;
                }
            }
            // A rigorously enclosed point may refute the law, never admit the
            // rectangle. Probe initial interior points before depth-first coverage
            // spends all work near a boundary with zero residual.
            if out.cells == 1 && bound.error_upper > tolerance && max_spans - out.spans >= 34 {
                let interior = rect.map(|d| {
                    let middle = d[0] * 0.5 + d[1] * 0.5;
                    [middle, d[0] * 0.5 + middle * 0.5, middle * 0.5 + d[1] * 0.5]
                });
                for probe_index in 0..9 {
                    if max_spans - out.spans < 34 {
                        break;
                    }
                    let point = [
                        [interior[0][probe_index / 3]; 2],
                        [interior[1][probe_index % 3]; 2],
                    ];
                    let (p, used) = self.enclose_with_transverse_correction(
                        point[0],
                        point[1],
                        (max_spans - out.spans).min(100000),
                    )?;
                    out.spans += used;
                    if let Some(p) = p {
                        let ct = mapped(domains[0], point[1])?;
                        let rt = mapped(domains[1], point[1])?;
                        let used = spans(centers, ct) + spans(radius, rt);
                        if used <= max_spans - out.spans {
                            out.spans += used;
                            let c = interval_eval::evaluate_interval(centers, ct)?;
                            let r = interval_eval::evaluate_interval(radius, rt)?;
                            let probe = crate::moving_radius::bound_expression(
                                p,
                                std::array::from_fn(|axis| [c[axis].lo, c[axis].hi]),
                                [r[0].lo.max(0.), r[0].hi],
                            )?;
                            if probe.error_lower > tolerance {
                                out.violation = Some(Violation {
                                    domain: point,
                                    error_bounds: [probe.error_lower, probe.error_upper],
                                    squared_residual: probe.squared_residual,
                                });
                                out.pending.push(rect);
                                out.reason = "expression-radius-law-mismatch";
                                return Ok(out);
                            }
                        }
                    }
                }
            }
            if bound.error_upper <= tolerance {
                out.accepted_cells += 1;
                out.accepted_error_upper = out.accepted_error_upper.max(bound.error_upper);
                continue;
            }
            if bound.error_lower > tolerance {
                out.violation = Some(Violation {
                    domain: rect,
                    error_bounds: [bound.error_lower, bound.error_upper],
                    squared_residual: bound.squared_residual,
                });
                out.pending.push(rect);
                out.reason = "expression-radius-law-mismatch";
                return Ok(out);
            }
            let balanced_axis = usize::from(
                rect[1][1] - rect[1][0] > (rect[0][1] - rect[0][0]) / (range[1] - range[0]),
            );
            // Split the dominant whole-cell residual contribution. This only
            // chooses coverage order; every child still requires interval proof.
            let axis = split_contributions
                .filter(|c| c[0] != c[1])
                .map_or(balanced_axis, |c| usize::from(c[1] > c[0]));
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                out.reason = "expression-radius-resolution-limit";
                return Ok(out);
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            out.pending.push(right);
            out.pending.push(left);
        }
        out.complete = true;
        out.reason = "expression-radius-complete-coverage";
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn law_jets_preserve_nonunit_parameter_scaling_and_work_refusal() {
        let curve = Curve {
            degree: 1,
            knots: vec![2., 2., 4., 4.],
            control_points: vec![vec![1., 2., 3.], vec![5., 6., 7.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        curve.validate().unwrap();
        let (derivative, spent) =
            law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 1).unwrap();
        assert_eq!(spent, 1);
        for d in derivative.unwrap() {
            assert!(d.lo <= 2. && d.hi >= 2.);
            assert!(d.hi - d.lo < 1e-10);
        }
        assert!(
            law_derivative(&curve, I::new(2.5, 3.5).unwrap(), [2.5, 3.5], 0)
                .unwrap()
                .0
                .is_none()
        );
    }
    #[test]
    fn rational_cylinder_radius_covers_whole_corrected_expression() {
        let w = std::f64::consts::FRAC_1_SQRT_2;
        let surface = crate::surface::Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [0., 1.]
                .into_iter()
                .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
                .collect(),
            weights: vec![vec![1., w, 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
            .unwrap()
            .0
            .unwrap();
        let bridge = crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0, 0.).unwrap(),
            profile.section(0, 1.).unwrap(),
            [1.; 2],
        )
        .unwrap();
        let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let centers = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let radius = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let report = corrected
            .inspect_radius(&centers, &radius, 0.1, 10000, 1000000)
            .unwrap();
        assert!(
            report.complete,
            "{} cells={} spans={} pending={}",
            report.reason,
            report.cells,
            report.spans,
            report.pending.len()
        );
        assert!(report.pending.is_empty());
        assert_eq!(report.pending_parameter_fraction(), 0.);
        assert!(report.accepted_error_upper <= 0.1);
        let mut extended = centers.clone();
        extended.control_points[0][0] = -1.;
        extended.control_points[1][0] = 2.;
        let restricted = corrected
            .inspect_radius_on_domains(
                &extended,
                &radius,
                [[1. / 3., 2. / 3.], [0.25, 0.75]],
                0.1,
                10000,
                1000000,
            )
            .unwrap();
        assert!(restricted.complete);
        assert!(restricted.pending.is_empty());
        assert!(corrected
            .inspect_radius_on_domains(&centers, &radius, [[-0.1, 1.], [0., 1.]], 0.1, 100, 10000)
            .is_err());
        assert!(corrected
            .inspect_radius_on_domains(&centers, &radius, [[0.5, 0.5], [0., 1.]], 0.1, 100, 10000)
            .is_err());
        for (cells, spans) in [(0, 1000), (100, 0), (1, 1)] {
            let refused = corrected
                .inspect_radius(&centers, &radius, 0.1, cells, spans)
                .unwrap();
            assert!(!refused.complete);
            assert!(!refused.pending.is_empty());
            assert_eq!(refused.pending_parameter_fraction(), 1.);
        }
        let mut wrong = radius.clone();
        // Radius equals the cylinder at the center but differs at interior
        // quarter parameters. A center-only refutation probe misses this law.
        let off_center_law = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1.2, 0.], vec![0.8, 0.], vec![1.2, 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let off_center = corrected
            .inspect_radius(&centers, &off_center_law, 0.01, 100, 10000)
            .unwrap();
        assert_eq!(off_center.reason, "expression-radius-law-mismatch");
        assert!(!off_center.complete);
        let violation = off_center.violation.as_ref().unwrap();
        assert!(violation.error_bounds[0] > 0.01);
        assert_eq!(off_center.cells, 1);
        assert_eq!(violation.domain[1][0], violation.domain[1][1]);
        assert_ne!(violation.domain[1][0], 0.5);
        for p in &mut wrong.control_points {
            p[0] = 2.;
        }
        let refused = corrected
            .inspect_radius(&centers, &wrong, 0.1, 100, 10000)
            .unwrap();
        assert!(!refused.complete);
        assert!(!refused.pending.is_empty());
        assert_eq!(refused.reason, "expression-radius-law-mismatch");
        let violation = refused.violation.unwrap();
        assert!(violation.error_bounds[0] > 0.1);
        assert!(violation.error_bounds[0] <= 1. && violation.error_bounds[1] >= 1.);
        let jump = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 0.5, 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 0.], vec![2., 0.], vec![2., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        assert!(jump.validate().is_err());
        assert!(corrected
            .inspect_radius(&centers, &jump, 0.1, 100, 1000)
            .is_err());
        let (mean, spent) = corrected
            .mean_radius_residual(
                &centers,
                &jump,
                [[0., 1.]; 2],
                [[0., 1.]; 2],
                [[0.; 2]; 3],
                &[I::point(0.); 3],
                I::point(1.),
                1000,
            )
            .unwrap();
        assert!(mean.is_none());
        assert_eq!(spent, 0);
    }
}
