//! Diagnostic counterexamples only. Point equality never admits an edge/body.
use num_rational::BigRational as Q;
use num_traits::{One, ToPrimitive, Zero};
use nurbs_core::{curve::Curve, surface::Surface};
fn q(x: f64) -> Q {
    Q::from_float(x).expect("finite authored value")
}
fn spline(degree: usize, knots: &[f64], poles: &[Vec<Q>], t: &Q) -> Vec<Q> {
    assert!(degree <= 32 && poles.len() <= 4096);
    assert!(*t >= q(knots[degree]) && *t <= q(knots[poles.len()]));
    let span = if *t == q(knots[poles.len()]) {
        poles.len() - 1
    } else {
        (degree..poles.len())
            .find(|&i| q(knots[i]) <= *t && *t < q(knots[i + 1]))
            .unwrap()
    };
    let mut d = poles[span - degree..=span].to_vec();
    for r in 1..=degree {
        for j in (r..=degree).rev() {
            let i = span - degree + j;
            let width = q(knots[i + degree - r + 1]) - q(knots[i]);
            assert!(!width.is_zero());
            let a = (t - q(knots[i])) / width;
            d[j] = (0..d[j].len())
                .map(|k| (Q::one() - &a) * &d[j - 1][k] + &a * &d[j][k])
                .collect();
        }
    }
    d.pop().unwrap()
}
fn curve(c: &Curve, t: &Q) -> Vec<Q> {
    assert!(!c.periodic);
    let poles = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(p, w)| {
            let w = q(*w);
            p.iter()
                .map(|v| q(*v) * &w)
                .chain(std::iter::once(w.clone()))
                .collect()
        })
        .collect::<Vec<_>>();
    let h = spline(c.degree, &c.knots, &poles, t);
    h[..h.len() - 1]
        .iter()
        .map(|v| v / h.last().unwrap())
        .collect()
}
fn surface(s: &Surface, uv: &[Q]) -> Vec<Q> {
    assert!(!s.periodic_u && !s.periodic_v);
    let rows = s
        .control_points
        .iter()
        .zip(&s.weights)
        .map(|(row, weights)| {
            let poles = row
                .iter()
                .zip(weights)
                .map(|(p, w)| {
                    let w = q(*w);
                    p.iter()
                        .map(|v| q(*v) * &w)
                        .chain(std::iter::once(w.clone()))
                        .collect()
                })
                .collect::<Vec<_>>();
            spline(s.degree_v, &s.knots_v, &poles, &uv[1])
        })
        .collect::<Vec<_>>();
    let h = spline(s.degree_u, &s.knots_u, &rows, &uv[0]);
    h[..3].iter().map(|v| v / &h[3]).collect()
}
pub fn inspect(c: &Curve, p: &Curve, s: &Surface, t: f64) -> [Q; 3] {
    c.validate().unwrap();
    p.validate().unwrap();
    s.validate().unwrap();
    let xyz = curve(c, &q(t));
    let uv = curve(p, &q(t));
    let lifted = surface(s, &uv);
    std::array::from_fn(|k| &xyz[k] - &lifted[k])
}
pub fn print(c: &Curve, p: &Curve, s: &Surface, side: usize, t: f64) {
    let residual = inspect(c, p, s, t);
    println!(
        "exact_contact_point side={side} t={t} equal={} residual_approx={:?} numerator_bits={:?}",
        residual.iter().all(Q::is_zero),
        residual.each_ref().map(|x| x.to_f64()),
        residual.each_ref().map(|x| x.numer().bits())
    );
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_point_detects_a_sub_float_evaluation_residual() {
        let c = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let p = Curve {
            control_points: vec![vec![0., 0.], vec![1., 0.]],
            ..c.clone()
        };
        let mut s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: c.knots.clone(),
            knots_v: c.knots.clone(),
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(inspect(&c, &p, &s, 0.5).iter().all(Q::is_zero));
        s.control_points[1][0][2] = 2f64.powi(-100);
        assert_eq!(inspect(&c, &p, &s, 0.5)[2], -q(2f64.powi(-101)));
    }
}
