#![allow(dead_code)] // shared by several test targets; each one uses a subset
use super::Surface;
/// C(q)=10(q^5,q^2+q/4), q=2u-1. Every authored pole is dyadic.
/// X has a fourth-order critical projection at q=0, while Y'=5/2 there.
/// The alternative fixture C(q)=10(q^5,q^4) has actual rank loss.
fn pair(rank_loss: bool) -> (Surface, Surface) {
    let a = Surface {
        degree_u: 5,
        degree_v: 2,
        knots_u: [vec![0.; 6], vec![1.; 6]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..=5)
            .map(|i| {
                let i = i as i64;
                let x = if i % 2 == 0 { -10. } else { 10. };
                let y = if rank_loss {
                    (2 * (5 - 2 * i) * if i % 2 == 0 { 1 } else { -1 }) as f64
                } else {
                    (2 * i * i - 9 * i) as f64 + 7.5
                };
                (0..3).map(|j| vec![x, y, 0.5 * j as f64]).collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 6],
        periodic_u: false,
        periodic_v: false,
    };
    let mut b = a.clone();
    for (i, row) in b.control_points.iter_mut().enumerate() {
        for p in row {
            p[2] += 1.;
        }
        for w in &mut b.weights[i] {
            *w *= 2f64.powi(5 - i as i32);
        }
    }
    (a, b)
}
pub fn fixture() -> (Surface, Surface) {
    pair(false)
}
pub fn rank_loss() -> (Surface, Surface) {
    pair(true)
}
