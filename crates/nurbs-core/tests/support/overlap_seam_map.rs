use super::Surface;
/// A regular closed Cartesian trace with no globally monotone XYZ coordinate.
/// qA=s+s²-1, qB=t+t³-1; C(q)=(q-q³,q²). Original source cuts at
/// s=1/2 and t=1/4 have distinct images. All coefficients are exact dyadics.
pub fn fixture() -> (Surface, Surface) {
    fn product(a: &[i64], b: &[i64]) -> Vec<i64> {
        let mut out = vec![0; a.len() + b.len() - 1];
        for (i, x) in a.iter().enumerate() {
            for (j, y) in b.iter().enumerate() {
                out[i + j] += x * y;
            }
        }
        out
    }
    fn choose(n: usize, k: usize) -> i64 {
        let mut v = 1;
        for j in 0..k {
            v = v * (n - j) as i64 / (j + 1) as i64;
        }
        v
    }
    fn make(clock: Vec<i64>, at: f64, z: f64) -> Surface {
        let y = product(&clock, &clock);
        let q3 = product(&y, &clock);
        let mut x = vec![0; q3.len()];
        for (k, c) in q3.iter().enumerate() {
            x[k] = -c;
        }
        for (k, c) in clock.iter().enumerate() {
            x[k] += c;
        }
        let degree = x.len() - 1;
        let coeff = |p: &[i64], i: usize| -> f64 {
            p.iter()
                .enumerate()
                .take(i + 1)
                .map(|(k, c)| {
                    assert_eq!(2520 % choose(degree, k), 0);
                    c * choose(i, k) * (2520 / choose(degree, k))
                })
                .sum::<i64>() as f64
        };
        let original = (0..=degree)
            .map(|i| {
                (0..3)
                    .map(|j| vec![coeff(&x, i), coeff(&y, i), z + 0.5 * j as f64])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut work = original.clone();
        let mut left = vec![work[0].clone()];
        let mut right = vec![work[degree].clone()];
        for level in 1..=degree {
            for i in 0..=degree - level {
                for j in 0..3 {
                    for k in 0..3 {
                        work[i][j][k] = (1. - at) * work[i][j][k] + at * work[i + 1][j][k];
                    }
                }
            }
            left.push(work[0].clone());
            right.push(work[degree - level].clone());
        }
        right.reverse();
        left.extend(right.into_iter().skip(1));
        let mut knots = vec![0.; degree + 1];
        knots.extend(vec![at; degree]);
        knots.extend(vec![1.; degree + 1]);
        Surface {
            degree_u: degree,
            degree_v: 2,
            knots_u: knots,
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            weights: vec![vec![1.; 3]; left.len()],
            control_points: left,
            periodic_u: false,
            periodic_v: false,
        }
    }
    (
        make(vec![-1, 1, 1], 0.5, 0.),
        make(vec![-1, 1, 0, 1], 0.25, 1.),
    )
}
