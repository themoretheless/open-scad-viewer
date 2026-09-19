//! Box × sphere: the `Mate` of `sphere_mate` for the six-face `cuboid`
//! (bilinear affine faces over the unit square, unit weights, four-coedge
//! unit-square trims, rigid placement admitted). Every crossing face plane
//! cuts the sphere in a circle; the sphere meets each box edge in at most two
//! hit vertices shared by the two faces at that edge, so the sphere may sit
//! over a face, an edge or a corner, or run through a slab or a bar.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::sphere_sphere::{self, CanonicalSphere};
use crate::sphere_mate::{
    self, BOUNDARY_UV, Circle, Imprint, Mate, PlaneContact, TAU, affine_arc_pcurve, cross, dist,
    dot, face_reversed, norm, plane_contact, point3, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

/// Structural recognition band for the box faces, relative to their size.
const RECOGNITION: f64 = 1e-9;

/// One recognized box face: an affine rectangle `P(u,v) = origin + u U + v V`
/// with its outward unit normal.
#[derive(Clone, Debug)]
struct BoxFace {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    u_len: f64,
    v_len: f64,
    /// Outward unit normal of the solid at this face.
    outward: [f64; 3],
}

impl BoxFace {
    fn uv_of(&self, p: [f64; 3]) -> [f64; 2] {
        let rel = sub(p, self.origin);
        [
            dot(rel, self.u) / (self.u_len * self.u_len),
            dot(rel, self.v) / (self.v_len * self.v_len),
        ]
    }
    fn point_at(&self, uv: [f64; 2]) -> [f64; 3] {
        std::array::from_fn(|k| self.origin[k] + uv[0] * self.u[k] + uv[1] * self.v[k])
    }
}

struct CanonicalBox<'m> {
    model: &'m Model,
    faces: Vec<BoxFace>,
    error: f64,
    /// Box edge -> the two faces at it.
    edge_faces: Vec<Vec<usize>>,
}

