//! A canonical cylinder drilled through planar caps of any other solid.
//!
//! The other operand may be arbitrary (involute gear teeth, freeform walls)
//! as long as every one of its faces that can reach the infinite cylinder is
//! a planar parallelogram cap perpendicular to the axis, strictly between
//! the cylinder's own caps, whose trimming boundary stays away from the
//! cylinder. Reach is decided conservatively from the convex hull of each
//! face's (or edge span's) control points projected along the axis, so the
//! intersection is exactly the set of circles where the wall crosses those
//! caps, and its completeness needs no tracing.
//!
//! Each circle is cut at the cylinder's quadrant seams into exact rational
//! 90-degree arcs (exact by construction, so no interval certificate); the
//! cap pcurve is the affine image of the arc and the
//! wall pcurve is fitted and certified against the cap plane. Material:
//! a cap point lies inside the cylinder by the canonical test, a cylinder
//! point inside the other solid by the parity of crossed caps above it along
//! the axis (no other face of the solid meets the cylinder).
use super::boolean::{inside_cylinder, network_end, refuse};
use super::traced_lift::Other;
use super::*;
use crate::implicit_trace::{PlaneImplicit, certify_lift};
use crate::sphere_cylinder::recognize_cylinder;
use brep_core::network_boolean::{NetworkPiece, network_boolean};
use nurbs_core::foundation::interpolation::{Parameterization, interpolate_curve};

/// Clearance between the cylinder and any face it is certified to miss.
const CLEARANCE: f64 = 1e-6;
/// Planarity and parallelogram tolerance, relative to the model scale.
const FLAT: f64 = 1e-9;

type P2 = [f64; 2];

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] + b[k])
}

/// A cap of the other solid that the cylinder pierces.
struct Cap {
    face: usize,
    /// Axial coordinate measured from the cylinder centre.
    z: f64,
    /// Affine inverse of the cap surface: uv = inv * (x - origin), on the
    /// two in-plane surface directions.
    origin: [f64; 3],
    du: [f64; 3],
    dv: [f64; 3],
}

impl Cap {
    fn uv(&self, p: [f64; 3]) -> P2 {
        // Solve p - origin = u du + v dv in the plane (Gram system).
        let d = sub(p, self.origin);
        let (a, b, c) = (dot(self.du, self.du), dot(self.du, self.dv), dot(self.dv, self.dv));
        let (r, s) = (dot(d, self.du), dot(d, self.dv));
        let det = a * c - b * b;
        [(c * r - b * s) / det, (a * s - b * r) / det]
    }
}

