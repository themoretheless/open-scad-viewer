//! Bounded local-coordinate sketch trim and extension.
use super::{Result, Value, cad_sketch_offset, encode, field, input};
type P = [f64; 2];
fn points(s: &Value) -> Result<Vec<P>> {
    let p: Vec<P> = field(s, "points")?;
    if !(2..=512).contains(&p.len()) || !p.iter().flatten().all(|x| x.is_finite()) {
        return Err(input("Invalid contour."));
    }
    Ok(p)
}
fn hit(a: P, b: P, c: P, d: P) -> Result<Option<(f64, f64)>> {
    let x = b[0] - a[0];
    let y = b[1] - a[1];
    let vx = d[0] - c[0];
    let vy = d[1] - c[1];
    let det = x * vy - y * vx;
    if !det.is_finite() {
        return Err(input("Sketch intersection exceeds finite numeric range."));
    }
    if det.abs() < 1e-10 {
        return Ok(None);
    }
    let t = ((c[0] - a[0]) * vy - (c[1] - a[1]) * vx) / det;
    let u = ((c[0] - a[0]) * y - (c[1] - a[1]) * x) / det;
    if !t.is_finite() || !u.is_finite() {
        return Err(input("Sketch intersection exceeds finite numeric range."));
    }
    Ok(Some((t, u)))
}
fn boundaries(v: &Value) -> Result<Vec<(String, Vec<P>, usize)>> {
    let raw: Vec<Value> = field(v, "boundaries")?;
    let mut result = Vec::new();
    let mut count = 0usize;
    for s in raw {
        let p = points(&s)?;
        let closed: bool = field(&s, "closed")?;
        let edges = if closed { p.len() } else { p.len() - 1 };
        count = count.saturating_add(edges);
        if count > 131072 {
            return Err(input("Sketch boundary budget exceeded."));
        }
        result.push((field(&s, "id")?, p, edges));
    }
    Ok(result)
}
fn along(a: P, b: P, t: f64) -> Result<P> {
    let p = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
    if p.iter().all(|x| x.is_finite()) {
        Ok(p)
    } else {
        Err(input("Sketch intersection exceeds finite numeric range."))
    }
}
pub fn trim(v: Value) -> Result<Value> {
    let sketch: Value = field(&v, "sketch")?;
    let p = points(&sketch)?;
    let closed: bool = field(&sketch, "closed")?;
    let id: String = field(&sketch, "id")?;
    let edge: usize = field(&v, "edge")?;
    let at: f64 = field(&v, "at")?;
    let n = p.len();
    if edge >= if closed { n } else { n - 1 } || !at.is_finite() || !(0. ..=1.).contains(&at) {
        return Err(input("Select an edge to trim."));
    }
    let a = p[edge];
    let b = p[(edge + 1) % n];
    let mut cuts = vec![0., 1.];
    for (other, q, edges) in boundaries(&v)? {
        for i in 0..edges {
            if other == id && i == edge {
                continue;
            }
            if let Some((t, u)) = hit(a, b, q[i], q[(i + 1) % q.len()])? {
                if t > 1e-8 && t < 1. - 1e-8 && (0. ..=1.).contains(&u) {
                    cuts.push(t);
                }
            }
        }
    }
    cuts.sort_by(f64::total_cmp);
    let hi = cuts.iter().copied().find(|&t| t > at + 1e-8).unwrap_or(1.);
    let lo = cuts
        .iter()
        .rev()
        .copied()
        .find(|&t| t < at - 1e-8)
        .unwrap_or(0.);
    let chains = if closed {
        let mut chain = vec![along(a, b, hi)?];
        for k in 1..=n {
            chain.push(p[(edge + k) % n]);
        }
        chain.push(along(a, b, lo)?);
        vec![chain]
    } else {
        let mut left = p[..=edge].to_vec();
        left.push(along(a, b, lo)?);
        let mut right = vec![along(a, b, hi)?];
        right.extend_from_slice(&p[edge + 1..]);
        vec![left, right]
    };
    let mut result = Vec::new();
    for chain in chains {
        let points: Vec<P> = chain
            .iter()
            .enumerate()
            .filter(|(i, p)| {
                *i == 0 || (p[0] - chain[i - 1][0]).hypot(p[1] - chain[i - 1][1]) > 1e-8
            })
            .map(|(_, p)| *p)
            .collect();
        if points.len() < 2 {
            continue;
        }
        if points.len() > 512 {
            return Err(input("Trim result exceeds 512 points."));
        }
        let mut s = sketch
            .as_object()
            .ok_or_else(|| input("Expected sketch record"))?
            .clone();
        s.remove("analytic");
        s.insert(
            "id".into(),
            encode(if result.is_empty() {
                id.clone()
            } else {
                format!("{id}-trim")
            })?,
        );
        s.insert("closed".into(), encode(false)?);
        s.insert("points".into(), encode(points)?);
        result.push(Value::Object(s));
    }
    encode(result)
}
pub fn extend(v: Value) -> Result<Value> {
    let mut sketch: Value = field(&v, "sketch")?;
    if field::<bool>(&sketch, "closed")? || sketch.get("analytic").is_some() {
        return Err(input("Extend requires an open polyline."));
    }
    let mut p = points(&sketch)?;
    let id: String = field(&sketch, "id")?;
    let end: String = field(&v, "end")?;
    if end != "start" && end != "end" {
        return Err(input("Select a polyline endpoint."));
    }
    let i = if end == "start" { 0 } else { p.len() - 1 };
    let j = if end == "start" { 1 } else { p.len() - 2 };
    let a = p[j];
    let b = p[i];
    let mut best = f64::INFINITY;
    for (other, q, edges) in boundaries(&v)? {
        if other == id {
            continue;
        }
        for k in 0..edges {
            if let Some((t, u)) = hit(a, b, q[k], q[(k + 1) % q.len()])? {
                if t > 1. + 1e-8 && t < best && (0. ..=1.).contains(&u) {
                    best = t;
                }
            }
        }
    }
    if !best.is_finite() {
        return Err(input("No boundary intersects the extended ray."));
    }
    p[i] = along(a, b, best)?;
    cad_sketch_offset::validate_request(value_codec::json!({"points":p,"closed":false}))?;
    sketch["points"] = encode(p)?;
    Ok(sketch)
}
