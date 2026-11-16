//! Analytic plane/torus intersection for a canonical planar patch operand
//! and a canonical ring-torus solid, in the axial configurations only.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, plane operand first). The torus
//! operand must be the exact canonical ring torus of `analytic::torus`:
//! sixteen rational biquadratic patches tiling four profile quadrants times
//! four revolution quadrants, with exact tensor-product weights, exact
//! unit-square boundary trims and sixteen ring/quadrant vertices, optionally
//! carried through a rigid affine placement (`recognize_torus` below). The
//! canonical constructor admits only strict ring tori (R - r >= 1e-5): horn
//! (r == R) and spindle (r > R) tori are refused at construction, so the
//! recognizer never certifies one and this cell never sees a
//! self-intersecting torus. Anything else is an explicit
//! `UnsupportedSurface` region — this cell never falls back to numerical
//! surface/surface subdivision.
//!
//! Only axial plane placements are analytic here:
//! Plane perpendicular to the torus axis at axial height h from the center
//!   plane: |h| > r (clear beyond the outward band) resolves empty; |h| == r
//!   within the band is a tangency and stays `TangencyOrMultipleRoot`, never
//!   a guessed point or circle; |h| < r yields the two exact circles of
//!   radii R +- sqrt(r^2 - h^2) centered on the axis in the plane (h == 0 is
//!   the equator pair R +- r of the same branch). The circles are clipped by
//!   the patch's finite rectangular domain with exact eccentric-angle trims
//!   (`plane_sphere::clip_ellipse`); a tangency to a domain edge stays
//!   unresolved. An inner radius R - sqrt(r^2 - h^2) collapsing into the
//!   band would be an inner-tangency/horn configuration and stays
//!   unresolved (unreachable for a canonical strict ring torus, where
//!   R - r >= 1e-5; kept as an honest guard, never guessed through).
//! Plane containing the axis (normal perpendicular to the axis, center in
//!   the plane at pure-rounding scale): the two exact meridian circles of
//!   radius r centered at +-R along the in-plane line where the plane meets
//!   the center plane, clipped the same way. A parallel-to-axis plane at a
//!   certified nonzero distance cuts a Cassini oval, not a circle pair:
//!   clearly offset planes are `UnsupportedSurface`, and an offset inside
//!   the recognition-scale band reports `NearCoincidence` — never snapped.
//! Oblique planes are a quartic section with Villarceau degeneracies:
//!   clearly oblique planes are `UnsupportedSurface`, tilts inside the
//!   recognition-scale band off either axial configuration report
//!   `NearCoincidence`.
//! A plane can never coincide with a curved torus patch, so `CoincidentTrim`
//! is deliberately unused by this cell.
//!
//! Resolved contacts are exact rational circles (full: four 90-degree arcs,
//! weights cos(pi/4), knots 0..=4; clipped: exact swept arcs) with UV lifts
//! on both operands: on the plane, exact rational quadratic ellipse arcs per
//! the `recognize_plane` metric; on the torus the perpendicular-plane
//! circles are iso-v parallels (constant exact v over the profile quadrant,
//! u clipped per revolution quadrant) and the through-axis circles are
//! iso-u meridians (constant exact u over the revolution quadrant, v
//! clipped per profile quadrant) — both are degree-1 UV line segments on
//! the exact patch grid, one per covered patch. Classification uses outward
//! binary64 bands widened by both recognition deviations; the angular snap
//! is 64 binary64 epsilons plus the recognition-error angular allowances,
//! and recognition-scale tilts report `NearCoincidence`, never forced.
//! Nothing here authorizes a topology change.
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION};
use super::*;
use crate::Model;

/// A recognized canonical ring torus: the exact sixteen-patch solid of
/// `analytic::torus`, optionally rigidly placed.
#[derive(Clone, Debug)]
pub struct CanonicalTorus {
    /// Torus center (the center of the symmetry-plane circle of radius R).
    pub center: [f64; 3],
    /// Unit symmetry axis; the profile circle rises toward +axis.
    pub axis: [f64; 3],
    pub major: f64,
    pub minor: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Face index by [profile quadrant][revolution quadrant].
    pub patches: [[usize; 4]; 4],
}

/// Profile-span control points (radial, axial) of the canonical torus
/// profile circle of radius `minor` centered at radial `major`: four
/// 90-degree arcs starting at the outer equator point, turning toward +axis.
fn profile_controls(major: f64, minor: f64) -> [[[f64; 2]; 3]; 4] {
    let (r, m) = (major, minor);
    [
        [[r + m, 0.], [r + m, m], [r, m]],
        [[r, m], [r - m, m], [r - m, 0.]],
        [[r - m, 0.], [r - m, -m], [r, -m]],
        [[r, -m], [r + m, -m], [r + m, 0.]],
    ]
}

