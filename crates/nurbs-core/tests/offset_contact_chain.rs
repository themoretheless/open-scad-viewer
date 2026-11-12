use nurbs_core::{
    surface::Surface,
    surface_offset::contact_curve::chain::{self, Limits, Span},
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn planes() -> [Surface; 2] {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    [a, b]
}
fn switches() -> [Span; 2] {
    [
        Span {
            owner: 0,
            axis: 0,
            drive: [0.2, 0.5],
            free: [0.1, 0.3],
            other: [[0.3, 0.6], [0.1, 0.3]],
            reversed: false,
        },
        Span {
            owner: 1,
            axis: 0,
            drive: [0.5000000001, 0.6],
            free: [0.1, 0.3],
            other: [[0.4, 0.9], [0.1, 0.3]],
            reversed: false,
        },
    ]
}
fn joined(c: &chain::Chain, l: Limits) {
    for (index, pair) in c.pieces().windows(2).enumerate() {
        let a = &pair[0];
        let b = &pair[1];
        let end = if a.span().reversed {
            0
        } else {
            a.world().control_points.len() - 1
        };
        let start = if b.span().reversed {
            b.world().control_points.len() - 1
        } else {
            0
        };
        assert_eq!(
            a.world().control_points[end],
            b.world().control_points[start]
        );
        for s in 0..2 {
            let a = &a.source_uv()[s];
            let b = &b.source_uv()[s];
            let end = if pair[0].span().reversed {
                0
            } else {
                a.control_points.len() - 1
            };
            let start = if pair[1].span().reversed {
                b.control_points.len() - 1
            } else {
                0
            };
            assert_eq!(a.control_points[end], b.control_points[start]);
        }
        assert!(c.joins()[index].world_error_upper <= 1e-7);
        assert!(c.joins()[index].uv_error_upper.iter().all(|x| *x <= 1e-7));
    }
    for p in c.pieces() {
        assert_eq!(p.uv_proofs().len(), 2);
        assert!(
            p.uv_proofs()
                .iter()
                .all(|r| r.correspondence_proven && r.visited <= l.final_cells_per_curve)
        );
        assert!(
            p.world_proof().world_correspondence_proven
                && p.world_proof().error_upper_mm.unwrap() <= 1e-7
        );
        assert!(p.world_proof().visited <= l.final_cells_per_curve);
        assert!(p.uv_fitting().root_queries <= l.uv_fit.root_queries);
        assert!(p.world_fitting().root_queries <= l.world_fit.root_queries);
        assert_eq!(p.lift_proofs().len(), 2);
        assert!(p.lift_proofs().iter().all(|r| r.lift_correspondence_proven
            && r.error_upper_mm.unwrap() <= 1e-7
            && r.visited <= l.final_cells_per_curve));
    }
    c.reverify().unwrap();
}
#[test]
fn source_owner_switch_binds_literal_uv_and_world_nets_with_complete_gap_bound() {
    let s = planes();
    let saved = s.clone();
    let l = Limits::default();
    let c = chain::build(s.each_ref(), [0.2, -0.2], &switches(), 1e-7, 1e-7, l).unwrap();
    assert_eq!(c.pieces().len(), 2);
    assert_eq!(c.joins().len(), 1);
    assert!(c.joins()[0].clock_derivative[0] > 0.);
    assert!(c.joins()[0].source_root_window.first_uv[0][1] > 0.5);
    joined(&c, l);
    assert_eq!(s, saved);
}
#[test]
fn reversed_traversal_and_nonunit_clocks_keep_exact_original_endpoint_ownership() {
    let mut s = planes();
    s[0].knots_u = vec![2., 2., 7., 7.];
    s[1].knots_u = vec![-3., -3., 4., 4.];
    let spans = [
        Span {
            owner: 0,
            axis: 0,
            drive: [4., 6.],
            free: [0.1, 0.3],
            other: [[-0.5, 1.5], [0.1, 0.3]],
            reversed: true,
        },
        Span {
            owner: 0,
            axis: 0,
            drive: [3., 4.],
            free: [0.1, 0.3],
            other: [[-0.5, 1.5], [0.1, 0.3]],
            reversed: true,
        },
    ];
    let l = Limits::default();
    let c = chain::build(s.each_ref(), [0.2, -0.2], &spans, 1e-7, 1e-7, l).unwrap();
    assert_eq!(c.pieces()[0].source_uv()[0].domain(), [4., 6.]);
    assert_eq!(c.joins()[0].endpoint_roots[0].first_uv[0], [4.; 2]);
    joined(&c, l);
}
fn curved() -> [Surface; 2] {
    let mut a = plane();
    a.degree_u = 2;
    a.knots_u = vec![0., 0., 0., 1., 1., 1.];
    a.control_points = [[1., 0.], [1., 1.], [0., 1.]]
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
        .to_vec();
    a.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
        .map(|w| vec![w; 2])
        .to_vec();
    let mut b = plane();
    b.control_points = vec![
        vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        vec![vec![0., 2., 0.], vec![0.5, 2., 1.]],
    ];
    [a, b]
}
#[test]
fn nonlinear_source_clock_switch_preserves_actual_piece_lifts_after_binding() {
    let s = curved();
    let r = nurbs_core::offset_contact_predictor::propose(
        nurbs_core::offset_contact_pcurve::OffsetFaces {
            surfaces: s.each_ref(),
            distances: [0.1; 2],
        },
        [[0.92, 0.55], [0.12, 0.555]],
        1,
        1e-12,
        24,
        0.001,
        128,
    )
    .unwrap();
    assert!(r.residual_mm < 1e-12);
    let spans = [
        Span {
            owner: 0,
            axis: 1,
            drive: [0.35, 0.55],
            free: [0.88, 0.96],
            other: [[0.07, 0.22], [0.3, 0.6]],
            reversed: false,
        },
        Span {
            owner: 1,
            axis: 1,
            drive: [r.second_uv[1] + 1e-9, 0.8],
            free: [0.07, 0.22],
            other: [[0.88, 0.96], [0.5, 0.9]],
            reversed: false,
        },
    ];
    let l = Limits::default();
    let c = chain::build(s.each_ref(), [0.1; 2], &spans, 1e-7, 1e-7, l).unwrap();
    joined(&c, l);
}
#[test]
fn oversized_gap_backtracking_or_inconsistent_traversal_publish_no_chain() {
    let s = planes();
    let saved = s.clone();
    for (start, reverse) in [(0.5001, false), (0.49, false), (0.5000000001, true)] {
        let mut spans = switches();
        spans[1].drive[0] = start;
        spans[1].reversed = reverse;
        assert!(
            chain::build(
                s.each_ref(),
                [0.2, -0.2],
                &spans,
                1e-7,
                1e-7,
                Limits::default()
            )
            .is_err()
        );
    }
    assert_eq!(s, saved);
}
#[test]
fn every_independent_work_limit_is_checked_without_partial_publication() {
    let s = planes();
    let spans = switches();
    for kind in 0..6 {
        let mut l = Limits::default();
        match kind {
            0 => l.max_pieces = 1,
            1 => l.uv_fit.root_queries = 1,
            2 => l.world_fit.coefficient_work = 1,
            3 => l.final_cells_per_curve = 0,
            4 => l.refinements = 17,
            _ => l.spans = 0,
        }
        assert!(chain::build(s.each_ref(), [0.2, -0.2], &spans, 1e-7, 1e-7, l).is_err());
    }
    let mut spans = spans;
    spans[0].owner = 2;
    assert!(
        chain::build(
            s.each_ref(),
            [0.2, -0.2],
            &spans,
            1e-7,
            1e-7,
            Limits::default()
        )
        .is_err()
    );
}

#[test]
fn individually_qualified_coincident_world_sheets_cannot_form_a_false_uv_join() {
    let mut a = plane();
    for p in a.control_points.iter_mut().flatten() {
        p[0] *= 0.6;
    }
    let mut b = plane();
    b.degree_u = 3;
    b.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    b.control_points = [0., 2., -1., 1.]
        .map(|x| vec![vec![x, 0.5, 0.], vec![x, 0.5, 1.]])
        .to_vec();
    b.weights = vec![vec![1.; 2]; 4];
    let spans = [
        Span {
            owner: 0,
            axis: 0,
            drive: [0.995, 0.997],
            free: [0.28, 0.32],
            other: [[0.149, 0.154], [0.19, 0.21]],
            reversed: false,
        },
        Span {
            owner: 0,
            axis: 0,
            drive: [0.997, 0.999],
            free: [0.28, 0.32],
            other: [[0.915, 0.92], [0.19, 0.21]],
            reversed: false,
        },
    ];
    for span in spans {
        chain::build([&a, &b], [0.2; 2], &[span], 1e-7, 1e-7, Limits::default()).unwrap();
    }
    assert!(chain::build([&a, &b], [0.2; 2], &spans, 1e-7, 1e-7, Limits::default()).is_err());
}
