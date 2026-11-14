//! Triangle-triangle intersection (exact signs) and closest distance.

use super::P;
use super::predicates::{orient2d, orient3d};
use mesh_query::closest_triangle;
use std::cmp::Ordering::{self, Equal, Greater, Less};

fn mixed(signs: &[Ordering]) -> bool {
    signs.contains(&Greater) && signs.contains(&Less)
}

/// Coordinate axis to drop when projecting triangle `t` to 2D.
fn drop_axis(t: &[P; 3]) -> usize {
    let u: [f64; 3] = std::array::from_fn(|k| t[1][k] - t[0][k]);
    let v: [f64; 3] = std::array::from_fn(|k| t[2][k] - t[0][k]);
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    (0..3)
        .max_by(|&a, &b| n[a].abs().total_cmp(&n[b].abs()))
        .unwrap_or(2)
}

fn segments_cross_2d(p: P, q: P, r: P, s: P, axis: usize) -> bool {
    let o = [
        orient2d(p, q, r, axis),
        orient2d(p, q, s, axis),
        orient2d(r, s, p, axis),
        orient2d(r, s, q, axis),
    ];
    if o.iter().all(|s| s.is_eq()) {
        // Collinear: overlap of the coordinate intervals on both kept axes.
        return (0..3)
            .filter(|&k| k != axis)
            .all(|k| p[k].min(q[k]) <= r[k].max(s[k]) && r[k].min(s[k]) <= p[k].max(q[k]));
    }
    !(o[0] == o[1] && o[0] != Equal) && !(o[2] == o[3] && o[2] != Equal)
}

fn point_in_triangle_2d(p: P, t: &[P; 3], axis: usize) -> bool {
    !mixed(&[
        orient2d(t[0], t[1], p, axis),
        orient2d(t[1], t[2], p, axis),
        orient2d(t[2], t[0], p, axis),
    ])
}

fn segment_triangle_2d(p: P, q: P, t: &[P; 3], axis: usize) -> bool {
    point_in_triangle_2d(p, t, axis)
        || point_in_triangle_2d(q, t, axis)
        || (0..3).any(|i| segments_cross_2d(p, q, t[i], t[(i + 1) % 3], axis))
}

fn coplanar_triangles(a: &[P; 3], b: &[P; 3]) -> bool {
    let axis = drop_axis(b);
    (0..3).any(|i| segment_triangle_2d(a[i], a[(i + 1) % 3], b, axis))
        || point_in_triangle_2d(b[0], a, axis)
}

/// Closed segment `pq` against closed non-degenerate triangle `t`.
pub(crate) fn segment_triangle(p: P, q: P, t: &[P; 3]) -> bool {
    let sp = orient3d(t[0], t[1], t[2], p);
    let sq = orient3d(t[0], t[1], t[2], q);
    if sp == sq {
        return sp == Equal && segment_triangle_2d(p, q, t, drop_axis(t));
    }
    !mixed(&[
        orient3d(p, q, t[0], t[1]),
        orient3d(p, q, t[1], t[2]),
        orient3d(p, q, t[2], t[0]),
    ])
}

/// Closed non-degenerate triangles share at least one point.
pub(crate) fn triangles_intersect(a: &[P; 3], b: &[P; 3]) -> bool {
    let da = a.map(|p| orient3d(b[0], b[1], b[2], p));
    if !da.contains(&Less) && !da.contains(&Equal) || !da.contains(&Greater) && !da.contains(&Equal)
    {
        return false;
    }
    let db = b.map(|p| orient3d(a[0], a[1], a[2], p));
    if !db.contains(&Less) && !db.contains(&Equal) || !db.contains(&Greater) && !db.contains(&Equal)
    {
        return false;
    }
    if da.iter().all(|s| s.is_eq()) {
        return coplanar_triangles(a, b);
    }
    (0..3).any(|i| segment_triangle(a[i], a[(i + 1) % 3], b))
        || (0..3).any(|i| segment_triangle(b[i], b[(i + 1) % 3], a))
}

/// Edge `(u, v)` shared by both triangles with opposite corners `x` (of
/// one) and `y` (of the other): they overlap beyond the edge only when
/// coplanar and folded onto the same side.
pub(crate) fn edge_neighbours_overlap(u: P, v: P, x: P, y: P) -> bool {
    if orient3d(u, v, x, y) != Equal {
        return false;
    }
    let axis = drop_axis(&[u, v, x]);
    orient2d(u, v, x, axis) == orient2d(u, v, y, axis)
}

