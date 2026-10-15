use nurbs_core::{
    polynomial,
    surface::{Axis, Surface},
    surface_edit,
    surface_g2::{self, Orientation},
    surface_join::{Boundary, Decision},
};
fn patch(left: bool, curvature: f64, nonlinear: bool) -> Surface {
    let mut c = vec![vec![[0.; 3]; 2]; if nonlinear { 5 } else { 3 }];
    c[1][0][0] = if nonlinear { 2. } else { 1. };
    c[0][1][1] = 1.;
    if nonlinear {
        c[2][0][0] = 3.;
        c[1][0][1] = 1.;
        c[2][0][2] = 4. * curvature;
        c[3][0][2] = 12. * curvature;
        c[4][0][2] = 9. * curvature;
    } else {
        c[2][0][2] = curvature;
    }
    polynomial::parametric_surface(
        if left {
            [-1., 0., 0., 1.]
        } else {
            [0., 1., 0., 1.]
        },
        &c,
    )
    .unwrap()
}
#[test]
fn geometric_g2_allows_speed_shear_and_nonlinear_parameter_freedom() {
    let a = patch(true, 1., false);
    let b = patch(false, 1., true);
    let before = b.clone();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Same));
    assert!(r.regularity_proven);
    assert!(r.branches[0].tensor_upper.unwrap() <= 0.1);
    assert_eq!(b, before);
}
#[test]
fn geometric_g2_accepts_different_boundary_degrees_and_knot_layouts() {
    let a = patch(true, 1., false);
    let elevated = surface_edit::elevate(&patch(false, 1., true), Axis::V, 3).unwrap();
    let b = surface_edit::insert(&elevated, Axis::V, 0.37, 1).unwrap();
    let before = (a.clone(), b.clone());
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        5000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Same));
    assert!(r.regularity_proven);
    assert!(r.branches[0].tensor_upper.unwrap() <= 0.1);
    assert_eq!((a, b), before);
}
#[test]
fn opposite_normals_require_the_same_tensor_sign_branch() {
    let a = patch(true, 1., false);
    let b = surface_edit::reverse(&patch(false, 1., true), Axis::V).unwrap();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        true,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Opposite));
    assert!(r.branches[0].normal_lower > 0.02);
    assert!(r.branches[1].tensor_upper.unwrap() <= 0.1);
}
#[test]
fn g1_matching_planes_do_not_hide_a_curvature_gap() {
    let a = patch(true, 1., false);
    let b = patch(false, 2., false);
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(r.branches[0].tensor_lower > 1.9 && r.branches[0].tensor_lower <= 2.);
    assert!(r.branches[1].normal_lower > 0.02);
    assert!(r.orientation.is_none());
}
#[test]
fn unavailable_smoothness_regularity_and_budget_never_report_g2() {
    let a = patch(true, 1., false);
    let b = surface_edit::insert(&patch(false, 1., false), Axis::V, 0.5, 1).unwrap();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        31,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
    assert!(r.branches[0].tensor_upper.is_none());
    // Use a genuinely varying boundary normal. A shared straight seam with
    // constant curvature can now be proved in one cell by the graph enclosure.
    let varying=|left| {
        let mut coefficients=vec![vec![[0.;3];3];3];
        coefficients[1][0][0]=1.;coefficients[0][1][1]=1.;
        coefficients[2][0][2]=1.;coefficients[1][1][2]=1.;coefficients[0][2][2]=1.;
        polynomial::parametric_surface(if left {[-1.,0.,0.,1.]}else{[0.,1.,0.,1.]},&coefficients).unwrap()
    };
    let a=varying(true);
    let b=varying(false);
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        1e-12,
        0.02,
        0.1,
        1,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(
        surface_g2::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            0.02,
            -1.,
            100
        )
        .is_err()
    );
    assert!(
        surface_g2::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            0.02,
            0.1,
            0
        )
        .is_err()
    );
    let mut a = a;
    let mut b = b;
    for s in [&mut a, &mut b] {
        for row in &mut s.control_points {
            for p in row {
                p[1] = 0.;
            }
        }
    }
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        11,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
}

