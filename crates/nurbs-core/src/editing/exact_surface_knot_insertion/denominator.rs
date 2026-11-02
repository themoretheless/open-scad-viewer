//! Independent local-power Cox-de Boor proof of a common denominator gauge.
use super::{Curve, PResult, Q, Work};
use num_traits::{One, Zero};
type Poly = Vec<Q>;
fn add(a: &mut Poly, b: &Poly, w: &mut Work) -> PResult<()> {
    a.resize(a.len().max(b.len()), Q::zero());
    for (x, y) in a.iter_mut().zip(b) {
        *x = w.op(x, y, |x, y| x + y)?;
    }
    Ok(())
}
fn linear(p: &Poly, a: &Q, b: &Q, w: &mut Work) -> PResult<Poly> {
    let mut out = vec![Q::zero(); p.len() + 1];
    for (i, x) in p.iter().enumerate() {
        let l = w.op(x, a, |x, a| x * a)?;
        let r = w.op(x, b, |x, b| x * b)?;
        out[i] = w.op(&out[i], &l, |x, y| x + y)?;
        out[i + 1] = w.op(&out[i + 1], &r, |x, y| x + y)?;
    }
    Ok(out)
}
fn polynomial(c: &Curve, lo: f64, hi: f64, w: &mut Work) -> PResult<Poly> {
    let k = c
        .knots
        .iter()
        .map(|&x| w.lift(x))
        .collect::<PResult<Vec<_>>>()?;
    let a = w.lift(lo)?;
    let b = w.lift(hi)?;
    let width = w.op(&b, &a, |x, y| x - y)?;
    // This selects a degree-zero basis by order only; no rounded midpoint.
    let span = c.knots.partition_point(|x| *x <= lo) - 1;
    let mut basis = (0..c.knots.len() - 1)
        .map(|i| vec![if i == span { Q::one() } else { Q::zero() }])
        .collect::<Vec<_>>();
    for degree in 1..=c.degree {
        let mut next = Vec::new();
        for i in 0..basis.len() - 1 {
            let mut value = vec![Q::zero(); degree + 1];
            let left = w.op(&k[i + degree], &k[i], |x, y| x - y)?;
            if !left.is_zero() && basis[i].iter().any(|x| !x.is_zero()) {
                let distance = w.op(&a, &k[i], |x, y| x - y)?;
                let intercept = w.op(&distance, &left, |x, y| x / y)?;
                let slope = w.op(&width, &left, |x, y| x / y)?;
                add(&mut value, &linear(&basis[i], &intercept, &slope, w)?, w)?;
            }
            let right = w.op(&k[i + degree + 1], &k[i + 1], |x, y| x - y)?;
            if !right.is_zero() && basis[i + 1].iter().any(|x| !x.is_zero()) {
                let distance = w.op(&k[i + degree + 1], &a, |x, y| x - y)?;
                let intercept = w.op(&distance, &right, |x, y| x / y)?;
                let slope = w.op(&width, &right, |x, y| -x / y)?;
                add(
                    &mut value,
                    &linear(&basis[i + 1], &intercept, &slope, w)?,
                    w,
                )?;
            }
            next.push(value);
        }
        basis = next;
    }
    let mut out = vec![Q::zero(); c.degree + 1];
    for (p, &weight) in basis.iter().zip(&c.weights) {
        let weight = w.lift(weight)?;
        let p = p
            .iter()
            .map(|x| w.op(x, &weight, |x, y| x * y))
            .collect::<PResult<Vec<_>>>()?;
        add(&mut out, &p, w)?;
    }
    Ok(out)
}
pub(super) fn proportional(
    source: &Curve,
    target: &Curve,
    source_anchor: f64,
    target_anchor: f64,
    w: &mut Work,
) -> PResult<bool> {
    let [start, end] = source.domain();
    let mut cuts = source
        .knots
        .iter()
        .chain(&target.knots)
        .copied()
        .filter(|&x| start <= x && x <= end)
        .collect::<Vec<_>>();
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let sa = w.lift(source_anchor)?;
    let ta = w.lift(target_anchor)?;
    for pair in cuts.windows(2) {
        let a = polynomial(source, pair[0], pair[1], w)?;
        let b = polynomial(target, pair[0], pair[1], w)?;
        for (x, y) in a.iter().zip(&b) {
            let x = w.op(x, &ta, |x, y| x * y)?;
            let y = w.op(y, &sa, |x, y| x * y)?;
            if !w.op(&x, &y, |x, y| x - y)?.is_zero() {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn line(weights: Vec<f64>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![-2., -2., 6., 6.],
            control_points: vec![vec![0., 0., 0.]; 2],
            weights,
            periodic: false,
        }
    }
    fn refined(weights: Vec<f64>) -> Curve {
        Curve {
            degree: 1,
            knots: vec![-2., -2., 2., 6., 6.],
            control_points: vec![vec![0., 0., 0.]; 3],
            weights,
            periodic: false,
        }
    }
    #[test]
    fn proportional_law_uses_the_same_external_anchor_for_every_row() {
        let a = line(vec![1., 2.]);
        let b = refined(vec![0.5, 0.75, 1.]);
        let mut w = Work {
            used: 0,
            limit: 1_000_000,
        };
        assert!(proportional(&a, &b, 1., 0.5, &mut w).unwrap());
        // Same constant Cartesian row in both candidates. Independent row
        // normalization would pass rational geometry but alter tensor mixing.
        let wrong = refined(vec![1., 1.5, 2.]);
        assert!(!proportional(&a, &wrong, 1., 0.5, &mut w).unwrap());
        let wrong = refined(vec![0.5, 0.875, 1.]);
        assert!(!proportional(&a, &wrong, 1., 0.5, &mut w).unwrap());
    }
    #[test]
    fn polynomial_coefficients_are_exact_on_original_absolute_cells() {
        let a = Curve {
            degree: 2,
            knots: vec![-2., -2., -2., 6., 6., 6.],
            control_points: vec![vec![0., 0., 0.]; 3],
            weights: vec![1., 2., 4.],
            periodic: false,
        };
        let mut w = Work {
            used: 0,
            limit: 1_000_000,
        };
        assert_eq!(
            polynomial(&a, -2., 6., &mut w).unwrap(),
            vec![Q::one(), Q::from_integer(2.into()), Q::one()]
        );
        let b = Curve {
            degree: 2,
            knots: vec![-2., -2., -2., 2., 6., 6., 6.],
            control_points: vec![vec![0., 0., 0.]; 4],
            weights: vec![1., 1.5, 3., 4.],
            periodic: false,
        };
        assert!(proportional(&a, &b, 1., 1., &mut w).unwrap());
        let mut low = Work { used: 0, limit: 1 };
        assert_eq!(
            proportional(&a, &b, 1., 1., &mut low),
            Err(super::super::curve_edit::Refusal::ResourceLimit)
        );
        assert!(low.used <= 1);
    }
}