/// Faces, vertices and loops of the canonical sixteen-patch torus.
const PATCH_COUNT: usize = 16;
/// Edges of the canonical torus: two per patch.
const EDGE_COUNT: usize = 32;
/// Smallest and largest admitted outer-equator distance from the center.
const OUTER_RANGE: std::ops::RangeInclusive<f64> = 2e-5..=1e6;
/// Smallest and largest admitted minor radius.
const MINOR_RANGE: std::ops::RangeInclusive<f64> = 1e-5..=1e6;
/// Absolute floor added to the relative recognition band.
const ABSOLUTE_SLACK: f64 = 1e-12;

fn length3(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Certifies the canonical torus topology counts, the closed shell, and every
/// face's exact biquadratic surface structure, weights and unit-square trims.
fn torus_structure(model: &Model) -> bool {
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != PATCH_COUNT
        || model.vertices.len() != PATCH_COUNT
        || model.edges.len() != EDGE_COUNT
        || model.loops.len() != PATCH_COUNT
    {
        return false;
    }
    let shell = &model.shells[0];
    if !shell.closed || shell.faces.len() != PATCH_COUNT {
        return false;
    }
    let w2 = ARC_WEIGHT * ARC_WEIGHT;
    let expected_weights = [
        [1., ARC_WEIGHT, 1.],
        [ARC_WEIGHT, w2, ARC_WEIGHT],
        [1., ARC_WEIGHT, 1.],
    ];
    model.faces.iter().enumerate().all(|(face_index, face)| {
        let surface = &face.surface;
        surface.degree_u == 2
            && surface.degree_v == 2
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 0., 1., 1., 1.]
            && surface.knots_v == [0., 0., 0., 1., 1., 1.]
            && surface.control_points.len() == 3
            && surface.control_points.iter().all(|row| row.len() == 3)
            && surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p.len() == 3)
            && surface.weights == expected_weights
            && face.holes.is_empty()
            && super::recognize::unit_square_boundary(model, face_index)
    })
}

/// The torus center, axis, radii and ring frame recovered from the vertices,
/// plus the running structural deviation bound.
struct TorusFit {
    center: [f64; 3],
    axis: [f64; 3],
    major: f64,
    minor: f64,
    scale: f64,
    x_dir: [f64; 3],
    y_dir: [f64; 3],
    profile: [[[f64; 2]; 3]; 4],
    error: f64,
}

impl TorusFit {
    /// Recovers the center, the axis from the outer equator ring, both radii
    /// and the ring frame.
    fn measure(model: &Model) -> Option<Self> {
        // Center: each profile ring's four quadrant vertices average to the
        // same axis point, so all sixteen vertices average to the torus center.
        let mut center = [0.; 3];
        for vertex in &model.vertices {
            for k in 0..3 {
                center[k] += vertex.point[k] / 16.;
            }
        }
        let mut error: f64 = 0.;
        // The outer equator ring (rho = R + r, axial 0) is strictly the
        // farthest vertex set from the center for a ring torus:
        // (R+r)^2 > R^2 + r^2.
        let dmax = model
            .vertices
            .iter()
            .map(|v| length3(sub(v.point, center)))
            .fold(0., f64::max);
        if !dmax.is_finite() || !OUTER_RANGE.contains(&dmax) {
            return None;
        }
        let scale = dmax;
        let outer: Vec<[f64; 3]> = model
            .vertices
            .iter()
            .map(|v| v.point)
            .filter(|p| (length3(sub(*p, center)) - dmax).abs() <= RECOGNITION * scale)
            .collect();
        if outer.len() != 4 {
            return None;
        }
        // Axis from the outer ring plane; every outer vertex must lie in the
        // center plane within the band.
        let axis_raw = cross(sub(outer[1], outer[0]), sub(outer[2], outer[0]));
        let axis_len = length3(axis_raw);
        if !axis_len.is_finite() || axis_len <= 0. {
            return None;
        }
        let axis = axis_raw.map(|x| x / axis_len);
        for p in &outer {
            let deviation = dot(sub(*p, center), axis).abs();
            if !deviation.is_finite() || deviation > RECOGNITION * scale {
                return None;
            }
            error = error.max(deviation);
        }
        // r is the extreme axial vertex coordinate; R = (R + r) - r.
        let minor = model
            .vertices
            .iter()
            .map(|v| dot(sub(v.point, center), axis).abs())
            .fold(0., f64::max);
        let major = dmax - minor;
        if !minor.is_finite()
            || !major.is_finite()
            || !MINOR_RANGE.contains(&minor)
            || major <= minor
        {
            return None;
        }
        // Ring frame: x from the first outer-ring vertex, y = axis x x.
        let x_dir = sub(outer[0], center).map(|x| x / dmax);
        let y_dir = cross(axis, x_dir);
        Some(Self {
            center,
            axis,
            major,
            minor,
            scale,
            x_dir,
            y_dir,
            profile: profile_controls(major, minor),
            error,
        })
    }