#[test]
fn nonaffine_boundary_relation_is_checked_on_original_surfaces() {
    // A: (u, v, u^2); B: (u, (v+v^2)/2, u^2).
    let mut ca = vec![vec![[0.; 3]; 3]; 3];
    ca[1][0][0] = 1.;
    ca[0][1][1] = 1.;
    ca[2][0][2] = 1.;
    let mut cb = ca.clone();
    cb[0][1][1] = 0.5;
    cb[0][2][1] = 0.5;
    let a = polynomial::parametric_surface([-1., 0., 0., 1.], &ca).unwrap();
    let b = polynomial::parametric_surface([0., 1., 0., 1.], &cb).unwrap();
    let before = (a.clone(), b.clone());
    let affine = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.03,
        0.05,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(affine.decision, Decision::CorrespondenceExceedsTolerance);
    // Candidate nodes approximate the inverse quadratic relation. Their
    // generation is not used as evidence; the entire relation is enclosed.
    let nodes: Vec<_> = (0..=32)
        .map(|i| {
            let v = i as f64 / 32.;
            [0.5 * (v + v * v), v]
        })
        .collect();
    let r = surface_g2::inspect_with_correspondence(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        &nodes,
        0.03,
        0.05,
        0.1,
        2000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert!(r.position_bounds[1] <= 0.03);
    let inferred = surface_g2::inspect_inferred(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        32,
        0.03,
        0.05,
        0.1,
        2000,
    )
    .unwrap();
    assert!(inferred.candidate_monotone);
    assert_eq!(
        inferred.verification.unwrap().decision,
        Decision::WithinTolerance
    );
    assert_eq!((a.clone(), b.clone()), before);
    assert!(
        surface_g2::inspect_with_correspondence(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            &[[0., 0.], [0.5, 0.7], [0.4, 0.8], [1., 1.]],
            0.03,
            0.05,
            0.1,
            2000
        )
        .is_err()
    );
}

#[test]
fn inferred_rational_cylinder_seam_and_reverse_preserve_orientation() {
    fn cylinder(z0: f64, z1: f64, reparameterize: bool) -> Surface {
        let points = [vec![1., 0.], vec![1., 1.], vec![0., 1.]];
        let mut weights = vec![1., 0.5_f64.sqrt(), 1.];
        if reparameterize {
            weights[1] *= 2.;
            weights[2] *= 4.;
        }
        Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: [z0, z1]
                .iter()
                .map(|z| points.iter().map(|p| vec![p[0], p[1], *z]).collect())
                .collect(),
            weights: vec![weights.clone(), weights],
            periodic_u: false,
            periodic_v: false,
        }
    }
    let a = cylinder(-1., 0., false);
    let b = cylinder(0., 1., true);
    let before = (a.clone(), b.clone());
    for reverse in [false, true] {
        let source = if reverse {
            surface_edit::reverse(&b, Axis::V).unwrap()
        } else {
            b.clone()
        };
        let r = surface_g2::inspect_inferred(
            &a,
            Boundary::UMax,
            &source,
            Boundary::UMin,
            reverse,
            32,
            0.04,
            0.08,
            0.15,
            6000,
        )
        .unwrap();
        let report = r.verification.unwrap();
        assert_eq!(report.decision, Decision::WithinTolerance, "{report:?}");
        assert_eq!(
            report.orientation,
            Some(if reverse {
                Orientation::Opposite
            } else {
                Orientation::Same
            })
        );
    }
    assert_eq!((a, b), before);
}

#[test]
fn identical_owned_boundary_keeps_zero_position_tolerance_without_hiding_curvature() {
    let a=patch(true,1.,false);
    let b=patch(false,1.,true);
    let report=surface_g2::inspect(&a,Boundary::UMax,&b,Boundary::UMin,false,0.,0.02,0.1,1000).unwrap();
    assert_eq!(report.position_bounds,[0.,0.]);
    assert_eq!(report.decision,Decision::WithinTolerance,"{report:?}");
    let wrong=patch(false,2.,false);
    let report=surface_g2::inspect(&a,Boundary::UMax,&wrong,Boundary::UMin,false,0.,0.02,0.1,1000).unwrap();
    assert_eq!(report.position_bounds,[0.,0.]);
    assert_eq!(report.decision,Decision::CorrespondenceExceedsTolerance,"{report:?}");
    let mut moved=b;
    for p in &mut moved.control_points[0] {p[0]+=1e-8;}
    let report=surface_g2::inspect(&a,Boundary::UMax,&moved,Boundary::UMin,false,0.,0.02,0.1,1000).unwrap();
    assert_ne!(report.decision,Decision::WithinTolerance,"A moved boundary must not inherit the identity shortcut");
}

#[test]
fn subrange_audit_uses_original_surfaces_and_never_claims_the_full_boundary() {
    let a=patch(true,1.,false);let b=patch(false,1.,true);let before=(a.clone(),b.clone());
    let nodes=[[0.2,0.2],[0.6,0.6]];
    let r=surface_g2::inspect_boundary_ranges(&a,Boundary::UMax,&b,Boundary::UMin,false,&nodes,0.05,0.02,0.1,1000).unwrap();
    assert_eq!(r.decision,Decision::WithinTolerance);
    assert!(surface_g2::inspect_with_correspondence(&a,Boundary::UMax,&b,Boundary::UMin,false,&nodes,0.05,0.02,0.1,1000).is_err());
    for nodes in [[[0.6,0.2],[0.2,0.6]],[[-0.1,0.],[0.5,0.5]],[[0.,0.],[0.5,1.1]]] {
        assert!(surface_g2::inspect_boundary_ranges(&a,Boundary::UMax,&b,Boundary::UMin,false,&nodes,0.05,0.02,0.1,1000).is_err());
    }
    assert_eq!((a,b),before);
}
