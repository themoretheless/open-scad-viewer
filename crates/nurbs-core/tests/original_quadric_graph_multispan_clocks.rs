use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphContacts,
        OriginalQuadricGraphParameterization, OriginalQuadricGraphUvProof,
    },
    surface::Surface,
};
fn target() -> (Surface, [[f64; 4]; 4], f64) {
    let l = 117733140.;
    let g = [0., 1., 3., 4.];
    let k = [0., 0., 2., 4.];
    let d = |i: usize, j: usize| {
        64. - 8. * g[i] - 8. * g[j] + 5. * g[i] * g[j] - 2. * k[i] * g[j] - 2. * g[i] * k[j]
            + k[i] * k[j]
    };
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -2., -2., 0., 2., 2., 2.],
        knots_v: vec![10., 10., 10., 12., 14., 14., 14.],
        weights: (0..4)
            .map(|i| (0..4).map(|j| d(i, j) / 16.).collect())
            .collect(),
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        vec![
                            l * (8. * g[i] - g[i] * g[j] + 2. * k[i] * g[j] - k[i] * k[j])
                                / d(i, j),
                            l * (8. * g[j] - g[i] * g[j] + 2. * g[i] * k[j] - k[i] * k[j])
                                / d(i, j),
                            l * (g[i] * g[j] + k[i] * k[j]) / d(i, j),
                        ]
                    })
                    .collect()
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let q = [
        [0., 0.5, 0., 0.],
        [0.5, 0., 0., 0.],
        [0., 0., 0., -l / 2.],
        [0., 0., -l / 2., 0.],
    ];
    (s, q, l)
}
fn source(l: f64, d: f64) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 6., 6.],
        control_points: vec![
            vec![l / d, l / d, l / (d * d) - 1.],
            vec![l / d, l / d, l / (d * d) + 1.],
        ],
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn present(r: &OriginalQuadricGraphContacts, expected: [f64; 3]) {
    assert!(r.complete && r.target_graph_proven, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(
        r.target_parameterization,
        OriginalQuadricGraphParameterization::HomogeneousSeparable
    );
    for (axis, x) in expected.into_iter().enumerate() {
        let b = r.contacts[0].parameter_box[axis];
        assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
        assert!(b[1] - b[0] < if axis == 0 { 1e-8 } else { 1e-12 });
    }
}
#[test]
fn original_nonlinear_multispan_clocks_preserve_global_and_internal_knot_uv() {
    for gauge in [1., 7.] {
        for rotated in [false, true] {
            for d in [2., 3.] {
                let (mut s, mut q, l) = target();
                let mut c = source(l, d);
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
                let r =
                    original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
                assert_eq!(s, before);
                let u = if d == 3. { 0. } else { 2. / 3. };
                present(&r, [4., u, u + 12.]);
            }
        }
    }
}
#[test]
fn original_multispan_nonlinear_clocks_keep_global_algebraic_edge_ownership() {
    let (s, q, l) = target();
    let a = [1., 0., -1., -2.];
    let f = [1., -1., 0., 4.];
    let c = Curve {
        degree: 3,
        knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
        control_points: (0..4)
            .map(|i| vec![l * a[i], l / 3., (l / 3.) * a[i] - l * f[i]])
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    present(&r, [2. + 4. / 3., -2., 12.]);
    assert!(r.boundary_exact_work > 0);
    assert_eq!(
        r.contacts[0].uv_proof,
        OriginalQuadricGraphUvProof::QuadricCoordinateSign
    );
}
#[test]
fn nonlinear_multispan_original_projection_and_quadric_checks_are_both_required() {
    let (mut s, q, l) = target();
    s.control_points[1][1][0] = f64::from_bits(s.control_points[1][1][0].to_bits() + 1);
    let r = original_quadric_graph_contacts(&source(l, 3.), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert_eq!(r.target_parameterization, OriginalQuadricGraphParameterization::JointJacobian);
    assert_eq!(r.target_membership.outcome, cad_predicates::BezierIdentity::Different);
    assert!(!r.target_graph_proven && !r.complete && r.carrier.is_none() && r.contacts.is_empty());
    let (mut s, q, l) = target();
    s.control_points[1][1][2] += 0.125;
    let r =
        original_quadric_graph_contacts(&source(l, 3.), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && !r.complete && r.carrier.is_none());
}
#[test]
fn nonlinear_multispan_certificate_does_not_reset_zero_predicate_budget() {
    let (s, q, l) = target();
    let r = original_quadric_graph_contacts(&source(l, 3.), &s, q, &[], 1e-9, 10, 0).unwrap();
    assert_eq!(r.exact_work, 0);
    assert!(!r.target_graph_proven && r.carrier.is_none());
}
