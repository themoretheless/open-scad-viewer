//! airfoil algorithm: cst_fit.
use super::*;
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
