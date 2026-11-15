//! Analytic sphere/cylinder intersection for canonical solids in the axial
//! configuration (sphere center certified on the cylinder axis).
//!
//! The sphere operand must be the exact canonical stereographic sphere of
//! `analytic::sphere` (recognized by `sphere_sphere::recognize`); the cylinder
//! operand must be the exact canonical six-face cylinder of
//! `analytic::cylinder` (four rational circular side patches, two bilinear
//! caps with inscribed-circle trims), either optionally carried through a
//! rigid affine placement. Anything else is an explicit `UnsupportedSurface`
//! region — this cell never falls back to numerical surface/surface
//! subdivision. Axiality is certified, never forced: a perpendicular offset
//! at pure rounding scale snaps to the axis, an offset inside the
//! recognition-scale band reports a `NearCoincidence` region, and a clearly
//! off-axis pair is `UnsupportedSurface` (the general sphere/cylinder pair is
//! a quartic, out of scope). Classification uses outward binary64 bands
//! widened by the observed recognition deviation. Resolved contacts are exact
//! rational circles (four 90-degree arcs, weights cos(pi/4)) with UV lifts on
//! both surfaces: iso-v lines across all four cylinder side patches, an exact
//! UV circle on a cap face, and per-patch UV circles/lines on the sphere.
//! Every tangency — cap-plane touch, rim (cap circle radius == cylinder
//! radius), the r == R coincident band — stays an explicit unresolved region;
//! tangent contacts are never reported as point or guessed-circle components,
//! matching the house tangency discipline. Nothing here authorizes a topology
//! change.
use super::sphere_sphere::RECOGNITION;
use super::*;
use crate::Model;

#[derive(Clone, Debug)]
pub struct CanonicalCylinder {
    /// Midpoint of the axis segment.
    pub center: [f64; 3],
    /// Unit axis, bottom cap toward top cap.
    pub axis: [f64; 3],
    pub radius: f64,
    pub half_height: f64,
    /// Observed structural deviation bound; feeds the outward classification.
    pub error: f64,
    /// In-plane orthonormal ring frame: x = quadrant-0 direction, y = axis x x.
    pub frame: [[f64; 3]; 2],
    /// Side face indices ordered by quadrant.
    pub sides: [usize; 4],
    /// Cap face indices: [bottom, top].
    #[allow(dead_code)]
    pub caps: [usize; 2],
}

/// Smallest and largest admitted cylinder height and radius.
const SCALE_RANGE: std::ops::RangeInclusive<f64> = 1e-5..=1e6;
/// Absolute floor added to the relative control-point recognition band.
const ABSOLUTE_SLACK: f64 = 1e-12;

fn point_of(jet_point: &[f64]) -> [f64; 3] {
    [jet_point[0], jet_point[1], jet_point[2]]
}

/// The cylinder axis, radius and ring frame recovered from the side patches,
/// plus the running structural deviation bound.
struct CylinderFit {
    bottom: [f64; 3],
    axis: [f64; 3],
    height: f64,
    center: [f64; 3],
    radius: f64,
    x_dir: [f64; 3],
    y_dir: [f64; 3],
    error: f64,
}

impl CylinderFit {
    /// Recovers the ring centers, axis, radius and in-plane frame.
    fn measure(model: &Model, sides: &[usize]) -> Result<Option<Self>> {
        let [bottom, top] = recognize::ring_centers(model, sides)?;
        let axis_vec = sub(top, bottom);
        let height = axis_vec[0].hypot(axis_vec[1]).hypot(axis_vec[2]);
        if !height.is_finite() || !SCALE_RANGE.contains(&height) {
            return Ok(None);
        }
        let axis = axis_vec.map(|x| x / height);
        let center = std::array::from_fn(|k| (bottom[k] + top[k]) / 2.);
        // Radius from the four arc midpoints (u = v = 1/2 sits on the
        // 45-degree point of the quarter arc at half height).
        let mut radius = 0.;
        let mut mid_radii = Vec::with_capacity(4);
        for &index in sides {
            let point = point_of(&model.faces[index].surface.evaluate(0.5, 0.5)?.point);
            let perp = recognize::radial(point, center, axis);
            let r = perp[0].hypot(perp[1]).hypot(perp[2]);
            mid_radii.push(r);
            radius += r / 4.;
        }
        if !radius.is_finite() || !SCALE_RANGE.contains(&radius) {
            return Ok(None);
        }
        let mut error: f64 = 0.;
        for r in &mid_radii {
            let deviation = (r - radius).abs();
            if deviation > RECOGNITION * radius {
                return Ok(None);
            }
            error = error.max(deviation);
        }
        // In-plane frame: x from the first side patch's bottom start direction.
        let start = point_of(&model.faces[sides[0]].surface.evaluate(0., 0.)?.point);
        let x_perp = recognize::radial(start, bottom, axis);
        let x_length = x_perp[0].hypot(x_perp[1]).hypot(x_perp[2]);
        if !x_length.is_finite() || x_length <= 0. {
            return Ok(None);
        }
        let x_dir = x_perp.map(|x| x / x_length);
        error = error.max((x_length - radius).abs());
        let y_dir = cross(axis, x_dir);
        Ok(Some(Self {
            bottom,
            axis,
            height,
            center,
            radius,
            x_dir,
            y_dir,
            error,
        }))
    }

