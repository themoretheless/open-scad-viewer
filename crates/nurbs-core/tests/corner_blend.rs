use nurbs_core::{
    corner_blend::{self, Options},
    surface::Surface,
    surface_blend::Boundary,
};
fn support(x: [f64; 2], y: [f64; 2], rational: bool) -> Surface {
    let mut points = vec![vec![vec![0.; 3]; 3]; 3];
    let mut weights = vec![vec![1.; 3]; 3];
    let square = |r: [f64; 2]| [r[0] * r[0], r[0] * r[1], r[1] * r[1]];
    for i in 0..3 {
        for j in 0..3 {
            let a = x[0] + (x[1] - x[0]) * i as f64 / 2.;
            let b = y[0] + (y[1] - y[0]) * j as f64 / 2.;
            let w = if rational {
                1. + 0.1 * a + 0.05 * b
            } else {
                1.
            };
            weights[i][j] = w;
            points[i][j] = vec![a / w, b / w, (square(x)[i] / 4. + square(y)[j] / 10.) / w];
        }
    }
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: points,
        weights,
        periodic_u: false,
        periodic_v: false,
    }
}
fn options() -> Options {
    Options {
        boundaries: [
            Boundary::UEnd,
            Boundary::UStart,
            Boundary::VEnd,
            Boundary::VStart,
        ],
        gains: [1.; 4],
        reverse_along: [false; 4],
        compatibility_tolerance: 1e-9,
        max_controls: 4096,
    }
}
fn supports(rational: bool) -> [Surface; 4] {
    [
        support([-1., 0.], [0., 1.], rational),
        support([1., 2.], [0., 1.], rational),
        support([0., 1.], [-1., 0.], rational),
        support([0., 1.], [1., 2.], rational),
    ]
}
#[test]
fn four_original_supports_with_distinct_curvatures_qualify_all_seams() {
    for rational in [false, true] {
        let s = supports(rational);
        let saved = s.clone();
        let r = corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000).unwrap();
        assert!(r.complete && r.within_tolerance);
        assert!(r.admitted && r.interior.certified);
        assert_eq!(r.seams.len(), 4);
        assert!(r.cells <= 100000);
        assert!(r.coefficient_work <= 2000000);
        for seam in &r.seams {
            assert!(seam.position_bounds[1] <= 1e-7);
            let bounds = &seam.branches[usize::from(matches!(
                seam.orientation,
                Some(nurbs_core::surface_g2::Orientation::Opposite)
            ))];
            assert!(bounds.normal_upper.unwrap() <= 1e-5);
            assert!(bounds.tensor_upper.unwrap() <= 1e-4);
        }
        for u in [0., 0.17, 0.51, 0.83, 1.] {
            for v in [0., 0.19, 0.47, 0.89, 1.] {
                let p = r.surface.evaluate(u, v).unwrap().point;
                let w = if rational {
                    1. + 0.1 * u + 0.05 * v
                } else {
                    1.
                };
                let expected = [u / w, v / w, (u * u / 4. + v * v / 10.) / w];
                for k in 0..3 {
                    assert!((p[k] - expected[k]).abs() < 1e-10);
                }
            }
        }
        assert_eq!(s, saved);
    }
}
#[test]
fn incompatible_corners_and_exhausted_proof_work_never_qualify() {
    let mut s = supports(false);
    let r = corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 0).unwrap();
    assert!(!r.complete && !r.within_tolerance && r.seams.is_empty());
    for row in &mut s[3].control_points {
        for p in row {
            p[2] += 0.1;
        }
    }
    assert!(corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000).is_err());
    // Allow conflicting proposals deliberately: the independent original
    // support audit must still prevent a successful continuity flag.
    let mut permissive = options();
    permissive.compatibility_tolerance = 10.;
    let r = corner_blend::g2(s.each_ref(), permissive, [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(!r.within_tolerance);
    let s = supports(false);
    let mut o = options();
    o.gains[0] = f64::NAN;
    assert!(corner_blend::g2(s.each_ref(), o, [1e-7, 1e-5, 1e-4], 100000).is_err());
}
#[test]
fn original_reversed_rational_charts_and_unequal_parameter_scales_qualify() {
    let mut s = supports(true);
    s[0] = nurbs_core::surface_edit::reverse(&s[0], nurbs_core::surface::Axis::V).unwrap();
    for k in &mut s[0].knots_u {
        *k = 2. + 2. * *k;
    }
    let mut o = options();
    o.reverse_along[0] = true;
    o.gains[0] = 2.;
    let r = corner_blend::g2(s.each_ref(), o, [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(r.complete && r.within_tolerance && r.seams.len() == 4);
}
#[test]
fn curvature_defect_with_unchanged_boundary_positions_and_tangents_is_refused() {
    let mut s = supports(false);
    for row in &mut s[3].control_points {
        row[2][2] += 0.01;
    }
    let mut o = options();
    o.compatibility_tolerance = 10.;
    let r = corner_blend::g2(s.each_ref(), o, [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(!r.within_tolerance);
    assert!(r.cells <= 100000 && r.coefficient_work <= 2000000);
}
#[test]
fn exact_multispan_maps_qualify_with_reversal_and_nonunit_domains() {
    use nurbs_core::{surface::Axis, surface_edit};
    let mut s = supports(true);
    s[0] = surface_edit::elevate(&s[0], Axis::V, 3).unwrap();
    s[0] = surface_edit::insert(&s[0], Axis::V, 0.5, 1).unwrap();
    let r = corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(r.complete && r.within_tolerance);
    assert!(r.coefficient_work <= 2000000);
    for k in &mut s[0].knots_v {
        *k = 2. + 2. * *k;
    }
    let r = corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(r.complete && r.within_tolerance && r.exact_work <= 1000000);
    let zero =
        corner_blend::g2_with_exact_limit(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000, 0)
            .unwrap();
    assert!(!zero.complete && !zero.within_tolerance && zero.exact_work == 0);
    s[0] = surface_edit::reverse(&s[0], Axis::V).unwrap();
    let mut o = options();
    o.reverse_along[0] = true;
    let r = corner_blend::g2(s.each_ref(), o, [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(r.complete && r.within_tolerance && r.exact_work <= 1000000);
}
#[test]
fn rounded_multispan_knot_map_cannot_authorize_missing_slivers() {
    use nurbs_core::{surface::Axis, surface_edit};
    let mut s = supports(true);
    s[0] = surface_edit::elevate(&s[0], Axis::V, 3).unwrap();
    s[0] = surface_edit::insert(&s[0], Axis::V, 0.5, 1).unwrap();
    for k in &mut s[0].knots_v {
        *k = 0.1 + *k;
    }
    let r = corner_blend::g2(s.each_ref(), options(), [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(!r.complete && !r.within_tolerance && r.exact_work <= 1000000);
}
#[test]
fn interior_partial_traces_are_proved_on_immutable_extended_originals() {
    use nurbs_core::corner_blend::Trace;
    for rational in [false, true] {
        let mut original = support([-1., 2.], [-1., 2.], rational);
        for k in original.knots_u.iter_mut().chain(&mut original.knots_v) {
            *k = 2. + 6. * *k;
        }
        let before = original.clone();
        let traces = [
            Trace {
                at: 4.,
                along: [4., 6.],
            },
            Trace {
                at: 6.,
                along: [4., 6.],
            },
            Trace {
                at: 4.,
                along: [4., 6.],
            },
            Trace {
                at: 6.,
                along: [4., 6.],
            },
        ];
        let mut o = options();
        o.gains = [2.; 4];
        let r = corner_blend::g2_traces(
            [&original; 4],
            traces,
            o,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000,
        )
        .unwrap();
        assert!(r.admitted && r.complete && r.within_tolerance && r.seams.len() == 4);
        assert!(r.cells <= 100000 && r.coefficient_work <= 2000000 && r.exact_work <= 1000000);
        for u in [0., 0.13, 0.57, 1.] {
            for v in [0., 0.29, 0.71, 1.] {
                let p = r.surface.evaluate(u, v).unwrap().point;
                let w = if rational {
                    1. + 0.1 * u + 0.05 * v
                } else {
                    1.
                };
                let expected = [u / w, v / w, (u * u / 4. + v * v / 10.) / w];
                for k in 0..3 {
                    assert!((p[k] - expected[k]).abs() < 1e-10);
                }
            }
        }
        let mut wrong = traces;
        let refined =
            nurbs_core::surface_edit::elevate(&original, nurbs_core::surface::Axis::U, 3).unwrap();
        let refined =
            nurbs_core::surface_edit::insert(&refined, nurbs_core::surface::Axis::U, 4., 1)
                .unwrap();
        let knot_cut = corner_blend::g2_traces(
            [&refined; 4],
            traces,
            o,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000,
        )
        .unwrap();
        assert!(knot_cut.complete && knot_cut.within_tolerance);
        wrong[0].along = [4., 9.];
        assert!(
            corner_blend::g2_traces(
                [&original; 4],
                wrong,
                o,
                [1e-7, 1e-5, 1e-4],
                100000,
                1000000
            )
            .is_err()
        );
        assert_eq!(original, before);
    }
}
#[test]
fn flat_g2_boundary_cannot_hide_a_folded_interior() {
    use nurbs_core::{corner_blend::Trace, surface::Axis, surface_edit};
    let mut s = supports(false);
    for surface in &mut s {
        for p in surface.control_points.iter_mut().flatten() {
            p[2] = 0.;
        }
    }
    let o = options();
    let base = corner_blend::g2(s.each_ref(), o, [1e-7, 1e-5, 1e-4], 100000).unwrap();
    assert!(base.admitted);
    let candidate = surface_edit::elevate(&base.surface, Axis::U, 6).unwrap();
    let mut candidate = surface_edit::elevate(&candidate, Axis::V, 6).unwrap();
    // B^6_3(u) B^6_3(v) has zero position, first and second jets
    // on all four sides, while the interior x derivative reverses sign.
    candidate.control_points[3][3][0] += 8.;
    let (du, _) = candidate
        .evaluate(0.75, 0.5)
        .unwrap()
        .first_derivatives()
        .unwrap();
    assert!(du[0] < -1.);
    let traces = [
        Trace {
            at: 1.,
            along: [0., 1.],
        },
        Trace {
            at: 0.,
            along: [0., 1.],
        },
        Trace {
            at: 1.,
            along: [0., 1.],
        },
        Trace {
            at: 0.,
            along: [0., 1.],
        },
    ];
    let result = corner_blend::inspect_candidate(
        &candidate,
        s.each_ref(),
        traces,
        o,
        [1e-7, 1e-5, 1e-4],
        100000,
        1000000,
    )
    .unwrap();
    assert!(result.complete && result.within_tolerance);
    assert!(!result.interior.certified && !result.admitted);
    let mut bounded = o;
    bounded.max_controls = 36;
    assert!(
        corner_blend::inspect_candidate(
            &candidate,
            s.each_ref(),
            traces,
            bounded,
            [1e-7, 1e-5, 1e-4],
            100000,
            1000000
        )
        .is_err()
    );
    assert!(
        result.cells <= 100000
            && result.exact_work <= 1000000
            && result.coefficient_work <= 2000000
    );
}
