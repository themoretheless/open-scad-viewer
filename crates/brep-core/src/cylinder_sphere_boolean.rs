//! Axial cylinder × sphere: the `Mate` of `sphere_mate` for the canonical
//! cylinder (four rational quarter-arc wall patches and two planar caps,
//! rigid placement admitted) against a sphere centred on the cylinder axis.
//!
//! On the axis the intersection curve is exact: each cap plane cuts the
//! sphere in a circle concentric with the cap, and the wall meets the sphere
//! in horizontal rings at `z = z_c ± sqrt(r² - R²)`, which are iso-curves of
//! the wall patches. A wall ring is split at the four wall seams, where the
//! sphere's own seams coincide when both bodies share the frame; the engine
//! merges those events into shared hit vertices. A sphere off the axis meets
//! the wall in a quartic that has no exact rational form and is refused.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::sphere_sphere::{self, CanonicalSphere};
use crate::intersections::{CanonicalCylinder, recognize_cylinder};
use crate::sphere_mate::{
    self, Circle, Imprint, Mate, PlaneContact, TAU, affine_arc_pcurve, affine_uv_of, dot, norm,
    plane_contact, point3, quarter_arc_parameter, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

struct AxialCylinder<'m> {
    model: &'m Model,
    canon: CanonicalCylinder,
    /// Quadrant of each side face (inverse of `canon.sides`), `usize::MAX`
    /// for caps.
    quadrant_of: Vec<usize>,
    /// Vertical edge at the start (u = 0) of each side quadrant, with its
    /// bottom-to-top parameterization: (edge, axial coordinate at t = 0,
    /// axial coordinate at t = 1), relative to the cylinder centre.
    seams: [(usize, f64, f64); 4],
}

impl<'m> AxialCylinder<'m> {
    fn recognize(model: &'m Model) -> Result<Option<Self>> {
        let Some(canon) = recognize_cylinder(model)? else {
            return Ok(None);
        };
        if model.edges.len() != 12 || model.vertices.len() != 8 {
            return Ok(None);
        }
        let mut quadrant_of = vec![usize::MAX; model.faces.len()];
        for (q, &face) in canon.sides.iter().enumerate() {
            quadrant_of[face] = q;
        }
        let mut seams = [(usize::MAX, 0., 0.); 4];
        for (q, &face) in canon.sides.iter().enumerate() {
            let wire = &model.loops[model.faces[face].outer];
            let Some(coedge) = wire
                .coedges
                .iter()
                .find(|c| sphere_sphere::axis_line(&c.pcurve, [0., 1.], [0., 0.]))
            else {
                return Ok(None);
            };
            let edge = &model.edges[coedge.edge];
            let axial = |v: usize| dot(sub(model.vertices[v].point, canon.center), canon.axis);
            seams[q] = (
                coedge.edge,
                axial(edge.vertices[0]),
                axial(edge.vertices[1]),
            );
        }
        Ok(Some(Self {
            model,
            canon,
            quadrant_of,
            seams,
        }))
    }

    fn local(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        let d = sub(p, self.canon.center);
        let axial = dot(d, self.canon.axis);
        let perp: [f64; 3] = std::array::from_fn(|k| d[k] - axial * self.canon.axis[k]);
        (axial, perp)
    }
}

