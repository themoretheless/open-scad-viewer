//! Unequal nonuniform rational cap weights, a real hole and spatial source laws.
//! Fixture arithmetic only proposes authored binary64 inputs; the normal fresh
//! native composition, region, embedding and jet factories supply all authority.
use super::*;
const L: i128 = 1068602784865;
const E: i128 = 30531508139;
fn gcd(mut a: i128, mut b: i128) -> i128 {
    while b != 0 {
        let r = a % b;
        a = b;
        b = r;
    }
    a
}
fn dyadic(mut n: i128, mut d: i128, mut exponent: i32) -> f64 {
    if n == 0 {
        return 0.;
    }
    let g = gcd(n.abs(), d);
    n /= g;
    d /= g;
    assert!(
        d > 0 && (d as u128).is_power_of_two(),
        "non-dyadic fixture ratio: {n}/{d}"
    );
    exponent -= d.trailing_zeros() as i32;
    while n % 2 == 0 {
        n /= 2;
        exponent += 1;
    }
    assert!(
        n.abs() < (1i128 << 53),
        "fixture mantissa exceeds exact binary64: {n}"
    );
    (n as f64) * 2_f64.powi(exponent)
}
// Homogeneous blossom at p-k copies of a/4 and k copies of b/4. All arithmetic
// before conversion is integral; no rounded knot insertion is trusted.
fn coefficients(a: i128, b: i128, k: usize) -> (i128, i128) {
    let mut w = [5, 4, 3, 2, 1];
    let mut x = [0, 4, 3, 6, 4]; // wi * [0,1,1,3,4]
    for level in 0..4 {
        let t = if level < 4 - k { a } else { b };
        for j in 0..4 - level {
            w[j] = (4 - t) * w[j] + t * w[j + 1];
            x[j] = (4 - t) * x[j] + t * x[j + 1];
        }
    }
    (x[0], w[0])
}
fn position(t: i128) -> f64 {
    let (n, d) = coefficients(t, t, 0);
    dyadic(L * n, d, -51)
}
fn profile(a: [i128; 2], b: [i128; 2]) -> Curve {
    let moving = usize::from(a[0] == b[0]);
    let fixed = 1 - moving;
    let mut points = Vec::new();
    let mut weights = Vec::new();
    for k in 0..5 {
        let (n, d) = coefficients(a[moving], b[moving], k);
        let mut p = vec![0.; 3];
        p[moving] = dyadic(L * n, d, -51);
        p[fixed] = position(a[fixed]);
        points.push(p);
        weights.push(dyadic(d, 256, 0));
    }
    Curve {
        degree: 4,
        knots: [vec![-2.; 5], vec![6.; 5]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn source() -> (Vec<Curve>, [LoftCap; 2]) {
    // Full original profiles are retained. The scale is a common binary unit
    // chosen to make every required original pole exactly representable. It
    // does not replace the existing unit-size spatial qualification fixtures.
    let rings = [
        vec![[0, 0], [1, 0], [4, 0], [4, 4], [0, 4], [0, 0]],
        vec![[1, 1], [1, 3], [3, 3], [3, 1], [1, 1]],
    ];
    let profiles = rings
        .iter()
        .flat_map(|r| r.windows(2).map(|p| profile(p[0], p[1])))
        .collect();
    let wi = [5i128, 4, 3, 2, 1];
    let xi = [0, 1, 1, 3, 4];
    let q = [0, -9, 20, -9, 0];
    let caps = std::array::from_fn(|end| {
        let (u, v) = if end == 0 {
            ([-3., 5.], [7., 11.])
        } else {
            ([17., 25.], [-7., 1.])
        };
        let mut points = vec![vec![vec![0.; 3]; 5]; 5];
        let mut weights = vec![vec![0.; 5]; 5];
        for i in 0..5 {
            for j in 0..5 {
                let w = wi[i] * wi[j];
                if end == 0 {
                    weights[i][j] = w as f64;
                    points[i][j] = vec![dyadic(L * xi[i], 1, -51), dyadic(L * xi[j], 1, -51), 0.];
                } else {
                    // W1=2 W0+16 q(u)q(v); N1xy=2 N0xy. Positive weights
                    // are deliberately NOT a common gauge of the first cap.
                    let numerator = 32 * w + q[i] * q[j];
                    assert!(numerator > 0);
                    weights[i][j] = dyadic(numerator, 16, 0);
                    points[i][j] = vec![
                        dyadic(L * xi[i] * 32 * w, numerator, -51),
                        dyadic(L * xi[j] * 32 * w, numerator, -51),
                        dyadic(E * q[i] * q[j], numerator, -62),
                    ];
                }
            }
        }
        let surface = Surface {
            degree_u: 4,
            degree_v: 4,
            knots_u: [vec![u[0]; 5], vec![u[1]; 5]].concat(),
            knots_v: [vec![v[0]; 5], vec![v[1]; 5]].concat(),
            control_points: points,
            weights,
            periodic_u: false,
            periodic_v: false,
        };
        let trims = rings
            .iter()
            .map(|r| {
                r.windows(2)
                    .map(|p| Curve {
                        degree: 1,
                        knots: vec![13., 13., 19., 19.],
                        control_points: p
                            .iter()
                            .map(|p| {
                                vec![
                                    u[0] + (u[1] - u[0]) * p[0] as f64 / 4.,
                                    v[0] + (v[1] - v[0]) * p[1] as f64 / 4.,
                                ]
                            })
                            .collect(),
                        weights: vec![if end == 0 { 1. } else { 2. }; 2],
                        periodic: false,
                    })
                    .collect()
            })
            .collect();
        LoftCap { surface, trims }
    });
    assert_ne!(
        caps[0].surface.weights[1][1] * caps[1].surface.weights[2][3],
        caps[1].surface.weights[1][1] * caps[0].surface.weights[2][3]
    );
    assert_ne!(
        caps[0].surface.control_points,
        caps[1].surface.control_points
    );
    (profiles, caps)
}
fn run(authored: bool, arc: bool, affine: bool) {
    let (profiles, caps) = source();
    let mut classes = vec![OriginalProfileClass::C0; 9];
    classes[0] = OriginalProfileClass::G2;
    super::spatial::run_source(
        authored,
        arc,
        affine,
        true,
        "rational-hollow",
        profiles,
        caps,
        vec![5, 4],
        classes,
    );
}
#[test]
fn authored_unequal_rational_hollow_caps_clock_and_affine_matrix() {
    for arc in [false, true] {
        for affine in [false, true] {
            run(true, arc, affine);
        }
    }
}
#[test]
fn guided_unequal_rational_hollow_caps_clock_and_affine_matrix() {
    for arc in [false, true] {
        for affine in [false, true] {
            run(false, arc, affine);
        }
    }
}

// The original source proof is insufficient: exercise the complete retained
// admission independently, with unchanged native transport proof allowances.
#[test]
fn rounded_rational_hollow_cap_refuses_when_translation_breaks_exact_trim_trace() {
    use crate::sweep_boundary_owned::{
        OriginalClassifiedBodyAttempt, construct_with_original_classified_material,
    };
    let (profiles, caps) = source();
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let mut end = caps[1].clone();
    for p in end.surface.control_points.iter_mut().flatten() {
        p[2] += 2.;
    }
    let mut classes = vec![OriginalProfileClass::C0; 9];
    classes[0] = OriginalProfileClass::G2;
    let before = (
        profiles.clone(),
        caps.iter()
            .map(|c| (c.surface.clone(), c.trims.clone()))
            .collect::<Vec<_>>(),
    );
    let original = certify_original_classified_capped_material(
        &sweep,
        &[5, 4],
        [&caps[0], &caps[1]],
        &classes,
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        1e-8,
        limits(),
    )
    .unwrap();
    let OriginalClassifiedCappedMaterialAttempt::Certified(original) = original else {
        panic!("source rational material must be independently proven")
    };
    assert!(original.volume().report().charts.volume.embedding_certified);
    let out = construct_with_original_classified_material(
        &sweep,
        &[5, 4],
        5,
        [&caps[0], &caps[1]],
        [&caps[0], &end],
        &classes,
        1e-8,
        0.05,
        crate::sweep_boundary_owned_tests::limits(),
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
    );
    let error = match out {
        Err(error) => error,
        Ok(_) => panic!("rounded filled cap must not borrow the original material certificate"),
    };
    assert_eq!(error.code, "BREP_RATIONAL_SWEEP_REFUSED");
    assert!(
        error
            .message
            .contains("Authored cap whole-boundary agreement is not proven exact")
    );
    assert!(error.message.contains("outcome: Different"));
    assert_eq!(profiles, before.0);
    assert_eq!(
        caps.iter()
            .map(|c| (c.surface.clone(), c.trims.clone()))
            .collect::<Vec<_>>(),
        before.1
    );
}

#[test]
fn rational_hollow_cap_amplitude_translation_retains_nonplanarity_and_exact_material_traces() {
    let (profiles, caps) = source();
    let before = caps
        .iter()
        .map(|c| (c.surface.clone(), c.trims.clone()))
        .collect::<Vec<_>>();
    let literal =
        crate::rational_cap_translation::propose(&caps[0], [0., 0., 2.], [0.; 3], 0., 1000000)
            .unwrap();
    assert!(literal.cap.is_some());
    assert_eq!(literal.displacement_upper, Some(0.));
    assert_eq!(literal.modified_axes, [false; 3]);
    let report =
        crate::rational_cap_translation::propose(&caps[1], [0., 0., 2.], [0.; 3], 0.05, 1000000)
            .unwrap();
    let end = report.cap.as_ref().unwrap_or_else(|| {
        panic!(
            "proposal refused: {} bound={:?}",
            report.reason, report.displacement_upper
        )
    });
    assert_eq!(report.modified_axes, [false, false, true]);
    assert!(
        end.surface
            .control_points
            .iter()
            .flatten()
            .any(|p| p[2] != 2.)
    );
    let mut work = 0;
    for (profile, uv) in profiles.iter().zip(end.trims.iter().flatten()) {
        let mut world = profile.clone();
        for p in &mut world.control_points {
            p[2] += 2.;
        }
        let proof = nurbs_core::curve_surface_agreement::verify_exact(
            &world,
            uv,
            &end.surface,
            false,
            1000000 - work,
        )
        .unwrap()
        .unwrap();
        work += proof.work_used;
        assert_eq!(proof.outcome, cad_predicates::BezierIdentity::Equal);
    }
    let replay = crate::rational_cap_translation::propose(
        &caps[1],
        [0., 0., 2.],
        [0.; 3],
        0.05,
        report.work,
    )
    .unwrap();
    assert!(replay.cap.is_some());
    let short = crate::rational_cap_translation::propose(
        &caps[1],
        [0., 0., 2.],
        [0.; 3],
        0.05,
        report.work - 1,
    )
    .unwrap();
    assert!(short.cap.is_none());
    let strict =
        crate::rational_cap_translation::propose(&caps[1], [0., 0., 2.], [0.; 3], 1e-8, 1000000)
            .unwrap();
    assert!(strict.cap.is_none());
    assert_eq!(
        strict.reason,
        "rational-cap-translation-error-budget-exceeded"
    );
    let path = nurbs_core::primitives::line([0.; 3], [0., 0., 2.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let axis = constant_vector_law([0., 0., 1.]).unwrap();
    let normal = constant_vector_law([1., 0., 0.]).unwrap();
    let sweep = MultiSweep::new(
        &profiles,
        &path,
        &scale,
        &twist,
        Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 5,
            max_sections: 5,
            max_deviation: 0.05,
        },
    )
    .unwrap()
    .with_frame_laws(&axis, &normal)
    .unwrap();
    let mut classes = vec![OriginalProfileClass::C0; 9];
    classes[0] = OriginalProfileClass::G2;
    let body = crate::sweep_boundary_owned::construct_affine_with_original_classified_material(
        &sweep,
        &[5, 4],
        5,
        [&caps[0], &caps[1]],
        &classes,
        1e-8,
        0.05,
        crate::sweep_boundary_owned_tests::limits(),
        [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
        limits(),
        crate::AffineStationLimits {
            quantum: 2f64.powi(-20),
            tolerance: 0.001,
            work: 1000000,
            matrix_cells: 100000,
        },
    )
    .unwrap();
    let crate::sweep_boundary_owned::OriginalClassifiedBodyAttempt::Certified(body) = body else {
        let crate::sweep_boundary_owned::OriginalClassifiedBodyAttempt::Refused(r) = body else {
            unreachable!()
        };
        panic!(
            "corrected rational actual solid refused: {} retained={:?}",
            r.reason,
            r.retained.as_ref().map(|r| (
                r.reason,
                r.affine_volume.as_ref().map(|v| (
                    v.work,
                    v.agreement.as_ref().map(|a| (a.work, a.all_equal)),
                    v.cells,
                    v.volume.as_ref().map(|f| (f.reason, f.cells, f.exact_work))
                ))
            ))
        );
    };
    assert!(body.retained_report().certified);
    let actual = body.retained_report().affine_volume.as_ref().unwrap();
    assert!(actual.certified && actual.translated_trace_transport && actual.paired_carriers);
    let filled = actual.volume.as_ref().unwrap();
    assert!(
        filled.embedding_certified
            && filled.regularity_certified
            && filled.parameter_topology_preserved
    );
    assert!(actual.orientation.as_ref().unwrap().sign.is_some());
    assert!(
        body.retained_report()
            .affine_candidate
            .as_ref()
            .unwrap()
            .cap_correction_upper[1]
            > 0.
    );
    let run = |work, cells, tolerance| {
        crate::sweep_boundary_owned::construct_affine_with_original_classified_material(
            &sweep,
            &[5, 4],
            5,
            [&caps[0], &caps[1]],
            &classes,
            1e-8,
            0.05,
            crate::sweep_boundary_owned_tests::limits(),
            [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            limits(),
            crate::AffineStationLimits {
                quantum: 2f64.powi(-20),
                tolerance,
                work,
                matrix_cells: cells,
            },
        )
        .unwrap()
    };
    for (w, c) in [(actual.work, 100000), (1000000, actual.cells)] {
        let crate::sweep_boundary_owned::OriginalClassifiedBodyAttempt::Certified(_) =
            run(w, c, 0.001)
        else {
            panic!("measured actual replay refused work={w} cells={c}")
        };
    }
    for (w, c, t) in [
        (actual.work - 1, 100000, 0.001),
        (1000000, actual.cells - 1, 0.001),
        (1000000, 100000, 1e-8),
    ] {
        let crate::sweep_boundary_owned::OriginalClassifiedBodyAttempt::Refused(r) = run(w, c, t)
        else {
            panic!("incomplete actual correction admitted")
        };
        assert!(r.original.is_none() && r.retained.is_some());
        if let Some(v) = r.retained.as_ref().unwrap().affine_volume.as_ref() {
            assert!(v.work <= w && v.cells <= c);
        }
    }
    assert_eq!(
        caps.iter()
            .map(|c| (c.surface.clone(), c.trims.clone()))
            .collect::<Vec<_>>(),
        before
    );
    eprintln!(
        "corrected actual field: work={} cells={} filled_cells={} filled_exact={} error={:?}",
        actual.work,
        actual.cells,
        filled.cells,
        filled.exact_work,
        body.retained_report().boundary_error_upper
    );
    assert_eq!(body.cap_wall_continuity(), "C0");
    assert_eq!(body.boundary_classes(), classes);
    eprintln!(
        "rational translated nonplanar cap: work={} boundary_work={} bound={:?}",
        report.work, work, report.displacement_upper
    );
}
