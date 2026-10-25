//! Bounded local-coordinate trim and extension of polylines.
use math_core::{Error, Result};
pub type Point = [f64; 2];
type P = Point;
fn input(message: impl Into<String>) -> Error {
    Error::new("GEOMETRY_INVALID_INPUT", message)
}
#[derive(Clone, Debug)]
pub struct Boundary {
    pub id: String,
    pub points: Vec<Point>,
    pub closed: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum End {
    Start,
    End,
}
fn points(p: &[Point]) -> Result<()> {
    if !(2..=512).contains(&p.len()) || !p.iter().flatten().all(|x| x.is_finite()) {
        return Err(input("Invalid contour."));
    }
    Ok(())
}
fn boundaries(raw: &[Boundary]) -> Result<Vec<(&str, &[Point], usize)>> {
    let mut result = Vec::with_capacity(raw.len());
    let mut count = 0usize;
    for s in raw {
        points(&s.points)?;
        let edges = if s.closed {
            s.points.len()
        } else {
            s.points.len() - 1
        };
        count = count.saturating_add(edges);
        if count > 131072 {
            return Err(input("Sketch boundary budget exceeded."));
        }
        result.push((s.id.as_str(), s.points.as_slice(), edges));
    }
    Ok(result)
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
fn along(a: P, b: P, t: f64) -> Result<P> {
    let p = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
    if p.iter().all(|x| x.is_finite()) {
        Ok(p)
    } else {
        Err(input("Sketch intersection exceeds finite numeric range."))
    }
}
pub fn trim(
    p: &[Point],
    closed: bool,
    id: &str,
    edge: usize,
    at: f64,
    raw: &[Boundary],
) -> Result<Vec<Vec<Point>>> {
    points(p)?;
    let n = p.len();
    if edge >= if closed { n } else { n - 1 } || !at.is_finite() || !(0. ..=1.).contains(&at) {
        return Err(input("Select an edge to trim."));
    }
    let a = p[edge];
    let b = p[(edge + 1) % n];
    let mut cuts = vec![0., 1.];
    for (other, q, edges) in boundaries(raw)? {
        for i in 0..edges {
            if other == id && i == edge {
                continue;
            }
            if let Some((t, u)) = hit(a, b, q[i], q[(i + 1) % q.len()])?
                && t > 1e-8
                && t < 1. - 1e-8
                && (0. ..=1.).contains(&u)
            {
                cuts.push(t);
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
        result.push(points);
    }
    Ok(result)
}
pub fn extend(p: &[Point], id: &str, end: End, raw: &[Boundary]) -> Result<Vec<Point>> {
    points(p)?;
    let mut p = p.to_vec();
    let i = if end == End::Start { 0 } else { p.len() - 1 };
    let j = if end == End::Start { 1 } else { p.len() - 2 };
    let a = p[j];
    let b = p[i];
    let mut best = f64::INFINITY;
    for (other, q, edges) in boundaries(raw)? {
        if other == id {
            continue;
        }
        for k in 0..edges {
            if let Some((t, u)) = hit(a, b, q[k], q[(k + 1) % q.len()])?
                && t > 1. + 1e-8
                && t < best
                && (0. ..=1.).contains(&u)
            {
                best = t;
            }
        }
    }
    if !best.is_finite() {
        return Err(input("No boundary intersects the extended ray."));
    }
    p[i] = along(a, b, best)?;
    crate::sketch_offset::validate(&p, false)?;
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trims_open_chain_and_extends_to_nearest_boundary() {
        let p = [[0., 0.], [10., 0.]];
        let boundaries = vec![Boundary {
            id: "cut".into(),
            points: vec![[4., -1.], [4., 1.]],
            closed: false,
        }];
        assert_eq!(
            trim(&p, false, "source", 0, 0.8, &boundaries).unwrap(),
            vec![vec![[0., 0.], [4., 0.]]]
        );
        let boundaries = vec![
            Boundary {
                id: "near".into(),
                points: vec![[12., -1.], [12., 1.]],
                closed: false,
            },
            Boundary {
                id: "far".into(),
                points: vec![[20., -1.], [20., 1.]],
                closed: false,
            },
        ];
        assert_eq!(
            extend(&p, "source", End::End, &boundaries).unwrap(),
            vec![[0., 0.], [12., 0.]]
        );
        assert!(trim(&p, false, "source", 1, 0.5, &boundaries).is_err());
    }
    #[test]
    fn rejects_nonfinite_boundaries_and_preserves_inputs() {
        let p = [[0., 0.], [10., 0.]];
        let boundaries = vec![Boundary {
            id: "bad".into(),
            points: vec![[f64::NAN, 0.], [0., 1.]],
            closed: false,
        }];
        assert!(extend(&p, "source", End::Start, &boundaries).is_err());
        assert_eq!(p, [[0., 0.], [10., 0.]]);
    }
}
