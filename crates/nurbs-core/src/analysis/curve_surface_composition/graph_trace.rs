//! Correlated one-dimensional graph-jet residuals on authored pcurves.
//! A cell bound is not a whole seam certificate; traversal and fresh position
//! ownership remain the caller's responsibility.
#[path = "graph_trace/compiled.rs"]
mod compiled;
use super::{
    Poly,
    exact_trace::{GraphTrace, Support, Work},
    mul,
};
use crate::{Result, check, curve::Curve, distance_bounds::Interval as I, surface::Surface};
#[derive(Clone, Copy)]
pub(crate) struct CellBounds {
    pub upper: Option<[f64; 2]>,
    pub same_orientation: Option<bool>,
}
pub(crate) struct Pair {
    source: Support,
    candidate: Support,
    cartesian: bool,
    compiled: compiled::Cache,
    pub work: usize,
    pub max_work: usize,
}
pub(crate) fn curve_box(
    p: &Curve,
    range: [f64; 2],
    used: &mut usize,
    max: usize,
) -> Result<[[f64; 2]; 2]> {
    curve_box_oriented(p, range, used, max, false)
}
pub(crate) fn curve_box_oriented(
    p: &Curve,
    range: [f64; 2],
    used: &mut usize,
    max: usize,
    reversed: bool,
) -> Result<[[f64; 2]; 2]> {
    let mut w = Work {
        used: *used,
        limit: max,
    };
    let result = super::exact_trace::pcurve_oriented(p, range, [[0., 1.]; 2], &mut w, reversed);
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
        let diagonal = Support::new(candidate, &mut w)?;
        let diagonal_hulls = diagonal.hulls()?;
        let world = Support::new_in_frame(candidate, &mut w, true)?;
        let world_hulls = world.hulls()?;
        // Proposal-only conditioning score. Admission below still uses exact
        // original projections and whole-cell outward bounds in that frame.
        let score = |h: &[[I; 4]]| {
            let mid = |x: I| x.lo * 0.5 + x.hi * 0.5;
            let diagonal = mid(h[1][0]).abs() + mid(h[2][1]).abs();
            let cross = mid(h[1][1]).abs() + mid(h[2][0]).abs();
            if diagonal > 0. {
                cross / diagonal
            } else {
                f64::INFINITY
            }
        };
        let cartesian = score(&world_hulls) < score(&diagonal_hulls);
        let candidate = if cartesian { world } else { diagonal };
        let source = Support::new_in_frame(source, &mut w, cartesian)?;
        Ok(Self {
            source,
            candidate,
            cartesian,
            compiled: Default::default(),
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
        self.charge(2usize.saturating_mul(a.len().saturating_add(b.len())))?;
        let zero = |p: &[I]| p.iter().all(|v| v.lo == 0. && v.hi == 0.);
        if zero(a) || zero(b) {
            return Ok(vec![I::point(0.); a.len() + b.len() - 1]);
        }
        if a.len() == 1 && a[0] == I::point(1.) {
            return Ok(b.to_vec());
        }
        if b.len() == 1 && b[0] == I::point(1.) {
            return Ok(a.to_vec());
        }
        self.charge(12usize.saturating_mul(a.len()).saturating_mul(b.len()))?;
        mul(a, b)
    }
    fn sum(&mut self, a: &[I], b: &[I], subtract: bool) -> Result<Poly> {
        self.charge(2usize.saturating_mul(a.len().saturating_add(b.len())))?;
        let zero = |p: &[I]| p.iter().all(|v| v.lo == 0. && v.hi == 0.);
        if zero(b) {
            return Ok(a.to_vec());
        }
        if zero(a) {
            return if subtract {
                negative(b)
            } else {
                Ok(b.to_vec())
            };
        }
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
        a.iter()
            .map(|&x| {
                // Exact algebraic zeros/identity must not become interval noise:
                // that noise would elevate a zero quotient term to the full
                // position degree and destroy otherwise useful cancellation.
                if k == I::point(0.) || x == I::point(0.) {
                    Ok(I::point(0.))
                } else if k == I::point(1.) {
                    Ok(x)
                } else {
                    x.mul(k)
                }
            })
            .collect()
    }
    fn difference_products(&mut self, a: &[I], b: &[I], c: &[I], d: &[I]) -> Result<Poly> {
        let exact_constant = |p: Poly| {
            if p[0].lo == p[0].hi && p.iter().all(|v| *v == p[0]) {
                vec![p[0]]
            } else {
                p
            }
        };
        let x = exact_constant(self.product(a, b)?);
        let y = exact_constant(self.product(c, d)?);
        self.sum(&x, &y, true)
    }
    /// Homogeneous source graph derivatives [gx,gy,Hxx,Hxy,Hyy].
    fn graph_fields(&mut self, h: &GraphTrace) -> Result<[(Poly, Poly); 5]> {
        let w = &h.weight;
        let mut first = vec![vec![Vec::new(); 3]; 2];
        for i in 0..2 {
            for k in 0..3 {
                first[i][k] = if let Some(position) = &h.position {
                    self.difference_products(
                        &h.derivatives[i][k],
                        w,
                        &position[k],
                        &h.derivatives[i][3],
                    )?
                } else {
                    self.product(&h.derivatives[i][k], w)?
                };
            }
        }
        let mut second = vec![vec![Vec::new(); 3]; 3];
        let w2 = self.product(w, w)?;
        for (ij, (a, b)) in [(0, 0), (0, 1), (1, 1)].into_iter().enumerate() {
            for k in 0..3 {
                let mut value = self.product(&h.derivatives[ij + 2][k], &w2)?;
                if let Some(position) = &h.position {
                    for (i, j) in [(a, b), (b, a)] {
                        let term = self.product(&h.derivatives[i][k], w)?;
                        let term = self.product(&term, &h.derivatives[j][3])?;
                        value = self.sum(&value, &term, true)?;
                    }
                    let term = self.product(&position[k], w)?;
                    let term = self.product(&term, &h.derivatives[ij + 2][3])?;
                    value = self.sum(&value, &term, true)?;
                    let term = self.product(&position[k], &h.derivatives[a][3])?;
                    let term = self.product(&term, &h.derivatives[b][3])?;
                    let term = self.scale(&term, I::point(2.))?;
                    value = self.sum(&value, &term, false)?;
                }
                second[ij][k] = value;
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
        self.bound_adaptive(p, range, spans, None)
    }
    pub(crate) fn bound_adaptive(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        tolerances: Option<[f64; 2]>,
    ) -> Result<CellBounds> {
        self.bound_adaptive_oriented(p, range, spans, tolerances, [false; 2])
    }
    pub(crate) fn bound_adaptive_oriented(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        tolerances: Option<[f64; 2]>,
        reversed: [bool; 2],
    ) -> Result<CellBounds> {
        let [source, candidate] = self.compositions_oriented(p, range, spans, reversed)?;
        let fields = self.graph_fields(&source)?;
        let source_sign = sign(hull(&fields[0].1)?);
        let Some(source_sign) = source_sign else {
            return Ok(CellBounds {
                upper: None,
                same_orientation: None,
            });
        };
        let local = self.local_fields_bound(&candidate, &fields, source_sign);
        let Some(tolerances) = tolerances else {
            return local;
        };
        if local.as_ref().is_ok_and(|b| {
            b.upper.is_some_and(|u| {
                b.same_orientation.is_some() && u[0] <= tolerances[0] && u[1] <= tolerances[1]
            })
        }) || self.work >= self.max_work
        {
            return local;
        }
        // Reuse the SAME original compositions and the SAME ledger. This
        // alternative is attempted only when the cheaper local chart bound
        // cannot admit the cell; it never replaces original position ownership.
        let direct = self.direct_fields_bound(&candidate, &fields, source_sign);
        match (local, direct) {
            (Ok(a), Ok(b)) if a.same_orientation == b.same_orientation => Ok(CellBounds {
                upper: match (a.upper, b.upper) {
                    (Some(a), Some(b)) => Some([a[0].min(b[0]), a[1].min(b[1])]),
                    (a, b) => a.or(b),
                },
                same_orientation: a.same_orientation,
            }),
            (_, Ok(b)) => Ok(b),
            (local, Err(_)) => local,
        }
    }
    fn compositions(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
    ) -> Result<compiled::Jets> {
        self.compositions_oriented(p, range, spans, [false; 2])
    }
    fn compositions_oriented(
        &mut self,
        p: [&Curve; 2],
        range: [f64; 2],
        spans: [[usize; 2]; 2],
        reversed: [bool; 2],
    ) -> Result<compiled::Jets> {
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let cached = self
            .compiled
            .get_oriented(p, range, spans, &mut w, reversed);
        self.work = w.used;
        if let Some(jets) = cached? {
            return Ok(jets);
        }
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let source = self
            .source
            .compose_graph_oriented(p[0], range, spans[0], &mut w, reversed[0]);
        // Retain consumed exact work even when the operation refuses.
        self.work = w.used;
        let source = source?;
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let candidate =
            self.candidate
                .compose_graph_oriented(p[1], range, spans[1], &mut w, reversed[1]);
        self.work = w.used;
        let candidate = candidate?;
        let jets = [source, candidate];
        let mut w = Work {
            used: self.work,
            limit: self.max_work,
        };
        let saved = self
            .compiled
            .store_oriented(p, range, spans, &jets, &mut w, reversed);
        self.work = w.used;
        saved?;
        Ok(jets)
    }
    fn local_fields_bound(
        &mut self,
        candidate: &GraphTrace,
        fields: &[(Poly, Poly); 5],
        source_sign: bool,
    ) -> Result<CellBounds> {
        // Corrections need the original candidate jets along THIS seam cell,
        // not the full untrimmed surface. compose encloses those literal jets
        // over the entire range; unit candidate weights make them Cartesian.
        // Local hulls shrink under subdivision without dropping any seam point.
        self.charge(candidate.polynomials().map(Vec::len).sum())?;
        let common_weight = hull(&candidate.weight)?;
        self.charge(15)?;
        let global = candidate
            .derivatives
            .iter()
            .enumerate()
            .map(|(_index, jet)| {
                let values = jet
                    .iter()
                    .enumerate()
                    .map(|(k, p)| {
                        let value = hull(p)?;
                        // The stored candidate has unit SURFACE weights, but a
                        // rational pcurve contributes its common denominator power
                        // to every composed homogeneous derivative. Remove that
                        // factor before using Cartesian chart Jacobians/Hessians.
                        if k < 3 && common_weight != I::point(1.) {
                            value.div_signed(common_weight)
                        } else {
                            Ok(value)
                        }
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(values.try_into().ok().unwrap())
            })
            .collect::<Result<Vec<[I; 4]>>>()?;
        let alpha = I::point(global[0][0].lo * 0.5 + global[0][0].hi * 0.5);
        let beta = I::point(global[1][1].lo * 0.5 + global[1][1].hi * 0.5);
        let j = [[global[0][0], global[1][0]], [global[0][1], global[1][1]]];
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
            let den = self.scale(&candidate.weight, factors[i])?;
            let numerator = self.difference_products(
                &candidate.derivatives[i][2],
                &fields[i].1,
                &fields[i].0,
                &den,
            )?;
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
                .mul(global[2 + i + j][0])?
                .add(gactual[1].mul(global[2 + i + j][1])?)
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
        self.finish_fields_bound(base, delta, source_sign == candidate_sign)
    }
    fn direct_fields_bound(
        &mut self,
        candidate: &GraphTrace,
        fields: &[(Poly, Poly); 5],
        source_sign: bool,
    ) -> Result<CellBounds> {
        let candidate_fields = self.graph_fields(candidate)?;
        let Some(candidate_sign) = sign(hull(&candidate_fields[0].1)?) else {
            return Ok(CellBounds {
                upper: None,
                same_orientation: None,
            });
        };
        let mut base = [I::point(0.); 5];
        let mut delta = [I::point(0.); 5];
        for i in 0..5 {
            let numerator = self.difference_products(
                &candidate_fields[i].0,
                &fields[i].1,
                &fields[i].0,
                &candidate_fields[i].1,
            )?;
            let denominator = self.product(&candidate_fields[i].1, &fields[i].1)?;
            delta[i] = hull(&numerator)?.div_signed(hull(&denominator)?)?;
            base[i] = hull(&fields[i].0)?.div_signed(hull(&fields[i].1)?)?;
        }
        self.finish_fields_bound(base, delta, source_sign == candidate_sign)
    }
    fn finish_fields_bound(
        &mut self,
        base: [I; 5],
        delta: [I; 5],
        same_orientation: bool,
    ) -> Result<CellBounds> {
        let remaining = self.max_work.saturating_sub(self.work);
        let (upper, used) = if self.cartesian {
            crate::analysis::surface_collar::graph_difference_bounds_cartesian(
                base, delta, remaining,
            )?
        } else {
            crate::analysis::surface_collar::graph_difference_bounds(base, delta, remaining)?
        };
        self.charge(used)?;
        Ok(CellBounds {
            upper,
            same_orientation: Some(same_orientation),
        })
    }
}

#[cfg(test)]
#[path = "graph_trace/tests/mod.rs"]
mod tests;
