use super::*;
#[path = "polynomial/conditioned.rs"]
mod conditioned;
#[path = "polynomial/exact_product.rs"]
mod exact_product;
pub(super) use exact_product::{Work as ExactProductWork, prepare as prepare_exact_product};
pub(super) type Poly = Vec<I>;
pub(super) type Homogeneous = [Poly; 4];
pub(super) fn diff(a: &[I]) -> Result<Poly> {
    if a.len() == 1 {
        return Ok(vec![I::point(0.)]);
    }
    a.windows(2)
        .map(|w| w[1].sub(w[0])?.mul(I::point((a.len() - 1) as f64)))
        .collect()
}
pub(super) fn eval(a: &[I], t: I) -> Result<I> {
    let lo = a.iter().map(|x| x.lo).fold(f64::INFINITY, f64::min);
    let hi = a.iter().map(|x| x.hi).fold(f64::NEG_INFINITY, f64::max);
    let mut c = a.to_vec();
    let complement = I::point(1.).sub(t)?.intersect(0., 1.)?;
    for n in (1..c.len()).rev() {
        for i in 0..n {
            c[i] = c[i].mul(complement)?.add(c[i + 1].mul(t)?)?;
        }
    }
    c[0].intersect(lo, hi)
}
pub(super) fn mul(a: &[I], b: &[I]) -> Result<Poly> {
    crate::curve_surface_composition::mul(a, b)
}
fn add(a: &[I], b: &[I], sign: f64) -> Result<Poly> {
    check(a.len() == b.len(), "Profile polynomial degrees differ")?;
    a.iter()
        .zip(b)
        .map(|(&a, &b)| a.add(b.mul(I::point(sign))?))
        .collect()
}
pub(super) fn elevate(mut h: Homogeneous, degree: usize) -> Result<Homogeneous> {
    while h[0].len() < degree + 1 {
        for c in &mut h {
            let old = c.clone();
            let n = old.len();
            if old.iter().all(|x| x.lo == 0. && x.hi == 0.) {
                *c = vec![I::point(0.); n + 1];
                continue;
            }
            let mut out = vec![I::point(0.); n + 1];
            out[0] = old[0];
            out[n] = old[n - 1];
            for i in 1..n {
                let a = I::point(i as f64).div(I::point(n as f64))?;
                out[i] = old[i - 1].mul(a)?.add(old[i].mul(I::point(1.).sub(a)?)?)?;
            }
            *c = out;
        }
    }
    Ok(h)
}
struct ExactJets {
    first: [Poly; 3],
    second: [Poly; 3],
    first_weights: Poly,
    second_weights: Poly,
}
impl ExactJets {
    /// Positive Bernstein denominator coefficients give a rational convex hull
    /// on this SAME restricted cell. No global-min denominator division.
    fn component(&self, second: bool, axis: usize, t: I, denominator: I) -> Result<I> {
        let (p, w) = if second {
            (&self.second[axis], &self.second_weights)
        } else {
            (&self.first[axis], &self.first_weights)
        };
        let ordinary = eval(p, t).and_then(|v| v.div(denominator));
        let rational = (|| {
            check(
                p.len() == w.len(),
                "Rational jet numerator/weight degrees differ",
            )?;
            let p = conditioned::restrict_poly(p, [t.lo, t.hi])?;
            let w = conditioned::restrict_poly(w, [t.lo, t.hi])?;
            let mut lo = f64::INFINITY;
            let mut hi = f64::NEG_INFINITY;
            for (p, w) in p.iter().zip(w) {
                check(
                    w.lo > 0.,
                    "Rational jet hull needs positive original weights",
                )?;
                let ratio = p.div(w)?;
                lo = lo.min(ratio.lo);
                hi = hi.max(ratio.hi);
            }
            I::new(lo, hi)
        })();
        match (ordinary, rational) {
            (Ok(a), Ok(b)) => I::new(a.lo.max(b.lo), a.hi.min(b.hi)),
            (Ok(a), Err(_)) => Ok(a),
            (Err(_), Ok(b)) => Ok(b),
            (Err(e), Err(_)) => Err(e),
        }
    }
}
pub(super) struct Pair {
    a: Homogeneous,
    b: Homogeneous,
    e: [Poly; 3],
    d: Poly,
    derivatives: Option<[ExactJets; 2]>,
    delta_derivatives: Option<ExactJets>,
}
impl Pair {
    pub fn new(a: Homogeneous, b: Homogeneous) -> Result<Self> {
        let mut e: [Poly; 3] = std::array::from_fn(|_| Vec::new());
        for k in 0..3 {
            e[k] = add(&mul(&b[k], &a[3])?, &mul(&a[k], &b[3])?, -1.)?;
        }
        let d = mul(&a[3], &b[3])?;
        Ok(Self {
            a,
            b,
            e,
            d,
            derivatives: None,
            delta_derivatives: None,
        })
    }
    pub fn bounds(&self, t: [f64; 2], axis: usize) -> Result<Option<[[f64; 2]; 2]>> {
        let direct = self.bounds_direct(t, axis)?;
        if t[0] >= t[1] {
            return Ok(direct);
        }
        let local = match self.conditioned(t) {
            Ok(Some(pair)) => pair.bounds_direct([0., 1.], axis).ok().flatten(),
            _ => None,
        };
        Ok(match (direct, local) {
            (Some(a), Some(b)) => {
                let combined =
                    std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
                combined.iter().all(|v| v[0] <= v[1]).then_some(combined)
            }
            (a, b) => a.or(b),
        })
    }
    fn bounds_direct(&self, t: [f64; 2], axis: usize) -> Result<Option<[[f64; 2]; 2]>> {
        let ordinary = self.bounds_jets(t, axis, false)?;
        let exact = if self.derivatives.is_some() || self.delta_derivatives.is_some() {
            self.bounds_jets(t, axis, true).ok().flatten()
        } else {
            None
        };
        Ok(match (ordinary, exact) {
            (Some(a), Some(b)) => {
                let combined =
                    std::array::from_fn(|k| [a[k][0].max(b[k][0]), a[k][1].min(b[k][1])]);
                combined.iter().all(|v| v[0] <= v[1]).then_some(combined)
            }
            (a, b) => a.or(b),
        })
    }
    fn bounds_jets(
        &self,
        t: [f64; 2],
        axis: usize,
        use_exact: bool,
    ) -> Result<Option<[[f64; 2]; 2]>> {
        let t = I::new(t[0], t[1])?;
        let jets = |h: &Homogeneous, exact: Option<&ExactJets>| -> Result<jets::Jets> {
            if let Some(exact) = exact {
                let denominator = eval(&h[3], t)?;
                if denominator.lo > 0. {
                    let squared = denominator.mul(denominator)?;
                    let cubed = squared.mul(denominator)?;
                    let mut j = [[[I::point(0.); 3]; 4]; 4];
                    for k in 0..3 {
                        j[0][0][k] = eval(&h[k], t)?.div(denominator)?;
                        j[1][0][k] = exact.component(false, k, t, squared)?;
                        j[2][0][k] = exact.component(true, k, t, cubed)?;
                    }
                    j[0][1][axis] = I::point(1.);
                    return Ok(j);
                }
            }
            let net = (0..h[0].len())
                .map(|i| vec![std::array::from_fn(|k| h[k][i]); 2])
                .collect::<Vec<_>>();
            let mut j = jets::evaluate_homogeneous_jets(&net, [t, I::point(0.)], 2)?;
            if h[axis].iter().all(|x| x.lo == 0. && x.hi == 0.) {
                j[1][0][axis] = I::point(0.);
                j[2][0][axis] = I::point(0.);
            }
            j[0][1] = [I::point(0.); 3];
            j[0][1][axis] = I::point(1.);
            j[1][1] = [I::point(0.); 3];
            j[0][2] = [I::point(0.); 3];
            Ok(j)
        };
        let mut a = jets(
            &self.a,
            self.derivatives
                .as_ref()
                .filter(|_| use_exact)
                .map(|d| &d[0]),
        )?;
        let mut b = jets(
            &self.b,
            self.derivatives
                .as_ref()
                .filter(|_| use_exact)
                .map(|d| &d[1]),
        )?;
        let d = eval(&self.d, t)?;
        if d.lo <= 0. {
            return Ok(None);
        }
        let d1 = eval(&diff(&self.d)?, t)?;
        let d2 = eval(&diff(&diff(&self.d)?)?, t)?;
        let d_sq = d.mul(d)?;
        let d_cb = d_sq.mul(d)?;
        let mut delta = [[I::point(0.); 3]; 3];
        for k in 0..3 {
            if k == axis {
                continue;
            }
            if let Some(exact) = self.delta_derivatives.as_ref().filter(|_| use_exact) {
                delta[0][k] = exact.component(false, k, t, d_sq)?;
                delta[1][k] = exact.component(true, k, t, d_cb)?;
                continue;
            }
            let e = eval(&self.e[k], t)?;
            let e1 = eval(&diff(&self.e[k])?, t)?;
            let e2 = eval(&diff(&diff(&self.e[k])?)?, t)?;
            delta[0][k] = e1.div(d)?.sub(e.mul(d1)?.div(d_sq)?)?;
            delta[1][k] = e2
                .div(d)?
                .sub(e.mul(d2)?.div(d_sq)?)?
                .sub(I::point(2.).mul(e1)?.mul(d1)?.div(d_sq)?)?
                .add(I::point(2.).mul(e)?.mul(d1)?.mul(d1)?.div(d_cb)?)?;
        }
        // Whole-cell jet midpoints PROPOSE positive clocks only. The complete
        // original interval jets and correlated jumps still prove the fields.
        // A local positive affine clock scales first AND second derivatives;
        // normal and world shape tensors are invariant under that change.
        let norm = |j: &jets::Jets| {
            j[1][0]
                .iter()
                .fold(0f64, |n, x| n.hypot(x.lo * 0.5 + x.hi * 0.5))
        };
        let ratio = norm(&a) / norm(&b);
        let scales = if ratio.is_finite() && (1e-6..=1e6).contains(&ratio) {
            [1., ratio]
        } else if ratio.is_finite() && (1e-12..=1e12).contains(&ratio) {
            let r = ratio.sqrt();
            [1. / r, r]
        } else {
            [1.; 2]
        };
        if scales != [1.; 2] {
            let first = scales.map(I::point);
            let second = [first[0].mul(first[0])?, first[1].mul(first[1])?];
            for k in 0..3 {
                if k == axis
                    && [a[1][0][k], a[2][0][k], b[1][0][k], b[2][0][k]]
                        .iter()
                        .all(|x| x.lo == 0. && x.hi == 0.)
                {
                    delta[0][k] = I::point(0.);
                    delta[1][k] = I::point(0.);
                    continue;
                }
                delta[0][k] = first[1]
                    .mul(delta[0][k])?
                    .add(first[1].sub(first[0])?.mul(a[1][0][k])?)?;
                delta[1][k] = second[1]
                    .mul(delta[1][k])?
                    .add(second[1].sub(second[0])?.mul(a[2][0][k])?)?;
                a[1][0][k] = a[1][0][k].mul(first[0])?;
                b[1][0][k] = b[1][0][k].mul(first[1])?;
                a[2][0][k] = a[2][0][k].mul(second[0])?;
                b[2][0][k] = b[2][0][k].mul(second[1])?;
            }
        }
        if [a[1][0][axis], a[2][0][axis], b[1][0][axis], b[2][0][axis]]
            .iter()
            .all(|x| x.lo == 0. && x.hi == 0.)
        {
            super::super::paired::extrusion_bounds(&a, &b, delta, axis)
        } else {
            super::super::paired::bounds(&a, &b, delta)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn parabola(speed: f64, curvature: f64) -> Homogeneous {
        [
            vec![I::point(0.), I::point(speed / 2.), I::point(speed)],
            vec![I::point(0.); 3],
            vec![
                I::point(0.),
                I::point(0.),
                I::point(curvature * speed * speed),
            ],
            vec![I::point(1.); 3],
        ]
    }
    #[test]
    fn positive_clocks_prove_curved_fields_at_extreme_endpoint_speeds() {
        let p = Pair::new(parabola(1e-8, 1.), parabola(1., 1.)).unwrap();
        for t in [[0., 0.], [0., 1e-12]] {
            let r = p.bounds(t, 1).unwrap().expect("regular projected fields");
            assert!(r[0][1] <= 1e-5 && r[1][1] <= 1e-4, "{r:?}");
        }
    }
    #[test]
    fn conditioning_does_not_erase_a_curvature_mismatch_or_zero_speed() {
        let p = Pair::new(parabola(1e-8, 1.), parabola(1., 1.1)).unwrap();
        let r = p.bounds([0., 0.], 1).unwrap().unwrap();
        assert!(r[1][0] > 1e-4, "{r:?}");
        let p = Pair::new(parabola(0., 1.), parabola(1., 1.)).unwrap();
        assert!(p.bounds([0., 0.], 1).unwrap().is_none());
    }
}
