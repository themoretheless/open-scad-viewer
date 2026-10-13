use super::*;

pub(super) fn chord_parameters(points: &[Vec<f64>]) -> Vec<f64> {
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
pub fn fit_curve_points(
    points: Vec<Vec<f64>>,
    control_count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    check(
        points.len() >= 2 && points.len() <= 4096,
        "Curve fitting needs 2..4096 data sites",
    )?;
    let dimension = points[0].len();
    check(
        (dimension == 2 || dimension == 3)
            && points
                .iter()
                .all(|p| p.len() == dimension && p.iter().all(|x| x.is_finite())),
        "Curve fitting data must be finite 2D or 3D points",
    )?;
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
    Ok(
        json!({"curve":curve,"certificate":{"version":"nurbs-foundation/3","classification":"approximate_fit",
        "fittedToExactPromotion":false,"dataSiteErrorUpper":next_up(residual),"siteCount":points.len(),
        "controlCount":control_count,"degree":degree,"conditioning":{"method":"normal-equations-pivoted-elimination","rank":control_count},
        "resource":{"maxSites":4096,"maxControls":26},"evidence":tolerance_evidence(&tolerance)}}),
    )
}
pub fn interpolate_surface_grid(
    points: Vec<Vec<[f64; 3]>>,
    fitting: bool,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
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
    Ok(
        json!({"surface":surface,"certificate":{"version":"nurbs-foundation/3",
        "classification":if fitting{"approximate_fit"}else{"interpolation"},
        "fittedToExactPromotion":false,"dataSiteErrorUpper":0.,"rank":points.len()*nv,
        "conditioning":{"method":"tensor-degree-one-cardinal","conditionUpper":1.},
        "resource":{"maxControlsPerAxis":32},"evidence":tolerance_evidence(&tolerance)}}),
    )
}
