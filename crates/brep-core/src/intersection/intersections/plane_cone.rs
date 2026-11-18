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
use super::sphere_sphere::RECOGNITION;
use super::*;
use crate::Model;

#[derive(Clone, Debug)]
pub struct CanonicalCone {
    /// Bottom ring center (the apex itself when the bottom radius is zero).
    pub bottom: [f64; 3],
    /// Unit axis, bottom ring toward top ring.
    pub axis: [f64; 3],
    pub r_bottom: f64,
    pub r_top: f64,
    pub height: f64,
    /// Radius slope (r_top - r_bottom) / height, certified nonzero.
    pub slope: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub sides: [usize; 4],
    /// Cap face indices [bottom, top]; None for an apex ring.
    #[allow(dead_code)]
    pub caps: [Option<usize>; 2],
}

/// Smallest and largest admitted frustum height and ring radius.
const SCALE_RANGE: std::ops::RangeInclusive<f64> = 1e-5..=1e6;
/// Absolute floor added to the relative recognition band.
const ABSOLUTE_SLACK: f64 = 1e-12;

fn point3_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

fn hypot3(v: [f64; 3]) -> f64 {
    v[0].hypot(v[1]).hypot(v[2])
}

/// The frustum axis, ring radii and ring frame recovered from the side
/// patches, plus the running structural deviation bound.
struct ConeFit {
    bottom: [f64; 3],
    top: [f64; 3],
    axis: [f64; 3],
    height: f64,
    r_bottom: f64,
    r_top: f64,
    r_scale: f64,
    /// Center and side-patch v of the nonzero ring the frame is read from.
    ring_center: [f64; 3],
    ring_v: f64,
    x_dir: [f64; 3],
    y_dir: [f64; 3],
    error: f64,
}

/// Ring radii averaged from the four arc midpoints, before snapping.
struct RingRadii {
    r_bottom: f64,
    r_top: f64,
    r_scale: f64,
    error: f64,
}

impl RingRadii {
    /// Ring radii from the four arc midpoints (u = v = 1/2 sits on the
    /// 45-degree point of the quarter arc at that height's radius).
    fn measure(
        model: &Model,
        sides: &[usize],
        [bottom, top]: [[f64; 3]; 2],
        axis: [f64; 3],
    ) -> Result<Option<Self>> {
        let mut r_bottom = 0.;
        let mut r_top = 0.;
        let mut mid_radii = Vec::with_capacity(8);
        for &index in sides {
            let pb = point3_of(&model.faces[index].surface.evaluate(0.5, 0.)?.point);
            let pt = point3_of(&model.faces[index].surface.evaluate(0.5, 1.)?.point);
            let rb = hypot3(recognize::radial(pb, bottom, axis));
            let rt = hypot3(recognize::radial(pt, top, axis));
            mid_radii.push((rb, false));
            mid_radii.push((rt, true));
            r_bottom += rb / 4.;
            r_top += rt / 4.;
        }
        let r_scale = r_bottom.max(r_top);
        if !r_scale.is_finite() || !SCALE_RANGE.contains(&r_scale) {
            return Ok(None);
        }
        let mut error: f64 = 0.;
        for (r, is_top) in &mid_radii {
            let ring = if *is_top { r_top } else { r_bottom };
            let deviation = (r - ring).abs();
            if deviation > RECOGNITION * r_scale + ABSOLUTE_SLACK {
                return Ok(None);
            }
            error = error.max(deviation);
        }
        Ok(Some(Self {
            r_bottom,
            r_top,
            r_scale,
            error,
        }))
    }

    /// An apex ring is exactly zero in the canonical construction; snaps a
    /// rounding-scale radius to zero and records the deviation.
    fn snap_apex(&mut self) {
        let band = RECOGNITION * self.r_scale + ABSOLUTE_SLACK;
        for radius in [&mut self.r_bottom, &mut self.r_top] {
            if *radius <= band {
                self.error = self.error.max(*radius);
                *radius = 0.;
            }
        }
    }
}

