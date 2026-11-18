//! Booleans of two straight prisms that share an extrusion direction and
//! whose material is strictly separated (interleaved gear teeth with
//! backlash, keyed parts with clearance).
//!
//! A solid is a prism here when every face is either a planar cap
//! perpendicular to one direction `d` at one of two axial levels, or a wall
//! whose surface has exactly two control rows in a degree-1 direction, the
//! second row being the first translated by the same vector along `d` with
//! equal weights. Such a solid is `region x [h0, h1]`, and its walls project
//! onto the plane perpendicular to `d` as the rational curve of their first
//! row (the face trim only removes part of that curve, so the curve is a
//! conservative superset of the projected boundary).
//!
//! When the axial intervals overlap, the solids are disjoint exactly when
//! the regions are. Strict separation of the region boundaries is proved by
//! convex hulls of projected control points (positive weights keep each
//! rational span inside its hull), split recursively until a separating axis
//! of the two hulls clears a margin far above rounding. Disjoint boundaries
//! of two connected regions leave only nesting, which is excluded when each
//! region has a boundary point outside the other's bounding box. Every input
//! face is kept unchanged: union concatenates the bodies, difference returns
//! the first operand, intersection is empty.
use crate::{Error, Model, Result};
use nurbs_core::curve::Curve;

/// Separation margin and flatness, relative to the joint model scale.
const MARGIN: f64 = 1e-9;
/// Subdivision depth per curve pair and total hull tests.
const DEPTH: usize = 40;
const WORK: usize = 4_000_000;

type P2 = [f64; 2];

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: &[f64], b: &[f64]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

struct Prism {
    /// Unit extrusion direction (sign as found in the first wall).
    axis: [f64; 3],
    /// Axial interval [h0, h1] measured along `axis`.
    levels: [f64; 2],
    /// Wall profile rows as 3D curves (projected later).
    walls: Vec<Curve>,
    /// Points of the solid (vertices), for the nesting test.
    vertices: Vec<[f64; 3]>,
}

fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_BOOLEAN_UNPROVED", format!("Separated prism Boolean: {message}"))
}

fn scale(model: &Model) -> f64 {
    model
        .faces
        .iter()
        .flat_map(|f| f.surface.control_points.iter().flatten())
        .flat_map(|p| p.iter().map(|x| x.abs()))
        .fold(1., f64::max)
}

/// Straight-extrusion readings of a surface: (first row curve, translation)
/// for an extrusion in u and/or in v.
fn walls(surface: &nurbs_core::surface::Surface, tol: f64) -> Vec<(Curve, [f64; 3])> {
    let cp = &surface.control_points;
    let w = &surface.weights;
    let mut out = Vec::new();
    // Extrusion along v: every u-row has two points.
    if surface.degree_v == 1 && cp.iter().all(|row| row.len() == 2) && !surface.periodic_u {
        let t = sub(&cp[0][1], &cp[0][0]);
        let ok = cp.iter().zip(w).all(|(row, wr)| {
            let d = sub(&row[1], &row[0]);
            (0..3).all(|k| (d[k] - t[k]).abs() <= tol) && wr[0] == wr[1]
        });
        if ok {
            out.push((
                Curve {
                    degree: surface.degree_u,
                    knots: surface.knots_u.clone(),
                    control_points: cp.iter().map(|row| row[0].clone()).collect(),
                    weights: w.iter().map(|wr| wr[0]).collect(),
                    periodic: false,
                },
                t,
            ));
        }
    }
    // Extrusion along u: two rows.
    if surface.degree_u == 1 && cp.len() == 2 && !surface.periodic_v && cp[0].len() == cp[1].len() {
        let t = sub(&cp[1][0], &cp[0][0]);
        let ok = cp[0].iter().zip(&cp[1]).all(|(a, b)| {
            let d = sub(b, a);
            (0..3).all(|k| (d[k] - t[k]).abs() <= tol)
        }) && w[0] == w[1];
        if ok {
            out.push((
                Curve {
                    degree: surface.degree_v,
                    knots: surface.knots_v.clone(),
                    control_points: cp[0].clone(),
                    weights: w[0].clone(),
                    periodic: false,
                },
                t,
            ));
        }
    }
    out
}

fn p3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}

