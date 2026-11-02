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
fn asymmetric_global_weights_can_still_have_a_proven_local_fold() {
    let mut s = multispan_fold();
    s.weights[0][0] = 1.01;
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.375, 0.5], [0.625, 0.5]]));
    let a = s.evaluate(-0.5, 0.5).unwrap().point;
    let b = s.evaluate(0.5, 0.5).unwrap().point;
    for i in 0..3 {
        assert!((a[i] - b[i]).abs() < 1e-13);
    }
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
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert!(r.witness.is_none());
    let exact = r.exact_witness.unwrap();
    assert_eq!(
        exact[0][0],
        num_rational::BigRational::new(1.into(), 12.into())
    );
    assert_eq!(
        exact[1][0],
        num_rational::BigRational::new(1.into(), 4.into())
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

fn non_bernstein_fold() -> Surface {
    let mut s = graph();
    s.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
    s.control_points = [0., 1., -3., 4.]
        .into_iter()
        .map(|x| vec![vec![x, 0., 2. * x], vec![x, 1., 2. * x]])
        .collect();
    s.weights = [1., 1.5, 0.5, 1.].into_iter().map(|w| vec![w; 2]).collect();
    s
}
#[test]
fn exact_original_bspline_basis_proves_a_fold_without_knot_insertion() {
    let s = non_bernstein_fold();
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
fn original_bspline_proof_rejects_one_ulp_pole_and_weight_changes() {
    for weight in [false, true] {
        let mut s = non_bernstein_fold();
        if weight {
            s.weights[2][0] = 0.5_f64.next_up();
        } else {
            s.control_points[2][0][0] = (-3_f64).next_up();
        }
        assert_ne!(
            surface_self_intersection::inspect(&s, 100).unwrap().status,
            Status::Present
        );
    }
}
#[test]
fn exact_original_bspline_basis_also_handles_v_folds() {
    let mut s = non_bernstein_fold();
    s.control_points = (0..2)
        .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.5, 0.125], [0.5, 0.375]]));
}

#[test]
fn exact_cubic_span_recurrence_detects_a_non_global_fold() {
    let mut s = graph();
    s.degree_u = 3;
    s.knots_u = vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.];
    s.control_points = [0., 1., 1., -3., 4.]
        .into_iter()
        .map(|x| vec![vec![x, 0., 2. * x], vec![x, 1., 2. * x]])
        .collect();
    s.weights = vec![vec![1.; 2]; 5];
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.125, 0.5], [0.375, 0.5]]));
    for u in [0.125, 0.375] {
        assert!((s.evaluate(u, 0.5).unwrap().point[0] - 0.5625).abs() < 1e-13);
    }
}

fn projective_fold() -> Surface {
    let mut s = graph();
    s.degree_u = 3;
    s.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    s.control_points = [0., 1.5, 4., 0.]
        .into_iter()
        .map(|x| vec![vec![x, 0., 2. * x], vec![x, 1., 2. * x]])
        .collect();
    s.weights = [1., 2., 3., 4.].into_iter().map(|w| vec![w; 2]).collect();
    s
}
#[test]
fn asymmetric_homogeneous_gauge_still_proves_rational_fold() {
    let s = projective_fold();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.25, 0.5], [0.75, 0.5]]));
    // X/W = 9u(1-u), W=1+3u. Both X and W are asymmetric.
    for u in [0.25, 0.75] {
        let p = s.evaluate(u, 0.5).unwrap().point;
        assert!((p[0] - 1.6875).abs() < 1e-13);
        assert!((p[1] - 0.5).abs() < 1e-13);
    }
}
#[test]
fn independent_row_gauges_do_not_prove_a_tensor_surface_fold() {
    let mut s = projective_fold();
    let original = s.clone();
    for i in 0..4 {
        s.weights[i][1] = original.weights[3 - i][1];
        s.control_points[i][1] = original.control_points[3 - i][1].clone();
    }
    let a = s.evaluate(0.25, 0.5).unwrap().point;
    let b = s.evaluate(0.75, 0.5).unwrap().point;
    assert!((a[0] - b[0]).abs() < 1e-13); // each row has the same symmetric x
    assert!((a[1] - b[1]).abs() > 0.1); // their mixture has different y
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}
#[test]
fn projective_fold_rejects_one_ulp_changes_to_source_homogeneous_data() {
    let mut s = projective_fold();
    s.weights[1][1] = 2_f64.next_up();
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}

