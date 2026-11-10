//! Axial cone × sphere: the `Mate` of `sphere_mate` for the canonical cone
//! (a frustum with two caps, or a true cone with one cap and a shared apex;
//! four ruled quarter-arc side patches, rigid placement admitted) against a
//! sphere centred on the cone axis.
//!
//! On the axis every contact is an exact circle. A cap plane cuts the sphere
//! in a circle concentric with the cap. The side, whose radius grows linearly
//! with the axial coordinate `t` measured from the bottom ring,
//! `rho(t) = r_bottom + m t`, meets the sphere where
//! `(1 + m^2) t^2 + 2 (r_bottom m - z_s) t + r_bottom^2 + z_s^2 - r^2 = 0`.
//! Each root inside the side is a horizontal ring, an iso-v curve of the side
//! patches (a side patch interpolates linearly between its two ring arcs), so
//! its pcurve is a straight line, as for the cylinder. Off the axis the
//! contact is a quartic and is refused; tangencies, rims and the apex are
//! refused rather than guessed.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::sphere_sphere::{self, CanonicalSphere};
use crate::intersections::{CanonicalCone, recognize_cone};
use crate::sphere_mate::{
    self, Circle, Imprint, Mate, PlaneContact, TAU, affine_arc_pcurve, affine_uv_of, dot, norm,
    plane_contact, point3, quarter_arc_parameter, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

const QUARTER: f64 = std::f64::consts::FRAC_PI_2;

struct AxialCone<'m> {
    model: &'m Model,
    canon: CanonicalCone,
    /// Quadrant of each side face, `usize::MAX` for caps.
    quadrant_of: Vec<usize>,
    /// Ruling edge at the start (u = 0) of each side quadrant with the axial
    /// coordinates (from the bottom ring) of its t = 0 and t = 1 ends.
    seams: [(usize, f64, f64); 4],
}

