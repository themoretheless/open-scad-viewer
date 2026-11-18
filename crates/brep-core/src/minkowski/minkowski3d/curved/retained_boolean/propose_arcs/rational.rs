//! Bounded rational interpolation proposals, never interpolation authority.
use super::*;

pub(super) fn reconstruct(b: [f64; 2]) -> Result<Q> {
    let mut lo = q(b[0]);
    let mut hi = q(b[1]);
    let mut floors = Vec::new();
    for _ in 0..80 {
        let integer = lo.ceil();
        if integer <= hi {
            let mut x = integer;
            for a in floors.into_iter().rev() {
                x = a + Q::one() / x;
            }
            if x.denom().bits() > 40 || x.numer().bits() > 256 {
                return Err(fail("rational sample reconstruction budget exceeded"));
            }
            return Ok(x);
        }
        let a = lo.floor();
        let next_lo = Q::one() / (&hi - &a);
        let next_hi = Q::one() / (&lo - &a);
        floors.push(a);
        lo = next_lo;
        hi = next_hi;
    }
    Err(fail("rational sample continued-fraction budget exceeded"))
}
fn gcd(mut a: num_bigint::BigInt, mut b: num_bigint::BigInt) -> num_bigint::BigInt {
    while !b.is_zero() {
        let r = &a % &b;
        a = b;
        b = r;
    }
    a.abs()
}
pub(super) fn fit(points: &[Vec<Q>], nodes: &[Q], degree: usize) -> Result<ArcProposal> {
    let dim = 7;
    let width = degree + 1;
    let columns = (dim + 1) * width;
    if points.len() != nodes.len()
        || points.len() != 2 * degree + 1
        || points.iter().any(|p| p.len() != dim)
        || !(1..=8).contains(&degree)
    {
        return Err(fail("invalid bounded rational interpolation proposal"));
    }
    let choose = |k: usize| -> Q {
        (0..k).fold(Q::one(), |a, i| {
            a * integer((degree - i) as i64) / integer((i + 1) as i64)
        })
    };
    let mut rows = Vec::new();
    for (p, t) in points.iter().zip(nodes) {
        let basis = (0..width)
            .map(|i| choose(i) * t.pow(i as i32) * (Q::one() - t).pow((degree - i) as i32))
            .collect::<Vec<_>>();
        for k in 0..dim {
            let mut row = vec![Q::zero(); columns];
            for i in 0..width {
                row[k * width + i] = basis[i].clone();
                row[dim * width + i] = -&p[k] * &basis[i];
            }
            rows.push(row);
        }
    }
    let vectors = atlas::linear::kernel(rows, columns, &mut Work::default())?;
    for mut v in vectors {
        if v[dim * width].is_negative() {
            for q in &mut v {
                *q = -q.clone();
            }
        }
        let w = &v[dim * width..];
        if w.iter().any(|q| q <= &Q::zero()) {
            continue;
        }
        let attempt = (|| -> Result<ArcProposal> {
            let mut scale = num_bigint::BigInt::one();
            for q in w {
                let d = q.denom();
                scale = (&scale / gcd(scale.clone(), d.clone())) * d;
                if scale.bits() > 4096 {
                    return Err(fail("rational weight normalization budget exceeded"));
                }
            }
            let weights = w
                .iter()
                .map(|q| q * Q::from_integer(scale.clone()))
                .collect::<Vec<_>>();
            let divisor = weights
                .iter()
                .map(|q| q.numer().clone())
                .reduce(gcd)
                .unwrap();
            let weights = weights
                .iter()
                .map(|q| float(&(q / Q::from_integer(divisor.clone()))))
                .collect::<Result<Vec<_>>>()?;
            let curve = |axes: &[usize]| -> Result<Curve> {
                Ok(Curve {
                    degree,
                    knots: [vec![0.; width], vec![1.; width]].concat(),
                    control_points: (0..width)
                        .map(|i| {
                            axes.iter()
                                .map(|&k| float(&(&v[k * width + i] / &w[i])))
                                .collect()
                        })
                        .collect::<Result<_>>()?,
                    weights: weights.clone(),
                    periodic: false,
                })
            };
            Ok(ArcProposal {
                curve: curve(&[0, 1, 2])?,
                pc: [curve(&[3, 4])?, curve(&[5, 6])?],
            })
        })();
        if let Ok(a) = attempt {
            return Ok(a);
        }
    }
    Err(fail("no positive exactly saved rational trace proposal"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_degree_reduction_avoids_unrepresentable_elevated_affine_poles() {
        let nodes = (0..=5)
            .map(|i| {
                if i == 5 {
                    Q::one()
                } else {
                    Q::new(i.into(), 8.into())
                }
            })
            .collect::<Vec<_>>();
        let points = nodes
            .iter()
            .map(|t| vec![t.clone(), integer(2) * t + integer(1)])
            .collect::<Vec<_>>();
        let p = super::super::interpolate(&points, &nodes).unwrap();
        assert_eq!(p.degree, 1);
        assert_eq!(p.control_points, vec![vec![0., 1.], vec![1., 3.]]);
    }
    #[test]
    fn reconstructs_nondyadic_samples_and_a_positive_rational_clock() {
        let r = reconstruct([1. / 3. - 1e-12, 1. / 3. + 1e-12]).unwrap();
        assert_eq!(r, Q::new(1.into(), 3.into()));
        let nodes = (0..=4)
            .map(|i| Q::new(i.into(), 4.into()))
            .collect::<Vec<_>>();
        let points = nodes
            .iter()
            .map(|t| {
                let z = t / (integer(3) - integer(2) * t);
                vec![
                    t.clone(),
                    Q::zero(),
                    z.clone(),
                    t.clone(),
                    z,
                    t.clone(),
                    Q::zero(),
                ]
            })
            .collect::<Vec<_>>();
        let p = fit(&points, &nodes, 2).unwrap();
        assert_eq!(p.curve.weights, vec![3., 2., 1.]);
        assert_eq!(
            p.curve.control_points,
            vec![vec![0., 0., 0.], vec![0.75, 0., 0.25], vec![1., 0., 1.]]
        );
        exact::monotone(&p.pc[0], 0, true).unwrap();
        let plane = crate::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 1.]],
                vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let strip = crate::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.75, 0., 0.25], vec![0.75, 1., 0.25]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![3.; 2], vec![2.; 2], vec![1.; 2]],
            periodic_u: false,
            periodic_v: false,
        };
        exact::lift(&p.curve, &p.pc[0], &plane, false).unwrap();
        exact::lift(&p.curve, &p.pc[1], &strip, false).unwrap();
        let mut bad = p.pc[0].clone();
        bad.control_points[1][0] = -1.;
        assert!(exact::monotone(&bad, 0, true).is_err());
    }
}
