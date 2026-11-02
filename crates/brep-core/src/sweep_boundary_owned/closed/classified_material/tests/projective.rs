use super::*;
fn ring(center: [f64; 2], scale: f64, reverse: bool) -> (Vec<Curve>, Vec<Curve>) {
    let r = 1. / 1024.;
    let e = 1. / 32768.;
    let signs = [[-1., -1.], [1., -1.], [1., 1.], [0., 0.], [-1., -1.]];
    let points: Vec<[f64; 2]> = signs
        .map(|p| std::array::from_fn(|k| center[k] + scale * p[k]))
        .to_vec();
    let xyz = |p: [f64; 2]| vec![-2. * r * p[0], -r * p[1], 9. * r * p[0] + e * p[0] * p[1]];
    let uv = |p: [f64; 2]| vec![1. + 4. * p[0], 9. + 2. * p[1]];
    let mut profiles = Vec::new();
    let mut trims = Vec::new();
    for leg in points.windows(2) {
        let (a, b) = (leg[0], leg[1]);
        let middle = vec![
            -r * (a[0] + b[0]),
            -r * (a[1] + b[1]) * 0.5,
            9. * r * (a[0] + b[0]) * 0.5 + e * (a[0] * b[1] + b[0] * a[1]) * 0.5,
        ];
        profiles.push(Curve {
            degree: 2,
            knots: vec![3., 3., 3., 7., 7., 7.],
            control_points: vec![xyz(a), middle, xyz(b)],
            weights: vec![1.; 3],
            periodic: false,
        });
        trims.push(Curve {
            degree: 1,
            knots: vec![13., 13., 19., 19.],
            control_points: vec![uv(a), uv(b)],
            weights: vec![1., 2.],
            periodic: false,
        });
    }
    if reverse {
        profiles = profiles
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        trims = trims
            .into_iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
    }
    (profiles, trims)
}
#[test]
fn curved_nonplanar_projective_images_bind_g2_material_joins_and_a_hole() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (_, mut cap, _) = source::material();
    for (i, row) in cap.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] += (if i == j { 1. } else { -1. }) / 32768.;
        }
    }
    let (mut profiles, outer) = ring([0., 0.], 1., false);
    let (inner, hole) = ring([0.5, -0.5], 0.125, true);
    profiles.extend(inner);
    assert_ne!(
        profiles[2].control_points[1][2],
        (profiles[2].control_points[0][2] + profiles[2].control_points[2][2]) * 0.5
    );
    let material = LoftCap {
        surface: cap,
        trims: vec![outer, hole],
    };
    let before = (profiles.clone(), material.clone(), path.clone());
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let classes = [
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ];
    let strict = super::super::super::prepare_closed_material(
        &sweep,
        &[4, 4],
        &material,
        1e-8,
        OriginalClosedLimits {
            source_cells: 100000,
            source_products: 1000000,
            source_exact_work: 1000000,
            material: limits().material,
            ideal: IdealLimits {
                cells: 0,
                exact_work: 0,
                projection: projection(),
            },
            wall_faces: 0,
        },
    )
    .unwrap();
    assert!(matches!(
        strict,
        super::super::super::PreparedMaterialAttempt::Refused(_)
    ));
    let body = match certify_original_classified_closed_material(
        &sweep,
        &[4, 4],
        &material,
        &classes,
        projection(),
        1e-8,
        limits(),
    )
    .unwrap()
    {
        OriginalClassifiedClosedMaterialAttempt::Certified(b) => b,
        OriginalClassifiedClosedMaterialAttempt::Refused(r) => {
            panic!("curved image material: {:?} {:?}", r.reason, r.volume)
        }
    };
    assert_eq!(body.line_image_spans(), 4);
    assert_eq!(body.projective_image_spans(), 4);
    assert_eq!(body.bindings().len(), 8);
    for (i, binding) in body.bindings().iter().enumerate() {
        assert_eq!(binding.ring(), i / 4);
        assert_eq!(binding.profile(), (i, 0));
        assert_eq!(binding.trim(), (i % 4, 0));
    }
    assert_eq!(
        body.bindings()
            .iter()
            .filter(|b| b.kind() == MaterialSpanBindingKind::OrientedProjectiveImage)
            .count(),
        4
    );
    assert_eq!(body.material_report().bound_spans, 8);
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.closed_seam_order(), 2);
    assert_eq!(
        body.volume()
            .report()
            .profiles
            .as_ref()
            .unwrap()
            .smooth_joins,
        2
    );
    assert_eq!(
        body.material_report().trim.as_ref().unwrap().winding,
        vec![Some(1), Some(-1)]
    );
    assert!(std::ptr::eq(body.material(), &material));
    let v = &body.volume().report().charts.volume;
    assert!(v.embedding_certified && v.regularity_certified && v.parameter_topology_preserved);
    let work = body.work();
    let native = body.volume().report();
    let exact = OriginalClassifiedClosedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: work.cells,
                exact_work: work.exact_work,
                products: work.products,
                ..limits().volume.charts
            },
            steps: native.profiles.as_ref().unwrap().steps,
        },
        ..limits()
    };
    assert!(matches!(
        certify_original_classified_closed_material(
            &sweep,
            &[4, 4],
            &material,
            &classes,
            projection(),
            1e-8,
            exact
        )
        .unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Certified(_)
    ));
    let mut empty = exact;
    empty.volume.charts.exact_work = 0;
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify_original_classified_closed_material(
            &sweep,
            &[4, 4],
            &material,
            &classes,
            projection(),
            1e-8,
            empty,
        )
        .unwrap()
    else {
        panic!("empty exact work acquired curved image authority")
    };
    assert_eq!(r.work.exact_work, 0);
    let mut bowed = profiles.clone();
    bowed[2].control_points[1][2] += 1. / 4096.;
    let bad = MultiSweep::new(&bowed, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify_original_classified_closed_material(
            &bad,
            &[4, 4],
            &material,
            &[OriginalProfileClass::C0; 8],
            projection(),
            1e-8,
            limits(),
        )
        .unwrap()
    else {
        panic!("unchanged endpoints hid a different curved boundary")
    };
    assert_eq!(r.reason, Some("original-material-boundary-unproved"));
    assert!(r.volume.is_none());
    assert_eq!(
        (profiles, material.surface, material.trims, path),
        (before.0, before.1.surface, before.1.trims, before.2)
    );
    eprintln!(
        "curved projective original material: cells={} exact={} products={}",
        work.cells, work.exact_work, work.products
    );
}
