//! Local C1 cubic curve interpolation (The NURBS Book, section 9.3.4).
//!
//! Each segment depends only on its two sites and their neighbours, so moving
//! one site changes at most the four segments around it. This is the
//! difference from global interpolation ([`super::interpolation`]), where one
//! site influences the whole curve.
//!
//! The derivative at each site is the Bessel estimate: the derivative of the
//! parabola through the site and its two neighbours. The curve is a cubic
//! Hermite spline written as a B-spline with double interior knots, so it is
//! C1 at every site and its parameter at site `k` is exactly `parameters[k]`.
use super::interpolation::{Parameterization, parameters};
use super::{Curve, Result, check};

fn difference(a: [f64; 3], b: [f64; 3], scale: f64) -> [f64; 3] {
    std::array::from_fn(|k| (a[k] - b[k]) / scale)
}

/// Interpolate `points` with a C1 cubic. `end_tangents`, when given, are the
/// derivatives with respect to the normalized parameter at the first and last
/// site; otherwise the end derivatives come from the end parabolas.
///
/// Consecutive sites must be distinct under the chosen parameterization.
/// The result has `3 * points.len() - 2` control points, so at most 86 sites
/// fit the 256 control point limit.
pub fn interpolate_curve_local(
    points: &[[f64; 3]],
    param: Parameterization,
    end_tangents: Option<([f64; 3], [f64; 3])>,
) -> Result<Curve> {
    let n = points.len();
    check(
        (2..=86).contains(&n),
        "Local cubic interpolation needs 2..86 data sites",
    )?;
    check(
        points
            .iter()
            .flatten()
            .all(|x| x.is_finite() && x.abs() <= 1e9),
        "Interpolation sites must be finite and bounded by 1e9",
    )?;
    if let Some((a, b)) = end_tangents {
        check(
            a.iter().chain(&b).all(|x| x.is_finite()),
            "End tangents must be finite",
        )?;
    }
    let u = parameters(points, param);
    let steps: Vec<f64> = u.windows(2).map(|w| w[1] - w[0]).collect();
    check(
        steps.iter().all(|&h| h > 0.)
            && points.windows(2).all(|w| w[0] != w[1]),
        "Consecutive interpolation sites must be distinct",
    )?;
    // Divided differences: the slope of each chord in parameter units.
    let slopes: Vec<[f64; 3]> = (0..n - 1)
        .map(|k| difference(points[k + 1], points[k], steps[k]))
        .collect();
    let mut tangents = vec![[0.; 3]; n];
    for k in 1..n - 1 {
        let alpha = steps[k - 1] / (steps[k - 1] + steps[k]);
        tangents[k] = std::array::from_fn(|i| (1. - alpha) * slopes[k - 1][i] + alpha * slopes[k][i]);
    }
    match end_tangents {
        Some((first, last)) => {
            tangents[0] = first;
            tangents[n - 1] = last;
        }
        None if n == 2 => {
            tangents[0] = slopes[0];
            tangents[1] = slopes[0];
        }
        None => {
            // End parabola: its end derivative is 2 * slope - inner derivative.
            tangents[0] = std::array::from_fn(|i| 2. * slopes[0][i] - tangents[1][i]);
            tangents[n - 1] =
                std::array::from_fn(|i| 2. * slopes[n - 2][i] - tangents[n - 2][i]);
        }
    }
    let mut control_points = vec![points[0].to_vec()];
    for k in 0..n - 1 {
        let third = steps[k] / 3.;
        control_points.push((0..3).map(|i| points[k][i] + third * tangents[k][i]).collect());
        control_points.push(
            (0..3)
                .map(|i| points[k + 1][i] - third * tangents[k + 1][i])
                .collect(),
        );
        control_points.push(points[k + 1].to_vec());
    }
    let mut knots = vec![0.; 4];
    for &site in &u[1..n - 1] {
        knots.extend([site; 3]);
    }
    knots.extend([1.; 4]);
    // Triple interior knots make each segment a separate Bezier piece; the
    // shared tangents make adjacent pieces meet with equal derivatives.
    let curve = Curve {
        degree: 3,
        knots,
        control_points,
        weights: vec![1.; 3 * n - 2],
        periodic: false,
    };
    curve.validate()?;
    Ok(curve)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sites() -> Vec<[f64; 3]> {
        vec![
            [0., 0., 0.],
            [1., 2., 0.5],
            [3., 2.5, 1.],
            [4., 0., 0.],
            [6., -1., 2.],
            [7., 1., 2.5],
        ]
    }

    fn close(a: &[f64], b: &[f64], tolerance: f64) {
        for (x, y) in a.iter().zip(b) {
            assert!((x - y).abs() <= tolerance, "{a:?} vs {b:?}");
        }
    }

    #[test]
    fn passes_through_every_site_at_its_parameter() {
        for param in [
            Parameterization::Uniform,
            Parameterization::ChordLength,
            Parameterization::Centripetal,
        ] {
            let points = sites();
            let curve = interpolate_curve_local(&points, param, None).unwrap();
            for (point, &u) in points.iter().zip(&parameters(&points, param)) {
                close(&curve.evaluate(u).unwrap().point, point, 1e-12);
            }
        }
    }

    #[test]
    fn is_c1_at_interior_sites() {
        let points = sites();
        let curve = interpolate_curve_local(&points, Parameterization::ChordLength, None).unwrap();
        for &u in &parameters(&points, Parameterization::ChordLength)[1..points.len() - 1] {
            let left = curve.derivatives(u - 1e-9, 1).unwrap();
            let right = curve.derivatives(u, 1).unwrap();
            close(&left[1], &right[1], 1e-6);
        }
    }

    #[test]
    fn reproduces_a_parabola_and_honours_end_tangents() {
        // Bessel tangents are exact for a quadratic sampled uniformly.
        let points: Vec<[f64; 3]> = (0..7)
            .map(|i| {
                let t = i as f64 / 6.;
                [t, t * t, 0.]
            })
            .collect();
        let curve = interpolate_curve_local(&points, Parameterization::Uniform, None).unwrap();
        for step in 0..=60 {
            let t = step as f64 / 60.;
            close(&curve.evaluate(t).unwrap().point, &[t, t * t, 0.], 1e-12);
        }
        let tangents = ([0., 3., 0.], [1., 0., 1.]);
        let pinned =
            interpolate_curve_local(&points, Parameterization::Uniform, Some(tangents)).unwrap();
        close(&pinned.derivatives(0., 1).unwrap()[1], &tangents.0, 1e-12);
        close(&pinned.derivatives(1., 1).unwrap()[1], &tangents.1, 1e-12);
    }

    #[test]
    fn moving_one_site_changes_only_nearby_segments() {
        let points = sites();
        let mut moved = points.clone();
        moved[1][1] += 3.;
        let param = Parameterization::Uniform;
        let a = interpolate_curve_local(&points, param, None).unwrap();
        let b = interpolate_curve_local(&moved, param, None).unwrap();
        let u = parameters(&points, param);
        // Site 1 influences segments 0..=2; segments 3 and 4 are untouched.
        for step in 0..=20 {
            let t = u[3] + (u[5] - u[3]) * step as f64 / 20.;
            close(&a.evaluate(t).unwrap().point, &b.evaluate(t).unwrap().point, 0.);
        }
    }

    #[test]
    fn invalid_sites_are_refused() {
        let p = Parameterization::ChordLength;
        assert!(interpolate_curve_local(&[[0.; 3]], p, None).is_err());
        assert!(interpolate_curve_local(&[[0.; 3], [0.; 3], [1.; 3]], p, None).is_err());
        assert!(interpolate_curve_local(&[[0.; 3], [f64::NAN, 0., 0.]], p, None).is_err());
        assert!(interpolate_curve_local(&vec![[0.; 3]; 87], p, None).is_err());
        let two = interpolate_curve_local(&[[0.; 3], [1., 2., 3.]], p, None).unwrap();
        close(&two.evaluate(0.5).unwrap().point, &[0.5, 1., 1.5], 1e-14);
    }
}
