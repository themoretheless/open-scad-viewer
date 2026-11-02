use modeling_kernel_informed::{
    brep::{Body, Frame, Profile},
    curve::Curve,
    loft,
    math::*,
    planar::{Convex, Plane},
};
use std::f64::consts::{FRAC_PI_4, PI};
fn near(a: f64, b: f64, t: f64) {
    assert!((a - b).abs() <= t, "{a} != {b}")
}
fn point(a: Point, b: Point) {
    near(norm(sub(a, b)), 0., 1e-9)
}

#[test]
fn retained_disk_extrusion_is_independent_of_display_resolution() {
    let body = Body::extrude(
        Profile::disk([0.; 2], 2.).unwrap(),
        Frame::xy(),
        [0., 0., 5.],
        1e-9,
    )
    .unwrap();
    let audit = body.audit().unwrap();
    assert_eq!((audit.vertices, audit.edges, audit.faces), (8, 12, 6));
    assert_eq!(body.tracking().len(), 4);
    near(body.volume(), 20. * PI, 1e-12);
    point(body.centroid(), [0., 0., 2.5]);
    let coarse = body.tessellate(0.2, 4096).unwrap();
    let fine = body.tessellate(0.002, 4096).unwrap();
    assert!(coarse.mesh.topology().is_closed_manifold());
    assert!(fine.mesh.topology().is_closed_manifold());
    assert!(fine.mesh.triangles().len() > coarse.mesh.triangles().len());
    assert!(fine.boundary_chord_deviation <= 0.002);
    assert_eq!(fine.triangle_faces.len(), fine.mesh.triangles().len());
    assert!(fine.triangle_faces.iter().all(|i| *i < body.faces().len()));
    assert_eq!(body.audit().unwrap(), audit);
    near(body.volume(), 20. * PI, 1e-12);
    for track in body.tracking() {
        let face = &body.faces()[track.side_face];
        for u in [0., 0.3, 1.] {
            for v in [0., 0.27, 1.] {
                let p = face.carrier.evaluate(u, v).unwrap();
                near(p[0].hypot(p[1]), 2., 1e-12);
                near(p[2], 5. * u, 1e-12);
                let raw = face.carrier.differential(u, v).unwrap().normal().unwrap();
                let outward = scale(raw, if face.reversed { -1. } else { 1. });
                assert!(dot(outward, [p[0], p[1], 0.]) > 0.);
            }
        }
    }
}
#[test]
fn annular_body_keeps_hole_topology_and_analytic_volume() {
    let body = Body::extrude(
        Profile::annulus([1., 2.], 3., 1.).unwrap(),
        Frame::xy(),
        [0., 0., 4.],
        1e-9,
    )
    .unwrap();
    let a = body.audit().unwrap();
    assert_eq!((a.vertices, a.edges, a.faces), (16, 24, 10));
    assert_eq!(body.faces()[0].loops.len(), 2);
    near(body.volume(), 32. * PI, 1e-12);
    point(body.centroid(), [1., 2., 2.]);
    let display = body.tessellate(0.001, 4096).unwrap();
    assert!(display.mesh.topology().is_closed_manifold());
    assert_eq!(display.mesh.topology().components, 1);
    near(display.mesh.signed_volume(), body.volume(), 0.12);
    let boundary_edges = display.mesh.topology().boundary_edges;
    assert_eq!(boundary_edges, 0);
    let v = display.mesh.vertices().len();
    let f = display.mesh.triangles().len();
    assert_eq!(v as isize - f as isize / 2, 0); // V-E+F=0 for genus-one boundary.
    for i in 6..10 {
        let face = &body.faces()[i];
        let p = face.carrier.evaluate(0.5, 0.4).unwrap();
        let n = scale(
            face.carrier
                .differential(0.5, 0.4)
                .unwrap()
                .normal()
                .unwrap(),
            if face.reversed { -1. } else { 1. },
        );
        assert!(dot(n, [p[0] - 1., p[1] - 2., 0.]) < 0.);
    }
}
#[test]
fn workplane_and_negative_oblique_extrusion_keep_orientation() {
    let frame = Frame::new([10., -2., 3.], [0., 1., 0.], [1., 0., 0.]).unwrap();
    for vector in [[4., 1., 2.], [-4., 1., 2.]] {
        let body = Body::extrude(Profile::disk([0.; 2], 2.).unwrap(), frame, vector, 1e-8).unwrap();
        near(body.volume(), 16. * PI, 1e-11);
        let mesh = body.tessellate(0.01, 1024).unwrap().mesh;
        assert!(mesh.topology().is_closed_manifold());
        assert!(mesh.signed_volume() > 0.);
        near(mesh.signed_volume(), body.volume(), 0.4);
        point(body.centroid(), add([10., -2., 3.], scale(vector, 0.5)));
    }
}
#[test]
fn concave_polygon_retains_planes_and_boundary_tracking() {
    let profile = Profile::polygon(vec![
        [0., 0.],
        [3., 0.],
        [3., 1.],
        [1., 1.],
        [1., 3.],
        [0., 3.],
    ])
    .unwrap();
    near(profile.area(), 5., 1e-12);
    let body = Body::extrude(profile, Frame::xy(), [2., 0., 3.], 1e-10).unwrap();
    near(body.volume(), 15., 1e-12);
    assert_eq!(body.tracking().len(), 6);
    let display = body.tessellate(0.01, 1024).unwrap();
    near(display.mesh.signed_volume(), 15., 1e-11);
    assert!(display.mesh.topology().is_closed_manifold());
    assert_eq!(display.boundary_chord_deviation, 0.);
}
#[test]
fn extrusion_refuses_unresolved_boundaries_and_invalid_profiles() {
    assert!(Profile::annulus([0.; 2], 1., 1.).is_err());
    assert!(Profile::polygon(vec![[0., 0.], [1., 1.], [0., 1.], [1., 0.]]).is_err());
    assert!(Frame::new([0.; 3], [1., 0., 0.], [1., 0., 0.]).is_err());
    let disk = Profile::disk([0.; 2], 1.).unwrap();
    assert!(Body::extrude(disk.clone(), Frame::xy(), [1., 0., 0.], 1e-9).is_err());
    let b = Body::extrude(disk, Frame::xy(), [0., 0., 2.], 1e-9).unwrap();
    assert!(b.tessellate(1e-16, 8).is_err());
    assert!(b.tessellate(f64::NAN, 64).is_err());
}

