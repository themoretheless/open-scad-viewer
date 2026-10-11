use super::*;
pub(crate) fn values(v: &Value, what: &str) -> Result<Vec<f64>> {
    list(v, what)?.iter().map(|v| number(v, what)).collect()
}
pub(crate) fn ints(v: &Value, what: &str) -> Result<Vec<usize>> {
    list(v, what)?
        .iter()
        .map(|v| usize_value(v, what))
        .collect()
}
pub(crate) fn expand(mults: &[usize], knots: &[f64]) -> Result<Vec<f64>> {
    if mults.len() != knots.len() {
        return Err(refuse("Knot multiplicity count mismatch"));
    }
    let mut out = Vec::new();
    for (&m, &k) in mults.iter().zip(knots) {
        out.extend(std::iter::repeat_n(k, m));
    }
    Ok(out)
}
pub(crate) fn point(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "CARTESIAN_POINT" || args.len() != 2 {
        return Err(refuse("Expected CARTESIAN_POINT"));
    }
    let mut p = values(&args[1], "CARTESIAN_POINT coordinates")?;
    if p.len() != dim {
        return Err(refuse("CARTESIAN_POINT dimension mismatch"));
    }
    if dim == 3 {
        for x in &mut p {
            *x *= scale;
        }
    }
    Ok(p)
}
pub(crate) fn direction(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "DIRECTION" || args.len() != 2 {
        return Err(refuse("Expected DIRECTION"));
    }
    let d = values(&args[1], "DIRECTION ratios")?;
    if d.len() != dim {
        return Err(refuse("DIRECTION dimension mismatch"));
    }
    let n = d.iter().map(|x| x * x).sum::<f64>().sqrt();
    if !n.is_finite() || n <= 1e-12 {
        return Err(refuse("Degenerate DIRECTION"));
    }
    Ok(d.into_iter().map(|x| x / n).collect())
}
fn vector(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<Vec<f64>> {
    let (ty, args) = call(entities, id)?;
    if ty != "VECTOR" || args.len() != 3 {
        return Err(refuse("Expected VECTOR"));
    }
    let d = direction(entities, one_ref(&args[1], "VECTOR orientation")?, dim)?;
    let magnitude = number(&args[2], "VECTOR magnitude")? * if dim == 3 { scale } else { 1. };
    if magnitude <= 0. {
        return Err(refuse("VECTOR magnitude must be positive"));
    }
    Ok(d.into_iter().map(|x| x * magnitude).collect())
}
pub(crate) fn axis2(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
) -> Result<(Vec<f64>, Vec<f64>, Vec<f64>)> {
    let (ty, args) = call(entities, id)?;
    let expected = if dim == 3 {
        "AXIS2_PLACEMENT_3D"
    } else {
        "AXIS2_PLACEMENT_2D"
    };
    if ty != expected || args.len() < 2 {
        return Err(refuse(format!("Expected {expected}")));
    }
    let origin = point(entities, one_ref(&args[1], "AXIS2 location")?, dim, scale)?;
    if dim == 2 {
        let x = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
            direction(entities, one_ref(&args[2], "AXIS2 ref direction")?, 2)?
        } else {
            vec![1., 0.]
        };
        return Ok((origin, x.clone(), vec![-x[1], x[0]]));
    }
    let z = if args.len() > 2 && !matches!(args[2], Value::Omitted) {
        direction(entities, one_ref(&args[2], "AXIS2 axis")?, 3)?
    } else {
        vec![0., 0., 1.]
    };
    let mut x = if args.len() > 3 && !matches!(args[3], Value::Omitted) {
        direction(entities, one_ref(&args[3], "AXIS2 ref direction")?, 3)?
    } else {
        vec![1., 0., 0.]
    };
    let dot = x.iter().zip(&z).map(|(a, b)| a * b).sum::<f64>();
    for i in 0..3 {
        x[i] -= dot * z[i];
    }
    let nx = x.iter().map(|v| v * v).sum::<f64>().sqrt();
    if nx <= 1e-12 {
        return Err(refuse("AXIS2 directions are parallel"));
    }
    for v in &mut x {
        *v /= nx;
    }
    let y = vec![
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    Ok((origin, x, y))
}
fn conic_curve(origin: &[f64], x: &[f64], y: &[f64], a: f64, b: f64) -> Result<Curve> {
    if !(a > 0. && b > 0. && a.is_finite() && b.is_finite()) {
        return Err(refuse("Conic radii must be positive"));
    }
    let endpoints = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.], [1., 0.]];
    let shoulders = [[1., 1.], [-1., 1.], [-1., -1.], [1., -1.]];
    let map = |q: [f64; 2]| {
        (0..origin.len())
            .map(|i| origin[i] + a * q[0] * x[i] + b * q[1] * y[i])
            .collect::<Vec<_>>()
    };
    let mut cps = vec![map(endpoints[0])];
    let mut weights = vec![1.];
    for i in 0..4 {
        cps.push(map(shoulders[i]));
        weights.push(std::f64::consts::FRAC_1_SQRT_2);
        cps.push(map(endpoints[i + 1]));
        weights.push(1.);
    }
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.],
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
/// Isolate a point inverse with positive-weight rational convex-hull
/// exclusion.  Every retained interval is produced by exact knot insertion
/// (`Curve::trim`); therefore its Euclidean control hull encloses the complete
/// rational image.  Ambiguous, non-isolated and non-representable roots refuse.
pub(crate) fn isolate_curve_point(base: &Curve, target: &[f64]) -> Result<f64> {
    const MAX_CELLS: usize = 16_384;
    const PARAMETER_WIDTH: f64 = 1e-12;
    const GEOMETRIC_WIDTH: f64 = 1e-9;
    fn contains(curve: &Curve, target: &[f64]) -> bool {
        (0..target.len()).all(|axis| {
            let lo = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::INFINITY, f64::min);
            let hi = curve
                .control_points
                .iter()
                .map(|point| point[axis])
                .fold(f64::NEG_INFINITY, f64::max);
            target[axis] >= lo - 1e-12 && target[axis] <= hi + 1e-12
        })
    }
    let domain = base.domain();
    let mut breaks = base
        .knots
        .iter()
        .copied()
        .filter(|value| *value >= domain[0] && *value <= domain[1])
        .collect::<Vec<_>>();
    breaks.dedup();
    let mut pending = breaks
        .array_windows()
        .map(|[a, b]| base.trim(*a, *b))
        .collect::<Result<Vec<_>>>()?;
    let mut isolated = Vec::<[f64; 2]>::new();
    let mut visited = 0usize;
    while let Some(cell) = pending.pop() {
        visited += 1;
        if visited > MAX_CELLS {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exceeds certified isolation budget",
            ));
        }
        if !contains(&cell, target) {
            continue;
        }
        let domain = cell.domain();
        let hull_width = (0..target.len())
            .map(|axis| {
                let lo = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::INFINITY, f64::min);
                let hi = cell
                    .control_points
                    .iter()
                    .map(|point| point[axis])
                    .fold(f64::NEG_INFINITY, f64::max);
                hi - lo
            })
            .fold(0., f64::max);
        if domain[1] - domain[0] <= PARAMETER_WIDTH || hull_width <= GEOMETRIC_WIDTH {
            isolated.push(domain);
            continue;
        }
        let mid = (domain[0] + domain[1]) / 2.;
        if mid == domain[0] || mid == domain[1] {
            return Err(refuse(
                "TRIMMED_CURVE point inverse exhausted parameter precision",
            ));
        }
        let [left, right] = cell.split(mid)?;
        pending.push(right);
        pending.push(left);
    }
    isolated.sort_by(|left, right| left[0].total_cmp(&right[0]));
    let mut merged = Vec::<[f64; 2]>::new();
    for interval in isolated {
        if let Some(last) = merged.last_mut()
            && interval[0] <= last[1] + PARAMETER_WIDTH
        {
            last[1] = last[1].max(interval[1]);
            continue;
        }
        merged.push(interval)
    }
    if merged.len() != 1 {
        return Err(refuse(if merged.is_empty() {
            "TRIMMED_CURVE point selector is not on the basis curve"
        } else {
            "TRIMMED_CURVE point selector inverse is not unique"
        }));
    }
    let interval = merged[0];
    let mut parameter = (interval[0] + interval[1]) / 2.;
    for _ in 0..12 {
        let evaluation = base.evaluate(parameter)?;
        let Some(derivative) = evaluation.d1 else {
            break;
        };
        let denominator = derivative.iter().map(|value| value * value).sum::<f64>();
        if denominator <= 1e-24 {
            break;
        }
        let correction = evaluation
            .point
            .iter()
            .zip(target)
            .zip(&derivative)
            .map(|((point, wanted), tangent)| (point - wanted) * tangent)
            .sum::<f64>()
            / denominator;
        let next = (parameter - correction).clamp(interval[0], interval[1]);
        if next == parameter {
            break;
        }
        parameter = next;
    }
    let residual = base
        .evaluate(parameter)?
        .point
        .iter()
        .zip(target)
        .map(|(point, wanted)| (point - wanted) * (point - wanted))
        .sum::<f64>()
        .sqrt();
    if !residual.is_finite() || residual > GEOMETRIC_WIDTH {
        return Err(refuse(
            "TRIMMED_CURVE isolated point inverse failed certified residual bound",
        ));
    }
    Ok(parameter)
}
fn trim_parameters(
    entities: &BTreeMap<usize, Entity>,
    v: &Value,
    base: &Curve,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<f64> {
    let items = list(v, "TRIMMED_CURVE trim selector")?;
    if items.len() != 1 {
        return Err(refuse(
            "TRIMMED_CURVE admits one parameter selector per end",
        ));
    }
    match &items[0] {
        Value::Number(_) | Value::Integer(_) => number(&items[0], "TRIMMED_CURVE parameter"),
        Value::Call(name, args) if name == "PARAMETER_VALUE" && args.len() == 1 => {
            number(&args[0], "PARAMETER_VALUE")
        }
        Value::Ref(id) => {
            let target = point(entities, *id, dim, scale)?;
            let domain = base.domain();
            let start = base.evaluate(domain[0])?.point;
            let end = base.evaluate(domain[1])?.point;
            let distance = |p: &[f64]| {
                p.iter()
                    .zip(&target)
                    .map(|(a, b)| (a - b) * (a - b))
                    .sum::<f64>()
                    .sqrt()
            };
            if distance(&start) <= 1e-7 {
                Ok(domain[0])
            } else if distance(&end) <= 1e-7 {
                Ok(domain[1])
            } else if interior_points {
                isolate_curve_point(base, &target)
            } else {
                Err(refuse(
                    "TRIMMED_CURVE point selector is not an exact basis endpoint",
                ))
            }
        }
        _ => Err(refuse(
            "TRIMMED_CURVE selector is outside the parameter/exact-endpoint subset",
        )),
    }
}
fn trim_conic(base: &Curve, start: f64, end: f64, sense: bool) -> Result<Curve> {
    let mut a0 = start;
    let mut a1 = end;
    if !sense {
        std::mem::swap(&mut a0, &mut a1);
    }
    while a1 <= a0 {
        a1 += std::f64::consts::TAU;
    }
    if a1 - a0 > std::f64::consts::TAU + 1e-10 {
        return Err(refuse("TRIMMED_CURVE conic sweep exceeds one revolution"));
    }
    let center = {
        let p0 = &base.control_points[0];
        let p4 = &base.control_points[4];
        p0.iter()
            .zip(p4)
            .map(|(a, b)| (a + b) / 2.)
            .collect::<Vec<_>>()
    };
    let ex: Vec<_> = base.control_points[0]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let ey: Vec<_> = base.control_points[2]
        .iter()
        .zip(&center)
        .map(|(a, b)| a - b)
        .collect();
    let pieces = ((a1 - a0) / std::f64::consts::FRAC_PI_2).ceil().max(1.) as usize;
    let mut cps = Vec::new();
    let mut weights = Vec::new();
    let mut knots = vec![0., 0., 0.];
    for i in 0..pieces {
        let s = a0 + (a1 - a0) * i as f64 / pieces as f64;
        let e = a0 + (a1 - a0) * (i + 1) as f64 / pieces as f64;
        let m = (s + e) / 2.;
        let w = ((e - s) / 2.).cos();
        let p = |angle: f64, factor: f64| {
            center
                .iter()
                .enumerate()
                .map(|(j, c)| c + factor * (ex[j] * angle.cos() + ey[j] * angle.sin()))
                .collect::<Vec<_>>()
        };
        if i == 0 {
            cps.push(p(s, 1.));
            weights.push(1.);
        }
        cps.push(p(m, 1. / w));
        weights.push(w);
        cps.push(p(e, 1.));
        weights.push(1.);
        if i + 1 < pieces {
            knots.push((i + 1) as f64);
            knots.push((i + 1) as f64);
        }
    }
    knots.extend([pieces as f64; 3]);
    let c = Curve {
        degree: 2,
        knots,
        control_points: cps,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(c)
}
fn curve_with_selectors(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let (ty, a) = call(entities, id)?;
    if ty == "LINE" {
        if a.len() != 3 {
            return Err(refuse("LINE argument count mismatch"));
        }
        let p = point(entities, one_ref(&a[1], "LINE point")?, dim, scale)?;
        let d = vector(entities, one_ref(&a[2], "LINE vector")?, dim, scale)?;
        return Curve::from_polyline(vec![
            p.clone(),
            p.iter().zip(d).map(|(x, d)| x + d).collect(),
        ]);
    }
    if matches!(ty, "CIRCLE" | "ELLIPSE") {
        let expected = if ty == "CIRCLE" { 3 } else { 4 };
        if a.len() != expected {
            return Err(refuse(format!("{ty} argument count mismatch")));
        }
        let (o, x, y) = axis2(entities, one_ref(&a[1], "conic placement")?, dim, scale)?;
        let major = number(&a[2], "conic radius")? * if dim == 3 { scale } else { 1. };
        let minor = if ty == "ELLIPSE" {
            number(&a[3], "ellipse minor radius")? * if dim == 3 { scale } else { 1. }
        } else {
            major
        };
        return conic_curve(&o, &x, &y, major, minor);
    }
    if ty == "TRIMMED_CURVE" {
        if a.len() != 6 {
            return Err(refuse("TRIMMED_CURVE argument count mismatch"));
        }
        let basis_id = one_ref(&a[1], "TRIMMED_CURVE basis")?;
        let (basis_ty, _) = call(entities, basis_id)?;
        let base = curve_with_selectors(entities, basis_id, dim, scale, interior_points)?;
        let start = trim_parameters(entities, &a[2], &base, dim, scale, interior_points)?;
        let end = trim_parameters(entities, &a[3], &base, dim, scale, interior_points)?;
        let sense = boolean(&a[4], "TRIMMED_CURVE sense")?;
        if matches!(basis_ty, "CIRCLE" | "ELLIPSE") {
            return trim_conic(&base, start, end, sense);
        }
        if basis_ty == "LINE" {
            let p0 = base.evaluate(start)?.point;
            let p1 = base.evaluate(end)?.point;
            return Curve::from_polyline(if sense { vec![p0, p1] } else { vec![p1, p0] });
        }
        return Err(refuse(
            "TRIMMED_CURVE basis is outside the exact direct subset",
        ));
    }
    if ty != "B_SPLINE_CURVE_WITH_KNOTS" {
        return Err(refuse(format!(
            "Reachable curve {ty} is typed-refused by the direct native subset"
        )));
    }
    if a.len() != 11 {
        return Err(refuse("B_SPLINE_CURVE_WITH_KNOTS argument count mismatch"));
    }
    let degree = usize_value(&a[1], "curve degree")?;
    let cps = list(&a[2], "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = values(&a[7], "curve weights")?;
    let knots = expand(
        &ints(&a[8], "curve multiplicities")?,
        &values(&a[9], "curve knots")?,
    )?;
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: boolean(&a[5], "curve closed flag")?,
    };
    c.validate()?;
    Ok(c)
}
pub(crate) fn curve(entities: &BTreeMap<usize, Entity>, id: usize, dim: usize, scale: f64) -> Result<Curve> {
    curve_with_selectors(entities, id, dim, scale, false)
}
pub(crate) fn curve_v5(
    entities: &BTreeMap<usize, Entity>,
    id: usize,
    dim: usize,
    scale: f64,
    interior_points: bool,
) -> Result<Curve> {
    let parts = components(entities, id)?;
    if parts.len() == 1 && !matches!(parts[0].0, "B_SPLINE_CURVE_WITH_KNOTS") {
        return curve_with_selectors(entities, id, dim, scale, interior_points);
    }
    let (degree, cp_values, closed, multiplicities, knot_values, weights) = if parts.len() == 1 {
        let a = parts[0].1;
        if a.len() != 9 {
            return Err(refuse(
                "AP242 B_SPLINE_CURVE_WITH_KNOTS requires nine inherited/explicit attributes",
            ));
        }
        (
            usize_value(&a[1], "curve degree")?,
            &a[2],
            boolean(&a[4], "curve closed flag")?,
            &a[6],
            &a[7],
            None,
        )
    } else {
        let base = component(&parts, "B_SPLINE_CURVE").ok_or_else(|| {
            refuse("RATIONAL_B_SPLINE_CURVE complex lacks B_SPLINE_CURVE component")
        })?;
        let knots = component(&parts, "B_SPLINE_CURVE_WITH_KNOTS")
            .ok_or_else(|| refuse("RATIONAL_B_SPLINE_CURVE complex lacks knot component"))?;
        let rational = component(&parts, "RATIONAL_B_SPLINE_CURVE").ok_or_else(|| {
            refuse("Complex B-spline curve lacks RATIONAL_B_SPLINE_CURVE component")
        })?;
        if base.len() != 5 || knots.len() != 3 || rational.len() != 1 {
            return Err(refuse(
                "Malformed AP242 rational B-spline curve composition",
            ));
        }
        (
            usize_value(&base[0], "curve degree")?,
            &base[1],
            boolean(&base[3], "curve closed flag")?,
            &knots[0],
            &knots[1],
            Some(&rational[0]),
        )
    };
    let cps = list(cp_values, "curve control points")?
        .iter()
        .map(|v| point(entities, one_ref(v, "curve control point")?, dim, scale))
        .collect::<Result<Vec<_>>>()?;
    let weights = match weights {
        Some(weight_data) => values(weight_data, "curve weights")?,
        None => vec![1.; cps.len()],
    };
    if weights.len() != cps.len() {
        return Err(refuse(
            "RATIONAL_B_SPLINE_CURVE weight/control-point count mismatch",
        ));
    }
    let c = Curve {
        degree,
        knots: expand(
            &ints(multiplicities, "curve multiplicities")?,
            &values(knot_values, "curve knots")?,
        )?,
        control_points: cps,
        weights,
        periodic: closed,
    };
    c.validate()?;
    Ok(c)
}
