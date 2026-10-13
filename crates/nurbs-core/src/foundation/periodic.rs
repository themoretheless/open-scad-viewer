use super::*;

pub(super) fn periodic_active_knots(curve: &Curve) -> Vec<f64> {
    curve.knots[curve.degree..=curve.control_points.len()].to_vec()
}

pub(crate) fn periodic_knots(active: &[f64], degree: usize) -> Vec<f64> {
    let period = active[active.len() - 1] - active[0];
    let mut knots = active[active.len() - 1 - degree..active.len() - 1]
        .iter()
        .map(|value| value - period)
        .collect::<Vec<_>>();
    knots.extend_from_slice(active);
    knots.extend(active[1..=degree].iter().map(|value| value + period));
    knots
}

pub(crate) fn refit_periodic(source: &Curve, degree: usize, active: Vec<f64>) -> Result<Curve> {
    let unique = active.len() - 1;
    check(
        unique > degree && unique + degree <= 256,
        "Periodic edit exceeds control resources",
    )?;
    let knots = periodic_knots(&active, degree);
    let period = active[unique] - active[0];
    let parameters = (0..unique)
        .map(|i| {
            // Cyclic Greville sites avoid the singular midpoint system for odd
            // degrees with an even number of uniform periodic controls.
            let greville = knots[i+1..=i+degree].iter().sum::<f64>() / degree as f64;
            active[0] + (greville - active[0]).rem_euclid(period)
        })
        .collect::<Vec<_>>();
    let matrix = parameters
        .iter()
        .map(|&u| {
            let basis = crate::curve::basis(degree, &knots, unique + degree, u, true)?.basis;
            Ok((0..unique)
                .map(|column| {
                    basis
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| index % unique == column)
                        .map(|(_, value)| value)
                        .sum()
                })
                .collect())
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let dimension = source.control_points[0].len();
    let values = parameters
        .iter()
        .map(|&u| {
            let basis = crate::curve::basis(
                source.degree,
                &source.knots,
                source.control_points.len(),
                u,
                true,
            )?
            .basis;
            let weight = basis
                .iter()
                .zip(&source.weights)
                .map(|(b, w)| b * w)
                .sum::<f64>();
            let point = source.evaluate(u)?.point;
            Ok(point
                .into_iter()
                .map(|value| value * weight)
                .chain(std::iter::once(weight))
                .collect())
        })
        .collect::<Result<Vec<Vec<f64>>>>()?;
    let homogeneous = solve(matrix, values)?;
    let weights = homogeneous
        .iter()
        .map(|value| value[dimension])
        .collect::<Vec<_>>();
    numeric(
        weights
            .iter()
            .all(|weight| *weight >= 1e-12 && *weight <= 1e12),
        "Periodic edit produced inadmissible weights",
    )?;
    let points = homogeneous
        .iter()
        .map(|value| {
            value[..dimension]
                .iter()
                .map(|x| x / value[dimension])
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut control_points = points.clone();
    control_points.extend(points[..degree].iter().cloned());
    let mut wrapped_weights = weights.clone();
    wrapped_weights.extend(weights[..degree].iter().copied());
    let curve = Curve {
        degree,
        knots,
        control_points,
        weights: wrapped_weights,
        periodic: true,
    };
    curve.validate()?;
    Ok(curve)
}

pub(super) fn seam_certificate(curve: &Curve) -> Result<Value> {
    let [a, b] = curve.domain();
    let first = curve.evaluate(a)?;
    let last = curve.evaluate(b)?;
    let residual = |x: &[f64], y: &[f64]| next_up(distance(x, y));
    let c0 = residual(&first.point, &last.point);
    let c1 = match (&first.d1, &last.d1) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    let c2 = match (&first.d2, &last.d2) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    Ok(json!({"domain":[a,b],"period":b-a,
        "c0":{"available":true,"residualUpper":c0,"certified":c0<=64.*f64::EPSILON},
        "c1":{"available":c1.is_some(),"residualUpper":c1,"certified":c1.is_some_and(|value|value<=256.*f64::EPSILON)},
        "c2":{"available":c2.is_some(),"residualUpper":c2,"certified":c2.is_some_and(|value|value<=1024.*f64::EPSILON)}
    }))
}

pub(super) fn periodic_error(source: &Curve, target: &Curve) -> Result<f64> {
    let [a, b] = source.domain();
    check(target.domain() == [a, b], "Deviation requires matching parameter domains")?;
    // A Lipschitz envelope cannot bridge jumps at fully repeated interior knots.
    for curve in [source, target] {
        for &knot in &curve.knots {
            if knot > a && knot < b {
                check(
                    curve.knots.iter().filter(|&&value| value == knot).count() <= curve.degree,
                    "Deviation envelope requires continuous curves at interior knots",
                )?;
            }
        }
    }
    let speed_upper = |curve: &Curve| -> Result<f64> {
        let mut maximum = 0_f64;
        for segment in curve.decompose()? {
            let c = segment.definition();
            let dimension = c.control_points[0].len();
            let homogeneous = c
                .control_points
                .iter()
                .zip(&c.weights)
                .map(|(point, weight)| {
                    point
                        .iter()
                        .map(|coordinate| coordinate * weight)
                        .chain(std::iter::once(*weight))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let derivative = homogeneous
                .array_windows()
                .map(|[a, b]| {
                    a.iter()
                        .zip(b)
                        .map(|(x, y)| c.degree as f64 * (y - x))
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let w = homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>();
            let dw = derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>();
            let denominator = interval(w.iter().copied());
            let components = (0..dimension)
                .map(|axis| {
                    let first = bernstein_product(
                        &derivative
                            .iter()
                            .map(|value| value[axis])
                            .collect::<Vec<_>>(),
                        &w,
                    );
                    let second = bernstein_product(
                        &homogeneous
                            .iter()
                            .map(|value| value[axis])
                            .collect::<Vec<_>>(),
                        &dw,
                    );
                    first
                        .into_iter()
                        .zip(second)
                        .map(|(x, y)| (x - y).abs())
                        .fold(0., f64::max)
                        / (denominator[0] * denominator[0])
                })
                .collect::<Vec<_>>();
            maximum = maximum.max(
                components
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>()
                    .sqrt()
                    // Bernstein differences differentiate in the local [0,1] coordinate.
                    // Convert to the original parameter before forming the global envelope.
                    / (c.domain()[1] - c.domain()[0]),
            );
        }
        Ok(next_up(maximum))
    };
    let lipschitz = next_up(speed_upper(source)? + speed_upper(target)?);
    let mut error = 0_f64;
    for i in 0..=1024 {
        let u = a + (b - a) * i as f64 / 1024.;
        error = error.max(distance(
            &source.evaluate(u)?.point,
            &target.evaluate(u)?.point,
        ));
    }
    Ok(next_up(error + lipschitz * (b - a) / 2048.))
}

pub(super) fn periodic_candidate(
    source: &Curve,
    operation: &str,
    parameter: f64,
    degree: usize,
) -> Result<Curve> {
    let mut active = periodic_active_knots(source);
    let target_degree = match operation {
        "insert" => {
            check(
                parameter >= active[0] && parameter < *active.last().unwrap(),
                "Periodic insertion must use the half-open fundamental domain",
            )?;
            active.insert(
                active.partition_point(|value| *value <= parameter),
                parameter,
            );
            source.degree
        }
        "remove" => {
            let index = active
                .iter()
                .position(|value| {
                    *value == parameter && *value > active[0] && *value < *active.last().unwrap()
                })
                .ok_or_else(|| crate::input("Removable periodic knot is absent"))?;
            active.remove(index);
            source.degree
        }
        "elevate" => {
            check(
                degree > source.degree && degree <= 25,
                "Periodic elevation target is invalid",
            )?;
            let delta = degree - source.degree;
            let original = active.clone();
            for value in original.iter().rev() {
                let index = active.partition_point(|k| *k <= *value);
                for _ in 0..delta {
                    active.insert(index, *value);
                }
            }
            degree
        }
        "reduce" => {
            check(
                degree >= 1 && degree < source.degree,
                "Periodic reduction target is invalid",
            )?;
            for _ in 0..source.degree - degree {
                let distinct =
                    active
                        .iter()
                        .copied()
                        .fold(Vec::<f64>::new(), |mut values, value| {
                            if values.last() != Some(&value) {
                                values.push(value)
                            }
                            values
                        });
                for value in distinct.into_iter().rev() {
                    if active.iter().filter(|k| **k == value).count() > 1 {
                        active.remove(active.iter().rposition(|k| *k == value).unwrap());
                    }
                }
            }
            degree
        }
        _ => return Err(crate::input("Unknown periodic edit")),
    };
    refit_periodic(source, target_degree, active)
}

pub fn edit_periodic_curve(
    source: &Curve,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(
        source.periodic,
        "Wrapped periodic edit requires periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = periodic_candidate(source, operation, parameter, degree)?;
    let error = periodic_error(source, &candidate)?;
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let seam = seam_certificate(&output)?;
    let tolerance = context(tolerance);
    Ok(
        json!({"curve":output,"certificate":{"version":"nurbs-foundation/3","operation":operation,
        "accepted":accepted,"rolledBack":!accepted,"hausdorffErrorUpper":error,"budget":max_error,
        "wrappedStorage":true,"seam":seam,
        "method":"cyclic-collocation-with-global-outward-hull","evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn edit_periodic_surface(
    source: &Surface,
    axis: Axis,
    operation: &str,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    let periodic = match axis {
        Axis::U => source.periodic_u,
        Axis::V => source.periodic_v,
    };
    check(periodic, "Selected surface axis is not periodic")?;
    let candidate = source.edit_axis(axis, |curve| {
        periodic_candidate(curve, operation, parameter, degree)
    })?;
    let mut error = 0_f64;
    let samples = 32;
    for i in 0..=samples {
        let parameter_other = match axis {
            Axis::U => {
                source.knots_v[source.degree_v]
                    + (source.knots_v[source.control_points[0].len()]
                        - source.knots_v[source.degree_v])
                        * i as f64
                        / samples as f64
            }
            Axis::V => {
                source.knots_u[source.degree_u]
                    + (source.knots_u[source.control_points.len()]
                        - source.knots_u[source.degree_u])
                        * i as f64
                        / samples as f64
            }
        };
        let fixed = match axis {
            Axis::U => Axis::V,
            Axis::V => Axis::U,
        };
        error = error.max(periodic_error(
            &source.iso(fixed, parameter_other)?,
            &candidate.iso(fixed, parameter_other)?,
        )?);
    }
    let controls = source
        .control_points
        .iter()
        .flatten()
        .chain(candidate.control_points.iter().flatten())
        .cloned()
        .collect::<Vec<_>>();
    let (minimum, maximum) = box_of(&controls);
    error = error.max(next_up(
        minimum
            .iter()
            .zip(maximum)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f64>()
            .sqrt(),
    ));
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let representative = match axis {
        Axis::U => output.iso(Axis::V, output.knots_v[output.degree_v])?,
        Axis::V => output.iso(Axis::U, output.knots_u[output.degree_u])?,
    };
    let tolerance = context(tolerance);
    Ok(
        json!({"surface":output,"certificate":{"version":"nurbs-foundation/3",
        "operation":format!("periodic-surface-{operation}"),"axis":match axis{Axis::U=>"u",Axis::V=>"v"},
        "accepted":accepted,"rolledBack":!accepted,"hausdorffErrorUpper":next_up(error),"budget":max_error,
        "wrappedStorage":true,"representativeSeam":seam_certificate(&representative)?,
        "conditioning":{"sampledTransverseLines":samples+1},"evidence":tolerance_evidence(&tolerance)}}),
    )
}

pub fn split_periodic_curve(
    source: &Curve,
    parameter: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    source.validate()?;
    check(source.periodic, "Periodic split requires periodic storage")?;
    let [a, b] = source.domain();
    check(
        parameter > a && parameter < b,
        "Periodic split must be strictly inside the fundamental domain",
    )?;
    let pieces = [source.trim(parameter, b)?, source.trim(a, parameter)?];
    let tolerance = context(tolerance);
    Ok(
        json!({"curves":pieces,"certificate":{"version":"nurbs-foundation/3",
        "operation":"periodic-seam-split","sourceSeam":seam_certificate(source)?,
        "coverage":[[parameter,b],[a,parameter]],"orientationPreserved":true,
        "evidence":tolerance_evidence(&tolerance)}}),
    )
}