impl Mate for AxialCylinder<'_> {
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let (axial, perp) = self.local(point);
        let axial_excess = axial.abs() - self.canon.half_height;
        let radial_excess = norm(perp) - self.canon.radius;
        let worst = axial_excess.max(radial_excess);
        if worst < -band {
            Ok(true)
        } else if worst > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Cylinder/sphere Boolean: region classification falls within the error band \
                 of the cylinder boundary",
            ))
        }
    }

    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]> {
        let q = self.quadrant_of[face];
        if q == usize::MAX {
            return Ok(affine_uv_of(&self.model.faces[face].surface, point));
        }
        let (axial, perp) = self.local(point);
        let [x, y] = self.canon.frame;
        let angle = dot(perp, y).atan2(dot(perp, x));
        let delta = (angle - q as f64 * QUARTER + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        if !(-1e-9..=QUARTER + 1e-9).contains(&delta) {
            return Err(unsupported(
                "Cylinder/sphere Boolean: network vertex outside its wall patch",
            ));
        }
        let (sin, cos) = delta.clamp(0., QUARTER).sin_cos();
        let u = quarter_arc_parameter(cos, sin).clamp(0., 1.);
        let v = ((axial + self.canon.half_height) / (2. * self.canon.half_height)).clamp(0., 1.);
        Ok([u, v])
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        if self.quadrant_of[face] == usize::MAX {
            return Ok(affine_arc_pcurve(&self.model.faces[face].surface, arc));
        }
        // A wall ring piece between two seams is the patch's own u-arc at
        // constant v: its pcurve is a straight line, verified by the engine.
        let a = self.uv_of(face, point3(&arc.control_points[0]))?;
        let b = self.uv_of(face, point3(&arc.control_points[2]))?;
        Ok(Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        })
    }

    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>> {
        let band = imprint.band;
        let sphere = imprint.sphere.clone();
        let c = &self.canon;
        let (zs, perp) = self.local(sphere.center);
        let r = sphere.radius;
        let big_r = c.radius;
        let hh = c.half_height;
        let offset = norm(perp);
        // Empty algebra never needs the axial position: clear of the wall
        // radially or of a cap axially is disjoint, and a sphere clear of
        // the wall and both caps from inside is contained.
        if offset > big_r + r + band || zs.abs() > hh + r + band {
            return Ok(Some(SpatialRelation::Disjoint));
        }
        if offset + r < big_r - band && zs.abs() + r < hh - band {
            return Ok(Some(imprint.mate_contains_sphere()));
        }
        if offset > band {
            return Err(unsupported(
                "Cylinder/sphere Boolean: only a sphere centred on the cylinder axis meets the \
                 wall in exact circles; a general position needs a quartic and is refused",
            ));
        }
        // Caps.
        let mut contacts = [(PlaneContact::Inside, 0.); 2];
        for k in 0..2 {
            let sign = if k == 0 { -1. } else { 1. };
            let outward = c.axis.map(|x| x * sign);
            let origin: [f64; 3] = std::array::from_fn(|i| c.center[i] + sign * hh * c.axis[i]);
            contacts[k] = plane_contact(origin, outward, &sphere, band);
        }
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Tangent)
        {
            return Err(unsupported(
                "Cylinder/sphere Boolean: a cap plane lies within the tangency band of the sphere",
            ));
        }
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Outside)
        {
            return Ok(Some(SpatialRelation::Disjoint));
        }
        if (r - big_r).abs() <= band {
            return Err(unsupported(
                "Cylinder/sphere Boolean: the sphere is tangent to the cylinder wall",
            ));
        }
        if contacts
            .iter()
            .all(|(kind, _)| *kind == PlaneContact::Inside)
            && r < big_r
        {
            return Ok(Some(imprint.mate_contains_sphere()));
        }
        for k in 0..2 {
            let (kind, d) = contacts[k];
            if kind != PlaneContact::Crossing {
                continue;
            }
            let face = c.caps[k];
            let rho = (r * r - d * d).sqrt();
            if rho < big_r - band {
                let surface = &self.model.faces[face].surface;
                let p00 = point3(&surface.control_points[0][0]);
                let u = sub(point3(&surface.control_points[1][0]), p00);
                let v = sub(point3(&surface.control_points[0][1]), p00);
                let sign = if k == 0 { -1. } else { 1. };
                let origin: [f64; 3] = std::array::from_fn(|i| c.center[i] + sign * hh * c.axis[i]);
                let mut circle = Circle::new(
                    face,
                    origin,
                    rho,
                    u.map(|x| x / norm(u)),
                    v.map(|x| x / norm(v)),
                );
                circle.push_ring();
                imprint.circles.push(circle);
            } else if rho > big_r + band {
                // Cap entirely inside the sphere: classified whole by the engine.
            } else {
                return Err(unsupported(
                    "Cylinder/sphere Boolean: the sphere passes through a cap rim",
                ));
            }
        }
        // Wall rings.
        if r > big_r + band {
            let h = (r * r - big_r * big_r).sqrt();
            for sign in [-1., 1.] {
                let z = zs + sign * h;
                if z.abs() < hh - band {
                    self.wall_ring(imprint, z)?;
                } else if z.abs() <= hh + band {
                    return Err(unsupported(
                        "Cylinder/sphere Boolean: a wall ring falls on a cap rim",
                    ));
                }
            }
        }
        Ok(None)
    }
}

