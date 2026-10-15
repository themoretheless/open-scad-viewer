//! CST profile parameterization and bounded inverse fitting.
use super::*;

/// CST (class function / shape function) profile parameterization.
/// y(x) = x^n1 * (1-x)^n2 * sum_i a_i B_i^k(x), B the Bernstein basis of
/// order k = coefficients.len() - 1.
#[derive(Clone, Debug)]
pub struct CstProfile {
    pub n1: f64,
    pub n2: f64,
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
}

/// CST shape function value at chord fraction x.
pub fn cst_shape(x: f64, n1: f64, n2: f64, coefficients: &[f64]) -> f64 {
    if x <= 0. || x >= 1. {
        return 0.;
    }
    let class = x.powf(n1) * (1. - x).powf(n2);
    let order = coefficients.len() - 1;
    let mut shape = 0.;
    for (i, &a) in coefficients.iter().enumerate() {
        shape += a * bernstein(order, i, x);
    }
    class * shape
}

fn bernstein(order: usize, i: usize, x: f64) -> f64 {
    binomial(order, i) * x.powi(i as i32) * (1. - x).powi((order - i) as i32)
}

fn binomial(n: usize, k: usize) -> f64 {
    if k > n {
        return 0.;
    }
    let mut value = 1.;
    for j in 0..k.min(n - k) {
        value = value * (n - j) as f64 / (j + 1) as f64;
    }
    value
}

/// Ordered point loop of a CST profile (upper then lower, nose clustered).
pub fn cst_points(profile: &CstProfile, chord: f64, count: usize) -> Result<Vec<[f64; 3]>> {
    check(
        profile.n1.is_finite() && profile.n2.is_finite() && profile.n1 > 0. && profile.n2 > 0.,
        "CST class exponents must be positive and finite",
    )?;
    check(
        (2..=12).contains(&profile.upper.len()) && profile.upper.len() == profile.lower.len(),
        "CST coefficient orders must match in 2..=12 per surface",
    )?;
    require_finite_slice(&profile.upper, "cst_upper")?;
    require_finite_slice(&profile.lower, "cst_lower")?;
    check(
        profile
            .upper
            .iter()
            .chain(&profile.lower)
            .all(|x| x.abs() <= 1.),
        "CST coefficients must satisfy |a| <= 1",
    )?;
    check(
        (8..=2048).contains(&count) && chord.is_finite() && chord > 0.,
        "CST sampling needs 8..2048 stations and a positive chord",
    )?;
    let mut points = Vec::with_capacity(2 * count + 1);
    let station = |i: usize| {
        let s = i as f64 / count as f64;
        s * s
    };
    for i in (0..=count).rev() {
        let x = station(i);
        points.push([chord * x, chord * cst_shape(x, profile.n1, profile.n2, &profile.upper), 0.]);
    }
    for i in 1..=count {
        let x = station(i);
        points.push([chord * x, chord * cst_shape(x, profile.n1, profile.n2, &profile.lower), 0.]);
    }
    Ok(points)
}

/// Result of the CST inverse problem.
#[derive(Clone, Debug)]
pub struct CstFitResult {
    pub profile: CstProfile,
    /// Max |y_data - y_fit| over data sites (vertical residual).
    pub max_residual: f64,
    /// Max distance from data sites to a dense CST sampling (Hausdorff-like).
    pub hausdorff_residual: f64,
    /// Tikhonov factor used in (A^T A + lambda I).
    pub lambda: f64,
}

