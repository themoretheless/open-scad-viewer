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
        let surface = a.profile().source();
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
                return Ok((None, 0));
            }
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
        self.derivative_bounds_impl(free, across, budget, false)
    }
    fn derivative_bounds_impl(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
        common_difference: bool,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.validate(free, across, budget)?;
        let (data, mut spent) = self.data(free, budget, true)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let common = if common_difference {
            let a = self.section(false);
            let b = self.section(true);
            check(
                std::ptr::eq(a.profile(), b.profile()) && a.axis() == b.axis(),
                "Require common source difference ownership",
            )?;
            if spent == budget {
                return Ok((None, spent));
            }
            let (difference, used) = a.profile().coordinate_difference_bounds(
                a.axis(),
                [a.parameter(), b.parameter()],
                free,
                budget - spent,
            )?;
            spent += used;
            let Some(difference) = difference else {
                return Ok((None, spent));
            };
            Some(difference)
        } else {
            None
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
            let mut difference = I::new(data[1][0][k][0], data[1][0][k][1])?
                .sub(I::new(data[0][0][k][0], data[0][0][k][1])?)?;
            if let Some(common) = common {
                difference = difference.intersect(common[k][0], common[k][1])?;
            }
            across_d[k] = across_d[k].add(d[2].mul(difference)?)?;
        }
        Ok((
            Some([free_d.map(|r| [r.lo, r.hi]), across_d.map(|r| [r.lo, r.hi])]),
            spent,
        ))
    }
    /// Mixed derivative with respect to the original free parameter and
    /// normalized bridge coordinate. Uses original expression jets throughout.
    pub fn mixed_derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.validate(free, across, budget)?;
        let (data, spent) = self.data(free, budget, true)?;
        let Some(data) = data else {
            return Ok((None, spent));
        };
        let t = I::new(across[0], across[1])?;
        let q = I::point(1.).sub(t)?;
        let d0 = q
            .mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?
            .mul(I::point(self.scales[0]))?;
        let d1 = t
            .mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?
            .mul(I::point(self.scales[1]))?;
        let position = I::point(6.).mul(t)?.mul(q)?;
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let v =
                |side: usize, slot: usize| I::new(data[side][slot][k][0], data[side][slot][k][1]);
            let value = position
                .mul(v(1, 2)?.sub(v(0, 2)?)?)?
                .add(d0.mul(v(0, 3)?)?)?
                .add(d1.mul(v(1, 3)?)?)?;
            bounds[k] = [value.lo, value.hi];
        }
        Ok((Some(bounds), spent))
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
    fn source_rail_domain(&self, high: bool, across: [f64; 2]) -> Result<[[f64; 2]; 2]> {
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
        Ok(domain)
    }
    fn source_rail(
        &self,
        high: bool,
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge
            .section(false)
            .profile()
            .enclose(self.source_rail_domain(high, across)?, budget)
    }
    /// Position enclosure for the full side Hermite correction. This is a
    /// distinct expression from `enclose`: its normals and regularity must be
    /// qualified separately before assembly.
    pub fn enclose_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        let range = self.bridge.section(false).range();
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.enclose(free, across, budget);
        }
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                return self.source_rail(high, across, budget);
            }
        }
        let (base, mut spent) = self.bridge.enclose(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let position_weights = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let tangent_weights = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let mut result = [I::point(0.); 3];
        for k in 0..3 {
            result[k] = I::new(base[k][0], base[k][1])?;
        }
        for high in [false, true] {
            let section = self.bridge.section(false);
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (value, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let (jet, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        (
                            jet.map(|j| j.map(|component| component[1 - section.axis()])),
                            used,
                        )
                    }
                    _ => {
                        let (jet, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (jet.map(|j| j[0]), used)
                    }
                };
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                result[k] = result[k]
                    .add(position_weights[usize::from(high)].mul(v(0)?.sub(v(1)?)?)?)?
                    .add(tangent_weights[usize::from(high)].mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|v| [v.lo, v.hi])), spent))
    }
    /// First jets of the full side Hermite expression, not of the linear
    /// position-only correction. Shares every source query's work allowance.
    pub fn derivatives_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let section = self.bridge.section(false);
        let range = section.range();
        for high in [false, true] {
            if free == [range[usize::from(high)]; 2] {
                let (jet, used) = section
                    .profile()
                    .derivative_bounds(self.source_rail_domain(high, across)?, budget)?;
                let Some(jet) = jet else {
                    return Ok((None, used));
                };
                let mut out = [[[0.; 2]; 3]; 2];
                for k in 0..3 {
                    out[0][k] = jet[k][1 - section.axis()];
                    let v = I::new(jet[k][section.axis()][0], jet[k][section.axis()][1])?
                        .mul(I::point(self.bridge.scales[0]))?;
                    out[1][k] = [v.lo, v.hi];
                }
                return Ok((Some(out), used));
            }
        }
        let (base, mut spent) = self
            .bridge
            .derivative_bounds_impl(free, across, budget, true)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut out = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                out[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        let q = I::point(1.).sub(t)?;
        let h = [
            q.mul(q)?.mul(I::point(1.).add(I::point(2.).mul(t)?)?)?,
            t.mul(t)?.mul(I::point(3.).sub(I::point(2.).mul(t)?)?)?,
        ];
        let g = [
            t.mul(q)?.mul(q)?.mul(width)?,
            I::point(-1.).mul(t)?.mul(t)?.mul(q)?.mul(width)?,
        ];
        let dh = [
            I::point(-6.).mul(t)?.mul(q)?.div(width)?,
            I::point(6.).mul(t)?.mul(q)?.div(width)?,
        ];
        let dg = [
            q.mul(I::point(1.).sub(I::point(3.).mul(t)?)?)?,
            t.mul(I::point(3.).mul(t)?.sub(I::point(2.))?)?,
        ];
        for high in [false, true] {
            let domain = self.source_rail_domain(high, across)?;
            let rail_free = [range[usize::from(high)]; 2];
            let mut values = Vec::new();
            let mut source_jet: Option<[[[f64; 2]; 2]; 3]> = None;
            let mut bridge_jet: Option<[[[f64; 2]; 3]; 2]> = None;
            for query in 0..8 {
                if spent == budget && query != 4 && query != 5 {
                    return Ok((None, spent));
                }
                let (value, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 | 4 => {
                        let (j, used) = if query == 4 {
                            (source_jet, 0)
                        } else {
                            let (j, used) = section
                                .profile()
                                .derivative_bounds(domain, budget - spent)?;
                            source_jet = j;
                            (j, used)
                        };
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let axis = if query == 2 {
                                    1 - section.axis()
                                } else {
                                    section.axis()
                                };
                                let scale = if query == 2 {
                                    1.
                                } else {
                                    self.bridge.scales[0]
                                };
                                let x =
                                    I::new(j[k][axis][0], j[k][axis][1])?.mul(I::point(scale))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    3 | 5 => {
                        let (j, used) = if query == 5 {
                            (bridge_jet, 0)
                        } else {
                            let (j, used) = self.bridge.derivative_bounds_impl(
                                rail_free,
                                across,
                                budget - spent,
                                true,
                            )?;
                            bridge_jet = j;
                            (j, used)
                        };
                        (j.map(|j| j[usize::from(query == 5)]), used)
                    }
                    6 => {
                        let (j, used) = section
                            .profile()
                            .mixed_derivative_bounds(domain, budget - spent)?;
                        let mut v = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let x = I::new(j[k][0], j[k][1])?
                                    .mul(I::point(self.bridge.scales[0]))?;
                                v[k] = [x.lo, x.hi];
                            }
                            (Some(v), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => self
                        .bridge
                        .mixed_derivative_bounds(rail_free, across, budget - spent)?,
                };
                spent += used;
                let Some(value) = value else {
                    return Ok((None, spent));
                };
                values.push(value);
            }
            let i = usize::from(high);
            for k in 0..3 {
                let v = |i: usize| I::new(values[i][k][0], values[i][k][1]);
                out[0][k] = out[0][k]
                    .add(dh[i].mul(v(0)?.sub(v(1)?)?)?)?
                    .add(dg[i].mul(v(2)?.sub(v(3)?)?)?)?;
                out[1][k] = out[1][k]
                    .add(h[i].mul(v(4)?.sub(v(5)?)?)?)?
                    .add(g[i].mul(v(6)?.sub(v(7)?)?)?)?;
            }
        }
        Ok((Some(out.map(|axis| axis.map(|v| [v.lo, v.hi]))), spent))
    }
    pub fn normals_with_transverse_correction(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (jet, spent) = self.derivatives_with_transverse_correction(free, across, budget)?;
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
    pub fn inspect_transverse_corrected_regularity(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        self.bridge.validate(free, across, max_spans)?;
        check(max_cells <= 100000, "Bound bridge cell work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "transverse-corrected-bridge-regularity-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[free, across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (bounds, spent) =
                self.normals_with_transverse_correction(rect[0], rect[1], max_spans - out.spans)?;
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
            let range = self.bridge.section(false).range();
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
            // Finish refining a local rectangle before distributing work over
            // every unresolved breadth level. The untouched sibling remains
            // in the queue, preserving complete requested-domain coverage.
            queue.push_front(right);
            queue.push_front(left);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "transverse-corrected-bridge-regularity-qualified"
        } else {
            "transverse-corrected-bridge-regularity-resolution-limit"
        };
        Ok(out)
    }
    /// Exact side first-jet identity is established by the full Hermite
    /// expression. Only regular source support normals remain to be enclosed.
    /// This checks the original profile support, not neighboring Body ownership.
    pub fn inspect_transverse_corrected_side_join(
        &self,
        high: bool,
        across: [f64; 2],
        max_cells: usize,
        max_spans: usize,
    ) -> Result<crate::surface_weight_profile::TransitionRegularityReport> {
        let free = self.bridge.section(false).range()[usize::from(high)];
        self.bridge.validate([free; 2], across, max_spans)?;
        check(max_cells <= 100000, "Bound full side join work")?;
        let mut out = crate::surface_weight_profile::TransitionRegularityReport {
            regular: Vec::new(),
            pending: Vec::new(),
            cells: 0,
            spans: 0,
            complete_regular: false,
            reason: "full-side-join-work-limit",
        };
        let mut queue = std::collections::VecDeque::from([[[free; 2], across]]);
        while let Some(rect) = queue.pop_front() {
            if out.cells == max_cells || out.spans == max_spans {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            }
            out.cells += 1;
            let (normal, used) = self.bridge.section(false).profile().normal_bounds(
                self.source_rail_domain(high, rect[1])?,
                max_spans - out.spans,
            )?;
            out.spans += used;
            let Some(normal) = normal else {
                out.pending.push(rect);
                out.pending.extend(queue);
                return Ok(out);
            };
            if normal.iter().any(|r| r[0] > 0. || r[1] < 0.) {
                out.regular.push(rect);
                continue;
            }
            let mid = rect[1][0] * 0.5 + rect[1][1] * 0.5;
            if mid <= rect[1][0] || mid >= rect[1][1] {
                out.pending.push(rect);
                continue;
            }
            let mut left = rect;
            left[1][1] = mid;
            let mut right = rect;
            right[1][0] = mid;
            queue.push_back(left);
            queue.push_back(right);
        }
        out.complete_regular = out.pending.is_empty();
        out.reason = if out.complete_regular {
            "full-side-join-qualified"
        } else {
            "full-side-join-resolution-limit"
        };
        Ok(out)
    }
    pub fn derivative_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[[f64; 2]; 3]; 2]>, usize)> {
        self.bridge.validate(free, across, budget)?;
        if across == [0., 0.] || across == [1., 1.] {
            return self.bridge.derivative_bounds(free, across, budget);
        }
        let (base, mut spent) = self.bridge.derivative_bounds(free, across, budget)?;
        let Some(base) = base else {
            return Ok((None, spent));
        };
        let mut result = [[I::point(0.); 3]; 2];
        for axis in 0..2 {
            for k in 0..3 {
                result[axis][k] = I::new(base[axis][k][0], base[axis][k][1])?;
            }
        }
        let range = self.bridge.section(false).range();
        let width = I::point(range[1]).sub(I::point(range[0]))?;
        let t = I::new(free[0], free[1])?
            .sub(I::point(range[0]))?
            .div(width)?;
        for high in [false, true] {
            let mut values = Vec::new();
            for query in 0..4 {
                if spent == budget {
                    return Ok((None, spent));
                }
                let (bounds, used) = match query {
                    0 => self.source_rail(high, across, budget - spent)?,
                    1 => self.bridge.rail(high).enclose(across, budget - spent)?,
                    2 => {
                        let section = self.bridge.section(false);
                        let (j, used) = section.profile().derivative_bounds(
                            self.source_rail_domain(high, across)?,
                            budget - spent,
                        )?;
                        let mut bounds = [[0.; 2]; 3];
                        if let Some(j) = j {
                            for k in 0..3 {
                                let value =
                                    I::new(j[k][section.axis()][0], j[k][section.axis()][1])?
                                        .mul(I::point(self.bridge.scales[0]))?;
                                bounds[k] = [value.lo, value.hi];
                            }
                            (Some(bounds), used)
                        } else {
                            (None, used)
                        }
                    }
                    _ => {
                        let (j, used) = self.bridge.derivative_bounds(
                            [range[usize::from(high)]; 2],
                            across,
                            budget - spent,
                        )?;
                        (j.map(|j| j[1]), used)
                    }
                };
                spent += used;
                let Some(bounds) = bounds else {
                    return Ok((None, spent));
                };
                values.push(bounds);
            }
            let weight = if high { t } else { I::point(1.).sub(t)? };
            let sign = I::point(if high { 1. } else { -1. }).div(width)?;
            for k in 0..3 {
                let v = |q: usize| I::new(values[q][k][0], values[q][k][1]);
                result[0][k] = result[0][k].add(sign.mul(v(0)?.sub(v(1)?)?)?)?;
                result[1][k] = result[1][k].add(weight.mul(v(2)?.sub(v(3)?)?)?)?;
            }
        }
        Ok((Some(result.map(|axis| axis.map(|r| [r.lo, r.hi]))), spent))
    }
    pub fn normal_bounds(
        &self,
        free: [f64; 2],
        across: [f64; 2],
        budget: usize,
    ) -> Result<(Option<[[f64; 2]; 3]>, usize)> {
        let (j, spent) = self.derivative_bounds(free, across, budget)?;
        let Some([a, b]) = j else {
            return Ok((None, spent));
        };
        let mut bounds = [[0.; 2]; 3];
        for k in 0..3 {
            let i = (k + 1) % 3;
            let l = (k + 2) % 3;
            let n = I::new(a[i][0], a[i][1])?
                .mul(I::new(b[l][0], b[l][1])?)?
                .sub(I::new(a[l][0], a[l][1])?.mul(I::new(b[i][0], b[i][1])?)?)?;
            bounds[k] = [n.lo, n.hi];
        }
        Ok((Some(bounds), spent))
    }
    pub fn inspect_support_rail_normals(
        &self,
        high: bool,
        across: [f64; 2],
        threshold: f64,
        budget: usize,
    ) -> Result<crate::normal_alignment::PairReport> {
        check(
            threshold.is_finite() && (0. ..1.).contains(&threshold),
            "Bound constrained rail angle",
        )?;
        let free = self.bridge.section(false).range()[usize::from(high)];
        let (a, mut spent) = self.normal_bounds([free; 2], across, budget)?;
        let mut out = crate::normal_alignment::PairReport {
            aligned: None,
            sine_squared_interval: None,
            normal_components: [a, None],
            spans: spent,
            reason: "constrained-rail-normal-work-limit",
        };
        let Some(a) = a else {
            return Ok(out);
        };
        if spent == budget {
            return Ok(out);
        }
        let (b, used) = self
            .bridge
            .section(false)
            .profile()
            .normal_bounds(self.source_rail_domain(high, across)?, budget - spent)?;
        spent += used;
        let Some(b) = b else {
            out.spans = spent;
            return Ok(out);
        };
        let mut compared = crate::normal_alignment::inspect_pair_bounds([a, b], threshold)?;
        compared.spans = spent;
        Ok(compared)
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
    fn common_source_difference_tightens_translated_bridge_jet() {
        let surface = crate::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1e8, 0., 0.], vec![1e8, 1., 0.]],
                vec![vec![1e8 + 1., 0., 0.], vec![1e8 + 1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 4)
            .unwrap()
            .0
            .unwrap();
        let bridge = ProfileSectionBridge::new(
            profile.section(0, 0.5).unwrap(),
            profile.section(0, 0.75).unwrap(),
            [0.25; 2],
        )
        .unwrap();
        let old = bridge
            .derivative_bounds([0.25, 0.75], [0.25, 0.75], 8)
            .unwrap()
            .0
            .unwrap();
        let (new, spent) = bridge
            .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 9, true)
            .unwrap();
        let new = new.unwrap();
        assert_eq!(spent, 9);
        assert!(new[1][0][0] <= 0.25 && new[1][0][1] >= 0.25);
        assert!(new[1][0][1] - new[1][0][0] < old[1][0][1] - old[1][0][0]);
        assert!(
            bridge
                .derivative_bounds_impl([0.25, 0.75], [0.25, 0.75], 8, true)
                .unwrap()
                .0
                .is_none()
        );
    }
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