impl AxialCylinder<'_> {
    /// One horizontal ring at axial coordinate `z` (relative to the centre):
    /// four hits on the wall seams and one quarter interval per side patch.
    fn wall_ring(&self, imprint: &mut Imprint<'_, Self>, z: f64) -> Result<()> {
        let c = &self.canon;
        let [x, y] = c.frame;
        let ring_center: [f64; 3] = std::array::from_fn(|i| c.center[i] + z * c.axis[i]);
        let mut hits = [usize::MAX; 4];
        for q in 0..4 {
            let (edge, z0, z1) = self.seams[q];
            let t = (z - z0) / (z1 - z0);
            if !(1e-9..=1. - 1e-9).contains(&t) {
                return Err(unsupported(
                    "Cylinder/sphere Boolean: wall ring hit outside its seam edge",
                ));
            }
            let (sin, cos) = (q as f64 * QUARTER).sin_cos();
            let point: [f64; 3] =
                std::array::from_fn(|i| ring_center[i] + c.radius * (cos * x[i] + sin * y[i]));
            hits[q] = imprint.hit(point, edge, t);
        }
        for q in 0..4 {
            let mut circle = Circle::new(c.sides[q], ring_center, c.radius, x, y);
            circle.hits.push((q as f64 * QUARTER, hits[q]));
            circle.push_interval(
                q as f64 * QUARTER,
                (q + 1) as f64 * QUARTER,
                hits[q],
                hits[(q + 1) % 4],
            );
            imprint.circles.push(circle);
        }
        Ok(())
    }
}

/// Cylinder/sphere Boolean entry: `Ok(None)` unless one operand is the
/// canonical cylinder and the other the canonical sphere.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (mate_index, cylinder, sphere) = match (
        AxialCylinder::recognize(a)?,
        sphere_sphere::recognize(b)?,
        AxialCylinder::recognize(b)?,
        sphere_sphere::recognize(a)?,
    ) {
        (Some(cylinder), Some(sphere), _, _) => (0usize, cylinder, sphere),
        (_, _, Some(cylinder), Some(sphere)) => (1usize, cylinder, sphere),
        _ => return Ok(None),
    };
    let band = shared_band(&cylinder, &sphere);
    sphere_mate::run(a, b, mate_index, &cylinder, sphere, band, operation).map(Some)
}

