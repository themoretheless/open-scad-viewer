use super::*;
fn q(a: i64, b: i64) -> Q {
    Q::new(a.into(), b.into())
}
fn ratio(a: i64, b: i64) -> ExactRatio {
    roots::ratio(&q(a, b))
}
fn native(domain: DomainPolynomial) -> AlgebraicModelPocket {
    let m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let f = 1;
    let mut cp = vec![vec![vec![0.; 3]; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let u = i as f64 / 2.;
            cp[i][j] = vec![
                u,
                j as f64 / 2.,
                0.75 + 1. / 256. - u / 32. + if i == 2 { 1. / 16. } else { 0. },
            ];
        }
    }
    let floor = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: cp,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    certify_model_pocket(
        &m,
        f,
        &floor,
        &domain,
        &[],
        [[ratio(3, 16), ratio(5, 16)], [ratio(3, 16), ratio(5, 16)]],
        None,
        0,
    )
    .unwrap()
}
fn circle() -> DomainPolynomial {
    encode(&vec![
        vec![q(1, 1024) - q(1, 8), q(1, 2), -Q::one()],
        vec![q(1, 2)],
        vec![-Q::one()],
    ])
}
#[test]
fn straight_algebraic_pocket_becomes_an_ordinary_model() {
    let p = native(vec![vec![ratio(1, 1)]]);
    let r = certify_pocket_boolean_model(&p, "difference").unwrap();
    r.model().validate_continuous(8192).unwrap();
    assert!(r.model().faces.len() > p.original.faces.len());
    assert_eq!(r.certificate.maximum_boundary_error_mm, ratio(0, 1));
    verify_pocket_boolean_model(&p.original, &p.floor, r.model(), r.certificate()).unwrap();
}
#[test]
fn circle_has_exact_rational_arcs_curved_floor_and_nongraph_nurbs_walls() {
    let p = native(circle());
    let r = certify_pocket_boolean_model(&p, "difference").unwrap();
    r.model().validate_continuous(16384).unwrap();
    assert!(
        r.model()
            .faces
            .iter()
            .any(|f| f.surface.degree_u == 4 && f.surface.degree_v == 1)
    );
    assert!(r.model().edges.iter().any(|e| e.curve.degree == 4));
    let restored: Model = value_codec::from_value(r.model().to_value()).unwrap();
    assert_eq!(&restored, r.model());
    restored.validate_continuous(16384).unwrap();
    assert_eq!(r.certificate.maximum_boundary_error_mm, ratio(0, 1));
    verify_pocket_boolean_model(&p.original, &p.floor, r.model(), r.certificate()).unwrap();
    let mut bad = r.certificate().clone();
    bad.arcs[0].curve.control_points[1][1] += 1. / 1024.;
    assert!(verify_pocket_boolean_model(&p.original, &p.floor, r.model(), &bad).is_err());
}
#[test]
fn all_four_ordinary_model_booleans_use_the_proved_material_subset() {
    let p = native(circle());
    let difference = certify_pocket_boolean_model(&p, "difference").unwrap();
    let xor = certify_pocket_boolean_model(&p, "xor").unwrap();
    assert_eq!(xor.model(), difference.model());
    let union = certify_pocket_boolean_model(&p, "union").unwrap();
    assert_eq!(union.model(), &p.original);
    let intersection = certify_pocket_boolean_model(&p, "intersection").unwrap();
    intersection.model().validate_continuous(16384).unwrap();
    assert_eq!(intersection.model().bodies.len(), 1);
    assert!(intersection.model().faces.len() < difference.model().faces.len());
    verify_pocket_boolean_model(
        &p.original,
        &p.floor,
        intersection.model(),
        intersection.certificate(),
    )
    .unwrap();
}
fn request(face: usize, floor: Surface) -> ModelPocketRequest {
    ModelPocketRequest {
        source_face: face,
        floor,
        domain: circle(),
        factors: vec![],
        window: [[ratio(3, 16), ratio(5, 16)], [ratio(3, 16), ratio(5, 16)]],
        selected: None,
        depth: 1,
    }
}
#[test]
fn folded_xy_edited_and_floor_nurbs_return_a_real_ordinary_model() {
    let mut source = crate::cuboid([-1., 0., 0.], [1., 1., 1.]).unwrap();
    let k = 1. / 8.;
    for face in &mut source.faces {
        let old = face.surface.control_points.clone();
        let a = &old[0][0];
        let du: Vec<_> = old[1][0].iter().zip(a).map(|(x, a)| x - a).collect();
        let dv: Vec<_> = old[0][1].iter().zip(a).map(|(x, a)| x - a).collect();
        face.surface.degree_u = 2;
        face.surface.degree_v = 2;
        face.surface.knots_u = vec![0., 0., 0., 1., 1., 1.];
        face.surface.knots_v = face.surface.knots_u.clone();
        face.surface.weights = vec![vec![1.; 3]; 3];
        face.surface.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let u = i as f64 / 2.;
                        let v = j as f64 / 2.;
                        let mut p: Vec<_> = (0..3).map(|c| a[c] + du[c] * u + dv[c] * v).collect();
                        p[2] += k
                            * (a[0] * a[0]
                                + 2. * a[0] * (du[0] * u + dv[0] * v)
                                + du[0] * du[0] * if i == 2 { 1. } else { 0. }
                                + dv[0] * dv[0] * if j == 2 { 1. } else { 0. }
                                + 2. * du[0] * dv[0] * u * v);
                        p
                    })
                    .collect()
            })
            .collect();
    }
    for e in &mut source.edges {
        let a = e.curve.control_points[0].clone();
        let b = e.curve.control_points[1].clone();
        let mut first = a.clone();
        first[2] += k * a[0] * a[0];
        let mut last = b.clone();
        last[2] += k * b[0] * b[0];
        let mut middle: Vec<_> = a.iter().zip(&b).map(|(a, b)| (a + b) / 2.).collect();
        middle[2] += k * a[0] * b[0];
        e.curve.degree = 2;
        e.curve.knots = vec![0., 0., 0., 1., 1., 1.];
        e.curve.control_points = vec![first, middle, last];
        e.curve.weights = vec![1.; 3];
    }
    for v in &mut source.vertices {
        v.point[2] += k * v.point[0] * v.point[0];
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let mut floor = source.faces[1].surface.clone();
    for i in 0..3 {
        for j in 0..3 {
            floor.control_points[i][j][2] = if i == 1 {
                0.75 - 1. / 16.
            } else {
                0.75 + 1. / 16.
            };
        }
    }
    let folded = PocketCoordinateFrame { axes: [3, 2, 1] };
    let source = folded.transform_model(&source).unwrap();
    let floor = folded.transform_surface(&floor).unwrap();
    assert_eq!(
        source.faces[1].surface.control_points[0][0][0],
        source.faces[1].surface.control_points[2][0][0]
    );
    assert_ne!(
        source.faces[1].surface.control_points[0][0][2],
        source.faces[1].surface.control_points[2][0][2]
    );
    let r =
        certify_oriented_pocket_boolean_model(&source, &request(1, floor.clone()), "difference")
            .unwrap();
    assert_eq!(r.certificate().frame.axes[2].unsigned_abs(), 1);
    r.model().validate_continuous(32768).unwrap();
    verify_framed_pocket_boolean_model(&source, &floor, r.model(), r.certificate()).unwrap();
}
#[test]
fn sequential_cuts_on_different_original_faces_keep_ordinary_model_sources() {
    let source = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut top = source.faces[1].surface.clone();
    for row in &mut top.control_points {
        for p in row {
            p[2] = 0.75;
        }
    }
    let mut front = source.faces[2].surface.clone();
    for row in &mut front.control_points {
        for p in row {
            p[1] = 0.25;
        }
    }
    let floors = [top.clone(), front.clone()];
    let mut requests = [request(1, top), request(2, front)];
    for request in &mut requests {
        request.domain = vec![vec![ratio(1, 1)]];
    }
    let r = certify_model_pocket_batch(&source, &requests).unwrap();
    assert_eq!(r.certificate().steps.len(), 2);
    r.model().validate_continuous(32768).unwrap();
    verify_model_pocket_batch(&source, &floors, r.model(), r.certificate()).unwrap();
}

#[test]
fn circular_pocket_can_be_the_source_of_a_second_face_edit() {
    let source = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut top = source.faces[1].surface.clone();
    for row in &mut top.control_points {
        for p in row {
            p[2] = 0.75;
        }
    }
    let mut front = source.faces[2].surface.clone();
    for row in &mut front.control_points {
        for p in row {
            p[1] = 0.25;
        }
    }
    let floors = [top.clone(), front.clone()];
    let mut second = request(2, front);
    second.depth = 2;
    let r = certify_model_pocket_batch(&source, &[request(1, top), second]).unwrap();
    r.model().validate_continuous(32768).unwrap();
    verify_model_pocket_batch(&source, &floors, r.model(), r.certificate()).unwrap();
}