/// Recognizes the six-face cuboid, rigidly placed. Returns `None` for
/// anything else; the caller then refuses through the analytic matrix.
fn recognize_box(model: &Model) -> Result<Option<CanonicalBox<'_>>> {
    model.validate()?;
    if model.bodies.len() != 1
        || model.shells.len() != 1
        || model.faces.len() != 6
        || model.vertices.len() != 8
        || model.edges.len() != 12
        || !model.shells[0].closed
    {
        return Ok(None);
    }
    let centroid = {
        let mut c = [0.; 3];
        for vertex in &model.vertices {
            for axis in 0..3 {
                c[axis] += vertex.point[axis] / 8.;
            }
        }
        c
    };
    let mut faces = Vec::with_capacity(6);
    let mut error: f64 = 0.;
    let mut edge_faces = vec![Vec::new(); 12];
    for (index, face) in model.faces.iter().enumerate() {
        let surface = &face.surface;
        if surface.degree_u != 1
            || surface.degree_v != 1
            || surface.periodic_u
            || surface.periodic_v
            || surface.knots_u != [0., 0., 1., 1.]
            || surface.knots_v != [0., 0., 1., 1.]
            || surface.control_points.len() != 2
            || surface.control_points.iter().any(|row| row.len() != 2)
            || surface.weights != [[1., 1.], [1., 1.]]
            || !face.holes.is_empty()
        {
            return Ok(None);
        }
        let wire = &model.loops[face.outer];
        if wire.coedges.len() != 4 {
            return Ok(None);
        }
        let boundary = [
            ([0., 0.], [1., 0.]),
            ([1., 0.], [1., 1.]),
            ([1., 1.], [0., 1.]),
            ([0., 1.], [0., 0.]),
        ];
        let mut seen = [false; 4];
        for coedge in &wire.coedges {
            let mut hit = false;
            for (k, &(from, to)) in boundary.iter().enumerate() {
                if !seen[k] && sphere_sphere::axis_line(&coedge.pcurve, from, to) {
                    seen[k] = true;
                    hit = true;
                    break;
                }
            }
            if !hit || model.edges[coedge.edge].curve.degree != 1 {
                return Ok(None);
            }
            edge_faces[coedge.edge].push(index);
        }
        let p = &surface.control_points;
        let p00 = point3(&p[0][0]);
        let p10 = point3(&p[1][0]);
        let p01 = point3(&p[0][1]);
        let p11 = point3(&p[1][1]);
        let u = sub(p10, p00);
        let v = sub(p01, p00);
        let u_len = norm(u);
        let v_len = norm(v);
        if !(1e-5..=1e6).contains(&u_len) || !(1e-5..=1e6).contains(&v_len) {
            return Ok(None);
        }
        let scale = u_len.max(v_len);
        let affine_deviation = dist(
            p11,
            [
                p00[0] + u[0] + v[0],
                p00[1] + u[1] + v[1],
                p00[2] + u[2] + v[2],
            ],
        );
        if !affine_deviation.is_finite() || affine_deviation > RECOGNITION * scale + 1e-12 {
            return Ok(None);
        }
        error = error.max(affine_deviation);
        let skew = dot(u, v).abs() / u_len.min(v_len);
        if !skew.is_finite() || skew > RECOGNITION * scale {
            return Ok(None);
        }
        error = error.max(skew);
        let normal = cross(u, v);
        let n_len = norm(normal);
        if !(n_len > 0.) || !n_len.is_finite() {
            return Ok(None);
        }
        let surface_normal = normal.map(|x| x / n_len);
        let outward = if face_reversed(model, index) {
            surface_normal.map(|x| -x)
        } else {
            surface_normal
        };
        // The face must look away from the box centroid: this is what makes the
        // half-space classification below a solid classification.
        if dot(outward, sub(p00, centroid)) <= RECOGNITION * scale {
            return Ok(None);
        }
        faces.push(BoxFace {
            origin: p00,
            u,
            v,
            u_len,
            v_len,
            outward,
        });
    }
    for i in 0..6 {
        for j in i + 1..6 {
            let d = dot(faces[i].outward, faces[j].outward).abs();
            if d > RECOGNITION && (d - 1.).abs() > RECOGNITION {
                return Ok(None);
            }
        }
    }
    if edge_faces.iter().any(|f| f.len() != 2) {
        return Ok(None);
    }
    Ok(Some(CanonicalBox {
        model,
        faces,
        error,
        edge_faces,
    }))
}

impl Mate for CanonicalBox<'_> {
    fn model(&self) -> &Model {
        self.model
    }

    fn inside(&self, point: [f64; 3], band: f64) -> Result<bool> {
        let mut worst = f64::NEG_INFINITY;
        for face in &self.faces {
            worst = worst.max(dot(face.outward, sub(point, face.origin)));
        }
        if worst < -band {
            Ok(true)
        } else if worst > band {
            Ok(false)
        } else {
            Err(unsupported(
                "Box/sphere Boolean: region classification falls within the error band \
                 of a box face plane",
            ))
        }
    }

    fn uv_of(&self, face: usize, point: [f64; 3]) -> Result<[f64; 2]> {
        Ok(self.faces[face].uv_of(point))
    }

    fn arc_pcurve(&self, face: usize, arc: &Curve) -> Result<Curve> {
        Ok(affine_arc_pcurve(&self.model.faces[face].surface, arc))
    }

    fn network(&self, imprint: &mut Imprint<'_, Self>) -> Result<Option<SpatialRelation>> {
        let band = imprint.band;
        let sphere = imprint.sphere.clone();
        let contacts: Vec<(PlaneContact, f64)> = self
            .faces
            .iter()
            .map(|face| plane_contact(face.origin, face.outward, &sphere, band))
            .collect();
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Tangent)
        {
            return Err(unsupported(
                "Box/sphere Boolean: a box face plane lies within the tangency band of the sphere",
            ));
        }
        if contacts
            .iter()
            .any(|(kind, _)| *kind == PlaneContact::Outside)
        {
            return Ok(Some(SpatialRelation::Disjoint));
        }
        if contacts
            .iter()
            .all(|(kind, _)| *kind == PlaneContact::Inside)
        {
            return Ok(Some(imprint.mate_contains_sphere()));
        }
        // Section circles of the crossing planes, in each face's own basis
        // so that +phi is counter-clockwise in the face UV.
        for (i, face) in self.faces.iter().enumerate() {
            let (kind, d) = contacts[i];
            if kind != PlaneContact::Crossing {
                continue;
            }
            let rho2 = sphere.radius * sphere.radius - d * d;
            if !(rho2 > 0.) || !rho2.is_finite() {
                return Err(unsupported(
                    "Box/sphere Boolean: section circle radius is not certifiable",
                ));
            }
            let center: [f64; 3] = std::array::from_fn(|k| sphere.center[k] - d * face.outward[k]);
            imprint.circles.push(Circle::new(
                i,
                center,
                rho2.sqrt(),
                face.u.map(|x| x / face.u_len),
                face.v.map(|x| x / face.v_len),
            ));
        }
        self.edge_hits(imprint)?;
        self.face_intervals(imprint)?;
        Ok(None)
    }
}

