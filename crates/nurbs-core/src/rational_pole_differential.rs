//! Source-owned exact rational pole decomposition and weighted chord columns.
use crate::{check, interval_eval::Interval as I, normal_alignment, surface::Surface, Result};
use cad_predicates::{
    AuthoredScalar, LeafRef, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
type Net = Vec<Vec<[I; 4]>>;
pub struct Certificate<'a> {
    surface: &'a Surface,
    end: bool,
    anchor: usize,
    a: [I; 3],
    weight: Net,
    lambda: Net,
    alpha: Net,
    e: Net,
    b: Net,
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub work: u64,
    pub reason: &'static str,
}
struct Builder<'a> {
    arena: &'a SourceArena,
    leaves: &'a [Vec<[LeafRef; 4]>],
    tolerance: &'a ToleranceContext,
    work: u64,
    limit: u64,
    reason: &'static str,
}
impl Builder<'_> {
    fn query(
        &mut self,
        coordinate: usize,
        axis: usize,
        anchor: Option<usize>,
        row: usize,
        column: Option<usize>,
    ) -> Option<cad_predicates::SurfaceDerivativeDecision> {
        if self.work == self.limit {
            self.reason = "rational-pole-work-limit";
            return None;
        }
        let mut ctx = PredicateContext::new(
            self.arena,
            self.tolerance,
            Limits {
                max_work: (self.limit - self.work).min(cad_predicates::MAX_WORK),
                ..Limits::default()
            },
            None,
        );
        let d = match (anchor, column) {
            (None, None) => cad_predicates::rational_surface_derivative_numerator_row_bounds(
                &mut ctx,
                self.leaves,
                coordinate,
                axis,
                row,
            ),
            (None, Some(c)) => {
                cad_predicates::rational_surface_derivative_numerator_coefficient_bounds(
                    &mut ctx,
                    self.leaves,
                    coordinate,
                    axis,
                    [row, c],
                )
            }
            (Some(a), None) => cad_predicates::rational_surface_derivative_transverse_row_bounds(
                &mut ctx,
                self.leaves,
                coordinate,
                a,
                axis,
                row,
            ),
            (Some(a), Some(c)) => {
                cad_predicates::rational_surface_derivative_transverse_coefficient_bounds(
                    &mut ctx,
                    self.leaves,
                    coordinate,
                    a,
                    axis,
                    [row, c],
                )
            }
        }
        .unwrap();
        self.work += d.work_used;
        Some(d)
    }
    fn grid(
        &mut self,
        coordinate: usize,
        axis: usize,
        anchor: Option<usize>,
        order: usize,
        divisor: I,
    ) -> Result<Option<Vec<Vec<I>>>> {
        let nu = 2 * (self.leaves.len() - 1) - usize::from(axis == 0);
        let nv = 2 * (self.leaves[0].len() - 1) - usize::from(axis == 1);
        let mut net = Vec::new();
        for u in 0..=nu {
            let Some(row) = self.query(coordinate, axis, anchor, u, None) else {
                return Ok(None);
            };
            let mut values = Vec::new();
            for v in 0..=nv {
                let (bounds, sign, scale) = if let (Some(b), Some(s), Some(scale)) =
                    (&row.coefficient_bounds, &row.signs, row.coefficient_scale)
                {
                    (b[0][v], s[0][v], scale)
                } else {
                    let Some(cell) = self.query(coordinate, axis, anchor, u, Some(v)) else {
                        return Ok(None);
                    };
                    let (Some(b), Some(s), Some(scale)) =
                        (cell.coefficient_bounds, cell.signs, cell.coefficient_scale)
                    else {
                        self.reason = "rational-pole-exact-work-unproven";
                        return Ok(None);
                    };
                    (b[0][0], s[0][0], scale)
                };
                if u < order {
                    if sign != Sign::Zero {
                        self.reason = "rational-pole-factor-unproven";
                        return Ok(None);
                    }
                    continue;
                }
                if scale[0] <= 0. {
                    self.reason = "rational-pole-scale-unproven";
                    return Ok(None);
                }
                let mut x = I::new(bounds[0], bounds[1])?
                    .div(I::new(scale[0], scale[1])?)?
                    .div_signed(divisor)?;
                for k in 0..order {
                    x = x.mul(I::point((nu - k) as f64).div(I::point((u - k) as f64))?)?;
                }
                values.push(x);
            }
            if u >= order {
                net.push(values);
            }
        }
        Ok(Some(net))
    }
}
fn empty(u: usize, v: usize) -> Net {
    vec![vec![[I::point(0.), I::point(0.), I::point(0.), I::point(1.)]; v]; u]
}
fn scalar(g: Vec<Vec<I>>) -> Net {
    g.into_iter()
        .map(|r| {
            r.into_iter()
                .map(|x| [x, I::point(0.), I::point(0.), I::point(1.)])
                .collect()
        })
        .collect()
}
fn choose(n: usize, k: usize) -> u64 {
    (0..k).fold(1, |x, i| x * (n - i) as u64 / (i + 1) as u64)
}
fn weight_square(weights: &[Vec<f64>]) -> Result<Net> {
    // Preparation admits degree <= 8. All binomial products below are exact
    // integers below 2^53; only their quotient needs outward rounding.
    let (p, q) = (weights.len() - 1, weights[0].len() - 1);
    let mut out = empty(2 * p + 1, 2 * q + 1);
    for u in 0..=2 * p {
        for v in 0..=2 * q {
            let mut sum = I::point(0.);
            for i in 0..=p {
                for j in 0..=q {
                    if u < i || u - i > p || v < j || v - j > q {
                        continue;
                    }
                    let numerator =
                        (choose(p, i) * choose(p, u - i) * choose(q, j) * choose(q, v - j)) as f64;
                    let denominator = (choose(2 * p, u) * choose(2 * q, v)) as f64;
                    let factor = I::point(numerator).div(I::point(denominator))?;
                    sum = sum.add(
                        I::point(weights[i][j])
                            .mul(I::point(weights[u - i][v - j]))?
                            .mul(factor)?,
                    )?;
                }
            }
            out[u][v][3] = sum;
        }
    }
    Ok(out)
}
fn rational_net(mut net: Net, denominator: &Net) -> Result<Net> {
    // Elevate the original polynomial numerator to W²'s tensor degrees.
    // Packing the common denominator allows restricted coefficient quotients
    // to retain correlations lost by dividing two independent range hulls.
    for axis in 0..2 {
        let target = if axis == 0 {
            denominator.len()
        } else {
            denominator[0].len()
        };
        while (if axis == 0 { net.len() } else { net[0].len() }) < target {
            let (nu, nv) = (net.len(), net[0].len());
            let mut elevated = empty(nu + usize::from(axis == 0), nv + usize::from(axis == 1));
            let degree = if axis == 0 { nu } else { nv };
            for u in 0..elevated.len() {
                for v in 0..elevated[0].len() {
                    let i = if axis == 0 { u } else { v };
                    if i == 0 {
                        elevated[u][v] = net[u][v];
                        continue;
                    }
                    if i == degree {
                        elevated[u][v] =
                            net[u - usize::from(axis == 0)][v - usize::from(axis == 1)];
                        continue;
                    }
                    let t = I::point(i as f64).div(I::point(degree as f64))?;
                    let one = I::point(1.).sub(t)?;
                    for k in 0..3 {
                        elevated[u][v][k] = net[u - usize::from(axis == 0)]
                            [v - usize::from(axis == 1)][k]
                            .mul(t)?
                            .add(net[u][v][k].mul(one)?)?;
                    }
                }
            }
            net = elevated;
        }
    }
    for u in 0..net.len() {
        for v in 0..net[0].len() {
            net[u][v][3] = denominator[u][v][3];
        }
    }
    Ok(net)
}
/// No input geometry is fitted or changed. Every dropped coefficient has an
/// exact Zero sign; all other magnitudes retain authored coefficient units.
pub fn prepare(s: &Surface, end: bool, max_work: u64) -> Result<Report<'_>> {
    s.validate()?;
    check(
        (1..=100_000_000).contains(&max_work),
        "Bound exact rational pole preparation work",
    )?;
    let (p, q) = (s.degree_u, s.degree_v);
    let single = |d: usize, k: &[f64], n: usize| {
        n == d + 1
            && k.len() == 2 * (d + 1)
            && k[..=d].iter().all(|x| *x == k[0])
            && k[d + 1..].iter().all(|x| *x == k[d + 1])
    };
    check(
        (2..=8).contains(&p)
            && (1..=8).contains(&q)
            && !s.periodic_u
            && !s.periodic_v
            && single(p, &s.knots_u, s.control_points.len())
            && single(q, &s.knots_v, s.control_points[0].len()),
        "Rational pole needs a single clamped original Bezier chart",
    )?;
    let mut source = s.clone();
    if end {
        source.control_points.reverse();
        source.weights.reverse();
    }
    let anchor = (0..3)
        .max_by(|a, b| {
            (source.control_points[1][0][*a] - source.control_points[0][0][*a])
                .abs()
                .total_cmp(
                    &(source.control_points[1][0][*b] - source.control_points[0][0][*b]).abs(),
                )
        })
        .unwrap();
    let mut direction = [I::point(0.); 3];
    for k in 0..3 {
        direction[k] = I::point(source.control_points[1][0][k])
            .sub(I::point(source.control_points[0][0][k]))?;
    }
    if direction[anchor].lo <= 0. && direction[anchor].hi >= 0. {
        return Ok(Report {
            certificate: None,
            work: 0,
            reason: "rational-pole-direction-unproven",
        });
    }
    let mut a = [I::point(0.); 3];
    for k in 0..3 {
        a[k] = if k == anchor {
            I::point(1.)
        } else {
            direction[k].div_signed(direction[anchor])?
        };
    }
    let values = source
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(u, row)| {
            let weights = &source.weights;
            row.iter()
                .enumerate()
                .flat_map(move |(v, p)| [p[0], p[1], p[2], weights[u][v]])
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("owned-rational-pole-differential", 1, values).unwrap();
    let leaves = (0..=p)
        .map(|u| {
            (0..=q)
                .map(|v| std::array::from_fn(|k| arena.leaf((u * (q + 1) + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut builder = Builder {
        arena: &arena,
        leaves: &leaves,
        tolerance: &tolerance,
        work: 0,
        limit: max_work,
        reason: "rational-pole-unproven",
    };
    let prepared = (|| -> Result<Option<Certificate<'_>>> {
        let Some(lambda) = builder.grid(anchor, 0, None, 0, I::point(1.))? else {
            return Ok(None);
        };
        let Some(alpha) = builder.grid(anchor, 1, None, 1, I::point(1.))? else {
            return Ok(None);
        };
        let mut e = empty(2 * p - 1, 2 * q + 1);
        let mut b = empty(2 * p - 1, 2 * q);
        for k in (0..3).filter(|k| *k != anchor) {
            let Some(eg) = builder.grid(k, 0, Some(anchor), 1, direction[anchor])? else {
                return Ok(None);
            };
            let Some(bg) = builder.grid(k, 1, Some(anchor), 2, direction[anchor])? else {
                return Ok(None);
            };
            for u in 0..eg.len() {
                for v in 0..eg[0].len() {
                    e[u][v][k] = eg[u][v];
                }
            }
            for u in 0..bg.len() {
                for v in 0..bg[0].len() {
                    b[u][v][k] = bg[u][v];
                }
            }
        }
        let mut weight = empty(p + 1, q + 1);
        for u in 0..=p {
            for v in 0..=q {
                weight[u][v][0] = I::point(source.weights[u][v]);
            }
        }
        let denominator = weight_square(&source.weights)?;
        Ok(Some(Certificate {
            surface: s,
            end,
            anchor,
            a,
            weight,
            lambda: rational_net(scalar(lambda), &denominator)?,
            alpha: rational_net(scalar(alpha), &denominator)?,
            e: rational_net(e, &denominator)?,
            b: rational_net(b, &denominator)?,
        }))
    })()?;
    let reason = if prepared.is_some() {
        "rational-pole-original-decomposition-qualified"
    } else {
        builder.reason
    };
    Ok(Report {
        certificate: prepared,
        work: builder.work,
        reason,
    })
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub(crate) fn end(&self) -> bool {
        self.end
    }
    pub fn anchor(&self) -> usize {
        self.anchor
    }
    /// Columns enclosing the weighted mean chord matrix and both endpoint tangent
    /// frames. A separated signed longitudinal derivative is required everywhere.
    pub fn bounds(&self, domain: [[f64; 2]; 2]) -> Result<Option<[[I; 2]; 3]>> {
        let s = self.surface;
        let original = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        check(
            (0..2).all(|a| {
                domain[a][0].is_finite()
                    && domain[a][1].is_finite()
                    && original[a][0] <= domain[a][0]
                    && domain[a][0] <= domain[a][1]
                    && domain[a][1] <= original[a][1]
            }),
            "Rational pole rectangle must lie in its original chart",
        )?;
        let mut d = [[0.; 2]; 2];
        for a in 0..2 {
            let length = I::point(original[a][1]).sub(I::point(original[a][0]))?;
            for e in 0..2 {
                let x = I::point(domain[a][e])
                    .sub(I::point(original[a][0]))?
                    .div(length)?
                    .intersect(0., 1.)?;
                d[a][e] = if e == 0 { x.lo } else { x.hi };
            }
        }
        if self.end {
            let lo = I::point(1.).sub(I::point(d[0][1]))?;
            let hi = I::point(1.).sub(I::point(d[0][0]))?;
            d[0] = [lo.lo.max(0.), hi.hi.min(1.)];
        }
        let hull = |net: &Net| {
            normal_alignment::restrict_hull(
                net,
                [net.len() - 1, net[0].len() - 1],
                [[0., 1.], [0., 1.]],
                d,
            )
        };
        let w = hull(&self.weight)?[0];
        check(w.lo > 0., "Positive original rational pole denominator")?;
        let quotient = |net: &Net| normal_alignment::rational_quotient_hull(net, d);
        let lambda = quotient(&self.lambda)?[0];
        if lambda.lo <= 0. && lambda.hi >= 0. {
            return Ok(None);
        }
        let alpha = quotient(&self.alpha)?[0];
        let eh = quotient(&self.e)?;
        let bh = quotient(&self.b)?;
        let first = I::new(0., d[0][1])?.div_signed(lambda)?;
        let second = alpha.div_signed(lambda)?.mul(I::new(0., 1.)?)?;
        let mut j = [[I::point(0.); 2]; 3];
        for k in 0..3 {
            let e = eh[k];
            let b = bh[k];
            j[k][0] = self.a[k].add(e.mul(first)?)?;
            j[k][1] = b.sub(e.mul(second)?)?;
        }
        Ok(Some(j))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn source() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|u| {
                    (0..2)
                        .map(|v| vec![u as f64 / 2., if u == 2 { v as f64 } else { 0. }, 0.])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1., 2.], vec![1., 3.], vec![1., 2.]],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_tangent_term_is_retained_and_original_frame_is_owned() {
        for end in [false, true] {
            let mut s = source();
            if end {
                s.control_points.reverse();
                s.weights.reverse();
            }
            let r = prepare(&s, end, 10_000_000).unwrap();
            let c = r.certificate.unwrap();
            assert!(std::ptr::eq(c.surface(), &s));
            assert_eq!(c.anchor(), 0);
            let d = [if end { [0.999, 1.] } else { [0., 0.001] }, [0., 0.1]];
            let j = c.bounds(d).unwrap().unwrap();
            assert!(j[0][0].contains(1.));
            assert!(j[1][1].lo > 0.);
            assert!(j[2][0].contains(0.) && j[2][1].contains(0.));
            let d = [d[0], [0., 0.0001]];
            let admitted =
                crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 5)
                    .unwrap();
            assert!(admitted.certificate.is_some());
            assert_eq!(admitted.spans, 5);
            let limited =
                crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 4)
                    .unwrap();
            assert!(limited.certificate.is_none() && !limited.complete);
            assert_eq!(limited.spans, 0);
            let foreign = s.clone();
            assert!(crate::surface_self_chord::qualify_rational_polar_rectangle(
                &foreign, d, &c, 1e-4, 5,
            )
            .is_err());
            let normal_report =
                crate::factored_surface_normals::prepare(&s, end, 10_000_000).unwrap();
            let normals = normal_report.certificate.unwrap();
            let near = if end { [1. - 1e-12, 1.] } else { [0., 1e-12] };
            let pair = [[near, [0., 1e-12]], [near, [0.245, 0.245 + 1e-12]]];
            let excluded = crate::surface_self_chord::rational_pole_pair_exclusion(
                &s, pair, &c, &normals, 1e-4, 7,
            )
            .unwrap();
            // Correlated numerator/denominator coefficients now keep the
            // wide angular pair nonsingular, unlike the old independent hulls.
            assert!(excluded.0 && excluded.1 <= 7);
            let narrow = [[near, [0., 1e-12]], [near, [0.1, 0.1 + 1e-12]]];
            let local = crate::surface_self_chord::rational_pole_pair_exclusion(
                &s, narrow, &c, &normals, 1e-4, 7,
            )
            .unwrap();
            assert!(local.0 && local.1 <= 7);
            assert_eq!(
                crate::surface_self_chord::rational_pole_pair_exclusion(
                    &s, pair, &c, &normals, 1e-4, 5
                )
                .unwrap(),
                (false, 5)
            );
            assert!(crate::surface_self_chord::rational_pole_pair_exclusion(
                &foreign, pair, &c, &normals, 1e-4, 7
            )
            .is_err());

            let limited = crate::surface_self_chord_coverage::qualify_with_rational_pole(
                &s,
                0.1,
                1e-4,
                crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
                &normals,
                &c,
            )
            .unwrap();
            assert!(limited.certificate.is_none() && limited.pending > 0 && limited.spans <= 1);
            assert!(
                crate::surface_self_chord_coverage::qualify_with_rational_pole(
                    &foreign,
                    0.1,
                    1e-4,
                    crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
                    &normals,
                    &c
                )
                .is_err()
            );
            assert!(c.bounds([[-0.1, 1.], [0., 1.]]).is_err());
        }
    }
    #[test]
    fn independent_rational_endpoint_frames_survive_scaled_domains() {
        // Direct quotient formulas, independent of coefficient preparation.
        for end in [false, true] {
            let mut s = source();
            if end {
                s.control_points.reverse();
                s.weights.reverse();
            }
            s.knots_u = vec![2., 2., 2., 7., 7., 7.];
            s.knots_v = vec![-3., -3., 9., 9.];
            let before = s.clone();
            let r = prepare(&s, end, 10_000_000).unwrap();
            let c = r.certificate.unwrap();
            let d = [if end { [6.995, 7.] } else { [2., 2.005] }, [-3., -1.8]];
            let j = c.bounds(d).unwrap().unwrap();
            for u in [0.0001, 0.0005, 0.001] {
                for v in [0., 0.05, 0.1] {
                    let w = 1. + v + 2. * u * (1. - u) * v;
                    let wu = 2. * (1. - 2. * u) * v;
                    let wv = 1. + 2. * u * (1. - u);
                    let x = u * (1. + 2. * v) - u * u * v;
                    let y = 2. * u * u * v;
                    let xu = ((1. + 2. * v - 2. * u * v) * w - x * wu) / (w * w);
                    let xv = ((2. * u - u * u) * w - x * wv) / (w * w);
                    let yu = (4. * u * v * w - y * wu) / (w * w);
                    let yv = (2. * u * u * w - y * wv) / (w * w);
                    assert!(j[1][0].contains(yu / xu));
                    assert!(j[1][1].contains((yv - yu * xv / xu) / (u * u)));
                    if v == 0.05 {
                        assert!(xv.abs() > 0.);
                    }
                }
            }
            assert_eq!(s.control_points, before.control_points);
            assert_eq!(s.weights, before.weights);
            assert_eq!(s.knots_u, before.knots_u);
            assert_eq!(s.knots_v, before.knots_v);
        }
    }
    #[test]
    fn analytic_cubic_chords_and_endpoint_frames_are_enclosed() {
        // Independent polynomial oracle S=(3u,3u²v,u³).
        let s = Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0usize..4)
                .map(|i| {
                    (0..2)
                        .map(|v| {
                            vec![
                                i as f64,
                                (i * i.saturating_sub(1)) as f64 / 2. * v as f64,
                                if i == 3 { 1. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let r = prepare(&s, false, 10_000_000).unwrap();
        let c = r.certificate.unwrap();
        let j = c.bounds([[0., 0.25], [0., 0.5]]).unwrap().unwrap();
        for (u0, v0, u1, v1) in [
            (0., 0., 0.25, 0.5),
            (0.25, 0.5, 0.01, 0.1),
            (0.1, 0.5, 0.1, 0.),
            (0., 0.1, 0., 0.4),
        ] {
            let (du, dv) = (u1 - u0, v1 - v0);
            let uv = u0 * v0 + (u0 * dv + v0 * du) / 2. + du * dv / 3.;
            let uu = u0 * u0 + u0 * du + du * du / 3.;
            let first = [1., 2. * uv, uu];
            let second = [0., 3., 0.];
            for k in 0..3 {
                assert!(j[k][0].contains(first[k]));
                assert!(j[k][1].contains(second[k]));
            }
            let p = |u: f64, v: f64| [3. * u, 3. * u * u * v, u * u * u];
            let (p0, p1) = (p(u0, v0), p(u1, v1));
            for k in 0..3 {
                let chord = first[k] * 3. * du + second[k] * dv * uu;
                assert!((chord - (p1[k] - p0[k])).abs() < 1e-14);
            }
            for (u, v) in [(u0, v0), (u1, v1)] {
                for (k, x) in [1., 2. * u * v, u * u].into_iter().enumerate() {
                    assert!(j[k][0].contains(x));
                }
            }
        }
    }
    #[test]
    fn partial_work_and_nonzero_transverse_coefficient_refuse() {
        let s = source();
        let r = prepare(&s, false, 1).unwrap();
        assert!(r.certificate.is_none() && r.work <= 1);
        let mut s = source();
        s.control_points[1][1][1] = 1e-12;
        assert!(prepare(&s, false, 10_000_000)
            .unwrap()
            .certificate
            .is_none());
    }
}
