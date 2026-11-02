use modeling_clean::{
    boolean::{self, Boolean},
    curve::Curve,
    math::*,
    mesh::Mesh,
    surface::Surface,
};
use std::f64::consts::{PI, TAU};
fn near(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() <= t, "{a} != {b}")
}
fn point(a: Point, b: Point) {
    near(norm(sub(a, b)), 0., 1e-10)
}
fn closed(mesh: &Mesh) {
    let t = mesh.topology();
    assert!(t.is_closed_manifold(), "{t:?}");
    assert!(mesh.signed_volume() > 0.);
}

#[test]
fn rational_circle_and_knot_edits_preserve_locus() {
    let c = Curve::circle([2., -3., 5.], 7.).unwrap();
    let refined = c.insert_knot(0.13).unwrap().insert_knot(0.13).unwrap();
    for i in 0..=200 {
        let t = i as f64 / 200.;
        let p = c.evaluate(t).unwrap();
        near((p[0] - 2.).hypot(p[1] + 3.), 7., 1e-12);
        near(p[2], 5., 1e-12);
        point(p, refined.evaluate(t).unwrap());
    }
    let [left, right] = c.split(0.43).unwrap();
    for i in 0..=100 {
        let t = i as f64 / 100.;
        point(
            left.evaluate(t * 0.43).unwrap(),
            c.evaluate(t * 0.43).unwrap(),
        );
        point(
            right.evaluate(0.43 + t * 0.57).unwrap(),
            c.evaluate(0.43 + t * 0.57).unwrap(),
        );
    }
    let trimmed = c.trim(0.2, 0.8).unwrap();
    assert_eq!(trimmed.domain(), [0.2, 0.8]);
    for t in [0.2, 0.3, 0.65, 0.8] {
        point(trimmed.evaluate(t).unwrap(), c.evaluate(t).unwrap());
        point(
            c.reversed().unwrap().evaluate(1. - t).unwrap(),
            c.evaluate(t).unwrap(),
        );
    }
}
#[test]
fn spline_endpoint_multiplicity_and_invalid_inputs() {
    assert!(
        Curve::new(
            2,
            vec![0., 0., 0., 1., 1., 1.],
            vec![[0.; 3]; 3],
            vec![1., 0., 1.]
        )
        .is_err()
    );
    assert!(Curve::circle([0.; 3], -1.).is_err());
    assert!(Curve::line([0.; 3], [0.; 3]).is_err());
    let c = Curve::bezier(&[[0., 0., 0.], [2., 4., 0.], [4., 0., 0.]]).unwrap();
    point(c.evaluate(0.5).unwrap(), [2., 2., 0.]);
    assert!(c.evaluate(f64::NAN).is_err());
    assert!(c.insert_knot(0.).is_err());
    assert!(c.trim(0.8, 0.2).is_err());
    let halves = c.split(0.5).unwrap();
    point(
        halves[0].evaluate(0.5).unwrap(),
        halves[1].evaluate(0.5).unwrap(),
    );
}
#[test]
fn rational_surface_construction_matches_analytic_constraints() {
    let circle = Curve::circle([0.; 3], 2.).unwrap();
    let extrude = Surface::extrude(&circle, [0., 0., 3.]).unwrap();
    for i in 0..=10 {
        for j in 0..=10 {
            let p = extrude.evaluate(i as f64 / 10., j as f64 / 10.).unwrap();
            near(p[0].hypot(p[1]), 2., 1e-12);
            near(p[2], 3. * i as f64 / 10., 1e-12);
        }
    }
    let profile = Curve::line([2., 0., 0.], [2., 0., 4.]).unwrap();
    let revolved = Surface::revolve(&profile, [0.; 3], [0., 0., 1.], TAU).unwrap();
    for i in 0..=20 {
        let p = revolved.evaluate(i as f64 / 20., 0.4).unwrap();
        near(p[0].hypot(p[1]), 2., 1e-12);
        near(p[2], 1.6, 1e-12);
    }
    let translated = profile
        .transformed(&Affine::translation([3., 0., 0.]).unwrap())
        .unwrap();
    let loft = Surface::loft(&[profile.clone(), translated]).unwrap();
    point(loft.evaluate(0.25, 0.5).unwrap(), [2.75, 0., 2.]);
    let sweep = Surface::sweep(
        &circle,
        &[[0., 0., 0.], [0., 0., 3.], [3., 0., 3.]],
        [0., 1., 0.],
        PI / 2.,
    )
    .unwrap();
    assert!(finite(sweep.evaluate(0.4, 0.2).unwrap()));
    assert!(Surface::sweep(&circle, &[[0.; 3], [0., 0., 1.], [0.; 3]], [0., 1., 0.], 0.).is_err());
}
#[test]
fn primitives_are_closed_and_converge_to_analytic_volume() {
    let cube = Mesh::cuboid(Bounds::new([0.; 3], [2., 3., 4.]).unwrap()).unwrap();
    closed(&cube);
    near(cube.signed_volume(), 24., 1e-12);
    near(cube.surface_area(), 52., 1e-12);
    let cylinder = Mesh::cylinder(2., 3., 96).unwrap();
    closed(&cylinder);
    near(cylinder.signed_volume(), PI * 12., 0.03);
    let sphere = Mesh::sphere(2., 32, 64).unwrap();
    closed(&sphere);
    near(sphere.signed_volume(), 32. * PI / 3., 0.15);
    let torus = Mesh::torus(4., 1., [64, 32]).unwrap();
    closed(&torus);
    near(torus.signed_volume(), 8. * PI * PI, 0.8);
    let reflected = cube
        .transformed(&Affine::scaling([-2., 3., 1.]).unwrap())
        .unwrap();
    closed(&reflected);
    near(reflected.signed_volume(), 144., 1e-10);
}
#[test]
fn concave_extrusion_reversed_winding_and_mesh_interchange() {
    let profile = vec![[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 3.], [0., 3.]];
    let a = Mesh::extrude(&profile, 2.).unwrap();
    closed(&a);
    near(a.signed_volume(), 10., 1e-12);
    let b = Mesh::extrude(&profile.iter().copied().rev().collect::<Vec<_>>(), 2.).unwrap();
    closed(&b);
    near(b.signed_volume(), 10., 1e-12);
    assert_eq!(a, Mesh::from_obj(&a.to_obj()).unwrap());
    assert!(a.to_stl().unwrap().starts_with("solid independent"));
    assert!(Mesh::extrude(&[[0., 0.], [2., 2.], [0., 2.], [2., 0.]], 1.).is_err());
    assert!(Mesh::new(vec![[0.; 3]], vec![[0, 1, 2]]).is_err());
    assert!(Mesh::from_obj("v 0 0 0\nf 0 1 2").is_err());
}
#[test]
fn box_booleans_have_correct_volume_cavities_and_empty_semantics() {
    let a = Bounds::new([0.; 3], [2.; 3]).unwrap();
    let b = Bounds::new([1.; 3], [3.; 3]).unwrap();
    for (op, expected) in [
        (Boolean::Union, 15.),
        (Boolean::Intersection, 1.),
        (Boolean::Difference, 7.),
    ] {
        let m = boolean::boxes(&[a], &[b], op).unwrap();
        closed(&m);
        near(m.signed_volume(), expected, 1e-10);
    }
    // XOR of intersecting solids has edge/point contacts, not a regular manifold.
    assert!(boolean::boxes(&[a], &[b], Boolean::Xor).is_err());
    assert!(
        boolean::boxes(&[a], &[a], Boolean::Difference)
            .unwrap()
            .is_empty()
    );
    assert!(boolean::boxes(&[], &[], Boolean::Union).unwrap().is_empty());
    let cavity = boolean::boxes(
        &[Bounds::new([0.; 3], [3.; 3]).unwrap()],
        &[Bounds::new([1.; 3], [2.; 3]).unwrap()],
        Boolean::Difference,
    )
    .unwrap();
    closed(&cavity);
    near(cavity.signed_volume(), 26., 1e-10);
    let other = Bounds::new([4.; 3], [5.; 3]).unwrap();
    let disconnected = boolean::boxes(&[a], &[other], Boolean::Union).unwrap();
    near(disconnected.signed_volume(), 9., 1e-12);
    assert_eq!(disconnected.topology().components, 2);
}
#[test]
fn near_adjacent_binary64_box_boundaries_do_not_use_rounded_midpoints() {
    let lo = 1.;
    let hi = f64::from_bits(1f64.to_bits() + 2);
    let m = boolean::boxes(
        &[Bounds::new([lo, 0., 0.], [hi, 1., 1.]).unwrap()],
        &[],
        Boolean::Union,
    )
    .unwrap();
    near(m.signed_volume(), hi - lo, 1e-30);
}
#[test]
fn raycast_returns_physical_distance_and_triangle_barycentrics() {
    let cube = Mesh::cuboid(Bounds::new([0.; 3], [1.; 3]).unwrap()).unwrap();
    let hit = cube
        .raycast([-2., 0.3, 0.7], [10., 0., 0.])
        .unwrap()
        .unwrap();
    near(hit.distance, 2., 1e-12);
    point(hit.point, [0., 0.3, 0.7]);
    near(hit.barycentric.iter().sum(), 1., 1e-12);
    assert_eq!(cube.adjacent_triangles(hit.triangle).unwrap().len(), 3);
    assert!(cube.raycast([-2., 4., 0.], [1., 0., 0.]).unwrap().is_none());
    assert!(cube.raycast([0.; 3], [0.; 3]).is_err());
}
#[test]
fn loft_caps_and_side_incidence_are_consistent() {
    let a = vec![[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
    let b = a.iter().map(|p| [p[0] * 2., p[1] * 2.]).collect();
    let m = Mesh::loft(&[a, b], &[0., 3.]).unwrap();
    closed(&m);
    near(m.signed_volume(), 28., 1e-12);
}

#[test]
fn profile_edits_shells_sections_and_mass_properties() {
    let rounded = modeling_clean::profile::rounded_rectangle([6., 4.], 0.5, 8).unwrap();
    let mesh = Mesh::extrude(&rounded, 2.).unwrap();
    closed(&mesh);
    near(mesh.signed_volume(), 2. * (24. - 1. + PI * 0.25), 0.02);
    let chamfered = modeling_clean::profile::chamfered_rectangle([6., 4.], 0.5).unwrap();
    let cut = Mesh::extrude(&chamfered, 2.).unwrap();
    near(cut.signed_volume(), 47., 1e-12);
    let bounds = Bounds::new([0.; 3], [4.; 3]).unwrap();
    let shell =
        boolean::mesh_cells(&boolean::shell_box(bounds, 1., Some((2, true))).unwrap()).unwrap();
    closed(&shell);
    near(shell.signed_volume(), 52., 1e-10);
    let cube = Mesh::cuboid(bounds).unwrap();
    let (volume, center) = cube.mass_properties().unwrap();
    near(volume, 64., 1e-12);
    point(center, [2.; 3]);
    let segments = cube.section_segments([0., 0., 2.], 4.).unwrap();
    assert_eq!(segments.len(), 8);
    assert!(segments.iter().flatten().all(|p| (p[2] - 2.).abs() < 1e-12));
    assert!(cube.section_segments([0., 0., 1.], 0.).is_err());
    assert!(boolean::shell_box(bounds, 3., None).is_err());
}

#[test]
fn pinched_vertex_is_not_a_manifold_solid() {
    // Two closed tetrahedra share one vertex but no edge: edge counts alone pass.
    let m = Mesh::new(
        vec![
            [0., 0., 0.],
            [1., 0., 0.],
            [0., 1., 0.],
            [0., 0., 1.],
            [-1., 0., 0.],
            [0., -1., 0.],
            [0., 0., -1.],
        ],
        vec![
            [0, 2, 1],
            [0, 1, 3],
            [1, 2, 3],
            [2, 0, 3],
            [0, 4, 5],
            [0, 6, 4],
            [4, 6, 5],
            [5, 6, 0],
        ],
    )
    .unwrap();
    let t = m.topology();
    assert_eq!(t.nonmanifold_edges, 0);
    assert_eq!(t.nonmanifold_vertices, 1);
    assert!(!t.is_closed_manifold());
}

#[test]
fn rational_derivatives_match_curvature_and_surface_fundamental_forms() {
    let c = Curve::circle([0.; 3], 3.).unwrap();
    for t in [0., 0.13, 0.25, 0.37, 0.99, 1.] {
        let d = c.differential(t).unwrap();
        near(dot(d.point, d.first), 0., 1e-11);
        near(d.curvature().unwrap(), 1. / 3., 1e-12);
        near(norm(d.tangent().unwrap()), 1., 1e-12);
    }
    let line = Curve::line([0., 0., 0.], [1., 0., 0.]).unwrap();
    let d = line.differential(0.4).unwrap();
    point(d.first, [1., 0., 0.]);
    point(d.second, [0.; 3]);
    near(d.curvature().unwrap(), 0., 1e-15);
    let cylinder = Surface::extrude(&c, [0., 0., 5.]).unwrap();
    let d = cylinder.differential(0.4, 0.13).unwrap();
    let [k, h] = d.curvatures().unwrap();
    near(k, 0., 1e-12);
    near(h.abs(), 1. / 6., 1e-12);
    near(norm(d.normal().unwrap()), 1., 1e-12);
    assert!(cylinder.differential(4., 0.).is_err());
}
