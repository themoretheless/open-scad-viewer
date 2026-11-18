//! Derivatives of arbitrary order for rational curves and surfaces, per The
//! NURBS Book: A2.3 for the basis, A3.2 with A4.2 for curves and A3.6 with
//! A4.4 for surfaces.
//!
//! [`Curve::evaluate`] and [`Surface::evaluate`] stay the entry points for
//! positions with first and second derivatives, continuity metadata and
//! rounding evidence. The functions here answer the remaining question: the
//! derivative of any order at a parameter.
//!
//! # Knots
//! A spline of degree `p` is only `C^(p-m)` at a knot of multiplicity `m`.
//! At such a parameter the result is the one-sided derivative of the span to
//! the right, and of the last span at the end of the domain. Away from knots
//! every order is two-sided.
use crate::curve::{Curve, find_span};
use crate::surface::Surface;
use crate::{Result, check, numeric};

/// Highest order accepted by [`Curve::derivatives`] and
/// [`Surface::derivatives`]. Each further order multiplies rounding error by
/// roughly `degree / span length`, so far higher orders carry no digits.
pub const MAX_DERIVATIVE_ORDER: usize = 16;

/// The span whose polynomial piece defines the derivatives at `u`: the one to
/// the right of `u`, or the last nonempty one at the end of the domain.
fn derivative_span(degree: usize, knots: &[f64], controls: usize, u: f64) -> Result<usize> {
    let mut span = find_span(degree, knots, controls, u)?;
    while span > degree && knots[span] == knots[span + 1] {
        span -= 1;
    }
    Ok(span)
}