impl CanonicalBox<'_> {
    /// Sphere ∩ box edges: the shared vertices of the arc network.
    fn edge_hits(&self, imprint: &mut Imprint<'_, Self>) -> Result<()> {
        let model = self.model;
        let c = imprint.sphere.center;
        let r = imprint.sphere.radius;
        let band = imprint.band;
        for (e, edge) in model.edges.iter().enumerate() {
            let p0 = model.vertices[edge.vertices[0]].point;
            let p1 = model.vertices[edge.vertices[1]].point;
            let dir = sub(p1, p0);
            let len = norm(dir);
            let qa = dot(dir, dir);
            let qb = 2. * dot(dir, sub(p0, c));
            let qc = dot(sub(p0, c), sub(p0, c)) - r * r;
            let disc = qb * qb - 4. * qa * qc;
            // Discriminant scale: (2 len r)^2 at a diametral hit.
            let disc_band = 8. * len * r * band;
            if disc.abs() <= disc_band {
                let t = -qb / (2. * qa);
                if (-band / len..=1. + band / len).contains(&t) {
                    return Err(unsupported(
                        "Box/sphere Boolean: the sphere is tangent to a box edge",
                    ));
                }
                continue;
            }
            if disc < 0. {
                continue;
            }
            let sq = disc.sqrt();
            let clear = band / len;
            for t in [(-qb - sq) / (2. * qa), (-qb + sq) / (2. * qa)] {
                if t < -clear || t > 1. + clear {
                    continue;
                }
                if t < 16. * clear + 1e-9 || t > 1. - 16. * clear - 1e-9 {
                    return Err(unsupported(
                        "Box/sphere Boolean: the sphere passes through a box vertex region",
                    ));
                }
                let point: [f64; 3] = std::array::from_fn(|k| p0[k] + t * dir[k]);
                let id = imprint.hit(point, e, t);
                for &face in &self.edge_faces[e] {
                    let Some(circle) = imprint.circles.iter_mut().find(|c| c.face == face) else {
                        return Err(unsupported(
                            "Box/sphere Boolean: edge hit on a face whose plane does not cross",
                        ));
                    };
                    let phi = circle.phi_of(point);
                    circle.hits.push((phi, id));
                }
            }
        }
        Ok(())
    }

    /// Per crossing face: the parts of the circle inside the rectangle.
    fn face_intervals(&self, imprint: &mut Imprint<'_, Self>) -> Result<()> {
        let band = imprint.band;
        for ci in 0..imprint.circles.len() {
            let face = &self.faces[imprint.circles[ci].face];
            let hits = imprint.circles[ci].sorted_hits()?;
            let clear_u = band / face.u_len + BOUNDARY_UV;
            let clear_v = band / face.v_len + BOUNDARY_UV;
            let inside_rect = |uv: [f64; 2]| -> Result<bool> {
                let inside = uv[0] > clear_u
                    && uv[0] < 1. - clear_u
                    && uv[1] > clear_v
                    && uv[1] < 1. - clear_v;
                let outside = uv[0] < -clear_u
                    || uv[0] > 1. + clear_u
                    || uv[1] < -clear_v
                    || uv[1] > 1. + clear_v;
                if inside {
                    Ok(true)
                } else if outside {
                    Ok(false)
                } else {
                    Err(unsupported(
                        "Box/sphere Boolean: section circle grazes the face boundary",
                    ))
                }
            };
            if hits.is_empty() {
                let mut corners_in = 0;
                for corner in [[0., 0.], [1., 0.], [1., 1.], [0., 1.]] {
                    if imprint.inside_sphere(face.point_at(corner))? {
                        corners_in += 1;
                    }
                }
                if corners_in == 4 {
                    // The face lies inside the sphere; the engine classifies
                    // it whole by its centre sample.
                    continue;
                }
                if corners_in != 0 {
                    return Err(unsupported(
                        "Box/sphere Boolean: face corners straddle the sphere without an edge hit",
                    ));
                }
                let circle = &mut imprint.circles[ci];
                let foot = face.uv_of(circle.center);
                let ru = circle.rho / face.u_len;
                let rv = circle.rho / face.v_len;
                if foot[0] - ru > clear_u
                    && foot[0] + ru < 1. - clear_u
                    && foot[1] - rv > clear_v
                    && foot[1] + rv < 1. - clear_v
                {
                    circle.push_ring();
                }
                // Otherwise the circle is disjoint from the rectangle (grazing
                // already refused through the hit scan): face untouched.
                continue;
            }
            let n = hits.len();
            if n % 2 != 0 {
                return Err(unsupported(
                    "Box/sphere Boolean: odd number of edge hits on one section circle",
                ));
            }
            let circle = &mut imprint.circles[ci];
            let mut expected: Option<bool> = None;
            for k in 0..n {
                let lo = hits[k].0;
                let hi = if k + 1 < n {
                    hits[k + 1].0
                } else {
                    hits[0].0 + TAU
                };
                let mid = circle.point_at((lo + hi) / 2.);
                let inside = inside_rect(face.uv_of(mid))?;
                if let Some(previous) = expected {
                    if previous == inside {
                        return Err(unsupported(
                            "Box/sphere Boolean: section arcs do not alternate at edge hits",
                        ));
                    }
                }
                expected = Some(inside);
                if inside {
                    circle.push_interval(lo, hi, hits[k].1, hits[(k + 1) % n].1);
                }
            }
        }
        Ok(())
    }
}

