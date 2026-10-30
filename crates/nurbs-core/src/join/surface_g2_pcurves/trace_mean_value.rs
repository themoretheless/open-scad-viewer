//! Original-chart mean-value bounds for the difference of two shape fields.
//! Third source jets bound derivatives; no sampled surface substitutes evidence.
//! Validated pcurves are C0 with bounded piecewise derivatives. Original C2
//! charts and proved regularity make the shape field continuous at crossed
//! knots, so its piecewise derivative bounds cover the complete integral.
use super::*;
use crate::{numerics::curve_jets, surface_measure::jets};

#[derive(Clone, Copy)]
struct D {
    v: I,
    d: I,
}
impl D {
    fn zero() -> Self {
        Self {
            v: I::point(0.),
            d: I::point(0.),
        }
    }
    fn add(self, b: Self) -> Result<Self> {
        Ok(Self {
            v: self.v.add(b.v)?,
            d: self.d.add(b.d)?,
        })
    }
    fn sub(self, b: Self) -> Result<Self> {
        Ok(Self {
            v: self.v.sub(b.v)?,
            d: self.d.sub(b.d)?,
        })
    }
    fn mul(self, b: Self) -> Result<Self> {
        Ok(Self {
            v: self.v.mul(b.v)?,
            d: self.d.mul(b.v)?.add(self.v.mul(b.d)?)?,
        })
    }
    fn div(self, b: Self) -> Result<Self> {
        Ok(Self {
            v: self.v.div(b.v)?,
            d: self.d.mul(b.v)?.sub(self.v.mul(b.d)?)?.div(b.v.mul(b.v)?)?,
        })
    }
    fn neg(self) -> Result<Self> {
        Ok(Self {
            v: I::new(-self.v.hi, -self.v.lo)?,
            d: I::new(-self.d.hi, -self.d.lo)?,
        })
    }
}
fn dot(a: [D; 3], b: [D; 3]) -> Result<D> {
    (0..3).try_fold(D::zero(), |x, k| x.add(a[k].mul(b[k])?))
}
pub(super) fn shape_derivative(j: &jets::Jets, r: [I; 2]) -> Result<Option<Vec<I>>> {
    let field = |a: usize, b: usize| -> Result<[D; 3]> {
        let mut v = [D::zero(); 3];
        for k in 0..3 {
            v[k] = D {
                v: j[a][b][k],
                d: j[a + 1][b][k].mul(r[0])?.add(j[a][b + 1][k].mul(r[1])?)?,
            };
        }
        Ok(v)
    };
    shape_derivative_fields(field)
}
fn shape_derivative_fields(field: impl Fn(usize, usize) -> Result<[D; 3]>) -> Result<Option<Vec<I>>> {
    let (u, v) = (field(1, 0)?, field(0, 1)?);
    let mut n = [D::zero(); 3];
    for k in 0..3 {
        let (a, b) = ((k + 1) % 3, (k + 2) % 3);
        n[k] = u[a].mul(v[b])?.sub(u[b].mul(v[a])?)?;
    }
    let w = n.map(|x| x.v);
    let (lo, hi) = box_distance(&w, &[I::point(0.); 3])?;
    if lo <= 0. {
        return Ok(None);
    }
    let area_value = I::new(lo, hi)?;
    let area = D {
        v: area_value,
        d: dot(
            n,
            n.map(|x| D {
                v: x.d,
                d: I::point(0.),
            }),
        )?
        .v
        .div(area_value)?,
    };
    for x in &mut n {
        *x = x.div(area)?;
    }
    let determinant = area.mul(area)?;
    let (e, f, g) = (dot(u, u)?, dot(u, v)?, dot(v, v)?);
    let inv = [
        [g.div(determinant)?, f.neg()?.div(determinant)?],
        [f.neg()?.div(determinant)?, e.div(determinant)?],
    ];
    let second = [
        [dot(n, field(2, 0)?)?, dot(n, field(1, 1)?)?],
        [dot(n, field(1, 1)?)?, dot(n, field(0, 2)?)?],
    ];
    let mut coefficients = [[D::zero(); 2]; 2];
    for a in 0..2 {
        for b in 0..2 {
            for k in 0..2 {
                for l in 0..2 {
                    coefficients[a][b] =
                        coefficients[a][b].add(inv[a][k].mul(second[k][l])?.mul(inv[l][b])?)?;
                }
            }
        }
    }
    let vectors = [u, v];
    let mut out = Vec::with_capacity(12);
    out.extend_from_slice(&n.map(|x| x.d));
    for a in 0..3 {
        for b in 0..3 {
            let mut value = D::zero();
            for k in 0..2 {
                for l in 0..2 {
                    value =
                        value.add(vectors[k][a].mul(coefficients[k][l])?.mul(vectors[l][b])?)?;
                }
            }
            out.push(value.d);
        }
    }
    Ok(Some(out))
}
fn derivative(
    pc: &Curve,
    s: &Surface,
    q: [f64; 2],
    reverse: bool,
    work: &mut usize,
    max: usize,
) -> Result<Option<Vec<I>>> {
    if pc.periodic
        || s.periodic_u
        || s.periodic_v
        || pc.control_points.len() > 4096
        || s.control_points
            .len()
            .saturating_mul(s.control_points[0].len())
            > 4096
    {
        return Ok(None);
    }
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    // This exact positive-rational hull containment permits clipping rounding
    // expansion in UV bounds, including fixed natural-boundary coordinates.
    if !pc
        .control_points
        .iter()
        .all(|p| (0..2).all(|k| domain[k][0] <= p[k] && p[k] <= domain[k][1]))
    {
        return Ok(None);
    }
    let d = pc.domain();
    let a = crate::curve_secant_trace::map(d, q[0], reverse)?;
    let b = crate::curve_secant_trace::map(d, q[1], reverse)?;
    let t = [a.lo.min(b.lo), a.hi.max(b.hi)];
    let signed = I::point(d[1])
        .sub(I::point(d[0]))?
        .mul(I::point(if reverse { -1. } else { 1. }))?;
    let mut result: Option<Vec<I>> = None;
    for span in pc.degree..pc.control_points.len() {
        let clipped = [t[0].max(pc.knots[span]), t[1].min(pc.knots[span + 1])];
        if clipped[0] >= clipped[1] {
            continue;
        }
        if *work == max {
            return Ok(None);
        }
        *work += 1;
        let p = curve_jets::enclose(pc, span, clipped)?;
        let factor = signed.div(I::point(clipped[1]).sub(I::point(clipped[0]))?)?;
        let rates = [p[1][0].mul(factor)?, p[1][1].mul(factor)?];
        let uv = [
            p[0][0].intersect(domain[0][0], domain[0][1])?,
            p[0][1].intersect(domain[1][0], domain[1][1])?,
        ];
        for (axis, (degree, knots, n)) in [
            (s.degree_u, &s.knots_u, s.control_points.len()),
            (s.degree_v, &s.knots_v, s.control_points[0].len()),
        ]
        .into_iter()
        .enumerate()
        {
            let max_mult = degree.saturating_sub(2);
            let mut i = 0;
            while i < knots.len() {
                let k = knots[i];
                let mut j = i + 1;
                while j < knots.len() && knots[j] == k {
                    j += 1;
                }
                if knots[degree] < k
                    && k < knots[n]
                    && uv[axis].lo <= k
                    && k <= uv[axis].hi
                    && (j - i) > max_mult
                {
                    return Ok(None);
                }
                i = j;
            }
        }
        let knots = [&s.knots_u, &s.knots_v];
        for u in s.degree_u..s.control_points.len() {
            for v in s.degree_v..s.control_points[0].len() {
                let spans = [u, v];
                let cell = std::array::from_fn(|k| {
                    [
                        uv[k].lo.max(knots[k][spans[k]]),
                        uv[k].hi.min(knots[k][spans[k] + 1]),
                    ]
                });
                if (0..2).any(|k| {
                    knots[k][spans[k]] >= knots[k][spans[k] + 1] || cell[k][0] > cell[k][1]
                }) {
                    continue;
                }
                if *work == max {
                    return Ok(None);
                }
                *work += 1;
                let width = [
                    I::point(s.knots_u[u + 1]).sub(I::point(s.knots_u[u]))?,
                    I::point(s.knots_v[v + 1]).sub(I::point(s.knots_v[v]))?,
                ];
                let j = jets::calculate_partial_stable(s, spans, cell, [None; 2])?;
                let Some(bound) =
                    shape_derivative(&j, [rates[0].div(width[0])?, rates[1].div(width[1])?])?
                else {
                    return Ok(None);
                };
                if let Some(out) = &mut result {
                    for (x, y) in out.iter_mut().zip(bound) {
                        x.lo = x.lo.min(y.lo);
                        x.hi = x.hi.max(y.hi);
                    }
                } else {
                    result = Some(bound);
                }
            }
        }
    }
    Ok(result)
}
pub(super) fn upper(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<[[f64; 2]; 2]>> {
    Ok(bounds(p, s, reverse, t, work, max)?.map(|(upper, _)| upper))
}
/// Whole-cell upper enclosure and midpoint lower witnesses from the SAME
/// original interval fields. No second midpoint query is needed.
pub(super) fn bounds(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<([[f64; 2]; 2], [[f64; 2]; 2])>> {
    if t[0] >= t[1] {
        return Ok(None);
    }
    let mid = t[0] * 0.5 + t[1] * 0.5;
    let mut center = [[I::point(0.); 12]; 2];
    let mut derivatives = Vec::with_capacity(2);
    for k in 0..2 {
        let Some(uv) = uv_box(p[k], [mid, mid], reverse[k], work, max)? else {
            return Ok(None);
        };
        let report = crate::surface_shape_operator::rectangle_bounds(s[k], uv, max - *work)?;
        *work += report.cells;
        let (Some(n), Some(b)) = (report.shape.normal, report.shape.tensor) else {
            return Ok(None);
        };
        for i in 0..3 {
            center[k][i] = I::new(n[i][0], n[i][1])?;
        }
        for i in 0..3 {
            for j in 0..3 {
                center[k][3 + 3 * i + j] = I::new(b[i][j][0], b[i][j][1])?;
            }
        }
        let Some(d) = derivative(p[k], s[k], t, reverse[k], work, max)? else {
            return Ok(None);
        };
        derivatives.push(d);
    }
    let delta = I::new(t[0], t[1])?.sub(I::point(mid))?;
    let mut out = [[0.; 2]; 2];
    let mut lower = [[0.; 2]; 2];
    for sign in 0..2 {
        let scale = I::point(if sign == 0 { 1. } else { -1. });
        let mut residual = [I::point(0.); 12];
        let mut midpoint = [I::point(0.); 12];
        for k in 0..12 {
            let mid_k = center[0][k].sub(center[1][k].mul(scale)?)?;
            midpoint[k] = mid_k;
            residual[k] =
                mid_k.add(delta.mul(derivatives[0][k].sub(derivatives[1][k].mul(scale)?)?)?)?;
        }
        lower[sign] = [
            box_distance(&midpoint[..3], &[I::point(0.); 3])?.0,
            box_distance(&midpoint[3..], &[I::point(0.); 9])?.0,
        ];
        out[sign] = [
            box_distance(&residual[..3], &[I::point(0.); 3])?.1.min(2.),
            box_distance(&residual[3..], &[I::point(0.); 9])?.1,
        ];
    }
    Ok(Some((out, lower)))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn midpoint_witness_bounds_cover_independent_graph_shape_errors() {
        let source = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    [0., 1.]
                        .iter()
                        .map(|&v| vec![i as f64 / 2., v, if i == 2 { 0.25 + v } else { v }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let mut plane = source.clone();
        for row in &mut plane.control_points {
            for point in row {
                point[2] = point[1];
            }
        }
        let pc = Curve::from_polyline(vec![vec![0.25, 0.5], vec![0.75, 0.5]]).unwrap();
        let mut work = 0;
        let (upper, lower) = bounds(
            [&pc, &pc],
            [&source, &plane],
            [false; 2],
            [0.49, 0.51],
            &mut work,
            100000,
        )
        .unwrap()
        .unwrap();
        let slope = 0.25_f64;
        let length = (2. + slope * slope).sqrt();
        let normal = [-slope / length, -1. / length, 1. / length];
        let flat = [0., -1. / 2_f64.sqrt(), 1. / 2_f64.sqrt()];
        let normal_error = normal
            .iter()
            .zip(flat)
            .map(|(a, b)| (a - b) * (a - b))
            .sum::<f64>()
            .sqrt();
        let tensor_error = 1. / length.powi(3);
        for (k, expected) in [normal_error, tensor_error].into_iter().enumerate() {
            assert!(lower[0][k] <= expected + 1e-12 && upper[0][k] >= expected - 1e-12);
            assert!(lower[0][k] > 0.1);
        }
        assert!(work <= 100000);
    }
    #[test]
    fn graph_shape_derivative_covers_independent_normal_and_tensor_formula() {
        // S=(u,v,u²/2+v²/4) at the origin, along increasing u.
        // N'=(-1,0,0), B'_{xz}=B'_{zx}=1, other tensor entries vanish.
        let mut j = [[[I::point(0.); 3]; 4]; 4];
        j[1][0][0] = I::point(1.);
        j[0][1][1] = I::point(1.);
        j[2][0][2] = I::point(1.);
        j[0][2][2] = I::point(0.5);
        let expected = [-1., 0., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0.];
        for sign in [1., -1.] {
            let bound = shape_derivative(&j, [I::point(sign), I::point(0.)])
                .unwrap()
                .unwrap();
            for (x, y) in bound.iter().zip(expected) {
                assert!(x.lo <= sign * y && sign * y <= x.hi);
                assert!(x.hi - x.lo < 1e-10);
            }
        }
        j[2][0][2] = I::point(0.);
        j[0][2][2] = I::point(0.);
        j[3][0][2] = I::point(6.);
        let cubic = shape_derivative(&j, [I::point(1.), I::point(0.)])
            .unwrap()
            .unwrap();
        for (k, x) in cubic.iter().enumerate() {
            let expected = if k == 3 { 6. } else { 0. };
            assert!(x.lo <= expected && expected <= x.hi);
            assert!(x.hi - x.lo < 1e-10);
        }
        j[0][1] = [I::point(0.); 3];
        assert!(
            shape_derivative(&j, [I::point(1.), I::point(0.)])
                .unwrap()
                .is_none()
        );
    }
}

#[path = "trace_mean_value/graph.rs"]
mod graph;
pub(super) fn graph_shape_derivative(j: &jets::Jets, r: [I; 2], frame: ([usize; 3], [f64; 2]))
    -> Result<Option<Vec<I>>> {
    graph::bounds(j, r, frame)
}