impl<'m> AxialCone<'m> {
    fn recognize(model: &'m Model) -> Result<Option<Self>> {
        let Some(canon) = recognize_cone(model)? else {
            return Ok(None);
        };

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
            if edge.degenerate {
                return Ok(None);
            }
            let axial = |v: usize| dot(sub(model.vertices[v].point, canon.bottom), canon.axis);
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

    /// Axial coordinate from the bottom ring and the radial offset vector.
    fn local(&self, p: [f64; 3]) -> (f64, [f64; 3]) {
        let d = sub(p, self.canon.bottom);
        let axial = dot(d, self.canon.axis);
        let perp: [f64; 3] = std::array::from_fn(|k| d[k] - axial * self.canon.axis[k]);
        (axial, perp)
    }

    fn radius_at(&self, t: f64) -> f64 {
        self.canon.r_bottom + self.canon.slope * t
    }

    /// Distance factor from a horizontal radial excess to the distance from
    /// the side surface.
    fn normal_scale(&self) -> f64 {
        1. / self.canon.slope.hypot(1.)
    }

    /// Cap at the bottom (`k = 0`) or top (`k = 1`) with its plane origin,
    /// outward normal and ring radius; `None` at an apex.
    fn cap(&self, k: usize) -> Option<(usize, [f64; 3], [f64; 3], f64)> {
        let c = &self.canon;
        let face = c.caps[k]?;
        let (t, sign, radius) = if k == 0 {
            (0., -1., c.r_bottom)
        } else {
            (c.height, 1., c.r_top)
        };
        let origin = std::array::from_fn(|i| c.bottom[i] + t * c.axis[i]);
        Some((face, origin, c.axis.map(|x| x * sign), radius))
    }
}

impl Mate for AxialCone<'_> {
    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let (axial, perp) = self.local(point);
        let below = -axial;
        let above = axial - self.canon.height;
        let radial = (norm(perp) - self.radius_at(axial)) * self.normal_scale();
        let worst = below.max(above).max(radial);
        if worst < -band {
            Ok(true)
        } else if worst > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Cone/sphere Boolean: region classification falls within the error band of the \
                 cone boundary",
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
                "Cone/sphere Boolean: network vertex outside its side patch",
            ));
        }
        let (sin, cos) = delta.clamp(0., QUARTER).sin_cos();
        let u = quarter_arc_parameter(cos, sin).clamp(0., 1.);
        let v = (axial / self.canon.height).clamp(0., 1.);
        Ok([u, v])
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        if self.quadrant_of[face] == usize::MAX {
            return Ok(affine_arc_pcurve(&self.model.faces[face].surface, arc));
        }
        // A side ring piece between two seams is the patch's own u-arc at
        // constant v: a straight pcurve, verified by the engine.
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
        let offset = norm(perp);
        let widest = c.r_bottom.max(c.r_top);
        if offset > widest + r + band || zs < -r - band || zs > c.height + r + band {
            return Ok(Some(SpatialRelation::Disjoint));
        }
        if offset > band {
            return Err(unsupported(
                "Cone/sphere Boolean: only a sphere centred on the cone axis meets the side in \
                 exact circles; a general position needs a quartic and is refused",
            ));
        }
        // On the axis the distance to the (infinite) side line in the
        // meridian plane is rho(z_s) / sqrt(1 + m^2); it bounds the distance
        // to the finite side from below, so containment stays conservative.
        let side_clearance = self.radius_at(zs) * self.normal_scale();
        if zs - r > band && zs + r < c.height - band && side_clearance > r + band {
            return Ok(Some(imprint.mate_contains_sphere()));
        }
        // Caps.
        let mut crossing_caps = Vec::new();
        for k in 0..2 {
            let Some((face, origin, outward, radius)) = self.cap(k) else {
                continue;
            };
            let (kind, d) = plane_contact(origin, outward, &sphere, band);
            match kind {
                PlaneContact::Tangent => {
                    return Err(unsupported(
                        "Cone/sphere Boolean: a cap plane lies within the tangency band of the \
                         sphere",
                    ));
                }
                PlaneContact::Outside => return Ok(Some(SpatialRelation::Disjoint)),
                PlaneContact::Inside => {}
                PlaneContact::Crossing => crossing_caps.push((face, origin, radius, d)),
            }
        }
        // Side rings: roots of the quadratic in t.
        let m = c.slope;
        let qa = 1. + m * m;
        let qb = 2. * (c.r_bottom * m - zs);
        let qc = c.r_bottom * c.r_bottom + zs * zs - r * r;
        let discriminant = qb * qb - 4. * qa * qc;
        let scale = (qb * qb).max((4. * qa * qc).abs()).max(1.);
        let mut rings = Vec::new();
        if discriminant > 1e-9 * scale + band * r {
            let root = discriminant.sqrt();
            for t in [(-qb - root) / (2. * qa), (-qb + root) / (2. * qa)] {
                // Root positions beyond the side mean no ring there.
                if t < -band || t > c.height + band {
                    continue;
                }
                if t < band || t > c.height - band {
                    return Err(unsupported(
                        "Cone/sphere Boolean: a side ring falls on a rim or on the apex",
                    ));
                }
                rings.push(t);
            }
        } else if discriminant >= -(1e-9 * scale + band * r) {
            // A double root: the sphere touches the side along a circle.
            let t = -qb / (2. * qa);
            if (-band..=c.height + band).contains(&t) {
                return Err(unsupported(
                    "Cone/sphere Boolean: the sphere is tangent to the cone side",
                ));
            }
        }
        if rings.is_empty() && crossing_caps.is_empty() {
            // No boundary crossing at all: either the sphere lies inside the
            // cone, or the two are disjoint. One point decides, away from
            // every band: the sphere centre is inside iff contained.
            return Ok(Some(if self.inside(sphere.center, band)? {
                imprint.mate_contains_sphere()
            } else {
                SpatialRelation::Disjoint
            }));
        }
        for (face, origin, radius, d) in crossing_caps {
            let rho = (r * r - d * d).sqrt();
            if rho < radius - band {
                let surface = &self.model.faces[face].surface;
                let p00 = point3(&surface.control_points[0][0]);
                let u = sub(point3(&surface.control_points[1][0]), p00);
                let v = sub(point3(&surface.control_points[0][1]), p00);
                let mut circle = Circle::new(
                    face,
                    origin,
                    rho,
                    u.map(|x| x / norm(u)),
                    v.map(|x| x / norm(v)),
                );
                circle.push_ring();
                imprint.circles.push(circle);
            } else if rho > radius + band {
                // Cap entirely inside the sphere: classified whole.
            } else {
                return Err(unsupported(
                    "Cone/sphere Boolean: the sphere passes through a cap rim",
                ));
            }
        }
        for t in rings {
            self.side_ring(imprint, t)?;
        }
        Ok(None)
    }
}

