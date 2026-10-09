//! Native point fitting, interpolation and cloud error reports.
use super::{
    Curve, Result, Surface, ToleranceContext, binomial, box_of, check, context, distance, interval,
    next_up, numeric, solve,
};
use crate::foundation::guards::{Budget, require_finite_at};
#[cfg(feature = "codec")]
pub mod serialization;

pub struct CurveFitResult<C> {
    pub curve: Curve,
    pub certificate: C,
}
pub struct SurfaceFitResult<C> {
    pub surface: Surface,
    pub certificate: C,
}
pub struct CurveFitCertificate {
    pub data_site_error_upper: f64,
    pub site_count: usize,
    pub control_count: usize,
    pub degree: usize,
    pub tolerance: ToleranceContext,
}
pub struct GridFitCertificate {
    pub fitting: bool,
    pub rank: usize,
    pub tolerance: ToleranceContext,
}
pub struct CloudFitEvidence {
    pub data_site_error_upper: f64,
    pub hausdorff_error_upper: f64,
    pub site_count: usize,
    pub rank: usize,
    pub pivot_lower: f64,
    pub tolerance: ToleranceContext,
}
pub struct CloudCurveCertificate {
    pub control_count: usize,
    pub requested_controls: usize,
    pub evidence: CloudFitEvidence,
}
pub struct CloudSurfaceCertificate {
    pub controls_u: usize,
    pub controls_v: usize,
    pub evidence: CloudFitEvidence,
}