#[test]
fn rational_projective_fold_is_proven_on_v_axis() {
    let mut s = projective_fold();
    s.control_points = (0..2)
        .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.witness, Some([[0.5, 0.25], [0.5, 0.75]]));
}

#[test]
fn non_dyadic_v_witness_is_preserved_exactly() {
    use num_rational::BigRational as Q;
    let mut s = local_fold();
    s.knots_u = vec![-2., -2., -2., -1., -1., 1., 1., 1.];
    s.control_points = (0..2)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert!(r.witness.is_none());
    let w = r.exact_witness.unwrap();
    assert_eq!(
        w[0],
        [Q::new(1.into(), 2.into()), Q::new(1.into(), 12.into())]
    );
    assert_eq!(
        w[1],
        [Q::new(1.into(), 2.into()), Q::new(1.into(), 4.into())]
    );
}
#[test]
fn binary64_witness_is_also_backed_by_an_exact_pair() {
    use num_traits::FromPrimitive;
    let r = surface_self_intersection::inspect(&projective_fold(), 100).unwrap();
    let f = r.witness.unwrap();
    let q = r.exact_witness.unwrap();
    for i in 0..2 {
        for j in 0..2 {
            assert_eq!(
                num_rational::BigRational::from_f64(f[i][j]).unwrap(),
                q[i][j]
            );
        }
    }
}
#[test]
fn absent_and_unresolved_reports_do_not_carry_a_witness() {
    let absent = surface_self_intersection::inspect(&graph(), 100).unwrap();
    assert_eq!(absent.status, Status::Absent);
    assert!(absent.witness.is_none() && absent.exact_witness.is_none());
    let mut s = multispan_fold();
    s.knots_u = vec![0., 0., 0., 0.25, 0.75_f64.next_up(), 1., 1., 1.];
    let unresolved = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(unresolved.status, Status::Unresolved);
    assert!(unresolved.witness.is_none() && unresolved.exact_witness.is_none());
}

fn crossing_strip() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 3., 3.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[0., 0.], [1., 1.], [0., 1.], [1., 0.]]
            .into_iter()
            .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn spatial_linear_span_crossing_has_exact_nonreflection_witness() {
    use num_rational::BigRational as Q;
    let s = crossing_strip();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.reflected_axis, None);
    let w = r.exact_witness.unwrap();
    assert_eq!(
        w[0],
        [Q::new(1.into(), 6.into()), Q::new(1.into(), 2.into())]
    );
    assert_eq!(
        w[1],
        [Q::new(5.into(), 6.into()), Q::new(1.into(), 2.into())]
    );
    for u in [0.5, 2.5] {
        let p = s.evaluate(u, 0.5).unwrap().point;
        assert_eq!(p, [0.5, 0.5, 1.]);
    }
}
#[test]
fn rational_linear_crossing_uses_weights_in_parameter_clock() {
    use num_rational::BigRational as Q;
    let mut s = crossing_strip();
    s.weights = vec![vec![1.; 2], vec![3.; 2], vec![2.; 2], vec![1.; 2]];
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][0], Q::new(1.into(), 12.into()));
    assert_eq!(w[1][0], Q::new(8.into(), 9.into()));
    for u in [0.25, 8. / 3.] {
        let p = s.evaluate(u, 0.5).unwrap().point;
        assert!((p[0] - 0.5).abs() < 1e-13 && (p[1] - 0.5).abs() < 1e-13);
    }
}
#[test]
fn shifted_linear_sheets_contact_on_different_fibers() {
    let mut s = crossing_strip();
    for i in 2..4 {
        for j in 0..2 {
            s.control_points[i][j][2] += 0.01;
        }
    }
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert_eq!(c.proof, surface_self_intersection::CrossingProof::CurveSurface);
    assert!(c.parameter_boxes[0][1][1] < c.parameter_boxes[1][1][0]
        || c.parameter_boxes[1][1][1] < c.parameter_boxes[0][1][0]);
}
#[test]
fn adjacent_span_endpoints_are_not_interior_self_intersections() {
    let mut s = crossing_strip();
    s.control_points[3] = s.control_points[0].clone();
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}

