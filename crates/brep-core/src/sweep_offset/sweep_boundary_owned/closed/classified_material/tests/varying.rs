//! Joint source certificate: authored periodic motion, affine laws and material.
use super::*;
fn periodic(base: [f64; 3], delta: [f64; 3], domain: [f64; 2]) -> Curve {
    Curve {
        degree: 4,
        knots: [vec![domain[0]; 5], vec![domain[1]; 5]].concat(),
        control_points: (0..5)
            .map(|i| {
                (0..3)
                    .map(|k| base[k] + if i == 2 { delta[k] } else { 0. })
                    .collect()
            })
            .collect(),
        weights: vec![1., 1., 0.75, 1., 1.],
        periodic: false,
    }
}
#[test]
fn closed_authored_varying_rational_affine_material_uses_each_original_law_clock() {
    run(false);
}
#[test]
fn closed_guided_varying_rational_affine_material_uses_each_original_law_clock() {
    run(true);
}
fn run(guided: bool) {
    let path = source::path();
    let (guide, normal, twist) = source::laws();
    let normal_before = normal.clone();
    let mut orientation_guide = path.clone();
    for p in &mut orientation_guide.control_points {
        p[2] += 1.;
    }
    let orientation_before = orientation_guide.clone();
    let (_, mut cap, _) = source::material();
    for (i, row) in cap.control_points.iter_mut().enumerate() {
        for (j, p) in row.iter_mut().enumerate() {
            p[2] += (if i == j { 1. } else { -1. }) / 32768.;
        }
    }
    let (mut profiles, outer) = polygon(1., false, true);
    let (inner, hole) = polygon(0.25, true, true);
    profiles.extend(inner);
    let material = LoftCap {
        surface: cap,
        trims: vec![outer, hole],
    };
    let before = (
        path.clone(),
        guide.clone(),
        material.clone(),
        profiles.clone(),
    );
    let scale = periodic([1., 0., 0.], [0.125, 0., 0.], [-11., -3.]);
    let axes = periodic([1., 1.125, 0.875], [0.0625, -0.0625, 0.03125], [31., 43.]);
    let center = periodic(
        [1. / 1024., 0., 0.],
        [1. / 4096., -1. / 4096., 1. / 8192.],
        [101., 105.],
    );
    let laws_before = (scale.clone(), axes.clone(), center.clone());
    let mut o = options();
    if guided {
        o.orientation = Orientation::RotationMinimizing;
    }
    let base = MultiSweep::new(&profiles, &path, &scale, &twist, o.clone()).unwrap();
    let sweep = if guided {
        base.with_orientation_guide(&orientation_guide)
    } else {
        base.with_frame_laws(&guide, &normal)
    }
    .unwrap()
    .with_affine_laws(&axes, &center)
    .unwrap();
    let classes = [
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ]
    .repeat(2);
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
            panic!("authored affine material: {:?} {:?}", r.reason, r.volume)
        }
    };
    let r = body.volume().report();
    assert!(
        r.certified
            && r.charts.volume.embedding_certified
            && r.charts.volume.regularity_certified
            && r.charts.volume.parameter_topology_preserved
    );
    assert_eq!(body.closed_seam_order(), 2);
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(
        body.material_report().trim.as_ref().unwrap().winding,
        vec![Some(1), Some(-1)]
    );
    assert_eq!(r.profiles.as_ref().unwrap().smooth_joins, 2);
    assert!(std::ptr::eq(body.material(), &material));
    assert!(std::ptr::eq(body.volume().sweep(), &sweep));
    let work = body.work();
    let exact = OriginalClassifiedClosedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: work.cells,
                exact_work: work.exact_work,
                products: work.products,
                ..limits().volume.charts
            },
            steps: r.profiles.as_ref().unwrap().steps,
        },
        ..limits()
    };
    assert!(matches!(
        certify_original_classified_closed_material(
            &sweep,
            &[5, 5],
            &material,
            &classes,
            projection(),
            1e-8,
            exact
        )
        .unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Certified(_)
    ));
    let mut bad = axes.clone();
    bad.control_points[1][0] += 1. / 256.;
    let base = MultiSweep::new(&profiles, &path, &scale, &twist, o).unwrap();
    let broken = if guided {
        base.with_orientation_guide(&orientation_guide)
    } else {
        base.with_frame_laws(&guide, &normal)
    }
    .unwrap()
    .with_affine_laws(&bad, &center)
    .unwrap();
    let OriginalClassifiedClosedMaterialAttempt::Refused(q) =
        certify_original_classified_closed_material(
            &broken,
            &[5, 5],
            &material,
            &[OriginalProfileClass::C0; 10],
            projection(),
            1e-8,
            limits(),
        )
        .unwrap()
    else {
        panic!("sharp classes bypassed broken affine-law seam")
    };
    assert!(q.volume.is_some());
    assert_eq!(
        (&path, &guide, &material.surface, &material.trims, &profiles),
        (
            &before.0,
            &before.1,
            &before.2.surface,
            &before.2.trims,
            &before.3
        )
    );
    assert_eq!(normal, normal_before);
    assert_eq!(orientation_guide, orientation_before);
    assert_eq!((scale, axes, center), laws_before);
    eprintln!(
        "varying affine nonplanar closed material: guided={} cells={} exact={} products={}",
        guided, work.cells, work.exact_work, work.products
    );
}
