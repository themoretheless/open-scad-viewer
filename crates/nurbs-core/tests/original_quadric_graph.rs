use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphContacts,
    },
    surface::Surface,
};
fn saddle() -> (Surface, [[f64; 4]; 4]) {
    (
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![-2., -2., 2., 2.],
            knots_v: vec![10., 10., 14., 14.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        },
        [
            [0., 0.5, 0., 0.],
            [0.5, 0., 0., 0.],
            [0., 0., 0., -0.5],
            [0., 0., -0.5, 0.],
        ],
    )
}
fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
fn inspect(c: &Curve, s: &Surface, q: [[f64; 4]; 4]) -> OriginalQuadricGraphContacts {
    original_quadric_graph_contacts(c, s, q, &[], 1e-9, 10, 1_000_000).unwrap()
}
fn present(r: &OriginalQuadricGraphContacts, expected: &[(f64, f64, f64, usize)]) {
    assert!(r.target_graph_proven && r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), expected.len());
    for (c, (t, u, v, m)) in r.contacts.iter().zip(expected) {
        for (axis, x) in [*t, *u, *v].into_iter().enumerate() {
            assert!(
                c.parameter_box[axis][0] <= x && x <= c.parameter_box[axis][1],
                "{c:?} vs {x}"
            );
        }
        assert_eq!(c.multiplicities[0], *m);
    }
    assert_eq!(
        r.exact_work,
        r.target_membership.work_used + r.carrier.as_ref().unwrap().exact_work
    );
}
#[test]
fn saddle_transverse_and_tangent_roots_are_admitted_on_the_actual_patch() {
    let (s, q) = saddle();
    let r = inspect(&line([0., 0.5, 0.25], [1., 0.5, 0.25]), &s, q);
    present(&r, &[(4., 0., 12., 1)]);
    let r = inspect(&line([0., 0., -0.25], [1., 1., 0.75]), &s, q);
    present(&r, &[(4., 0., 12., 2)]);
    assert_eq!(
        r.contacts[0].position_bounds,
        [[0.5, 0.5], [0.5, 0.5], [0.25, 0.25]]
    );
}
#[test]
fn two_finite_contacts_and_original_rational_clock_are_preserved() {
    let (s, q) = saddle();
    let mut c = line([0., 0., -0.1875], [1., 1., 0.8125]);
    present(&inspect(&c, &s, q), &[(3., -1., 11., 1), (5., 1., 13., 1)]);
    c.weights = vec![1., 3.];
    present(&inspect(&c, &s, q), &[(2.4, -1., 11., 1), (4., 1., 13., 1)]);
}
#[test]
fn carrier_root_outside_projection_proves_finite_patch_absence() {
    let (s, q) = saddle();
    let r = inspect(&line([2., 0., 0.5], [2., 1., 0.5]), &s, q);
    assert!(r.target_graph_proven && r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.carrier.as_ref().unwrap().roots.len(), 1);
}
fn paraboloid() -> (Surface, [[f64; 4]; 4]) {
    let mut s = saddle().0;
    s.degree_u = 2;
    s.degree_v = 2;
    s.knots_u = vec![-2., -2., -2., 2., 2., 2.];
    s.knots_v = vec![10., 10., 10., 14., 14., 14.];
    s.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    vec![
                        i as f64 / 2.,
                        j as f64 / 2.,
                        f64::from(i == 2) + f64::from(j == 2),
                    ]
                })
                .collect()
        })
        .collect();
    s.weights = vec![vec![1.; 3]; 3];
    (
        s,
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 0., -0.5],
            [0., 0., -0.5, 0.],
        ],
    )
}
#[test]
fn degree_two_paraboloid_has_irrational_interior_and_exact_boundary_tangent() {
    let (s, q) = paraboloid();
    let t = 0.5f64.sqrt();
    let r = inspect(&line([0., 0.5, 0.75], [1., 0.5, 0.75]), &s, q);
    present(&r, &[(2. + 4. * t, -2. + 4. * t, 12., 1)]);
    assert!(r.contacts[0].parameter_box[0][0] < r.contacts[0].parameter_box[0][1]);
    let r = inspect(&line([-0.5, 0.5, 0.25], [0.5, 0.5, 0.25]), &s, q);
    present(&r, &[(4., -2., 12., 2)]);
}
#[test]
fn rotated_dependent_axis_and_constant_weight_gauge_preserve_chart_ownership() {
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p.swap(0, 2);
        }
    }
    s.weights = vec![vec![7.; 2]; 2];
    let q = [
        [0., 0., 0., -0.5],
        [0., 0., 0.5, 0.],
        [0., 0.5, 0., 0.],
        [-0.5, 0., 0., 0.],
    ];
    let r = inspect(&line([0.25, 0.5, 0.], [0.25, 0.5, 1.]), &s, q);
    present(&r, &[(4., 0., 12., 1)]);
}
#[test]
fn perturbed_original_height_never_receives_the_unmodified_quadric_proof() {
    let (mut s, q) = saddle();
    s.control_points[1][1][2] = 1f64.next_up();
    let r = inspect(&line([0., 0.5, 0.25], [1., 0.5, 0.25]), &s, q);
    assert!(!r.target_graph_proven && !r.complete);
    assert!(r.carrier.is_none() && r.contacts.is_empty());
}
#[test]
fn unknown_sheets_collapsed_projection_and_nonconstant_weights_refuse_chart_admission() {
    let (mut s, q) = saddle();
    s.weights[1][1] = 2.;
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = 1.;
        }
    }
    let q = [[0.; 4], [0.; 4], [0., 0., 1., 0.], [0., 0., 0., -1.]];
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
    for row in &mut s.control_points {
        for p in row {
            p[1] = 0.;
        }
    }
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        saddle().1,
        &[],
        1e-9,
        10,
        100000
    )
    .is_err());
}
#[test]
fn target_and_source_predicates_share_one_total_work_budget() {
    let (s, q) = saddle();
    let c = line([0., 0., -0.25], [1., 1., 0.75]);
    let full = inspect(&c, &s, q);
    let zero = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).unwrap();
    assert!(!zero.target_graph_proven && !zero.complete);
    assert_eq!(zero.exact_work, 0);
    let budget = full.target_membership.work_used;
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, budget).unwrap();
    assert!(r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
    assert!(r.exact_work <= budget);
    assert_eq!(r.carrier.as_ref().unwrap().exact_work, 0);
}
#[test]
fn coincident_components_are_not_declared_complete_isolated_contact_coverage() {
    let (s, q) = saddle();
    let r = inspect(&line([0., 0., 0.], [1., 0., 0.]), &s, q);
    assert!(r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
    assert!(!r.carrier.as_ref().unwrap().unresolved_spans.is_empty());
}

#[test]
fn nonsingleton_boundary_root_is_admitted_from_original_active_control_identity() {
    let (s, q) = saddle();
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: [1., -1., 1.]
            .into_iter()
            .map(|z| vec![0., 0.5, z])
            .collect(),
        weights: vec![1., 2., 4.],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    present(&r, &[(2. + 4. / 3., -2., 12., 2)]);
    assert!(r.contacts[0].parameter_box[0][0] < r.contacts[0].parameter_box[0][1]);
}

#[test]
fn nonproportional_algebraic_boundary_membership_preserves_explicit_ambiguity() {
    let (s, q) = saddle();
    // F=X*Y-Z=(1-3t)^2, X=1-3t. At t=1/3 the point is on
    // the edge, but neither midpoint evaluation nor a control-hull clipping
    // can prove that. A further quadric-root coordinate sign proof is needed.
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![
            vec![1., 0.5, -0.5],
            vec![0., 0.5, 1.],
            vec![-1., 0.5, -0.5],
            vec![-2., 0.5, -5.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = inspect(&c, &s, q);
    assert!(r.target_graph_proven && !r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.ambiguous_parameters.len(), 1);
    assert!(r.carrier.as_ref().unwrap().complete);
}

#[test]
fn invalid_source_and_nonfinite_quadric_refuse_before_target_work() {
    let (s, q) = saddle();
    let mut c = line([0., 0., 0.], [1., 1., 1.]);
    c.periodic = true;
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    c.periodic = false;
    for p in &mut c.control_points {
        p.pop();
    }
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    let c = Curve {
        degree: 1,
        knots: vec![2., 2., 4., 4., 6., 6.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 1., 1.],
            vec![2., 2., 2.],
            vec![3., 3., 3.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 0).is_err());
    let mut q = q;
    q[0][0] = f64::INFINITY;
    assert!(original_quadric_graph_contacts(
        &line([0., 0., 0.], [1., 1., 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        0
    )
    .is_err());
}

#[test]
fn finite_graph_proof_excludes_a_different_quadric_component_and_refuses_singular_sheet_selection()
{
    let (mut s, _) = saddle();
    for row in &mut s.control_points {
        for p in row {
            p[2] = p[1];
        }
    }
    // F=(X+1)*(Z-Y). The represented target Z=Y is the unique branch
    // above X in [0,1]. The extra carrier component X=-1 must not be
    // mistaken for a finite target point.
    let q = [
        [0., -1., 1., 0.],
        [0., 0., 0., -1.],
        [0., 0., 0., 1.],
        [0., 0., 0., 0.],
    ];
    let r = inspect(&line([-2., 0.5, -1.], [1., 0.5, 1.]), &s, q);
    present(&r, &[(5., -1., 12., 1)]);
    assert_eq!(r.carrier.as_ref().unwrap().roots.len(), 2);
    // Extending this target to X=-1 loses the unique-implicit-branch proof.
    for p in &mut s.control_points[0] {
        p[0] = -1.;
    }
    assert!(original_quadric_graph_contacts(
        &line([-2., 0.5, -1.], [1., 0.5, 1.]),
        &s,
        q,
        &[],
        1e-9,
        10,
        1_000_000
    )
    .is_err());
}
