use crate::{Error, Result};
use serde_json::{json, Value};
pub type Point = [f64; 2];
pub fn validate(points: &[Point], path: &str) -> Result<()> {
    let minx = points.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let maxx = points
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let miny = points.iter().map(|p| p[1]).fold(f64::INFINITY, f64::min);
    let maxy = points
        .iter()
        .map(|p| p[1])
        .fold(f64::NEG_INFINITY, f64::max);
    let epsilon = f64::EPSILON.max((maxx - minx + maxy - miny).powi(2) * 1e-12);
    let cross = |a: Point, b: Point, c: Point| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let on = |a: Point, b: Point, p: Point| {
        cross(a, b, p).abs() <= epsilon
            && p[0] >= a[0].min(b[0])
            && p[0] <= a[0].max(b[0])
            && p[1] >= a[1].min(b[1])
            && p[1] <= a[1].max(b[1])
    };
    let mut area = 0.;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        if a == b {
            return Err(Error::new(
                "invalid_profile",
                format!("{path}/points/{i}"),
                "Consecutive points must differ; omit the repeated closing point.",
            ));
        }
        area += cross(points[0], a, b);
        for j in i + 2..points.len() {
            if i == 0 && j == points.len() - 1 {
                continue;
            }
            let (c, d) = (points[j], points[(j + 1) % points.len()]);
            if (cross(a, b, c) * cross(a, b, d) < 0. && cross(c, d, a) * cross(c, d, b) < 0.)
                || on(a, b, c)
                || on(a, b, d)
                || on(c, d, a)
                || on(c, d, b)
            {
                return Err(Error::new(
                    "invalid_profile",
                    format!("{path}/points"),
                    format!("Polygon edges {i} and {j} intersect."),
                ));
            }
        }
    }
    if area.abs() <= epsilon {
        return Err(Error::new(
            "invalid_profile",
            format!("{path}/points"),
            "Polygon must enclose nonzero area.",
        ));
    }
    Ok(())
}
pub struct Section {
    pub z: f64,
    pub scale: Point,
    pub offset: Point,
}
pub fn loft(mut profile: Vec<Point>, sections: &[Section], path: &str) -> Result<Value> {
    let err = |s: &str| Error::new("invalid_loft", path, s);
    validate(&profile, "profile").map_err(|e| err(&e.message))?;
    let area: f64 = profile
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let q = profile[(i + 1) % profile.len()];
            p[0] * q[1] - q[0] * p[1]
        })
        .sum();
    if area < 0. {
        profile.reverse();
    }
    let n = profile.len();
    for i in 0..n {
        let (a, b, c) = (profile[i], profile[(i + 1) % n], profile[(i + 2) % n]);
        if (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0]) <= 0. {
            return Err(err(
                "Loft profile must be strictly convex; omit collinear vertices.",
            ));
        }
    }
    for (i, s) in sections.iter().enumerate() {
        if ![s.z, s.scale[0], s.scale[1], s.offset[0], s.offset[1]]
            .iter()
            .all(|v| v.is_finite())
            || s.scale.iter().any(|v| *v <= 0.)
        {
            return Err(err(
                "Loft sections require finite values and positive scales.",
            ));
        }
        if i > 0 && s.z <= sections[i - 1].z {
            return Err(err("Loft section Z values must strictly increase."));
        }
    }
    let points: Vec<[f64; 3]> = sections
        .iter()
        .flat_map(|s| {
            profile.iter().map(move |p| {
                [
                    p[0] * s.scale[0] + s.offset[0],
                    p[1] * s.scale[1] + s.offset[1],
                    s.z,
                ]
            })
        })
        .collect();
    if points
        .iter()
        .flatten()
        .any(|v| !v.is_finite() || v.abs() > 1e6)
    {
        return Err(err("Loft coordinates exceed numeric limits."));
    }
    let mut faces = Vec::with_capacity(2 * (n - 2) + 2 * n * (sections.len() - 1));
    let last = (sections.len() - 1) * n;
    for i in 1..n - 1 {
        faces.push([0, i, i + 1]);
        faces.push([last, last + i + 1, last + i]);
    }
    for k in 0..sections.len() - 1 {
        for i in 0..n {
            let (a, b) = (k * n + i, k * n + (i + 1) % n);
            faces.push([a, b + n, b]);
            faces.push([a, a + n, b + n]);
        }
    }
    Ok(json!({"points":points,"faces":faces}))
}