    /// (profile quadrant, revolution quadrant, radial distance) of a
    /// ring-frame point; both angular residuals must sit at pure
    /// rounding/recognition scale.
    fn quadrant_of(&self, point: [f64; 3]) -> Option<(usize, usize, f64)> {
        let d = sub(point, self.center);
        let axial = dot(d, self.axis);
        let perp = sub(d, self.axis.map(|x| x * axial));
        let rho = length3(perp);
        let alpha = axial.atan2(rho - self.major);
        let theta = dot(perp, self.y_dir).atan2(dot(perp, self.x_dir));
        let j = super::recognize::snap_quadrant(alpha, RECOGNITION)?;
        let i = super::recognize::snap_quadrant(theta, RECOGNITION)?;
        Some((j, i, rho))
    }

    fn bound(&self) -> f64 {
        RECOGNITION * self.scale + ABSOLUTE_SLACK
    }

    /// Per-patch quadrant tiling and exact control-point certification;
    /// returns the face index by [profile quadrant][revolution quadrant].
    fn certify_patches(&mut self, model: &Model) -> Result<Option<[[usize; 4]; 4]>> {
        let mut seen = [[false; 4]; 4];
        let mut patches = [[0usize; 4]; 4];
        for (index, face) in model.faces.iter().enumerate() {
            let surface = &face.surface;
            let p00 = surface.evaluate(0., 0.)?.point;
            let Some((j, i, _)) = self.quadrant_of([p00[0], p00[1], p00[2]]) else {
                return Ok(None);
            };
            if seen[j][i] {
                return Ok(None);
            }
            seen[j][i] = true;
            patches[j][i] = index;
            let pattern = super::recognize::quarter_arc_pattern(i);
            for (a, dir) in pattern.iter().enumerate() {
                for (b, profile_point) in self.profile[j].iter().enumerate() {
                    let expected: [f64; 3] = std::array::from_fn(|k| {
                        self.center[k]
                            + profile_point[0] * (dir[0] * self.x_dir[k] + dir[1] * self.y_dir[k])
                            + profile_point[1] * self.axis[k]
                    });
                    let Some(actual) = surface.control_points.get(a).and_then(|row| row.get(b))
                    else {
                        return Ok(None);
                    };
                    let Some(deviation) =
                        super::recognize::certified_deviation(actual, expected, self.bound())
                    else {
                        return Ok(None);
                    };
                    self.error = self.error.max(deviation);
                }
            }
        }
        if seen.iter().flatten().any(|hit| !hit) {
            return Ok(None);
        }
        Ok(Some(patches))
    }

    /// Every vertex is a ring/quadrant vertex, each exactly once, certified
    /// radially and axially against the exact profile ring starts.
    fn certify_vertices(&mut self, model: &Model) -> bool {
        let mut used = [[false; 4]; 4];
        for vertex in &model.vertices {
            let Some((j, i, rho)) = self.quadrant_of(vertex.point) else {
                return false;
            };
            let axial = dot(sub(vertex.point, self.center), self.axis);
            if used[j][i] {
                return false;
            }
            used[j][i] = true;
            let start = self.profile[j][0];
            let deviation = (rho - start[0]).abs().max((axial - start[1]).abs());
            if !deviation.is_finite() || deviation > self.bound() {
                return false;
            }
            self.error = self.error.max(deviation);
        }
        used.iter().flatten().all(|hit| *hit)
    }
}

/// Recognizes a canonical ring-torus solid as built by `analytic::torus`,
/// certifying the 4x4 quadrant patch tiling, the exact tensor-product
/// weights [[1,w,1],[w,w^2,w],[1,w,1]] with w = cos(pi/4), the unit-square
/// boundary trims, every control point against the exact construction in the
/// recovered ring frame, and all sixteen ring/quadrant vertices. Rigid
/// affine placement is admitted; anything else returns `None`. Only strict
/// ring tori exist canonically (the constructor refuses horn and spindle
/// tori), and the recovered radii must keep R - r clearly positive.
pub fn recognize_torus(model: &Model) -> Result<Option<CanonicalTorus>> {
    model.validate()?;
    if !torus_structure(model) {
        return Ok(None);
    }
    let Some(mut fit) = TorusFit::measure(model) else {
        return Ok(None);
    };
    let Some(patches) = fit.certify_patches(model)? else {
        return Ok(None);
    };
    if !fit.certify_vertices(model) {
        return Ok(None);
    }
    Ok(Some(CanonicalTorus {
        center: fit.center,
        axis: fit.axis,
        major: fit.major,
        minor: fit.minor,
        error: fit.error,
        frame: [fit.x_dir, fit.y_dir],
        patches,
    }))
}

