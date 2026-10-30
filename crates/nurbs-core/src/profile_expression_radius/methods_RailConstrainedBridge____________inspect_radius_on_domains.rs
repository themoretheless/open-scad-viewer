//! profile expression radius operations: inspect_radius_on_domains.
use super::*;
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
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
