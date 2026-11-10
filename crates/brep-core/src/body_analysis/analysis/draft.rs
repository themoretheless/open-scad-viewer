//! Draft analysis for direct-modeling face tweaks (checklist 873).
//!
//! [`draft_angle`] samples the face normal against a candidate rotation axis
//! (min/max angle over a surface grid) and, when the neighborhood is a
//! planar polygonal body and the axis lies in the face plane, predicts the
//! maximum admissible rotation in both directions *before* any mutation is
//! attempted.
//!
//! The limit is geometric, not a heuristic: every vertex of the rotated
//! (driven) coplanar region follows a known path.
//!
//! * A vertex with two non-driven incident faces slides along the fixed
//!   line `N1 ∩ N2`; its position at angle θ is `L ∩ P'(θ)`. The first
//!   event that invalidates the edit is either the slide line becoming
//!   parallel to the rotating plane (`dir·n(θ) = 0`) or the sliding vertex
//!   reaching another face plane `Q` — solved in closed form, because
//!   `w = L ∩ Q` is fixed and `w ∈ P'(θ)` reduces to `A cosθ + B sinθ = 0`.
//! * A vertex interior to the driven region rotates rigidly; crossing a
//!   fixed plane `Q` is `C + A cosθ + B sinθ = 0`, again closed form.
//!
//! Curved faces/neighbors, axes outside the face plane and corner
//! configurations with 1 or ≥3 non-driven incident faces are not predicted:
//! the corresponding `max_angle_*` fields are `None` and `tweak_face`
//! refuses those cases with its own typed error.

use super::surface_classify::Axis;
use crate::direct_edit::{
    PlaneEq, canonical_polygons, coplanar_component, cross, dot, edge_face_adjacency,
    face_plane_fit, intersect_planes, line_plane, mul, norm, sub, vertex_face_incidence,
};
use crate::operations::loop_vertices;
use crate::{Error, Model, Result};
use nurbs_core::foundation::guards::{Budget, BudgetGuard};
use nurbs_core::surface::SurfaceSampler;
use std::collections::{BTreeMap, BTreeSet};

/// Error code when the draft request itself is malformed.
pub const DRAFT_INVALID: &str = "BREP_DRAFT_INVALID";

fn invalid(message: impl Into<String>) -> Error {
    Error::new(DRAFT_INVALID, message.into())
}

/// One predicted limiting event, signed by the right-hand rule about
/// `axis.direction`.
#[derive(Clone, Debug)]
pub struct DraftLimit {
    /// Input-model vertex whose path hits the limit.
    pub vertex: usize,
    /// The face plane the vertex reaches; `None` when the slide line becomes
    /// parallel to the rotating plane (the intersection ceases to exist).
    pub limiting_face: Option<usize>,
    /// Signed angle of the event (radians), in (−π, π] \ {0}.
    pub angle_rad: f64,
}

/// Draft analysis of one face against a candidate tweak axis.
#[derive(Clone, Debug)]
pub struct DraftReport {
    pub face: usize,
    pub axis: Axis,
    /// The face fit is planar (a prerequisite for the rotation limits).
    pub planar: bool,
    /// Number of surface samples that produced a usable normal.
    pub samples: usize,
    /// Min/max angle between the sampled outward-agnostic normal and the
    /// axis direction, in [0, π/2] (acos of |n·k|).
    pub normal_axis_min_rad: f64,
    pub normal_axis_max_rad: f64,
    /// Maximum admissible rotation about +axis (right-hand rule), radians.
    /// `None` when the neighborhood is outside the planar prediction subset
    /// or no limiting event exists on (0, π].
    pub max_angle_positive_rad: Option<f64>,
    /// Maximum admissible rotation magnitude about −axis, radians.
    pub max_angle_negative_rad: Option<f64>,
    /// The events that set the two limits (for diagnostics).
    pub limit_positive: Option<DraftLimit>,
    pub limit_negative: Option<DraftLimit>,
}

fn knot_domain(knots: &[f64], degree: usize) -> [f64; 2] {
    let lo = knots.get(degree).copied().unwrap_or(0.);
    let hi = knots
        .len()
        .checked_sub(degree + 1)
        .and_then(|i| knots.get(i))
        .copied()
        .unwrap_or(1.);
    [lo, hi]
}

