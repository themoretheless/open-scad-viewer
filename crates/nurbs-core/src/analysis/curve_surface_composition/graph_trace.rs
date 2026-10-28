//! Correlated one-dimensional graph-jet residuals on authored pcurves.
//! A cell bound is not a whole seam certificate; traversal and fresh position
//! ownership remain the caller's responsibility.
use super::{
    Poly,
    exact_trace::{Support, Work},
    mul,
};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface};
pub(crate) struct CellBounds {
    pub upper: Option<[f64; 2]>,
    pub same_orientation: Option<bool>,
}
pub(crate) struct Pair {
    source: Support,
    candidate: Support,
    candidate_hulls: Vec<[I; 4]>,
    pub work: usize,
    pub max_work: usize,
}
pub(crate) fn curve_box(
    p: &Curve,
    range: [f64; 2],
    used: &mut usize,
    max: usize,
) -> Result<[[f64; 2]; 2]> {
    let mut w = Work {
        used: *used,
        limit: max,
    };
    let result = super::exact_trace::pcurve(p, range, [[0., 1.]; 2], &mut w);
    *used = w.used;
    let h = result?;
    let weight = hull(&h[2])?;
    let mut out = [[0.; 2]; 2];
    for k in 0..2 {
        let i = hull(&h[k])?.div_signed(weight)?;
        // Positive rational Bernstein convex hull of the original authored
        // Euclidean poles remains valid, including constant trim coordinates.
        let lo = p
            .control_points
            .iter()
            .map(|v| v[k])
            .fold(f64::INFINITY, f64::min);
        let hi = p
            .control_points
            .iter()
            .map(|v| v[k])
            .fold(f64::NEG_INFINITY, f64::max);
        out[k] = [i.lo.max(lo), i.hi.min(hi)];
    }
    Ok(out)
}
fn hull(p: &[I]) -> Result<I> {
    I::new(
        p.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min),
        p.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max),
    )
}
fn neg(x: I) -> Result<I> {
    I::new(-x.hi, -x.lo)
}
fn negative(p: &[I]) -> Result<Poly> {
    p.iter().copied().map(neg).collect()
}
fn sign(i: I) -> Option<bool> {
    if i.lo > 0. {
        Some(true)
    } else if i.hi < 0. {
        Some(false)
    } else {
        None
    }
}
impl Pair {
    pub(crate) fn new(source: &Surface, candidate: &Surface, max_work: usize) -> Result<Self> {
        check(
            candidate.weights.iter().flatten().all(|w| *w == 1.),
            "Correlated graph candidate needs unit polynomial weights",
        )?;
        // Piecewise patches must possess their second jets at every joining
        // knot. No traversal can silently skip a disconnected or C1 join.
        for s in [source, candidate] {
            for (p, k, n) in [
                (s.degree_u, &s.knots_u, s.control_points.len()),
                (s.degree_v, &s.knots_v, s.control_points[0].len()),
            ] {
                let mut index = p + 1;
                while index < n {
                    let mut end = index + 1;
                    while end < n && k[end] == k[index] {
                        end += 1;
                    }
                    check(
                        p >= end - index + 2,
                        "Graph trace needs C2 internal support knots",
                    )?;
                    index = end;
                }
            }
        }
        let mut w = Work {
            used: 0,
            limit: max_work,
        };
        let source = Support::new(source, &mut w)?;
        let candidate = Support::new(candidate, &mut w)?;
        let candidate_hulls = candidate.hulls()?;
        Ok(Self {
            source,
            candidate,
            candidate_hulls,
            work: w.used,
            max_work,
        })
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        if n > self.max_work.saturating_sub(self.work) {
            self.work = self.max_work;
            return Err(crate::numeric_err(
                "Correlated graph coefficient budget exhausted",
            ));
        }
        self.work += n;
        Ok(())
    }
    fn product(&mut self, a: &[I], b: &[I]) -> Result<Poly> {
        self.charge(12usize.saturating_mul(a.len()).saturating_mul(b.len()))?;
        mul(a, b)
    }
    fn sum(&mut self, a: &[I], b: &[I], subtract: bool) -> Result<Poly> {
        let degree = a.len().max(b.len()) - 1;
        let a = if a.len() == degree + 1 {
            a.to_vec()
        } else {
            self.product(a, &vec![I::point(1.); degree + 2 - a.len()])?
        };
        let b = if b.len() == degree + 1 {
            b.to_vec()
        } else {
            self.product(b, &vec![I::point(1.); degree + 2 - b.len()])?
        };
        self.charge(2 * (degree + 1))?;
        a.iter()
            .zip(b)
            .map(|(&x, y)| if subtract { x.sub(y) } else { x.add(y) })
            .collect()
    }
    fn scale(&mut self, a: &[I], k: I) -> Result<Poly> {
        self.charge(a.len())?;
        a.iter().map(|x| x.mul(k)).collect()
    }
    fn difference_products(&mut self, a: &[I], b: &[I], c: &[I], d: &[I]) -> Result<Poly> {
        let x = self.product(a, b)?;
        let y = self.product(c, d)?;
        self.sum(&x, &y, true)
    }
    /// Homogeneous source graph derivatives [gx,gy,Hxx,Hxy,Hyy].
    fn graph_fields(&mut self, h: &[Vec<Poly>]) -> Result<[(Poly, Poly); 5]> {
        let w = &h[0][3];
        let mut first = vec![vec![Vec::new(); 3]; 2];
        for i in 0..2 {
            for k in 0..3 {
                first[i][k] = self.difference_products(&h[i + 1][k], w, &h[0][k], &h[i + 1][3])?;
            }
        }
        let mut second = vec![vec![Vec::new(); 3]; 3];
        let w2 = self.product(w, w)?;
        for (ij, (a, b)) in [(0, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            for k in 0..3 {
                let mut value = self.product(&h[ij + 3][k], &w2)?;
                for (i, j) in [(a, b), (b, a)] {
                    let term = self.product(&h[i + 1][k], w)?;
                    let term = self.product(&term, &h[j + 1][3])?;
                    value = self.sum(&value, &term, true)?;
                }
                let term = self.product(&h[0][k], w)?;
                let term = self.product(&term, &h[ij + 3][3])?;
                value = self.sum(&value, &term, true)?;
                let term = self.product(&h[0][k], &h[a + 1][3])?;
                let term = self.product(&term, &h[b + 1][3])?;
                let term = self.scale(&term, I::point(2.))?;
                second[ij][k] = self.sum(&value, &term, false)?;
            }
        }
        let d = self.difference_products(&first[0][0], &first[1][1], &first[1][0], &first[0][1])?;
        let gx =
            self.difference_products(&first[0][2], &first[1][1], &first[1][2], &first[0][1])?;
        let gy =
            self.difference_products(&first[0][0], &first[1][2], &first[1][0], &first[0][2])?;
        let mut r = Vec::new();
        for jet in &second {
            let x = self.product(&jet[2], &d)?;
            let y = self.product(&gx, &jet[0])?;
            let x = self.sum(&x, &y, true)?;
            let y = self.product(&gy, &jet[1])?;
            r.push(self.sum(&x, &y, true)?);
        }
        let adj = [
            [first[1][1].clone(), negative(&first[1][0])?],
            [negative(&first[0][1])?, first[0][0].clone()],
        ];
        let mut output = vec![(gx, d.clone()), (gy, d.clone())];
        let d2 = self.product(&d, &d)?;
        let d3 = self.product(&d2, &d)?;
        for (a, b) in [(0, 0), (0, 1), (1, 1)] {
            let mut sum = vec![I::point(0.)];
            for i in 0..2 {
                for j in 0..2 {
                    let term = self.product(&adj[i][a], &r[i + j])?;
                    let term = self.product(&term, &adj[j][b])?;
                    sum = self.sum(&sum, &term, false)?;
                }
            }
            output.push((self.product(w, &sum)?, d3.clone()));
        }
        Ok(output.try_into().ok().unwrap())
    }
    pub(crate) fn bound(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
    ) -> Result<CellBounds> {
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let source = self.source.compose(p[0], range, spans[0], &mut w);
        // Retain consumed exact work even when the operation refuses.
        self.work = w.used;
        let source = source?;
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let candidate = self.candidate.compose(p[1], range, spans[1], &mut w);
        self.work = w.used;
        let candidate = candidate?;
        let fields = self.graph_fields(&source)?;
        let source_sign = sign(hull(&fields[0].1)?);
        let Some(source_sign) = source_sign else {
            return Ok(CellBounds {
                upper: None,
                same_orientation: None,
            });
        };
        let global = self.candidate_hulls.clone();
        let alpha = I::point(global[1][0].lo * 0.5 + global[1][0].hi * 0.5);
        let beta = I::point(global[2][1].lo * 0.5 + global[2][1].hi * 0.5);
        let j = [[global[1][0], global[2][0]], [global[1][1], global[2][1]]];
        let det = j[0][0].mul(j[1][1])?.sub(j[0][1].mul(j[1][0])?)?;
        let Some(candidate_sign) = sign(det) else {
            return Ok(CellBounds {
                upper: None,
                same_orientation: None,
            });
        };
        let inv = [
            [j[1][1].div_signed(det)?, neg(j[0][1])?.div_signed(det)?],
            [neg(j[1][0])?.div_signed(det)?, j[0][0].div_signed(det)?],
        ];
        let e = [
            [j[0][0].sub(alpha)?, j[0][1]],
            [j[1][0], j[1][1].sub(beta)?],
        ];
        let mut t = [[I::point(0.); 2]; 2];
        for a in 0..2 {
            for b in 0..2 {
                for k in 0..2 {
                    t[a][b] = t[a][b].sub(e[a][k].mul(inv[k][b])?)?;
                }
            }
        }
        let factors = [
            alpha,
            beta,
            alpha.mul(alpha)?,
            alpha.mul(beta)?,
            beta.mul(beta)?,
        ];
        let mut base = [I::point(0.); 5];
        let mut delta = [I::point(0.); 5];
        for i in 0..5 {
            let den = self.scale(&candidate[0][3], factors[i])?;
            let numerator =
                self.difference_products(&candidate[i + 1][2], &fields[i].1, &fields[i].0, &den)?;
            let denominator = self.product(&den, &fields[i].1)?;
            delta[i] = hull(&numerator)?.div_signed(hull(&denominator)?)?;
            base[i] = hull(&fields[i].0)?.div_signed(hull(&fields[i].1)?)?;
        }
        // The candidate's stored floating world poles have a small, bounded
        // non-affine XY component. Correct the nominal graph jets; never
        // treat the requested frame as an exact stored projection.
        let gref = [base[0].add(delta[0])?, base[1].add(delta[1])?];
        let href = [
            [base[2].add(delta[2])?, base[3].add(delta[3])?],
            [base[3].add(delta[3])?, base[4].add(delta[4])?],
        ];
        let mut dg = [I::point(0.); 2];
        for a in 0..2 {
            for k in 0..2 {
                dg[a] = dg[a].add(gref[k].mul(t[k][a])?)?;
            }
        }
        let gactual = [gref[0].add(dg[0])?, gref[1].add(dg[1])?];
        for a in 0..2 {
            delta[a] = delta[a].add(dg[a])?;
        }
        let q = |i: usize, j: usize| -> Result<I> {
            gactual[0]
                .mul(global[3 + i + j][0])?
                .add(gactual[1].mul(global[3 + i + j][1])?)
        };
        for (index, (a, b)) in [(0, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            let mut correction = I::point(0.);
            for k in 0..2 {
                correction = correction
                    .add(t[k][a].mul(href[k][b])?)?
                    .add(href[a][k].mul(t[k][b])?)?;
                for l in 0..2 {
                    correction = correction
                        .add(t[k][a].mul(href[k][l])?.mul(t[l][b])?)?
                        .sub(inv[k][a].mul(q(k, l)?)?.mul(inv[l][b])?)?;
                }
            }
            delta[index + 2] = delta[index + 2].add(correction)?;
        }
        let remaining = self.max_work.saturating_sub(self.work);
        let (upper, used) =
            crate::analysis::surface_collar::graph_difference_bounds(base, delta, remaining)?;
        self.charge(used)?;
        Ok(CellBounds {
            upper,
            same_orientation: Some(source_sign == candidate_sign),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(gain: f64) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![vec![1.; 3]; 3],
            control_points: (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            let u = i as f64 / 2.;
                            let v = j as f64 / 2.;
                            let h = if i == 2 { gain } else { 0. };
                            vec![u + v + h, -u + v + h, -2. * v + h]
                        })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn trace() -> Curve {
        Curve::from_polyline(vec![vec![0.1, 0.2], vec![0.2, 0.3]]).unwrap()
    }
    #[test]
    fn exact_curved_graph_has_small_whole_cell_residual() {
        let s = fixture(1.);
        let p = trace();
        let mut pair = Pair::new(&s, &s, 100_000_000).unwrap();
        let bound = pair.bound([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
        assert_eq!(bound.same_orientation, Some(true));
        let upper = bound.upper.unwrap();
        assert!(upper[0] < 1e-8 && upper[1] < 1e-7, "{upper:?}");
        assert!(pair.work > 1000 && pair.work <= pair.max_work);
    }
    #[test]
    fn altered_curvature_does_not_receive_small_bound() {
        let s = fixture(1.);
        let changed = fixture(1.125);
        let p = trace();
        let mut pair = Pair::new(&s, &changed, 100_000_000).unwrap();
        let bound = pair.bound([&p, &p], [0., 1.], [[2, 2]; 2]).unwrap();
        assert!(bound.upper.is_none_or(|upper| upper[1] > 1e-4));
    }
    #[test]
    fn no_work_is_not_evidence() {
        let s = fixture(1.);
        assert!(Pair::new(&s, &s, 0).is_err());
    }
    #[test]
    fn rational_source_graph_jets_match_independent_chain_rule() {
        let mut source = fixture(1.);
        for i in 0..3 {
            for j in 0..3 {
                source.weights[i][j] = 1. + (i + 2 * j) as f64 * 0.125;
            }
        }
        let candidate = fixture(1.);
        let mut pair = Pair::new(&source, &candidate, 100_000_000).unwrap();
        let pc = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0.25, 0.375]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let mut w = Work {
            used: pair.work,
            limit: pair.max_work,
        };
        let h = pair.source.compose(&pc, [0., 1.], [2, 2], &mut w).unwrap();
        pair.work = w.used;
        let fields = pair.graph_fields(&h).unwrap();
        let evaluation = source.evaluate(0.25, 0.375).unwrap();
        let (u, v) = evaluation.first_derivatives().unwrap();
        let (uu, uv, vv) = evaluation.second_derivatives().unwrap();
        let project = |p: [f64; 3]| [p[0] - p[1], p[0] + p[1] - 2. * p[2], p[0] + p[1] + p[2]];
        let (u, v) = (project(u), project(v));
        let determinant = u[0] * v[1] - v[0] * u[1];
        let inverse = [
            [v[1] / determinant, -v[0] / determinant],
            [-u[1] / determinant, u[0] / determinant],
        ];
        let gradient =
            std::array::from_fn::<_, 2, _>(|a| u[2] * inverse[0][a] + v[2] * inverse[1][a]);
        let r = [uu, uv, vv].map(|x| {
            let x = project(x);
            x[2] - gradient[0] * x[0] - gradient[1] * x[1]
        });
        let mut expected = vec![gradient[0], gradient[1]];
        for (a, b) in [(0, 0), (0, 1), (1, 1)] {
            expected.push(
                inverse[0][a] * r[0] * inverse[0][b]
                    + (inverse[0][a] * inverse[1][b] + inverse[1][a] * inverse[0][b]) * r[1]
                    + inverse[1][a] * r[2] * inverse[1][b],
            );
        }
        for (field, expected) in fields.iter().zip(expected) {
            let interval = hull(&field.0)
                .unwrap()
                .div_signed(hull(&field.1).unwrap())
                .unwrap();
            assert!(
                expected >= interval.lo - 1e-10 && expected <= interval.hi + 1e-10,
                "{interval:?} versus {expected}"
            );
            assert!(interval.hi - interval.lo < 1e-7, "{interval:?}");
        }
    }
}
