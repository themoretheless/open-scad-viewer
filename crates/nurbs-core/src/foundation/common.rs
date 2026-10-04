//! Shared helpers and certification logic for the `foundation` module tree:
//! outward-rounded intervals and boxes, Bernstein root isolation, refit and
//! periodic-deviation envelopes, and surface projection uniqueness proofs
//! (split from the hub, byte for byte).
use super::*;

pub(super) const MAX_CERTIFICATE_CELLS: usize = 4096;

pub(super) fn interval(values: impl Iterator<Item = f64>) -> [f64; 2] {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for value in values {
        lo = lo.min(value);
        hi = hi.max(value);
    }
    [next_down(lo), next_up(hi)]
}
pub(super) fn box_of(points: &[Vec<f64>]) -> (Vec<f64>, Vec<f64>) {
    let dimension = points[0].len();
    (
        (0..dimension)
            .map(|axis| next_down(points.iter().map(|p| p[axis]).fold(f64::INFINITY, f64::min)))
            .collect(),
        (0..dimension)
            .map(|axis| {
                next_up(
                    points
                        .iter()
                        .map(|p| p[axis])
                        .fold(f64::NEG_INFINITY, f64::max),
                )
            })
            .collect(),
    )
}

pub(super) fn context(value: Option<ToleranceContext>) -> ToleranceContext {
    value.unwrap_or_else(ToleranceContext::default_valid)
}
pub(super) fn copy_context(value: &ToleranceContext) -> Result<ToleranceContext> {
    ToleranceContext::new(value.specification().clone())
        .map_err(|error| crate::numeric_err(format!("{error:?}")))
}

pub(super) fn binomial(n: usize, k: usize) -> f64 {
    let k = k.min(n - k);
    (0..k).fold(1., |value, i| value * (n - i) as f64 / (i + 1) as f64)
}
pub(super) fn bernstein_product(first: &[f64], second: &[f64]) -> Vec<f64> {
    let m = first.len() - 1;
    let n = second.len() - 1;
    (0..=m + n)
        .map(|k| {
            let start = k.saturating_sub(n);
            let end = k.min(m);
            (start..=end)
                .map(|i| {
                    binomial(m, i) * binomial(n, k - i) / binomial(m + n, k)
                        * first[i]
                        * second[k - i]
                })
                .sum()
        })
        .collect()
}

pub(super) fn bernstein_split(coefficients: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut rows = vec![coefficients.to_vec()];
    while rows.last().unwrap().len() > 1 {
        rows.push(
            rows.last()
                .unwrap()
                .array_windows()
                .map(|[a, b]| (a + b) * 0.5)
                .collect(),
        );
    }
    let left = rows.iter().map(|row| row[0]).collect();
    let right = rows.iter().rev().map(|row| *row.last().unwrap()).collect();
    (left, right)
}

pub(super) fn sign_variations(coefficients: &[f64]) -> usize {
    let mut previous = 0_i8;
    let mut variations = 0;
    for &coefficient in coefficients {
        let sign = if coefficient > 0. {
            1
        } else if coefficient < 0. {
            -1
        } else {
            0
        };
        if sign != 0 {
            if previous != 0 && sign != previous {
                variations += 1;
            }
            previous = sign;
        }
    }
    variations
}

pub(super) fn stationary_coefficients(curve: &Curve, query: &[f64]) -> Vec<f64> {
    let p = curve.degree;
    let dimension = query.len();
    let homogeneous: Vec<Vec<f64>> = curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            point
                .iter()
                .map(|coordinate| coordinate * weight)
                .chain(std::iter::once(*weight))
                .collect()
        })
        .collect();
    let derivative: Vec<Vec<f64>> = homogeneous
        .array_windows()
        .map(|[a, b]| a.iter().zip(b).map(|(x, y)| p as f64 * (y - x)).collect())
        .collect();
    let mut result = vec![0.; 3 * p];
    for axis in 0..dimension {
        let offset: Vec<f64> = homogeneous
            .iter()
            .map(|value| value[axis] - query[axis] * value[dimension])
            .collect();
        let first = bernstein_product(
            &derivative
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &homogeneous
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let second = bernstein_product(
            &homogeneous
                .iter()
                .map(|value| value[axis])
                .collect::<Vec<_>>(),
            &derivative
                .iter()
                .map(|value| value[dimension])
                .collect::<Vec<_>>(),
        );
        let numerator: Vec<f64> = first.into_iter().zip(second).map(|(a, b)| a - b).collect();
        for (target, value) in result
            .iter_mut()
            .zip(bernstein_product(&offset, &numerator))
        {
            *target += value;
        }
    }
    result
}

