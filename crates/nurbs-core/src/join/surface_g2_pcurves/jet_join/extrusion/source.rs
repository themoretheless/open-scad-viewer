//! Compare the actual original strip to the cylinder over its exact trace.
//! No nearly extruded control or nearly constant pcurve is snapped to an axis.
use super::*;
use polynomial::Homogeneous;
pub(super) struct Source<'a> {
    _source: &'a Surface,
    net: Vec<Vec<[I; 4]>>,
    uv: [I; 2],
    velocity: [I; 2],
    transverse: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pub trace: Homogeneous,
    pub range: [f64; 2],
    #[cfg_attr(not(test), allow(dead_code))]
    pub origin: [f64; 3],
    spans: [usize; 2],
    path: [[f64; 2]; 2],
}
impl<'a> Source<'a> {
    pub fn new(s: &'a Surface, pc: &Curve, origin: [f64; 3]) -> Result<Self> {
        check(
            pc.weights[0] == pc.weights[1]
                && (1..=16).contains(&s.degree_u)
                && (1..=16).contains(&s.degree_v)
                && s.degree_u + s.degree_v <= 16,
            "Source cylindrical comparison needs an affine original clock and bounded trace degree",
        )?;
        let mut spans = [0; 2];
        for k in 0..2 {
            let (knots, degree, n) = if k == 0 {
                (&s.knots_u, s.degree_u, s.control_points.len())
            } else {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            };
            let a = pc.control_points[0][k];
            let b = pc.control_points[1][k];
            spans[k] = (degree..n)
                .find(|&i| {
                    knots[i] < knots[i + 1] && a.min(b) >= knots[i] && a.max(b) <= knots[i + 1]
                })
                .ok_or_else(|| crate::input("Source seam must remain on one original knot cell"))?;
        }
        Self::on_span(s, pc, origin, spans, [0., 1.])
    }
    pub fn cover(s: &'a Surface, pc: &Curve, origin: [f64; 3]) -> Result<Vec<Self>> {
        // Preserve the established single-cell arithmetic where applicable.
        let one_cell = [&s.knots_u, &s.knots_v]
            .iter()
            .all(|knots| knots.windows(2).filter(|w| w[0] < w[1]).count() == 1);
        if one_cell {
            return Self::new(s, pc, origin).map(|one| vec![one]);
        }
        check(
            pc.weights[0] == pc.weights[1]
                && (1..=16).contains(&s.degree_u)
                && (1..=16).contains(&s.degree_v)
                && s.degree_u + s.degree_v <= 16,
            "Source cover needs a bounded affine original clock",
        )?;
        let axes = [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ];
        let mut choices: [Vec<(usize, [f64; 2])>; 2] = std::array::from_fn(|_| Vec::new());
        for k in 0..2 {
            let (knots, p, n) = axes[k];
            let a = pc.control_points[0][k];
            let b = pc.control_points[1][k];
            check(
                a.min(b) >= knots[p] && a.max(b) <= knots[n],
                "Original affine seam leaves the source chart",
            )?;
            for i in p..n {
                if knots[i] >= knots[i + 1] || a.min(b) > knots[i + 1] || a.max(b) < knots[i] {
                    continue;
                }
                let range = if a == b {
                    [0., 1.]
                } else {
                    let t = I::new(knots[i], knots[i + 1])?
                        .sub(I::point(a))?
                        .div_signed(I::point(b).sub(I::point(a))?)?;
                    [t.lo.max(0.), t.hi.min(1.)]
                };
                if range[0] <= range[1] {
                    choices[k].push((i, range));
                }
            }
        }
        check(
            choices[0].len().saturating_mul(choices[1].len()) <= 256,
            "Original source knot cover exceeds 256 cells",
        )?;
        let mut out = Vec::new();
        for &(u, a) in &choices[0] {
            for &(v, b) in &choices[1] {
                let range = [a[0].max(b[0]), a[1].min(b[1])];
                if range[0] <= range[1] {
                    // Recenter arithmetic on ORIGINAL coefficients of this cell.
                    // This does not translate or refit either saved surface.
                    let local = std::array::from_fn(|k| {
                        s.control_points[u - s.degree_u][v - s.degree_v][k]
                    });
                    out.push(Self::on_span(s, pc, local, [u, v], range)?);
                }
            }
        }
        check(!out.is_empty(), "Original source knot cover is empty")?;
        Ok(out)
    }
    fn on_span(
        s: &'a Surface,
        pc: &Curve,
        origin: [f64; 3],
        spans: [usize; 2],
        range: [f64; 2],
    ) -> Result<Self> {
        let domain = [
            [s.knots_u[spans[0]], s.knots_u[spans[0] + 1]],
            [s.knots_v[spans[1]], s.knots_v[spans[1] + 1]],
        ];
        let net =
            crate::curve_surface_composition::surface_net_on_centered(s, spans, domain, origin)?;
        let homogeneous = (0..3)
            .map(|k| {
                (0..2)
                    .map(|i| I::point(if k == 2 { 1. } else { pc.control_points[i][k] }))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let trace = crate::curve_surface_composition::lift_coefficients(
            &homogeneous,
            &net,
            [s.degree_u, s.degree_v],
            domain,
        )?;
        let trace: Homogeneous = trace
            .try_into()
            .map_err(|_| crate::input("Invalid original trace dimension"))?;
        let mut uv = [I::point(0.); 2];
        let mut velocity = uv;
        for k in 0..2 {
            let width = I::point(domain[k][1]).sub(I::point(domain[k][0]))?;
            uv[k] = I::point(pc.control_points[0][k])
                .sub(I::point(domain[k][0]))?
                .div(width)?;
            velocity[k] = I::point(pc.control_points[1][k])
                .sub(I::point(pc.control_points[0][k]))?
                .div(width)?;
        }
        let dominant = if (pc.control_points[1][0] - pc.control_points[0][0]).abs()
            >= (pc.control_points[1][1] - pc.control_points[0][1]).abs()
        {
            0
        } else {
            1
        };
        check(
            velocity[dominant].lo > 0. || velocity[dominant].hi < 0.,
            "Source trace lacks a regular affine coordinate",
        )?;
        Ok(Self {
            _source: s,
            net,
            uv,
            velocity,
            transverse: 1 - dominant,
            trace,
            range,
            origin,
            spans,
            path: std::array::from_fn(|k| [pc.control_points[0][k], pc.control_points[1][k]]),
        })
    }
    /// A rejection witness must belong to this ORIGINAL source cell. An
    /// outward overlap band alone is insufficient to establish membership.
    pub fn contains_point(&self, ends: [I; 2], t: f64) -> Result<bool> {
        let fraction = ends[0]
            .add(ends[1].sub(ends[0])?.mul(I::point(t))?)?
            .intersect(0., 1.)?;
        for k in 0..2 {
            let knots = if k == 0 {
                &self._source.knots_u
            } else {
                &self._source.knots_v
            };
            let domain = [knots[self.spans[k]], knots[self.spans[k] + 1]];
            let [a, b] = self.path[k];
            if a == b {
                if a < domain[0] || a > domain[1] {
                    return Ok(false);
                }
            } else {
                let uv = I::point(a).add(I::point(b).sub(I::point(a))?.mul(fraction)?)?;
                if uv.lo < domain[0] || uv.hi > domain[1] {
                    return Ok(false);
                }
            }
        }
        Ok(true)
    }
    #[cfg(test)]
    /// Exact original product structure removes the stationary parameter from
    /// the projected trace before arithmetic. It is not an approximate iso
    /// extraction or a new source surface. Nonproducts keep the full lift.
    pub fn projected_trace(&self, axis: usize) -> Result<Homogeneous> {
        self.trace_on([I::point(0.), I::point(1.)], axis, self.origin)
    }
    /// Recenter ORIGINAL source coefficients directly at the pair, then
    /// compose the original affine clock once. The interval endpoints retain
    /// the exact relation; no rounded extracted curve acquires source credit.
    pub fn trace_on(&self, ends: [I; 2], axis: usize, origin: [f64; 3]) -> Result<Homogeneous> {
        let s = self._source;
        let domain = [
            [s.knots_u[self.spans[0]], s.knots_u[self.spans[0] + 1]],
            [s.knots_v[self.spans[1]], s.knots_v[self.spans[1] + 1]],
        ];
        let net = crate::curve_surface_composition::surface_net_on_centered(
            s, self.spans, domain, origin,
        )?;
        let mut input = Vec::new();
        for [a, b] in self.path {
            let values = if a == b {
                vec![I::point(a); 2]
            } else {
                ends.iter()
                    .map(|&t| I::point(a).add(I::point(b).sub(I::point(a))?.mul(t)?))
                    .collect::<Result<Vec<_>>>()?
            };
            input.push(values);
        }
        input.push(vec![I::point(1.); 2]);
        let lifted = if self.literal_axial_sign(axis).is_some() {
            let profile = 1 - self.transverse;
            let degree = if profile == 0 { s.degree_u } else { s.degree_v };
            let curve = (0..=degree)
                .map(|i| {
                    let mut h = if profile == 0 { net[i][0] } else { net[0][i] };
                    h[axis] = I::point(0.);
                    vec![h]
                })
                .collect::<Vec<_>>();
            let input = vec![
                input[profile].clone(),
                vec![I::point(0.); 2],
                vec![I::point(1.); 2],
            ];
            crate::curve_surface_composition::lift_coefficients(
                &input,
                &curve,
                [degree, 0],
                [domain[profile], [0., 1.]],
            )?
        } else {
            crate::curve_surface_composition::lift_coefficients(
                &input,
                &net,
                [s.degree_u, s.degree_v],
                domain,
            )?
        };
        lifted
            .try_into()
            .map_err(|_| crate::input("Invalid original trace dimension"))
    }
    pub fn outside(&self, ends: [I; 2], t: [f64; 2]) -> Result<bool> {
        let fraction = ends[0].add(ends[1].sub(ends[0])?.mul(I::new(t[0], t[1])?)?)?;
        for k in 0..2 {
            let uv = self.uv[k].add(self.velocity[k].mul(fraction)?)?;
            if uv.hi < 0. || uv.lo > 1. {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub fn bounds(
        &self,
        ends: [I; 2],
        t: [f64; 2],
        axis: usize,
    ) -> Result<Option<([[f64; 2]; 2], i8)>> {
        let t = I::new(t[0], t[1])?;
        let dt = ends[1].sub(ends[0])?;
        // The caller has already proved projected-trace regularity in Pair.
        // Literal product geometry needs no subtraction of noisy near-zero
        // mixed derivatives to prove equality with its own trace cylinder.
        if let Some(axial) = self.literal_axial_sign(axis) {
            let determinant = if self.transverse == 1 {
                self.velocity[0].mul(dt)?
            } else {
                I::point(0.).sub(self.velocity[1].mul(dt)?)?
            };
            if determinant.lo > 0. {
                return Ok(Some(([[0.; 2]; 2], axial)));
            }
            if determinant.hi < 0. {
                return Ok(Some(([[0.; 2]; 2], -axial)));
            }
        }
        let fraction = ends[0].add(dt.mul(t)?)?.intersect(0., 1.)?;
        let mut uv = [I::point(0.); 2];
        let mut d = uv;
        for k in 0..2 {
            uv[k] = self.uv[k]
                .add(self.velocity[k].mul(fraction)?)?
                .intersect(0., 1.)?;
            d[k] = self.velocity[k].mul(dt)?;
        }
        let original = jets::evaluate_homogeneous_jets(&self.net, uv, 2)?;
        let mut a = [[[I::point(0.); 3]; 4]; 4];
        for k in 0..3 {
            a[1][0][k] = original[1][0][k]
                .mul(d[0])?
                .add(original[0][1][k].mul(d[1])?)?;
            a[2][0][k] = original[2][0][k]
                .mul(d[0])?
                .mul(d[0])?
                .add(I::point(2.).mul(original[1][1][k])?.mul(d[0])?.mul(d[1])?)?
                .add(original[0][2][k].mul(d[1])?.mul(d[1])?)?;
            if self.transverse == 1 {
                a[0][1][k] = original[0][1][k];
                a[1][1][k] = original[1][1][k]
                    .mul(d[0])?
                    .add(original[0][2][k].mul(d[1])?)?;
                a[0][2][k] = original[0][2][k];
            } else {
                a[0][1][k] = original[1][0][k];
                a[1][1][k] = original[2][0][k]
                    .mul(d[0])?
                    .add(original[1][1][k].mul(d[1])?)?;
                a[0][2][k] = original[2][0][k];
            }
        }
        let det = if self.transverse == 1 {
            d[0]
        } else {
            I::point(0.).sub(d[1])?
        };
        let sign = |x: I| {
            if x.lo > 0. {
                Some(1)
            } else if x.hi < 0. {
                Some(-1)
            } else {
                None
            }
        };
        let (Some(det_sign), Some(axial_sign)) = (sign(det), sign(a[0][1][axis])) else {
            return Ok(None);
        };
        let mut b = a;
        let mut delta = [[I::point(0.); 3]; 5];
        for (slot, (u, v)) in [(1, 0), (2, 0), (1, 1), (0, 1), (0, 2)]
            .into_iter()
            .enumerate()
        {
            for k in 0..3 {
                let remove = if v == 0 { k == axis } else { k != axis };
                if remove {
                    b[u][v][k] = I::point(0.);
                    delta[slot][k] = I::point(0.).sub(a[u][v][k])?;
                }
            }
        }
        Ok(super::super::paired::bounds_full(&a, &b, delta)?
            .map(|bounds| (bounds, det_sign * axial_sign)))
    }
    pub(super) fn original_product_profile(&self, axis: usize) -> Option<(Curve, [f64; 2])> {
        self.literal_axial_sign(axis)?;
        let s = self._source;
        let profile = 1 - self.transverse;
        let (degree, knots, span) = if profile == 0 {
            (s.degree_u, &s.knots_u, self.spans[0])
        } else {
            (s.degree_v, &s.knots_v, self.spans[1])
        };
        let d = [knots[span], knots[span + 1]];
        let n = if profile == 0 {
            s.control_points.len()
        } else {
            s.control_points[0].len()
        };
        for t in d {
            let outside = t == knots[degree] || t == knots[n];
            if knots.iter().filter(|&&x| x == t).count() != degree + usize::from(outside) {
                return None;
            }
        }
        let mut points = Vec::new();
        let mut weights = Vec::new();
        for i in span - degree..=span {
            let (u, v) = if profile == 0 {
                (i, self.spans[1] - s.degree_v)
            } else {
                (self.spans[0] - s.degree_u, i)
            };
            points.push(s.control_points[u][v].clone());
            weights.push(s.weights[u][v]);
        }
        Some((
            Curve {
                degree,
                knots: [vec![d[0]; degree + 1], vec![d[1]; degree + 1]].concat(),
                control_points: points,
                weights,
                periodic: false,
            },
            self.path[profile],
        ))
    }
    fn literal_axial_sign(&self, axis: usize) -> Option<i8> {
        let s = self._source;
        let degrees = [s.degree_u, s.degree_v];
        let profile = 1 - self.transverse;
        let lo = [self.spans[0] - degrees[0], self.spans[1] - degrees[1]];
        let at = |i: usize, j: usize| if profile == 0 { (i, j) } else { (j, i) };
        let mut sign = None;
        for i in lo[profile]..=self.spans[profile] {
            let (u, v) = at(i, lo[self.transverse]);
            let p = &s.control_points[u][v];
            let w = s.weights[u][v];
            for j in lo[self.transverse]..=self.spans[self.transverse] {
                let (u, v) = at(i, j);
                let (a, b) = at(lo[profile], j);
                let q = &s.control_points[u][v];
                if s.weights[u][v] != w
                    || (0..3).any(|k| k != axis && q[k] != p[k])
                    || q[axis] != s.control_points[a][b][axis]
                {
                    return None;
                }
                if j > lo[self.transverse] {
                    let (a, b) = at(i, j - 1);
                    let previous = s.control_points[a][b][axis];
                    let next = if q[axis] > previous {
                        1
                    } else if q[axis] < previous {
                        -1
                    } else {
                        return None;
                    };
                    if sign.is_some_and(|s| s != next) {
                        return None;
                    }
                    sign = Some(next);
                }
            }
        }
        sign
    }
}
