use super::*;
fn polar_sphere() -> Surface {
    let r = 2.;
    let c = [1., 1., 0.];
    let z = [0., 1., 1.];
    let w = [1., 1., 2.];
    let xy = [[1., 0.], [1., 1.], [0., 1.]];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 7., 7., 7.],
        knots_v: vec![-3., -3., -3., 5., 5., 5.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![r * c[i] * xy[j][0], r * c[i] * xy[j][1], r * z[i]])
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| (0..3).map(|j| w[i] * w[j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn sphere() -> [[f64; 4]; 4] {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., -4.],
    ]
}
#[test]
fn original_binary_sphere_patch_has_regular_implicit_support_including_its_collapsed_pole() {
    let s = polar_sphere();
    let saved = s.clone();
    let r = inspect(&s, sphere(), 1000000, 100000).unwrap();
    assert!(
        r.exact_membership && r.gradient_nonzero && r.regular_implicit_support,
        "{r:?}"
    );
    assert!(r.gradient_lower.unwrap() > 0.);
    assert_eq!(s, saved);
    let pole = nurbs_core_local_point(&s);
    assert_eq!(pole, [0., 0., 2.]);
    eprintln!("exact original sphere pole implicit support {r:?}");
}
fn nurbs_core_local_point(s: &Surface) -> [f64; 3] {
    s.evaluate(7., 1.).unwrap().point
}
#[test]
fn implicit_support_refuses_each_short_ledger_and_all_nonzero_coefficient_residuals() {
    let s = polar_sphere();
    let full = inspect(&s, sphere(), 1000000, 100000).unwrap();
    assert!(full.regular_implicit_support);
    for (exact, cells) in [
        (0, 100000),
        (full.exact_work - 1, 100000),
        (1000000, 0),
        (1000000, full.cells - 1),
    ] {
        let r = inspect(&s, sphere(), exact, cells).unwrap();
        assert!(!r.regular_implicit_support);
        assert!(r.exact_work <= exact && r.cells <= cells);
    }
    let mut changed = s.clone();
    changed.control_points[1][1][0] = changed.control_points[1][1][0].next_up();
    assert!(
        !inspect(&changed, sphere(), 1000000, 100000)
            .unwrap()
            .exact_membership
    );
    let mut changed = s.clone();
    changed.weights[1][1] = changed.weights[1][1].next_up();
    assert!(
        !inspect(&changed, sphere(), 1000000, 100000)
            .unwrap()
            .exact_membership
    );
    let mut q = sphere();
    q[3][3] = q[3][3].next_up();
    assert!(!inspect(&s, q, 1000000, 100000).unwrap().exact_membership);
}
#[test]
fn weight_and_quadric_scales_preserve_the_original_locus_without_common_coordinate_normalization() {
    let mut s = polar_sphere();
    for r in &mut s.weights {
        for w in r {
            *w *= 2.;
        }
    }
    let mut q = sphere();
    for r in &mut q {
        for x in r {
            *x *= 0.5;
        }
    }
    let r = inspect(&s, q, 1000000, 100000).unwrap();
    assert!(r.regular_implicit_support, "{r:?}");
    // Scaling ONLY the sphere's constant changes its radius. No source-wide
    // normalization may silently replace x^2+y^2+z^2=4 by a different locus.
    q[3][3] = -1.;
    assert!(!inspect(&s, q, 1000000, 100000).unwrap().exact_membership);
}
#[test]
fn constant_and_squared_plane_equations_never_supply_regular_support_credit() {
    let s = polar_sphere();
    let r = inspect(&s, [[0.; 4]; 4], 1000000, 100000).unwrap();
    assert!(r.exact_membership && !r.gradient_nonzero && !r.regular_implicit_support);
    let mut plane = s;
    for row in &mut plane.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    let mut q = [[0.; 4]; 4];
    q[2][2] = 1.;
    let r = inspect(&plane, q, 1000000, 64).unwrap();
    assert!(r.exact_membership && !r.regular_implicit_support && r.cells <= 64);
    // A plane's regular implicit support does NOT prove its parameter map is
    // regular: callers still need dimension/quotient and vertex-fan obligations.
    let mut q = [[0.; 4]; 4];
    q[2][3] = 0.5;
    q[3][2] = 0.5;
    let r = inspect(&plane, q, 1000000, 100000).unwrap();
    assert!(r.regular_implicit_support);
}

#[test]
fn original_radius_square_is_not_rounded_into_a_different_sphere_equation() {
    for radius in [3_f64.next_up(), 0.125, 12.] {
        let mut s = polar_sphere();
        for row in &mut s.control_points {
            for p in row {
                for x in p {
                    *x = *x / 2. * radius;
                }
            }
        }
        let r = inspect_sphere(&s, [0.; 3], radius, 1000000, 100000).unwrap();
        assert!(r.regular_implicit_support, "radius {radius}: {r:?}");
        if radius == 3_f64.next_up() {
            let mut q = sphere();
            q[3][3] = -radius * radius;
            assert!(
                !inspect(&s, q, 1000000, 100000).unwrap().exact_membership,
                "rounded r^2 must not replace original r*r"
            );
        }
    }
}