impl AxialCone<'_> {
    /// One ring at axial coordinate `t` from the bottom: four hits on the
    /// ruling seams and one quarter interval per side patch.
    fn side_ring(&self, imprint: &mut Imprint<'_, Self>, t: f64) -> Result<()> {
        let c = &self.canon;
        let [x, y] = c.frame;
        let radius = self.radius_at(t);
        let ring_center: [f64; 3] = std::array::from_fn(|i| c.bottom[i] + t * c.axis[i]);
        let mut hits = [usize::MAX; 4];
        for q in 0..4 {
            let (edge, z0, z1) = self.seams[q];
            let s = (t - z0) / (z1 - z0);
            if !(1e-9..=1. - 1e-9).contains(&s) {
                return Err(unsupported(
                    "Cone/sphere Boolean: side ring hit outside its ruling edge",
                ));
            }
            let (sin, cos) = (q as f64 * QUARTER).sin_cos();
            let point: [f64; 3] =
                std::array::from_fn(|i| ring_center[i] + radius * (cos * x[i] + sin * y[i]));
            hits[q] = imprint.hit(point, edge, s);
        }
        for q in 0..4 {
            let mut circle = Circle::new(c.sides[q], ring_center, radius, x, y);
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

/// Cone/sphere Boolean entry: `Ok(None)` unless one operand is the canonical
/// cone or frustum and the other the canonical sphere.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (mate_index, cone, sphere) = match (
        AxialCone::recognize(a)?,
        sphere_sphere::recognize(b)?,
        AxialCone::recognize(b)?,
        sphere_sphere::recognize(a)?,
    ) {
        (Some(cone), Some(sphere), _, _) => (0usize, cone, sphere),
        (_, _, Some(cone), Some(sphere)) => (1usize, cone, sphere),
        _ => return Ok(None),
    };
    let band = shared_band(&cone, &sphere);
    sphere_mate::run([a, b], mate_index, &cone, sphere, band, operation).map(Some)
}