#[test]
fn loft_refines_mismatched_knots_without_fitting() {
    let a = Curve::circle([0.; 3], 2.).unwrap();
    let b = Curve::circle([0., 0., 4.], 3.)
        .unwrap()
        .insert_knot(0.13)
        .unwrap()
        .insert_knot(0.77)
        .unwrap();
    assert!(modeling_clean::surface::Surface::loft(&[a.clone(), b.clone()]).is_err());
    let result = loft::loft(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(result.report.degree, 2);
    assert_eq!(result.report.spans, 6);
    for i in 0..=100 {
        let t = i as f64 / 100.;
        point(
            result.surface.evaluate(0., t).unwrap(),
            a.evaluate(t).unwrap(),
        );
        point(
            result.surface.evaluate(1., t).unwrap(),
            b.evaluate(t).unwrap(),
        );
    }
}
#[test]
fn loft_elevates_degrees_and_normalizes_authored_domains() {
    let a = Curve::line([0., 0., 0.], [3., 0., 0.]).unwrap();
    let b = Curve::new(
        3,
        vec![2., 2., 2., 2., 5., 5., 5., 5.],
        vec![[0., 0., 2.], [1., 1., 2.], [2., 1., 2.], [3., 0., 2.]],
        vec![1., 0.7, 1.3, 1.],
    )
    .unwrap();
    let result = loft::loft(&[a.clone(), b.clone()]).unwrap();
    assert_eq!(result.report.degree, 3);
    assert_eq!(result.report.source_domains, vec![[0., 1.], [2., 5.]]);
    for i in 0..=100 {
        let t = i as f64 / 100.;
        point(
            result.profiles[0].evaluate(t).unwrap(),
            a.evaluate(t).unwrap(),
        );
        point(
            result.profiles[1].evaluate(t).unwrap(),
            b.evaluate(2. + 3. * t).unwrap(),
        );
        point(
            result.surface.evaluate(1., t).unwrap(),
            b.evaluate(2. + 3. * t).unwrap(),
        );
    }
}
#[test]
fn loft_reports_discontinuous_profile_instead_of_smoothing_it() {
    let c = Curve::new(
        1,
        vec![0., 0., 0.5, 0.5, 1., 1.],
        vec![[0., 0., 0.], [1., 0., 0.], [2., 0., 0.], [3., 0., 0.]],
        vec![1.; 4],
    )
    .unwrap();
    assert!(loft::loft(&[c.clone(), c]).is_err());
}

fn cube() -> Convex {
    Convex::cuboid(
        Bounds::new([-1.; 3], [1.; 3]).unwrap(),
        &Affine::identity(),
        1e-9,
    )
    .unwrap()
}
#[test]
fn rotated_convex_boolean_preserves_planar_geometry() {
    let a = cube();
    let b = a
        .transformed(&Affine::rotation([0., 0., 1.], FRAC_PI_4).unwrap())
        .unwrap();
    let result = a.intersection(&b).unwrap();
    let body = result.body.unwrap();
    assert!(body.mesh().unwrap().topology().is_closed_manifold());
    near(body.volume().unwrap(), 16. * (2f64.sqrt() - 1.), 1e-9);
    point(body.mass_properties().unwrap().1, [0.; 3]);
    assert!(body.faces().len() >= 8);
    assert!(!body.contains([1., 1., 0.]));
    assert!(body.contains([0.; 3]));
}
#[test]
fn clipping_through_existing_vertices_and_edges_closes_caps() {
    let a = Convex::cuboid(
        Bounds::new([0.; 3], [1.; 3]).unwrap(),
        &Affine::identity(),
        1e-10,
    )
    .unwrap();
    let tetra = a
        .clip(Plane::new([1.; 3], 1.).unwrap(), 7)
        .unwrap()
        .unwrap();
    near(tetra.volume().unwrap(), 1. / 6., 1e-11);
    assert!(tetra.mesh().unwrap().topology().is_closed_manifold());
    let half = cube()
        .clip(Plane::new([1., 0., 1.], 0.).unwrap(), 1)
        .unwrap()
        .unwrap();
    near(half.volume().unwrap(), 4., 1e-10);
}
#[test]
fn contact_and_separation_regularize_to_empty_without_placeholder_shells() {
    let a = cube();
    for offset in [[2., 0., 0.], [2., 2., 0.], [2., 2., 2.], [3., 0., 0.]] {
        let b = a
            .transformed(&Affine::translation(offset).unwrap())
            .unwrap();
        assert!(a.intersection(&b).unwrap().body.is_none());
    }
    let identity = a.intersection(&a).unwrap().body.unwrap();
    near(identity.volume().unwrap(), 8., 1e-10);
}
#[test]
fn convex_difference_partition_conserves_volume_and_retains_overlap() {
    let a = cube();
    let b = a
        .transformed(
            &Affine::rotation([0., 0., 1.], 0.3)
                .unwrap()
                .then(&Affine::translation([0.8, 0.2, 0.]).unwrap())
                .unwrap(),
        )
        .unwrap();
    let partition = a.subtract_partition(&b).unwrap();
    let removed = partition.overlap.as_ref().unwrap().volume().unwrap();
    let kept = partition
        .outside
        .iter()
        .map(|c| c.volume().unwrap())
        .sum::<f64>();
    near(kept + removed, 8., 1e-8);
    for piece in &partition.outside {
        assert!(piece.mesh().unwrap().topology().is_closed_manifold());
    }
    for i in 0..partition.outside.len() {
        for j in i + 1..partition.outside.len() {
            assert!(
                partition.outside[i]
                    .intersection(&partition.outside[j])
                    .unwrap()
                    .body
                    .is_none()
            );
        }
    }
}
#[test]
fn convex_validation_refuses_open_inward_and_degenerate_input() {
    let c = cube();
    let mut faces = c
        .faces()
        .iter()
        .map(|f| f.points.clone())
        .collect::<Vec<_>>();
    faces.pop();
    assert!(Convex::new(faces, 1e-9).is_err());
    let mut faces = c
        .faces()
        .iter()
        .map(|f| f.points.clone())
        .collect::<Vec<_>>();
    faces[0].reverse();
    assert!(Convex::new(faces, 1e-9).is_err());
    assert!(Plane::new([0.; 3], 1.).is_err());
}

#[test]
fn arbitrary_convex_prism_uses_the_same_clipping_kernel() {
    let profile = [[0., 0.], [3., 0.], [0., 2.]];
    let frame = Frame::new([2., -1., 4.], [0., 1., 0.], [1., 0., 0.]).unwrap();
    let body = Convex::prism(&profile, frame, [4., 1., 0.], 1e-9).unwrap();
    near(body.volume().unwrap(), 12., 1e-9);
    let kept = body
        .clip(Plane::new([1., 0., 0.], 4.).unwrap(), 0)
        .unwrap()
        .unwrap();
    near(kept.volume().unwrap(), 6., 1e-9);
    assert!(kept.mesh().unwrap().topology().is_closed_manifold());
    assert!(
        Convex::prism(
            &[[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 3.], [0., 3.]],
            Frame::xy(),
            [0., 0., 2.],
            1e-9
        )
        .is_err()
    );
}

#[test]
fn bounded_rotation_corpus_conserves_partition_volume() {
    let a = cube();
    for i in 1..=8 {
        let angle = 0.07 * i as f64;
        let pose = Affine::rotation([1., 2., 3.], angle)
            .unwrap()
            .then(&Affine::translation([0.4, 0.2, -0.3]).unwrap())
            .unwrap();
        let b = a.transformed(&pose).unwrap();
        let ab = a.intersection(&b).unwrap().body.unwrap();
        let ba = b.intersection(&a).unwrap().body.unwrap();
        near(ab.volume().unwrap(), ba.volume().unwrap(), 1e-8);
        let result = a.subtract_partition(&b).unwrap();
        let outside = result
            .outside
            .iter()
            .map(|b| b.volume().unwrap())
            .sum::<f64>();
        near(
            outside + result.overlap.unwrap().volume().unwrap(),
            8.,
            1e-8,
        );
    }
}
