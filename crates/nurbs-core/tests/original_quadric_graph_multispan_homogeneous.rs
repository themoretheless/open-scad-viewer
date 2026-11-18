use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphParameterization,
        OriginalQuadricGraphUvProof,
    },
    surface::Surface,
};
fn target() -> (Surface, [[f64; 4]; 4], f64) {
    let l = 1808800.;
    let g = [0., 1., 3., 4.];
    let k = [0., 0., 2., 4.];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -2., -2., 0., 2., 2., 2.],
        knots_v: vec![10., 10., 10., 12., 14., 14., 14.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let d = 16. + g[i] * g[j];
                        vec![
                            l * (4. * g[i] + k[i] * g[j]) / d,
                            l * (4. * g[j] + g[i] * k[j]) / d,
                            l * (g[i] * g[j] + k[i] * k[j]) / d,
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: g
            .iter()
            .map(|a| g.iter().map(|b| (16. + a * b) / 16.).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let mut q = [[0.; 4]; 4];
    q[0][1] = 0.5;
    q[1][0] = 0.5;
    q[2][3] = -l / 2.;
    q[3][2] = -l / 2.;
    (s, q, l)
}
fn source(l: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![l / 2., l / 2., l / 4. - 1.],
            vec![l / 2., l / 2., l / 4. + 1.],
        ],
        weights: vec![1.; 2],
        periodic: false,
    }
}
#[test]
fn original_nonplanar_coupled_multispan_factor_has_global_exact_uv_and_weight_gauge() {
    for gauge in [1., 7.] {
        for rotated in [false, true] {
            let (mut s, mut q, l) = target();
            let mut c = source(l);
            for row in &mut s.weights {
                for w in row {
                    *w *= gauge;
                }
            }
            if rotated {
                for row in &mut s.control_points {
                    for p in row {
                        p.swap(0, 2);
                    }
                }
                for p in &mut c.control_points {
                    p.swap(0, 2);
                }
                q = [
                    [0., 0., 0., -l / 2.],
                    [0., 0., 0.5, 0.],
                    [0., 0.5, 0., 0.],
                    [-l / 2., 0., 0., 0.],
                ];
            }
            let before = s.clone();
            let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
            assert_eq!(s, before);
            assert!(r.target_graph_proven && r.complete);
            assert_eq!(r.contacts.len(), 1);
            assert_eq!(
                r.target_parameterization,
                OriginalQuadricGraphParameterization::HomogeneousAffine
            );
            assert_eq!(
                r.contacts[0].parameter_box,
                [[4., 4.], [0., 0.], [12., 12.]]
            );
        }
    }
}
#[test]
fn original_multispan_homogeneous_frame_proves_algebraic_outer_edge() {
    let (s, q, l) = target();
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: vec![
            vec![l, l / 2., -l / 2.],
            vec![0., l / 2., l],
            vec![-l, l / 2., -l / 2.],
            vec![-2. * l, l / 2., -5. * l],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(r.complete && r.target_graph_proven);
    assert_eq!(r.contacts.len(), 1);
    let contact = &r.contacts[0];
    assert_eq!(
        contact.uv_proof,
        OriginalQuadricGraphUvProof::QuadricCoordinateSign
    );
    assert!(r.boundary_exact_work > 0);
    for (axis, x) in [2. + 4. / 3., -2., 12.].into_iter().enumerate() {
        assert!(contact.parameter_box[axis][0] <= x && x <= contact.parameter_box[axis][1]);
    }
}
#[test]
fn no_original_multispan_projection_ulp_drift_is_admitted() {
    let (mut s, q, l) = target();
    s.control_points[1][1][0] = f64::from_bits(s.control_points[1][1][0].to_bits() + 1);
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert_eq!(r.target_parameterization, OriginalQuadricGraphParameterization::JointJacobian);
    assert_eq!(r.target_membership.outcome, cad_predicates::BezierIdentity::Different);
    assert!(!r.target_graph_proven && !r.complete && r.carrier.is_none() && r.contacts.is_empty());
}
#[test]
fn homogeneous_multispan_projection_does_not_replace_original_quadric_membership() {
    let (mut s, q, l) = target();
    s.control_points[1][1][2] += 0.125;
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && !r.complete && r.contacts.is_empty() && r.carrier.is_none());
}

#[test]
fn original_multispan_projection_never_resets_zero_predicate_budget() {
    let (s, q, l) = target();
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 0).unwrap();
    assert!(!r.target_graph_proven && !r.complete && r.carrier.is_none());
    assert_eq!(r.exact_work, 0);
    assert_eq!(r.target_membership.work_used, 0);
}