pub fn fit_curve_points_report(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveFitResult<CurveFitCertificate>> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Curve fitting needs 2..4096 data sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3) && points.iter().all(|p| p.len() == dimension),
        "Curve fitting data must be 2D or 3D points of one dimension",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "fit_curve_points", i * dimension + axis)?;
        }
    }
    check(
        (2..=26).contains(&control_count) && control_count <= points.len(),
        "Fit control count must be 2..26 and no larger than site count",
    )?;
    let degree = control_count - 1;
    let parameters = chord_parameters(&points);
    let design = parameters
        .iter()
        .map(|&t| {
            (0..control_count)
                .map(|i| {
                    binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32)
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let matrix = (0..control_count)
        .map(|i| {
            (0..control_count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..control_count)
        .map(|i| {
            (0..dimension)
                .map(|axis| {
                    design
                        .iter()
                        .zip(&points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let controls = solve(matrix, rhs)?;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: controls,
        weights: vec![1.; control_count],
        periodic: false,
    };
    curve.validate()?;
    let residual = parameters
        .iter()
        .zip(&points)
        .map(|(&u, point)| distance(&curve.evaluate(u).unwrap().point, point))
        .fold(0., f64::max);
    let tolerance = context(tolerance);
    Ok(CurveFitResult {
        curve,
        certificate: CurveFitCertificate {
            data_site_error_upper: next_up(residual),
            site_count: points.len(),
            control_count,
            degree,
            tolerance,
        },
    })
}

pub fn interpolate_surface_grid_report(
    points: Vec<Vec<[f64; 3]>>,
    fitting: bool,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceFitResult<GridFitCertificate>> {
    check(
        points.len() >= 2 && points.len() <= 32 && points[0].len() >= 2 && points[0].len() <= 32,
        "Surface interpolation grid must be 2..32 by 2..32",
    )?;
    let nv = points[0].len();
    check(
        points
            .iter()
            .all(|row| row.len() == nv && row.iter().flatten().all(|x| x.is_finite())),
        "Surface interpolation grid must be rectangular and finite",
    )?;
    let axis_knots = |count: usize| {
        std::iter::repeat_n(0., 2)
            .chain((1..count - 1).map(|i| i as f64 / (count - 1) as f64))
            .chain(std::iter::repeat_n(1., 2))
            .collect::<Vec<_>>()
    };
    let control_points = points
        .iter()
        .map(|row| row.iter().map(|point| point.to_vec()).collect())
        .collect::<Vec<Vec<Vec<f64>>>>();
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: axis_knots(points.len()),
        knots_v: axis_knots(nv),
        weights: vec![vec![1.; nv]; points.len()],
        control_points,
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let tolerance = context(tolerance);
    Ok(SurfaceFitResult {
        surface,
        certificate: GridFitCertificate {
            fitting,
            rank: points.len() * nv,
            tolerance,
        },
    })
}

pub fn fit_curve_cloud_certified_report(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveFitResult<CloudCurveCertificate>> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Cloud curve fitting needs 2..4096 sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3) && points.iter().all(|p| p.len() == dimension),
        "Cloud points must be 2D or 3D of one dimension",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "cloud_points", i * dimension + axis)?;
        }
    }
    check(
        (2..=26).contains(&control_count),
        "Cloud fit control count must be 2..26",
    )?;
    let mut working = control_count.min(points.len());
    let parameters = chord_parameters(&points);
    let mut last_error = None;
    // Rank-descent budget: `working` strictly decreases from at most 26.
    let mut descent = Budget::with_iterations(working + 1)?.guard("cloud-fit-rank-descent");
    let (controls, rank, pivot_min) = loop {
        descent.tick()?;
        let degree = working - 1;
        let design = parameters
            .iter()
            .map(|&t| {
                (0..working)
                    .map(|i| {
                        binomial(degree, i) * t.powi(i as i32) * (1. - t).powi((degree - i) as i32)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let matrix = (0..working)
            .map(|i| {
                (0..working)
                    .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                    .collect()
            })
            .collect();
        let rhs = (0..working)
            .map(|i| {
                (0..dimension)
                    .map(|axis| {
                        design
                            .iter()
                            .zip(&points)
                            .map(|(row, p)| row[i] * p[axis])
                            .sum()
                    })
                    .collect()
            })
            .collect();
        match robust_solve(matrix, rhs) {
            Ok(result) => break result,
            Err(error) => {
                last_error = Some(error);
                if working <= 2 {
                    break (Vec::new(), 0, 0.);
                }
                working -= 1;
            }
        }
    };
    if rank == 0 {
        return Err(last_error.unwrap_or_else(|| crate::input("Cloud fit failed")));
    }
    let degree = working - 1;
    let curve = Curve {
        degree,
        knots: std::iter::repeat_n(0., degree + 1)
            .chain(std::iter::repeat_n(1., degree + 1))
            .collect(),
        control_points: controls[..working].to_vec(),
        weights: vec![1.; working],
        periodic: false,
    };
    curve.validate()?;
    let site_error = parameters
        .iter()
        .zip(&points)
        .map(|(&u, point)| distance(&curve.evaluate(u).unwrap().point, point))
        .fold(0., f64::max);
    // Continuum remainder under admitted assumption: chordal parameters form a δ-net in the fitted domain
    // with δ = 1/(N-1); residual Lipschitz ≤ speed_upper of the fitted curve.
    let mut speed = 0_f64;
    for segment in curve.decompose()? {
        let c = segment.definition();
        for pair in c.control_points.windows(2) {
            speed = speed.max(distance(&pair[0], &pair[1]) * c.degree as f64);
        }
    }
    let delta = 1. / (points.len() - 1).max(1) as f64;
    let hausdorff = next_up(site_error + next_up(speed) * delta);
    let tolerance = context(tolerance);
    Ok(CurveFitResult {
        curve,
        certificate: CloudCurveCertificate {
            control_count: working,
            requested_controls: control_count,
            evidence: CloudFitEvidence {
                data_site_error_upper: next_up(site_error),
                hausdorff_error_upper: hausdorff,
                site_count: points.len(),
                rank,
                pivot_lower: pivot_min,
                tolerance,
            },
        },
    })
}

pub fn fit_surface_cloud_certified_report(
    points: Vec<[f64; 3]>,
    controls_u: usize,
    controls_v: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceFitResult<CloudSurfaceCertificate>> {
    check(
        points.len() >= 4 && points.len() <= 4096,
        "Cloud surface fitting needs 4..4096 sites",
    )?;
    check(
        (2..=8).contains(&controls_u) && (2..=8).contains(&controls_v),
        "Cloud surface controls per axis must be 2..8",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "cloud_surface_points", i * 3 + axis)?;
        }
    }
    // PCA plane parameters as admitted unstructured domain.
    let centroid =
        [0, 1, 2].map(|axis| points.iter().map(|p| p[axis]).sum::<f64>() / points.len() as f64);
    let mut cov = [[0.; 3]; 3];
    for point in &points {
        let d = [
            point[0] - centroid[0],
            point[1] - centroid[1],
            point[2] - centroid[2],
        ];
        for i in 0..3 {
            for j in 0..3 {
                cov[i][j] += d[i] * d[j];
            }
        }
    }
    // Power iteration for two dominant planar axes.
    let mut power_iter = Budget::with_iterations(24 * 2 + 1)?.guard("cloud-surface-power-iteration");
    let mut axes = [[1., 0., 0.], [0., 1., 0.]];
    for axis in &mut axes {
        for _ in 0..24 {
            power_iter.tick()?;
            let mut next = [0.; 3];
            for i in 0..3 {
                next[i] = cov[i][0] * axis[0] + cov[i][1] * axis[1] + cov[i][2] * axis[2];
            }
            let norm = next
                .iter()
                .map(|x| x * x)
                .sum::<f64>()
                .sqrt()
                .max(f64::from_bits(1));
            *axis = next.map(|x| x / norm);
        }
    }
    let mut uv: Vec<[f64; 2]> = points
        .iter()
        .map(|point| {
            let d = [
                point[0] - centroid[0],
                point[1] - centroid[1],
                point[2] - centroid[2],
            ];
            [
                d[0] * axes[0][0] + d[1] * axes[0][1] + d[2] * axes[0][2],
                d[0] * axes[1][0] + d[1] * axes[1][1] + d[2] * axes[1][2],
            ]
        })
        .collect();
    let u_bounds = interval(uv.iter().map(|p| p[0]));
    let v_bounds = interval(uv.iter().map(|p| p[1]));
    for parameter in &mut uv {
        parameter[0] =
            (parameter[0] - u_bounds[0]) / (u_bounds[1] - u_bounds[0]).max(f64::from_bits(1));
        parameter[1] =
            (parameter[1] - v_bounds[0]) / (v_bounds[1] - v_bounds[0]).max(f64::from_bits(1));
    }
    let degree_u = controls_u - 1;
    let degree_v = controls_v - 1;
    let count = controls_u * controls_v;
    let design = uv
        .iter()
        .map(|parameter| {
            (0..controls_u)
                .flat_map(|i| {
                    (0..controls_v).map(move |j| {
                        binomial(degree_u, i)
                            * parameter[0].powi(i as i32)
                            * (1. - parameter[0]).powi((degree_u - i) as i32)
                            * binomial(degree_v, j)
                            * parameter[1].powi(j as i32)
                            * (1. - parameter[1]).powi((degree_v - j) as i32)
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let matrix = (0..count)
        .map(|i| {
            (0..count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..count)
        .map(|i| {
            (0..3)
                .map(|axis| {
                    design
                        .iter()
                        .zip(&points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let (flat, rank, pivot_min) = robust_solve(matrix, rhs)?;
    let control_points = (0..controls_u)
        .map(|i| {
            (0..controls_v)
                .map(|j| flat[i * controls_v + j].clone())
                .collect()
        })
        .collect();
    let surface = Surface {
        degree_u,
        degree_v,
        knots_u: std::iter::repeat_n(0., degree_u + 1)
            .chain(std::iter::repeat_n(1., degree_u + 1))
            .collect(),
        knots_v: std::iter::repeat_n(0., degree_v + 1)
            .chain(std::iter::repeat_n(1., degree_v + 1))
            .collect(),
        control_points,
        weights: vec![vec![1.; controls_v]; controls_u],
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let site_error = uv
        .iter()
        .zip(&points)
        .map(|(parameter, point)| {
            distance(
                &surface
                    .evaluate_validated(parameter[0], parameter[1])
                    .unwrap()
                    .point,
                point,
            )
        })
        .fold(0., f64::max);
    let controls: Vec<Vec<f64>> = surface.control_points.iter().flatten().cloned().collect();
    let (min, max) = box_of(&controls);
    let diameter = next_up(
        min.iter()
            .zip(max)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f64>()
            .sqrt(),
    );
    let delta = 1. / ((points.len() as f64).sqrt().max(1.));
    let hausdorff = next_up(site_error + diameter * delta);
    let tolerance = context(tolerance);
    Ok(SurfaceFitResult {
        surface,
        certificate: CloudSurfaceCertificate {
            controls_u,
            controls_v,
            evidence: CloudFitEvidence {
                data_site_error_upper: next_up(site_error),
                hausdorff_error_upper: hausdorff,
                site_count: points.len(),
                rank,
                pivot_lower: pivot_min,
                tolerance,
            },
        },
    })
}

fn chord_parameters(points: &[Vec<f64>]) -> Vec<f64> {
    let mut parameters = vec![0.];
    for pair in points.windows(2) {
        parameters.push(parameters.last().unwrap() + distance(&pair[0], &pair[1]));
    }
    let total = *parameters.last().unwrap();
    if total == 0. {
        (0..points.len())
            .map(|i| i as f64 / (points.len() - 1) as f64)
            .collect()
    } else {
        parameters.into_iter().map(|value| value / total).collect()
    }
}

fn robust_solve(
    mut matrix: Vec<Vec<f64>>,
    mut values: Vec<Vec<f64>>,
) -> Result<(Vec<Vec<f64>>, usize, f64)> {
    let n = matrix.len();
    let mut rank = 0_usize;
    let mut pivot_min = f64::INFINITY;
    let mut column_perm: Vec<usize> = (0..n).collect();
    for column in 0..n {
        let mut pivot = None;
        for (row, row_values) in matrix.iter().enumerate().skip(rank) {
            for (candidate, value) in row_values.iter().enumerate().skip(column) {
                let magnitude = value.abs();
                if pivot.map(|(_, _, m)| magnitude > m).unwrap_or(true) {
                    pivot = Some((row, candidate, magnitude));
                }
            }
        }
        let Some((row, candidate, magnitude)) = pivot else {
            break;
        };
        if magnitude <= 1e-12 {
            break;
        }
        pivot_min = pivot_min.min(magnitude);
        matrix.swap(rank, row);
        values.swap(rank, row);
        for row in &mut matrix {
            row.swap(column, candidate);
        }
        column_perm.swap(column, candidate);
        let divisor = matrix[rank][column];
        for value in &mut matrix[rank][column..] {
            *value /= divisor;
        }
        for value in &mut values[rank] {
            *value /= divisor;
        }
        for r in 0..n {
            if r == rank {
                continue;
            }
            let factor = matrix[r][column];
            for j in column..n {
                matrix[r][j] -= factor * matrix[rank][j];
            }
            let pivot_values = values[rank].clone();
            for (value, pivot_value) in values[r].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
        rank += 1;
    }
    numeric(rank >= 1, "Fitting system is rank deficient below one")?;
    // Undo column permutation on the coefficient vector.
    let width = values[0].len();
    let mut ordered = vec![vec![0.; width]; n];
    for (logical, physical) in column_perm.iter().enumerate().take(rank) {
        ordered[*physical] = values[logical].clone();
    }
    Ok((ordered, rank, pivot_min))
}

/// Dense square solver re-exported for crate-internal consumers outside the
/// foundation module tree (e.g. `fairing`). Wraps `super::solve`.
pub(crate) fn dense_solve(
    matrix: Vec<Vec<f64>>,
    values: Vec<Vec<f64>>,
) -> Result<Vec<Vec<f64>>> {
    super::solve(matrix, values)
}

mod knots;
pub use knots::{FitCurveReport, optimize_knots_report};

#[cfg(test)]
mod optimize_knots_tests {
    use super::*;

    fn known_curve() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.3, 0.7, 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.1, 0.4, 0.1],
                vec![0.4, 0.6, -0.2],
                vec![0.6, 0.2, 0.3],
                vec![0.8, 0.5, -0.1],
                vec![1., 0.3, 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        }
    }

    fn misplaced_initial() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 0.42, 0.62, 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0.1, 0.4, 0.1],
                vec![0.4, 0.6, -0.2],
                vec![0.6, 0.2, 0.3],
                vec![0.8, 0.5, -0.1],
                vec![1., 0.3, 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        }
    }

    fn sampled_points(curve: &Curve, count: usize) -> Vec<[f64; 3]> {
        let [a, b] = curve.domain();
        (0..count)
            .map(|i| {
                let u = a + (b - a) * i as f64 / (count - 1) as f64;
                let point = curve.evaluate(u).unwrap().point;
                [point[0], point[1], point[2]]
            })
            .collect()
    }

    #[test]
    fn optimize_knots_reduces_residual_and_keeps_spacing() {
        let known = known_curve();
        let points = sampled_points(&known, 48);
        let initial = misplaced_initial();
        let report = optimize_knots_report(&points, &initial, 16, 0.05).unwrap();
        assert!(report.iterations_run <= 16);
        assert!(
            report.residual_max < report.initial_residual_max,
            "residual should drop: initial={} final={}",
            report.initial_residual_max,
            report.residual_max
        );
        // The misplaced knots were at 0.42/0.62, truth at 0.3/0.7; with
        // fixed chordal data parameters the achievable reduction is bounded
        // by parameterization mismatch, but must be substantial.
        assert!(report.residual_max < 0.8 * report.initial_residual_max);
        // Knots remain valid and respect the spacing floor.
        report.curve.validate().unwrap();
        let p = report.curve.degree;
        let n = report.curve.control_points.len();
        for index in p..n {
            let gap = report.curve.knots[index + 1] - report.curve.knots[index];
            if report.curve.knots[index + 1] < 1. && report.curve.knots[index] > 0. {
                assert!(
                    gap >= 0.05 - 1e-9,
                    "interior gap {gap} violates the spacing floor"
                );
            }
            assert!(gap > 0., "knots collapsed at index {index}");
        }
        assert_eq!(report.knot_deltas.len(), 2);
        assert!(report.knot_deltas.iter().all(|d| d.is_finite()));
    }

    #[test]
    fn optimize_knots_respects_iteration_cap_and_zero_iterations() {
        let known = known_curve();
        let points = sampled_points(&known, 32);
        let initial = misplaced_initial();
        let zero = optimize_knots_report(&points, &initial, 0, 0.05).unwrap();
        assert_eq!(zero.iterations_run, 0);
        assert_eq!(zero.residual_max, zero.initial_residual_max);
        assert!(zero.knot_deltas.iter().all(|&d| d == 0.));
        let capped = optimize_knots_report(&points, &initial, 2, 0.05).unwrap();
        assert!(capped.iterations_run <= 2);
        assert!(optimize_knots_report(&points, &initial, 65, 0.05).is_err());
        assert!(optimize_knots_report(&points, &initial, 4, 0.9).is_err());
    }

    #[test]
    fn fit_curve_points_rejects_non_finite_site_with_typed_payload() {
        let points = vec![vec![0., 0.], vec![1., f64::NAN], vec![2., 0.]];
        let err = fit_curve_points_report(points, 3, None).err().expect("invalid non-finite input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("fit_curve_points[3]"), "{err}");
        let points = vec![vec![0., f64::INFINITY], vec![1., 1.], vec![2., 0.]];
        assert!(fit_curve_points_report(points, 3, None).is_err());
    }

    #[test]
    fn cloud_fits_reject_non_finite_sites_with_typed_payload() {
        let mut curve_points: Vec<Vec<f64>> =
            (0..8).map(|i| vec![i as f64, (i as f64).sin()]).collect();
        curve_points[4][1] = f64::NAN;
        let err = fit_curve_cloud_certified_report(curve_points, 4, None).err().expect("invalid non-finite input must fail");
        assert!(err.contains("cloud_points[9]"), "{err}");
        let mut surface_points: Vec<[f64; 3]> = (0..8)
            .map(|i| [i as f64, (i as f64).cos(), 0.5 * i as f64])
            .collect();
        surface_points[2][2] = f64::NEG_INFINITY;
        let err = fit_surface_cloud_certified_report(surface_points, 2, 2, None).err().expect("invalid non-finite input must fail");
        assert!(err.contains("cloud_surface_points[8]"), "{err}");
    }

    #[test]
    fn optimize_knots_rejects_non_finite_site_with_typed_payload() {
        let initial = misplaced_initial();
        let mut points = sampled_points(&initial, 16);
        points[3][0] = f64::NAN;
        let err = optimize_knots_report(&points, &initial, 4, 0.05).err().expect("invalid non-finite input must fail");
        assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
        assert!(err.contains("knot_optimization_points[9]"), "{err}");
    }
}