fn shared_band(cylinder: &AxialCylinder<'_>, sphere: &CanonicalSphere) -> f64 {
    let c = &cylinder.canon;
    let scale =
        c.radius.max(c.half_height).max(sphere.radius) + norm(sub(sphere.center, c.center)) + 1.;
    c.error + sphere.error + 1e-9 * scale + 64. * f64::EPSILON * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cylinder, sphere, transform};

    fn placed_sphere(radius: f64, at: [f64; 3]) -> Model {
        transform::affine(
            &sphere(radius).unwrap(),
            [
                [1., 0., 0., at[0]],
                [0., 1., 0., at[1]],
                [0., 0., 1., at[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    }
    fn z_range(model: &Model) -> (f64, f64) {
        model
            .vertices
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v.point[2]), hi.max(v.point[2]))
            })
    }
    fn holes(model: &Model) -> usize {
        model.faces.iter().map(|f| f.holes.len()).sum()
    }
    fn all_ops(c: &Model, s: &Model) -> [Model; 3] {
        let run = |op: &str| {
            let model = boolean(c, s, op).unwrap().unwrap();
            model.validate().unwrap();
            assert!(!model.bodies.is_empty(), "{op}");
            model
        };
        [run("union"), run("difference"), run("intersection")]
    }

    #[test]
    fn ball_on_the_axis_bulging_through_the_wall() {
        // Cylinder radius 3, height 10; sphere radius 4 at mid height: two
        // wall rings at z = 5 ± sqrt(7), no cap contact.
        let c = cylinder(3., 10.).unwrap();
        let s = placed_sphere(4., [0., 0., 5.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert_eq!(holes(&union), 0);
        assert_eq!(
            difference.bodies.len(),
            2,
            "the cylinder is cut into two stubs"
        );
        assert!((z_range(&union).1 - 10.).abs() < 1e-9);
        // The intersection is the sphere with its belt replaced by the wall:
        // the sphere poles at z = 1 and z = 9 survive.
        assert!((z_range(&intersection).1 - 9.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 1.).abs() < 1e-9);
        let ring_level = |m: &Model| {
            m.vertices
                .iter()
                .filter(|v| (v.point[0].hypot(v.point[1]) - 3.).abs() < 1e-9)
                .map(|v| v.point[2])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        assert!((ring_level(&intersection) - (5. + 7f64.sqrt())).abs() < 1e-9);
        let belt = boolean(&s, &c, "difference").unwrap().unwrap();
        belt.validate().unwrap();
        assert_eq!(belt.bodies.len(), 1);
    }

    #[test]
    fn ball_on_the_axis_poking_through_the_top_cap() {
        // Sphere radius 2 centred 1 below the top cap: a ring in the cap,
        // the wall (radius 3) untouched.
        let c = cylinder(3., 10.).unwrap();
        let s = placed_sphere(2., [0., 0., 9.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert_eq!(holes(&union), 1);
        assert_eq!(holes(&difference), 1);
        assert!((z_range(&union).1 - 11.).abs() < 1e-9);
        assert!((z_range(&difference).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 7.).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 10.).abs() < 1e-9);
    }

    #[test]
    fn big_ball_through_wall_and_cap() {
        // Sphere radius 4 centred 2 below the top cap: the upper wall ring
        // is beyond the cap, the lower ring at z = 8 - sqrt(7) is inside, and
        // the cap (radius 3) lies inside the sphere (cap circle sqrt(12) > 3).
        let c = cylinder(3., 10.).unwrap();
        let s = placed_sphere(4., [0., 0., 8.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert_eq!(holes(&union), 0);
        assert!((z_range(&union).1 - 12.).abs() < 1e-9);
        assert!((z_range(&difference).1 - (8. - 7f64.sqrt())).abs() < 1e-9);
        assert_eq!(difference.bodies.len(), 1);
        assert!((z_range(&intersection).1 - 10.).abs() < 1e-9);
        assert!(
            intersection
                .faces
                .iter()
                .any(|f| f.surface.degree_u == 1 && f.surface.degree_v == 1),
            "the cap survives whole inside the intersection"
        );
    }

    #[test]
    fn ball_centred_on_the_top_cap_plane() {
        // The cap plane is a seam plane of the sphere: the ring is four
        // sphere edges and the sphere splits into whole hemispheres.
        let c = cylinder(3., 10.).unwrap();
        let s = placed_sphere(2., [0., 0., 10.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert_eq!(holes(&union), 1);
        assert_eq!(union.faces.len(), 6 + 4);
        assert_eq!(difference.faces.len(), 6 + 4);
        assert_eq!(intersection.faces.len(), 1 + 4);
        assert!((z_range(&union).1 - 12.).abs() < 1e-9);
    }

    #[test]
    fn containment_and_disjoint_pairs() {
        let c = cylinder(3., 10.).unwrap();
        let inside = placed_sphere(1., [0., 0., 5.]);
        let cavity = boolean(&c, &inside, "difference").unwrap().unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        let around = placed_sphere(20., [0., 0., 5.]);
        assert_eq!(
            boolean(&c, &around, "union").unwrap().unwrap().faces.len(),
            8
        );
        assert_eq!(
            boolean(&c, &around, "intersection")
                .unwrap()
                .unwrap()
                .faces
                .len(),
            6
        );
        let far = placed_sphere(1., [0., 0., 30.]);
        assert_eq!(boolean(&c, &far, "union").unwrap().unwrap().bodies.len(), 2);
    }

    #[test]
    fn off_axis_and_tangent_configurations_are_refused() {
        let c = cylinder(3., 10.).unwrap();
        let off = placed_sphere(2., [1., 0., 5.]);
        assert_eq!(
            boolean(&c, &off, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
        let tangent_wall = placed_sphere(3., [0., 0., 5.]);
        assert_eq!(
            boolean(&c, &tangent_wall, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
        let through_rim = placed_sphere(3f64.hypot(2.), [0., 0., 8.]);
        assert_eq!(
            boolean(&c, &through_rim, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
    }
}
