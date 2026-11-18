use super::*;
#[test]
fn nonlinear_line_clocks_bind_the_complete_original_material_without_map_authority() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (_, mut cap, _) = source::material();
    for (i, row) in cap.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] += (if i == j { 1. } else { -1. }) / 32768.;
        }
    }
    let (mut profiles, outer) = polygon(1., false, true);
    let (inner, hole) = polygon(0.25, true, true);
    profiles.extend(inner);
    let mut material = LoftCap {
        surface: cap,
        trims: vec![outer, hole],
    };
    for uv in material.trims.iter_mut().flatten() {
        uv.weights = vec![1., 2.];
    }
    let before = (profiles.clone(), material.clone(), path.clone());
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let classes = [
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ]
    .repeat(2);
    // Legacy retained/coedge preparation must STILL refuse map disagreement.
    let strict = super::super::super::prepare_closed_material(
        &sweep,
        &[5, 5],
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
        &[5, 5],
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
            panic!("image-bound material: {:?} {:?}", r.reason, r.volume)
        }
    };
    assert_eq!(body.line_image_spans(), 10);
    assert_eq!(body.bindings().len(), 10);
    for (i, binding) in body.bindings().iter().enumerate() {
        assert_eq!(binding.ring(), i / 5);
        assert_eq!(binding.profile(), (i, 0));
        assert_eq!(binding.trim(), (i % 5, 0));
        assert_eq!(binding.kind(), MaterialSpanBindingKind::OrientedLineImage);
    }
    assert_eq!(body.material_report().bound_spans, 10);
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.closed_seam_order(), 2);
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
    let replay = certify_original_classified_closed_material(
        &sweep,
        &[5, 5],
        &material,
        &classes,
        projection(),
        1e-8,
        exact,
    )
    .unwrap();
    assert!(matches!(
        replay,
        OriginalClassifiedClosedMaterialAttempt::Certified(_)
    ));
    let mut empty = exact;
    empty.volume.charts.exact_work = 0;
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify_original_classified_closed_material(
            &sweep,
            &[5, 5],
            &material,
            &classes,
            projection(),
            1e-8,
            empty,
        )
        .unwrap()
    else {
        panic!("zero exact work acquired image authority")
    };
    assert_eq!(r.work.exact_work, 0);
    let mut bowed = profiles.clone();
    let start = bowed[2].control_points[0].clone();
    let end = bowed[2].control_points[1].clone();
    let mut middle: Vec<_> = start.iter().zip(&end).map(|(a, b)| (a + b) * 0.5).collect();
    middle[2] += 1. / 4096.;
    bowed[2] = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![start, middle, end],
        weights: vec![1.; 3],
        periodic: false,
    };
    let bad = MultiSweep::new(&bowed, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify_original_classified_closed_material(
            &bad,
            &[5, 5],
            &material,
            &[OriginalProfileClass::C0; 10],
            projection(),
            1e-8,
            limits(),
        )
        .unwrap()
    else {
        panic!("equal endpoints hid a bowed material boundary")
    };
    assert_eq!(r.reason, Some("original-material-boundary-unproved"));
    assert!(r.volume.is_none());
    assert_eq!(
        (profiles, material.surface, material.trims, path),
        (before.0, before.1.surface, before.1.trims, before.2)
    );
    eprintln!(
        "image-bound original material: cells={} exact={} products={}",
        work.cells, work.exact_work, work.products
    );
}
