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