impl ConeFit {
    /// Recovers the ring centers, axis, snapped ring radii and in-plane
    /// frame, refusing an equal-radius (cylindrical) frustum.
    fn measure(model: &Model, sides: &[usize]) -> Result<Option<Self>> {
        // On an apex ring every corner evaluates to the one shared apex vertex.
        let [bottom, top] = recognize::ring_centers(model, sides)?;
        let axis_vec = sub(top, bottom);
        let height = hypot3(axis_vec);
        if !height.is_finite() || !SCALE_RANGE.contains(&height) {
            return Ok(None);
        }
        let axis = axis_vec.map(|x| x / height);
        let Some(mut radii) = RingRadii::measure(model, sides, [bottom, top], axis)? else {
            return Ok(None);
        };
        radii.snap_apex();
        let RingRadii {
            r_bottom,
            r_top,
            r_scale,
            error,
        } = radii;
        // Certified-nonzero taper: an equal-radius frustum is a cylinder and
        // is refused here (the plane/cylinder cell owns it).
        if (r_top - r_bottom).abs() <= RECOGNITION * r_scale + ABSOLUTE_SLACK {
            return Ok(None);
        }
        // In-plane frame from a nonzero ring: x is the quadrant-0 direction.
        let (ring_center, ring_v) = if r_bottom > 0. {
            (bottom, 0.)
        } else {
            (top, 1.)
        };
        let start = point3_of(&model.faces[sides[0]].surface.evaluate(0., ring_v)?.point);
        let x_perp = recognize::radial(start, ring_center, axis);
        let x_length = hypot3(x_perp);
        if !x_length.is_finite() || x_length <= 0. {
            return Ok(None);
        }
        let x_dir = x_perp.map(|x| x / x_length);
        let y_dir = cross(axis, x_dir);
        Ok(Some(Self {
            bottom,
            top,
            axis,
            height,
            r_bottom,
            r_top,
            r_scale,
            ring_center,
            ring_v,
            x_dir,
            y_dir,
            error,
        }))
    }

    /// Folds one control point's certified deviation into the error bound.
    fn absorb(&mut self, actual: &[f64], expected: [f64; 3]) -> bool {
        let bound = RECOGNITION * self.r_scale + ABSOLUTE_SLACK;
        match recognize::certified_deviation(actual, expected, bound) {
            Some(deviation) => {
                self.error = self.error.max(deviation);
                true
            }
            None => false,
        }
    }

    /// Per-patch quadrant tiling and exact control-point certification
    /// against the linearly interpolated ring radii (apex row: the apex);
    /// returns the side faces ordered by quadrant.
    fn certify_sides(&mut self, model: &Model, sides: &[usize]) -> Result<Option<[usize; 4]>> {
        let mut seen = [false; 4];
        let mut ordered = [0usize; 4];
        let rows = [(self.r_bottom, 0.), (self.r_top, self.height)];
        for &index in sides {
            let surface = &model.faces[index].surface;
            let point = point3_of(&surface.evaluate(0., self.ring_v)?.point);
            let perp = recognize::radial(point, self.ring_center, self.axis);
            let angle = dot(perp, self.y_dir).atan2(dot(perp, self.x_dir));
            let Some(quadrant) = recognize::snap_quadrant(angle, RECOGNITION) else {
                return Ok(None);
            };
            if seen[quadrant] {
                return Ok(None);
            }
            seen[quadrant] = true;
            ordered[quadrant] = index;
            let pattern = recognize::quarter_arc_pattern(quadrant);
            for (k, expected_xy) in pattern.iter().enumerate() {
                for (j, (ring_r, zz)) in rows.iter().enumerate() {
                    let expected: [f64; 3] = std::array::from_fn(|a| {
                        self.bottom[a]
                            + ring_r
                                * (expected_xy[0] * self.x_dir[a] + expected_xy[1] * self.y_dir[a])
                            + zz * self.axis[a]
                    });
                    let Some(actual) = surface.control_points.get(k).and_then(|row| row.get(j))
                    else {
                        return Ok(None);
                    };
                    if !self.absorb(actual, expected) {
                        return Ok(None);
                    }
                }
            }
        }
        if seen.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
        Ok(Some(ordered))
    }

