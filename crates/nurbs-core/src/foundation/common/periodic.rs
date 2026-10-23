//! Periodic active spans, reconstruction and whole-domain deviation.
use super::*;

pub(in crate::foundation) fn periodic_active_knots(curve: &Curve) -> Vec<f64> {
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
            let greville = knots[i + 1..=i + degree].iter().sum::<f64>() / degree as f64;
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

pub(in crate::foundation) fn periodic_error(source: &Curve, target: &Curve) -> Result<f64> {
    let [a, b] = source.domain();
    check(
        target.domain() == [a, b],
        "Deviation requires matching parameter domains",
    )?;
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