/// Sampled normal-to-axis angles over a 5×5 surface grid (clamped central
/// differences; degenerate normals are skipped).
fn normal_axis_angles(
    model: &Model,
    face: usize,
    axis: &Axis,
    guard: &mut BudgetGuard,
) -> Result<(usize, f64, f64)> {
    let surface = &model.faces[face].surface;
    let sampler = SurfaceSampler::new(surface)?;
    let du = knot_domain(&surface.knots_u, surface.degree_u);
    let dv = knot_domain(&surface.knots_v, surface.degree_v);
    let hu = (du[1] - du[0]) * 1e-5;
    let hv = (dv[1] - dv[0]) * 1e-5;
    let point = |u: f64, v: f64| -> Result<[f64; 3]> {
        let p = sampler.evaluate(u, v)?.point;
        Ok([p[0], p[1], p[2]])
    };
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut samples = 0usize;
    for i in 0..=4 {
        for j in 0..=4 {
            guard.tick()?;
            let u = du[0] + (du[1] - du[0]) * i as f64 / 4.;
            let v = dv[0] + (dv[1] - dv[0]) * j as f64 / 4.;
            let u0 = (u - hu).max(du[0]);
            let u1 = (u + hu).min(du[1]);
            let v0 = (v - hv).max(dv[0]);
            let v1 = (v + hv).min(dv[1]);
            if u1 - u0 <= 0. || v1 - v0 <= 0. {
                continue;
            }
            let tu = sub(point(u1, v)?, point(u0, v)?);
            let tv = sub(point(u, v1)?, point(u, v0)?);
            let n = cross(tu, tv);
            let len = norm(n);
            if !(len.is_finite() && len > 0.) {
                continue;
            }
            let align = dot(mul(n, 1. / len), axis.direction).abs().clamp(0., 1.);
            let angle = align.acos();
            min = min.min(angle);
            max = max.max(angle);
            samples += 1;
        }
    }
    if samples == 0 {
        return Err(invalid(format!(
            "Face {face} has no sampleable surface normal"
        )));
    }
    Ok((samples, min, max))
}

/// Record the roots of `a cosθ + b sinθ = 0` (both, one per direction).
fn push_harmonic_roots(
    events: &mut Vec<DraftLimit>,
    vertex: usize,
    face: Option<usize>,
    a: f64,
    b: f64,
) {
    if a.abs() <= 1e-12 && b.abs() <= 1e-12 {
        return;
    }
    let t0 = (-a).atan2(b);
    let other = if t0 > 0. { t0 - std::f64::consts::PI } else { t0 + std::f64::consts::PI };
    for t in [t0, other] {
        if t.abs() > 1e-9 && t.is_finite() {
            events.push(DraftLimit {
                vertex,
                limiting_face: face,
                angle_rad: t,
            });
        }
    }
}

/// Record the roots of `c + a cosθ + b sinθ = 0` (rigid-rotation plane
/// crossing), when the trajectory reaches the plane at all.
fn push_crossing_roots(
    events: &mut Vec<DraftLimit>,
    vertex: usize,
    face: Option<usize>,
    c: f64,
    a: f64,
    b: f64,
) {
    let r = a.hypot(b);
    if r <= 1e-12 || c.abs() > r {
        return;
    }
    let phi = b.atan2(a);
    let base = (-c / r).clamp(-1., 1.).acos();
    for s in [base, -base] {
        let mut t = s - phi;
        while t > std::f64::consts::PI {
            t -= 2. * std::f64::consts::PI;
        }
        while t <= -std::f64::consts::PI {
            t += 2. * std::f64::consts::PI;
        }
        if t.abs() > 1e-9 && t.is_finite() {
            events.push(DraftLimit {
                vertex,
                limiting_face: face,
                angle_rad: t,
            });
        }
    }
}

