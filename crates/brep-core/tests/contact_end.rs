use brep_core::{Model, contact_end, contact_trim, fillet_contact};
use nurbs_core::{offset::fillet_surfaces::FilletOptions, surface::Surface};
#[path = "../examples/support/curved_contact_body_fixture.rs"]
mod curved_fixture;

#[test]
fn variable_curved_support_contacts_bind_to_original_body_edges_and_close_both_patch_ends() {
    let f = curved_fixture::build().unwrap();
    f.source.validate().unwrap();
    f.patch.validate().unwrap();
    assert_eq!(f.patch.faces.len(), 1);
    assert_eq!(f.patch.edges.len(), 4);
    assert_eq!(f.patch.vertices.len(), 4);
    assert_eq!(f.source.faces.len(), f.original.faces.len());
    for (before, after) in f.original.faces.iter().zip(&f.source.faces) {
        assert_eq!(before.surface, after.surface);
    }
    for (i, &v) in f.mapping.iter().enumerate() {
        assert_eq!(f.patch.vertices[i].point, f.source.vertices[v].point);
    }
    for e in &f.patch.edges {
        assert_eq!(e.curve.control_points[0], f.patch.vertices[e.vertices[0]].point);
        assert_eq!(*e.curve.control_points.last().unwrap(), f.patch.vertices[e.vertices[1]].point);
    }
    assert!(f.law_domain[0] < f.law_domain[1]);
    assert!(f.keeps.iter().all(|&v| v < f.source.vertices.len()));
}
fn plane(u: [f64; 3], v: [f64; 3]) -> Surface {
    let p = |a: f64, b: f64| (0..3).map(|k| u[k] * a + v[k] * b).collect::<Vec<_>>();
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![vec![p(0., 0.), p(0., 10.)], vec![p(10., 0.), p(10., 10.)]],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cutter(y: f64) -> (Model, usize) {
    let m = brep_core::cuboid([-5., y, -5.], [15., y + 0.5, 15.]).unwrap();
    let f = m
        .faces
        .iter()
        .position(|f| f.surface.control_points.iter().flatten().all(|p| p[1] == y))
        .unwrap();
    (m, f)
}
#[test]
fn end_boundaries_retain_both_contact_vertices_and_lift_to_skin_and_original_end_faces() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = fillet_contact::build_g2(
        [&a, &b],
        1.5,
        &FilletOptions {
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let contacts = built.contacts.as_ref().unwrap();
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let increasing = contacts[0].curve.evaluate(d[1]).unwrap().point[1]
        > contacts[0].curve.evaluate(d[0]).unwrap().point[1];
    let ys = if increasing { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let first_cut = contact_trim::between_faces(
        &contacts[0],
        [&a, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let second_cut = contact_trim::between_faces(
        &contacts[1],
        [&b, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let before = [first.clone(), last.clone()];
    for (end, (model, face)) in [(&first, f), (&last, l)].into_iter().enumerate() {
        let out = contact_end::connect(
            [&first_cut, &second_cut],
            end,
            skin,
            model,
            face,
            1e-6,
            1e-8,
            100000,
            1000000,
        )
        .unwrap();
        assert!(out.positions.proven && out.cells <= 100000 && out.exact_work <= 1000000);
        assert_eq!(
            out.endpoints,
            [first_cut.endpoints[end], second_cut.endpoints[end]]
        );
        assert_eq!(out.curve.control_points[0], out.endpoints[0]);
        assert_eq!(*out.curve.control_points.last().unwrap(), out.endpoints[1]);
        let verification = nurbs_core::surface_g2_pcurves::verify_positions(
            &out.curve,
            [&out.skin_pcurve, &out.end_face_pcurve],
            [skin, &model.faces[face].surface],
            [false, false],
            1e-6,
            100000,
            1000000,
        )
        .unwrap();
        assert!(verification.proven);
        assert!(
            contact_end::connect(
                [&first_cut, &first_cut],
                end,
                skin,
                model,
                face,
                1e-6,
                1e-8,
                100000,
                1000000
            )
            .is_err()
        );
        assert!(
            contact_end::connect(
                [&first_cut, &second_cut],
                end,
                skin,
                model,
                face,
                1e-6,
                1e-8,
                1,
                1000000
            )
            .is_err()
        );
    }
    assert!(
        contact_end::connect(
            [&first_cut, &second_cut],
            0,
            skin,
            &last,
            l,
            1e-6,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    let patch = contact_end::patch(
        [&first_cut, &second_cut],
        skin,
        [(&first, f), (&last, l)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(patch.model.vertices.len(), 4);
    assert_eq!(patch.model.edges.len(), 4);
    assert!(patch.model.bodies.is_empty() && !patch.model.shells[0].closed);
    assert_eq!(patch.model.faces[0].surface, *skin);
    assert!(patch.model.persistent_naming_complete());
    for vertex in 0..4 {
        assert_eq!(
            patch
                .model
                .edges
                .iter()
                .filter(|e| e.vertices.contains(&vertex))
                .count(),
            2
        );
    }
    assert!(patch.cells <= 100000 && patch.exact_work <= 1000000);
    assert_eq!([first, last], before);
}

fn cap_with_hole(y: f64, center: [f64; 2]) -> Model {
    use nurbs_core::curve::Curve;
    let polygon = |ps: &[[f64; 2]]| {
        (0..ps.len())
            .map(|i| {
                Curve::from_polyline(vec![
                    vec![ps[i][0], ps[i][1], y],
                    vec![ps[(i + 1) % ps.len()][0], ps[(i + 1) % ps.len()][1], y],
                ])
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let x = center[0];
    let v = -center[1];
    let h = 0.05;
    let loops = vec![
        polygon(&[[-5., -15.], [15., -15.], [15., 5.], [-5., 5.]]),
        polygon(&[
            [x - h, v - h],
            [x - h, v + h],
            [x + h, v + h],
            [x + h, v - h],
        ]),
    ];
    let mut m = brep_core::planar_cap::build_sheet(&loops, y, 1e-6, 100000).unwrap();
    let rotate = |p: &mut [f64]| {
        let old = p[1];
        p[1] = p[2];
        p[2] = -old;
    };
    for p in &mut m.vertices {
        rotate(&mut p.point);
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            rotate(p);
        }
    }
    for f in &mut m.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            rotate(p);
        }
    }
    m.rebuild_topology_ids();
    m.validate().unwrap();
    m
}
#[test]
fn whole_end_curve_crossing_a_hole_refuses_despite_valid_endpoints() {
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let b = plane([0., 1., 0.], [0., 0., 1.]);
    let built = fillet_contact::build_g2(
        [&a, &b],
        1.5,
        &FilletOptions {
            max_sections: 5,
            ..Default::default()
        },
        1e-6,
        100000,
        1000000,
    )
    .unwrap();
    let contacts = built.contacts.as_ref().unwrap();
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let up = contacts[0].curve.evaluate(d[1]).unwrap().point[1]
        > contacts[0].curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 8.] } else { [8., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let c0 = contact_trim::between_faces(
        &contacts[0],
        [&a, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let c1 = contact_trim::between_faces(
        &contacts[1],
        [&b, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let end =
        contact_end::connect([&c0, &c1], 0, skin, &first, f, 1e-6, 1e-8, 100000, 1000000).unwrap();
    let d = end.curve.domain();
    let p = end.curve.evaluate((d[0] + d[1]) * 0.5).unwrap().point;
    let hole = cap_with_hole(ys[0], [p[0], p[2]]);
    let before = hole.clone();
    let c0 = contact_trim::between_faces(
        &contacts[0],
        [&a, skin],
        [(&hole, 0, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let c1 = contact_trim::between_faces(
        &contacts[1],
        [&b, skin],
        [(&hole, 0, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(c0.coverage[0].eligible.len() == 1 && c1.coverage[0].eligible.len() == 1);
    assert!(
        contact_end::connect([&c0, &c1], 0, skin, &hole, 0, 1e-6, 1e-8, 100000, 1000000).is_err()
    );
    assert_eq!(hole, before);
}

#[test]
fn rational_cylinder_transition_has_both_end_boundaries_and_four_shared_corner_ids() {
    use nurbs_core::curve::Curve;
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let profile = Curve::from_polyline(vec![vec![3., 0., 4.], vec![3., 8., 4.]]).unwrap();
    let cylinder =
        nurbs_core::surface::revolve(&profile, [0., 0., 4.], [0., 1., 0.], 360.).unwrap();
    let e = cylinder.evaluate(0.37, 0.41).unwrap();
    let n = e.unit_normal().unwrap();
    let options = FilletOptions {
        side_b: (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum(),
        max_sections: 5,
        ..Default::default()
    };
    let built =
        fillet_contact::build_g2([&a, &cylinder], 1., &options, 1e-6, 100000, 1000000).unwrap();
    let contacts = built.contacts.as_ref().unwrap();
    let skin = built.fillet.surface.as_ref().unwrap();
    let d = contacts[0].curve.domain();
    let up = contacts[0].curve.evaluate(d[1]).unwrap().point[1]
        > contacts[0].curve.evaluate(d[0]).unwrap().point[1];
    let ys = if up { [2., 6.] } else { [6., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let c0 = contact_trim::between_faces(
        &contacts[0],
        [&a, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let c1 = contact_trim::between_faces(
        &contacts[1],
        [&cylinder, skin],
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    let patch = contact_end::patch(
        [&c0, &c1],
        skin,
        [(&first, f), (&last, l)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(patch.ends.iter().all(|end| end.positions.proven));
    assert!(patch.cells <= 100000 && patch.exact_work <= 1000000);
    assert_eq!(patch.model.vertices.len(), 4);
    assert_eq!(patch.model.edges.len(), 4);
    assert_eq!(patch.model.faces[0].surface, *skin);
    assert!(patch.model.bodies.is_empty() && !patch.model.shells[0].closed);
    patch.model.validate().unwrap();
}