/// Derivatives of the `degree + 1` basis functions that are nonzero on
/// `span`, for orders `0..=order`. `result[k][j]` is the `k`-th derivative of
/// the basis function of control index `span - degree + j`. Rows above the
/// degree are zero, as the piece is a polynomial of that degree.
pub fn basis_derivatives(
    degree: usize,
    knots: &[f64],
    span: usize,
    u: f64,
    order: usize,
) -> Result<Vec<Vec<f64>>> {
    let p = degree;
    check((1..=25).contains(&p), "Degree must be an integer in [1, 25]")?;
    check(
        order <= MAX_DERIVATIVE_ORDER,
        "Derivative order must not exceed 16",
    )?;
    check(
        span >= p && span + p + 1 <= knots.len(),
        "Span is outside the valid knot range",
    )?;
    check(
        u.is_finite() && u >= knots[span] && u <= knots[span + 1],
        "Parameter is outside the given span",
    )?;
    // ndu[j][r] for r < j holds knot differences, ndu[r][j] basis values.
    let mut ndu = vec![vec![0.; p + 1]; p + 1];
    let mut left = vec![0.; p + 1];
    let mut right = vec![0.; p + 1];
    ndu[0][0] = 1.;
    for j in 1..=p {
        left[j] = u - knots[span + 1 - j];
        right[j] = knots[span + j] - u;
        let mut saved = 0.;
        for r in 0..j {
            ndu[j][r] = right[r + 1] + left[j - r];
            // 0/0 terms are defined as 0.
            let temp = if ndu[j][r] == 0. {
                0.
            } else {
                ndu[r][j - 1] / ndu[j][r]
            };
            ndu[r][j] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        ndu[j][j] = saved;
    }
    let mut ders = vec![vec![0.; p + 1]; order + 1];
    for j in 0..=p {
        ders[0][j] = ndu[j][p];
    }
    let top = order.min(p);
    let mut a = [vec![0.; p + 1], vec![0.; p + 1]];
    for r in 0..=p {
        let (mut s1, mut s2) = (0, 1);
        a[0][0] = 1.;
        for k in 1..=top {
            let mut d = 0.;
            let pk = p - k;
            if r >= k {
                let den = ndu[pk + 1][r - k];
                a[s2][0] = if den == 0. { 0. } else { a[s1][0] / den };
                d = a[s2][0] * ndu[r - k][pk];
            }
            let j1 = if r + 1 >= k { 1 } else { k - r };
            let j2 = if r <= pk + 1 { k - 1 } else { p - r };
            for j in j1..=j2 {
                // j >= k - r here, so the control offset is not negative.
                let index = r + j - k;
                let den = ndu[pk + 1][index];
                a[s2][j] = if den == 0. {
                    0.
                } else {
                    (a[s1][j] - a[s1][j - 1]) / den
                };
                d += a[s2][j] * ndu[index][pk];
            }
            if r <= pk {
                let den = ndu[pk + 1][r];
                a[s2][k] = if den == 0. { 0. } else { -a[s1][k - 1] / den };
                d += a[s2][k] * ndu[r][pk];
            }
            ders[k][r] = d;
            std::mem::swap(&mut s1, &mut s2);
        }
    }
    let mut factor = p as f64;
    for (k, row) in ders.iter_mut().enumerate().take(top + 1).skip(1) {
        for value in row {
            *value *= factor;
        }
        factor *= (p - k) as f64;
    }
    numeric(
        ders.iter().flatten().all(|v| v.is_finite()),
        "Knot scale exhausted finite derivative precision",
    )?;
    Ok(ders)
}

/// Pascal's triangle up to `order`; exact in binary64 for the admitted orders.
fn binomials(order: usize) -> Vec<Vec<f64>> {
    let mut rows = vec![vec![1.]];
    for n in 1..=order {
        let previous = &rows[n - 1];
        let mut row = vec![1.; n + 1];
        for k in 1..n {
            row[k] = previous[k - 1] + previous[k];
        }
        rows.push(row);
    }
    rows
}

fn check_order(order: usize) -> Result<()> {
    check(
        order <= MAX_DERIVATIVE_ORDER,
        "Derivative order must not exceed 16",
    )
}

impl Curve {
    /// The point and its derivatives with respect to the curve parameter:
    /// `result[k]` is the `k`-th derivative at `u` for `k` in `0..=order`,
    /// with as many coordinates as the control points. See the module notes
    /// for the meaning at knots.
    pub fn derivatives(&self, u: f64, order: usize) -> Result<Vec<Vec<f64>>> {
        check_order(order)?;
        self.validate()?;
        let p = self.degree;
        let span = derivative_span(p, &self.knots, self.control_points.len(), u)?;
        let basis = basis_derivatives(p, &self.knots, span, u, order)?;
        let dim = self.control_points[0].len();
        let first = span - p;
        // Same conditioning as `evaluate`: weights relative to the largest
        // one, coordinates relative to a control point of the active window.
        // Neither changes the quotient, and the shift only enters order zero.
        let scale = self.weights.iter().copied().fold(0., f64::max);
        let origin = &self.control_points[first];
        let mut numerator = vec![vec![0.; dim]; order + 1];
        let mut weight = vec![0.; order + 1];
        for k in 0..=order {
            for j in 0..=p {
                let w = basis[k][j] * (self.weights[first + j] / scale);
                weight[k] += w;
                for axis in 0..dim {
                    numerator[k][axis] += w * (self.control_points[first + j][axis] - origin[axis]);
                }
            }
        }
        numeric(
            weight[0] > 0. && weight[0].is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        let binomial = binomials(order);
        let mut result: Vec<Vec<f64>> = Vec::with_capacity(order + 1);
        for k in 0..=order {
            let mut value = numerator[k].clone();
            for i in 1..=k {
                for axis in 0..dim {
                    value[axis] -= binomial[k][i] * weight[i] * result[k - i][axis];
                }
            }
            for coordinate in &mut value {
                *coordinate /= weight[0];
            }
            result.push(value);
        }
        for axis in 0..dim {
            result[0][axis] += origin[axis];
        }
        numeric(
            result.iter().flatten().all(|v| v.is_finite()),
            "Curve derivatives exceeded finite precision",
        )?;
        Ok(result)
    }
}

impl Surface {
    /// The point and its mixed partial derivatives up to total order `order`:
    /// `result[k][l]` is the derivative `k` times in `u` and `l` times in `v`
    /// for `k + l <= order`, so row `k` has `order - k + 1` entries. See the
    /// module notes for the meaning at knots.
    pub fn derivatives(&self, u: f64, v: f64, order: usize) -> Result<Vec<Vec<[f64; 3]>>> {
        check_order(order)?;
        self.validate()?;
        let (p, q) = (self.degree_u, self.degree_v);
        let span_u = derivative_span(p, &self.knots_u, self.control_points.len(), u)?;
        let span_v = derivative_span(q, &self.knots_v, self.control_points[0].len(), v)?;
        let basis_u = basis_derivatives(p, &self.knots_u, span_u, u, order)?;
        let basis_v = basis_derivatives(q, &self.knots_v, span_v, v, order)?;
        let (first_u, first_v) = (span_u - p, span_v - q);
        let scale = self.weights.iter().flatten().copied().fold(0., f64::max);
        let origin: [f64; 3] = std::array::from_fn(|k| self.control_points[first_u][first_v][k]);
        let mut numerator = vec![vec![[0.; 3]; order + 1]; order + 1];
        let mut weight = vec![vec![0.; order + 1]; order + 1];
        for k in 0..=order {
            // Contract the V direction once per U basis row and V order.
            for l in 0..=order - k {
                for i in 0..=p {
                    let row = &self.control_points[first_u + i];
                    let weights = &self.weights[first_u + i];
                    for j in 0..=q {
                        let w = basis_u[k][i] * basis_v[l][j] * (weights[first_v + j] / scale);
                        weight[k][l] += w;
                        for axis in 0..3 {
                            numerator[k][l][axis] += w * (row[first_v + j][axis] - origin[axis]);
                        }
                    }
                }
            }
        }
        numeric(
            weight[0][0] > 0. && weight[0][0].is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        let binomial = binomials(order);
        let mut result = vec![vec![[0.; 3]; order + 1]; order + 1];
        for k in 0..=order {
            for l in 0..=order - k {
                let mut value = numerator[k][l];
                for j in 1..=l {
                    for axis in 0..3 {
                        value[axis] -= binomial[l][j] * weight[0][j] * result[k][l - j][axis];
                    }
                }
                for i in 1..=k {
                    for axis in 0..3 {
                        value[axis] -= binomial[k][i] * weight[i][0] * result[k - i][l][axis];
                    }
                    let mut inner = [0.; 3];
                    for j in 1..=l {
                        for axis in 0..3 {
                            inner[axis] +=
                                binomial[l][j] * weight[i][j] * result[k - i][l - j][axis];
                        }
                    }
                    for axis in 0..3 {
                        value[axis] -= binomial[k][i] * inner[axis];
                    }
                }
                result[k][l] = value.map(|c| c / weight[0][0]);
            }
        }
        for axis in 0..3 {
            result[0][0][axis] += origin[axis];
        }
        for (k, row) in result.iter_mut().enumerate() {
            row.truncate(order - k + 1);
        }
        numeric(
            result.iter().flatten().flatten().all(|v| v.is_finite()),
            "Surface derivatives exceeded finite precision",
        )?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quarter_circle() -> Curve {
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        }
    }

    /// A rational cubic with interior knots of multiplicity one and two.
    fn rational_cubic() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 2., 2., 4., 4., 4., 4.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.5],
                vec![2., -1., 1.],
                vec![4., 0.5, 3.],
                vec![5., 3., 2.],
                vec![7., 1., -1.],
                vec![8., 0., 0.],
            ],
            weights: vec![1., 0.7, 2.5, 1.2, 0.4, 3., 1.],
            periodic: false,
        }
    }

    fn rational_surface() -> Surface {
        let control_points = (0..5)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let (x, y) = (i as f64, j as f64);
                        vec![x, y, (0.7 * x).sin() * (0.9 * y + 0.3).cos() + 0.1 * x * y]
                    })
                    .collect()
            })
            .collect();
        let weights = (0..5)
            .map(|i| (0..4).map(|j| 1. + 0.3 * ((i * 3 + j * 5) % 4) as f64).collect())
            .collect();
        Surface {
            degree_u: 3,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 0., 1., 2., 2., 2., 2.],
            knots_v: vec![0., 0., 0., 1., 3., 3., 3.],
            control_points,
            weights,
            periodic_u: false,
            periodic_v: false,
        }
    }

    fn close(a: &[f64], b: &[f64], tolerance: f64) {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert!(
                (x - y).abs() <= tolerance * (1. + x.abs().max(y.abs())),
                "{a:?} vs {b:?}"
            );
        }
    }

    #[test]
    fn low_orders_agree_with_curve_evaluation() {
        for curve in [quarter_circle(), rational_cubic()] {
            let [start, end] = curve.domain();
            for step in 0..=40 {
                let u = start + (end - start) * step as f64 / 40.;
                let evaluation = curve.evaluate(u).unwrap();
                let derivatives = curve.derivatives(u, 2).unwrap();
                close(&derivatives[0], &evaluation.point, 1e-13);
                // `evaluate` withholds derivatives it cannot define two-sided.
                if let Some(d1) = &evaluation.d1
                    && evaluation.derivative_side == "two_sided"
                {
                    close(&derivatives[1], d1, 1e-11);
                }
                if let Some(d2) = &evaluation.d2
                    && evaluation.derivative_side == "two_sided"
                {
                    close(&derivatives[2], d2, 1e-10);
                }
            }
        }
    }

    #[test]
    fn curve_derivatives_reproduce_the_taylor_expansion() {
        // (parameter, step) pairs that stay inside one knot span.
        for (curve, samples) in [
            (quarter_circle(), vec![(0.3, 0.02), (0.6, -0.015)]),
            (rational_cubic(), vec![(0.4, 0.01), (1.5, -0.012), (3., 0.015)]),
        ] {
            for (u, h) in samples {
                let derivatives = curve.derivatives(u, 12).unwrap();
                let mut series = vec![0.; derivatives[0].len()];
                let mut term = 1.;
                for (k, derivative) in derivatives.iter().enumerate() {
                    if k > 0 {
                        term *= h / k as f64;
                    }
                    for (sum, value) in series.iter_mut().zip(derivative) {
                        *sum += term * value;
                    }
                }
                close(&series, &curve.evaluate(u + h).unwrap().point, 1e-11);
            }
        }
    }

    #[test]
    fn circle_derivatives_satisfy_the_differentiated_unit_norm() {
        let curve = quarter_circle();
        let dot = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f64>();
        for u in [0., 0.2, 0.5, 0.9, 1.] {
            let d = curve.derivatives(u, 4).unwrap();
            // Derivatives of |C|^2 = 1 of orders one to four all vanish.
            assert!((dot(&d[0], &d[0]) - 1.).abs() < 1e-14);
            assert!(dot(&d[0], &d[1]).abs() < 1e-13);
            assert!((dot(&d[1], &d[1]) + dot(&d[0], &d[2])).abs() < 1e-12);
            assert!((3. * dot(&d[1], &d[2]) + dot(&d[0], &d[3])).abs() < 1e-11);
            let fourth = 3. * dot(&d[2], &d[2]) + 4. * dot(&d[1], &d[3]) + dot(&d[0], &d[4]);
            assert!(fourth.abs() < 1e-10, "{fourth}");
        }
    }

    #[test]
    fn polynomial_pieces_vanish_above_their_degree() {
        let mut curve = rational_cubic();
        curve.weights = vec![1.; 7];
        let d = curve.derivatives(0.5, 6).unwrap();
        assert!(d[3].iter().any(|v| v.abs() > 1e-3));
        for higher in &d[4..] {
            assert!(higher.iter().all(|v| *v == 0.), "{higher:?}");
        }
        // The third derivative is constant on a span of a cubic.
        close(&d[3], &curve.derivatives(0.9, 3).unwrap()[3], 1e-10);
    }

    #[test]
    fn knots_and_domain_ends_take_the_documented_side() {
        let curve = rational_cubic();
        // Double knot at 2: C1 only, so the second derivative is one-sided.
        let at = curve.derivatives(2., 2).unwrap();
        let right = curve.derivatives(2. + 1e-9, 2).unwrap();
        let left = curve.derivatives(2. - 1e-9, 2).unwrap();
        close(&at[2], &right[2], 1e-6);
        assert!(
            at[2]
                .iter()
                .zip(&left[2])
                .any(|(a, b)| (a - b).abs() > 1e-2)
        );
        close(&at[1], &left[1], 1e-6);
        // The end of the domain belongs to the last span.
        let end = curve.derivatives(4., 3).unwrap();
        close(&end[0], &[8., 0., 0.], 1e-14);
        close(&end[3], &curve.derivatives(4. - 1e-9, 3).unwrap()[3], 1e-5);
    }

    #[test]
    fn low_orders_agree_with_surface_evaluation() {
        let surface = rational_surface();
        for (u, v) in [(0.25, 0.5), (1.5, 2.), (0.9, 0.1), (1.99, 2.9)] {
            let evaluation = surface.evaluate(u, v).unwrap();
            let d = surface.derivatives(u, v, 2).unwrap();
            assert_eq!(d.iter().map(Vec::len).collect::<Vec<_>>(), [3, 2, 1]);
            close(&d[0][0], &evaluation.point, 1e-13);
            let (du, dv) = evaluation.first_derivatives().unwrap();
            close(&d[1][0], &du, 1e-11);
            close(&d[0][1], &dv, 1e-11);
            let (duu, duv, dvv) = evaluation.second_derivatives().unwrap();
            close(&d[2][0], &duu, 1e-10);
            close(&d[1][1], &duv, 1e-10);
            close(&d[0][2], &dvv, 1e-10);
        }
    }

    #[test]
    fn surface_derivatives_reproduce_the_taylor_expansion() {
        let surface = rational_surface();
        for (u, v, hu, hv) in [(0.4, 0.5, 0.02, -0.015), (1.5, 2., -0.02, 0.025)] {
            let order = 12;
            let d = surface.derivatives(u, v, order).unwrap();
            let mut series = [0.; 3];
            let mut term_u = 1.;
            for (k, row) in d.iter().enumerate().take(order + 1) {
                if k > 0 {
                    term_u *= hu / k as f64;
                }
                let mut term = term_u;
                for (l, cell) in row.iter().enumerate().take(order - k + 1) {
                    if l > 0 {
                        term *= hv / l as f64;
                    }
                    for axis in 0..3 {
                        series[axis] += term * cell[axis];
                    }
                }
            }
            let expected = surface.evaluate(u + hu, v + hv).unwrap().point;
            close(&series, &expected, 1e-10);
        }
    }

    #[test]
    fn invalid_requests_are_refused() {
        let curve = quarter_circle();
        assert!(curve.derivatives(0.5, MAX_DERIVATIVE_ORDER + 1).is_err());
        assert!(curve.derivatives(1.5, 1).is_err());
        assert!(curve.derivatives(f64::NAN, 1).is_err());
        assert_eq!(curve.derivatives(0.5, 0).unwrap().len(), 1);
        let surface = rational_surface();
        assert!(surface.derivatives(0.5, 9., 1).is_err());
        assert!(
            surface
                .derivatives(0.5, 0.5, MAX_DERIVATIVE_ORDER + 1)
                .is_err()
        );
    }
}