#[test]
fn crossing_survives_rational_quadratic_transverse_basis() {
    let mut s = crossing_strip();
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = s
        .control_points
        .iter()
        .map(|row| {
            let [x, y, _] = row[0].as_slice() else {
                unreachable!()
            };
            [0., 1., 2.].into_iter().map(|z| vec![*x, *y, z]).collect()
        })
        .collect();
    s.weights = vec![vec![1., 2., 3.]; 4];
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    for u in [0.5, 2.5] {
        let p = s.evaluate(u, 0.5).unwrap().point;
        assert_eq!(p, [0.5, 0.5, 1.25]);
    }
}
#[test]
fn exact_linear_crossing_is_also_found_on_v_axis() {
    let mut s = crossing_strip();
    s.control_points = (0..2)
        .map(|j| (0..4).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..4).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][1], num_rational::BigRational::new(1.into(), 6.into()));
    assert_eq!(w[1][1], num_rational::BigRational::new(5.into(), 6.into()));
}
#[test]
fn exact_crossing_pair_budget_is_enforced() {
    let mut s = crossing_strip();
    s.knots_u = vec![0., 0., 1., 2., 3., 4., 4.];
    s.control_points = [[0., 0.], [1., 1.], [2., 2.], [0., 1.], [1., 0.]]
        .into_iter()
        .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
        .collect();
    s.weights = vec![vec![1.; 2]; 5];
    assert_eq!(
        surface_self_intersection::inspect(&s, 1).unwrap().status,
        Status::Unresolved
    );
    assert_eq!(
        surface_self_intersection::inspect(&s, 2).unwrap().status,
        Status::Present
    );
}

fn overlap_strip() -> Surface {
    let mut s = crossing_strip();
    s.control_points = [[0., 0.], [3., 0.], [1., 1.], [1., 0.], [2., 0.]]
        .into_iter()
        .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
        .collect();
    s.knots_u = vec![0., 0., 1., 2., 3., 4., 4.];
    s.weights = vec![vec![1.; 2]; 5];
    s
}
#[test]
fn positive_collinear_overlap_has_strict_interior_witness() {
    use num_rational::BigRational as Q;
    let s = overlap_strip();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][0], Q::new(1.into(), 8.into()));
    assert_eq!(w[1][0], Q::new(7.into(), 8.into()));
    assert_eq!(
        s.evaluate(0.5, 0.5).unwrap().point,
        s.evaluate(3.5, 0.5).unwrap().point
    );
}
#[test]
fn adjacent_backtracking_overlap_is_not_dismissed_as_a_shared_endpoint() {
    use num_rational::BigRational as Q;
    let mut s = crossing_strip();
    s.control_points = [[0., 0.], [3., 0.], [1., 0.]]
        .into_iter()
        .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
        .collect();
    s.knots_u = vec![0., 0., 1., 2., 2.];
    s.weights = vec![vec![1.; 2]; 3];
    let r = surface_self_intersection::inspect(&s, 1).unwrap();
    assert_eq!(r.status, Status::Present);
    assert_eq!(r.reflected_axis, None);
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][0], Q::new(1.into(), 3.into()));
    assert_eq!(w[1][0], Q::new(3.into(), 4.into()));
}
#[test]
fn parallel_offset_intervals_are_not_collinear_overlap() {
    let mut s = overlap_strip();
    for i in 3..5 {
        for j in 0..2 {
            s.control_points[i][j][2] += 1e-12;
        }
    }
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}
#[test]
fn rational_overlap_uses_original_weighted_clocks() {
    use num_rational::BigRational as Q;
    let mut s = overlap_strip();
    s.weights[1] = vec![3.; 2];
    s.weights[3] = vec![2.; 2];
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][0], Q::new(1.into(), 16.into()));
    assert_eq!(w[1][0], Q::new(11.into(), 12.into()));
}

#[test]
fn collinear_endpoint_join_is_not_a_positive_overlap() {
    let mut s = crossing_strip();
    s.control_points = [[0., 0.], [1., 0.], [2., 0.]]
        .into_iter()
        .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
        .collect();
    s.knots_u = vec![0., 0., 1., 2., 2.];
    s.weights = vec![vec![1.; 2]; 3];
    assert_ne!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Present
    );
}
#[test]
fn collinear_overlap_is_proven_along_v_too() {
    use num_rational::BigRational as Q;
    let mut s = overlap_strip();
    s.control_points = (0..2)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.exact_witness.unwrap();
    assert_eq!(w[0][1], Q::new(1.into(), 8.into()));
    assert_eq!(w[1][1], Q::new(7.into(), 8.into()));
}

