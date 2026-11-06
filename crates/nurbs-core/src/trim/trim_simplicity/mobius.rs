//! w_i=w_0 r^i gives C(t)=P(r t/(1-t+r t)), exactly. Positive r makes
//! this a nonsingular increasing self-map of the complete normalized interval.
//! In particular an isolated critical point need not be a dyadic source cut.
use super::*;
pub(super) fn prove(c: &Curve, used: &mut usize, max: usize) -> Result<bool> {
    c.validate()?;
    if !bezier(c) || !(1..=32).contains(&c.degree) || c.control_points.iter().any(|p| p.len() < 2) {
        return Ok(false);
    }
    if c.weights.iter().all(|w| *w == c.weights[0]) {
        return exact_subdivision::prove(c, used, max);
    }
    if !crate::numerics::geometric_weights::sequence(&c.weights, used, max) {
        return Ok(false);
    }
    let mut polynomial = c.clone();
    polynomial.weights.fill(1.);
    // This is an exact reparameterization witness, not equality at common t.
    exact_subdivision::prove(&polynomial, used, max)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_critical_clock_is_exact_and_spends_one_shared_ledger() {
        let c = Curve {
            degree: 5,
            knots: [vec![0.; 6], vec![1.; 6]].concat(),
            control_points: (0..6)
                .map(|i| vec![if i % 2 == 0 { -10. } else { 10. }, 0.])
                .collect(),
            weights: (0..6).map(|i| 2f64.powi(5 - i)).collect(),
            periodic: false,
        };
        let original = c.clone();
        let mut used = 0;
        assert!(prove(&c, &mut used, 1000).unwrap());
        assert!(used > 0 && used < 1000);
        let mut replay = 0;
        assert!(prove(&c, &mut replay, used).unwrap());
        assert_eq!(replay, used);
        let mut short = 0;
        assert!(!prove(&c, &mut short, used - 1).unwrap());
        assert!(short <= used - 1);
        let mut perturbed = c.clone();
        perturbed.weights[2] = perturbed.weights[2].next_up();
        assert!(!prove(&perturbed, &mut 0, 1000).unwrap());
        let mut folded = c.clone();
        for (p, x) in folded
            .control_points
            .iter_mut()
            .zip([-10., 100., -100., 100., -100., 10.])
        {
            p[0] = x;
        }
        assert!(!prove(&folded, &mut 0, 1000).unwrap());
        let mut pole = c.clone();
        pole.weights[2] = 0.;
        assert!(prove(&pole, &mut 0, 1000).is_err());
        assert_eq!(c, original);
    }
}