/// Triangles `[v, a1, a2]` and `[v, b1, b2]` sharing only corner `v`:
/// true when they meet anywhere besides `v`.
pub(crate) fn vertex_neighbours_overlap(v: P, a: [P; 2], b: [P; 2]) -> bool {
    let ta = [v, a[0], a[1]];
    let tb = [v, b[0], b[1]];
    // The opposite edges catch every contact away from `v` unless the two
    // triangles are coplanar, where the wedges at `v` can also overlap.
    if segment_triangle(a[0], a[1], &tb) || segment_triangle(b[0], b[1], &ta) {
        return true;
    }
    if b.iter().any(|&p| orient3d(v, a[0], a[1], p) != Equal) {
        return false;
    }
    let axis = drop_axis(&ta);
    let strictly_inside = |w: [P; 2], x: P| {
        let s = orient2d(v, w[0], w[1], axis);
        orient2d(v, w[0], x, axis) == s && orient2d(v, x, w[1], axis) == s
    };
    let same_ray = |p: P, q: P| {
        orient2d(v, p, q, axis) == Equal && (0..3).all(|k| (p[k] - v[k]) * (q[k] - v[k]) >= 0.0)
    };
    b.iter().any(|&x| strictly_inside(a, x))
        || a.iter().any(|&x| strictly_inside(b, x))
        || (same_ray(a[0], b[0]) && same_ray(a[1], b[1]))
        || (same_ray(a[0], b[1]) && same_ray(a[1], b[0]))
}

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: P, b: P) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// Squared distance between closed segments `p1q1` and `p2q2`.
fn segment_segment_sq(p1: P, q1: P, p2: P, q2: P) -> f64 {
    let d1 = sub(q1, p1);
    let d2 = sub(q2, p2);
    let r = sub(p1, p2);
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (s, t) = if a <= f64::EPSILON && e <= f64::EPSILON {
        (0.0, 0.0)
    } else if a <= f64::EPSILON {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= f64::EPSILON {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut s = if denom > 0.0 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut t = (b * s + f) / e;
            if t < 0.0 {
                t = 0.0;
                s = (-c / a).clamp(0.0, 1.0);
            } else if t > 1.0 {
                t = 1.0;
                s = ((b - c) / a).clamp(0.0, 1.0);
            }
            (s, t)
        }
    };
    let c1: P = std::array::from_fn(|k| p1[k] + d1[k] * s);
    let c2: P = std::array::from_fn(|k| p2[k] + d2[k] * t);
    let d = sub(c1, c2);
    dot(d, d)
}

/// Euclidean distance between two closed triangles (0 when they meet).
pub(crate) fn triangle_distance(a: &[P; 3], b: &[P; 3]) -> f64 {
    if triangles_intersect(a, b) {
        return 0.0;
    }
    let mut best = f64::INFINITY;
    for (x, y) in [(a, b), (b, a)] {
        for &p in x {
            let q = closest_triangle(p, y[0], y[1], y[2]);
            best = best.min(dot(sub(p, q), sub(p, q)));
        }
    }
    for i in 0..3 {
        for j in 0..3 {
            best = best.min(segment_segment_sq(
                a[i],
                a[(i + 1) % 3],
                b[j],
                b[(j + 1) % 3],
            ));
        }
    }
    best.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T: [P; 3] = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.]];

    #[test]
    fn piercing_and_separated_triangles() {
        let pierce = [[0.5, 0.5, -1.], [0.5, 0.5, 1.], [3., 3., 0.5]];
        assert!(triangles_intersect(&T, &pierce));
        let above = [[0., 0., 1.], [2., 0., 1.], [0., 2., 1.]];
        assert!(!triangles_intersect(&T, &above));
        assert!((triangle_distance(&T, &above) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn coplanar_overlap_and_touch() {
        let overlap = [[0.5, 0.5, 0.], [3., 0.5, 0.], [0.5, 3., 0.]];
        assert!(triangles_intersect(&T, &overlap));
        let apart = [[3., 3., 0.], [4., 3., 0.], [3., 4., 0.]];
        assert!(!triangles_intersect(&T, &apart));
        // Vertex exactly on the hypotenuse counts as contact (closed sets).
        let touch = [[1., 1., 0.], [3., 1., 0.], [3., 3., 0.]];
        assert!(triangles_intersect(&T, &touch));
    }

    #[test]
    fn neighbour_rules() {
        let (u, v) = ([0., 0., 0.], [1., 0., 0.]);
        assert!(!edge_neighbours_overlap(u, v, [0., 1., 0.], [0., -1., 0.]));
        assert!(!edge_neighbours_overlap(u, v, [0., 1., 0.], [0., 0., 1.]));
        assert!(edge_neighbours_overlap(u, v, [0., 1., 0.], [0.5, 2., 0.]));
        let o = [0.; 3];
        // Fan neighbours on either side of a plane: contact only at `o`.
        assert!(!vertex_neighbours_overlap(
            o,
            [[1., 0., 0.], [0., 1., 0.]],
            [[-1., 0., 0.], [0., -1., 0.]]
        ));
        // A spike from `o` through the other triangle's interior plane.
        assert!(vertex_neighbours_overlap(
            o,
            [[2., 0., 0.], [0., 2., 0.]],
            [[0.5, 0.5, 1.], [0.5, 0.5, -1.]]
        ));
        // Coplanar wedges overlapping at `o`.
        assert!(vertex_neighbours_overlap(
            o,
            [[2., 0., 0.], [0., 2., 0.]],
            [[2., 1., 0.], [1., 2., 0.]]
        ));
    }
}
