use super::*;

pub(super) fn robust_solve(
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

pub fn fit_curve_cloud_certified(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Cloud curve fitting needs 2..4096 sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3)
            && points
                .iter()
                .all(|p| p.len() == dimension && p.iter().all(|x| x.is_finite())),
        "Cloud points must be finite 2D or 3D",
    )?;
    check(
        (2..=26).contains(&control_count),
        "Cloud fit control count must be 2..26",
    )?;
    let mut working = control_count.min(points.len());
    let parameters = chord_parameters(&points);
    let mut last_error = None;
    let (controls, rank, pivot_min) = loop {
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
    Ok(
        json!({"curve":curve,"certificate":{"version":"nurbs-foundation/4","classification":"approximate_cloud_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(site_error),
        "hausdorffErrorUpper":hausdorff,
        "admittedAssumptions":["chordal-parameter-delta-net-of-fitted-domain","residual-Lipschitz-from-control-polygon-speed"],
        "siteCount":points.len(),"controlCount":working,"requestedControls":control_count,
        "conditioning":{"method":"robust-pivoted-normal-equations","rank":rank,"pivotLower":pivot_min},
        "resource":{"maxSites":4096,"maxControls":26},"evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn fit_surface_cloud_certified(
    points: Vec<[f64; 3]>,
    controls_u: usize,
    controls_v: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 4 && points.len() <= 4096,
        "Cloud surface fitting needs 4..4096 sites",
    )?;
    check(
        (2..=8).contains(&controls_u) && (2..=8).contains(&controls_v),
        "Cloud surface controls per axis must be 2..8",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Cloud surface points must be finite",
    )?;
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
    let mut axes = [[1., 0., 0.], [0., 1., 0.]];
    for axis in &mut axes {
        for _ in 0..24 {
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
    Ok(
        json!({"surface":surface,"certificate":{"version":"nurbs-foundation/4","classification":"approximate_cloud_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(site_error),"hausdorffErrorUpper":hausdorff,
        "admittedAssumptions":["PCA-plane-parameter-domain","delta-net-density-from-sqrt-N-samples","control-hull-Lipschitz-remainder"],
        "siteCount":points.len(),"controlsU":controls_u,"controlsV":controls_v,
        "conditioning":{"method":"robust-pivoted-normal-equations","rank":rank,"pivotLower":pivot_min},
        "resource":{"maxSites":4096,"maxControlsPerAxis":8},"evidence":tolerance_evidence(&tolerance)}}),
    )
}