/// Fit CST coefficients to a profile point cloud by least squares with
/// Tikhonov regularization (lambda * I). Points are split into upper/lower
/// by the sign of y relative to the chord line through the extreme-x data
/// points; the chord parameter is the normalized x coordinate.
pub fn cst_fit(
    points: &[[f64; 3]],
    order: usize,
    n1: f64,
    n2: f64,
    lambda: f64,
) -> Result<CstFitResult> {
    check(
        (16..=4096).contains(&points.len()),
        "CST fitting needs 16..4096 data sites",
    )?;
    check(
        (1..=11).contains(&order),
        "CST fit order must be 1..=11",
    )?;
    require_finite_slice(
        &points.iter().flatten().copied().collect::<Vec<_>>(),
        "cst_fit_data",
    )?;
    check(
        n1.is_finite() && n2.is_finite() && n1 > 0. && n2 > 0.,
        "CST class exponents must be positive and finite",
    )?;
    check(
        lambda.is_finite() && (0. ..=1.).contains(&lambda),
        "Tikhonov lambda must be in [0, 1]",
    )?;
    let mut x_min = f64::INFINITY;
    let mut x_max = f64::NEG_INFINITY;
    for p in points {
        x_min = x_min.min(p[0]);
        x_max = x_max.max(p[0]);
    }
    let chord = x_max - x_min;
    check(chord > 0., "CST data degenerate in the chord direction")?;
    let columns = order + 1;
    let row = |x: f64| -> Vec<f64> {
        let xi = ((x - x_min) / chord).clamp(1e-9, 1. - 1e-9);
        let class = xi.powf(n1) * (1. - xi).powf(n2);
        (0..columns).map(|i| class * bernstein(order, i, xi)).collect()
    };
    let fit_side = |upper: bool| -> Result<(Vec<f64>, f64, Vec<[f64; 2]>)> {
        let mut design = Vec::new();
        let mut target = Vec::new();
        let mut sites = Vec::new();
        for p in points {
            if (p[1] >= 0.) == upper {
                design.push(row(p[0]));
                target.push(vec![p[1]]);
                sites.push([p[0], p[1]]);
            }
        }
        check(
            sites.len() > columns,
            "CST fit needs more data sites per surface than coefficients",
        )?;
        let solved = lsq_tikhonov(&design, &target, lambda)?;
        let coefficients: Vec<f64> = solved.iter().map(|r| r[0]).collect();
        let mut max_residual = 0_f64;
        for (r, site) in design.iter().zip(&sites) {
            let y: f64 = r.iter().zip(&coefficients).map(|(b, a)| b * a).sum();
            max_residual = max_residual.max((y - site[1]).abs());
        }
        Ok((coefficients, max_residual, sites))
    };
    let (upper, res_upper, sites_upper) = fit_side(true)?;
    let (lower, res_lower, sites_lower) = fit_side(false)?;
    let profile = CstProfile {
        n1,
        n2,
        upper,
        lower,
    };
    // Hausdorff-like residual: distance from data sites to a dense sampling
    // of the fitted CST curves.
    let dense = |upper_side: bool| -> Vec<[f64; 2]> {
        (0..=512)
            .map(|i| {
                let s = i as f64 / 512.;
                let xi = s * s;
                let y = cst_shape(
                    xi.clamp(1e-9, 1. - 1e-9),
                    n1,
                    n2,
                    if upper_side {
                        &profile.upper
                    } else {
                        &profile.lower
                    },
                );
                [x_min + chord * xi, y]
            })
            .collect()
    };
    let dense_upper = dense(true);
    let dense_lower = dense(false);
    let hausdorff = |sites: &[[f64; 2]], dense: &[[f64; 2]]| -> f64 {
        sites
            .iter()
            .map(|s| {
                dense
                    .iter()
                    .map(|d| (d[0] - s[0]).hypot(d[1] - s[1]))
                    .fold(f64::INFINITY, f64::min)
            })
            .fold(0., f64::max)
    };
    let hausdorff_residual = hausdorff(&sites_upper, &dense_upper)
        .max(hausdorff(&sites_lower, &dense_lower));
    Ok(CstFitResult {
        profile,
        max_residual: next_up(res_upper.max(res_lower)),
        hausdorff_residual: next_up(hausdorff_residual),
        lambda,
    })
}

