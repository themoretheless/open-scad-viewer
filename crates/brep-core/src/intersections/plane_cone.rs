//! Analytic plane/cone (frustum) intersection for a canonical planar patch
//! operand and a canonical conical frustum solid — the conic-section cell.
//!
//! The plane operand is the canonical finite rectangular planar patch of
//! `plane_sphere::recognize_plane` (one-face open model, exact bilinear
//! affine surface over the unit square, orthogonal edge vectors); the cone
//! operand is the exact canonical frustum of `analytic::frustum` — four
//! rational ruled side patches linearly interpolated between the two radius
//! rings plus a bilinear cap per nonzero ring, with a true apex admitted
//! when exactly one radius is zero (the canonical constructor shares the
//! apex vertex across all four side patches) — recognized by
//! `recognize_cone` under rigid affine placement. An equal-radius frustum
//! is a cylinder and is refused (`UnsupportedSurface`, the plane/cylinder
//! cell owns it); anything else is an explicit `UnsupportedSurface` region
//! — this cell never falls back to numerical surface/surface subdivision.
//!
//! All angles are classified on outward binary64 bands widened by both
//! recognition deviations. With g = axis.normal, sigma = sin(alpha) and
//! alpha = atan(|slope|) the cone half-angle (slope = (r_top - r_bottom) /
//! height), the plane/axis angle beta satisfies sin(beta) = |g|:
//! |g| snapped to 1 (pure rounding): the section perpendicular to the
//!   axis is the exact circle with the radius LINEARLY INTERPOLATED between
//!   the rings at the section height, clipped by the patch rectangle
//!   (closed-form eccentric-angle clip). Provably outside the finite height
//!   resolves empty; a plane within the band of a nonzero-radius ring plane
//!   is a coincident cap/rim region (`CoincidentTrim`, never a guessed
//!   circle); a section at the apex level (radius <= band, apex ring
//!   included) stays `TangencyOrMultipleRoot`. Lifts are exact: iso-v lines
//!   on the side patches and ellipse arcs in the plane UV.
//! |g| snapped to 0 (pure rounding): the plane is parallel to the axis.
//!   A plane through the axis (apex on the plane within the band) yields
//!   the two exact straight ruling LINES through the apex (degree-1 NURBS,
//!   exact), clipped by the finite height and the patch rectangle
//!   (band-thin clips, boundary-coincident rulings and patch-corner touches
//!   stay unresolved), lifted as iso-u rulings on the side patches. An
//!   off-axis parallel plane yields the exact hyperbola below.
//! Oblique with ||g| - sigma| snapped to 0 (pure rounding — a plane built
//!   exactly parallel to a ruling): the exact rational parabola (polynomial
//!   quadratic Bezier arcs, all weights 1), clipped by the height rings,
//!   the nappe half-plane and the patch rectangle in one closed-form
//!   quadratic clip in the curve parameter. The parabola boundary is the
//!   degenerate threshold: an angle within the RECOGNITION-scale band of
//!   alpha but beyond the pure-rounding snap stays `TangencyOrMultipleRoot`
//!   — never a guessed conic.
//! Oblique with |g| > sigma: the exact ellipse (rational quadratic sweep,
//!   exact weights), clipped by the height rings, the nappe half-plane and
//!   the patch rectangle in one exact eccentric-angle clip (every
//!   constraint is linear in the plane UV, reusing `clip_ellipse`).
//! Oblique with |g| < sigma (and the off-axis parallel plane): the exact
//!   hyperbola. Each nappe arm is parameterized exactly as the rational
//!   quadratic x = x_c +- a (1+t^2)/(1-t^2), y = 2 b t/(1-t^2) and clipped
//!   by the height rings, the nappe half-plane and the patch rectangle —
//!   every constraint is quadratic in t, solved in closed form. The mirror
//!   arm of the double cone is clipped away by the nappe half-plane.
//! Any plane through the apex region (|d0| <= band, apex = the singular
//!   point of the side surface) stays `TangencyOrMultipleRoot` outside the
//!   exact through-axis ruling case; the apex is a multiple-root contact
//!   and is never guessed.
//!
//! Cone-side UV lifts: the circle (iso-v) and ruling (iso-u) cases are
//! exact. The ellipse/parabola/hyperbola side lifts are NOT shipped: on the
//! rational ruled side patches the conic unrolls through an azimuth/height
//! map with no exact LOW-DEGREE rational UV representation (an exact form
//! needs degree 4+ with seam splits), so `cone_uv` is null rather than a
//! numerical fit — the 3D conic and the plane-UV lift stay exact. This
//! mirrors the documented oblique plane/cylinder precedent.
//!
//! Scope note (shared with the other plane/* cells): only side-surface
//! contacts are reported; where the plane patch crosses a cap disk the cap
//! chord is not a component — coverage is NumericallyResolved, never
//! certified complete, and nothing here authorizes a topology change.
use super::cylinder_cylinder::arc_parameter;
use super::plane_sphere::{
    CanonicalPlane, EllipseClip, Halfplane2, PlanePatchCurve, clip_ellipse, conic_arcs_2d,
    conic_sweep, recognize_plane,
};
use super::sphere_cylinder::CylinderPatchCurve;
use super::sphere_sphere::{ARC_WEIGHT, RECOGNITION, ccw_intersect};
use super::*;
use crate::Model;

const TAU: f64 = std::f64::consts::TAU;
const QUARTER: f64 = std::f64::consts::FRAC_PI_2;
const QUADRANTS: [[f64; 2]; 4] = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];

