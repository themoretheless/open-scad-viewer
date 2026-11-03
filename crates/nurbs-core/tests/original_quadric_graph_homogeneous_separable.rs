use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        original_quadric_graph_contacts, OriginalQuadricGraphContacts,
        OriginalQuadricGraphParameterization, OriginalQuadricGraphUvProof,
    },
    surface::Surface,
};
fn target() -> (Surface, [[f64; 4]; 4], f64) {
    let l = 195.;
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -2., -2., 2., 2., 2.],
        knots_v: vec![10., 10., 10., 14., 14., 14.],
        weights: vec![vec![4., 3., 2.], vec![3., 3.25, 2.5], vec![2., 2.5, 2.]],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 65., 0.], vec![0., 195., 0.]],
            vec![vec![65., 0., 0.], vec![45., 45., 15.], vec![39., 195., 39.]],
            vec![
                vec![195., 0., 0.],
                vec![195., 39., 39.],
                vec![195., 195., 195.],
            ],
        ],
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
fn present(r: &OriginalQuadricGraphContacts, expected: [f64; 3]) {
    assert!(r.complete && r.target_graph_proven);
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(
        r.target_parameterization,
        OriginalQuadricGraphParameterization::HomogeneousSeparable
    );
    for (axis, x) in expected.into_iter().enumerate() {
        let b = r.contacts[0].parameter_box[axis];
        assert!(b[0] <= x && x <= b[1], "{b:?} excludes {x}");
        let width_limit = if axis == 0 { 1e-8 } else { 1e-12 };
        assert!(b[1] - b[0] < width_limit, "axis {axis}, box {b:?}");
    }
}
#[test]
fn nonlinear_boundary_clocks_prove_nonplanar_coupled_original_targets() {
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
            present(&r, [4., 2. / 3., 38. / 3.]);
        }
    }
}
#[test]
fn nonlinear_homogeneous_clocks_keep_all_algebraic_edges_and_corners() {
    for edge in 0..6 {
        let (s, q, l) = target();
        let a = [1., 0., -1., -2.];
        let f = [1., -1., 0., 4.];
        let controls = (0..4)
            .map(|i| {
                let x = a[i];
                let p = match edge {
                    0 => [x, 0.5, 0.5 * x - f[i]],
                    1 => [1. + x, 0.5, 0.5 * (1. + x) - f[i]],
                    2 => [0.5, x, 0.5 * x - f[i]],
                    3 => [0.5, 1. + x, 0.5 * (1. + x) - f[i]],
                    4 => [x, x, 0.],
                    _ => [1. + x, 1. + x, [3., 1., -1., -3.][i]],
                };
                p.into_iter().map(|x| x * l).collect()
            })
            .collect();
        let c = Curve {
            degree: 3,
            knots: vec![2., 2., 2., 2., 6., 6., 6., 6.],
            control_points: controls,
            weights: vec![1.; 4],
            periodic: false,
        };
        let r = original_quadric_graph_contacts(&c, &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
        let (u, v) = match edge {
            0 => (-2., 38. / 3.),
            1 => (2., 38. / 3.),
            2 => (2. / 3., 10.),
            3 => (2. / 3., 14.),
            4 => (-2., 10.),
            _ => (2., 14.),
        };
        present(&r, [2. + 4. / 3., u, v]);
        assert!(r.boundary_exact_work > 0);
        assert_eq!(
            r.contacts[0].uv_proof,
            OriginalQuadricGraphUvProof::QuadricCoordinateSign
        );
    }
}
#[test]
fn original_nonlinear_clock_identities_never_snap_or_replace_quadric_membership() {
    let (mut s, q, l) = target();
    s.control_points[1][1][0] = f64::from_bits(45f64.to_bits() + 1);
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert_eq!(r.target_parameterization,OriginalQuadricGraphParameterization::JointJacobian);
    assert_eq!(r.target_membership.outcome,cad_predicates::BezierIdentity::Different);
    assert!(!r.target_graph_proven && !r.complete && r.contacts.is_empty() && r.carrier.is_none());
    let (mut s, q, l) = target();
    s.control_points[1][1][2] += 0.125;
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 1_000_000).unwrap();
    assert!(!r.target_graph_proven && !r.complete && r.carrier.is_none());
}
#[test]
fn nonlinear_original_projection_preserves_zero_predicate_budget() {
    let (s, q, l) = target();
    let r = original_quadric_graph_contacts(&source(l), &s, q, &[], 1e-9, 10, 0).unwrap();
    assert_eq!(r.exact_work, 0);
    assert!(!r.target_graph_proven && r.carrier.is_none());
}
