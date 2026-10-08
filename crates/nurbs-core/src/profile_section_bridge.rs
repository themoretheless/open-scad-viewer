//! Exact two-ended Hermite transition. Boundary ownership is retained as
//! section expressions; this is not a sewn Surface or closed Body.
use crate::{Result, check, interval_eval::Interval as I, surface_weight_profile::ProfileSection};
fn projected_principal_minors_positive(j: [[I; 2]; 2]) -> Result<bool> {
    let off = j[0][1].add(j[1][0])?.mul(I::point(0.5))?;
    let symmetric = j[0][0].mul(j[1][1])?.sub(off.mul(off)?)?;
    let determinant = j[0][0].mul(j[1][1])?.sub(j[0][1].mul(j[1][0])?)?;
    Ok(j[0][0].lo > 0. && j[1][1].lo > 0. && (determinant.lo > 0. || symmetric.lo > 0.))
}
pub struct ProfileSectionBridge<'a, 'b, 'c, 'd> {
    sections: [Section<'a, 'b, 'c, 'd>; 2],
    scales: [f64; 2],
}
// Separate lifetimes permit independently owned source profiles.
enum Section<'a, 'b, 'c, 'd> {
    First(ProfileSection<'a, 'b>),
    Second(ProfileSection<'c, 'd>),
}
impl Section<'_, '_, '_, '_> {
    fn get(&self) -> &ProfileSection<'_, '_> {
        match self {
            Self::First(s) => s,
            Self::Second(s) => s,
        }
    }
}
impl<'a, 'b, 'c, 'd> ProfileSectionBridge<'a, 'b, 'c, 'd> {
    pub fn new(
        first: ProfileSection<'a, 'b>,
        second: ProfileSection<'c, 'd>,
        scales: [f64; 2],
    ) -> Result<Self> {
        check(
            first.range() == second.range() && scales.iter().all(|x| x.is_finite() && *x != 0.),
            "Require common original free parameters and finite nonzero tangent scales",
        )?;
        Ok(Self {
            sections: [Section::First(first), Section::Second(second)],
            scales,
        })
    }
    pub fn section(&self, high: bool) -> &ProfileSection<'_, '_> {
        self.sections[usize::from(high)].get()
    }
    pub fn rail(&self, high: bool) -> ProfileBridgeRail<'_, 'a, 'b, 'c, 'd> {
        ProfileBridgeRail {
            bridge: self,
            free: self.section(false).range()[usize::from(high)],
        }
    }
    /// Attach source-natural rails only when both sections and endpoint jets
    /// have the same exact affine support coordinate. Rounded scales refuse.
    pub fn constrain_original_rails(
        &self,
        max_controls: usize,
    ) -> Result<(Option<RailConstrainedBridge<'_, 'a, 'b, 'c, 'd>>, usize)> {
        check(max_controls <= 1000000, "Bound original rail control work")?;
        let a = self.section(false);
        let b = self.section(true);
        if !std::ptr::eq(a.profile(), b.profile()) || a.axis() != b.axis() {
            return Ok((None, 0));
        }
        let x = a.parameter();
        let y = b.parameter();
        let width = y - x;
        // Sterbenz guarantees this binary subtraction is exact. Zero is exact too.
        let exact = x == 0.
            || y == 0.
            || (x.signum() == y.signum() && x.abs() * 0.5 <= y.abs() && y.abs() <= x.abs() * 2.);
        if !exact || width == 0. || self.scales.iter().any(|s| s.to_bits() != width.to_bits()) {
            return Ok((None, 0));
        }
        let mut spent = 0;
        for high in [false, true] {
            let (boundary, used) =
                a.profile()
                    .preserved_boundary(1 - a.axis(), high, max_controls - spent)?;
            spent += used;
            if boundary.is_none() {
                return Ok((None, spent));
            }
        }
        Ok((Some(RailConstrainedBridge { bridge: self }), spent))
    }
    fn validate(&self, free: [f64; 2], across: [f64; 2], budget: usize) -> Result<()> {
        self.section(false).rectangle(free)?;
        self.section(true).rectangle(free)?;
        check(
            budget <= 100000
                && across.iter().all(|x| x.is_finite())
                && across[0] >= 0.
                && across[1] <= 1.
                && across[0] <= across[1],
            "Bound Hermite bridge domain and work",
        )
    }
    fn data(
        &self,
        free: [f64; 2],
        budget: usize,
        jet: bool,
    ) -> Result<(Option<Vec<Vec<[[f64; 2]; 3]>>>, usize)> {
        let mut values = Vec::new();
        let mut spent = 0;
        for high in [false, true] {
            let s = self.section(high);
            let mut side = Vec::new();
            for query in 0..if jet { 4 } else { 2 } {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (bounds, used) = match query {
                    0 => s.enclose(free, budget - spent)?,
                    1 => s.transverse_bounds(free, budget - spent)?,
                    2 => s.tangent_bounds(free, budget - spent)?,
                    _ => s
                        .profile()
                        .mixed_derivative_bounds(s.rectangle(free)?, budget - spent)?,
                };
                spent += used;
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                side.push(bounds);
            }
            values.push(side);
        }
        Ok((Some(values), spent))
    }
    pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        if across == [0., 0.] {
            return self.section(false).enclose(free, budget);
        }
        if across == [1., 1.] {
            return self.section(true).enclose(free, budget);
        }
        let (data, spent) = self.data(free, budget, false)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(q)?.mul(q)?.mul(I::point(self.scales[0]))?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
            I::point(-1.)
                .mul(t)?
                .mul(t)?
                .mul(q)?
                .mul(I::point(self.scales[1]))?,
        ];
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let mut value = I::point(0.);
            for (term, (side, slot)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
                value = value
                    .add(h[term].mul(I::new(data[side][slot][k][0], data[side][slot][k][1])?)?)?;
            }
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent))
    }
    /// Both section boundaries agree identically in position and tangent plane.
    /// Nonzero normals are still required; no interior regularity follows.
    pub fn regular_join(
        &self,
        high: bool,
        free: [f64; 2],
        budget: usize,
    ) -> Result<(Option<bool>, usize)> {
        let (n, spent) = self.section(high).support_normals(free, budget)?;
        Ok((
            n.and_then(|n| n.iter().any(|r| r[0] > 0. || r[1] < 0.).then_some(true)),
            spent,
        ))
    }
    pub fn derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.validate(free, across, budget)?;
        let (data, spent) = self.data(free, budget, true)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(q)?.mul(q)?.mul(I::point(self.scales[0]))?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
            I::point(-1.)
                .mul(t)?
                .mul(t)?
                .mul(q)?
                .mul(I::point(self.scales[1]))?,
        ];
        let d = [
            I::point(-6.).mul(t)?.mul(q)?,
            q.mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?
                .mul(I::point(self.scales[0]))?,
            I::point(6.).mul(t)?.mul(q)?,
            t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?
                .mul(I::point(self.scales[1]))?,
        ];
        let mut free_d = [I::point(0.); 3];
        let mut across_d = free_d;
        for k in 0..3 {
            for (term, (side, slot)) in [(0, 0), (0, 1), (1, 0), (1, 1)].into_iter().enumerate() {
                if slot == 1 {
                    across_d[k] = across_d[k].add(
                        d[term].mul(I::new(data[side][slot][k][0], data[side][slot][k][1])?)?,
                    )?;
                }
                free_d[k] = free_d[k].add(h[term].mul(I::new(
                    data[side][slot + 2][k][0],
                    data[side][slot + 2][k][1],
                )?)?)?;
            }
        }
        for k in 0..3 {
            let difference = I::new(data[1][0][k][0], data[1][0][k][1])?
                .sub(I::new(data[0][0][k][0], data[0][0][k][1])?)?;
            across_d[k] = across_d[k].add(d[2].mul(difference)?)?;
        }
        Ok((
            Some([free_d.map(|r| [r.lo, r.hi]), across_d.map(|r| [r.lo, r.hi])]),
            spent,
        ))
    }
    pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self.derivative_bounds(free, across, budget)?;
        let Some([a, b]) = jet else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let j = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[j][0], b[j][1])?)?
                .sub(I::new(a[j][0], a[j][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
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

/// Exact side-rail expression, retaining both incident section definitions.
pub struct ProfileBridgeRail<'r, 'a, 'b, 'c, 'd> {
    bridge: &'r ProfileSectionBridge<'a, 'b, 'c, 'd>,
    free: f64,
}
impl ProfileBridgeRail<'_, '_, '_, '_, '_> {
    pub fn free_parameter(&self) -> f64 {
        self.free
    }
    pub fn enclose(
        &self,
        range: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.enclose([self.free; 2], range, budget)
    }
    pub fn tangent_bounds(
        &self,
        range: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self
            .bridge
            .derivative_bounds([self.free; 2], range, budget)?;
        Ok((jet.map(|j| j[1]), spent))
    }
}
/// Position correction anchored to original natural support rails. It does
/// not inherit regularity/injectivity from the uncorrected Hermite bridge.
pub struct RailConstrainedBridge<'r, 'a, 'b, 'c, 'd> {
    bridge: &'r ProfileSectionBridge<'a, 'b, 'c, 'd>,
}
impl RailConstrainedBridge<'_, '_, '_, '_, '_> {
    fn source_rail(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let a = self.bridge.section(false);
        let b = self.bridge.section(true);
        let t = I::new(across[0], across[1])?;
        let coordinate = I::point(a.parameter())
            .add(t.mul(I::point(b.parameter()).sub(I::point(a.parameter()))?)?)?;
        let mut domain = [[0.; 2]; 2];
        domain[a.axis()] = [coordinate.lo, coordinate.hi];
        domain[1 - a.axis()] = [a.range()[usize::from(high)]; 2];
        let surface = a.profile().source();
        let axis = a.axis();
        let chart = if axis == 0 {
            [
                surface.knots_u[surface.degree_u],
                surface.knots_u[surface.control_points.len()],
            ]
        } else {
            [
                surface.knots_v[surface.degree_v],
                surface.knots_v[surface.control_points[0].len()],
            ]
        };
        domain[axis][0] = domain[axis][0].max(chart[0]);
        domain[axis][1] = domain[axis][1].min(chart[1]);
        a.profile().enclose(domain, budget)
    }
    pub fn enclose(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        let range = self.bridge.section(false).range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = base
            .map(|r| I::new(r[0], r[1]))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(I::point(range[1]).sub(I::point(range[0]))?)?;
        for high in [false, true] {
            if spent == budget {
                return Ok((None, spent));
            }
            let (rail, used) = self.source_rail(high, across, budget - spent)?;
            spent += used;
            let Some(rail) = rail else {
                return Ok((None, spent));
            };
            if spent == budget {
                return Ok((None, spent));
            }
            let (old, used) = self.bridge.rail(high).enclose(across, budget - spent)?;
            spent += used;
            let Some(old) = old else {
                return Ok((None, spent));
            };
            let weight = if high { t } else { I::point(1.).sub(t)? };
            for k in 0..3 {
                let error = I::new(rail[k][0], rail[k][1])?.sub(I::new(old[k][0], old[k][1])?)?;
                result[k] = result[k].add(weight.mul(error)?)?;
            }
        }
        Ok((
            Some(std::array::from_fn(|k| [result[k].lo, result[k].hi])),
            spent,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn principal_minors_admit_nonsymmetric_maps_and_refuse_orientation_failures() {
        let matrix = |m: [[f64; 2]; 2]| m.map(|row| row.map(I::point));
        // Positive principal minors, but indefinite symmetric part.
        assert!(projected_principal_minors_positive(matrix([[1., 8.], [-1., 1.]])).unwrap());
        assert!(!projected_principal_minors_positive(matrix([[-1., 0.], [0., -1.]])).unwrap());
        assert!(!projected_principal_minors_positive(matrix([[1., 2.], [2., 1.]])).unwrap());
        let uncertain = [
            [I::new(-0.1, 1.).unwrap(), I::point(0.)],
            [I::point(0.), I::point(1.)],
        ];
        assert!(!projected_principal_minors_positive(uncertain).unwrap());
    }
}