#[derive(Clone, Debug)]
pub enum PlaneConeComponent {
    /// Plane perpendicular to the axis: the exact circle (full, or clipped
    /// to the finite plane domain) with the radius linearly interpolated
    /// between the rings at the section height, with lifts on both
    /// surfaces.
    Circle {
        /// Exact rational quadratic sweep (full circle: four 90-degree
        /// arcs, weights cos(pi/4), knots 0..=4).
        curve: Curve,
        center: [f64; 3],
        radius: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the circle lies fully inside the plane patch domain.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-v circle arcs on the side patches.
        cone_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Plane through the axis: one exact straight ruling through the apex,
    /// clipped by the finite height and the patch rectangle.
    Line {
        /// Exact degree-1 line, knots [0,0,1,1], unit weights, start -> end.
        curve: Curve,
        start: [f64; 3],
        end: [f64; 3],
        /// Unit ruling direction (apex toward the rings).
        direction: [f64; 3],
        /// Clip endpoints sit on ring planes or the patch boundary.
        contact: Contact,
        plane_uv: Vec<PlanePatchCurve>,
        /// Iso-u segments on the side patches (two entries when the ruling
        /// sits on a quadrant seam).
        cone_uv: Vec<CylinderPatchCurve>,
        max_sample_residual: f64,
    },
    /// Oblique plane steeper than the side (|axis.n| > sin(alpha)): the
    /// exact ellipse (full, or clipped to the finite height and patch
    /// domain) with the exact plane-UV lift. The cone side lift is null:
    /// no exact low-degree rational UV representation exists on the ruled
    /// side patches (module header); the 3D ellipse is exact regardless.
    Ellipse {
        /// Exact rational quadratic sweep over the eccentric-angle interval.
        curve: Curve,
        center: [f64; 3],
        semi_major: f64,
        semi_minor: f64,
        /// Unit in-plane semi-major direction.
        major: [f64; 3],
        /// Unit in-plane semi-minor direction.
        minor: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        /// True when the full ellipse survives every clip.
        full: bool,
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
    /// Plane exactly parallel to a ruling (|axis.n| == sin(alpha) at pure
    /// rounding): the exact rational parabola (polynomial quadratic arcs,
    /// all weights 1) clipped by the finite height and the patch domain.
    Parabola {
        /// Exact degree-2 Bezier arcs over the clipped t-interval, weights 1.
        curve: Curve,
        /// Parabola vertex (3D).
        vertex: [f64; 3],
        /// Unit in-plane direction the parabola opens toward.
        direction: [f64; 3],
        /// Focal length |L| / 4 of y^2 = L (x - x0).
        focal_length: f64,
        /// Unit plane normal.
        normal: [f64; 3],
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
    /// Plane shallower than the side (|axis.n| < sin(alpha)), including
    /// off-axis parallel planes: one exact rational quadratic hyperbola arc
    /// clipped by the height rings and the patch domain.
    Hyperbola {
        /// Exact rational quadratic arc over the clipped t-interval.
        curve: Curve,
        /// Midpoint between the two branch vertices (3D).
        center: [f64; 3],
        /// Transverse semi-axis a (vertex distance from the center).
        semi_transverse: f64,
        /// Conjugate semi-axis b (asymptote slope b / a).
        semi_conjugate: f64,
        /// Unit in-plane transverse direction toward this branch.
        transverse: [f64; 3],
        /// Unit in-plane conjugate direction.
        conjugate: [f64; 3],
        /// Unit plane normal.
        normal: [f64; 3],
        plane_uv: Vec<PlanePatchCurve>,
        cone_uv: Option<Vec<CylinderPatchCurve>>,
        max_sample_residual: f64,
    },
}

#[derive(Clone, Debug)]
pub(crate) struct CanonicalCone {
    /// Bottom ring center (the apex itself when the bottom radius is zero).
    pub(crate) bottom: [f64; 3],
    /// Unit axis, bottom ring toward top ring.
    pub(crate) axis: [f64; 3],
    pub(crate) r_bottom: f64,
    pub(crate) r_top: f64,
    pub(crate) height: f64,
    /// Radius slope (r_top - r_bottom) / height, certified nonzero.
    pub(crate) slope: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub(crate) error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub(crate) frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub(crate) sides: [usize; 4],
    /// Cap face indices [bottom, top]; None for an apex ring.
    #[allow(dead_code)]
    pub(crate) caps: [Option<usize>; 2],
}

/// Degree-1 pcurve exactly from `from` to `to` with unit knots/weights.
fn unit_edge(curve: &Curve, from: [f64; 2], to: [f64; 2]) -> bool {
    curve.degree == 1
        && curve.knots == [0., 0., 1., 1.]
        && curve.control_points == [from.to_vec(), to.to_vec()]
        && curve.weights == [1., 1.]
}

/// One quadrant's exact cap trim arc: the quarter circle of radius 1/2
/// centered at [1/2, 1/2] in cap UV, weights cos(pi/4).
fn cap_quarter_arc(curve: &Curve, quadrant: usize) -> bool {
    let [x, y] = QUADRANTS[quadrant];
    let [nx, ny] = QUADRANTS[(quadrant + 1) % 4];
    curve.degree == 2
        && curve.knots == [0., 0., 0., 1., 1., 1.]
        && curve.weights == [1., ARC_WEIGHT, 1.]
        && curve.control_points
            == [
                [0.5 + x / 2., 0.5 + y / 2.].to_vec(),
                [0.5 + (x + nx) / 2., 0.5 + (y + ny) / 2.].to_vec(),
                [0.5 + nx / 2., 0.5 + ny / 2.].to_vec(),
            ]
}

fn point3_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

fn hypot3(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// Recognizes a canonical conical frustum solid as built by
/// `analytic::frustum`, including the true-apex cone (exactly one radius
/// zero: the canonical constructor collapses that ring onto one shared
/// pole vertex and skips its cap). Certifies the side/cap surface
/// structure, exact weights and trim pcurves, every control point against
/// the exact construction in the recovered ring frame, a globally
/// consistent quadrant tiling, both ring vertex sets, and a
/// certified-nonzero taper (an equal-radius frustum is a cylinder and is
/// refused here). Rigid affine placement is admitted; anything else
/// returns `None`.
pub(crate) fn recognize_cone(model: &Model) -> Result<Option<CanonicalCone>> {
    model.validate()?;
    if model.bodies.len() != 1 || model.shells.len() != 1 {
        return Ok(None);
    }
    // Frustum: 4 sides + 2 caps, 8 ring vertices. Apex cone: 4 sides + 1
    // cap, 4 ring vertices + 1 shared apex vertex.
    if !((model.faces.len() == 6 && model.vertices.len() == 8)
        || (model.faces.len() == 5 && model.vertices.len() == 5))
    {
        return Ok(None);
    }
    let mut sides: Vec<usize> = Vec::new();
    let mut caps: Vec<usize> = Vec::new();
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if !face.holes.is_empty() {
            return Ok(None);
        }
        let side = surface.degree_u == 2
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 0., 1., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 3
            && surface.control_points.iter().all(|row| row.len() == 2);
        let cap = surface.degree_u == 1
            && surface.degree_v == 1
            && !surface.periodic_u
            && !surface.periodic_v
            && surface.knots_u == [0., 0., 1., 1.]
            && surface.knots_v == [0., 0., 1., 1.]
            && surface.control_points.len() == 2
            && surface.control_points.iter().all(|row| row.len() == 2);
        if side {
            if surface.weights != [[1., 1.], [ARC_WEIGHT, ARC_WEIGHT], [1., 1.]] {
                return Ok(None);
            }
            sides.push(index);
        } else if cap {
            if surface.weights != [[1., 1.], [1., 1.]] {
                return Ok(None);
            }
            caps.push(index);
        } else {
            return Ok(None);
        }
    }
    if sides.len() != 4 || caps.is_empty() || caps.len() > 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    let boundary = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ];
    for &index in &sides {
        let loop_ = &model.loops[model.faces[index].outer];
        if loop_.coedges.len() != 4 {
            return Ok(None);
        }
        let mut seen = [false; 4];
        for coedge in &loop_.coedges {
            let mut hit = false;
            for (k, &(a, b)) in boundary.iter().enumerate() {
                if !seen[k] && unit_edge(&coedge.pcurve, a, b) {
                    seen[k] = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Ok(None);
            }
        }
        if seen.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
    }
    // Ring centers: each ring vertex is shared by two adjacent side patches,
    // so the eight evaluated corners average to the ring center. On an apex
    // ring every corner evaluates to the one shared apex vertex.
    let mut bottom = [0.; 3];
    let mut top = [0.; 3];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        for u in [0., 1.] {
            let b = point3_of(&surface.evaluate(u, 0.)?.point);
            let t = point3_of(&surface.evaluate(u, 1.)?.point);
            for k in 0..3 {
                bottom[k] += b[k] / 8.;
                top[k] += t[k] / 8.;
            }
        }
    }
    let axis_vec = sub(top, bottom);
    let height = hypot3(axis_vec);
    if !height.is_finite() || !(1e-5..=1e6).contains(&height) {
        return Ok(None);
    }
    let axis = axis_vec.map(|x| x / height);
    let radial = |point: [f64; 3], from: [f64; 3]| {
        let d = sub(point, from);
        let axial = dot(d, axis);
        sub(d, axis.map(|x| x * axial))
    };
    // Ring radii from the four arc midpoints (u = v = 1/2 sits on the
    // 45-degree point of the quarter arc at that height's radius).
    let mut r_bottom = 0.;
    let mut r_top = 0.;
    let mut mid_radii = Vec::with_capacity(8);
    for &index in &sides {
        let pb = point3_of(&model.faces[index].surface.evaluate(0.5, 0.)?.point);
        let pt = point3_of(&model.faces[index].surface.evaluate(0.5, 1.)?.point);
        let rb = hypot3(radial(pb, bottom));
        let rt = hypot3(radial(pt, top));
        mid_radii.push((rb, false));
        mid_radii.push((rt, true));
        r_bottom += rb / 4.;
        r_top += rt / 4.;
    }
    let r_scale = r_bottom.max(r_top);
    if !r_scale.is_finite() || !(1e-5..=1e6).contains(&r_scale) {
        return Ok(None);
    }
    let mut error: f64 = 0.;
    for (r, is_top) in &mid_radii {
        let ring = if *is_top { r_top } else { r_bottom };
        let deviation = (r - ring).abs();
        if deviation > RECOGNITION * r_scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(deviation);
    }
    // An apex ring is exactly zero in the canonical construction; snap a
    // rounding-scale radius to zero and record the deviation.
    let (r_bottom, r_top) = (
        if r_bottom <= RECOGNITION * r_scale + 1e-12 {
            error = error.max(r_bottom);
            0.
        } else {
            r_bottom
        },
        if r_top <= RECOGNITION * r_scale + 1e-12 {
            error = error.max(r_top);
            0.
        } else {
            r_top
        },
    );
    // Certified-nonzero taper: an equal-radius frustum is a cylinder and is
    // refused here (the plane/cylinder cell owns it).
    if (r_top - r_bottom).abs() <= RECOGNITION * r_scale + 1e-12 {
        return Ok(None);
    }
    // In-plane frame from a nonzero ring: x is the quadrant-0 direction.
    let (ring_center, ring_v) = if r_bottom > 0. {
        (bottom, 0.)
    } else {
        (top, 1.)
    };
    let start = point3_of(&model.faces[sides[0]].surface.evaluate(0., ring_v)?.point);
    let x_perp = radial(start, ring_center);
    let x_length = hypot3(x_perp);
    if !x_length.is_finite() || x_length <= 0. {
        return Ok(None);
    }
    let x_dir = x_perp.map(|x| x / x_length);
    let y_dir = cross(axis, x_dir);
    // Per-patch quadrant tiling and exact control-point certification
    // against the linearly interpolated ring radii (apex row: the apex).
    let mut seen = [false; 4];
    let mut ordered = [0usize; 4];
    for &index in &sides {
        let surface = &model.faces[index].surface;
        let point = point3_of(&surface.evaluate(0., ring_v)?.point);
        let perp = radial(point, ring_center);
        let angle = dot(perp, y_dir).atan2(dot(perp, x_dir));
        let quadrant = (angle / QUARTER).round() as i64;
        let quadrant = quadrant.rem_euclid(4) as usize;
        let residual = (angle - quadrant as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        if residual.abs() > RECOGNITION {
            return Ok(None);
        }
        if seen[quadrant] {
            return Ok(None);
        }
        seen[quadrant] = true;
        ordered[quadrant] = index;
        let qa = QUADRANTS[quadrant];
        let qb = QUADRANTS[(quadrant + 1) % 4];
        let pattern = [
            [qa[0], qa[1]],
            [qa[0] + qb[0], qa[1] + qb[1]],
            [qb[0], qb[1]],
        ];
        for (k, expected_xy) in pattern.iter().enumerate() {
            for (j, (ring_r, zz)) in [(r_bottom, 0.), (r_top, height)].iter().enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    bottom[a]
                        + ring_r * (expected_xy[0] * x_dir[a] + expected_xy[1] * y_dir[a])
                        + zz * axis[a]
                });
                let actual = &surface.control_points[k][j];
                if actual.len() != 3 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = hypot3(d);
                if !deviation.is_finite() || deviation > RECOGNITION * r_scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
    }
    if seen.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    // Caps: axial slot from any corner, exact bilinear corners in the ring
    // frame at that ring's radius, and the four inscribed quarter-arc trims
    // each exactly once. An apex ring has no cap.
    let mut cap_assigned = [false; 2];
    let mut cap_ids: [Option<usize>; 2] = [None, None];
    for &index in &caps {
        let surface = &model.faces[index].surface;
        let corner = point3_of(&surface.evaluate(0., 0.)?.point);
        let axial = dot(sub(corner, bottom), axis);
        let slot = if axial.abs() <= RECOGNITION * height {
            0
        } else if (axial - height).abs() <= RECOGNITION * height {
            1
        } else {
            return Ok(None);
        };
        error = error.max(if slot == 0 {
            axial.abs()
        } else {
            (axial - height).abs()
        });
        let ring_r = if slot == 0 { r_bottom } else { r_top };
        if ring_r == 0. {
            return Ok(None);
        }
        if cap_assigned[slot] {
            return Ok(None);
        }
        cap_assigned[slot] = true;
        cap_ids[slot] = Some(index);
        let ring_c = if slot == 0 { bottom } else { top };
        for (i, row) in surface.control_points.iter().take(2).enumerate() {
            for (j, actual) in row.iter().take(2).enumerate() {
                let expected: [f64; 3] = std::array::from_fn(|a| {
                    ring_c[a]
                        + ring_r
                            * ((2. * i as f64 - 1.) * x_dir[a] + (2. * j as f64 - 1.) * y_dir[a])
                });
                if actual.len() != 3 {
                    return Ok(None);
                }
                let d = [
                    actual[0] - expected[0],
                    actual[1] - expected[1],
                    actual[2] - expected[2],
                ];
                let deviation = hypot3(d);
                if !deviation.is_finite() || deviation > RECOGNITION * r_scale + 1e-12 {
                    return Ok(None);
                }
                error = error.max(deviation);
            }
        }
        let loop_ = &model.loops[model.faces[index].outer];
        if loop_.coedges.len() != 4 {
            return Ok(None);
        }
        let mut seen_quadrant = [false; 4];
        for coedge in &loop_.coedges {
            let mut hit = false;
            for (quadrant, seen) in seen_quadrant.iter_mut().enumerate() {
                if !*seen && cap_quarter_arc(&coedge.pcurve, quadrant) {
                    *seen = true;
                    hit = true;
                    break;
                }
            }
            if !hit {
                return Ok(None);
            }
        }
        if seen_quadrant.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
    }
    if cap_assigned[0] != (r_bottom > 0.) || cap_assigned[1] != (r_top > 0.) {
        return Ok(None);
    }
    // Every vertex on one of the two rings (the apex ring: the pole).
    for vertex in &model.vertices {
        let d = sub(vertex.point, bottom);
        let axial = dot(d, axis);
        let perp = sub(d, axis.map(|x| x * axial));
        let slot = if axial.abs() <= RECOGNITION * height {
            0
        } else if (axial - height).abs() <= RECOGNITION * height {
            1
        } else {
            return Ok(None);
        };
        let ring_r = if slot == 0 { r_bottom } else { r_top };
        let dev_r = (hypot3(perp) - ring_r).abs();
        if dev_r > RECOGNITION * r_scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(dev_r);
    }
    Ok(Some(CanonicalCone {
        bottom,
        axis,
        r_bottom,
        r_top,
        height,
        slope: (r_top - r_bottom) / height,
        error,
        frame: [x_dir, y_dir],
        sides: ordered,
        caps: cap_ids,
    }))
}

/// Plane UV coordinates of a 3D point under the rectangular patch frame.
fn plane_uv(plane: &CanonicalPlane, point: [f64; 3]) -> [f64; 2] {
    let rel = sub(point, plane.origin);
    [
        dot(rel, plane.u) / (plane.u_len * plane.u_len),
        dot(rel, plane.v) / (plane.v_len * plane.v_len),
    ]
}

/// Rectangle half-planes of the patch domain with per-axis bands.
fn rect_halfplanes(plane: &CanonicalPlane, band: f64) -> [Halfplane2; 4] {
    [
        Halfplane2 {
            normal: [-1., 0.],
            offset: 0.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [1., 0.],
            offset: 1.,
            band: band / plane.u_len,
        },
        Halfplane2 {
            normal: [0., -1.],
            offset: 0.,
            band: band / plane.v_len,
        },
        Halfplane2 {
            normal: [0., 1.],
            offset: 1.,
            band: band / plane.v_len,
        },
    ]
}

/// Worst residual over `count` curve samples against the cone side equation
/// (radius linearly interpolated in the axial coordinate) and the plane
/// equation.
fn side_plane_residuals(
    curve: &Curve,
    cone: &CanonicalCone,
    plane: &CanonicalPlane,
    count: usize,
) -> Result<f64> {
    let [lo, hi] = curve.domain();
    let mut worst = 0_f64;
    for i in 0..count {
        let t = lo + (hi - lo) * i as f64 / (count - 1) as f64;
        let p = point3_of(&curve.evaluate(t)?.point);
        let rel = sub(p, cone.bottom);
        let a = dot(rel, cone.axis);
        let perp = sub(rel, cone.axis.map(|x| x * a));
        let side = (hypot3(perp) - (cone.r_bottom + cone.slope * a)).abs();
        let planar = dot(plane.normal, sub(p, plane.origin)).abs();
        worst = worst.max(side).max(planar);
    }
    Ok(worst)
}

/// Clipped iso-v circle lifts on the side patches for a section circle whose
/// swept parameter intervals are `arcs`; per-quadrant trims use the exact
/// rational-arc angle parameter of `cylinder_cylinder::arc_parameter`.
fn lift_circle_side(
    cone: &CanonicalCone,
    center: [f64; 3],
    v0: f64,
    arcs: &[(f64, f64)],
    point_at: impl Fn(f64) -> [f64; 3],
) -> Vec<CylinderPatchCurve> {
    let mut lifted = Vec::new();
    for &(a, b) in arcs {
        let sweep = b - a;
        if !sweep.is_finite() || sweep <= 0. {
            continue;
        }
        let theta = |phi: f64| {
            let rel = sub(point_at(phi), center);
            dot(rel, cone.frame[1])
                .atan2(dot(rel, cone.frame[0]))
                .rem_euclid(TAU)
        };
        let (ta, tb, tm) = (theta(a), theta(b), theta(a + sweep / 2.));
        let forward = (tb - ta).rem_euclid(TAU);
        let image = if (tm - ta).rem_euclid(TAU) <= forward {
            (ta, forward)
        } else {
            (tb, (ta - tb).rem_euclid(TAU))
        };
        for (quadrant, patch) in cone.sides.iter().enumerate() {
            let span = (quadrant as f64 * QUARTER, QUARTER);
            for (s0, sw) in ccw_intersect(span, image) {
                let rel0 = s0 - quadrant as f64 * QUARTER;
                let u0 = arc_parameter(rel0);
                let u1 = arc_parameter(rel0 + sw);
                if u1 - u0 <= 1e-12 {
                    continue;
                }
                lifted.push(CylinderPatchCurve {
                    patch: *patch,
                    arcs: vec![Curve {
                        degree: 1,
                        knots: vec![0., 0., 1., 1.],
                        control_points: vec![[u0, v0].to_vec(), [u1, v0].to_vec()],
                        weights: vec![1., 1.],
                        periodic: false,
                    }],
                });
            }
        }
    }
    lifted
}

/// Iso-v lines on all four side patches (full circle at height v0).
fn lift_circle_side_full(cone: &CanonicalCone, v0: f64) -> Vec<CylinderPatchCurve> {
    cone.sides
        .iter()
        .map(|&patch| CylinderPatchCurve {
            patch,
            arcs: vec![Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![[0., v0].to_vec(), [1., v0].to_vec()],
                weights: vec![1., 1.],
                periodic: false,
            }],
        })
        .collect()
}

/// Per-patch iso-u lift of a ruling: the line apex + t*w over t in [lo, hi]
/// sits at one fixed quadrant angle on the side wall (the angle comes from
/// the ruling direction, since the apex itself has no radial part); v is the
/// height fraction (t + t_apex) / h. Seam rulings are emitted on both
/// adjacent patches.
fn lift_ruling(cone: &CanonicalCone, w: [f64; 3], lo: f64, hi: f64) -> Vec<CylinderPatchCurve> {
    const SEAM: f64 = 1e-12;
    let t_apex = -cone.r_bottom / cone.slope;
    let axial = dot(w, cone.axis);
    let perp = sub(w, cone.axis.map(|x| x * axial));
    let angle = dot(perp, cone.frame[1])
        .atan2(dot(perp, cone.frame[0]))
        .rem_euclid(TAU);
    let u_global = angle / QUARTER;
    let quadrant_f = u_global.floor();
    let u_linear = u_global - quadrant_f;
    let quadrant = quadrant_f as usize % 4;
    let u = arc_parameter(u_linear * QUARTER);
    let v_lo = (lo + t_apex) / cone.height;
    let v_hi = (hi + t_apex) / cone.height;
    let segment = |u: f64| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![[u, v_lo].to_vec(), [u, v_hi].to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    };
    if u_linear <= SEAM {
        vec![
            CylinderPatchCurve {
                patch: cone.sides[quadrant],
                arcs: vec![segment(0.)],
            },
            CylinderPatchCurve {
                patch: cone.sides[(quadrant + 3) % 4],
                arcs: vec![segment(1.)],
            },
        ]
    } else if u_linear >= 1. - SEAM {
        vec![
            CylinderPatchCurve {
                patch: cone.sides[(quadrant + 1) % 4],
                arcs: vec![segment(0.)],
            },
            CylinderPatchCurve {
                patch: cone.sides[quadrant],
                arcs: vec![segment(1.)],
            },
        ]
    } else {
        vec![CylinderPatchCurve {
            patch: cone.sides[quadrant],
            arcs: vec![segment(u)],
        }]
    }
}

/// Clip the line `foot_uv + t * dir_uv` to `[lo0, hi0]` and the unit square.
/// Returns the surviving interval, or the reason it fails: misses are
/// silent, boundary-coincident lines, band-thin clips and patch-corner
/// touches are tangencies.
enum LineClip {
    Miss,
    Tangent,
    Span(f64, f64),
}

fn clip_line_rect(
    foot_uv: [f64; 2],
    dir_uv: [f64; 2],
    lo0: f64,
    hi0: f64,
    band: f64,
    band_uv: [f64; 2],
) -> LineClip {
    let mut lo = lo0;
    let mut hi = hi0;
    for k in 0..2 {
        let f = foot_uv[k];
        let d = dir_uv[k];
        if d.abs() <= 1e-30 {
            if f < -band_uv[k] || f > 1. + band_uv[k] {
                return LineClip::Miss;
            }
            if f.abs() <= band_uv[k] || (f - 1.).abs() <= band_uv[k] {
                return LineClip::Tangent;
            }
            continue;
        }
        let t0 = -f / d;
        let t1 = (1. - f) / d;
        lo = lo.max(t0.min(t1));
        hi = hi.min(t0.max(t1));
    }
    if hi < lo - band {
        return LineClip::Miss;
    }
    if hi - lo <= band {
        return LineClip::Tangent;
    }
    for t in [lo, hi] {
        let q = [foot_uv[0] + t * dir_uv[0], foot_uv[1] + t * dir_uv[1]];
        let near_u = q[0].abs() <= band_uv[0] || (q[0] - 1.).abs() <= band_uv[0];
        let near_v = q[1].abs() <= band_uv[1] || (q[1] - 1.).abs() <= band_uv[1];
        if near_u && near_v {
            return LineClip::Tangent;
        }
    }
    LineClip::Span(lo, hi)
}

/// Polar form (blossom) of the quadratic c0 + c1 t + c2 t^2 at (u, v).
fn polar(c: [f64; 3], u: f64, v: f64) -> f64 {
    c[0] + c[1] * (u + v) / 2. + c[2] * u * v
}

/// Exact rational quadratic Bezier (single span, knots [0,0,0,1,1,1]) of the
/// 3D rational quadratic p(t) = num(t)/den(t) over [t0, t1], via the polar
/// form of the homogeneous numerator/denominator — never a fit.
fn bezier_rational_3d(num: [[f64; 3]; 3], den: [f64; 3], t0: f64, t1: f64) -> Curve {
    let ws = [polar(den, t0, t0), polar(den, t0, t1), polar(den, t1, t1)];
    let ts = [(t0, t0), (t0, t1), (t1, t1)];
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|k| polar(num[k], ts[i].0, ts[i].1) / ws[i])
                    .collect()
            })
            .collect(),
        weights: ws.to_vec(),
        periodic: false,
    }
}

