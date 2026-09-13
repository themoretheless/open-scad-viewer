//! Bounded planar sketch validation and miter offset, retaining analytic arcs.
use super::{Result, Value, cad_sketch, encode, field, input};
type P = [f64; 2];
fn finite(values: impl IntoIterator<Item = f64>) -> Result<()> {
    if values.into_iter().all(f64::is_finite) {
        Ok(())
    } else {
        Err(input("Contour arithmetic exceeds finite numeric range."))
    }
}
fn cross(a: P, b: P, c: P) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn validate(points: &[P], closed: bool) -> Result<()> {
    let n = points.len();
    if !(2..=512).contains(&n) {
        return Err(input("Invalid contour."));
    }
    finite(points.iter().flatten().copied())?;
    let edges = if closed { n } else { n - 1 };
    for i in 0..edges {
        for j in i + 2..edges {
            if closed && i == 0 && j == n - 1 {
                continue;
            }
            let (a, b, c, d) = (
                points[i],
                points[(i + 1) % n],
                points[j],
                points[(j + 1) % n],
            );
            if (0..2).any(|k| {
                a[k].max(b[k]) < c[k].min(d[k]) - 1e-8 || c[k].max(d[k]) < a[k].min(b[k]) - 1e-8
            }) {
                continue;
            }
            let q = [
                cross(a, b, c),
                cross(a, b, d),
                cross(c, d, a),
                cross(c, d, b),
            ];
            finite(q)?;
            let straddle = |a: f64, b: f64| (a <= 0. && b >= 0.) || (a >= 0. && b <= 0.);
            if straddle(q[0], q[1]) && straddle(q[2], q[3]) {
                return Err(input("The contour would self-intersect."));
            }
        }
    }
    Ok(())
}
pub fn validate_request(v: Value) -> Result<Value> {
    validate(&field::<Vec<P>>(&v, "points")?, field(&v, "closed")?)?;
    Ok(Value::Null)
}
pub fn offset(v: Value) -> Result<Value> {
    let mut sketch: Value = field(&v, "sketch")?;
    let distance: f64 = field(&v, "distance")?;
    if !distance.is_finite() || distance.abs() < 0.001 {
        return Err(input("Enter a nonzero offset."));
    }
    if let Some(curve) = sketch.get_mut("analytic") {
        let radius = field::<f64>(curve, "radius")? + distance;
        finite([radius])?;
        curve["radius"] = encode(radius)?;
        let points = cad_sketch::sample(curve.clone())?;
        sketch["points"] = points;
        return Ok(sketch);
    }
    if !field::<bool>(&sketch, "closed")? {
        return Err(input(
            "Offset requires a closed contour or an analytic arc.",
        ));
    }
    let p: Vec<P> = field(&sketch, "points")?;
    validate(&p, true)?;
    let n = p.len();
    let area = (0..n)
        .map(|i| p[i][0] * p[(i + 1) % n][1] - p[i][1] * p[(i + 1) % n][0])
        .sum::<f64>();
    finite([area])?;
    if area.abs() < 1e-8 {
        return Err(input("Degenerate contour."));
    }
    let sign = area.signum();
    let mut lines = Vec::with_capacity(n);
    for i in 0..n {
        let a = p[i];
        let b = p[(i + 1) % n];
        let d = [b[0] - a[0], b[1] - a[1]];
        let length = d[0].hypot(d[1]);
        finite([length])?;
        if length < 1e-8 {
            return Err(input("Zero edge."));
        }
        let origin = [
            a[0] + sign * distance * d[1] / length,
            a[1] - sign * distance * d[0] / length,
        ];
        finite(origin)?;
        lines.push((origin, d));
    }
    let mut points = Vec::with_capacity(n);
    for i in 0..n {
        let (a, u) = lines[(i + n - 1) % n];
        let (b, w) = lines[i];
        let det = u[0] * w[1] - u[1] * w[0];
        finite([det])?;
        let point = if det.abs() < 1e-9 {
            if u[0] * w[0] + u[1] * w[1] < 0. {
                return Err(input("Folded contour."));
            }
            b
        } else {
            let t = ((b[0] - a[0]) * w[1] - (b[1] - a[1]) * w[0]) / det;
            [a[0] + t * u[0], a[1] + t * u[1]]
        };
        finite(point)?;
        points.push(point);
    }
    validate(&points, true)?;
    for i in 0..n {
        let a = points[i];
        let b = points[(i + 1) % n];
        let old = lines[i].1;
        let forward = (b[0] - a[0]) * old[0] + (b[1] - a[1]) * old[1];
        finite([forward])?;
        if forward <= 1e-8 {
            return Err(input("Offset collapses an edge. Reduce the distance."));
        }
    }
    sketch["points"] = encode(points)?;
    Ok(sketch)
}
