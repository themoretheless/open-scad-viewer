//! Exact six-face affine box recognition, independent of redundant boundary
//! subdivisions. Inner ORIGINAL positive-weight pole hulls must lie strictly
//! inside that proved box; an outer bounding box alone grants no authority.
use crate::{Model, Result};
use nurbs_core::interval_eval::Interval as I;
use std::collections::BTreeSet;
fn bounds(model: &Model) -> [[f64; 2]; 3] {
    let mut out = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
        .map(|p| [p[0], p[1], p[2]])
        .chain(
            model
                .edges
                .iter()
                .flat_map(|e| e.curve.control_points.iter())
                .map(|p| [p[0], p[1], p[2]]),
        )
        .chain(model.vertices.iter().map(|v| v.point))
    {
        for k in 0..3 {
            out[k][0] = out[k][0].min(p[k]);
            out[k][1] = out[k][1].max(p[k]);
        }
    }
    out
}
fn exact_box(model: &Model) -> Result<Option<[[f64; 2]; 3]>> {
    if model.faces.len() != 6
        || model.shells.len() != 1
        || model.shells[0].faces.len() != 6
        || !model.shells[0].closed
    {
        return Ok(None);
    }
    // Outer edges may carry tolerance differences. The box itself is authored
    // by the six exact surface rectangles, not by approximate vertex extrema.
    let mut b = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for p in model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
    {
        for k in 0..3 {
            b[k][0] = b[k][0].min(p[k]);
            b[k][1] = b[k][1].max(p[k]);
        }
    }
    if b.iter().any(|d| d[0] >= d[1]) {
        return Ok(None);
    }
    let mut sides = BTreeSet::new();
    for usage in &model.shells[0].faces {
        let face = &model.faces[usage.face];
        let s = &face.surface;
        if !face.holes.is_empty()
            || s.degree_u != 1
            || s.degree_v != 1
            || s.control_points.len() != 2
            || s.control_points[0].len() != 2
            || s.knots_u.len() != 4
            || s.knots_v.len() != 4
            || s.knots_u[0] != s.knots_u[1]
            || s.knots_u[2] != s.knots_u[3]
            || s.knots_v[0] != s.knots_v[1]
            || s.knots_v[2] != s.knots_v[3]
            || s.periodic_u
            || s.periodic_v
            || s.weights.iter().flatten().any(|w| *w != s.weights[0][0])
        {
            return Ok(None);
        }
        let p = &s.control_points;
        let o = &p[0][0];
        let Some(a) =
            (0..3).find(|k| p[1][0][*k] != o[*k] && (0..3).all(|j| j == *k || p[1][0][j] == o[j]))
        else {
            return Ok(None);
        };
        let Some(c) =
            (0..3).find(|k| p[0][1][*k] != o[*k] && (0..3).all(|j| j == *k || p[0][1][j] == o[j]))
        else {
            return Ok(None);
        };
        if a == c {
            return Ok(None);
        }
        let fixed = 3 - a - c;
        if (0..3).any(|k| {
            p[1][1][k]
                != if k == a {
                    p[1][0][k]
                } else if k == c {
                    p[0][1][k]
                } else {
                    o[k]
                }
        }) {
            return Ok(None);
        }
        let natural = [[s.knots_u[1], s.knots_u[2]], [s.knots_v[1], s.knots_v[2]]];
        if crate::face_domain::FaceDomain::new(model, usage.face, 1e-8)?.literal_rectangle()
            != Some(natural)
        {
            return Ok(None);
        }
        if [a, c]
            .iter()
            .any(|k| o[*k].min(p[1][1][*k]) != b[*k][0] || o[*k].max(p[1][1][*k]) != b[*k][1])
        {
            return Ok(None);
        }
        let high = if o[fixed] == b[fixed][0] {
            false
        } else if o[fixed] == b[fixed][1] {
            true
        } else {
            return Ok(None);
        };
        let positive =
            ((a + 1) % 3 == c) ^ (p[1][0][a] < o[a]) ^ (p[0][1][c] < o[c]) ^ usage.reversed;
        if positive != high || !sides.insert((fixed, high)) {
            return Ok(None);
        }
    }
    Ok((sides.len() == 6).then_some(b))
}
pub(super) fn verify(outer: &Model, inner: &Model, tolerance: f64) -> Result<bool> {
    let Some(a) = exact_box(outer)? else {
        return Ok(false);
    };
    let b = bounds(inner);
    for k in 0..3 {
        if I::point(b[k][0]).sub(I::point(a[k][0]))?.lo <= tolerance
            || I::point(a[k][1]).sub(I::point(b[k][1]))?.lo <= tolerance
        {
            return Ok(false);
        }
    }
    Ok(true)
}