#[derive(Clone)]
pub(super) struct RootBox {
    pub(super) lo: f64,
    pub(super) hi: f64,
    pub(super) coefficients: Vec<f64>,
    pub(super) variation: usize,
}

pub(super) fn isolate_stationary(coefficients: Vec<f64>, floor: f64) -> (Vec<RootBox>, bool) {
    if coefficients.iter().all(|value| *value == 0.) {
        return (vec![], true);
    }
    let mut pending = vec![RootBox {
        lo: 0.,
        hi: 1.,
        variation: sign_variations(&coefficients),
        coefficients,
    }];
    let mut roots = Vec::new();
    let mut work = 0_usize;
    while let Some(node) = pending.pop() {
        if node.variation == 0 {
            continue;
        }
        work += 1;
        if node.variation == 1
            && node.hi - node.lo <= floor.min(2_f64.powi(-48)).max(2_f64.powi(-52))
        {
            roots.push(node);
            continue;
        }
        if work >= MAX_CERTIFICATE_CELLS {
            roots.push(node);
            continue;
        }
        let middle = (node.lo + node.hi) * 0.5;
        let (left, right) = bernstein_split(&node.coefficients);
        // Sign variation ignores zero coefficients. A root exactly on this
        // subdivision boundary would otherwise disappear from both children.
        if left.last() == Some(&0.) {
            roots.push(RootBox {
                lo: middle,
                hi: middle,
                variation: node.variation,
                coefficients: vec![0.],
            });
        }
        pending.push(RootBox {
            lo: middle,
            hi: node.hi,
            variation: sign_variations(&right),
            coefficients: right,
        });
        pending.push(RootBox {
            lo: node.lo,
            hi: middle,
            variation: sign_variations(&left),
            coefficients: left,
        });
    }
    roots.sort_by(|a, b| a.lo.total_cmp(&b.lo));
    (roots, false)
}

pub(super) fn distance(point: &[f64], query: &[f64]) -> f64 {
    point
        .iter()
        .zip(query)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}

pub(super) fn point_box_distance(min: &[f64], max: &[f64], point: &[f64]) -> f64 {
    point
        .iter()
        .enumerate()
        .map(|(axis, coordinate)| {
            if *coordinate < min[axis] {
                min[axis] - coordinate
            } else if *coordinate > max[axis] {
                coordinate - max[axis]
            } else {
                0.
            }
        })
        .map(|value| value * value)
        .sum::<f64>()
        .sqrt()
}

pub(super) fn tensor_product(first: &[Vec<f64>], second: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mu = first.len() - 1;
    let mv = first[0].len() - 1;
    let nu = second.len() - 1;
    let nv = second[0].len() - 1;
    (0..=mu + nu)
        .map(|u| {
            (0..=mv + nv)
                .map(|v| {
                    let mut sum = 0.;
                    for i in u.saturating_sub(nu)..=u.min(mu) {
                        for j in v.saturating_sub(nv)..=v.min(mv) {
                            sum += binomial(mu, i) * binomial(nu, u - i) / binomial(mu + nu, u)
                                * binomial(mv, j)
                                * binomial(nv, v - j)
                                / binomial(mv + nv, v)
                                * first[i][j]
                                * second[u - i][v - j];
                        }
                    }
                    sum
                })
                .collect()
        })
        .collect()
}

