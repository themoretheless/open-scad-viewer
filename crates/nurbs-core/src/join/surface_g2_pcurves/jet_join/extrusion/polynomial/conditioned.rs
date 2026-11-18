//! Enclose the SAME matched profile interval on a local positive clock.
//! A common Mobius clock is applied to both homogeneous profiles and their
//! residual/denominator polynomials, preserving point correspondence and G2.
use super::*;
fn mix(a: I, b: I, t: I) -> Result<I> {
    if t.lo == 0. && t.hi == 0. {
        return Ok(a);
    }
    if t.lo == 1. && t.hi == 1. {
        return Ok(b);
    }
    if a.lo == 0. && a.hi == 0. && b.lo == 0. && b.hi == 0. {
        return Ok(I::point(0.));
    }
    a.mul(I::point(1.).sub(t)?.intersect(0., 1.)?)?
        .add(b.mul(t)?)
}
fn split(p: &Poly, t: I) -> Result<(Poly, Poly)> {
    let mut net = p.clone();
    let mut left = vec![net[0]];
    let mut right = vec![*net.last().unwrap()];
    while net.len() > 1 {
        for i in 0..net.len() - 1 {
            net[i] = mix(net[i], net[i + 1], t)?;
        }
        net.pop();
        left.push(net[0]);
        right.push(*net.last().unwrap());
    }
    right.reverse();
    Ok((left, right))
}
pub(super) fn restrict_poly(p: &Poly, t: [f64; 2]) -> Result<Poly> {
    let p = if t[1] == 1. {
        p.clone()
    } else {
        split(p, I::point(t[1]))?.0
    };
    if t[0] == 0. {
        return Ok(p);
    }
    let a = I::point(t[0]).div(I::point(t[1]))?.intersect(0., 1.)?;
    Ok(split(&p, a)?.1)
}
fn scale(p: &mut Poly, clock: f64) -> Result<()> {
    let mut factor = I::point(1.);
    for value in p {
        if value.lo != 0. || value.hi != 0. {
            *value = value.mul(factor)?;
        }
        factor = factor.mul(I::point(clock))?;
    }
    Ok(())
}
impl Pair {
    fn local_clock(&self) -> f64 {
        let estimate = |h: &Homogeneous| -> Option<f64> {
            let n = h[3].len() - 1;
            if n == 0 {
                return None;
            }
            let value = |v: I| {
                let x = v.lo * 0.5 + v.hi * 0.5;
                (x.is_finite() && x > 0.).then_some(x)
            };
            Some((value(h[3][0])?.log2() - value(*h[3].last()?)?.log2()) / n as f64)
        };
        let (Some(a), Some(b)) = (estimate(&self.a), estimate(&self.b)) else {
            return 1.;
        };
        // Binary powers propose a positive bijection; no estimated value is
        // used as a geometric or regularity certificate.
        2f64.powi(((a + b) * 0.5).round().clamp(-16., 16.) as i32)
    }
    pub(super) fn conditioned(&self, t: [f64; 2]) -> Result<Option<Self>> {
        if !(0. <= t[0] && t[0] < t[1] && t[1] <= 1.) {
            return Ok(None);
        }
        let restrict_h = |h: &Homogeneous| -> Result<Homogeneous> {
            Ok([
                restrict_poly(&h[0], t)?,
                restrict_poly(&h[1], t)?,
                restrict_poly(&h[2], t)?,
                restrict_poly(&h[3], t)?,
            ])
        };
        let mut pair = Self {
            // The old jet numerators use the original clock. The conditioned
            // interval path recomputes jets from its own transformed net.
            derivatives: None,
            delta_derivatives: None,
            a: restrict_h(&self.a)?,
            b: restrict_h(&self.b)?,
            e: [
                restrict_poly(&self.e[0], t)?,
                restrict_poly(&self.e[1], t)?,
                restrict_poly(&self.e[2], t)?,
            ],
            d: restrict_poly(&self.d, t)?,
        };
        let clock = pair.local_clock();
        if t == [0., 1.] && clock == 1. {
            return Ok(None);
        }
        for h in [&mut pair.a, &mut pair.b] {
            for p in h {
                scale(p, clock)?;
            }
        }
        for p in &mut pair.e {
            scale(p, clock)?;
        }
        scale(&mut pair.d, clock)?;
        Ok(Some(pair))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn h(scale: f64, bend: f64) -> Homogeneous {
        [
            vec![I::point(0.), I::point(0.5 * scale), I::point(scale)],
            vec![I::point(0.); 3],
            vec![I::point(0.), I::point(0.), I::point(bend * scale * scale)],
            vec![I::point(1.); 3],
        ]
    }
    #[test]
    fn local_clock_covers_same_bend_and_preserves_a_real_mismatch() {
        let p = Pair::new(h(1., 1.), h(1., 1.1)).unwrap();
        let q = p.conditioned([0.25, 0.75]).unwrap().unwrap();
        let a = p.bounds_direct([0.5, 0.5], 1).unwrap().unwrap();
        let b = q.bounds_direct([0.5, 0.5], 1).unwrap().unwrap();
        assert!(a[1][0] > 1e-4 && b[1][0] > 1e-4);
        assert!(b[1][0] <= a[1][1] && a[1][0] <= b[1][1]);
        assert!(p.conditioned([0.5, 0.5]).unwrap().is_none());
        assert!(p.conditioned([-0.1, 0.5]).unwrap().is_none());
    }
    #[test]
    fn overflowing_conservative_jet_enclosure_keeps_the_ordinary_proof() {
        let mut p = Pair::new(h(1., 0.), h(1., 0.)).unwrap();
        let broad = I::new(-1e200, 1e200).unwrap();
        let make = || ExactJets {
            first_weights: vec![I::point(1.)],
            second_weights: vec![I::point(1.)],
            first: std::array::from_fn(|_| vec![broad]),
            second: std::array::from_fn(|_| vec![broad]),
        };
        p.derivatives = Some([make(), make()]);
        // These deliberately broad intervals still contain the true jets.
        let b = p.bounds([0.25, 0.75], 1).unwrap().unwrap();
        assert!(b[0][1] < 1e-10 && b[1][1] < 1e-10, "{b:?}");
    }
    #[test]
    fn rational_jet_hull_retains_shared_weight_correlation() {
        let weights = vec![I::point(1e-20), I::point(1.)];
        let j = ExactJets {
            first: std::array::from_fn(|_| weights.clone()),
            second: std::array::from_fn(|_| vec![I::point(0.); 2]),
            first_weights: weights.clone(),
            second_weights: weights,
        };
        let r = j
            .component(
                false,
                0,
                I::new(0., 1.).unwrap(),
                I::new(1e-20, 1.).unwrap(),
            )
            .unwrap();
        assert!(r.lo > 0.999999 && r.hi < 1.000001, "{r:?}");
        let z = j
            .component(
                true,
                0,
                I::new(0.17, 0.83).unwrap(),
                I::new(1e-20, 1.).unwrap(),
            )
            .unwrap();
        assert!(z.lo <= 0. && z.hi >= 0. && z.hi - z.lo < 1e-12);
    }
    #[test]
    fn common_mobius_clock_keeps_exact_zero_residual() {
        let mut a = h(1., 0.);
        a[3] = vec![I::point(0.0625), I::point(0.25), I::point(1.)];
        let p = Pair::new(a.clone(), a).unwrap();
        let q = p.conditioned([0., 1.]).unwrap().unwrap();
        assert!(q.e.iter().flatten().all(|x| x.lo <= 0. && x.hi >= 0.));
        assert!(q.a[3][0].lo <= q.a[3][2].hi && q.a[3][2].lo <= q.a[3][0].hi);
        let zero = Pair::new(h(0., 0.), h(0., 0.)).unwrap();
        assert!(zero.bounds([0.2, 0.8], 1).unwrap().is_none());
    }
}