/// Closed-form rotation limits; `Ok(None)` when the neighborhood is outside
/// the planar prediction subset (curved faces/edges, axis not in the face
/// plane, unsupported corner incidence).
fn rotation_limits(
    model: &Model,
    face: usize,
    axis: &Axis,
    guard: &mut BudgetGuard,
) -> Result<Option<Vec<DraftLimit>>> {
    let linear = model.tolerance_mm * 8.;
    let Ok(plane) = face_plane_fit(model, face) else {
        return Ok(None);
    };
    // The axis must lie in the face plane: point on the plane, direction
    // perpendicular to the plane normal.
    if (dot(plane.normal, axis.point) - plane.offset).abs() > linear
        || dot(plane.normal, axis.direction).abs() > 1e-9
    {
        return Ok(None);
    }
    for (i, f) in model.faces.iter().enumerate() {
        guard.tick()?;
        if face_plane_fit(model, i).is_err() {
            return Ok(None);
        }
        for &ring in std::iter::once(&f.outer).chain(&f.holes) {
            if model.loops[ring]
                .coedges
                .iter()
                .any(|c| model.edges[c.edge].curve.degree != 1)
            {
                return Ok(None);
            }
        }
    }
    let polygons = canonical_polygons(model, &BTreeMap::new(), guard)?;
    let adjacency = edge_face_adjacency(model, guard)?;
    let driven = coplanar_component(model, face, plane, &BTreeSet::new(), &adjacency, guard)?;
    let incidence = vertex_face_incidence(model);
    let n = plane.normal;
    let k = axis.direction;
    let m = cross(k, n);
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    for &d in &driven {
        for &ring in std::iter::once(&model.faces[d].outer).chain(&model.faces[d].holes) {
            for v in loop_vertices(model, ring)? {
                if !seen.insert(v) {
                    continue;
                }
                guard.tick()?;
                let p = model.vertices[v].point;
                let r = sub(p, axis.point);
                let axial = dot(r, k);
                let radial = sub(r, mul(k, axial));
                if norm(radial) <= linear {
                    continue; // on the axis: the vertex is a fixed pivot
                }
                let non_driven: Vec<usize> = incidence[v]
                    .iter()
                    .copied()
                    .filter(|f| !driven.contains(f))
                    .collect();
                match non_driven.len() {
                    0 => {
                        // Rigid rotation about the axis: signed distance to a
                        // fixed plane Q is C + A cosθ + B sinθ.
                        let pivot = crate::direct_edit::add(axis.point, mul(k, axial));
                        for (f, poly) in polygons.iter().enumerate() {
                            if driven.contains(&f) {
                                continue;
                            }
                            let q: PlaneEq = poly.plane;
                            push_crossing_roots(
                                &mut events,
                                v,
                                Some(f),
                                dot(q.normal, pivot) - q.offset,
                                dot(q.normal, radial),
                                dot(q.normal, cross(k, radial)),
                            );
                        }
                    }
                    2 => {
                        let (n1, n2) = (polygons[non_driven[0]].plane, polygons[non_driven[1]].plane);
                        let Some((lp, ld)) = intersect_planes(n1, n2, 1e-9) else {
                            return Ok(None);
                        };
                        // The slide line becomes parallel to the rotating
                        // plane: ld·n(θ) = 0, a pure harmonic root.
                        push_harmonic_roots(&mut events, v, None, dot(ld, n), dot(ld, m));
                        for (f, poly) in polygons.iter().enumerate() {
                            if driven.contains(&f)
                                || f == non_driven[0]
                                || f == non_driven[1]
                            {
                                continue;
                            }
                            // The sliding vertex reaches plane Q exactly when
                            // the fixed point w = L ∩ Q lies on P'(θ).
                            let Some(w) = line_plane(lp, ld, poly.plane, 1e-12) else {
                                continue;
                            };
                            let rw = sub(w, axis.point);
                            push_harmonic_roots(
                                &mut events,
                                v,
                                Some(f),
                                dot(n, rw),
                                dot(m, rw),
                            );
                        }
                    }
                    _ => return Ok(None),
                }
            }
        }
    }
    Ok(Some(events))
}

/// Draft analysis of `face` against `axis`: sampled normal-to-axis angles
/// plus the closed-form maximum tweak rotation in both directions when the
/// neighborhood admits the planar prediction (see the module docs).
pub fn draft_angle(model: &Model, face: usize, axis: Axis, budget: &Budget) -> Result<DraftReport> {
    if face >= model.faces.len() {
        return Err(invalid(format!("Face {face} is out of range")));
    }
    if axis.point.iter().chain(&axis.direction).any(|v| !v.is_finite()) {
        return Err(invalid("Axis point/direction must be finite"));
    }
    let length = norm(axis.direction);
    if length <= 0. {
        return Err(invalid("Axis direction must be nonzero"));
    }
    let axis = Axis {
        point: axis.point,
        direction: mul(axis.direction, 1. / length),
    };
    let mut guard = budget.guard("draft-angle");
    guard.check()?;
    let (samples, normal_axis_min_rad, normal_axis_max_rad) =
        normal_axis_angles(model, face, &axis, &mut guard)?;
    let planar = face_plane_fit(model, face).is_ok();
    let events = rotation_limits(model, face, &axis, &mut guard)?;
    let mut report = DraftReport {
        face,
        axis,
        planar,
        samples,
        normal_axis_min_rad,
        normal_axis_max_rad,
        max_angle_positive_rad: None,
        max_angle_negative_rad: None,
        limit_positive: None,
        limit_negative: None,
    };
    if let Some(events) = events {
        for event in events {
            if event.angle_rad > 0. {
                if report
                    .max_angle_positive_rad
                    .is_none_or(|best| event.angle_rad < best)
                {
                    report.max_angle_positive_rad = Some(event.angle_rad);
                    report.limit_positive = Some(event);
                }
            } else if report
                .max_angle_negative_rad
                .is_none_or(|best| -event.angle_rad < best)
            {
                report.max_angle_negative_rad = Some(-event.angle_rad);
                report.limit_negative = Some(event);
            }
        }
    }
    guard.check()?;
    Ok(report)
}