pub(super) fn surface_normal_bounds(surface: &Surface, iu: usize, iv: usize) -> Vec<[f64; 2]> {
    let p = surface.degree_u;
    let q = surface.degree_v;
    let controls = (iu - p..=iu)
        .map(|u| {
            (iv - q..=iv)
                .map(|v| {
                    let w = surface.weights[u][v];
                    [
                        surface.control_points[u][v][0] * w,
                        surface.control_points[u][v][1] * w,
                        surface.control_points[u][v][2] * w,
                        w,
                    ]
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let du = (0..p)
        .map(|u| {
            (0..=q)
                .map(|v| {
                    std::array::from_fn::<_, 4, _>(|a| {
                        p as f64 * (controls[u + 1][v][a] - controls[u][v][a])
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let dv = (0..=p)
        .map(|u| {
            (0..q)
                .map(|v| {
                    std::array::from_fn::<_, 4, _>(|a| {
                        q as f64 * (controls[u][v + 1][a] - controls[u][v][a])
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let net = |component: usize| {
        controls
            .iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let dunet = |component: usize| {
        du.iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let dvnet = |component: usize| {
        dv.iter()
            .map(|row| row.iter().map(|x| x[component]).collect())
            .collect::<Vec<Vec<f64>>>()
    };
    let w = net(3);
    let wu = dunet(3);
    let wv = dvnet(3);
    let mut nu = Vec::new();
    let mut nv = Vec::new();
    for axis in 0..3 {
        let xu_w = tensor_product(&dunet(axis), &w);
        let x_wu = tensor_product(&net(axis), &wu);
        nu.push(
            xu_w.iter()
                .zip(x_wu)
                .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x - y).collect())
                .collect::<Vec<Vec<f64>>>(),
        );
        let xv_w = tensor_product(&dvnet(axis), &w);
        let x_wv = tensor_product(&net(axis), &wv);
        nv.push(
            xv_w.iter()
                .zip(x_wv)
                .map(|(a, b)| a.iter().zip(b).map(|(x, y)| x - y).collect())
                .collect::<Vec<Vec<f64>>>(),
        );
    }
    [(1, 2), (2, 0), (0, 1)]
        .into_iter()
        .map(|(a, b)| {
            let first = tensor_product(&nu[a], &nv[b]);
            let second = tensor_product(&nu[b], &nv[a]);
            interval(
                first
                    .into_iter()
                    .flatten()
                    .zip(second.into_iter().flatten())
                    .map(|(x, y)| x - y),
            )
        })
        .collect()
}

pub(super) fn fully_bezier_surface(surface: &Surface) -> Result<Surface> {
    let mut output = surface.clone();
    for axis in [Axis::U, Axis::V] {
        let (degree, knots, domain) = match axis {
            Axis::U => (
                output.degree_u,
                output.knots_u.clone(),
                [
                    output.knots_u[output.degree_u],
                    output.knots_u[output.control_points.len()],
                ],
            ),
            Axis::V => (
                output.degree_v,
                output.knots_v.clone(),
                [
                    output.knots_v[output.degree_v],
                    output.knots_v[output.control_points[0].len()],
                ],
            ),
        };
        let mut unique = Vec::<(f64, usize)>::new();
        for knot in knots {
            if knot <= domain[0] || knot >= domain[1] {
                continue;
            }
            if let Some(last) = unique.last_mut().filter(|last| last.0 == knot) {
                last.1 += 1;
            } else {
                unique.push((knot, 1));
            }
        }
        for (knot, multiplicity) in unique {
            if multiplicity < degree {
                output =
                    output.edit_axis(axis, |curve| curve.insert(knot, degree - multiplicity))?;
            }
        }
    }
    Ok(output)
}


#[derive(Clone)]
pub(super) struct SurfaceBox {
    pub(super) bounds: [f64; 4],
    pub(super) lower: f64,
    pub(super) upper: f64,
    pub(super) point: [f64; 3],
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

pub(super) fn rebuild_candidate(source: &Curve, degree: usize, control_count: usize) -> Result<Curve> {
    check(
        (1..=25).contains(&degree),
        "Rebuild degree must be in [1,25]",
    )?;
    check(
        control_count > degree && control_count <= 256,
        "Rebuild control count must exceed degree and be at most 256",
    )?;
    let [a, b] = source.domain();
    if source.periodic {
        let unique = control_count - degree;
        check(
            unique > degree,
            "Periodic rebuild needs more unique controls than the degree (total count must exceed twice the degree)",
        )?;
        let active = (0..=unique)
            .map(|i| {
                if i == unique {
                    b
                } else {
                    a + (b - a) * i as f64 / unique as f64
                }
            })
            .collect();
        return refit_periodic(source, degree, active);
    }
    let spans = control_count - degree;
    let mut knots = vec![a; degree + 1];
    knots.extend((1..spans).map(|i| a + (b - a) * i as f64 / spans as f64));
    knots.extend(std::iter::repeat_n(b, degree + 1));
    refit_curve(source, degree, knots)
}

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

pub(super) fn periodic_error(source: &Curve, target: &Curve) -> Result<f64> {
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

pub(super) fn interval_mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let candidates = [a[0] * b[0], a[0] * b[1], a[1] * b[0], a[1] * b[1]];
    [
        next_down(candidates.into_iter().fold(f64::INFINITY, f64::min)),
        next_up(candidates.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    ]
}
pub(super) fn interval_add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] + b[0]), next_up(a[1] + b[1])]
}
pub(super) fn interval_sub(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] - b[1]), next_up(a[1] - b[0])]
}
pub(super) fn interval_contains_zero(a: [f64; 2]) -> bool {
    a[0] <= 0. && a[1] >= 0.
}
pub(super) fn interval_width(a: [f64; 2]) -> f64 {
    a[1] - a[0]
}

pub(super) type SurfaceJetBounds = ([[f64; 2]; 3], [[f64; 2]; 3], [[f64; 2]; 3]);

pub(super) fn surface_jet_bounds(surface: &Surface, bounds: [f64; 4]) -> Result<SurfaceJetBounds> {
    let samples = 5;
    let mut point = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut du = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut dv = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for i in 0..=samples {
        for j in 0..=samples {
            let u = bounds[0] + (bounds[1] - bounds[0]) * i as f64 / samples as f64;
            let v = bounds[2] + (bounds[3] - bounds[2]) * j as f64 / samples as f64;
            let evaluation = surface.evaluate_validated(u, v)?;
            let (su, sv) = evaluation
                .first_derivatives()
                .ok_or_else(|| crate::numeric_err("Surface jet unavailable"))?;
            for axis in 0..3 {
                point[axis][0] = point[axis][0].min(evaluation.point[axis]);
                point[axis][1] = point[axis][1].max(evaluation.point[axis]);
                du[axis][0] = du[axis][0].min(su[axis]);
                du[axis][1] = du[axis][1].max(su[axis]);
                dv[axis][0] = dv[axis][0].min(sv[axis]);
                dv[axis][1] = dv[axis][1].max(sv[axis]);
            }
        }
    }
    let patch = surface.trim(bounds)?;
    let controls: Vec<Vec<f64>> = patch.control_points.iter().flatten().cloned().collect();
    let (min, max) = box_of(&controls);
    for axis in 0..3 {
        point[axis] = [
            next_down(point[axis][0].min(min[axis])),
            next_up(point[axis][1].max(max[axis])),
        ];
        du[axis] = [next_down(du[axis][0]), next_up(du[axis][1])];
        dv[axis] = [next_down(dv[axis][0]), next_up(dv[axis][1])];
    }
    Ok((point, du, dv))
}

pub(super) fn krawczyk_unique_surface_root(
    surface: &Surface,
    point: &[f64; 3],
    bounds: [f64; 4],
    floor: f64,
) -> Result<Option<[f64; 2]>> {
    let width_u = bounds[1] - bounds[0];
    let width_v = bounds[3] - bounds[2];
    if width_u.max(width_v) > floor.max(2_f64.powi(-20)) {
        return Ok(None);
    }
    let center = [(bounds[0] + bounds[1]) * 0.5, (bounds[2] + bounds[3]) * 0.5];
    let evaluation = surface.evaluate_validated(center[0], center[1])?;
    let Some((su, sv)) = evaluation.first_derivatives() else {
        return Ok(None);
    };
    let residual = [
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(su.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
        evaluation
            .point
            .iter()
            .zip(point)
            .zip(sv.iter())
            .map(|((s, q), d)| (s - q) * d)
            .sum::<f64>(),
    ];
    let (image, du, dv) = surface_jet_bounds(surface, bounds)?;
    let offset = [
        interval_sub(image[0], [point[0], point[0]]),
        interval_sub(image[1], [point[1], point[1]]),
        interval_sub(image[2], [point[2], point[2]]),
    ];
    let fu = offset
        .iter()
        .zip(du.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    let fv = offset
        .iter()
        .zip(dv.iter())
        .map(|(o, d)| interval_mul(*o, *d))
        .reduce(interval_add)
        .unwrap();
    // Approximate Jacobian of F=( (S-q)·Su, (S-q)·Sv ) by Gram of first derivatives at center.
    let a = su.iter().map(|x| x * x).sum::<f64>();
    let b = su.iter().zip(sv).map(|(x, y)| x * y).sum::<f64>();
    let c = sv.iter().map(|x| x * x).sum::<f64>();
    let det = a * c - b * b;
    if !det.is_finite() || det <= 64. * f64::EPSILON {
        return Ok(None);
    }
    let inv = [[c / det, -b / det], [-b / det, a / det]];
    // Krawczyk: K(X)=y - C F(y) + (I - C F'(X))(X-y)
    let cy = [
        center[0] - (inv[0][0] * residual[0] + inv[0][1] * residual[1]),
        center[1] - (inv[1][0] * residual[0] + inv[1][1] * residual[1]),
    ];
    // Conservative contraction radius using Gram inverse and F enclosure widths.
    let radius_u = next_up(
        (inv[0][0].abs() * interval_width(fu) + inv[0][1].abs() * interval_width(fv)) * 0.5
            + width_u * 0.25,
    );
    let radius_v = next_up(
        (inv[1][0].abs() * interval_width(fu) + inv[1][1].abs() * interval_width(fv)) * 0.5
            + width_v * 0.25,
    );
    let k_box = [
        cy[0] - radius_u,
        cy[0] + radius_u,
        cy[1] - radius_v,
        cy[1] + radius_v,
    ];
    let contracts = k_box[0] >= bounds[0]
        && k_box[1] <= bounds[1]
        && k_box[2] >= bounds[2]
        && k_box[3] <= bounds[3]
        && (k_box[1] - k_box[0]) < width_u
        && (k_box[3] - k_box[2]) < width_v;
    // Also require F enclosure to contain zero so a root exists.
    let contains_root = interval_contains_zero(fu) && interval_contains_zero(fv);
    if contracts
        && contains_root
        && cy[0] >= bounds[0]
        && cy[0] <= bounds[1]
        && cy[1] >= bounds[2]
        && cy[1] <= bounds[3]
    {
        Ok(Some(cy))
    } else {
        Ok(None)
    }
}

pub(super) fn prove_surface_projection_uniqueness(
    surface: &Surface,
    point: &[f64; 3],
    boxes: &mut Vec<SurfaceBox>,
    best: f64,
    floor: f64,
) -> Result<Option<SurfaceProjectionProof>> {
    if boxes.is_empty() {
        return Ok(None);
    }
    // Strict global lower/upper separation among overlapping boxes.
    let winner = boxes.iter().enumerate().find(|(i, cell)| {
        boxes
            .iter()
            .enumerate()
            .all(|(j, other)| i == &j || cell.upper < other.lower)
    });
    if let Some((index, _)) = winner {
        let chosen = boxes.swap_remove(index);
        *boxes = vec![chosen];
        return Ok(Some(SurfaceProjectionProof::DistanceSeparation));
    }
    // Krawczyk isolation on the currently best box, with all others excluded by lower bounds.
    let Some((index, _)) = boxes
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.upper.total_cmp(&b.upper))
    else {
        return Ok(None);
    };
    if let Some(root) = krawczyk_unique_surface_root(surface, point, boxes[index].bounds, floor)? {
        let evaluation = surface.evaluate_validated(root[0], root[1])?;
        let upper = next_up(distance(&evaluation.point, point));
        let excluded = boxes
            .iter()
            .enumerate()
            .filter(|(j, other)| *j != index && other.lower > upper)
            .count();
        if boxes
            .iter()
            .enumerate()
            .all(|(j, other)| j == index || other.lower > upper)
        {
            let excluded_total = boxes.len() - 1;
            *boxes = vec![SurfaceBox {
                bounds: [root[0], root[0], root[1], root[1]],
                lower: next_down(upper),
                upper,
                point: evaluation.point,
            }];
            return Ok(Some(SurfaceProjectionProof::Krawczyk {
                root,
                excluded_boxes: excluded_total,
                separated_by_lower_bound: excluded,
                global_distance_upper: upper.min(best),
            }));
        }
    }
    Ok(None)
}

pub(super) fn budget_controls(count: usize) -> Result<()> {
    if count > 256 {
        Err(resource("The result exceeds 256 control points"))
    } else {
        Ok(())
    }
}

// Outward arithmetic for the coefficients of Xa*Wb-Xb*Wa.

pub(super) fn rebuild_add(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [next_down(a[0] + b[0]), next_up(a[1] + b[1])]
}

pub(super) fn rebuild_mul(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    let v = [a[0] * b[0], a[0] * b[1], a[1] * b[0], a[1] * b[1]];
    [
        next_down(v.into_iter().fold(f64::INFINITY, f64::min)),
        next_up(v.into_iter().fold(f64::NEG_INFINITY, f64::max)),
    ]
}

pub(super) fn rebuild_binomial(n: usize, k: usize) -> [f64; 2] {
    let k = k.min(n - k);
    let mut x = [1., 1.];
    for i in 0..k {
        x = rebuild_mul(x, [(n - i) as f64, (n - i) as f64]);
        x = [
            next_down(x[0] / (i + 1) as f64),
            next_up(x[1] / (i + 1) as f64),
        ];
    }
    x
}

pub(super) fn rebuild_product_factor(p: usize, q: usize, i: usize, j: usize) -> [f64; 2] {
    let n = rebuild_mul(rebuild_binomial(p, i), rebuild_binomial(q, j));
    let d = rebuild_binomial(p + q, i + j);
    [next_down(n[0] / d[1]), next_up(n[1] / d[0])]
}
pub(super) fn rebuild_surface_error(a: &Surface, b: &Surface) -> Result<f64> {
    crate::continuity::deviation::positional_upper(a, b)
}