fn recognize(model: &Model, tol: f64, prefer: Option<[f64; 3]>) -> Option<Prism> {
    if model.bodies.len() != 1 || model.shells.len() != 1 || !model.bodies[0].inner_shells.is_empty() {
        return None;
    }
    if model
        .faces
        .iter()
        .any(|f| f.surface.weights.iter().flatten().any(|w| !(w.is_finite() && *w > 0.)))
    {
        return None;
    }
    let readings: Vec<_> = model.faces.iter().map(|f| walls(&f.surface, tol)).collect();
    // Candidate axes: translations of curved walls first (a flat 2x2 cap
    // also reads as an extrusion, in the wrong direction).
    // A preferred axis (the other operand's) is tried first: a box reads as
    // a prism along any of its three edges.
    let mut candidates: Vec<[f64; 3]> = prefer.into_iter().collect();
    for curved in [true, false] {
        for r in readings.iter().flatten() {
            if (r.0.degree > 1) == curved {
                let len = dot(r.1, r.1).sqrt();
                if len > 1e3 * tol {
                    let d = r.1.map(|x| x / len);
                    if !candidates.iter().any(|c| (dot(*c, d).abs() - 1.).abs() <= 1e-12) {
                        candidates.push(d);
                    }
                }
            }
        }
    }
    'axis: for axis in candidates.into_iter().take(8) {
        let level = |p: &[f64]| dot(axis, p3(p));
        let mut wall_rows: Vec<(Curve, [f64; 2])> = Vec::new();
        let mut caps: Vec<f64> = Vec::new();
        for (face, reading) in model.faces.iter().zip(&readings) {
            let along = reading.iter().find(|(_, t)| {
                let len = dot(*t, *t).sqrt();
                len > 1e3 * tol && (dot(axis, *t).abs() / len - 1.).abs() <= 1e-12
            });
            if let Some((curve, t)) = along {
                let h0 = level(&curve.control_points[0]);
                if curve.control_points.iter().any(|p| (level(p) - h0).abs() > tol) {
                    continue 'axis;
                }
                let h1 = h0 + dot(axis, *t);
                wall_rows.push((curve.clone(), [h0.min(h1), h0.max(h1)]));
                continue;
            }
            let pts: Vec<f64> = face.surface.control_points.iter().flatten().map(|p| level(p)).collect();
            if pts.iter().any(|x| (x - pts[0]).abs() > tol) {
                continue 'axis;
            }
            caps.push(pts[0]);
        }
        if wall_rows.is_empty() {
            continue;
        }
        let levels = wall_rows[0].1;
        if !(levels[1] - levels[0] > 1e3 * tol)
            || wall_rows.iter().any(|(_, l)| (0..2).any(|k| (l[k] - levels[k]).abs() > tol))
            || caps.iter().any(|h| (h - levels[0]).abs() > tol && (h - levels[1]).abs() > tol)
        {
            continue;
        }
        let vertices = model.vertices.iter().map(|v| v.point).collect();
        return Some(Prism { axis, levels, walls: wall_rows.into_iter().map(|w| w.0).collect(), vertices });
    }
    None
}

/// A 2D rational curve piece kept for subdivision.
struct Piece {
    curve: Curve,
    hull: Vec<P2>,
    lo: P2,
    hi: P2,
}