/// Exact rational quadratic Bezier of the 2D p(t) = num(t)/den(t) over
/// [t0, t1].
fn bezier_rational_2d(num: [[f64; 3]; 2], den: [f64; 3], t0: f64, t1: f64) -> Curve {
    let ws = [polar(den, t0, t0), polar(den, t0, t1), polar(den, t1, t1)];
    let ts = [(t0, t0), (t0, t1), (t1, t1)];
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|k| polar(num[k], ts[i].0, ts[i].1) / ws[i])
                    .collect()
            })
            .collect(),
        weights: ws.to_vec(),
        periodic: false,
    }
}

/// Clip the rational quadratic p(t) = num(t)/den(t) (2D coefficient arrays
/// c0 + c1 t + c2 t^2) on t in (lo, hi) — where den > 0 — by the half-planes
/// n.p <= offset. Each half-plane boundary crossing solves the exact
/// quadratic f(t) = n.num(t) - offset den(t) = 0; an extremum of a
/// constraint function within the half-plane's band of the boundary is a
/// tangency reported through the flag and never clipped through; surviving
/// spans are decided by exact midpoint evaluation; band-thin spans are
/// tangencies; nothing is merged across real gaps.
fn clip_conic_t(
    num: [[f64; 3]; 2],
    den: [f64; 3],
    lo: f64,
    hi: f64,
    halfplanes: &[Halfplane2],
    t_band: f64,
) -> (Vec<(f64, f64)>, bool) {
    let d = |t: f64| (den[2] * t + den[1]) * t + den[0];
    let p = |t: f64, k: usize| (num[k][2] * t + num[k][1]) * t + num[k][0];
    let coeffs = |hp: &Halfplane2| {
        (
            hp.normal[0] * num[0][2] + hp.normal[1] * num[1][2] - hp.offset * den[2],
            hp.normal[0] * num[0][1] + hp.normal[1] * num[1][1] - hp.offset * den[1],
            hp.normal[0] * num[0][0] + hp.normal[1] * num[1][0] - hp.offset * den[0],
        )
    };
    let mut cuts: Vec<f64> = Vec::new();
    let mut tangent = false;
    for hp in halfplanes {
        let (a, b, c) = coeffs(hp);
        if !a.is_finite() || !b.is_finite() || !c.is_finite() {
            return (Vec::new(), true);
        }
        let f = |t: f64| (a * t + b) * t + c;
        let band_f = |t: f64| hp.band * d(t).abs();
        let mid = (lo + hi) / 2.;
        if a == 0. {
            if b == 0. {
                if c.abs() <= band_f(mid) {
                    tangent = true;
                } else if c > 0. {
                    return (Vec::new(), tangent);
                }
                continue;
            }
            cuts.push(-c / b);
            continue;
        }
        let t_ext = -b / (2. * a);
        let f_ext = f(t_ext);
        // An extremum within the band of the boundary: a tangency, never
        // clipped through.
        if f_ext.abs() <= band_f(t_ext) {
            tangent = true;
            continue;
        }
        if a > 0. && f_ext > 0. {
            // Violated everywhere: the conic misses this region.
            return (Vec::new(), tangent);
        }
        if a < 0. && f_ext < 0. {
            // Satisfied everywhere: the constraint never binds.
            continue;
        }
        let disc = b * b - 4. * a * c;
        if !disc.is_finite() || disc <= 0. {
            if f_ext > 0. {
                return (Vec::new(), tangent);
            }
            continue;
        }
        let root = disc.sqrt();
        let q = if b >= 0. {
            -0.5 * (b + root)
        } else {
            -0.5 * (b - root)
        };
        cuts.push(q / a);
        if q != 0. {
            cuts.push(c / q);
        } else {
            cuts.push(0.);
        }
    }
    let inside = |t: f64| {
        halfplanes.iter().all(|hp| {
            let (a, b, c) = coeffs(hp);
            let dv = d(t);
            let f = (a * t + b) * t + c;
            let scale = (hp.normal[0].abs() * p(t, 0).abs()
                + hp.normal[1].abs() * p(t, 1).abs()
                + hp.offset.abs() * dv.abs())
                / dv.abs().max(1e-300)
                + 1.;
            f / dv <= 1e-12 * scale
        })
    };
    if cuts.is_empty() {
        if hi - lo > t_band {
            return (vec![(lo, hi)], tangent);
        }
        return (Vec::new(), true);
    }
    cuts.retain(|t| *t > lo && *t < hi && t.is_finite());
    cuts.sort_by(f64::total_cmp);
    cuts.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * (1. + a.abs()));
    let mut arcs: Vec<(f64, f64)> = Vec::new();
    let mut start = lo;
    for &cut in cuts.iter().chain(std::iter::once(&hi)) {
        let end = cut;
        if end - start > 1e-14 * (1. + start.abs()) && inside((start + end) / 2.) {
            arcs.push((start, end));
        }
        start = end;
    }
    // Merge spans split only by a phantom (sub-rounding) cut.
    let mut merged: Vec<(f64, f64)> = Vec::new();
    for arc in arcs {
        if let Some(last) = merged.last_mut()
            && arc.0 - last.1 <= 1e-12 * (1. + last.1.abs())
        {
            last.1 = arc.1;
            continue;
        }
        merged.push(arc);
    }
    let mut out = Vec::new();
    for (a, b) in merged {
        if b - a <= t_band {
            // Band-thin clip: a boundary touch, never a degenerate arc.
            tangent = true;
            continue;
        }
        out.push((a, b));
    }
    (out, tangent)
}