/// Box/sphere Boolean entry: `Ok(None)` unless one operand is the canonical
/// cuboid and the other the canonical sphere. Admitted pairs return the
/// regularized result; everything else is an explicit refusal.
pub(crate) fn boolean(a: &Model, b: &Model, operation: &str) -> Result<Option<Model>> {
    if !matches!(operation, "union" | "difference" | "intersection") {
        return Ok(None);
    }
    let (mate_index, cuboid, sphere) = match (
        recognize_box(a)?,
        sphere_sphere::recognize(b)?,
        recognize_box(b)?,
        sphere_sphere::recognize(a)?,
    ) {
        (Some(cuboid), Some(sphere), _, _) => (0usize, cuboid, sphere),
        (_, _, Some(cuboid), Some(sphere)) => (1usize, cuboid, sphere),
        _ => return Ok(None),
    };
    let band = shared_band(&cuboid, &sphere);
    sphere_mate::run(a, b, mate_index, &cuboid, sphere, band, operation).map(Some)
}

fn shared_band(cuboid: &CanonicalBox<'_>, sphere: &CanonicalSphere) -> f64 {
    let scale = cuboid
        .faces
        .iter()
        .map(|f| f.u_len.max(f.v_len))
        .fold(sphere.radius, f64::max)
        + norm(sub(sphere.center, cuboid.faces[0].origin))
        + 1.;
    cuboid.error + sphere.error + 1e-9 * scale + 64. * f64::EPSILON * scale
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cuboid, sphere, transform};

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
    fn big_box() -> Model {
        cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap()
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
    fn all_ops(b: &Model, s: &Model) -> [Model; 3] {
        let run = |op: &str| {
            let model = boolean(b, s, op).unwrap().unwrap();
            model.validate().unwrap();
            assert!(!model.bodies.is_empty(), "{op}");
            model
        };
        [run("union"), run("difference"), run("intersection")]
    }

    #[test]
    fn recognizes_the_cuboid() {
        let b = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let cuboid = recognize_box(&b).unwrap().expect("cuboid recognized");
        assert_eq!(cuboid.faces.len(), 6);
        for face in &cuboid.faces {
            assert!((norm(face.outward) - 1.).abs() < 1e-12);
        }
        let _ = &cuboid.model;
        assert!(recognize_box(&sphere(1.).unwrap()).unwrap().is_none());
    }

    #[test]
    fn disjoint_pair_resolves_by_empty_algebra() {
        let b = cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let s = placed_sphere(1., [20., 0., 0.]);
        let union = boolean(&b, &s, "union").unwrap().unwrap();
        assert_eq!(union.bodies.len(), 2);
        let difference = boolean(&b, &s, "difference").unwrap().unwrap();
        assert_eq!(difference.faces.len(), 6);
        let intersection = boolean(&b, &s, "intersection").unwrap().unwrap();
        assert!(intersection.is_empty());
    }

    #[test]
    fn contained_sphere_difference_is_a_cavity() {
        let b = big_box();
        let s = placed_sphere(2., [1., -1., 0.5]);
        let result = boolean(&b, &s, "difference").unwrap().unwrap();
        assert_eq!(result.bodies[0].inner_shells.len(), 1);
        assert_eq!(result.faces.len(), 6 + 8);
        assert_eq!(boolean(&b, &s, "union").unwrap().unwrap().faces.len(), 6);
        assert_eq!(
            boolean(&s, &b, "intersection")
                .unwrap()
                .unwrap()
                .faces
                .len(),
            8
        );
    }

    #[test]
    fn box_inside_sphere_resolves_by_containment() {
        let b = cuboid([-1., -1., -1.], [1., 1., 1.]).unwrap();
        let s = placed_sphere(5., [0.2, 0.1, -0.3]);
        let union = boolean(&b, &s, "union").unwrap().unwrap();
        assert_eq!(union.faces.len(), 8);
        let difference = boolean(&s, &b, "difference").unwrap().unwrap();
        assert_eq!(difference.bodies[0].inner_shells.len(), 1);
        assert!(boolean(&b, &s, "difference").unwrap().unwrap().is_empty());
    }

    #[test]
    fn sphere_poking_past_a_corner_without_touching_a_face_is_disjoint() {
        let b = big_box();
        // Crosses all three corner planes, but every face lies outside it.
        let s = placed_sphere(2., [11.5, 11.5, 11.5]);
        let union = boolean(&b, &s, "union").unwrap().unwrap();
        assert_eq!(union.bodies.len(), 2);
    }

    #[test]
    fn dome_over_one_face() {
        let b = big_box();
        let s = placed_sphere(5., [0.3, -0.7, 8.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 1);
        assert!((z_range(&union).1 - 13.).abs() < 1e-9);
        assert_eq!(holes(&difference), 1);
        assert!((z_range(&difference).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 3.).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 10.).abs() < 1e-9);
        let cap = boolean(&s, &b, "difference").unwrap().unwrap();
        cap.validate().unwrap();
        assert!((z_range(&cap).0 - 10.).abs() < 1e-9);
    }

    #[test]
    fn sphere_over_a_box_edge() {
        let b = big_box();
        // Crosses x = 10 and z = 10, hits their shared edge twice, clears
        // y = ±10.
        let s = placed_sphere(4., [9., 0.4, 9.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert!((z_range(&union).1 - 13.).abs() < 1e-9);
        assert!((z_range(&difference).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 10.).abs() < 1e-9);
        assert!(union.faces.len() > 6);
        assert_eq!(holes(&difference), 0);
        assert_eq!(holes(&union), 0);
    }

    #[test]
    fn sphere_over_a_box_corner() {
        let b = big_box();
        let s = placed_sphere(4., [9.3, 8.9, 9.1]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert!((z_range(&union).1 - 13.1).abs() < 1e-9);
        assert!((z_range(&difference).1 - 10.).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 10.).abs() < 1e-9);
        // The corner vertex of the box is inside the sphere: gone from the
        // difference, present in the intersection.
        let has_corner = |m: &Model| {
            m.vertices
                .iter()
                .any(|v| dist(v.point, [10., 10., 10.]) < 1e-9)
        };
        assert!(!has_corner(&difference));
        assert!(has_corner(&intersection));
    }

    #[test]
    fn small_sphere_on_a_big_face_is_a_ring_inside_one_patch() {
        let b = cuboid([-50., -50., -50.], [50., 50., 50.]).unwrap();
        // Centre 0.4 mm below the top face with radius 1: the section circle
        // has radius sqrt(1 - 0.16) and lies inside one octant patch of the
        // sphere (the cap is entirely within the +z, +x, +y octant).
        let s = placed_sphere(1., [20.7, 30.3, 49.6]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 1);
        assert_eq!(holes(&difference), 1);
        assert!((z_range(&union).1 - 50.6).abs() < 1e-9);
        assert!((z_range(&difference).1 - 50.).abs() < 1e-9);
        assert!((z_range(&intersection).0 - 48.6).abs() < 1e-9);
    }

    #[test]
    fn ring_inside_one_sphere_patch_on_a_tilted_face() {
        // Axis-aligned faces always cut a circle around a sphere pole, which
        // crosses four seams. A face whose normal is the octant diagonal
        // (1,1,1)/sqrt(3), cutting 0.9 mm from the centre of a unit sphere,
        // leaves a cap of angular radius 25.8 degrees inside one patch.
        let n = [1. / 3f64.sqrt(); 3];
        let u = [1. / 2f64.sqrt(), -1. / 2f64.sqrt(), 0.];
        let v = [
            n[1] * u[2] - n[2] * u[1],
            n[2] * u[0] - n[0] * u[2],
            n[0] * u[1] - n[1] * u[0],
        ];
        let t = 0.9 - 50.;
        let b = transform::affine(
            &cuboid([-50., -50., -50.], [50., 50., 50.]).unwrap(),
            [
                [u[0], v[0], n[0], t * n[0]],
                [u[1], v[1], n[1], t * n[1]],
                [u[2], v[2], n[2], t * n[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let s = sphere(1.).unwrap();
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 1, "ring hole in the box face");
        assert_eq!(
            holes(&difference),
            2,
            "ring holes in the box face and in the sphere patch"
        );
        let apex = |m: &Model| {
            m.vertices
                .iter()
                .map(|p| p.point[0] * n[0] + p.point[1] * n[1] + p.point[2] * n[2])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        assert!(
            (apex(&union) - 0.9).abs() < 1e-9,
            "no original vertex above the cut"
        );
        assert!(apex(&difference) <= 0.9 + 1e-9);
        assert_eq!(
            intersection.faces.len(),
            1 + 8,
            "disk + seven whole patches + ring-out patch"
        );
        let ring_patch_faces = union
            .faces
            .iter()
            .filter(|f| f.surface.degree_u == 2)
            .count();
        assert_eq!(
            ring_patch_faces, 1,
            "union keeps only the cap disk of the sphere"
        );
    }

    #[test]
    fn sphere_through_a_slab() {
        let b = cuboid([-10., -10., -2.], [10., 10., 2.]).unwrap();
        let s = placed_sphere(5., [0.3, -0.7, 0.1]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 2, "both slab faces carry a ring");
        assert_eq!(holes(&difference), 2);
        assert!((z_range(&intersection).0 + 2.).abs() < 1e-9);
        assert!((z_range(&intersection).1 - 2.).abs() < 1e-9);
        assert!((z_range(&union).1 - 5.1).abs() < 1e-9);
    }

    #[test]
    fn sphere_through_a_bar() {
        // Four crossing planes: the bar runs through the sphere.
        let b = cuboid([-10., -1.5, -1.2], [10., 1.5, 1.2]).unwrap();
        let s = placed_sphere(4., [0.3, 0.1, -0.2]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(union.bodies.len(), 1);
        assert_eq!(intersection.bodies.len(), 1);
        assert_eq!(difference.bodies.len(), 2, "the bar is cut into two stubs");
        assert_eq!(holes(&union), 0);
        assert_eq!(holes(&difference), 0);
        assert!((z_range(&intersection).1 - 1.2).abs() < 1e-9);
        assert!(union.faces.len() > 8);
    }

    #[test]
    fn sphere_centred_on_a_face_plane_runs_along_sphere_seams() {
        // translate([10, 0, 0]) sphere(5) against cube(20, center = true):
        // the face plane x = 10 is a seam plane of the sphere, so the ring
        // is made of four original sphere edges.
        let b = big_box();
        let s = placed_sphere(5., [10., 0., 0.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 1);
        assert_eq!(
            union.faces.len(),
            6 + 4,
            "half the sphere patches survive whole"
        );
        assert_eq!(holes(&difference), 1);
        assert_eq!(difference.faces.len(), 6 + 4);
        assert_eq!(
            intersection.faces.len(),
            1 + 4,
            "disk plus four whole patches"
        );
        let x_max = |m: &Model| {
            m.vertices
                .iter()
                .map(|v| v.point[0])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        assert!((x_max(&union) - 15.).abs() < 1e-9);
        assert!((x_max(&difference) - 10.).abs() < 1e-9);
        // Off the face centre, the plane still passes through the sphere centre.
        let s2 = placed_sphere(3., [10., 0., 5.]);
        let [u2, d2, i2] = all_ops(&b, &s2);
        assert_eq!(holes(&u2), 1);
        assert_eq!(holes(&d2), 1);
        assert_eq!(i2.faces.len(), 5);
    }

    #[test]
    fn sphere_centred_on_a_box_edge_hits_the_sphere_poles() {
        // translate([10, 10, 0]) sphere(4): both crossing planes are seam
        // planes and the box edge meets the sphere at its ±z poles.
        let b = big_box();
        let s = placed_sphere(4., [10., 10., 0.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 0);
        assert_eq!(
            union.faces.len(),
            6 + 6,
            "six of eight patches lie outside the box"
        );
        assert_eq!(intersection.faces.len(), 2 + 2);
        assert!(
            (z_range(&intersection).1 - 4.).abs() < 1e-9,
            "the pole vertex survives"
        );
        assert_eq!(difference.faces.len(), 6 + 2);
    }

    #[test]
    fn sphere_centred_on_a_box_corner() {
        let b = big_box();
        let s = placed_sphere(4., [10., 10., 10.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(union.faces.len(), 6 + 7);
        assert_eq!(intersection.faces.len(), 3 + 1);
        assert_eq!(difference.faces.len(), 6 + 1);
    }

    #[test]
    fn seam_ring_split_by_a_box_edge_off_the_poles() {
        // Centre on the face plane x = 10 but shifted toward y = 10: the
        // seam ring on x = 10 is cut by the box edge at (10, 10, ±4), which
        // are not sphere vertices, so two sphere edges are split there.
        let b = big_box();
        let s = placed_sphere(5., [10., 7., 0.]);
        let [union, difference, intersection] = all_ops(&b, &s);
        assert_eq!(holes(&union), 0);
        assert!(union.faces.len() > 6);
        assert!((z_range(&intersection).1 - 5.).abs() < 1e-9);
        assert!(difference.faces.iter().all(|f| f.holes.is_empty()));
    }

    #[test]
    fn tangent_and_vertex_configurations_are_refused() {
        let b = big_box();
        let tangent_plane = placed_sphere(5., [0., 0., 15.]);
        assert_eq!(
            boolean(&b, &tangent_plane, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
        let k = 10. + 3. / 3f64.sqrt();
        let through_corner = placed_sphere(3., [k, k, k]);
        assert_eq!(
            boolean(&b, &through_corner, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
        // Tangent to the +x/+z edge.
        let tangent_edge = placed_sphere(2., [10. + 2f64.sqrt(), 0.3, 10. + 2f64.sqrt()]);
        assert_eq!(
            boolean(&b, &tangent_edge, "union").unwrap_err().code,
            "BREP_UNSUPPORTED_OPERATION"
        );
    }
}