fn convex_hull(mut pts: Vec<P2>) -> Vec<P2> {
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() < 3 {
        return pts;
    }
    let cross = |o: P2, a: P2, b: P2| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut hull: Vec<P2> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let order: Vec<P2> = if pass == 0 { pts.clone() } else { pts.iter().rev().copied().collect() };
        for p in order {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0. {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    hull
}

fn piece(curve: Curve, frame: &[[f64; 3]; 2]) -> Piece {
    let pts: Vec<P2> = curve
        .control_points
        .iter()
        .map(|p| {
            let q = [p[0], p[1], p[2]];
            [dot(q, frame[0]), dot(q, frame[1])]
        })
        .collect();
    let mut lo = [f64::INFINITY; 2];
    let mut hi = [f64::NEG_INFINITY; 2];
    for p in &pts {
        for k in 0..2 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    Piece { curve, hull: convex_hull(pts), lo, hi }
}

/// Largest gap between two convex point sets along the edge normals of
/// either hull (separating axis theorem; exact for convex polygons).
fn gap(a: &[P2], b: &[P2]) -> f64 {
    let mut best = f64::NEG_INFINITY;
    for (x, y) in [(a, b), (b, a)] {
        let n = x.len();
        let edges = if n < 2 { 0 } else if n == 2 { 2 } else { n };
        for i in 0..edges {
            let (p, q) = (x[i % n], x[(i + 1) % n]);
            let e = [q[0] - p[0], q[1] - p[1]];
            let len = e[0].hypot(e[1]);
            if len == 0. {
                continue;
            }
            let axis = [-e[1] / len, e[0] / len];
            let proj = |s: &[P2]| {
                s.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |m, p| {
                    let v = p[0] * axis[0] + p[1] * axis[1];
                    [m[0].min(v), m[1].max(v)]
                })
            };
            let (u, v) = (proj(x), proj(y));
            best = best.max((v[0] - u[1]).max(u[0] - v[1]));
        }
    }
    if a.len() == 1 && b.len() == 1 {
        best = best.max((a[0][0] - b[0][0]).hypot(a[0][1] - b[0][1]));
    }
    best
}

fn halves(p: &Piece, frame: &[[f64; 3]; 2]) -> Option<[Piece; 2]> {
    let [lo, hi] = p.curve.domain();
    let [l, r] = p.curve.split(0.5 * (lo + hi)).ok()?;
    Some([piece(l, frame), piece(r, frame)])
}

fn separated(a: Piece, b: Piece, frame: &[[f64; 3]; 2], margin: f64, work: &mut usize) -> bool {
    let mut stack = vec![(a, b, 0usize)];
    while let Some((a, b, depth)) = stack.pop() {
        *work += 1;
        if *work > WORK {
            return false;
        }
        if (0..2).any(|k| a.lo[k] - b.hi[k] > margin || b.lo[k] - a.hi[k] > margin) {
            continue;
        }
        if gap(&a.hull, &b.hull) > margin {
            continue;
        }
        if depth >= DEPTH {
            return false;
        }
        let size = |p: &Piece| (p.hi[0] - p.lo[0]).max(p.hi[1] - p.lo[1]);
        if size(&a) >= size(&b) {
            let Some([l, r]) = halves(&a, frame) else { return false };
            stack.push((l, Piece { curve: b.curve.clone(), hull: b.hull.clone(), lo: b.lo, hi: b.hi }, depth + 1));
            stack.push((r, b, depth + 1));
        } else {
            let Some([l, r]) = halves(&b, frame) else { return false };
            stack.push((Piece { curve: a.curve.clone(), hull: a.hull.clone(), lo: a.lo, hi: a.hi }, l, depth + 1));
            stack.push((a, r, depth + 1));
        }
    }
    true
}

/// Separated straight prisms with a common extrusion direction. Returns
/// None when the operands are not such prisms or the separation is unproved.
pub(crate) fn try_separated_prism_boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    let s = scale(a).max(scale(b));
    let tol = MARGIN * s;
    let Some(pb) = recognize(b, tol, None) else {
        return Ok(None);
    };
    let Some(pa) = recognize(a, tol, Some(pb.axis)) else {
        return Ok(None);
    };
    // The first operand may also fix the axis of an ambiguous second one.
    let pb = match recognize(b, tol, Some(pa.axis)) {
        Some(p) => p,
        None => return Ok(None),
    };
    let c = dot(pa.axis, pb.axis);
    if (c.abs() - 1.).abs() > 1e-12 {
        return Ok(None);
    }
    let axis = pa.axis;
    let lb = if c > 0. { pb.levels } else { [-pb.levels[1], -pb.levels[0]] };
    // Axially apart prisms are left to the box test; touching ones refuse.
    if !(lb[0] < pa.levels[1] - tol && pa.levels[0] < lb[1] - tol) {
        return Ok(None);
    }
    // Plane frame perpendicular to the axis.
    let helper = if axis[0].abs() < 0.9 { [1., 0., 0.] } else { [0., 1., 0.] };
    let cross = |u: [f64; 3], v: [f64; 3]| [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
    let e1 = cross(axis, helper);
    let n1 = dot(e1, e1).sqrt();
    let e1 = e1.map(|x| x / n1);
    let e2 = cross(axis, e1);
    let frame = [e1, e2];
    let margin = 10. * tol;
    let wa: Vec<Piece> = pa.walls.iter().cloned().map(|c| piece(c, &frame)).collect();
    let wb: Vec<Piece> = pb.walls.iter().cloned().map(|c| piece(c, &frame)).collect();
    let mut work = 0usize;
    for p in &wa {
        for q in &wb {
            let copy = |x: &Piece| Piece { curve: x.curve.clone(), hull: x.hull.clone(), lo: x.lo, hi: x.hi };
            if !separated(copy(p), copy(q), &frame, margin, &mut work) {
                return Err(refuse("wall profiles are not certified apart"));
            }
        }
    }
    // Nesting: each region needs a vertex outside the other's box (the box
    // of all wall control points contains that region).
    let bbox = |w: &[Piece]| {
        w.iter().fold([[f64::INFINITY; 2], [f64::NEG_INFINITY; 2]], |m, p| {
            [[m[0][0].min(p.lo[0]), m[0][1].min(p.lo[1])], [m[1][0].max(p.hi[0]), m[1][1].max(p.hi[1])]]
        })
    };
    let outside = |vertices: &[[f64; 3]], b: [P2; 2]| {
        vertices.iter().any(|v| {
            let q = [dot(*v, e1), dot(*v, e2)];
            (0..2).any(|k| q[k] < b[0][k] - margin || q[k] > b[1][k] + margin)
        })
    };
    if !outside(&pa.vertices, bbox(&wb)) || !outside(&pb.vertices, bbox(&wa)) {
        return Err(refuse("one profile may nest inside the other"));
    }
    let model = match operation {
        "union" | "xor" => crate::boolean_support::separated_union(a, b)?,
        "difference" => a.clone(),
        "intersection" => Model::empty(a.tolerance_mm.max(b.tolerance_mm))?,
        _ => return Ok(None),
    };
    Ok(Some(model))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GearSpec, cuboid, gear, operations, transform};

    fn spur(backlash: f64) -> Model {
        gear(&GearSpec { module: 2., teeth: 20, height: 6., backlash, ..GearSpec::default() }).unwrap()
    }

    fn placed(model: &Model, degrees: f64, x: f64) -> Model {
        let (c, s) = (degrees.to_radians().cos(), degrees.to_radians().sin());
        transform::affine(model, [[c, -s, 0., x], [s, c, 0., 0.], [0., 0., 1., 0.], [0., 0., 0., 1.]]).unwrap()
    }

    #[test]
    fn meshing_gears_with_backlash_are_certified_apart() {
        let (a, b) = (spur(0.), placed(&spur(0.2), 9., 40.));
        let union = operations::boolean(&a, &b, "union").unwrap();
        union.validate().unwrap();
        assert_eq!(union.shells.len(), 2);
        assert_eq!(union.faces.len(), a.faces.len() + b.faces.len());
        assert_eq!(operations::boolean(&a, &b, "difference").unwrap().faces, a.faces);
        assert!(operations::boolean(&a, &b, "intersection").unwrap().is_empty());
        // The mirrored axis (one gear flipped over) is the same prism pair.
        let flipped = transform::affine(&b, [[1., 0., 0., 0.], [0., -1., 0., 0.], [0., 0., -1., 6.], [0., 0., 0., 1.]]).unwrap();
        assert!(try_separated_prism_boolean(&a, &flipped, "union").unwrap().is_some());
    }

    #[test]
    fn colliding_teeth_are_refused() {
        // No phase shift: tooth meets tooth on the line of centres.
        let (a, b) = (spur(0.), placed(&spur(0.2), 0., 40.));
        let e = try_separated_prism_boolean(&a, &b, "union").err().expect("collision certified");
        assert_eq!(e.code, "BREP_NURBS_BOOLEAN_UNPROVED");
    }

    #[test]
    fn a_gear_inside_a_block_is_not_mistaken_for_separated() {
        let small = gear(&GearSpec { module: 0.5, teeth: 10, height: 6., ..GearSpec::default() }).unwrap();
        let block = cuboid([-20., -20., -1.], [20., 20., 10.]).unwrap();
        let e = try_separated_prism_boolean(&block, &small, "difference").err().expect("nesting certified");
        assert!(e.message.contains("nest"), "{}", e.message);
    }
}