fn nonlinear_crossing_strip() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [[-1., -1.], [0., 0.], [1., 1.], [1., -1.], [-1., 1.]]
            .into_iter()
            .map(|[x, y]| vec![vec![x, y, 0.], vec![x, y, 2.]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn nonlinear_original_spans_have_certified_distinct_uv_boxes() {
    let r = surface_self_intersection::inspect(&nonlinear_crossing_strip(), 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    assert!(r.witness.is_none() && r.exact_witness.is_none());
    let c = r.certified_crossing.unwrap();
    for (i, t) in [5. / 18., 5. / 6.].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1]);
        assert_eq!(c.parameter_boxes[i][1], [0.5, 0.5]);
    }
    for (i, x) in [1. / 9., 1. / 9., 1.].into_iter().enumerate() {
        assert!(c.position_bounds[i][0] <= x && x <= c.position_bounds[i][1]);
    }
    assert!(c.cells > 0 && c.cells <= 10000);
}
#[test]
fn nonlinear_crossing_budget_cannot_be_relabelled_a_proof() {
    let r = surface_self_intersection::inspect(&nonlinear_crossing_strip(), 1).unwrap();
    assert_eq!(r.status, Status::Unresolved);
    assert!(r.certified_crossing.is_none());
}
#[test]
fn shifted_nonlinear_sheets_contact_on_different_fibers() {
    let mut s = nonlinear_crossing_strip();
    for i in 3..5 {
        for j in 0..2 {
            s.control_points[i][j][2] += 0.1;
        }
    }
    let r = surface_self_intersection::inspect(&s, 1000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert_eq!(c.proof, surface_self_intersection::CrossingProof::CurveSurface);
    assert!(c.parameter_boxes[0][1][1] < c.parameter_boxes[1][1][0]
        || c.parameter_boxes[1][1][1] < c.parameter_boxes[0][1][0]);
}

#[test]
fn certified_nonlinear_crossing_also_works_with_rational_weights() {
    let mut s = nonlinear_crossing_strip();
    s.weights = [1., 3., 1., 2., 1.]
        .into_iter()
        .map(|w| vec![w; 2])
        .collect();
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for i in 0..2 {
        assert!(c.position_bounds[i][0] <= 1. / 33. && 1. / 33. <= c.position_bounds[i][1]);
    }
}
#[test]
fn certified_nonlinear_crossing_also_has_original_v_parameter_boxes() {
    let mut s = nonlinear_crossing_strip();
    s.control_points = (0..2)
        .map(|j| (0..5).map(|i| s.control_points[i][j].clone()).collect())
        .collect();
    s.weights = (0..2)
        .map(|j| (0..5).map(|i| s.weights[i][j]).collect())
        .collect();
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for (i, t) in [5. / 18., 5. / 6.].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][1][0] <= t && t <= c.parameter_boxes[i][1][1]);
        assert_eq!(c.parameter_boxes[i][0], [0.5, 0.5]);
    }
}

#[test]
fn original_surface_bounds_admit_nonrepresentable_iso_controls() {
    let mut s = nonlinear_crossing_strip();
    s.degree_v = 2;
    s.knots_v = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = s
        .control_points
        .iter()
        .map(|row| {
            let x = row[0][0];
            let y = row[0][1];
            [0., 1., 2.].into_iter().map(|z| vec![x, y, z]).collect()
        })
        .collect();
    s.weights = vec![vec![1., 2., 2.]; 5]; // exact middle iso has z=8/7, not binary64
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert!(c.position_bounds[2][0] <= 8. / 7. && 8. / 7. <= c.position_bounds[2][1]);
}

#[path = "surface_self_intersection/distinct_spans.rs"]
mod distinct_spans;

#[path = "surface_self_intersection/within_span.rs"]
mod within_span;

#[path = "surface_self_intersection/oblique_plane.rs"]
mod oblique_plane;

#[path = "surface_self_intersection/original_iso.rs"]
mod original_iso;

#[path = "surface_self_intersection/iso_probe.rs"]
mod iso_probe;

#[path = "surface_self_intersection/automatic_probes.rs"]
mod automatic_probes;

#[path = "surface_self_intersection/spatial_windows.rs"]
mod spatial_windows;

#[path="surface_self_intersection/separate_iso_target.rs"]
mod separate_iso_target;
