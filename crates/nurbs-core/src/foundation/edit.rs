use super::*;

/// Exact interpolation of the supplied data sites as a degree-one spline.
pub fn interpolate_polyline(
    points: Vec<Vec<f64>>,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    let curve = Curve::from_polyline(points)?;
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":curve,
        "certificate":{
            "version":"nurbs-foundation/1",
            "method":"piecewise-linear-interpolation",
            "dataSiteErrorUpper":0.,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

/// Conservative piecewise-linear approximation. The hull diameter is a
/// symmetric Hausdorff upper bound because source span and chord share a hull.
pub fn approximate_curve(curve: &Curve, tolerance: Option<ToleranceContext>) -> Result<Value> {
    curve.validate()?;
    let tolerance = context(tolerance);
    let segments = curve.decompose()?;
    let mut points = Vec::new();
    let mut error: f64 = 0.;
    for (index, segment) in segments.iter().enumerate() {
        let c = segment.definition();
        let [a, b] = segment.domain();
        if index == 0 {
            points.push(c.evaluate(a)?.point);
        }
        points.push(c.evaluate(b)?.point);
        for first in &c.control_points {
            for second in &c.control_points {
                error = error.max(
                    first
                        .iter()
                        .zip(second)
                        .map(|(a, b)| (a - b) * (a - b))
                        .sum::<f64>()
                        .sqrt(),
                );
            }
        }
    }
    let approximation = Curve::from_polyline(points)?;
    Ok(json!({
        "curve":approximation,
        "certificate":{
            "version":"nurbs-foundation/1",
            "method":"per-span-convex-hull-diameter",
            "hausdorffErrorUpper":next_up(error),
            "withinEntityTolerance":next_up(error)<=tolerance.entity_error_bounds().maximum_mm,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

/// Exact affine monotone reparameterization, including periodic exterior knots.
pub fn reparameterize_curve(
    curve: &Curve,
    domain: [f64; 2],
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    curve.validate()?;
    check(
        domain.iter().all(|value| value.is_finite()) && domain[0] < domain[1],
        "Reparameterization domain must be finite and increasing",
    )?;
    let old = curve.domain();
    let scale = (domain[1] - domain[0]) / (old[1] - old[0]);
    let mut output = curve.clone();
    output.knots = curve
        .knots
        .iter()
        .map(|knot| domain[0] + (knot - old[0]) * scale)
        .collect();
    output.validate()?;
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":output,
        "certificate":{
            "version":"nurbs-foundation/1",
            "mapping":{"oldDomain":old,"newDomain":domain,"scale":scale},
            "monotone":true,
            "geometryErrorUpper":0.,
            "periodPreserved":curve.periodic,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

pub fn certify_exact_edit(
    source: &Curve,
    operation: &str,
    parameter: f64,
    count: usize,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    let output = match operation {
        "insert" => source.insert(parameter, count)?,
        "elevate" => source.elevate(count)?,
        _ => return Err(crate::input("Unknown exact certified edit")),
    };
    let tolerance = context(tolerance);
    Ok(json!({
        "curve":output,
        "certificate":{
            "version":"nurbs-foundation/1",
            "operation":operation,
            "geometryErrorUpper":0.,
            "periodPreserved":source.periodic==output.periodic,
            "evidence":tolerance_evidence(&tolerance)
        }
    }))
}

pub(super) fn solve(mut matrix: Vec<Vec<f64>>, mut values: Vec<Vec<f64>>) -> Result<Vec<Vec<f64>>> {
    let n = matrix.len();
    for column in 0..n {
        let pivot = (column..n)
            .max_by(|&a, &b| matrix[a][column].abs().total_cmp(&matrix[b][column].abs()))
            .unwrap();
        numeric(
            matrix[pivot][column].abs() > 64. * f64::EPSILON,
            "Reduction interpolation system is singular",
        )?;
        matrix.swap(column, pivot);
        values.swap(column, pivot);
        let divisor = matrix[column][column];
        for value in &mut matrix[column][column..] {
            *value /= divisor;
        }
        for value in &mut values[column] {
            *value /= divisor;
        }
        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = matrix[row][column];
            for j in column..n {
                matrix[row][j] -= factor * matrix[column][j];
            }
            let pivot_values = values[column].clone();
            for (value, pivot_value) in values[row].iter_mut().zip(pivot_values) {
                *value -= factor * pivot_value;
            }
        }
    }
    Ok(values)
}

pub(crate) fn refit_curve(source: &Curve, degree: usize, knots: Vec<f64>) -> Result<Curve> {
    let count = knots.len() - degree - 1;
    check(
        count > degree && count <= 256,
        "Reduction would produce an invalid control count",
    )?;
    let parameters = (0..count)
        .map(|i| {
            if degree == 0 {
                knots[i]
            } else {
                knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64
            }
        })
        .collect::<Vec<_>>();
    let matrix = parameters
        .iter()
        .map(|&u| crate::curve::basis(degree, &knots, count, u, false).map(|basis| basis.basis))
        .collect::<Result<Vec<_>>>()?;
    let dimension = source.control_points[0].len();
    let mut values = Vec::new();
    for &u in &parameters {
        let basis = crate::curve::basis(
            source.degree,
            &source.knots,
            source.control_points.len(),
            u,
            source.periodic,
        )?
        .basis;
        let weight = basis
            .iter()
            .zip(&source.weights)
            .map(|(b, w)| b * w)
            .sum::<f64>();
        let point = source.evaluate(u)?.point;
        values.push(
            point
                .into_iter()
                .map(|x| x * weight)
                .chain(std::iter::once(weight))
                .collect(),
        );
    }
    let homogeneous = solve(matrix, values)?;
    let weights = homogeneous
        .iter()
        .map(|value| value[dimension])
        .collect::<Vec<_>>();
    numeric(
        weights
            .iter()
            .all(|weight| *weight >= 1e-12 && *weight <= 1e12),
        "Reduction produced inadmissible weights",
    )?;
    let control_points = homogeneous
        .iter()
        .map(|value| {
            value[..dimension]
                .iter()
                .map(|x| x / value[dimension])
                .collect()
        })
        .collect();
    let output = Curve {
        degree,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    output.validate()?;
    Ok(output)
}

pub(super) fn exact_curve(first: &Curve, second: &Curve) -> bool {
    first.degree == second.degree
        && first.knots == second.knots
        && first.control_points == second.control_points
        && first.weights == second.weights
        && first.periodic == second.periodic
}
pub(super) fn exact_surface(first: &Surface, second: &Surface) -> bool {
    first.degree_u == second.degree_u
        && first.degree_v == second.degree_v
        && first.knots_u == second.knots_u
        && first.knots_v == second.knots_v
        && first.control_points == second.control_points
        && first.weights == second.weights
        && first.periodic_u == second.periodic_u
        && first.periodic_v == second.periodic_v
}

pub fn remove_curve_knot(
    source: &Curve,
    knot: f64,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified removal currently requires non-periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let domain = source.domain();
    check(
        knot > domain[0] && knot < domain[1],
        "Only interior knots can be removed",
    )?;
    let Some(index) = source.knots.iter().position(|value| *value == knot) else {
        return Err(crate::input("Knot is absent"));
    };
    let mut knots = source.knots.clone();
    knots.remove(index);
    let candidate = refit_curve(source, source.degree, knots)?;
    let reconstructed = candidate.insert(knot, 1)?;
    let exact = exact_curve(source, &reconstructed);
    let error = if exact {
        0.
    } else {
        periodic_error(source, &reconstructed)?
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/3","operation":"knot-removal","accepted":accepted,"rolledBack":!accepted,
        "exactZeroRecognized":exact,"hausdorffErrorUpper":error,"budget":max_error,
        "method":"inverse-insertion-with-adaptive-Lipschitz-envelope","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub(super) fn rebuild_candidate(source: &Curve, degree: usize, control_count: usize) -> Result<Curve> {
    check((1..=25).contains(&degree), "Rebuild degree must be in [1,25]")?;
    check(control_count > degree && control_count <= 256,
          "Rebuild control count must exceed degree and be at most 256")?;
    let [a,b] = source.domain();
    if source.periodic {
        let unique = control_count - degree;
        check(unique > degree, "Periodic rebuild needs more unique controls than the degree (total count must exceed twice the degree)")?;
        let active = (0..=unique).map(|i| if i == unique {b} else {a + (b-a)*i as f64/unique as f64}).collect();
        return refit_periodic(source, degree, active);
    }
    let spans = control_count - degree;
    let mut knots = vec![a; degree + 1];
    knots.extend((1..spans).map(|i| a + (b-a) * i as f64 / spans as f64));
    knots.extend(std::iter::repeat_n(b, degree + 1));
    refit_curve(source, degree, knots)
}

/// Refit into a uniform clamped or wrapped periodic basis. Acceptance is
/// checked over the full parameter domain using the derivative envelope.
pub fn rebuild_curve(
    source: &Curve, degree: usize, control_count: usize,
    max_error: f64, tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(max_error.is_finite() && max_error >= 0., "Maximum error must be finite and nonnegative")?;
    let candidate = rebuild_candidate(source, degree, control_count)?;
    let exact = exact_curve(source, &candidate);
    let error = if exact {0.} else {periodic_error(source, &candidate)?};
    numeric(error.is_finite(), "Rebuild deviation bound overflowed")?;
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(json!({"curve":if accepted {candidate.clone()} else {source.clone()}, "certificate":{
        "version":"nurbs-foundation/3", "operation":"curve-rebuild",
        "accepted":accepted, "rolledBack":!accepted, "exactZeroRecognized":exact,
        "hausdorffErrorUpper":error, "budget":max_error,
        "method":"homogeneous-greville-refit-with-Lipschitz-envelope",
        "wrappedStorage":source.periodic, "seam":if source.periodic {Some(seam_certificate(&candidate)?)} else {None},
        "evidence":tolerance_evidence(&tolerance)}}))
}

pub fn reduce_curve_degree(
    source: &Curve,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified reduction currently requires non-periodic storage",
    )?;
    check(
        degree >= 1 && degree < source.degree,
        "Target degree must be in [1, source degree)",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let remove = source.degree - degree;
    let mut knots = source.knots.clone();
    for _ in 0..remove {
        let mut index = knots.len();
        let mut previous = None;
        while index > 0 {
            index -= 1;
            if previous != Some(knots[index]) {
                previous = Some(knots[index]);
                knots.remove(index);
            }
        }
    }
    let candidate = refit_curve(source, degree, knots)?;
    let reconstructed = candidate.elevate(source.degree)?;
    let exact = exact_curve(source, &reconstructed);
    let error = if exact {
        0.
    } else {
        periodic_error(source, &reconstructed)?
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/3","operation":"degree-reduction","accepted":accepted,"rolledBack":!accepted,
        "exactZeroRecognized":exact,"hausdorffErrorUpper":error,"budget":max_error,
        "method":"inverse-elevation-with-adaptive-Lipschitz-envelope","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn reduce_surface_axis(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    check(operation == "remove" || operation == "reduce", "Unknown surface simplification operation")?;
    let (source_degree, periodic) = match axis {
        Axis::U => (source.degree_u, source.periodic_u),
        Axis::V => (source.degree_v, source.periodic_v),
    };
    check(!periodic, "Use periodic surface editing for a periodic axis")?;
    if operation == "reduce" {
        check(degree >= 1 && degree < source_degree, "Target degree must be in [1, source degree)")?;
    } else {
        check(parameter.is_finite(), "Knot parameter must be finite")?;
    }
    let candidate = source.edit_axis(axis, |curve| {
        if operation == "remove" {
            let mut knots = curve.knots.clone();
            let index = knots
                .iter()
                .position(|value| *value == parameter)
                .ok_or_else(|| crate::input("Knot is absent"))?;
            knots.remove(index);
            refit_curve(curve, curve.degree, knots)
        } else {
            let mut knots = curve.knots.clone();
            for _ in 0..curve.degree - degree {
                let mut index = knots.len();
                let mut previous = None;
                while index > 0 {
                    index -= 1;
                    if previous != Some(knots[index]) {
                        previous = Some(knots[index]);
                        knots.remove(index);
                    }
                }
            }
            refit_curve(curve, degree, knots)
        }
    })?;
    let reconstructed = candidate.edit_axis(axis, |curve| {
        if operation == "remove" {
            curve.insert(parameter, 1)
        } else {
            curve.elevate(match axis {
                Axis::U => source.degree_u,
                Axis::V => source.degree_v,
            })
        }
    })?;
    let exact = exact_surface(source, &reconstructed);
    let first = source
        .control_points
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let second = reconstructed
        .control_points
        .iter()
        .flatten()
        .cloned()
        .collect::<Vec<_>>();
    let (min, max) = box_of(&first.iter().chain(&second).cloned().collect::<Vec<_>>());
    let error = if exact {
        0.
    } else {
        next_up(
            min.iter()
                .zip(max)
                .map(|(a, b)| (b - a) * (b - a))
                .sum::<f64>()
                .sqrt(),
        )
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":if accepted {candidate} else {source.clone()},"certificate":{
        "version":"nurbs-foundation/2","operation":if operation=="remove"{"surface-knot-removal"}else{"surface-degree-reduction"},
        "accepted":accepted,"rolledBack":!accepted,"exactZeroRecognized":exact,
        "hausdorffErrorUpper":error,"budget":max_error,"method":"axiswise-refit-with-outward-global-hull",
        "evidence":tolerance_evidence(&tolerance)}}),
    )
}
