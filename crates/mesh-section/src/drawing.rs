//! Orthographic drawing views of a triangle mesh: hidden-line removal,
//! silhouettes, hatched sections and overhang detection.
//!
//! Edges drawn are boundary, non-manifold and sharp feature edges (dihedral
//! angle above a threshold, so coplanar triangulation edges vanish) plus
//! silhouettes, where the facing sign of the two adjacent triangles changes.
//! An edge-on triangle counts as facing away, so a curved surface yields one
//! silhouette per side. Each edge is split at the projected crossings with
//! triangle edges and every piece is depth-tested at its midpoint: a triangle
//! occludes only when the point lies strictly inside its projection (by a
//! margin) and the triangle is nearer by more than the tolerance. Coplanar and
//! touching faces therefore never hide an edge. Collinear overlapping pieces
//! are merged with visible winning over hidden. Output is sorted.

use crate::{MeshView, Result, slice};
use math_core::{cross, sub};
use planar_geometry::rings::{Rings, area};
use std::collections::BTreeMap;

fn check(condition: bool, message: &str) -> Result<()> {
    math_core::ensure(condition, "MESH_DRAWING_INVALID_INPUT", message)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Front,
    Top,
    Right,
    Iso,
}

impl View {
    pub fn parse(name: &str) -> Result<Self> {
        match name {
            "front" => Ok(Self::Front),
            "top" => Ok(Self::Top),
            "right" => Ok(Self::Right),
            "iso" => Ok(Self::Iso),
            _ => check(false, "Unknown drawing view").map(|()| Self::Front),
        }
    }
    /// Screen axes `u`, `v` (v up) and direction `d` toward the viewer; `u x v = d`.
    pub fn frame(self) -> [[f64; 3]; 3] {
        match self {
            Self::Front => [[1., 0., 0.], [0., 0., 1.], [0., -1., 0.]],
            Self::Top => [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            Self::Right => [[0., 1., 0.], [0., 0., 1.], [1., 0., 0.]],
            Self::Iso => {
                let (a, b, c) = (1. / 2f64.sqrt(), 1. / 6f64.sqrt(), 1. / 3f64.sqrt());
                [[a, a, 0.], [-b, b, 2. * b], [c, -c, c]]
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DrawingSegment {
    pub a: [f64; 2],
    pub b: [f64; 2],
    pub hidden: bool,
    pub silhouette: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ViewOptions {
    /// Dihedral angle (degrees) above which a shared edge is a feature edge.
    pub feature_angle_deg: f64,
    /// Depth and containment tolerance in model units.
    pub tolerance: f64,
    /// Maximum edge/triangle tests.
    pub max_work: usize,
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self {
            feature_angle_deg: 30.,
            tolerance: 1e-6,
            max_work: 20_000_000,
        }
    }
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn key(p: [f64; 3]) -> [u64; 3] {
    p.map(|x| if x == 0. { 0 } else { x.to_bits() })
}

struct Welded {
    points: Vec<[f64; 3]>,
    triangles: Vec<[usize; 3]>,
}

fn weld(mesh: &MeshView<'_>) -> Result<Welded> {
    mesh.validate()?;
    let mut ids = BTreeMap::new();
    let mut points = Vec::new();
    let mut triangles = Vec::new();
    for t in mesh.indices.as_chunks::<3>().0 {
        let mut w = [0; 3];
        for k in 0..3 {
            let p = mesh.point(t[k])?;
            w[k] = *ids.entry(key(p)).or_insert_with(|| {
                points.push(p);
                points.len() - 1
            });
        }
        if w[0] != w[1] && w[1] != w[2] && w[0] != w[2] {
            triangles.push(w);
        }
    }
    Ok(Welded { points, triangles })
}

fn normal(p: &[[f64; 3]], t: [usize; 3]) -> Option<[f64; 3]> {
    let n = cross(sub(p[t[1]], p[t[0]]), sub(p[t[2]], p[t[0]]));
    let l = dot(n, n).sqrt();
    (l > 1e-300).then(|| n.map(|x| x / l))
}

/// Visible and hidden edges of `mesh` in the given orthographic view.
pub fn view_edges(
    mesh: &MeshView<'_>,
    view: View,
    options: ViewOptions,
) -> Result<Vec<DrawingSegment>> {
    check(
        options.feature_angle_deg.is_finite()
            && (0. ..180.).contains(&options.feature_angle_deg)
            && options.tolerance.is_finite()
            && options.tolerance > 0.,
        "Invalid drawing options",
    )?;
    let w = weld(mesh)?;
    let [u, v, d] = view.frame();
    let screen: Vec<[f64; 3]> = w
        .points
        .iter()
        .map(|&p| [dot(p, u), dot(p, v), dot(p, d)])
        .collect();
    let normals: Vec<Option<[f64; 3]>> =
        w.triangles.iter().map(|&t| normal(&w.points, t)).collect();
    let mut edges: BTreeMap<(usize, usize), Vec<usize>> = BTreeMap::new();
    for (i, t) in w.triangles.iter().enumerate() {
        for k in 0..3 {
            let (a, b) = (t[k], t[(k + 1) % 3]);
            edges.entry((a.min(b), a.max(b))).or_default().push(i);
        }
    }
    let cos_feature = options.feature_angle_deg.to_radians().cos();
    let facing_eps = 1e-9;
    let mut candidates = Vec::new();
    for (&(a, b), tris) in &edges {
        let ns: Vec<_> = tris.iter().filter_map(|&t| normals[t]).collect();
        let (feature, silhouette) = if ns.len() != 2 || tris.len() != 2 {
            (true, false)
        } else {
            let front = |n: [f64; 3]| dot(n, d) > facing_eps;
            (
                dot(ns[0], ns[1]) < cos_feature,
                front(ns[0]) != front(ns[1]),
            )
        };
        if feature || silhouette {
            candidates.push((a, b, silhouette && !feature));
        }
    }
    let tris2: Vec<[[f64; 3]; 3]> = w.triangles.iter().map(|t| t.map(|i| screen[i])).collect();
    let boxes: Vec<[f64; 4]> = tris2
        .iter()
        .map(|t| {
            let f = |k: usize, m: fn(f64, f64) -> f64| m(m(t[0][k], t[1][k]), t[2][k]);
            [
                f(0, f64::min),
                f(1, f64::min),
                f(0, f64::max),
                f(1, f64::max),
            ]
        })
        .collect();
    let tol = options.tolerance;
    let mut work = 0usize;
    let mut pieces = Vec::new();
    for (a, b, silhouette) in candidates {
        let (p, q) = (screen[a], screen[b]);
        let len = (q[0] - p[0]).hypot(q[1] - p[1]);
        if len <= tol {
            continue; // edge seen end-on
        }
        let bb = [
            p[0].min(q[0]) - tol,
            p[1].min(q[1]) - tol,
            p[0].max(q[0]) + tol,
            p[1].max(q[1]) + tol,
        ];
        let near: Vec<usize> = (0..tris2.len())
            .filter(|&i| {
                let c = boxes[i];
                !(c[2] < bb[0] || c[0] > bb[2] || c[3] < bb[1] || c[1] > bb[3])
            })
            .collect();
        work = work.saturating_add(near.len() * 2);
        check(
            work <= options.max_work,
            "Drawing hidden-line work budget exceeded",
        )?;
        let mut ts = vec![0., 1.];
        let r = [q[0] - p[0], q[1] - p[1]];
        for &i in &near {
            let t = tris2[i];
            for k in 0..3 {
                let (s0, s1) = (t[k], t[(k + 1) % 3]);
                let e = [s1[0] - s0[0], s1[1] - s0[1]];
                let den = r[0] * e[1] - r[1] * e[0];
                if den.abs() < 1e-14 * len * e[0].hypot(e[1]).max(1e-300) {
                    continue;
                }
                let w0 = [s0[0] - p[0], s0[1] - p[1]];
                let s = (w0[0] * e[1] - w0[1] * e[0]) / den;
                let g = (w0[0] * r[1] - w0[1] * r[0]) / den;
                if s > 0. && s < 1. && (-1e-12..=1. + 1e-12).contains(&g) {
                    ts.push(s);
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y) * len <= tol * 0.5);
        if let Some(last) = ts.last_mut() {
            *last = 1.;
        }
        for pair in ts.windows(2) {
            let m = (pair[0] + pair[1]) * 0.5;
            let pt = [
                p[0] + (q[0] - p[0]) * m,
                p[1] + (q[1] - p[1]) * m,
                p[2] + (q[2] - p[2]) * m,
            ];
            let hidden = near.iter().any(|&i| occludes(&tris2[i], pt, tol));
            let at = |s: f64| [p[0] + r[0] * s, p[1] + r[1] * s];
            pieces.push(DrawingSegment {
                a: at(pair[0]),
                b: at(pair[1]),
                hidden,
                silhouette,
            });
        }
    }
    Ok(merge(pieces, tol))
}

fn occludes(t: &[[f64; 3]; 3], p: [f64; 3], tol: f64) -> bool {
    let [a, b, c] = *t;
    let det = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    let scale = (b[0] - a[0])
        .hypot(b[1] - a[1])
        .max((c[0] - a[0]).hypot(c[1] - a[1]));
    if det.abs() <= 1e-12 * scale * scale.max(1.) {
        return false; // edge-on triangle
    }
    let s = ((p[0] - a[0]) * (c[1] - a[1]) - (p[1] - a[1]) * (c[0] - a[0])) / det;
    let r = ((b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])) / det;
    // Distance-like margin: barycentric coordinates scaled by edge heights.
    let heights = [
        det.abs() / (b[0] - c[0]).hypot(b[1] - c[1]),
        det.abs() / (c[0] - a[0]).hypot(c[1] - a[1]),
        det.abs() / (b[0] - a[0]).hypot(b[1] - a[1]),
    ];
    let bary = [1. - s - r, s, r];
    if (0..3).any(|k| bary[k] * heights[k] <= tol) {
        return false;
    }
    let depth = a[2] + s * (b[2] - a[2]) + r * (c[2] - a[2]);
    depth > p[2] + tol
}

type Group = ([f64; 2], [f64; 2], Vec<(f64, f64, bool, bool)>);

fn merge(pieces: Vec<DrawingSegment>, tol: f64) -> Vec<DrawingSegment> {
    // Group collinear pieces; within a group, visible union wins over hidden.
    let mut groups: Vec<Group> = Vec::new();
    for s in pieces {
        let len = (s.b[0] - s.a[0]).hypot(s.b[1] - s.a[1]);
        if len <= tol {
            continue;
        }
        let mut dir = [(s.b[0] - s.a[0]) / len, (s.b[1] - s.a[1]) / len];
        if dir[0] < -1e-12 || (dir[0].abs() <= 1e-12 && dir[1] < 0.) {
            dir = [-dir[0], -dir[1]];
        }
        let off = |o: [f64; 2], dir: [f64; 2], p: [f64; 2]| {
            (p[0] - o[0]) * -dir[1] + (p[1] - o[1]) * dir[0]
        };
        let group = groups.iter_mut().find(|(o, gd, _)| {
            (gd[0] * dir[1] - gd[1] * dir[0]).abs() < 1e-9
                && off(*o, *gd, s.a).abs() <= tol
                && off(*o, *gd, s.b).abs() <= tol
        });
        let (o, gd, list) = match group {
            Some(g) => (g.0, g.1, &mut g.2),
            None => {
                groups.push((s.a, dir, Vec::new()));
                let g = groups.last_mut().expect("just pushed");
                (g.0, g.1, &mut g.2)
            }
        };
        let t = |p: [f64; 2]| (p[0] - o[0]) * gd[0] + (p[1] - o[1]) * gd[1];
        let (t0, t1) = (t(s.a).min(t(s.b)), t(s.a).max(t(s.b)));
        list.push((t0, t1, s.hidden, s.silhouette));
    }
    let union = |mut v: Vec<(f64, f64, bool)>| {
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut out: Vec<(f64, f64, bool)> = Vec::new();
        for (a, b, s) in v {
            match out.last_mut() {
                Some(l) if a <= l.1 + tol => {
                    l.1 = l.1.max(b);
                    l.2 &= s;
                }
                _ => out.push((a, b, s)),
            }
        }
        out
    };
    let mut result = Vec::new();
    for (o, d, list) in groups {
        let at = |t: f64| [o[0] + d[0] * t, o[1] + d[1] * t];
        let visible = union(
            list.iter()
                .filter(|x| !x.2)
                .map(|x| (x.0, x.1, x.3))
                .collect(),
        );
        let hidden = union(
            list.iter()
                .filter(|x| x.2)
                .map(|x| (x.0, x.1, x.3))
                .collect(),
        );
        for &(a, b, s) in &visible {
            result.push(DrawingSegment {
                a: at(a),
                b: at(b),
                hidden: false,
                silhouette: s,
            });
        }
        for (a, b, s) in hidden {
            let mut pieces = vec![(a, b)];
            for &(va, vb, _) in &visible {
                pieces = pieces
                    .into_iter()
                    .flat_map(|(x, y)| {
                        if vb <= x + tol || va >= y - tol {
                            vec![(x, y)]
                        } else {
                            [(x, va), (vb, y)]
                                .into_iter()
                                .filter(|(p, q)| q - p > tol)
                                .collect()
                        }
                    })
                    .collect();
            }
            for (x, y) in pieces {
                result.push(DrawingSegment {
                    a: at(x),
                    b: at(y),
                    hidden: true,
                    silhouette: s,
                });
            }
        }
    }
    let snap = |x: f64| {
        let r = (x * 1e9).round() / 1e9;
        if r == 0. { 0. } else { r }
    };
    for s in &mut result {
        s.a = s.a.map(snap);
        s.b = s.b.map(snap);
        if (s.b[0], s.b[1]) < (s.a[0], s.a[1]) {
            std::mem::swap(&mut s.a, &mut s.b);
        }
    }
    result.sort_by(|x, y| {
        (x.hidden, x.a[0], x.a[1], x.b[0], x.b[1])
            .partial_cmp(&(y.hidden, y.a[0], y.a[1], y.b[0], y.b[1]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    result
}

#[derive(Debug, Clone)]
pub struct HatchedSection {
    /// Closed rings in view screen coordinates: outer counter-clockwise, holes clockwise.
    pub rings: Rings,
    pub hatch: Vec<[[f64; 2]; 2]>,
    pub area: f64,
}

/// Cut `mesh` by the plane `p . d = offset` (d toward the viewer of `view`)
/// via [`slice`] and hatch the material at 45 degrees with `spacing`.
pub fn hatched_section(
    mesh: &MeshView<'_>,
    view: View,
    offset: f64,
    spacing: f64,
) -> Result<HatchedSection> {
    check(
        offset.is_finite() && spacing.is_finite() && spacing > 0.,
        "Invalid section parameters",
    )?;
    mesh.validate()?;
    let [u, v, d] = view.frame();
    let mut positions = Vec::with_capacity(mesh.positions.len());
    for p in mesh.positions.as_chunks::<3>().0 {
        positions.extend([dot(*p, u), dot(*p, v), dot(*p, d) - offset]);
    }
    let rings = slice(&MeshView::new(&positions, mesh.indices), 0.)?;
    let total: f64 = rings.iter().map(|r| area(r)).sum();
    // Even-odd scanlines along s = x + y.
    let s_of = |p: [f64; 2]| p[0] + p[1];
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for p in rings.iter().flatten() {
        lo = lo.min(s_of(*p));
        hi = hi.max(s_of(*p));
    }
    let mut hatch = Vec::new();
    if lo.is_finite() {
        let step = spacing * 2f64.sqrt();
        check((hi - lo) / step < 100_000., "Section hatch budget exceeded")?;
        let mut k = (lo / step).floor() as i64 + 1;
        while (k as f64) * step < hi {
            let s = k as f64 * step;
            let mut xs = Vec::new();
            for r in &rings {
                for i in 0..r.len() {
                    let (a, b) = (r[i], r[(i + 1) % r.len()]);
                    let (sa, sb) = (s_of(a), s_of(b));
                    if (sa > s) != (sb > s) {
                        let f = (s - sa) / (sb - sa);
                        xs.push(a[0] + (b[0] - a[0]) * f);
                    }
                }
            }
            xs.sort_by(f64::total_cmp);
            for pair in xs.as_chunks::<2>().0 {
                hatch.push([[pair[0], s - pair[0]], [pair[1], s - pair[1]]]);
            }
            k += 1;
        }
    }
    Ok(HatchedSection {
        rings,
        hatch,
        area: total,
    })
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverhangReport {
    pub triangles: Vec<usize>,
    pub area: f64,
    pub threshold_deg: f64,
}

/// Downward faces steeper than `threshold_deg` from vertical (+Z up), excluding
/// faces lying on the build plate (minimum Z within `tolerance`).
pub fn overhangs(
    mesh: &MeshView<'_>,
    threshold_deg: f64,
    tolerance: f64,
) -> Result<OverhangReport> {
    check(
        threshold_deg.is_finite()
            && (0. ..90.).contains(&threshold_deg)
            && tolerance.is_finite()
            && tolerance >= 0.,
        "Invalid overhang parameters",
    )?;
    mesh.validate()?;
    let mut min_z = f64::INFINITY;
    for p in mesh.positions.as_chunks::<3>().0 {
        min_z = min_z.min(p[2]);
    }
    let limit = (90. - threshold_deg).to_radians().cos();
    let mut report = OverhangReport {
        threshold_deg,
        ..Default::default()
    };
    for (i, t) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let p = [mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?];
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let l = dot(n, n).sqrt();
        if l <= 1e-300 || p.iter().all(|q| q[2] <= min_z + tolerance) {
            continue;
        }
        if -n[2] / l > limit + 1e-12 {
            report.triangles.push(i);
            report.area += l * 0.5;
        }
    }
    Ok(report)
}

#[cfg(test)]
#[path = "drawing_tests.rs"]
mod tests;
