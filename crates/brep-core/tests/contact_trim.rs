use brep_core::{Model, contact_trim, fillet_contact};
use nurbs_core::{offset::fillet_surfaces::FilletOptions, surface::Surface};
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
fn original_root_bands_bound_both_contact_ends_and_all_restricted_lifts() {
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
    assert_eq!(built.status, fillet_contact::Status::VerifiedContacts);
    let initial = &built.contacts.as_ref().unwrap()[0].curve;
    let d = initial.domain();
    let increasing =
        initial.evaluate(d[1]).unwrap().point[1] > initial.evaluate(d[0]).unwrap().point[1];
    let first_y = if increasing { 2. } else { 8. };
    let last_y = if increasing { 8. } else { 2. };
    let (first, f) = cutter(first_y);
    let (last, l) = cutter(last_y);
    let before = [first.clone(), last.clone()];
    let skin = built.fillet.surface.as_ref().unwrap();
    for (side, c) in built.contacts.as_ref().unwrap().iter().enumerate() {
        let original = c.curve.clone();
        let supports = [if side == 0 { &a } else { &b }, skin];
        let out = contact_trim::between_faces(
            c,
            supports,
            [(&first, f, 0), (&last, l, 0)],
            1e-6,
            1e-8,
            100000,
            1000000,
        )
        .unwrap();
        assert!(out.cells <= 100000 && out.exact_work <= 1000000);
        assert!(
            out.coverage
                .iter()
                .all(|r| r.complete && !r.source_is_owned && r.eligible.len() == 1)
        );
        assert!(out.root_position_error_upper.iter().all(|&e| e <= 1e-6));
        assert!((out.endpoints[0][1] - first_y).abs() <= 1e-6);
        assert!((out.endpoints[1][1] - last_y).abs() <= 1e-6);
        assert_eq!(out.contact.curve.control_points[0], out.endpoints[0]);
        assert_eq!(
            *out.contact.curve.control_points.last().unwrap(),
            out.endpoints[1]
        );
        assert!(out.contact.positions.proven);
        assert_eq!(
            out.original_agreement.status,
            nurbs_core::curve_surface_agreement::Status::WithinTolerance
        );
        let g2 = nurbs_core::surface_g2_pcurves::inspect(
            &out.contact.curve,
            [&out.contact.support_pcurve, &out.contact.fillet_pcurve],
            supports,
            [false, false],
            1e-6,
            1e-5,
            1e-4,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(
            g2.geometric.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(
            contact_trim::between_faces(
                c,
                supports,
                [(&last, l, 0), (&first, f, 0)],
                1e-6,
                1e-8,
                100000,
                1000000
            )
            .is_err()
        );
        assert!(
            contact_trim::between_faces(
                c,
                supports,
                [(&first, f, 1), (&last, l, 0)],
                1e-6,
                1e-8,
                100000,
                1000000
            )
            .is_err()
        );
        assert!(
            contact_trim::between_faces(
                c,
                supports,
                [(&first, f, 0), (&last, l, 0)],
                1e-6,
                1e-8,
                1,
                1000000
            )
            .is_err()
        );
        assert_eq!(c.curve, original);
    }
    assert_eq!([first, last], before);
}

fn holed_cutter(y: f64) -> Model {
    use nurbs_core::curve::Curve;
    let polygon = |points: &[[f64; 2]]| {
        (0..points.len())
            .map(|i| {
                Curve::from_polyline(vec![
                    vec![points[i][0], points[i][1], y],
                    vec![
                        points[(i + 1) % points.len()][0],
                        points[(i + 1) % points.len()][1],
                        y,
                    ],
                ])
                .unwrap()
            })
            .collect::<Vec<_>>()
    };
    let loops = vec![
        polygon(&[[-5., -15.], [15., -15.], [15., 5.], [-5., 5.]]),
        polygon(&[[1., -1.], [1., 1.], [2., 1.], [2., -1.]]),
    ];
    let mut model = brep_core::planar_cap::build_sheet(&loops, y, 1e-6, 100000).unwrap();
    let rotate = |p: &mut [f64]| {
        let old = p[1];
        p[1] = p[2];
        p[2] = -old;
    };
    for v in &mut model.vertices {
        rotate(&mut v.point);
    }
    for e in &mut model.edges {
        for p in &mut e.curve.control_points {
            rotate(p);
        }
    }
    for f in &mut model.faces {
        for p in f.surface.control_points.iter_mut().flatten() {
            rotate(p);
        }
    }
    model.rebuild_topology_ids();
    model.validate().unwrap();
    model
}
#[test]
fn end_inside_cutter_hole_never_authorizes_a_contact_restriction() {
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
    let contact = &built.contacts.as_ref().unwrap()[0];
    let original = contact.curve.clone();
    let hole = holed_cutter(2.);
    let before = hole.clone();
    let roots =
        brep_core::face_crossings::inspect_curve(&contact.curve, &hole, 0, 1e-8, 100000).unwrap();
    assert!(roots.complete && !roots.source_is_owned);
    assert_eq!(roots.roots.len(), 1);
    assert!(roots.eligible.is_empty());
    assert_eq!(
        roots.roots[0].location,
        nurbs_core::trim_domain::Location::Outside
    );
    let (last, l) = cutter(8.);
    assert!(
        contact_trim::between_faces(
            contact,
            [&a, built.fillet.surface.as_ref().unwrap()],
            [(&hole, 0, 0), (&last, l, 0)],
            1e-6,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!(contact.curve, original);
    assert_eq!(hole, before);
}

#[test]
fn rational_cylinder_contact_restriction_preserves_whole_interval_g2() {
    use nurbs_core::curve::Curve;
    let a = plane([1., 0., 0.], [0., 1., 0.]);
    let profile = Curve::from_polyline(vec![vec![3., 0., 4.], vec![3., 8., 4.]]).unwrap();
    let c = nurbs_core::surface::revolve(&profile, [0., 0., 4.], [0., 1., 0.], 360.).unwrap();
    let e = c.evaluate(0.37, 0.41).unwrap();
    let n = e.unit_normal().unwrap();
    let options = FilletOptions {
        side_b: (n[0] * e.point[0] + n[2] * (e.point[2] - 4.)).signum(),
        max_sections: 5,
        ..Default::default()
    };
    let built = fillet_contact::build_g2([&a, &c], 1., &options, 1e-6, 100000, 1000000).unwrap();
    assert_eq!(built.status, fillet_contact::Status::VerifiedContacts);
    let contact = &built.contacts.as_ref().unwrap()[1];
    let before = contact.curve.clone();
    let d = contact.curve.domain();
    let increasing = contact.curve.evaluate(d[1]).unwrap().point[1]
        > contact.curve.evaluate(d[0]).unwrap().point[1];
    let ys = if increasing { [2., 6.] } else { [6., 2.] };
    let (first, f) = cutter(ys[0]);
    let (last, l) = cutter(ys[1]);
    let supports = [&c, built.fillet.surface.as_ref().unwrap()];
    let out = contact_trim::between_faces(
        contact,
        supports,
        [(&first, f, 0), (&last, l, 0)],
        1e-6,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    assert!(
        out.contact.positions.proven && out.root_position_error_upper.iter().all(|&e| e <= 1e-6)
    );
    for k in 0..2 {
        assert!((out.endpoints[k][1] - ys[k]).abs() <= 1e-6);
    }
    let proof = nurbs_core::surface_g2_pcurves::inspect(
        &out.contact.curve,
        [&out.contact.support_pcurve, &out.contact.fillet_pcurve],
        supports,
        [false, false],
        1e-6,
        1e-5,
        1e-4,
        100000,
        1000000,
    )
    .unwrap();
    assert_eq!(
        proof.geometric.decision,
        nurbs_core::surface_join::Decision::WithinTolerance
    );
    assert_eq!(contact.curve, before);
}