    /// Folds one control point's certified deviation into the error bound.
    fn absorb(&mut self, actual: &[f64], expected: [f64; 3]) -> bool {
        let bound = RECOGNITION * self.radius + ABSOLUTE_SLACK;
        match recognize::certified_deviation(actual, expected, bound) {
            Some(deviation) => {
                self.error = self.error.max(deviation);
                true
            }
            None => false,
        }
    }

    /// Per-patch quadrant tiling and exact control-point certification;
    /// returns the side faces ordered by quadrant.
    fn certify_sides(&mut self, model: &Model, sides: &[usize]) -> Result<Option<[usize; 4]>> {
        let mut seen = [false; 4];
        let mut ordered = [0usize; 4];
        for &index in sides {
            let surface = &model.faces[index].surface;
            let point = point_of(&surface.evaluate(0., 0.)?.point);
            let perp = recognize::radial(point, self.bottom, self.axis);
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
                for (j, actual) in surface.control_points[k].iter().take(2).enumerate() {
                    let expected: [f64; 3] = std::array::from_fn(|a| {
                        self.bottom[a]
                            + self.radius
                                * (expected_xy[0] * self.x_dir[a] + expected_xy[1] * self.y_dir[a])
                            + self.height * j as f64 * self.axis[a]
                    });
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
    /// frame, and the four inscribed quarter-arc trims each exactly once.
    /// Returns the cap faces as [bottom, top].
    fn certify_caps(&mut self, model: &Model, caps: &[usize]) -> Result<Option<[usize; 2]>> {
        let mut cap_assigned = [false; 2];
        let mut cap_ids = [0usize; 2];
        for &index in caps {
            let surface = &model.faces[index].surface;
            let corner = point_of(&surface.evaluate(0., 0.)?.point);
            let axial = dot(sub(corner, self.bottom), self.axis);
            let Some((slot, plane_deviation)) =
                recognize::ring_slot(axial, self.height, RECOGNITION)
            else {
                return Ok(None);
            };
            self.error = self.error.max(plane_deviation);
            if cap_assigned[slot] {
                return Ok(None);
            }
            cap_assigned[slot] = true;
            cap_ids[slot] = index;
            let lift = if slot == 1 { self.height } else { 0. };
            for (i, row) in surface.control_points.iter().take(2).enumerate() {
                for (j, actual) in row.iter().take(2).enumerate() {
                    let expected: [f64; 3] = std::array::from_fn(|a| {
                        self.bottom[a]
                            + self.radius
                                * ((2. * i as f64 - 1.) * self.x_dir[a]
                                    + (2. * j as f64 - 1.) * self.y_dir[a])
                            + lift * self.axis[a]
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
        if cap_assigned.into_iter().any(|hit| !hit) {
            return Ok(None);
        }
        Ok(Some(cap_ids))
    }

    /// Every vertex on one of the two rings.
    fn certify_vertices(&mut self, model: &Model) -> bool {
        for vertex in &model.vertices {
            let d = sub(vertex.point, self.center);
            let axial = dot(d, self.axis);
            let perp = sub(d, self.axis.map(|x| x * axial));
            let dev_r = (perp[0].hypot(perp[1]).hypot(perp[2]) - self.radius).abs();
            let dev_a = (axial.abs() - self.height / 2.).abs();
            if dev_r > RECOGNITION * self.radius || dev_a > RECOGNITION * self.height {
                return false;
            }
            self.error = self.error.max(dev_r).max(dev_a);
        }
        true
    }
}

/// Recognizes a canonical cylinder solid as built by `analytic::cylinder`,
/// certifying the side/cap surface structure, exact weights and trim pcurves,
/// every control point against the exact construction in the recovered ring
/// frame, a globally consistent quadrant tiling, and both ring vertex sets.
/// Rigid affine placement is admitted; anything else returns `None`.
pub fn recognize_cylinder(model: &Model) -> Result<Option<CanonicalCylinder>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 6
        || model.vertices.len() != 8
    {
        return Ok(None);
    }
    let Some((sides, caps)) = recognize::classify_analytic_faces(model) else {
        return Ok(None);
    };
    if sides.len() != 4 || caps.len() != 2 {
        return Ok(None);
    }
    // Side trims: the unit-square boundary, each edge exactly once.
    if !sides
        .iter()
        .all(|&index| recognize::unit_square_boundary(model, index))
    {
        return Ok(None);
    }
    let Some(mut fit) = CylinderFit::measure(model, &sides)? else {
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
    Ok(Some(CanonicalCylinder {
        center: fit.center,
        axis: fit.axis,
        radius: fit.radius,
        half_height: fit.height / 2.,
        error: fit.error,
        frame: [fit.x_dir, fit.y_dir],
        sides: ordered,
        caps: cap_ids,
    }))
}

