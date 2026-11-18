use super::*;
fn q(a: i64, b: i64) -> Q {
    Q::new(a.into(), b.into())
}
fn ratio(a: i64, b: i64) -> ExactRatio {
    roots::ratio(&q(a, b))
}
fn window() -> [[ExactRatio; 2]; 2] {
    [[ratio(3, 16), ratio(5, 16)], [ratio(3, 16), ratio(5, 16)]]
}
fn top(model: &Model) -> usize {
    model
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 1.)
        })
        .unwrap()
}
fn floor(s: &Surface) -> Surface {
    assert_eq!((s.degree_u, s.degree_v), (1, 1));
    let a = &s.control_points[0][0];
    let b = &s.control_points[1][0];
    let c = &s.control_points[0][1];
    let mut control = vec![vec![vec![0.; 3]; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let u = i as f64 / 2.;
            let v = j as f64 / 2.;
            control[i][j] = vec![
                a[0] + (b[0] - a[0]) * u + (c[0] - a[0]) * v,
                a[1] + (b[1] - a[1]) * u + (c[1] - a[1]) * v,
                0.75 + 1. / 256. - u / 32. + if i == 2 { 1. / 16. } else { 0. },
            ];
        }
    }
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: control,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn squared_distance() -> Bi {
    vec![
        vec![q(1, 8), q(-1, 2), Q::one()],
        vec![q(-1, 2)],
        vec![Q::one()],
    ]
}
fn circle() -> DomainPolynomial {
    encode(
        &sum(
            &vec![vec![q(1, 1024)]],
            &scale(&squared_distance(), &-Q::one()),
        )
        .unwrap(),
    )
}
fn pocket(m: &Model, depth: u8) -> AlgebraicModelPocket {
    let f = top(m);
    certify_model_pocket(
        m,
        f,
        &floor(&m.faces[f].surface),
        PocketRegion {
            domain: &circle(),
            factors: &[],
            window: window(),
        },
        None,
        depth,
    )
    .unwrap()
}
#[test]
fn existing_model_is_cut_and_actual_side_walls_are_sewn() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let saved = model.clone();
    let result = pocket(&model, 0);
    assert_eq!(model, saved);
    assert_eq!(result.source_model(), &saved);
    assert!(result
        .evaluate_retained_rational(&ratio(1, 4), &ratio(1, 4))
        .is_err());
    assert_eq!(
        result
            .evaluate_floor_rational(0, &ratio(1, 4), &ratio(1, 4))
            .unwrap()[2],
        ratio(3, 4)
    );
    assert_eq!(
        result
            .evaluate_retained_rational(&ratio(3, 16), &ratio(1, 4))
            .unwrap()[2],
        ratio(1, 1)
    );
    let u = ratio(1, 4);
    let v = ratio(7, 32);
    let mut wall = false;
    for face in result.faces() {
        if let PocketFaceKind::Wall { domain_edge } = face.kind {
            if let Ok(p) = result.evaluate_wall_rational(domain_edge, &u, &v, &ratio(1, 2)) {
                assert_eq!(p[2], ratio(7, 8));
                wall = true;
            }
        }
    }
    assert!(wall);
    assert_eq!(
        result.evaluate_retained_rational(&u, &v).unwrap()[2],
        ratio(1, 1)
    );
    verify_model_pocket(&model, result.floor_carrier(), result.certificate()).unwrap();
    let mut forged = result.certificate().clone();
    let face = forged
        .faces
        .iter_mut()
        .find(|f| matches!(f.kind, PocketFaceKind::Wall { .. }))
        .unwrap();
    face.loops[0][0].positive ^= true;
    assert!(verify_model_pocket(&model, result.floor_carrier(), &forged).is_err());
    let mut changed = result.floor_carrier().clone();
    changed.control_points[1][1][2] += 0.125;
    assert!(verify_model_pocket(&model, &changed, result.certificate()).is_err());
}
#[test]
fn nonconvex_existing_container_is_modified_without_convex_support_assumptions() {
    let profile = [[0., 0.], [4., 0.], [4., 1.5], [2., 1.5], [2., 4.], [0., 4.]];
    let model = crate::operations::extrude_polygon(&profile, 0., 1.).unwrap();
    let result = pocket(&model, 0);
    assert!(result
        .faces()
        .iter()
        .any(|f| matches!(f.kind, PocketFaceKind::Wall { .. })));
    verify_model_pocket(&model, result.floor_carrier(), result.certificate()).unwrap();
}
#[test]
fn nongraph_curved_cylinder_carriers_are_preserved() {
    let cylinder = crate::frustum(1., 1., 1.).unwrap();
    let result = pocket(&cylinder, 1);
    let nongraph = cylinder
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .map(|(id, _)| id)
        .collect::<Vec<_>>();
    assert_eq!(nongraph.len(), 4);
    assert!(nongraph.iter().all(|id| result
        .faces()
        .iter()
        .any(|f| matches!(f.kind,PocketFaceKind::Source{face,..} if face==*id))));
    verify_model_pocket(&cylinder, result.floor_carrier(), result.certificate()).unwrap();
}
#[test]
fn annular_removal_keeps_an_original_surface_island() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let f = top(&model);
    let cap = floor(&model.faces[f].surface);
    let s = squared_distance();
    let a = sum(&vec![vec![q(1, 1024)]], &scale(&s, &-Q::one())).unwrap();
    let b = sum(&s, &vec![vec![q(-1, 4096)]]).unwrap();
    let d = product(&a, &b, &mut Work::default()).unwrap();
    let result = certify_model_pocket(
        &model,
        f,
        &cap,
        PocketRegion {
            domain: &encode(&d),
            factors: &[encode(&a), encode(&b)],
            window: window(),
        },
        None,
        0,
    )
    .unwrap();
    assert_eq!(
        result
            .faces()
            .iter()
            .filter(|f| matches!(f.kind, PocketFaceKind::Remainder { .. }))
            .count(),
        2
    );
    assert!(result
        .evaluate_floor_rational(0, &ratio(1, 4), &ratio(1, 4))
        .is_err());
    assert_eq!(
        result
            .evaluate_retained_rational(&ratio(1, 4), &ratio(1, 4))
            .unwrap()[2],
        ratio(1, 1)
    );
    verify_model_pocket(&model, &cap, result.certificate()).unwrap();
    // Negative-sign CAD cells are also valid selections inside the window;
    // a sign label must not forbid an explicitly selected bounded cell.
    let negative = result
        .certificate()
        .partition
        .faces
        .iter()
        .enumerate()
        .filter(|(_, c)| c.signs[0] == -1 && c.signs[1] == 1 && c.signs[2] == 1)
        .map(|(i, _)| i as u32)
        .collect::<Vec<_>>();
    let one = negative
        .iter()
        .find(|id| {
            let c = &result.certificate().partition.faces[**id as usize];
            let slab = &result.certificate().partition.arrangement.slabs[c.slab];
            roots::decode(&slab.sample).unwrap() > q(15, 64)
                && roots::decode(&slab.sample).unwrap() < q(17, 64)
        })
        .copied()
        .unwrap();
    let partial = certify_model_pocket(
        &model,
        f,
        &cap,
        PocketRegion {
            domain: &encode(&d),
            factors: &[encode(&a), encode(&b)],
            window: window(),
        },
        Some(&[one]),
        0,
    )
    .unwrap();
    assert_eq!(partial.certificate().selected, vec![one]);
    assert!(partial.faces().iter().any(|face|matches!(face.kind,PocketFaceKind::Wall{domain_edge} if matches!(partial.certificate().partition.edges[domain_edge as usize].kind,domains::DomainEdgeKind::Fiber{..}))));
    verify_model_pocket(&model, &cap, partial.certificate()).unwrap();
}
#[test]
fn independent_cell_cuts_and_excessive_depth_do_not_make_unproved_bodies() {
    let model = crate::cuboid([0.; 3], [1.; 3]).unwrap();
    let f = top(&model);
    let cap = floor(&model.faces[f].surface);
    // A simple crossing has zero gradient at its node. A selected CAD quadrant
    // is still a valid cornered pocket, certified by exact cell incidence.
    let d = vec![vec![q(1, 16), q(-1, 4)], vec![q(-1, 4), Q::one()]];
    let factors = [
        encode(&vec![vec![q(-1, 4), Q::one()]]),
        encode(&vec![vec![q(-1, 4)], vec![Q::one()]]),
    ];
    let reference = certify_model_pocket(
        &model,
        f,
        &cap,
        PocketRegion {
            domain: &encode(&d),
            factors: &factors,
            window: window(),
        },
        Some(&[]),
        0,
    );
    assert!(reference.is_err());
    let region = PocketRegion { domain: &encode(&d), factors: &factors, window: window() };
    let all = certify_model_pocket(&model, f, &cap, region, None, 0);
    assert!(
        all.is_err(),
        "diagonally touching selected quadrants must not form a manifold pocket"
    );
    let guard = encode(&vec![vec![q(-15, 256), q(1, 2), -Q::one()]]);
    let guard_v = encode(&vec![vec![q(-15, 256)], vec![q(1, 2)], vec![-Q::one()]]);
    let partition = domains::partition_cut_factors(
        &[encode(&d), guard.clone(), guard_v.clone()],
        &[factors[0].clone(), factors[1].clone(), guard, guard_v],
    )
    .unwrap();
    let cell = partition
        .faces
        .iter()
        .position(|f| f.signs == vec![1, 1, 1])
        .unwrap() as u32;
    let quadrant = certify_model_pocket(
        &model,
        f,
        &cap,
        PocketRegion {
            domain: &encode(&d),
            factors: &factors,
            window: window(),
        },
        Some(&[cell]),
        0,
    )
    .unwrap();
    assert_eq!(quadrant.certificate().selected, vec![cell]);
    verify_model_pocket(&model, &cap, quadrant.certificate()).unwrap();
    let mut too_deep = cap.clone();
    for row in &mut too_deep.control_points {
        for p in row {
            p[2] -= 2.;
        }
    }
    let region = PocketRegion { domain: &circle(), factors: &[], window: window() };
    assert!(certify_model_pocket(&model, f, &too_deep, region, None, 0).is_err());
    let mut invalid = window();
    invalid[0] = [ratio(1, 2), ratio(9, 16)];
    let region = PocketRegion { domain: &circle(), factors: &[], window: invalid };
    assert!(certify_model_pocket(&model, f, &cap, region, None, 0).is_err());
}
