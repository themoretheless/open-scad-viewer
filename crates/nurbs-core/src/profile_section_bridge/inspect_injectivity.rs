//! profile section bridge operations: inspect_injectivity.
use super::*;
impl<'a, 'b, 'c, 'd> ProfileSectionBridge<'a, 'b, 'c, 'd> {
/// Gale-Nikaido (1965), Theorem 4: a differentiable projected map on a
    /// closed rectangle is injective when its Jacobian is a P-matrix everywhere.
    /// In 2D all principal minors are j00, j11 and det(J). Complete interval
    /// coverage proves their positivity; symmetric positivity is also sufficient.
    /// https://fig.if.usp.br/~marchett/fluidos/jacobian-matrix-global-univalence-mapping_gale-nikaido.pdf
    pub fn inspect_injectivity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        projection: [[f64; 3]; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.validate(free, across, max_spans)?;
        check(
            max_cells <= 100000 && projection.iter().flatten().all(|x| x.is_finite()),
            "Bound injectivity work and projection",
        )?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "section-bridge-injectivity-work-limit",
        };
        // The transverse field must be single-valued and piecewise C1. Refuse
        // internal knots whose multiplicity would allow first-derivative jumps.
        for high in [false, true] {
            let surface = self.section(high).profile().source();
            for (knots, degree, count) in [
                (
                    &surface.knots_u,
                    surface.degree_u,
                    surface.control_points.len(),
                ),
                (
                    &surface.knots_v,
                    surface.degree_v,
                    surface.control_points[0].len(),
                ),
            ] {
                if knots.iter().any(|&k| {
                    k > knots[degree]
                        && k < knots[count]
                        && knots.iter().filter(|&&x| x == k).count() >= degree
                }) {
                    out.pending.push([free, across]);
                    out.reason = "section-bridge-injectivity-continuity-unproven";
                    return Ok(out);
                }
            }
        }
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (jet, spent) = self.derivative_bounds(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(jet) = jet else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            let mut j = [[I::point(0.); 2]; 2];
            for row in 0..2 {
                for axis in 0..2 {
                    for k in 0..3 {
                        j[row][axis] = j[row][axis].add(
                            I::point(projection[row][k])
                                .mul(I::new(jet[axis][k][0], jet[axis][k][1])?)?,
                        )?;
                    }
                }
            }
            if projected_principal_minors_positive(j)? {
                out.regular.push(rect);
                continue;
            }
            let range = self.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "section-bridge-injectivity-qualified"
        } else {
            "section-bridge-injectivity-resolution-limit"
        };
        Ok(out)
    }
/// Complete adaptive cover. Regularity is proved only by nonzero full-cell
    /// normal bounds; all unproved rectangles survive budget exhaustion.
    pub fn inspect_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.validate(free, across, max_spans)?;
        check(max_cells <= 100000, "Bound bridge cell work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "section-bridge-regularity-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) = self.normal_bounds(rect[0], rect[1], max_spans - out.spans)?;
            out.spans += spent;
            let Some(bounds) = bounds else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if bounds.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let range = self.section(false).range();
            let axis = usize::from(
                (rect[0][1] - rect[0][0]) / (range[1] - range[0]) < rect[1][1] - rect[1][0],
            );
            let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
            if mid <= rect[axis][0] || mid >= rect[axis][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[axis][1] = mid;
            let mut right = rect;
            right[axis][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "section-bridge-regularity-qualified"
        } else {
            "section-bridge-regularity-resolution-limit"
        };
        Ok(out)
    }
}