fn shared_band(cone: &AxialCone<'_>, sphere: &CanonicalSphere) -> f64 {
    let c = &cone.canon;
    let scale = c.r_bottom.max(c.r_top).max(c.height).max(sphere.radius)
        + norm(sub(sphere.center, c.bottom))
        + 1.;
    c.error + sphere.error + 1e-9 * scale + 64. * f64::EPSILON * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{frustum, sphere, transform};

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
    fn all_ops(c: &Model, s: &Model) -> [Model; 3] {
        let run = |op: &str| {
            let model = boolean(c, s, op)
                .unwrap_or_else(|e| panic!("{op}: {e:?}"))
                .unwrap();
            model.validate().unwrap();
            assert!(!model.bodies.is_empty(), "{op}");
            model
        };
        [run("union"), run("difference"), run("intersection")]
    }
    /// Axial levels of the vertices that sit on the cone side.
    fn side_levels(model: &Model, r_bottom: f64, slope: f64) -> Vec<f64> {
        let mut levels: Vec<f64> = model
            .vertices
            .iter()
            .filter(|v| {
                let rho = v.point[0].hypot(v.point[1]);
                (rho - (r_bottom + slope * v.point[2])).abs() < 1e-9
                    && v.point[2] > 1e-9
                    && v.point[2] < 10. - 1e-9
            })
            .map(|v| v.point[2])
            .collect();
        levels.sort_by(f64::total_cmp);
        levels.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        levels
    }

    #[test]
    fn ball_on_the_axis_cut_by_the_frustum_side() {
        // Frustum r 4 -> 2 over height 10 (slope -0.2); sphere radius 3.5 at
        // z = 5. Side rings solve 1.04 t^2 - 11.6 t + 28.75 = 0.
        let c = frustum(4., 2., 10.).unwrap();
        let s = placed_sphere(3.5, [0., 0., 5.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        let (qa, qb, qc) = (1.04_f64, -11.6_f64, 28.75_f64);
        let root = (qb * qb - 4. * qa * qc).sqrt();
        let expected = [(-qb - root) / (2. * qa), (-qb + root) / (2. * qa)];
        for model in [&union, &difference, &intersection] {
            let levels = side_levels(model, 4., -0.2);
            assert_eq!(levels.len(), 2, "{levels:?}");
            for (got, want) in levels.iter().zip(expected) {
                assert!((got - want).abs() < 1e-9, "{got} vs {want}");
            }
        }
        assert_eq!(difference.bodies.len(), 2, "the frustum is cut in two");
        assert!((z_range(&union).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 1.5).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 8.5).abs() < 1e-9);
    }

    #[test]
    fn ball_through_the_top_cap_of_a_frustum() {
        // Sphere radius 1.5 centred 0.5 below the top cap (radius 2): a cap
        // circle of radius sqrt(2) and no side contact.
        let c = frustum(4., 2., 10.).unwrap();
        let s = placed_sphere(1.5, [0., 0., 9.5]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert!((z_range(&union).1 - 11.).abs() < 1e-9);
        assert!((z_range(&difference).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 8.).abs() < 1e-9);
        assert_eq!(union.faces.iter().map(|f| f.holes.len()).sum::<usize>(), 1);
    }

    #[test]
    fn true_cone_cap_contact_keeps_the_apex() {
        // Apex cone r 4 at z = 0 to the apex at z = 8. A sphere of radius 1
        // centred 0.5 below the base cuts only the base cap.
        let c = frustum(4., 0., 8.).unwrap();
        let s = placed_sphere(1., [0., 0., -0.5]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert!((z_range(&union).1 - 8.).abs() < 1e-9, "the apex survives");
        assert!((z_range(&union).0 + 1.5).abs() < 1e-9);
        assert!(z_range(&difference).0.abs() < 1e-9);
        assert!((z_range(&intersection).1 - 0.5).abs() < 1e-9);
    }

    #[test]
    fn true_cone_with_a_ball_around_its_middle() {
        // Apex cone r 4 -> apex at z = 8 (slope -0.5); a sphere of radius 2.5
        // at z = 4 crosses the side twice, away from the apex.
        let c = frustum(4., 0., 8.).unwrap();
        let s = placed_sphere(2.5, [0., 0., 4.]);
        let [union, difference, intersection] = all_ops(&c, &s);
        assert!((z_range(&union).1 - 8.).abs() < 1e-9, "the apex survives");
        assert_eq!(difference.bodies.len(), 2);
        assert!((z_range(&intersection).0 - 1.5).abs() < 1e-9);
    }

    #[test]
    fn containment_and_disjoint_pairs() {
        let c = frustum(4., 2., 10.).unwrap();
        let inside = placed_sphere(1., [0., 0., 4.]);
        let cavity = boolean(&c, &inside, "difference").unwrap().unwrap();
        assert_eq!(cavity.bodies[0].inner_shells.len(), 1);
        let around = placed_sphere(30., [0., 0., 5.]);
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
        let beside = placed_sphere(1., [9., 0., 5.]);
        assert_eq!(boolean(&c, &beside, "union").unwrap().unwrap().bodies.len(), 2);
    }

    #[test]
    fn off_axis_tangent_rim_and_apex_configurations_are_refused() {
        let c = frustum(4., 2., 10.).unwrap();
        let refused = |s: Model| {
            assert_eq!(
                boolean(&c, &s, "union").unwrap_err().code,
                "BREP_UNSUPPORTED_OPERATION"
            );
        };
        refused(placed_sphere(2., [1., 0., 5.]));
        // Tangent to the side: distance from the axis point at z = 5 to the
        // side line is rho(5) / sqrt(1.04) = 3 / sqrt(1.04).
        refused(placed_sphere(3. / 1.04_f64.sqrt(), [0., 0., 5.]));
        // Through the top rim: radius sqrt(2^2 + 1^2) centred 1 below it.
        refused(placed_sphere(5_f64.sqrt(), [0., 0., 9.]));
    }
}
