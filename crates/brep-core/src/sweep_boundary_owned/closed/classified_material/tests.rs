use super::*;
use nurbs_core as nurbs;
use nurbs_core::progressive_sweep::{Options, Orientation, Spacing, constant_vector_law};
#[path = "../../../../../nurbs-core/tests/support/general_frame_source.rs"]
mod source;
fn limits() -> OriginalClassifiedClosedMaterialLimits {
    OriginalClassifiedClosedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                order: 2,
                cells: 1000000,
                exact_work: 1000000,
                products: 1000000,
            },
            steps: 1000000,
        },
        material: MaterialLimits {
            pairs: 10000,
            cells: 100000,
            domain_cells: 1000000,
        },
    }
}
fn projection() -> [[f64; 3]; 3] {
    [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]
}
fn polygon(scale: f64, reverse: bool, nonplanar: bool) -> (Vec<Curve>, Vec<Curve>) {
    let r = 1. / 1024.;
    let signs = if reverse {
        [
            [-1., -1.],
            [-1., 0.],
            [-1., 1.],
            [1., 1.],
            [1., -1.],
            [-1., -1.],
        ]
    } else {
        [
            [-1., -1.],
            [0., -1.],
            [1., -1.],
            [1., 1.],
            [-1., 1.],
            [-1., -1.],
        ]
    };
    let xyz = |p: [f64; 2]| {
        let [a, b] = p.map(|v| v * scale);
        [
            -2. * r * a,
            -r * b,
            9. * r * a + if nonplanar { r * a * b / 32. } else { 0. },
        ]
    };
    let uv = |p: [f64; 2]| vec![1. + 4. * scale * p[0], 9. + 2. * scale * p[1]];
    (
        signs
            .windows(2)
            .map(|p| nurbs_core::primitives::line(xyz(p[0]), xyz(p[1])).unwrap())
            .collect(),
        signs
            .windows(2)
            .map(|p| Curve::from_polyline(vec![uv(p[0]), uv(p[1])]).unwrap())
            .collect(),
    )
}
fn options() -> Options {
    Options {
        normal: [0., 0., 1.],
        orientation: Orientation::Fixed,
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 100000,
        },
        initial_sections: 33,
        max_sections: 129,
        max_deviation: 0.05,
    }
}
fn run(hole: bool, nonplanar: bool) {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (_, mut cap, _) = source::material();
    if nonplanar {
        for (i, row) in cap.control_points.iter_mut().enumerate() {
            for (j, p) in row.iter_mut().enumerate() {
                p[2] += (if i == j { 1. } else { -1. }) / 32768.;
            }
        }
    }
    let (mut profiles, outer) = polygon(1., false, nonplanar);
    let mut material = LoftCap {
        surface: cap,
        trims: vec![outer],
    };
    let mut loops = vec![5];
    let mut classes = vec![
        OriginalProfileClass::G2,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
        OriginalProfileClass::C0,
    ];
    if hole {
        let (p, t) = polygon(0.25, true, nonplanar);
        profiles.extend(p);
        material.trims.push(t);
        loops.push(5);
        classes.extend(classes.clone());
    }
    let before = (profiles.clone(), material.clone(), path.clone());
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    macro_rules! certify {
        ($m:expr,$c:expr,$l:expr) => {
            certify_original_classified_closed_material(
                &sweep,
                &loops,
                $m,
                $c,
                projection(),
                1e-8,
                $l,
            )
        };
    }
    let body = match certify!(&material, &classes, limits()).unwrap() {
        OriginalClassifiedClosedMaterialAttempt::Certified(b) => b,
        OriginalClassifiedClosedMaterialAttempt::Refused(r) => {
            panic!("material binding: {:?}, volume={:?}", r.reason, r.volume)
        }
    };
    assert!(body.material_report().certified);
    assert_eq!(body.material_report().bound_spans, classes.len());
    assert_eq!(
        body.material_report().trim.as_ref().unwrap().valid,
        Some(true)
    );
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.closed_seam_order(), 2);
    assert!(std::ptr::eq(body.material(), &material));
    assert!(std::ptr::eq(body.volume().caps()[0], &material.surface));
    assert!(std::ptr::eq(body.volume().sweep(), &sweep));
    let work = body.work();
    let native = body.volume().report();
    assert!(
        work.cells > native.cells
            && work.products > native.products
            && work.exact_work > native.exact_work
    );
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
        certify!(&material, &classes, exact).unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Certified(_)
    ));
    for l in [
        OriginalClassifiedClosedMaterialLimits {
            volume: OriginalClassifiedVolumeLimits {
                charts: OriginalSmoothVolumeLimits {
                    products: work.products - 1,
                    ..exact.volume.charts
                },
                ..exact.volume
            },
            ..exact
        },
        OriginalClassifiedClosedMaterialLimits {
            volume: OriginalClassifiedVolumeLimits {
                charts: OriginalSmoothVolumeLimits {
                    exact_work: 0,
                    ..exact.volume.charts
                },
                ..exact.volume
            },
            ..exact
        },
        OriginalClassifiedClosedMaterialLimits {
            volume: OriginalClassifiedVolumeLimits {
                charts: OriginalSmoothVolumeLimits {
                    cells: 0,
                    ..exact.volume.charts
                },
                ..exact.volume
            },
            ..exact
        },
        OriginalClassifiedClosedMaterialLimits {
            material: MaterialLimits {
                pairs: 0,
                ..exact.material
            },
            ..exact
        },
    ] {
        let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
            certify!(&material, &classes, l).unwrap()
        else {
            panic!("exhausted shared material work admitted")
        };
        assert!(
            r.work.cells <= l.volume.charts.cells
                && r.work.products <= l.volume.charts.products
                && r.work.exact_work <= l.volume.charts.exact_work
        );
    }
    let regularity_cells = body
        .material_report()
        .profiles
        .iter()
        .map(|p| p.cells)
        .sum();
    let domain_limited = OriginalClassifiedClosedMaterialLimits {
        volume: OriginalClassifiedVolumeLimits {
            charts: OriginalSmoothVolumeLimits {
                cells: regularity_cells,
                ..limits().volume.charts
            },
            ..limits().volume
        },
        ..limits()
    };
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify!(&material, &classes, domain_limited).unwrap()
    else {
        panic!("carrier-domain hull stage reset exhausted source cells")
    };
    assert_eq!(
        r.reason,
        Some("original-material-carrier-domain-work-limit")
    );
    assert!(r.work.cells <= regularity_cells && r.volume.is_none());
    // Endpoints and the complete straight UV image are unchanged; rational
    // speed differs from the original world profile: only image authority
    // may bind this source. The strict map route is tested separately.
    let mut wrong_clock = material.clone();
    wrong_clock.trims[0][0].weights[1] = 2.;
    let OriginalClassifiedClosedMaterialAttempt::Certified(image_bound) =
        certify!(&wrong_clock, &classes, limits()).unwrap()
    else {
        panic!("complete oriented line image failed fresh source binding")
    };
    assert_eq!(image_bound.line_image_spans(), 1);
    assert_eq!(image_bound.boundary_classes(), classes);
    assert!(std::ptr::eq(image_bound.material(), &wrong_clock));
    let mut promoted = classes.clone();
    promoted[1] = OriginalProfileClass::G2;
    assert!(matches!(
        certify!(&material, &promoted, limits()).unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Refused(_)
    ));
    let mut wrong = material.clone();
    wrong.trims[0][0].control_points[1][0] += 0.125;
    assert!(matches!(
        certify!(&wrong, &classes, limits()).unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Refused(_)
    ));
    let mut changed = material.clone();
    for p in changed.surface.control_points.iter_mut().flatten() {
        p[2] += 0.125;
    }
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify!(&changed, &classes, limits()).unwrap()
    else {
        panic!("source carrier replacement rebound old traces")
    };
    assert_eq!(r.reason, Some("original-material-boundary-unproved"));
    if hole {
        let mut wrong = material.clone();
        wrong.trims[1] = wrong.trims[1]
            .iter()
            .rev()
            .map(|c| c.reverse().unwrap())
            .collect();
        let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
            certify!(&wrong, &classes, limits()).unwrap()
        else {
            panic!("wrong hole orientation admitted")
        };
        assert_eq!(r.material.trim.unwrap().reason, "hole-orientation");
        let mut wrong = material.clone();
        wrong.trims.pop();
        assert!(certify!(&wrong, &classes, limits()).is_err());
    }
    assert_eq!(
        (&profiles, &material.surface, &material.trims, &path),
        (&before.0, &before.1.surface, &before.1.trims, &before.2)
    );
    eprintln!(
        "closed classified material: hole={hole} nonplanar={nonplanar} cells={} exact={} products={}",
        work.cells, work.exact_work, work.products
    );
}
#[test]
fn exact_closed_material_binding_covers_mixed_classes_and_source_immutability() {
    run(false, false);
}
#[test]
fn spatial_nonplanar_closed_material_with_hole_preserves_sharp_and_smooth_joins() {
    run(true, true);
}
#[test]
fn rational_material_g1_jumps_are_bound_to_the_actual_uv_clock() {
    let path = source::path();
    let (a, n, twist) = source::laws();
    let (mut profiles, cap, mut trims) = source::material();
    profiles[0].weights[1] = 0.25;
    trims[0][0].weights[1] = 0.25;
    let material = LoftCap {
        surface: cap,
        trims,
    };
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(&profiles, &path, &scale, &twist, options())
        .unwrap()
        .with_frame_laws(&a, &n)
        .unwrap();
    let classes = [
        OriginalProfileClass::G1,
        OriginalProfileClass::G2,
        OriginalProfileClass::G2,
        OriginalProfileClass::G1,
    ];
    let body = match certify_original_classified_closed_material(
        &sweep,
        &[4],
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
            panic!("bound rational G1: {:?}, {:?}", r.reason, r.volume)
        }
    };
    assert_eq!(body.boundary_classes(), classes);
    assert_eq!(body.material_report().bound_spans, 4);
    assert_eq!(body.closed_seam_order(), 2);
    assert!(matches!(
        certify_original_classified_closed_material(
            &sweep,
            &[4],
            &material,
            &[OriginalProfileClass::G2; 4],
            projection(),
            1e-8,
            limits()
        )
        .unwrap(),
        OriginalClassifiedClosedMaterialAttempt::Refused(_)
    ));
    let mut wrong = material.clone();
    wrong.trims[0][0].weights[1] = 0.5;
    let OriginalClassifiedClosedMaterialAttempt::Refused(r) =
        certify_original_classified_closed_material(
            &sweep,
            &[4],
            &wrong,
            &classes,
            projection(),
            1e-8,
            limits(),
        )
        .unwrap()
    else {
        panic!("different rational trace acquired binding")
    };
    assert_eq!(r.reason, Some("original-material-boundary-unproved"));
}

mod guided;

mod authored;

mod varying;

mod image;
