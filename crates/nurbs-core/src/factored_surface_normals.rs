//! Exact original Bezier normal numerator with a verified boundary U² factor.
//! Limiting directions do not assert a unique regular tangent plane at a pole.
use crate::{check, interval_eval::Interval as I, surface::Surface, Result};
use cad_predicates::{
    AuthoredScalar, Limits, PredicateContext, Sign, SourceArena, ToleranceContext,
};
type Net = Vec<Vec<[I; 3]>>;
pub struct Certificate<'a> {
    surface: &'a Surface,
    end: bool,
    net: Net,
}
/// Bounds owned by one immutable original source and exact endpoint pair.
pub(crate) struct PairBounds<'a> {
    surface: &'a Surface,
    end: bool,
    pair: [[[f64; 2]; 2]; 2],
    components: [[[f64; 2]; 3]; 2],
}
impl PairBounds<'_> {
    pub(crate) fn components(
        &self, proof: &Certificate<'_>, pair: [[[f64; 2]; 2]; 2],
    ) -> Result<[[[f64; 2]; 3]; 2]> {
        check(std::ptr::eq(self.surface, proof.surface) && self.end == proof.end
            && self.pair == pair, "Use normal bounds for their original source and pair")?;
        Ok(self.components)
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub work: u64,
    pub reason: &'static str,
}
/// Diagnostic of two original one-sided limits at a collapsed U boundary.
/// This never supplies a regular normal or a seam admission certificate.
#[derive(Debug)]
pub struct BoundaryLimits {
    pub collapsed_point: Option<[f64; 3]>,
    pub components: [[[f64; 2]; 3]; 2],
    pub sine_squared_interval: Option<[f64; 2]>,
    pub distinct_tangent_limits: bool,
}
impl<'a> Certificate<'a> {
    pub fn boundary_limits(&self, max_sine_squared: f64) -> Result<BoundaryLimits> {
        let s = self.surface;
        let row = &s.control_points[if self.end { s.control_points.len() - 1 } else { 0 }];
        // The original clamped rational boundary is constant when every
        // Euclidean control point is exactly equal; positive weights preserve it.
        let collapsed_point = row.iter().all(|p| p == &row[0])
            .then(|| [row[0][0], row[0][1], row[0][2]]);
        let u = s.knots_u[if self.end { s.control_points.len() } else { s.degree_u }];
        let components = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]]
            .map(|v| self.bounds([[u, u], [v, v]]))
            .into_iter().collect::<Result<Vec<_>>>()?;
        let components = [components[0], components[1]];
        let angle = crate::normal_alignment::inspect_pair_bounds(components, max_sine_squared)?;
        Ok(BoundaryLimits {
            collapsed_point,
            components,
            sine_squared_interval: angle.sine_squared_interval,
            distinct_tangent_limits: collapsed_point.is_some() && angle.aligned == Some(false),
        })
    }
    pub(crate) fn pair_bounds(&self, pair: [[[f64; 2]; 2]; 2]) -> Result<PairBounds<'a>> {
        Ok(PairBounds {
            surface: self.surface, end: self.end, pair,
            components: [self.bounds(pair[0])?, self.bounds(pair[1])?],
        })
    }
    pub(crate) fn end(&self) -> bool {
        self.end
    }
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    /// Encloses the factored oriented normal numerator on every point of the
    /// original closed rectangle, including all one-sided boundary limits.
    pub fn bounds(&self, domain: [[f64; 2]; 2]) -> Result<[[f64; 2]; 3]> {
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
            "Factored normal rectangle must lie in the original chart",
        )?;
        let mut normalized = [[I::point(0.); 2]; 2];
        for a in 0..2 {
            let length = I::point(original[a][1]).sub(I::point(original[a][0]))?;
            for e in 0..2 {
                normalized[a][e] = I::point(domain[a][e])
                    .sub(I::point(original[a][0]))?
                    .div(length)?;
            }
        }
        if self.end {
            normalized[0] = [
                I::point(1.).sub(normalized[0][1])?,
                I::point(1.).sub(normalized[0][0])?,
            ];
        }
        let mut net = self.net.clone();
        for a in 0..2 {
            if a == 0 {
                for v in 0..net[0].len() {
                    let row = net.iter().map(|r| r[v]).collect::<Vec<_>>();
                    let restricted = restrict(&row, normalized[a], domain[a][0] == domain[a][1])?;
                    for u in 0..net.len() {
                        net[u][v] = restricted[u];
                    }
                }
            } else {
                for row in &mut net {
                    *row = restrict(row, normalized[a], domain[a][0] == domain[a][1])?;
                }
            }
        }
        Ok(std::array::from_fn(|k| {
            [
                net.iter()
                    .flatten()
                    .map(|p| p[k].lo)
                    .fold(f64::INFINITY, f64::min),
                net.iter()
                    .flatten()
                    .map(|p| p[k].hi)
                    .fold(f64::NEG_INFINITY, f64::max),
            ]
        }))
    }
}
fn split(row: &[[I; 3]], t: I) -> Result<(Vec<[I; 3]>, Vec<[I; 3]>)> {
    let t = t.intersect(0., 1.)?;
    let mut r = row.to_vec();
    let mut left = vec![r[0]];
    let mut right = vec![*r.last().unwrap()];
    let one = I::point(1.).sub(t)?.intersect(0., 1.)?;
    while r.len() > 1 {
        let mut next = Vec::new();
        for p in r.windows(2) {
            let mut value = [I::point(0.); 3];
            for k in 0..3 {
                // The exact convex blend belongs to the parent hull even when
                // interval dependency in t and (1-t) widens the arithmetic.
                value[k] = p[0][k].mul(one)?.add(p[1][k].mul(t)?)?.intersect(
                    p[0][k].lo.min(p[1][k].lo), p[0][k].hi.max(p[1][k].hi))?;
            }
            next.push(value);
        }
        r = next;
        left.push(r[0]);
        right.push(*r.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
fn restrict(row: &[[I; 3]], d: [I; 2], point: bool) -> Result<Vec<[I; 3]>> {
    if point {
        let value = *split(row, d[0])?.0.last().unwrap();
        return Ok(vec![value; row.len()]);
    }
    if d[1].lo <= f64::MIN_POSITIVE {
        // Normalization can round a subnormal positive upper endpoint across
        // zero. Enclose the entire original parameter interval directly; this
        // avoids an invalid reciprocal and never replaces original controls.
        let parameters = I::new(d[0].lo.min(d[1].lo).max(0.), d[0].hi.max(d[1].hi).min(1.))?;
        let value = *split(row, parameters)?.0.last().unwrap();
        return Ok(vec![value; row.len()]);
    }
    // Restrict the left piece at the upper endpoint, then take the right
    // piece at lower/upper. A positive upper endpoint is guaranteed by width.
    let left = split(row, d[1])?.0;
    Ok(split(&left, d[0].div(d[1])?)?.1)
}
/// Only exact original, clamped single Bezier charts are supported. All three
/// normal grids and every leading coefficient must be covered by bounded work.
pub fn prepare(s: &Surface, end: bool, max_work: u64) -> Result<Report<'_>> {
    Ok(prepare_factor_data(s,end,2,max_work)?.0)
}
fn prepare_factor_data(s:&Surface,end:bool,order:usize,max_work:u64)->Result<(Report<'_>,Option<usize>)> {
    s.validate()?;
    check(s.control_points[0][0].len()==3, "Factored normals require a 3D original surface")?;
    check(
        (1..=100_000_000).contains(&max_work),
        "Bound exact factored normal work",
    )?;
    let clamped = |p: usize, k: &[f64], n: usize| {
        (1..=8).contains(&p)
            && n == p + 1
            && k.len() == 2 * (p + 1)
            && k[..=p].iter().all(|x| *x == k[0])
            && k[p + 1..].iter().all(|x| *x == k[p + 1])
    };
    check(
        !s.periodic_u
            && !s.periodic_v
            && clamped(s.degree_u, &s.knots_u, s.control_points.len())
            && clamped(s.degree_v, &s.knots_v, s.control_points[0].len()),
        "Factored normals require a single clamped original Bezier chart",
    )?;
    let mut source = s.clone();
    if end {
        source.control_points.reverse();
        source.weights.reverse();
    }
    let nv = source.control_points[0].len();
    let values = source
        .control_points
        .iter()
        .enumerate()
        .flat_map(|(u, row)| {
            let w = &source.weights;
            row.iter()
                .enumerate()
                .flat_map(move |(v, p)| [p[0], p[1], p[2], w[u][v]])
        })
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("original-factored-normal", 1, values).unwrap();
    let leaves = (0..source.control_points.len())
        .map(|u| {
            (0..nv)
                .map(|v| std::array::from_fn(|k| arena.leaf((u * nv + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let nu = 3 * s.degree_u;
    let nv = 3 * s.degree_v;
    check((1..nu).contains(&order), "Choose a proper original normal factor order")?;
    let mut first_nonzero:Option<usize>=None;
    let mut net = vec![vec![[I::point(0.); 3]; nv]; nu - order];
    let mut out = Report {
        certificate: None,
        work: 0,
        reason: "factored-normal-work-limit",
    };
    for (axis, axes) in [[1, 2], [2, 0], [0, 1]].into_iter().enumerate() {
        for u in 0..nu {
            if out.work == max_work {
                return Ok((out,first_nonzero));
            }
            let mut ctx = PredicateContext::new(
                &arena,
                &tolerance,
                Limits {
                    max_work: (max_work - out.work).min(cad_predicates::MAX_WORK),
                    ..Limits::default()
                },
                None,
            );
            let row = cad_predicates::rational_surface_projected_jacobian_row_bounds(
                &mut ctx, &leaves, axes, u,
            )
            .unwrap();
            out.work += row.work_used;
            for v in 0..nv {
                let (bounds, sign) =
                    if let (Some(b), Some(signs)) = (&row.coefficient_bounds, &row.signs) {
                        (b[0][v], signs[0][v])
                    } else {
                        if out.work == max_work {
                            return Ok((out,first_nonzero));
                        }
                        let mut ctx = PredicateContext::new(
                            &arena,
                            &tolerance,
                            Limits {
                                max_work: (max_work - out.work).min(cad_predicates::MAX_WORK),
                                ..Limits::default()
                            },
                            None,
                        );
                        let cell =
                            cad_predicates::rational_surface_projected_jacobian_coefficient_bounds(
                                &mut ctx,
                                &leaves,
                                axes,
                                [u, v],
                            )
                            .unwrap();
                        out.work += cell.work_used;
                        let (Some(b), Some(signs)) = (cell.coefficient_bounds, cell.signs) else {
                            return Ok((out,first_nonzero));
                        };
                        (b[0][0], signs[0][0])
                    };
                if sign != Sign::Zero { first_nonzero=Some(first_nonzero.map_or(u,|i|i.min(u))); }
                if u < order {
                    if sign != Sign::Zero {
                        out.reason = "factored-normal-boundary-factor-unproven";
                        return Ok((out,first_nonzero));
                    }
                } else {
                    // B_i^n(t)/t² = n(n-1)/(i(i-1)) B_(i-2)^(n-2)(t).
                    let n = nu - 1;
                    let scale = if order==2 {
                        I::point((n * (n - 1)) as f64).div(I::point((u * (u - 1)) as f64))?
                    } else {
                        let mut factor=I::point(1.);
                        for j in 0..order {factor=factor.mul(I::point((n-j) as f64).div(I::point((u-j) as f64))?)?;}
                        factor
                    };
                    let x = I::new(bounds[0], bounds[1])?.mul(scale)?;
                    net[u - order][v][axis] = if end { I::new(-x.hi, -x.lo)? } else { x };
                }
            }
        }
    }
    out.certificate = Some(Certificate {
        surface: s,
        end,
        net,
    });
    out.reason = "factored-normal-original-grid-qualified";
    Ok((out,first_nonzero))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn convex_restriction_retains_constant_and_parent_hulls_at_closed_endpoints() {
        let constant = [I::point(1e100), I::point(-1e-100), I::point(0.)];
        let row = vec![constant; 24];
        for parameter in [I::new(-f64::EPSILON, f64::EPSILON).unwrap(),
            I::new(0., 1.).unwrap(), I::new(1.-f64::EPSILON, 1.+f64::EPSILON).unwrap()] {
            let (a,b) = split(&row, parameter).unwrap();
            for p in a.iter().chain(&b) {
                for k in 0..3 {
                    assert_eq!(p[k].lo, constant[k].lo);
                    assert_eq!(p[k].hi, constant[k].hi);
                }
            }
        }
        assert!(split(&row, I::new(2.,3.).unwrap()).is_err());
    }
    fn collapsed() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn collapsed_boundary_distinguishes_tangent_limits_without_regularizing_pole() {
        let mut s = collapsed();
        s.degree_v = 2;
        s.knots_v = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = vec![
            vec![vec![0., 0., 0.]; 3],
            vec![vec![0.5, 0., 0.]; 3],
            vec![vec![1., 0., 0.], vec![1., 0.5, 0.], vec![1., 1., 1.]],
        ];
        s.weights = vec![vec![1.; 3]; 3];
        // S=(u,u²v,u²v²). Independent limiting normals are (0,-2v,1).
        for end in [false, true] {
            if end { s.control_points.reverse(); }
            let p = prepare(&s, end, 100000000).unwrap().certificate.unwrap();
            let r = p.boundary_limits(1e-6).unwrap();
            assert_eq!(r.collapsed_point, Some([0.; 3]));
            assert!(r.distinct_tangent_limits);
            let angle = r.sine_squared_interval.unwrap();
            assert!(angle[0] <= 0.8 && angle[1] >= 0.8 && angle[0] > 0.79);
            let u = if end { 1. } else { 0. };
            let regular = crate::normal_alignment::normal_bounds(&s, [[u,u],[0.,0.]], 100).unwrap().0;
            assert!(regular.is_none_or(|n| n.iter().all(|r| r[0] <= 0. && r[1] >= 0.)));
        }
        let constant = collapsed();
        let r = prepare(&constant, false, 100000000).unwrap().certificate.unwrap()
            .boundary_limits(1e-6).unwrap();
        assert!(!r.distinct_tangent_limits);
        assert!(r.sine_squared_interval.unwrap()[1] <= 1e-6);
    }
    #[test]
    fn distinct_limits_at_different_points_do_not_prove_pole_divergence() {
        // S=(u³,u³v,v): U² divides its normal, but its U=0 boundary is a line.
        let s = Surface {
            degree_u: 3, degree_v: 1,
            knots_u: vec![0.,0.,0.,0.,1.,1.,1.,1.],
            knots_v: vec![0.,0.,1.,1.],
            control_points: vec![
                vec![vec![0.,0.,0.],vec![0.,0.,1.]],
                vec![vec![0.,0.,0.],vec![0.,0.,1.]],
                vec![vec![0.,0.,0.],vec![0.,0.,1.]],
                vec![vec![1.,0.,0.],vec![1.,1.,1.]],
            ],
            weights: vec![vec![1.;2];4], periodic_u:false, periodic_v:false,
        };
        let r = prepare(&s,false,100000000).unwrap().certificate.unwrap()
            .boundary_limits(1e-6).unwrap();
        assert_eq!(r.collapsed_point,None);
        assert!(r.sine_squared_interval.unwrap()[0] > 0.49);
        assert!(!r.distinct_tangent_limits);
    }
    #[test]
    fn limiting_bounds_cover_subnormal_rectangles_without_regularizing_the_pole() {
        let tiny=f64::from_bits(1);
        for end in [false,true] {
            let mut s=collapsed();if end{s.control_points.reverse();}
            let c=prepare(&s,end,1_000_000).unwrap().certificate.unwrap();
            let u=if end{[1f64.next_down(),1.]}else{[0.,tiny]};
            for v in [[0.,tiny],[0.,0.],[1f64.next_down(),1.]] {
                let n=c.bounds([u,v]).unwrap();
                assert!(n[0][0]<=0.&&n[0][1]>=0.);assert!(n[1][0]<=0.&&n[1][1]>=0.);
                assert!(if end{n[2][1]<0.}else{n[2][0]>0.});
            }
            let point=if end{1.}else{0.};
            let (regular,_)=crate::normal_alignment::normal_bounds(&s,[[point,point],[0.,0.]],100).unwrap();
            assert!(regular.is_none()||regular.unwrap().iter().all(|r|r[0]<=0.&&r[1]>=0.));
        }
    }
    #[test]
    fn exact_factor_covers_boundary_and_reversed_original_normals() {
        for end in [false, true] {
            let mut s = collapsed();
            if end {
                s.control_points.reverse();
            }
            let r = prepare(&s, end, 1_000_000).unwrap();
            let c = r.certificate.unwrap();
            assert!(std::ptr::eq(c.surface(), &s));
            for u in [[0., 1.], [0., 0.], [1., 1.], [0.2, 0.7]] {
                let n = c.bounds([u, [0., 1.]]).unwrap();
                assert!(n[0][0] <= 0. && n[0][1] >= 0.);
                assert!(n[1][0] <= 0. && n[1][1] >= 0.);
                assert!(if end { n[2][1] < 0. } else { n[2][0] > 0. });
            }
            assert!(c.bounds([[-0.1, 1.], [0., 1.]]).is_err());
        }
    }
    #[test]
    fn restricted_bounds_contain_independent_cubic_graph_normal_formula() {
        // S(u,v)=(3u,3u²v,u³), so normal-X/normal-Z=-u².
        // Every Bezier control is an exactly representable integer.
        let s = Surface {
            degree_u: 3,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..4)
                .map(|u| {
                    (0..2)
                        .map(|v| {
                            vec![
                                u as f64,
                                [0., 0., 1., 3.][u] * v as f64,
                                if u == 3 { 1. } else { 0. },
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
        for domain in [
            [[0., 1.], [0., 1.]],
            [[0.2, 0.7], [0.1, 0.8]],
            [[0.3, 0.3], [0.6, 0.6]],
            [[0., 0.], [0., 1.]],
            [[0., f64::from_bits(1)], [0., f64::from_bits(1)]],
            [[0., f64::MIN_POSITIVE], [0., 1.]],
            [[1., 1.], [0., 1.]],
        ] {
            let b = c.bounds(domain).unwrap();
            let ratio = I::new(b[0][0], b[0][1])
                .unwrap()
                .div(I::new(b[2][0], b[2][1]).unwrap())
                .unwrap();
            for u in [
                domain[0][0],
                (domain[0][0] + domain[0][1]) * 0.5,
                domain[0][1],
            ] {
                assert!(ratio.contains(-u * u), "{domain:?} {ratio:?} u={u}");
            }
        }
    }
    #[test]
    fn missing_factor_and_missing_work_never_admit() {
        let s = collapsed();
        let r = prepare(&s, false, 1).unwrap();
        assert!(r.certificate.is_none() && r.work <= 1);
        let mut s = collapsed();
        s.control_points[0][1][1] = 1.;
        assert!(prepare(&s, false, 1_000_000).unwrap().certificate.is_none());
    }
}

mod pole_plane;
pub use pole_plane::{PlaneLimit, PlaneLimitReport, PlanePairReport, prepare_plane_limit, compare_plane_limits};
