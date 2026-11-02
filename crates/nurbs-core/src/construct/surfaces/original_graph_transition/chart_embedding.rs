//! Whole-chart injectivity via an affine physical clock and strict transverse
//! monotonicity. A regularity sample is never promoted to global embedding.
use super::*;
pub struct PolynomialChartEmbedding {
    parameter_axis: usize,
    physical_clock_axis: usize,
    transverse_axis: usize,
}
impl PolynomialChartEmbedding {
    pub fn parameter_axis(&self) -> usize {
        self.parameter_axis
    }
    pub fn physical_clock_axis(&self) -> usize {
        self.physical_clock_axis
    }
    pub fn transverse_axis(&self) -> usize {
        self.transverse_axis
    }
}
fn inspect(net: &[Vec<[Q; 3]>], b: &mut Budget) -> Result<Option<PolynomialChartEmbedding>> {
    check(
        (2..=33).contains(&net.len())
            && (2..=33).contains(&net[0].len())
            && net.iter().all(|r| r.len() == net[0].len()),
        "Invalid polynomial embedding tensor",
    )?;
    for point in net.iter().flatten() {
        for x in point {
            b.keep(x.clone())?;
        }
    }
    for axis in 0..2 {
        let n = if axis == 0 {
            net.len() - 1
        } else {
            net[0].len() - 1
        };
        let m = if axis == 0 {
            net[0].len() - 1
        } else {
            net.len() - 1
        };
        let point = |i: usize, j: usize| if axis == 0 { &net[i][j] } else { &net[j][i] };
        for k in 0..3 {
            let low = point(0, 0)[k].clone();
            let span = b.keep(&point(n, 0)[k] - &low)?;
            if span.is_zero() {
                continue;
            }
            let mut affine = true;
            for i in 0..=n {
                let expected = b.keep(&low + &span * ratio(i as u64, n as u64))?;
                for j in 0..=m {
                    if !b.keep(&point(i, j)[k] - &expected)?.is_zero() {
                        affine = false;
                    }
                }
            }
            if !affine {
                continue;
            }
            for other in (0..3).filter(|other| *other != k) {
                let mut sign = None;
                let mut strict = true;
                for i in 0..=n {
                    for j in 0..m {
                        let d = b.keep(&point(i, j + 1)[other] - &point(i, j)[other])?;
                        if d.is_zero() {
                            strict = false;
                        } else if let Some(s) = sign {
                            if s != d.is_positive() {
                                strict = false;
                            }
                        } else {
                            sign = Some(d.is_positive());
                        }
                    }
                }
                if strict && sign.is_some() {
                    return Ok(Some(PolynomialChartEmbedding {
                        parameter_axis: axis,
                        physical_clock_axis: k,
                        transverse_axis: other,
                    }));
                }
            }
        }
    }
    Ok(None)
}
impl Patch {
    /// Complete exact chart is injective, hence every trimmed subset is too.
    pub fn inspect_chart_embedding(
        &self,
        b: &mut Budget,
    ) -> Result<Option<PolynomialChartEmbedding>> {
        inspect(&self.coefficients, b)
    }
}
pub fn polynomial_surface_embedding(
    s: &Surface,
    b: &mut Budget,
) -> Result<Option<PolynomialChartEmbedding>> {
    s.validate()?;
    let (n, m) = (s.degree_u, s.degree_v);
    check(
        !s.periodic_u
            && !s.periodic_v
            && n <= 32
            && m <= 32
            && s.control_points.len() == n + 1
            && s.control_points[0].len() == m + 1
            && s.control_points.iter().flatten().all(|p| p.len() == 3)
            && s.weights.iter().flatten().all(|w| *w == s.weights[0][0]),
        "Embedding requires polynomial Bezier support",
    )?;
    for (knots, d) in [(&s.knots_u, n), (&s.knots_v, m)] {
        check(
            knots[..=d].iter().all(|x| *x == knots[d])
                && knots[d + 1..].iter().all(|x| *x == knots[d + 1]),
            "Embedding requires original clamped knots",
        )?;
    }
    let net = s
        .control_points
        .iter()
        .map(|r| {
            r.iter()
                .map(|p| Ok([b.authored(p[0])?, b.authored(p[1])?, b.authored(p[2])?]))
                .collect()
        })
        .collect::<Result<Vec<_>>>()?;
    inspect(&net, b)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn surface() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0.25, 3.], vec![0., 1., 1.]],
                vec![vec![1., 0., 4.], vec![1., 0.25, -2.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 3]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn full_polynomial_clock_and_transverse_derivative_prove_embedding() {
        let s = surface();
        let mut b = Budget::new(100_000).unwrap();
        let r = polynomial_surface_embedding(&s, &mut b).unwrap().unwrap();
        assert_eq!(
            (
                r.parameter_axis(),
                r.physical_clock_axis(),
                r.transverse_axis()
            ),
            (0, 0, 1)
        );
        let mut swapped = s.clone();
        for row in &mut swapped.control_points {
            for p in row {
                p.swap(0, 2);
                p[1] = -p[1];
            }
        }
        let r = polynomial_surface_embedding(&swapped, &mut b)
            .unwrap()
            .unwrap();
        assert_eq!(
            (
                r.parameter_axis(),
                r.physical_clock_axis(),
                r.transverse_axis()
            ),
            (0, 2, 1)
        );
    }
    #[test]
    fn a_folded_surface_is_not_certified_from_its_regular_endpoints() {
        let mut s = surface();
        for row in &mut s.control_points {
            for (j, p) in row.iter_mut().enumerate() {
                p[1] = [0.25, -0.25, 0.25][j];
                p[2] = 0.;
            }
        }
        let mut b = Budget::new(100_000).unwrap();
        assert!(polynomial_surface_embedding(&s, &mut b).unwrap().is_none());
    }
    #[test]
    fn embedding_refuses_incomplete_budget_and_rational_support() {
        let mut s = surface();
        assert!(polynomial_surface_embedding(&s, &mut Budget::new(0).unwrap()).is_err());
        s.weights[0][1] = 2.;
        assert!(polynomial_surface_embedding(&s, &mut Budget::new(100_000).unwrap()).is_err());
    }
}
