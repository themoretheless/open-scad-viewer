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

/// Report of `optimize_knots_report` (item 215).
pub struct FitCurveReport {
    /// Best-seen curve (least sum-of-squared site residuals encountered).
    pub curve: Curve,
    /// Max-norm site residual after refitting controls at the initial knots.
    pub initial_residual_max: f64,
    /// Honest max-norm site residual of the returned curve, recomputed on the
    /// full data set at its chordal parameters.
    pub residual_max: f64,
    /// Gauss-Newton iterations actually performed (≤ the requested budget).
    pub iterations_run: usize,
    /// True when the loop stopped on residual stagnation rather than budget.
    pub converged: bool,
    /// Per-interior-knot displacement (returned knots minus initial knots).
    pub knot_deltas: Vec<f64>,
}

struct KnotFit {
    curve: Curve,
    /// Sum of squared site residuals at the (possibly subsampled) loop sites.
    sse: f64,
    /// Per-site residual vectors at the loop sites.
    residuals: Vec<[f64; 3]>,
}

/// Least-squares control-point refit for fixed knots at fixed parameters,
/// reusing the design-matrix + robust_solve machinery of this module.
fn refit_at_knots(
    degree: usize,
    knots: &[f64],
    control_count: usize,
    parameters: &[f64],
    points: &[[f64; 3]],
) -> Result<KnotFit> {
    let design = parameters
        .iter()
        .map(|&u| {
            crate::curve::basis(degree, knots, control_count, u, false)
                .map(|basis| basis.basis)
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let matrix = (0..control_count)
        .map(|i| {
            (0..control_count)
                .map(|j| design.iter().map(|row| row[i] * row[j]).sum())
                .collect()
        })
        .collect();
    let rhs = (0..control_count)
        .map(|i| {
            (0..3)
                .map(|axis| {
                    design
                        .iter()
                        .zip(points)
                        .map(|(row, p)| row[i] * p[axis])
                        .sum()
                })
                .collect()
        })
        .collect();
    let (controls, rank, _) = robust_solve(matrix, rhs)?;
    check(
        rank == control_count,
        "Knot optimization produced a rank-deficient fitting system",
    )?;
    let curve = Curve {
        degree,
        knots: knots.to_vec(),
        control_points: controls[..control_count].to_vec(),
        weights: vec![1.; control_count],
        periodic: false,
    };
    curve.validate()?;
    let mut sse = 0.;
    let mut residuals = Vec::with_capacity(points.len());
    for (&u, point) in parameters.iter().zip(points) {
        let value = curve.evaluate(u)?.point;
        let residual = [value[0] - point[0], value[1] - point[1], value[2] - point[2]];
        sse += residual.iter().map(|x| x * x).sum::<f64>();
        residuals.push(residual);
    }
    numeric(sse.is_finite(), "Knot-fit residual exhausted numeric range")?;
    Ok(KnotFit {
        curve,
        sse,
        residuals,
    })
}

/// Gauss–Newton optimization of interior knot positions for a clamped
/// non-periodic curve, minimizing the least-squares site residual at fixed
/// chordal data parameters. Each iteration re-solves the control points
/// (fixed-knot LSQ refit through the existing design-matrix/robust_solve
/// machinery), finite-differences the residual vector with respect to every
/// interior knot, and solves the damped normal equations augmented with
/// repulsion penalty rows for every knot gap below
/// `min_spacing · domain`. Steps are backtracked (1, ½, ¼, …) and projected
/// so the strict knot ordering with the minimum gap always holds; the loop
/// stops early on relative residual stagnation and always returns the
/// best-seen curve.
pub fn optimize_knots_report(
    points: &[[f64; 3]],
    initial: &Curve,
    iterations: usize,
    min_spacing: f64,
) -> Result<FitCurveReport> {
    initial.validate()?;
    check(
        points.len() >= 4 && points.len() <= 4096,
        "Knot optimization needs 4..4096 data sites",
    )?;
    for (i, point) in points.iter().enumerate() {
        for (axis, &x) in point.iter().enumerate() {
            require_finite_at(x, "knot_optimization_points", i * 3 + axis)?;
        }
    }
    check(
        !initial.periodic && initial.control_points[0].len() == 3,
        "Knot optimization needs a non-periodic 3D curve",
    )?;
    check(
        iterations <= 64,
        "Knot optimization iteration budget is at most 64",
    )?;
    check(
        min_spacing.is_finite() && (0. ..=0.25).contains(&min_spacing),
        "Minimum knot spacing must be a fraction of the domain in [0, 0.25]",
    )?;
    let p = initial.degree;
    let n = initial.control_points.len();
    check(
        initial.knots[0] == initial.knots[p] && initial.knots[n] == initial.knots[n + p],
        "Knot optimization requires clamped end knots",
    )?;
    check(
        points.len() >= n,
        "Knot optimization needs at least as many data sites as control points",
    )?;
    let [a, b] = initial.domain();
    let domain = b - a;
    // Interior knots must be simple and strictly inside the domain.
    let interior: Vec<f64> = initial.knots[p + 1..n].to_vec();
    let m = interior.len();
    check(
        m <= 24,
        "Knot optimization budget allows at most 24 interior knots",
    )?;
    check(
        interior.iter().all(|&k| k > a && k < b)
            && interior.array_windows().all(|[x, y]| x < y),
        "Interior knots must be strictly increasing inside the domain",
    )?;
    let gap_floor = min_spacing * domain;
    check(
        (m + 1) as f64 * gap_floor < domain * 0.5,
        "Minimum knot spacing leaves no feasible knot configuration",
    )?;
    // Fixed chordal data parameters mapped onto the curve domain.
    let mut parameters = vec![0.];
    for pair in points.windows(2) {
        parameters.push(
            parameters.last().unwrap() + distance(&pair[0].to_vec(), &pair[1].to_vec()),
        );
    }
    let total = *parameters.last().unwrap();
    let mut parameters: Vec<f64> = if total == 0. {
        (0..points.len())
            .map(|i| i as f64 / (points.len() - 1) as f64)
            .collect()
    } else {
        parameters.into_iter().map(|value| value / total).collect()
    };
    for parameter in &mut parameters {
        *parameter = a + domain * *parameter;
    }
    // Subsample the loop sites when the cloud is large; the final residual
    // is always recomputed on the full set.
    let stride = (points.len() / 1024).max(1);
    let loop_index: Vec<usize> = (0..points.len()).step_by(stride).collect();
    let loop_params: Vec<f64> = loop_index.iter().map(|&i| parameters[i]).collect();
    let loop_points: Vec<[f64; 3]> = loop_index.iter().map(|&i| points[i]).collect();

    let mut knots = initial.knots.clone();
    let mut best = refit_at_knots(p, &knots, n, &loop_params, &loop_points)?;
    let initial_residual_max = {
        let mut full = refit_at_knots(p, &knots, n, &parameters, points)?;
        full.residuals
            .drain(..)
            .map(|r| r.iter().map(|x| x * x).sum::<f64>().sqrt())
            .fold(0., f64::max)
    };
    let penalty_weight = 1e3 * (1. + best.sse.sqrt());
    let mut iterations_run = 0;
    let mut stagnant = 0_usize;
    let mut converged = false;
    // Unified budget guard on the requested Gauss–Newton iteration count
    // (already capped at 64 above); the guard turns exhaustion into a typed
    // BudgetExhausted payload if the loop is ever restructured.
    let mut gauss_newton = Budget::with_iterations(iterations.max(1))?
        .guard("knot-optimization-gauss-newton");
    for _ in 0..iterations {
        gauss_newton.tick()?;
        iterations_run += 1;
        // Finite-difference Jacobian columns: one refit per interior knot.
        let mut columns: Vec<Vec<[f64; 3]>> = Vec::with_capacity(m);
        for j in 0..m {
            let kidx = p + 1 + j;
            let h = 1e-6 * (knots[kidx + 1] - knots[kidx - 1]).max(1e-9 * domain);
            let mut perturbed = knots.clone();
            perturbed[kidx] = (knots[kidx] + h).min(knots[kidx + 1] - 1e-9 * domain.max(1.));
            if !(perturbed[kidx] > knots[kidx]) {
                columns.push(vec![[0.; 3]; best.residuals.len()]);
                continue;
            }
            let step = perturbed[kidx] - knots[kidx];
            let fit = refit_at_knots(p, &perturbed, n, &loop_params, &loop_points)?;
            columns.push(
                fit.residuals
                    .iter()
                    .zip(&best.residuals)
                    .map(|(r1, r0)| {
                        [
                            (r1[0] - r0[0]) / step,
                            (r1[1] - r0[1]) / step,
                            (r1[2] - r0[2]) / step,
                        ]
                    })
                    .collect(),
            );
        }
        let dot = |x: &[[f64; 3]], y: &[[f64; 3]]| -> f64 {
            x.iter()
                .zip(y)
                .map(|(a, b)| a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .sum()
        };
        let mut jtj = vec![vec![0.; m]; m];
        let mut jtr = vec![0.; m];
        for i in 0..m {
            for j in 0..=i {
                let value = dot(&columns[i], &columns[j]);
                jtj[i][j] = value;
                jtj[j][i] = value;
            }
            jtr[i] = -dot(&columns[i], &best.residuals);
        }
        // Repulsion penalty rows for gaps below the floor: linearized
        // residual w·(g0 − gap) with ∂gap/∂knot = ±1 on the pair.
        let sequence: Vec<f64> = std::iter::once(a)
            .chain(knots[p + 1..n].iter().copied())
            .chain(std::iter::once(b))
            .collect();
        for q in 0..sequence.len() - 1 {
            let gap = sequence[q + 1] - sequence[q];
            if gap < gap_floor {
                let row = |jtj: &mut Vec<Vec<f64>>, jtr: &mut Vec<f64>| {
                    let lhs = q.checked_sub(1);
                    let rhs = if q < m { Some(q) } else { None };
                    if let Some(l) = lhs {
                        jtj[l][l] += penalty_weight * penalty_weight;
                        jtr[l] += penalty_weight * penalty_weight * (gap_floor - gap);
                    }
                    if let Some(r) = rhs {
                        jtj[r][r] += penalty_weight * penalty_weight;
                        jtr[r] -= penalty_weight * penalty_weight * (gap_floor - gap);
                    }
                    if let (Some(l), Some(r)) = (lhs, rhs) {
                        jtj[l][r] -= penalty_weight * penalty_weight;
                        jtj[r][l] -= penalty_weight * penalty_weight;
                    }
                };
                row(&mut jtj, &mut jtr);
            }
        }
        // Light Levenberg damping for rank safety.
        for j in 0..m {
            jtj[j][j] += 1e-12 * (1. + jtj[j][j]);
        }
        let rhs: Vec<Vec<f64>> = jtr.iter().map(|&v| vec![v]).collect();
        let Ok((delta, _, _)) = robust_solve(jtj, rhs) else {
            break;
        };
        let delta: Vec<f64> = delta[..m].iter().map(|row| row[0]).collect();
        if delta.iter().all(|d| !d.is_finite()) {
            break;
        }
        // Backtracking line search with feasibility projection.
        let mut accepted = false;
        for scale in [1., 0.5, 0.25, 0.1, 0.02] {
            let mut candidate_knots = knots.clone();
            for j in 0..m {
                candidate_knots[p + 1 + j] += scale * delta[j];
            }
            // Project to strict ordering with the minimum gap.
            let floor = gap_floor.max(1e-9 * domain);
            let mut feasible = true;
            for index in p + 1..n {
                let lower = candidate_knots[index - 1] + floor;
                if candidate_knots[index] < lower {
                    candidate_knots[index] = lower;
                }
                if candidate_knots[index] >= candidate_knots[index + 1] {
                    feasible = false;
                    break;
                }
            }
            if !feasible {
                continue;
            }
            let Ok(fit) = refit_at_knots(p, &candidate_knots, n, &loop_params, &loop_points)
            else {
                continue;
            };
            if fit.sse < best.sse {
                let improvement = (best.sse - fit.sse) / best.sse.max(1e-300);
                best = fit;
                knots = candidate_knots;
                accepted = true;
                stagnant = if improvement < 1e-10 { stagnant + 1 } else { 0 };
                break;
            }
        }
        if !accepted {
            stagnant += 1;
        }
        if stagnant >= 2 {
            converged = true;
            break;
        }
    }
    // Honest final residual on the full data set.
    let mut full = refit_at_knots(p, &best.curve.knots, n, &parameters, points)?;
    let residual_max = full
        .residuals
        .drain(..)
        .map(|r| r.iter().map(|x| x * x).sum::<f64>().sqrt())
        .fold(0., f64::max);
    let knot_deltas = best.curve.knots[p + 1..n]
        .iter()
        .zip(&interior)
        .map(|(new, old)| new - old)
        .collect();
    Ok(FitCurveReport {
        curve: full.curve,
        initial_residual_max: next_up(initial_residual_max),
        residual_max: next_up(residual_max),
        iterations_run,
        converged,
        knot_deltas,
    })
}

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