/// Analytic plane/cone intersection of a canonical planar patch and a
/// canonical conical frustum solid (true-apex cone admitted). Non-canonical
/// operands — including an equal-radius frustum (a cylinder) — are explicit
/// `UnsupportedSurface` regions, never a numerical fallback; tangencies
/// (ring planes, the apex, the parabola threshold band, boundary touches)
/// stay unresolved — tangent contacts are never guessed.
pub fn intersect_plane_cone(
    plane_model: &Model,
    cone_model: &Model,
    options: Options,
) -> Result<Report<PlaneConeComponent>> {
    let options = options.validate()?;
    let _ = options;
    let mut report = Report::default();
    let domain = vec![0., 1., 0., 1., 0., 1., 0., 1.];
    let (Some(plane), Some(cone)) = (recognize_plane(plane_model)?, recognize_cone(cone_model)?)
    else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    report.boxes_visited = 1;
    let h = cone.height;
    let m = cone.slope;
    let k2 = 1. + m * m;
    let sigma = m.abs() / k2.sqrt();
    let g = dot(cone.axis, plane.normal);
    let ga = g.abs();
    let terms = cone.r_bottom
        + cone.r_top
        + h
        + plane.u_len
        + plane.v_len
        + sub(cone.bottom, plane.origin)
            .iter()
            .map(|v| v.abs())
            .fold(0., f64::max)
        + 1.;
    // Outward binary64 classification band: both recognition deviations plus
    // a rounding allowance on the section arithmetic and the dimensions.
    let band = plane.error + cone.error + 16. * f64::EPSILON * terms;
    // Angular certification: pure-rounding tilts snap, recognition-scale
    // tilts report near_coincidence, clearly oblique angles are exact.
    let angular_snap =
        64. * f64::EPSILON + plane.error / plane.u_len.min(plane.v_len) + cone.error / h;
    let rect = rect_halfplanes(&plane, band);
    // Axial coordinate of the apex from the bottom ring center.
    let t_apex = -cone.r_bottom / m;
    let apex: [f64; 3] = std::array::from_fn(|i| cone.bottom[i] + t_apex * cone.axis[i]);
    if ga >= 1. - angular_snap {
        // Plane perpendicular to the axis: the exact circle at the section
        // height, radius linearly interpolated between the rings.
        let t_s = dot(sub(plane.origin, cone.bottom), plane.normal) / g;
        if !t_s.is_finite() {
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        if t_s < -band || t_s > h + band {
            return Ok(report);
        }
        if t_s.abs() <= band {
            // Bottom ring plane within the band: a coincident cap/rim region
            // (nonzero ring) or the apex contact (apex ring).
            let reason = if cone.r_bottom > band {
                UnresolvedReason::CoincidentTrim
            } else {
                UnresolvedReason::TangencyOrMultipleRoot
            };
            report.unresolved(domain, reason);
            return Ok(report);
        }
        if (t_s - h).abs() <= band {
            let reason = if cone.r_top > band {
                UnresolvedReason::CoincidentTrim
            } else {
                UnresolvedReason::TangencyOrMultipleRoot
            };
            report.unresolved(domain, reason);
            return Ok(report);
        }
        let r_s = cone.r_bottom + m * t_s;
        if !r_s.is_finite() || r_s <= band {
            // Section at the apex level: a point contact, never guessed.
            report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
            return Ok(report);
        }
        let center = std::array::from_fn(|i| cone.bottom[i] + t_s * cone.axis[i]);
        let e1 = plane.u.map(|x| x / plane.u_len);
        let e2 = plane.v.map(|x| x / plane.v_len);
        let [uc, vc] = plane_uv(&plane, center);
        let w1 = [r_s / plane.u_len, 0.];
        let w2 = [0., r_s / plane.v_len];
        let (clip, tangent) = clip_ellipse([uc, vc], w1, w2, &rect);
        if tangent {
            report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
        }
        let v0 = t_s / h;
        let point_at = |phi: f64| {
            std::array::from_fn::<f64, 3, _>(|i| {
                center[i] + r_s * (phi.cos() * e1[i] + phi.sin() * e2[i])
            })
        };
        let we1 = e1.map(|x| x * r_s);
        let we2 = e2.map(|x| x * r_s);
        match clip {
            EllipseClip::Empty => {}
            EllipseClip::Full if !tangent => {
                let curve = conic_sweep(center, we1, we2, 0., TAU);
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 16)?;
                report.components.push(PlaneConeComponent::Circle {
                    curve,
                    center,
                    radius: r_s,
                    normal: plane.normal,
                    full: true,
                    plane_uv: vec![PlanePatchCurve {
                        patch: 0,
                        arcs: conic_arcs_2d([uc, vc], w1, w2, 0., TAU),
                    }],
                    cone_uv: lift_circle_side_full(&cone, v0),
                    max_sample_residual,
                });
            }
            EllipseClip::Arcs(intervals) => {
                for (a, b) in intervals {
                    let curve = conic_sweep(center, we1, we2, a, b);
                    let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                    report.components.push(PlaneConeComponent::Circle {
                        curve,
                        center,
                        radius: r_s,
                        normal: plane.normal,
                        full: false,
                        plane_uv: vec![PlanePatchCurve {
                            patch: 0,
                            arcs: conic_arcs_2d([uc, vc], w1, w2, a, b),
                        }],
                        cone_uv: lift_circle_side(&cone, center, v0, &[(a, b)], point_at),
                        max_sample_residual,
                    });
                }
            }
            // Tangency within the band: the full circle is withheld; the
            // unresolved region above carries the contact.
            EllipseClip::Full => {}
        }
        return Ok(report);
    }
    // Conic kind classification on the outward bands.
    #[derive(Clone, Copy, PartialEq)]
    enum Conic {
        Hyperbola,
        Parabola,
        Ellipse,
    }
    let mut g = g;
    let mut s = (1. - g * g).max(0.).sqrt();
    let kind: Conic;
    if ga <= angular_snap {
        // Plane parallel to the axis. Through the axis (the apex on the
        // plane within the band): the two exact straight rulings.
        let d0 = dot(plane.normal, sub(plane.origin, apex));
        if d0.abs() <= band {
            let e2 = cross(plane.normal, cone.axis);
            let e2_len = hypot3(e2);
            if !e2_len.is_finite() || e2_len <= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let e2 = e2.map(|x| x / e2_len);
            let wlen = k2.sqrt();
            let band_uv = [band / plane.u_len, band / plane.v_len];
            let apex_uv = plane_uv(&plane, apex);
            for sign in [-1., 1.] {
                let w: [f64; 3] = std::array::from_fn(|i| cone.axis[i] + sign * m * e2[i]);
                let dir_uv = [
                    dot(w, plane.u) / (plane.u_len * plane.u_len),
                    dot(w, plane.v) / (plane.v_len * plane.v_len),
                ];
                match clip_line_rect(apex_uv, dir_uv, -t_apex, h - t_apex, band / wlen, band_uv) {
                    LineClip::Miss => {}
                    LineClip::Tangent => {
                        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                    }
                    LineClip::Span(lo, hi) => {
                        let start: [f64; 3] = std::array::from_fn(|i| apex[i] + lo * w[i]);
                        let end: [f64; 3] = std::array::from_fn(|i| apex[i] + hi * w[i]);
                        let curve = Curve {
                            degree: 1,
                            knots: vec![0., 0., 1., 1.],
                            control_points: vec![start.to_vec(), end.to_vec()],
                            weights: vec![1., 1.],
                            periodic: false,
                        };
                        let cone_uv = lift_ruling(&cone, w, lo, hi);
                        let plane_uv = vec![PlanePatchCurve {
                            patch: 0,
                            arcs: vec![Curve {
                                degree: 1,
                                knots: vec![0., 0., 1., 1.],
                                control_points: vec![
                                    [apex_uv[0] + lo * dir_uv[0], apex_uv[1] + lo * dir_uv[1]]
                                        .to_vec(),
                                    [apex_uv[0] + hi * dir_uv[0], apex_uv[1] + hi * dir_uv[1]]
                                        .to_vec(),
                                ],
                                weights: vec![1., 1.],
                                periodic: false,
                            }],
                        }];
                        let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                        report.components.push(PlaneConeComponent::Line {
                            curve,
                            start,
                            end,
                            direction: w.map(|x| x / wlen),
                            contact: Contact::Boundary,
                            plane_uv,
                            cone_uv,
                            max_sample_residual,
                        });
                    }
                }
            }
            return Ok(report);
        }
        // Off-axis parallel plane: the exact hyperbola with g = 0, s = 1.
        g = 0.;
        s = 1.;
        kind = Conic::Hyperbola;
    } else {
        let off_para = (ga - sigma).abs();
        if off_para <= angular_snap {
            // Pure-rounding distance from the parabola threshold: snap to the
            // exact parabola (a plane built exactly parallel to a ruling).
            g = sigma.copysign(g);
            s = (1. - sigma * sigma).sqrt();
            kind = Conic::Parabola;
        } else {
            let near = RECOGNITION + plane.error / plane.u_len.min(plane.v_len) + cone.error / h;
            if ga >= 1. - near || ga <= near {
                // Recognition-scale tilt: cannot be certified — never forced.
                report.unresolved(domain, UnresolvedReason::NearCoincidence);
                return Ok(report);
            }
            if off_para <= near {
                // The parabola threshold band: a degenerate multiple-root
                // boundary, never a guessed conic.
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            kind = if ga > sigma {
                Conic::Ellipse
            } else {
                Conic::Hyperbola
            };
        }
    }
    // Shared oblique geometry, apex-based: plane points are
    // P = Q + x e1 + y e2 with Q the apex projection, e1 the in-plane axis
    // projection; the cone side is A2 x^2 - L x + y^2 + C = 0.
    let d0 = dot(plane.normal, sub(plane.origin, apex));
    if d0.abs() <= band {
        // Plane through the apex region: the apex is a singular point of
        // the side surface — a multiple-root contact, never guessed (the
        // exact through-axis ruling case returned above).
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1_raw = sub(cone.axis, plane.normal.map(|x| x * g));
    let e1_len = hypot3(e1_raw);
    if !e1_len.is_finite() || e1_len <= 0. {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    let e1 = e1_raw.map(|x| x / e1_len);
    let e2 = cross(plane.normal, e1);
    let q: [f64; 3] = std::array::from_fn(|i| apex[i] + d0 * plane.normal[i]);
    let a2 = k2 * (g * g - sigma * sigma);
    let ll = 2. * k2 * s * g * d0;
    let cc = d0 * d0 * (1. - k2 * g * g);
    if !a2.is_finite() || !ll.is_finite() || !cc.is_finite() {
        report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
        return Ok(report);
    }
    match kind {
        Conic::Ellipse => {
            if a2 <= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x_c = ll / (2. * a2);
            let r2 = ll * ll / (4. * a2) - cc;
            if !r2.is_finite() {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            if r2 <= band * band {
                if r2 < -band * band {
                    // Provable miss of the double cone: resolved empty.
                    return Ok(report);
                }
                // Degenerate point contact: never a guessed point.
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let ae = (r2 / a2).sqrt();
            let be = r2.sqrt();
            let center3: [f64; 3] = std::array::from_fn(|i| q[i] + x_c * e1[i]);
            // Finite height, nappe and patch constraints are all linear in
            // the plane UV: one exact eccentric-angle clip.
            let x_q = dot(sub(plane.origin, q), e1);
            let xu = dot(plane.u, e1);
            let xv = dot(plane.v, e1);
            let mut halfplanes = rect.to_vec();
            halfplanes.push(Halfplane2 {
                normal: [s * xu, s * xv],
                offset: h - t_apex - g * d0 - s * x_q,
                band,
            });
            halfplanes.push(Halfplane2 {
                normal: [-s * xu, -s * xv],
                offset: t_apex + g * d0 + s * x_q,
                band,
            });
            halfplanes.push(Halfplane2 {
                normal: [-m * s * xu, -m * s * xv],
                offset: m * (g * d0 + s * x_q),
                band,
            });
            let c2 = plane_uv(&plane, center3);
            let w1 = [
                ae * dot(e1, plane.u) / (plane.u_len * plane.u_len),
                ae * dot(e1, plane.v) / (plane.v_len * plane.v_len),
            ];
            let w2 = [
                be * dot(e2, plane.u) / (plane.u_len * plane.u_len),
                be * dot(e2, plane.v) / (plane.v_len * plane.v_len),
            ];
            let (clip, tangent) = clip_ellipse(c2, w1, w2, &halfplanes);
            if tangent {
                report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
            }
            let (semi_major, major, semi_minor, minor) = if ae >= be {
                (ae, e1, be, e2)
            } else {
                (be, e2, ae, e1)
            };
            let we1 = e1.map(|x| x * ae);
            let we2 = e2.map(|x| x * be);
            let mut push_ellipse = |start: f64, end: f64, full: bool| -> Result<()> {
                let curve = conic_sweep(center3, we1, we2, start, end);
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                report.components.push(PlaneConeComponent::Ellipse {
                    curve,
                    center: center3,
                    semi_major,
                    semi_minor,
                    major,
                    minor,
                    normal: plane.normal,
                    full,
                    plane_uv: vec![PlanePatchCurve {
                        patch: 0,
                        arcs: conic_arcs_2d(c2, w1, w2, start, end),
                    }],
                    cone_uv: None,
                    max_sample_residual,
                });
                Ok(())
            };
            match clip {
                EllipseClip::Empty => {}
                EllipseClip::Full if !tangent => push_ellipse(0., TAU, true)?,
                EllipseClip::Arcs(intervals) => {
                    for (a, b) in intervals {
                        push_ellipse(a, b, false)?;
                    }
                }
                EllipseClip::Full => {}
            }
            Ok(report)
        }
        Conic::Parabola => {
            // y^2 = L x - C: the exact polynomial quadratic
            // p(t) = (x0 + t^2 / L, t), weights 1.
            if ll.abs() <= 1e-30 * (k2 * s * d0.abs() + 1.) {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x0 = cc / ll;
            let num = [[x0, 0., 1. / ll], [0., 1., 0.]];
            let den = [1., 0., 0.];
            // Finite height, nappe and patch half-planes in (x, y).
            let hps = conic_halfplanes(&plane, &cone, &rect, q, e1, e2, g, s, d0, t_apex, band);
            let big = 1e6 * terms;
            let t_band = band + 1e-12 * terms;
            let (intervals, tangent) = clip_conic_t(num, den, -big, big, &hps, t_band);
            if tangent {
                report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
            }
            let vertex: [f64; 3] = std::array::from_fn(|i| q[i] + x0 * e1[i]);
            let direction = if ll > 0. { e1 } else { e1.map(|x| -x) };
            for (t0, t1) in intervals {
                let curve = bezier_rational_3d(conic_num3(q, e1, e2, num, den), den, t0, t1);
                let plane_uv = vec![PlanePatchCurve {
                    patch: 0,
                    arcs: vec![bezier_rational_2d(
                        conic_num_uv(&plane, q, e1, e2, num, den),
                        den,
                        t0,
                        t1,
                    )],
                }];
                let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                report.components.push(PlaneConeComponent::Parabola {
                    curve,
                    vertex,
                    direction,
                    focal_length: ll.abs() / 4.,
                    normal: plane.normal,
                    plane_uv,
                    cone_uv: None,
                    max_sample_residual,
                });
            }
            Ok(report)
        }
        Conic::Hyperbola => {
            if a2 >= 0. {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let x_c = ll / (2. * a2);
            let hh = cc - a2 * x_c * x_c;
            if !hh.is_finite() {
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            if hh <= band * band {
                if hh < -band * band {
                    // Provable miss of the double cone: resolved empty.
                    return Ok(report);
                }
                report.unresolved(domain, UnresolvedReason::TangencyOrMultipleRoot);
                return Ok(report);
            }
            let ah = (hh / (-a2)).sqrt();
            let bh = hh.sqrt();
            let center3: [f64; 3] = std::array::from_fn(|i| q[i] + x_c * e1[i]);
            let hps = conic_halfplanes(&plane, &cone, &rect, q, e1, e2, g, s, d0, t_apex, band);
            for br in [-1., 1.] {
                // x = x_c + br a (1+t^2)/(1-t^2), y = 2 b t/(1-t^2), |t| < 1.
                let num = [[x_c + br * ah, 0., br * ah - x_c], [0., 2. * bh, 0.]];
                let den = [1., 0., -1.];
                let (intervals, tangent) = clip_conic_t(num, den, -1., 1., &hps, 1e-9);
                if tangent {
                    report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                }
                for (t0, t1) in intervals {
                    if t0 < -1. + 1e-9 || t1 > 1. - 1e-9 {
                        // Arc runs into the asymptote: never a guessed
                        // unbounded trim.
                        report.unresolved(domain.clone(), UnresolvedReason::TangencyOrMultipleRoot);
                        continue;
                    }
                    let curve = bezier_rational_3d(conic_num3(q, e1, e2, num, den), den, t0, t1);
                    let plane_uv = vec![PlanePatchCurve {
                        patch: 0,
                        arcs: vec![bezier_rational_2d(
                            conic_num_uv(&plane, q, e1, e2, num, den),
                            den,
                            t0,
                            t1,
                        )],
                    }];
                    let max_sample_residual = side_plane_residuals(&curve, &cone, &plane, 9)?;
                    report.components.push(PlaneConeComponent::Hyperbola {
                        curve,
                        center: center3,
                        semi_transverse: ah,
                        semi_conjugate: bh,
                        transverse: e1.map(|x| x * br),
                        conjugate: e2,
                        normal: plane.normal,
                        plane_uv,
                        cone_uv: None,
                        max_sample_residual,
                    });
                }
            }
            Ok(report)
        }
    }
}

/// Finite height, nappe and patch-rectangle half-planes in the plane's
/// (x, y) coordinates (x along the in-plane axis projection e1, y along
/// e2 = normal x e1, origin at the apex projection Q).
#[allow(clippy::too_many_arguments)]
fn conic_halfplanes(
    plane: &CanonicalPlane,
    cone: &CanonicalCone,
    rect: &[Halfplane2; 4],
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    g: f64,
    s: f64,
    d0: f64,
    t_apex: f64,
    band: f64,
) -> Vec<Halfplane2> {
    let m = cone.slope;
    let mut hps = vec![
        // Height: 0 <= t_apex + g d0 + s x <= h.
        Halfplane2 {
            normal: [s, 0.],
            offset: cone.height - t_apex - g * d0,
            band,
        },
        Halfplane2 {
            normal: [-s, 0.],
            offset: t_apex + g * d0,
            band,
        },
        // Nappe: m (g d0 + s x) >= 0.
        Halfplane2 {
            normal: [-m * s, 0.],
            offset: m * g * d0,
            band,
        },
    ];
    // Patch rectangle edges, affine in (x, y): u = u_q + x au + y bu.
    let u2 = plane.u_len * plane.u_len;
    let v2 = plane.v_len * plane.v_len;
    let au = dot(e1, plane.u) / u2;
    let bu = dot(e2, plane.u) / u2;
    let av = dot(e1, plane.v) / v2;
    let bv = dot(e2, plane.v) / v2;
    let u_q = dot(sub(q, plane.origin), plane.u) / u2;
    let v_q = dot(sub(q, plane.origin), plane.v) / v2;
    let _ = rect;
    hps.push(Halfplane2 {
        normal: [au, bu],
        offset: 1. - u_q,
        band: band / plane.u_len,
    });
    hps.push(Halfplane2 {
        normal: [-au, -bu],
        offset: u_q,
        band: band / plane.u_len,
    });
    hps.push(Halfplane2 {
        normal: [av, bv],
        offset: 1. - v_q,
        band: band / plane.v_len,
    });
    hps.push(Halfplane2 {
        normal: [-av, -bv],
        offset: v_q,
        band: band / plane.v_len,
    });
    hps
}

/// Homogeneous 3D numerator of Q + x(t) e1 + y(t) e2 with
/// (x, y) = num / den.
fn conic_num3(
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    num: [[f64; 3]; 2],
    den: [f64; 3],
) -> [[f64; 3]; 3] {
    std::array::from_fn(|k| {
        [
            q[k] * den[0] + e1[k] * num[0][0] + e2[k] * num[1][0],
            q[k] * den[1] + e1[k] * num[0][1] + e2[k] * num[1][1],
            q[k] * den[2] + e1[k] * num[0][2] + e2[k] * num[1][2],
        ]
    })
}

/// Homogeneous plane-UV numerator of the conic (x, y) = num / den.
fn conic_num_uv(
    plane: &CanonicalPlane,
    q: [f64; 3],
    e1: [f64; 3],
    e2: [f64; 3],
    num: [[f64; 3]; 2],
    den: [f64; 3],
) -> [[f64; 3]; 2] {
    let u2 = plane.u_len * plane.u_len;
    let v2 = plane.v_len * plane.v_len;
    let au = dot(e1, plane.u) / u2;
    let bu = dot(e2, plane.u) / u2;
    let av = dot(e1, plane.v) / v2;
    let bv = dot(e2, plane.v) / v2;
    let u_q = dot(sub(q, plane.origin), plane.u) / u2;
    let v_q = dot(sub(q, plane.origin), plane.v) / v2;
    [
        std::array::from_fn(|i| u_q * den[i] + au * num[0][i] + bu * num[1][i]),
        std::array::from_fn(|i| v_q * den[i] + av * num[0][i] + bv * num[1][i]),
    ]
}

impl value_codec::Serialize for PlaneConeComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"circle","curve":curve,"center":center,"radius":radius,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => {
                let contact = match contact {
                    Contact::Transverse => "transverse",
                    Contact::Boundary => "boundary",
                };
                value_codec::json!({"kind":"line","curve":curve,"start":start,"end":end,
                    "direction":direction,"contact":contact,"planeUv":plane_uv,
                    "coneUv":cone_uv,"maxSampleResidual":max_sample_residual})
            }
            Self::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"ellipse","curve":curve,"center":center,
                "semiMajor":semi_major,"semiMinor":semi_minor,"major":major,"minor":minor,
                "normal":normal,"full":full,"planeUv":plane_uv,"coneUv":cone_uv,
                "maxSampleResidual":max_sample_residual}),
            Self::Parabola {
                curve,
                vertex,
                direction,
                focal_length,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"parabola","curve":curve,"vertex":vertex,
                "direction":direction,"focalLength":focal_length,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
            Self::Hyperbola {
                curve,
                center,
                semi_transverse,
                semi_conjugate,
                transverse,
                conjugate,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } => value_codec::json!({"kind":"hyperbola","curve":curve,"center":center,
                "semiTransverse":semi_transverse,"semiConjugate":semi_conjugate,
                "transverse":transverse,"conjugate":conjugate,"normal":normal,
                "planeUv":plane_uv,"coneUv":cone_uv,"maxSampleResidual":max_sample_residual}),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::plane_sphere::plane_patch;
    use super::*;

    fn rotated_translated(model: &Model, angle: f64, offset: [f64; 3]) -> Model {
        let (sin, cos) = angle.sin_cos();
        crate::transform::affine(
            model,
            [
                [1., 0., 0., offset[0]],
                [0., cos, -sin, offset[1]],
                [0., sin, cos, offset[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }

    fn point_of(jet: &[f64]) -> [f64; 3] {
        [jet[0], jet[1], jet[2]]
    }

    /// Worst residual of the UV lifts evaluated through their own surfaces
    /// against the cone side and plane equations.
    fn lift_worst(
        plane_model: &Model,
        cone_model: &Model,
        plane_uv: &[PlanePatchCurve],
        cone_uv: &[CylinderPatchCurve],
        cone_def: ([f64; 3], [f64; 3], f64, f64),
        plane_def: ([f64; 3], [f64; 3]),
    ) -> f64 {
        let check = |p: [f64; 3]| {
            let rel = sub(p, cone_def.0);
            let a = dot(rel, cone_def.1);
            let perp = sub(rel, cone_def.1.map(|x| x * a));
            (hypot3(perp) - (cone_def.2 + cone_def.3 * a))
                .abs()
                .max(dot(plane_def.0, sub(p, plane_def.1)).abs())
        };
        let mut worst = 0_f64;
        for lift in plane_uv {
            let surface = &plane_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        for lift in cone_uv {
            let surface = &cone_model.faces[lift.patch].surface;
            for arc in &lift.arcs {
                for k in 0..=8 {
                    let uv = arc.evaluate(k as f64 / 8.).unwrap().point;
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[0]), "{uv:?}");
                    assert!((-1e-9..=1. + 1e-9).contains(&uv[1]), "{uv:?}");
                    let p = surface
                        .evaluate(uv[0].clamp(0., 1.), uv[1].clamp(0., 1.))
                        .unwrap()
                        .point;
                    worst = worst.max(check([p[0], p[1], p[2]]));
                }
            }
        }
        worst
    }

    #[test]
    fn perpendicular_circle_interpolates_radius_between_rings() {
        // Frustum r 3 -> 1 over z 0..5, plane z = 2.5 patch [-4,4]^2: the
        // exact circle of the linearly interpolated radius 2 at mid-height.
        let plane = plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]);
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(!report.permits_topology_change());
        let [
            PlaneConeComponent::Circle {
                curve,
                center,
                radius,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one circle component: {report:?}")
        };
        assert!(*full);
        assert!((*radius - 2.).abs() <= 1e-12, "{radius}");
        assert!(
            sub(*center, [0., 0., 2.5]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert_eq!(*normal, [0., 0., 1.]);
        assert_eq!(
            curve.knots,
            vec![0., 0., 0., 1., 1., 2., 2., 3., 3., 4., 4., 4.]
        );
        assert_eq!(curve.weights.len(), 9);
        assert!((curve.weights[1] - std::f64::consts::FRAC_1_SQRT_2).abs() <= 1e-15);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        assert_eq!(plane_uv[0].arcs.len(), 4);
        // Cone UV: iso-v rulings at mid-height on all four side patches.
        assert_eq!(cone_uv.len(), 4);
        for lift in cone_uv {
            assert_eq!(lift.arcs.len(), 1);
            let arc = &lift.arcs[0];
            assert_eq!(arc.degree, 1);
            assert_eq!(arc.control_points[0], vec![0., 0.5]);
            assert_eq!(arc.control_points[1], vec![1., 0.5]);
        }
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            cone_uv,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            ([0., 0., 1.], [-4., -4., 2.5]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn perpendicular_clipped_circle_has_exact_trims() {
        // Plane z = 2.5, patch x in [0, 4], y in [-4, 4]: the radius-2 circle
        // clipped to its x >= 0 half, swept [3pi/2, 5pi/2].
        let plane = plane_patch([0., -4., 2.5], [4., 0., 0.], [0., 8., 0.]);
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneConeComponent::Circle {
                curve,
                center,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
                ..
            },
        ] = &report.components[..]
        else {
            panic!("expected one clipped circle: {report:?}")
        };
        assert!(!*full);
        assert!(
            sub(*center, [0., 0., 2.5]).iter().all(|x| x.abs() <= 1e-12),
            "{center:?}"
        );
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 2., 2., 2.]);
        let start = point_of(&curve.evaluate(0.).unwrap().point);
        let end = point_of(&curve.evaluate(2.).unwrap().point);
        assert!(
            start[0].abs() <= 1e-12 && (start[1] + 2.).abs() <= 1e-12,
            "{start:?}"
        );
        assert!((start[2] - 2.5).abs() <= 1e-12);
        assert!(
            end[0].abs() <= 1e-12 && (end[1] - 2.).abs() <= 1e-12,
            "{end:?}"
        );
        assert!((end[2] - 2.5).abs() <= 1e-12);
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Plane UV: two arcs meeting at the phi = 0 seam (1/2, 1/2); the free
        // endpoints sit exactly on u = 0 at v = 1/4 and v = 3/4.
        assert_eq!(plane_uv[0].arcs.len(), 2);
        let mut edge_v: Vec<f64> = Vec::new();
        let mut seam: Vec<[f64; 2]> = Vec::new();
        for arc in &plane_uv[0].arcs {
            for endpoint in [&arc.control_points[0], &arc.control_points[2]] {
                if endpoint[0].abs() <= 1e-12 {
                    edge_v.push(endpoint[1]);
                } else {
                    seam.push([endpoint[0], endpoint[1]]);
                }
            }
        }
        edge_v.sort_by(f64::total_cmp);
        assert_eq!(edge_v.len(), 2, "{edge_v:?}");
        assert!((edge_v[0] - 0.25).abs() <= 1e-12, "{edge_v:?}");
        assert!((edge_v[1] - 0.75).abs() <= 1e-12, "{edge_v:?}");
        assert_eq!(seam.len(), 2, "{seam:?}");
        for p in &seam {
            assert!((p[0] - 0.5).abs() <= 1e-12, "{p:?}");
            assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
        }
        assert_eq!(cone_uv.len(), 2, "{cone_uv:?}");
        for lift in cone_uv {
            for arc in &lift.arcs {
                assert_eq!(arc.degree, 1);
                assert!((arc.control_points[0][1] - 0.5).abs() <= 1e-12);
                assert!((arc.control_points[1][1] - 0.5).abs() <= 1e-12);
            }
        }
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            cone_uv,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            ([0., 0., 1.], [0., -4., 2.5]),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn ring_planes_apex_level_and_beyond_classification() {
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let at = |z: f64| plane_patch([-4., -4., z], [8., 0., 0.], [0., 8., 0.]);
        // Plane coincident with a ring plane: coincident cap/rim region,
        // never a guessed circle.
        for z in [0., 5.] {
            let report = intersect_plane_cone(&at(z), &cone, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{z} {report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::CoincidentTrim
            );
        }
        // Provably beyond both rings: empty and resolved.
        for z in [6., -1., 5. + 1e-9, -1e-9] {
            let report = intersect_plane_cone(&at(z), &cone, Options::default()).unwrap();
            assert!(
                report.components.is_empty() && report.unresolved.is_empty(),
                "{z} {report:?}"
            );
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
        }
        // Just clear of the top ring band below: a full circle near the rim,
        // radius 1 + 0.4e-9 by linear interpolation.
        let report = intersect_plane_cone(&at(5. - 1e-9), &cone, Options::default()).unwrap();
        let [
            PlaneConeComponent::Circle {
                center,
                radius,
                full,
                ..
            },
        ] = &report.components[..]
        else {
            panic!("expected one circle: {report:?}")
        };
        assert!(*full);
        assert!((radius - (1. + 0.4e-9)).abs() <= 1e-12, "{radius}");
        assert!((center[2] - (5. - 1e-9)).abs() <= 1e-12, "{center:?}");
        // Apex cone (r_top == 0): the apex-level perpendicular plane is a
        // tangency, never a point component; a mid section is the circle.
        let apex_cone = crate::analytic::frustum(3., 0., 5.).unwrap();
        let report = intersect_plane_cone(&at(5.), &apex_cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        let report = intersect_plane_cone(&at(2.5), &apex_cone, Options::default()).unwrap();
        let [
            PlaneConeComponent::Circle {
                radius, cone_uv, ..
            },
        ] = &report.components[..]
        else {
            panic!("expected one circle on the apex cone: {report:?}")
        };
        assert!((*radius - 1.5).abs() <= 1e-12, "{radius}");
        assert_eq!(cone_uv.len(), 4);
    }

    #[test]
    fn through_axis_plane_yields_two_exact_rulings() {
        // Frustum r 3 -> 1 over z 0..5 (apex at z = 7.5), plane y = 0
        // through the axis: the two exact rulings (+-3, 0, 0) -> (+-1, 0, 5).
        let plane = plane_patch([-5., 0., -2.], [10., 0., 0.], [0., 0., 9.]);
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 2, "{report:?}");
        let wlen = 1.16_f64.sqrt();
        let mut seen: Vec<f64> = Vec::new();
        for component in &report.components {
            let PlaneConeComponent::Line {
                curve,
                start,
                end,
                direction,
                contact,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } = component
            else {
                panic!("expected line components: {report:?}")
            };
            assert_eq!(*contact, Contact::Boundary);
            assert_eq!(curve.degree, 1);
            assert_eq!(curve.knots, vec![0., 0., 1., 1.]);
            assert!((start[0].abs() - 3.).abs() <= 1e-12, "{start:?}");
            assert!(
                start[1].abs() <= 1e-12 && start[2].abs() <= 1e-12,
                "{start:?}"
            );
            assert!((end[0].abs() - 1.).abs() <= 1e-12, "{end:?}");
            assert!(
                end[1].abs() <= 1e-12 && (end[2] - 5.).abs() <= 1e-12,
                "{end:?}"
            );
            assert_eq!(start[0].signum(), end[0].signum());
            // Unit ruling direction (-/+0.4, 0, 1) / sqrt(1.16).
            assert!(
                (direction[0].abs() - 0.4 / wlen).abs() <= 1e-12,
                "{direction:?}"
            );
            assert!((direction[2] - 1. / wlen).abs() <= 1e-12, "{direction:?}");
            seen.push(start[0]);
            assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
            assert!(!plane_uv.is_empty() && !cone_uv.is_empty());
            // Cone UV: iso-u rulings spanning the full height fraction 0..1.
            for lift in cone_uv {
                for arc in &lift.arcs {
                    assert_eq!(arc.degree, 1);
                    assert!(arc.control_points[0][1].abs() <= 1e-12, "{arc:?}");
                    assert!((arc.control_points[1][1] - 1.).abs() <= 1e-12, "{arc:?}");
                }
            }
            let worst = lift_worst(
                &plane,
                &cone,
                plane_uv,
                cone_uv,
                ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
                ([0., 1., 0.], [-5., 0., -2.]),
            );
            assert!(worst <= 1e-9, "{worst}");
        }
        seen.sort_by(f64::total_cmp);
        assert!(seen[0] < 0. && seen[1] > 0., "{seen:?}");
        // Apex cone: the rulings meet at the apex (0, 0, 5) on the model.
        let apex_cone = crate::analytic::frustum(3., 0., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &apex_cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.components.len(), 2, "{report:?}");
        for component in &report.components {
            let PlaneConeComponent::Line { start, end, .. } = component else {
                panic!("expected lines: {report:?}")
            };
            assert!(
                (start[0].abs() - 3.).abs() <= 1e-12 && start[2].abs() <= 1e-12,
                "{start:?}"
            );
            assert!(end[0].abs() <= 1e-12 && end[1].abs() <= 1e-12, "{end:?}");
            assert!((end[2] - 5.).abs() <= 1e-12, "{end:?}");
        }
    }

    #[test]
    fn parallel_offset_plane_yields_exact_hyperbola() {
        // Frustum r 3 -> 1 over z 0..5 (slope -0.4, apex z = 7.5), plane
        // y = 0.5 parallel to the axis: the hyperbola 0.16 x^2 - X^2 = 0.25
        // in apex coordinates (x axial from the apex, X the world x), with
        // a = 1.25, b = 0.5 and asymptote half-angle atan(0.4) = alpha.
        let plane = plane_patch([-4., 0.5, -1.], [8., 0., 0.], [0., 0., 7.]);
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // The height rings clip the one nappe arm into the two arcs above
        // and below the symmetry plane.
        assert_eq!(report.components.len(), 2, "{report:?}");
        let mut signs: Vec<f64> = Vec::new();
        for component in &report.components {
            let PlaneConeComponent::Hyperbola {
                curve,
                center,
                semi_transverse,
                semi_conjugate,
                transverse,
                conjugate,
                normal,
                plane_uv,
                cone_uv,
                max_sample_residual,
            } = component
            else {
                panic!("expected hyperbola components: {report:?}")
            };
            assert!(
                (*semi_transverse - 1.25).abs() <= 1e-12,
                "{semi_transverse}"
            );
            assert!((*semi_conjugate - 0.5).abs() <= 1e-12, "{semi_conjugate}");
            // Asymptote slope b / a is exactly the cone slope magnitude.
            assert!((semi_conjugate / semi_transverse - 0.4).abs() <= 1e-12);
            assert!(
                sub(*center, [0., 0.5, 7.5])
                    .iter()
                    .all(|x| x.abs() <= 1e-12),
                "{center:?}"
            );
            assert!(
                sub(*transverse, [0., 0., -1.])
                    .iter()
                    .all(|x| x.abs() <= 1e-12)
            );
            assert!((conjugate[0].abs() - 1.).abs() <= 1e-12, "{conjugate:?}");
            assert!(conjugate[1].abs() <= 1e-12 && conjugate[2].abs() <= 1e-12);
            assert!((normal[1].abs() - 1.).abs() <= 1e-12, "{normal:?}");
            assert!(cone_uv.is_none());
            assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
            // Arc endpoints sit exactly on the ring planes: |X| = sqrt(8.75)
            // at z = 0 and sqrt(0.75) at z = 5.
            let start = point_of(&curve.evaluate(0.).unwrap().point);
            let end = point_of(&curve.evaluate(1.).unwrap().point);
            for p in [start, end] {
                assert!((p[1] - 0.5).abs() <= 1e-12, "{p:?}");
                let on_bottom =
                    p[2].abs() <= 1e-12 && (p[0].abs() - 8.75_f64.sqrt()).abs() <= 1e-12;
                let on_top =
                    (p[2] - 5.).abs() <= 1e-12 && (p[0].abs() - 0.75_f64.sqrt()).abs() <= 1e-12;
                assert!(on_bottom || on_top, "{p:?}");
            }
            // One arc carries the X > 0 sweep, the other X < 0.
            assert_eq!(start[0].signum(), end[0].signum(), "{start:?} {end:?}");
            signs.push(start[0].signum());
            // The plane UV lift evaluates through the patch onto both
            // implicit equations.
            let empty: Vec<CylinderPatchCurve> = Vec::new();
            let worst = lift_worst(
                &plane,
                &cone,
                plane_uv,
                &empty,
                ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
                ([0., 1., 0.], [-4., 0.5, -1.]),
            );
            assert!(worst <= 1e-9, "{worst}");
        }
        signs.sort_by(f64::total_cmp);
        assert!(signs[0] < 0. && signs[1] > 0., "{signs:?}");
    }

    #[test]
    fn oblique_ellipse_matches_the_independent_vertex_oracle() {
        // Frustum r 3 -> 1 over z 0..5; plane normal (0, 0.6, 0.8) through
        // (0, 0, 2.5) (offset 2.0): |axis.n| = 0.8 > sin(alpha) = 0.4/sqrt(1.16)
        // — an ellipse. Independent oracle: the two vertices are the
        // intersections of the plane with the cone generators in the
        // symmetry plane x = 0, y = +-(3 - 0.4 z): (0, 20/7, 5/14) and
        // (0, -20/13, 95/26), so the center is their midpoint and the
        // semi-major is half their distance, 250/91 exactly.
        let v1 = [0., 20. / 7., 5. / 14.];
        let v2 = [0., -20. / 13., 95. / 26.];
        let mid = [0., (v1[1] + v2[1]) / 2., (v1[2] + v2[2]) / 2.];
        // Patch exactly through the oracle center: u along world x, v along
        // the in-plane axis projection (0, -0.8, 0.6), half-extent 4.
        let e1p = [0., -0.8, 0.6];
        let origin = [mid[0] - 4., mid[1] - 4. * e1p[1], mid[2] - 4. * e1p[2]];
        let plane = plane_patch(origin, [8., 0., 0.], e1p.map(|x| x * 8.));
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneConeComponent::Ellipse {
                curve,
                center,
                semi_major,
                semi_minor,
                major,
                minor,
                normal,
                full,
                plane_uv,
                cone_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one ellipse: {report:?}")
        };
        assert!(*full);
        let v1 = [0., 20. / 7., 5. / 14.];
        let v2 = [0., -20. / 13., 95. / 26.];
        let mid = [0., (v1[1] + v2[1]) / 2., (v1[2] + v2[2]) / 2.];
        assert!(
            sub(*center, mid).iter().all(|x| x.abs() <= 1e-12),
            "{center:?} vs {mid:?}"
        );
        assert!((*semi_major - 250. / 91.).abs() <= 1e-12, "{semi_major}");
        assert!((*semi_minor - 2.096570).abs() <= 1e-5, "{semi_minor}");
        assert!(
            sub(*major, [0., -0.8, 0.6])
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{major:?}"
        );
        assert!(minor[0].abs() >= 1. - 1e-12, "{minor:?}");
        assert!(minor[1].abs() <= 1e-12 && minor[2].abs() <= 1e-12);
        assert!(normal[0].abs() <= 1e-12, "{normal:?}");
        assert!((normal[1].abs() - 0.6).abs() <= 1e-12 && (normal[2].abs() - 0.8).abs() <= 1e-12);
        assert!(cone_uv.is_none());
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // The curve's phi = 0 / pi endpoints are the oracle vertices.
        let p0 = point_of(&curve.evaluate(0.).unwrap().point);
        let p2 = point_of(&curve.evaluate(2.).unwrap().point);
        assert!(
            sub(p0, v2).iter().all(|x| x.abs() <= 1e-12),
            "{p0:?} vs {v2:?}"
        );
        assert!(
            sub(p2, v1).iter().all(|x| x.abs() <= 1e-12),
            "{p2:?} vs {v1:?}"
        );
        let empty: Vec<CylinderPatchCurve> = Vec::new();
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            &empty,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            ([0., 0.6, 0.8], origin),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn ruling_parallel_plane_yields_exact_parabola() {
        // Frustum r 3 -> 1 over z 0..5: sin(alpha) = 2/sqrt(29). The plane
        // built exactly parallel to a ruling (normal (0, cos, sin) with
        // sin = 2/sqrt(29)) through (0, 0, 2) cuts the exact parabola with
        // focal length 1.1 sin(alpha), all weights 1, clipped by the bottom
        // ring plane.
        let sigma = 0.4 / 1.16_f64.sqrt();
        let s = (1. - sigma * sigma).sqrt();
        let normal = [0., s, sigma];
        let offset = 2. * sigma;
        // Patch exactly through (0, 0, 2) (the plane offset 2 sigma holds by
        // construction): u along world x, v along the in-plane axis
        // projection, half-extents 6 and 7 covering the whole clipped arc.
        let e1 = [0., -sigma, s];
        let origin = [-6., 7. * sigma, 2. - 7. * s];
        let plane = plane_patch(origin, [12., 0., 0.], e1.map(|x| x * 14.));
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneConeComponent::Parabola {
                curve,
                vertex,
                direction,
                focal_length,
                normal: n,
                plane_uv,
                cone_uv,
                max_sample_residual,
            },
        ] = &report.components[..]
        else {
            panic!("expected one parabola: {report:?}")
        };
        assert_eq!(curve.degree, 2);
        assert_eq!(curve.knots, vec![0., 0., 0., 1., 1., 1.]);
        assert!(
            curve.weights.iter().all(|w| *w == 1.),
            "{:?}",
            curve.weights
        );
        // Exact focal-length relation: |L| = 0.8 |d0|, |d0| = 5.5 sigma.
        assert!(
            (*focal_length - 1.1 * sigma).abs() <= 1e-12,
            "{focal_length}"
        );
        assert!(
            sub(*vertex, [0., -1.0998, 4.7504])
                .iter()
                .all(|x| x.abs() <= 1e-3),
            "{vertex:?}"
        );
        assert!(
            sub(*direction, [0., sigma, -s])
                .iter()
                .all(|x| x.abs() <= 1e-12),
            "{direction:?}"
        );
        assert!(n[0].abs() <= 1e-12, "{n:?}");
        assert!((n[1].abs() - s).abs() <= 1e-12 && (n[2].abs() - sigma).abs() <= 1e-12);
        assert!(cone_uv.is_none());
        assert!(*max_sample_residual <= 1e-12, "{max_sample_residual}");
        // Both clip endpoints sit exactly on the bottom ring plane z = 0
        // with the ring radius 3.
        for t in [0., 1.] {
            let p = point_of(&curve.evaluate(t).unwrap().point);
            assert!(p[2].abs() <= 1e-12, "{p:?}");
            assert!((p[0].hypot(p[1]) - 3.).abs() <= 1e-9, "{p:?}");
        }
        // The plane offset is exactly n.(0,0,2).
        let d0 = offset - dot(normal, [0., 0., 7.5]);
        assert!((d0 + 5.5 * sigma).abs() <= 1e-12, "{d0}");
        let empty: Vec<CylinderPatchCurve> = Vec::new();
        let worst = lift_worst(
            &plane,
            &cone,
            plane_uv,
            &empty,
            ([0., 0., 0.], [0., 0., 1.], 3., -0.4),
            (normal, origin),
        );
        assert!(worst <= 1e-9, "{worst}");
    }

    #[test]
    fn parabola_threshold_band_and_apex_contact_stay_unresolved() {
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        let sigma = 0.4 / 1.16_f64.sqrt();
        let s = (1. - sigma * sigma).sqrt();
        // Recognition-scale tilt off the ruling-parallel angle: the
        // degenerate threshold is never a guessed conic. The patch plane
        // carries the tilted normal through (0, 0, 2).
        let delta = 5e-10_f64;
        let (sd, cd) = delta.sin_cos();
        let e1t = [0., -(sigma * cd + s * sd), s * cd - sigma * sd];
        let origin = [-6., -7. * e1t[1], 2. - 7. * e1t[2]];
        let plane = plane_patch(origin, [12., 0., 0.], e1t.map(|x| x * 14.));
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.components.is_empty(), "{report:?}");
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::TangencyOrMultipleRoot
        );
        // Oblique planes through the apex (ellipse angle and hyperbola
        // angle): the apex is a multiple-root contact, never guessed. The
        // patch spans the plane through the apex along e1 = (1,0,0) (both
        // normals have a zero x component) and e2 = normal x e1.
        let apex = [0., 0., 7.5];
        for normal in [[0., 0.6, 0.8], [0., 0.96_f64.sqrt(), 0.2]] {
            let e1 = [1., 0., 0.];
            let e2 = cross(normal, e1);
            let e2 = e2.map(|x| x / hypot3(e2));
            let origin = sub(sub(apex, e1.map(|x| x * 6.)), e2.map(|x| x * 8.));
            let plane = plane_patch(origin, e1.map(|x| x * 12.), e2.map(|x| x * 16.));
            let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{normal:?} {report:?}");
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::TangencyOrMultipleRoot
            );
        }
    }

    #[test]
    fn misses_resolve_empty() {
        let cone = crate::analytic::frustum(3., 1., 5.).unwrap();
        // Perpendicular plane beyond the top ring.
        let plane = plane_patch([-4., -4., 7.], [8., 0., 0.], [0., 8., 0.]);
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        // Oblique plane whose ellipse lives entirely below the bottom ring.
        let plane = plane_patch([-30., 20., -20.], [60., 0., 0.], [0., -48., 36.]);
        // normal of that patch is (0, 0.6, 0.8)-ish; ensure a clear miss:
        // n.p = 0.6*20 + 0.8*(-20) = -4 at the origin — far below the cone.
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(
            report.components.is_empty() && report.unresolved.is_empty(),
            "{report:?}"
        );
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
    }

    #[test]
    fn rotated_translated_placement_keeps_the_exact_ellipse() {
        // The ellipse configuration rotated 0.5 rad about X and translated:
        // same section against the transformed oracle.
        let angle = 0.5_f64;
        let offset = [10., -7., 3.];
        let (sa, ca) = angle.sin_cos();
        let rot = |p: [f64; 3]| {
            [
                p[0] + offset[0],
                ca * p[1] - sa * p[2] + offset[1],
                sa * p[1] + ca * p[2] + offset[2],
            ]
        };
        // Exact pre-rotation patch through the oracle center, then rotated.
        let mid0 = [0., (20. / 7. - 20. / 13.) / 2., (5. / 14. + 95. / 26.) / 2.];
        let e1p = [0., -0.8, 0.6];
        let origin0 = [mid0[0] - 4., mid0[1] - 4. * e1p[1], mid0[2] - 4. * e1p[2]];
        let plane = plane_patch(
            rot(origin0),
            sub(rot([origin0[0] + 8., origin0[1], origin0[2]]), rot(origin0)),
            sub(
                rot([
                    origin0[0],
                    origin0[1] + 8. * e1p[1],
                    origin0[2] + 8. * e1p[2],
                ]),
                rot(origin0),
            ),
        );
        let cone = rotated_translated(
            &crate::analytic::frustum(3., 1., 5.).unwrap(),
            angle,
            offset,
        );
        let report = intersect_plane_cone(&plane, &cone, Options::default()).unwrap();
        assert!(report.unresolved.is_empty(), "{report:?}");
        let [
            PlaneConeComponent::Ellipse {
                center,
                semi_major,
                major,
                full,
                cone_uv,
                max_sample_residual,
                ..
            },
        ] = &report.components[..]
        else {
            panic!("expected one placed ellipse: {report:?}")
        };
        assert!(*full);
        assert!(cone_uv.is_none());
        let mid = rot([0., (20. / 7. - 20. / 13.) / 2., (5. / 14. + 95. / 26.) / 2.]);
        assert!(
            sub(*center, mid).iter().all(|x| x.abs() <= 1e-9),
            "{center:?} vs {mid:?}"
        );
        assert!((*semi_major - 250. / 91.).abs() <= 1e-9, "{semi_major}");
        let major_expected = sub(rot([0., -0.8, 0.6]), rot([0., 0., 0.]));
        assert!(
            sub(*major, major_expected).iter().all(|x| x.abs() <= 1e-9),
            "{major:?} vs {major_expected:?}"
        );
        assert!(*max_sample_residual <= 1e-9, "{max_sample_residual}");
    }

    #[test]
    fn non_canonical_operands_are_explicit_refusals() {
        let plane = plane_patch([-4., -4., 2.5], [8., 0., 0.], [0., 8., 0.]);
        // An equal-radius frustum is a cylinder: refused here (the
        // plane/cylinder cell owns it).
        let cylinder = crate::analytic::cylinder(2., 5.).unwrap();
        let sphere = crate::analytic::sphere(2.).unwrap();
        let tube = crate::analytic::tube(2., 1., 5.).unwrap();
        let frustum = crate::analytic::frustum(3., 1., 5.).unwrap();
        for second in [&cylinder, &sphere, &tube] {
            let report = intersect_plane_cone(&plane, second, Options::default()).unwrap();
            assert!(report.components.is_empty(), "{report:?}");
            assert_eq!(report.coverage, Coverage::Incomplete);
            assert_eq!(
                report.unresolved[0].reason,
                UnresolvedReason::UnsupportedSurface
            );
            assert_eq!(
                report.unresolved[0].parameter_box,
                vec![0., 1., 0., 1., 0., 1., 0., 1.]
            );
            assert!(!report.permits_topology_change());
        }
        // A solid as the plane operand: fixed order, explicit refusal.
        let report = intersect_plane_cone(&frustum, &frustum, Options::default()).unwrap();
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        // Skewed non-rectangular patch: not the canonical planar patch.
        let skewed = plane_patch([-4., -4., 2.5], [8., 0., 0.], [3., 8., 0.]);
        let report = intersect_plane_cone(&skewed, &frustum, Options::default()).unwrap();
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
    }
}
