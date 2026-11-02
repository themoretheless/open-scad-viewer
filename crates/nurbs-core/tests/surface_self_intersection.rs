use nurbs_core::{
    surface::Surface,
    surface_self_intersection::{self, Status},
};
fn graph() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 1.], vec![0.5, 1., 1.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn global_graph_and_rotated_graph_are_proven_without_mesh_samples() {
    let mut s = graph();
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Absent
    );
    for p in s.control_points.iter_mut().flatten() {
        p.swap(1, 2);
    }
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Absent
    );
}
#[test]
fn exact_fold_has_distinct_interior_witness_not_boundary_alias() {
    let mut s = graph();
    s.control_points[2] = s.control_points[0].clone();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.witness.unwrap();
    assert_ne!(w[0], w[1]);
    // x=u(1-u), y=v, z=2u(1-u): analytical coincidence at 1/4 and 3/4.
    for uv in w {
        let p = s.evaluate(uv[0], uv[1]).unwrap().point;
        assert!((p[0] - 0.1875).abs() < 1e-14);
        assert!((p[2] - 0.375).abs() < 1e-14);
    }
}
#[test]
fn incomplete_coverage_and_invalid_input_never_claim_absence() {
    let mut s = graph();
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    assert_eq!(
        surface_self_intersection::inspect(&s, 1).unwrap().status,
        Status::Unresolved
    );
    assert!(surface_self_intersection::inspect(&s, 0).is_err());
    s.weights[0][0] = f64::NAN;
    assert!(surface_self_intersection::inspect(&s, 100).is_err());
}

fn multispan_fold() -> Surface {
    let mut s = graph();
    s.knots_u = vec![-2., -2., -2., -1., 1., 2., 2., 2.];
    let a = s.control_points[0].clone();
    let b = s.control_points[1].clone();
    let c = s.control_points[2].clone();
    s.control_points = vec![a.clone(), b.clone(), c, b, a];
    s.weights = vec![
        vec![1.; 2],
        vec![2.; 2],
        vec![3.; 2],
        vec![2.; 2],
        vec![1.; 2],
    ];
    s
}

#[test]
fn rational_multispan_reflection_proves_interior_coincidence_in_both_axes() {
    for transpose in [false, true] {
        let mut s = multispan_fold();
        if transpose {
            s.control_points = (0..2)
                .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
                .collect();
            s.weights = (0..2)
                .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
                .collect();
            std::mem::swap(&mut s.degree_u, &mut s.degree_v);
            std::mem::swap(&mut s.knots_u, &mut s.knots_v);
        }
        let r = surface_self_intersection::inspect(&s, 100).unwrap();
        assert_eq!(r.status, Status::Present);
        assert_eq!(r.reflected_axis, Some(usize::from(transpose)));
        let w = r.witness.unwrap();
        let point = |uv: [f64; 2]| {
            let u = if transpose { uv[0] } else { -2. + 4. * uv[0] };
            let v = if transpose { -2. + 4. * uv[1] } else { uv[1] };
            s.evaluate(u, v).unwrap().point
        };
        let (a, b) = (point(w[0]), point(w[1]));
        for axis in 0..3 {
            assert!((a[axis] - b[axis]).abs() < 1e-13);
        }
    }
}

#[test]
fn rounded_knot_symmetry_does_not_prove_a_fold() {
    let mut s = multispan_fold();
    s.knots_u = vec![0., 0., 0., 0.25, 0.75_f64.next_up(), 1., 1., 1.];
    assert_eq!(0.25 + 0.75_f64.next_up(), 1.); // rounded equality is insufficient
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Unresolved
    );
}

#[test]
fn asymmetric_weights_do_not_prove_a_fold() {
    let mut s = multispan_fold();
    s.weights[0][0] = 1.01;
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Unresolved
    );
}

#[test]
fn discontinuous_reflected_charts_do_not_admit_an_endpoint_witness() {
    let mut s = graph();
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.25, 0.25, 0.75, 0.75, 1., 1.];
    let a = s.control_points[0].clone();
    let b = s.control_points[1].clone();
    let c = s.control_points[2].clone();
    s.control_points = vec![a.clone(), b.clone(), c.clone(), c, b, a];
    s.weights = vec![vec![1.; 2]; 6];
    let error = surface_self_intersection::inspect(&s, 100).unwrap_err();
    assert_eq!(error.code, "NURBS_INVALID_INPUT");
    assert!(error.message.contains("Interior multiplicity"));
}

fn local_fold() -> Surface {
    let mut s = graph();
    s.knots_u = vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.];
    let a = s.control_points[0].clone();
    let b = s.control_points[1].clone();
    let c = s.control_points[2].clone();
    let mut d = c.clone();
    for p in &mut d {
        p[0] = 2.;
    }
    s.control_points = vec![a.clone(), b, a, c, d];
    s.weights = vec![
        vec![1.; 2],
        vec![2.; 2],
        vec![1.; 2],
        vec![1.; 2],
        vec![1.; 2],
    ];
    s
}

#[test]
fn local_fold_is_detected_without_global_reflection() {
    let s = local_fold();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.125, 0.5], [0.375, 0.5]]));
    let a = s.evaluate(0.125, 0.5).unwrap().point;
    let b = s.evaluate(0.375, 0.5).unwrap().point;
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 1e-13);
    }
}

#[test]
fn local_fold_does_not_round_an_unrepresentable_normalized_witness() {
    let mut s = local_fold();
    s.knots_u = vec![0., 0., 0., 1., 1., 3., 3., 3.];
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Unresolved
    );
}

#[test]
fn near_local_fold_is_not_an_exact_coincidence() {
    let mut s = local_fold();
    s.control_points[2][0][0] = 1e-12;
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}

#[test]
fn local_v_fold_uses_original_parameter_coordinates() {
    let mut s = local_fold();
    s.control_points = (0..2)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.reflected_axis, Some(1));
    assert_eq!(r.witness, Some([[0.5, 0.125], [0.5, 0.375]]));
}

#[test]
fn local_search_budget_does_not_skip_unvisited_spans() {
    let mut s = local_fold();
    s.control_points.reverse();
    s.weights.reverse();
    assert_eq!(
        surface_self_intersection::inspect(&s, 1).unwrap().status,
        Status::Unresolved
    );
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.625, 0.5], [0.875, 0.5]]));
}
