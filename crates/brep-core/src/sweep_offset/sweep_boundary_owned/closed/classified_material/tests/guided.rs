//! Joint source certificate: guided periodic motion, affine laws and material.
use super::*;
#[test]
fn closed_guided_affine_nonplanar_material_hole_has_owned_global_g2_seam() {
    let path = source::path();
    let mut guide = path.clone();
    for p in &mut guide.control_points {
        p[2] += 1.;
    }
    let (_, _, twist) = source::laws();
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
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 1.125, 0.875]).unwrap();
    let center = constant_vector_law([1. / 1024., 0., 0.]).unwrap();
    let mut o = options();
    o.orientation = Orientation::RotationMinimizing;
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, o.clone())
        .unwrap()
        .with_orientation_guide(&guide)
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
            panic!("guided affine material: {:?} {:?}", r.reason, r.volume)
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
    let mut bad = guide.clone();
    bad.control_points[1][2] += 0.125;
    let broken = MultiSweep::new(&profiles, &path, &scale, &twist, o)
        .unwrap()
        .with_orientation_guide(&bad)
        .unwrap()
        .with_affine_laws(&axes, &center)
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
        panic!("sharp classes bypassed broken guide seam")
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
    eprintln!(
        "guided affine nonplanar closed material: cells={} exact={} products={}",
        work.cells, work.exact_work, work.products
    );
}
