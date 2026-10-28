//! Exact parameter-preserving rational-to-polynomial source equivalence.
use super::*;
#[derive(Clone, Debug)]
pub struct QualifiedPolynomialSurface {
    source: Surface,
    polynomial: Surface,
    limits: QuadraticGraphSSLimits,
    operations: usize,
}
pub fn qualify_polynomial_surface(
    source: &Surface,
    limits: QuadraticGraphSSLimits,
) -> Result<QualifiedPolynomialSurface> {
    source.validate()?;
    check(
        (1..=4).contains(&source.degree_u)
            && (1..=4).contains(&source.degree_v)
            && source.control_points.len() == source.degree_u + 1
            && source.control_points[0].len() == source.degree_v + 1
            && !source.periodic_u
            && !source.periodic_v,
        "Need one original degree1..4 Bezier support",
    )?;
    let domain = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    for (axis, knots, degree) in [
        (0, &source.knots_u, source.degree_u),
        (1, &source.knots_v, source.degree_v),
    ] {
        check(
            knots.len() == 2 * (degree + 1)
                && knots[..=degree].iter().all(|&t| t == domain[axis][0])
                && knots[degree + 1..].iter().all(|&t| t == domain[axis][1]),
            "Original polynomial equivalence needs complete clamped domains",
        )?;
    }
    let mut work = Work::new(limits.max_operations)?;
    let den = rational_graph::homogeneous(source, 3, &mut work)?;
    let mut coordinates = Vec::new();
    for axis in 0..3 {
        let numerator = rational_graph::homogeneous(source, axis, &mut work)?;
        coordinates.push(rational_graph::quotient(&numerator, &den, &mut work)?);
    }
    let basis = [[1., 0., 0.], [1., 0.5, 0.], [1., 1., 1.]];
    let mut points = vec![vec![vec![0.; 3]; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            for axis in 0..3 {
                let mut value = 0.;
                for a in 0..3 {
                    for b in 0..3 {
                        if basis[i][a] == 0. || basis[j][b] == 0. || coordinates[axis][a][b] == 0. {
                            continue;
                        }
                        let c = work.mul(coordinates[axis][a][b], basis[i][a])?;
                        let c = work.mul(c, basis[j][b])?;
                        value = work.add(value, c)?;
                    }
                }
                points[i][j][axis] = value;
            }
        }
    }
    let polynomial = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: [vec![domain[0][0]; 3], vec![domain[0][1]; 3]].concat(),
        knots_v: [vec![domain[1][0]; 3], vec![domain[1][1]; 3]].concat(),
        control_points: points,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    polynomial.validate()?;
    Ok(QualifiedPolynomialSurface {
        source: source.clone(),
        polynomial,
        limits,
        operations: work.used,
    })
}
impl QualifiedPolynomialSurface {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn polynomial(&self) -> &Surface {
        &self.polynomial
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = qualify_polynomial_surface(&self.source, self.limits)?;
        check(
            self.polynomial == fresh.polynomial && self.operations == fresh.operations,
            "Original polynomial surface equivalence changed",
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_original_products_prove_parameter_preserving_reduction() {
        let x = [0., 0.0625, 0.5, 0.9375, 1.];
        let square = [0., 0., 1., 26.25, 30.];
        let source = Surface {
            degree_u: 4,
            degree_v: 2,
            knots_u: vec![2., 2., 2., 2., 2., 6., 6., 6., 6., 6.],
            knots_v: vec![-3., -3., -3., 1., 1., 1.],
            control_points: (0..5)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                x[i],
                                j as f64 / 2.,
                                2. + square[i] / 64. + [0.25, -0.25, 0.25][j],
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: [1., 4., 5., 4., 1.]
                .into_iter()
                .map(|w| vec![w; 3])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let before = source.clone();
        let proof = qualify_polynomial_surface(
            &source,
            QuadraticGraphSSLimits {
                max_operations: 4096,
            },
        )
        .unwrap();
        proof.reverify().unwrap();
        assert_eq!(source, before);
        for u in [2., 2.5, 3., 4., 5.5, 6.] {
            for v in [-3., -2., 0., 1.] {
                let a = source.evaluate(u, v).unwrap().point;
                let b = proof.polynomial().evaluate(u, v).unwrap().point;
                for k in 0..3 {
                    assert!((a[k] - b[k]).abs() < 1e-12);
                }
            }
        }
        let work = proof.operations();
        assert!(
            qualify_polynomial_surface(
                &source,
                QuadraticGraphSSLimits {
                    max_operations: work - 1
                }
            )
            .is_err()
        );
        assert_eq!(
            qualify_polynomial_surface(
                &source,
                QuadraticGraphSSLimits {
                    max_operations: work
                }
            )
            .unwrap()
            .operations(),
            work
        );
        let mut changed = source.clone();
        changed.control_points[2][1][0] = changed.control_points[2][1][0].next_up();
        assert!(
            qualify_polynomial_surface(
                &changed,
                QuadraticGraphSSLimits {
                    max_operations: 4096
                }
            )
            .is_err()
        );
    }
    #[test]
    fn high_order_original_tail_is_not_a_low_order_taylor_match() {
        let source = Surface {
            degree_u: 3,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            vec![
                                if i == 3 { 1. } else { 0. },
                                j as f64 / 2.,
                                2. + if i == 3 { 1. } else { 0. },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        source.validate().unwrap();
        assert!(
            qualify_polynomial_surface(
                &source,
                QuadraticGraphSSLimits {
                    max_operations: 4096
                }
            )
            .is_err()
        );
    }
}