    /// Caps: axial slot from any corner, exact bilinear corners in the ring
    /// frame at that ring's radius, and the four inscribed quarter-arc trims
    /// each exactly once. An apex ring has no cap. Returns the cap faces as
    /// [bottom, top].
    fn certify_caps(
        &mut self,
        model: &Model,
        caps: &[usize],
    ) -> Result<Option<[Option<usize>; 2]>> {
        let mut cap_ids: [Option<usize>; 2] = [None, None];
        for &index in caps {
            let surface = &model.faces[index].surface;
            let corner = point3_of(&surface.evaluate(0., 0.)?.point);
            let axial = dot(sub(corner, self.bottom), self.axis);
            let Some((slot, plane_deviation)) =
                recognize::ring_slot(axial, self.height, RECOGNITION)
            else {
                return Ok(None);
            };
            self.error = self.error.max(plane_deviation);
            let ring_r = if slot == 0 { self.r_bottom } else { self.r_top };
            if ring_r == 0. || cap_ids[slot].is_some() {
                return Ok(None);
            }
            cap_ids[slot] = Some(index);
            let ring_c = if slot == 0 { self.bottom } else { self.top };
            for (i, row) in surface.control_points.iter().take(2).enumerate() {
                for (j, actual) in row.iter().take(2).enumerate() {
                    let expected: [f64; 3] = std::array::from_fn(|a| {
                        ring_c[a]
                            + ring_r
                                * ((2. * i as f64 - 1.) * self.x_dir[a]
                                    + (2. * j as f64 - 1.) * self.y_dir[a])
                    });
                    if !self.absorb(actual, expected) {
                        return Ok(None);
                    }
                }
            }
            if !recognize::cap_quarter_trims(model, index) {
                return Ok(None);
            }
        }
        if cap_ids[0].is_some() != (self.r_bottom > 0.) || cap_ids[1].is_some() != (self.r_top > 0.)
        {
            return Ok(None);
        }
        Ok(Some(cap_ids))
    }

    /// Every vertex on one of the two rings (the apex ring: the pole).
    fn certify_vertices(&mut self, model: &Model) -> bool {
        for vertex in &model.vertices {
            let d = sub(vertex.point, self.bottom);
            let axial = dot(d, self.axis);
            let perp = sub(d, self.axis.map(|x| x * axial));
            let Some((slot, _)) = recognize::ring_slot(axial, self.height, RECOGNITION) else {
                return false;
            };
            let ring_r = if slot == 0 { self.r_bottom } else { self.r_top };
            let dev_r = (hypot3(perp) - ring_r).abs();
            if dev_r > RECOGNITION * self.r_scale + ABSOLUTE_SLACK {
                return false;
            }
            self.error = self.error.max(dev_r);
        }
        true
    }
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
pub fn recognize_cone(model: &Model) -> Result<Option<CanonicalCone>> {
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
    let Some((sides, caps)) = recognize::classify_analytic_faces(model) else {
        return Ok(None);
    };
    if sides.len() != 4 || caps.is_empty() || caps.len() > 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    if !sides
        .iter()
        .all(|&index| recognize::unit_square_boundary(model, index))
    {
        return Ok(None);
    }
    let Some(mut fit) = ConeFit::measure(model, &sides)? else {
        return Ok(None);
    };
    let Some(ordered) = fit.certify_sides(model, &sides)? else {
        return Ok(None);
    };
    let Some(cap_ids) = fit.certify_caps(model, &caps)? else {
        return Ok(None);
    };
    if !fit.certify_vertices(model) {
        return Ok(None);
    }
    Ok(Some(CanonicalCone {
        bottom: fit.bottom,
        axis: fit.axis,
        r_bottom: fit.r_bottom,
        r_top: fit.r_top,
        height: fit.height,
        slope: (fit.r_top - fit.r_bottom) / fit.height,
        error: fit.error,
        frame: [fit.x_dir, fit.y_dir],
        sides: ordered,
        caps: cap_ids,
    }))
}