/// Distance from the origin to the convex hull of 2D points (zero inside).
fn hull_distance(points: &[P2]) -> f64 {
    let mut pts: Vec<P2> = points.to_vec();
    pts.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    pts.dedup();
    if pts.len() == 1 {
        return pts[0][0].hypot(pts[0][1]);
    }
    let cross = |o: P2, a: P2, b: P2| (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0]);
    let mut hull: Vec<P2> = Vec::new();
    for pass in 0..2 {
        let start = hull.len();
        let iter: Box<dyn Iterator<Item = &P2>> = if pass == 0 { Box::new(pts.iter()) } else { Box::new(pts.iter().rev()) };
        for &p in iter {
            while hull.len() >= start + 2 && cross(hull[hull.len() - 2], hull[hull.len() - 1], p) <= 0. {
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
    }
    let seg = |a: P2, b: P2| {
        let d = [b[0] - a[0], b[1] - a[1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let t = if l2 > 0. { (-(a[0] * d[0] + a[1] * d[1]) / l2).clamp(0., 1.) } else { 0. };
        (a[0] + t * d[0]).hypot(a[1] + t * d[1])
    };
    let n = hull.len();
    let inside = n >= 3 && (0..n).all(|i| cross(hull[i], hull[(i + 1) % n], [0., 0.]) >= 0.);
    if inside {
        return 0.;
    }
    (0..n).map(|i| seg(hull[i], hull[(i + 1) % n])).fold(f64::INFINITY, f64::min)
}

/// Winding number of the origin around a closed 2D polyline.
fn winding(polygon: &[P2]) -> i32 {
    let mut w = 0;
    for i in 0..polygon.len() {
        let (a, b) = (polygon[i], polygon[(i + 1) % polygon.len()]);
        let side = a[0] * b[1] - a[1] * b[0];
        if a[1] <= 0. {
            if b[1] > 0. && side > 0. {
                w += 1;
            }
        } else if b[1] <= 0. && side < 0. {
            w -= 1;
        }
    }
    w
}

/// Boolean of a canonical cylinder and a solid it drills straight through
/// planar caps (either operand order).
pub fn cylinder_drill_boolean(a: &Model, b: &Model, operation: &str) -> Result<Model> {
    let (c_index, cylinder) = match (recognize_cylinder(a)?, recognize_cylinder(b)?) {
        (Some(c), _) => (0, c),
        (None, Some(c)) => (1, c),
        (None, None) => return Err(refuse("Drill Boolean needs a canonical cylinder")),
    };
    let operands = [a, b];
    let (cyl_model, other) = (operands[c_index], operands[1 - c_index]);
    let frame = [cylinder.frame[0], cylinder.frame[1], cylinder.axis];
    let local = |p: &[f64]| -> [f64; 3] {
        let d = sub([p[0], p[1], p[2]], cylinder.center);
        [dot(d, frame[0]), dot(d, frame[1]), dot(d, frame[2])]
    };
    let scale = cylinder.radius.max(cylinder.half_height);
    let reach = cylinder.radius + CLEARANCE;
    let mut caps: Vec<Cap> = Vec::new();
    for (face_index, face) in other.faces.iter().enumerate() {
        let s = &face.surface;
        let projected: Vec<P2> = s.control_points.iter().flatten().map(|p| {
            let q = local(p);
            [q[0], q[1]]
        }).collect();
        if hull_distance(&projected) > reach {
            continue;
        }
        // The face can reach the cylinder: it must be a flat parallelogram
        // perpendicular to the axis, strictly between the cylinder's caps.
        let cp = &s.control_points;
        if s.degree_u != 1 || s.degree_v != 1 || cp.len() != 2 || cp.iter().any(|row| row.len() != 2) {
            return Err(refuse("Drill Boolean: a face near the cylinder is not a planar cap"));
        }
        let w = s.weights[0][0];
        if s.weights.iter().flatten().any(|&x| (x - w).abs() > FLAT * w.abs()) {
            return Err(refuse("Drill Boolean: a face near the cylinder is not a planar cap"));
        }
        let corner = |i: usize, j: usize| [cp[i][j][0], cp[i][j][1], cp[i][j][2]];
        let (p00, p10, p01, p11) = (corner(0, 0), corner(1, 0), corner(0, 1), corner(1, 1));
        let skew = sub(add(p10, p01), add(p00, p11));
        let zs: Vec<f64> = [p00, p10, p01, p11].iter().map(|p| local(p)[2]).collect();
        let z = zs[0];
        if dot(skew, skew).sqrt() > FLAT * scale || zs.iter().any(|&q| (q - z).abs() > FLAT * scale) {
            return Err(refuse("Drill Boolean: a face near the cylinder is not a planar cap"));
        }
        if z.abs() >= cylinder.half_height - CLEARANCE {
            return Err(refuse("Drill Boolean: a cap reaches the cylinder's own caps"));
        }
        let (u0, u1) = (s.knots_u[1], s.knots_u[2]);
        let (v0, v1) = (s.knots_v[1], s.knots_v[2]);
        let du = sub(p10, p00).map(|x| x / (u1 - u0));
        let dv = sub(p01, p00).map(|x| x / (v1 - v0));
        let origin = sub(sub(p00, du.map(|x| x * u0)), dv.map(|x| x * v0));
        // Trimming boundary clear of the cylinder, span by span.
        let mut polygon: Vec<P2> = Vec::new();
        let mut wind = 0;
        for wire in std::iter::once(&face.outer).chain(&face.holes) {
            polygon.clear();
            for coedge in &other.loops[*wire].coedges {
                let edge = &other.edges[coedge.edge];
                for span in edge.curve.decompose()? {
                    let hull: Vec<P2> = span.definition().control_points.iter().map(|p| {
                        let q = local(p);
                        [q[0], q[1]]
                    }).collect();
                    if hull_distance(&hull) <= reach {
                        return Err(refuse("Drill Boolean: a cap boundary meets the cylinder"));
                    }
                }
                let [lo, hi] = edge.curve.domain();
                let mut samples: Vec<P2> = (0..=64)
                    .map(|k| {
                        let p = edge.curve.evaluate(lo + (hi - lo) * k as f64 / 64.)?.point;
                        let q = local(&p);
                        Ok([q[0], q[1]])
                    })
                    .collect::<Result<_>>()?;
                if coedge.reversed {
                    samples.reverse();
                }
                polygon.extend(samples);
            }
            // Every boundary point is farther than the radius from the axis,
            // so the sampled winding is exact for the whole disk.
            wind += winding(&polygon).abs();
        }
        if wind % 2 == 1 {
            caps.push(Cap { face: face_index, z, origin, du, dv });
        }
    }
    let plane = |z: f64| PlaneImplicit {
        origin: std::array::from_fn(|k| cylinder.center[k] + z * cylinder.axis[k]),
        normal: cylinder.axis,
    };
    let side = Other::Cylinder(&cylinder);
    let mut pieces = Vec::new();
    let mut tolerance = 1e-6_f64;
    let mut free_id = 0;
    for cap in &caps {
        let at = |phi: f64| -> [f64; 3] {
            let (s, c) = phi.sin_cos();
            std::array::from_fn(|k| {
                cylinder.center[k]
                    + cylinder.radius * (c * frame[0][k] + s * frame[1][k])
                    + cap.z * frame[2][k]
            })
        };
        let q = std::f64::consts::FRAC_PI_2;
        for quadrant in 0..4 {
            let (a0, a1) = (quadrant as f64 * q, (quadrant + 1) as f64 * q);
            let (p0, p1) = (at(a0), at(a1));
            // Exact rational quadratic quarter circle: the middle control
            // point is where the end tangents meet.
            let mid = std::array::from_fn::<f64, 3, _>(|k| {
                cylinder.center[k] + cap.z * frame[2][k]
                    + cylinder.radius * ((a0.cos() + a1.cos()) * frame[0][k] + (a0.sin() + a1.sin()) * frame[1][k])
            });
            let w = std::f64::consts::FRAC_1_SQRT_2;
            let curve = Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![p0.to_vec(), mid.to_vec(), p1.to_vec()],
                weights: vec![1., w, 1.],
                periodic: false,
            };
            // The arc is exact by construction: the standard rational
            // quarter circle of the wall's radius, every control point in the
            // cap plane, so it needs no interval certificate.
            // Cap pcurve: the affine image of the arc, same weights.
            let cap_pcurve = Curve {
                control_points: curve
                    .control_points
                    .iter()
                    .map(|p| cap.uv([p[0], p[1], p[2]]).to_vec())
                    .collect(),
                ..curve.clone()
            };
            let mid_point = at(0.5 * (a0 + a1));
            let wall_face = side
                .patch(mid_point)
                .ok_or_else(|| refuse("Drill Boolean: arc lies on no cylinder patch"))?;
            let mut count = 17;
            let wall_pcurve = loop {
                let uvs = (0..count)
                    .map(|i| -> Result<[f64; 3]> {
                        let s = i as f64 / (count - 1) as f64;
                        let p = curve.evaluate(s)?.point;
                        let uv = side
                            .uv(wall_face, [p[0], p[1], p[2]])
                            .ok_or_else(|| refuse("Drill Boolean: arc point does not invert"))?;
                        Ok([uv[0], uv[1], 0.])
                    })
                    .collect::<Result<Vec<_>>>()?;
                let fitted = interpolate_curve(&uvs, 3, Parameterization::Uniform, None)?;
                let fitted = Curve {
                    control_points: fitted.control_points.iter().map(|p| vec![p[0], p[1]]).collect(),
                    ..fitted
                };
                if let Some(b) = certify_lift(
                    &cyl_model.faces[wall_face].surface,
                    &fitted,
                    &plane(cap.z),
                    LIFT_TOLERANCE,
                )? {
                    tolerance = tolerance.max(2. * b);
                    break fitted;
                }
                if count >= 129 {
                    return Err(refuse("Drill Boolean: a wall pcurve misses its tolerance"));
                }
                count = 2 * count - 1;
            };
            let mut faces = [0; 2];
            faces[c_index] = wall_face;
            faces[1 - c_index] = cap.face;
            let mut pcurves = [wall_pcurve.clone(), wall_pcurve];
            pcurves[1 - c_index] = cap_pcurve;
            pieces.push(NetworkPiece {
                faces,
                curve,
                pcurves,
                ends: [
                    network_end(operands, p0, &mut free_id)?,
                    network_end(operands, p1, &mut free_id)?,
                ],
            });
        }
    }
    let cap_heights: Vec<f64> = caps.iter().map(|c| c.z).collect();
    let inside = |o: usize, p: [f64; 3]| -> Result<bool> {
        if o == c_index {
            // A cylinder point is inside the other solid when an odd number
            // of pierced caps lie above it along the axis.
            let z = local(&p)[2];
            if cap_heights.iter().any(|&h| (h - z).abs() <= CLEARANCE) {
                return Err(refuse("Drill Boolean: region sample on a pierced cap"));
            }
            Ok(cap_heights.iter().filter(|&&h| h > z).count() % 2 == 1)
        } else {
            inside_cylinder(&cylinder, p)
        }
    };
    network_boolean(a, b, operation, &pieces, &inside, tolerance)
}
