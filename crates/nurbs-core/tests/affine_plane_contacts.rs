use nurbs_core::{
    curve::Curve, intersection::certified_curve_surface::affine_plane_contacts, surface::Surface,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![0., -1., 0.], vec![0., -1., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn tangent(degree: usize) -> Curve {
    let a = if degree == 2 { 0.25 } else { 0.0625 };
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: (0..=degree)
            .map(|i| {
                vec![
                    if i % 2 == 0 { a } else { -a },
                    i as f64 / degree as f64,
                    1.,
                ]
            })
            .collect(),
        weights: vec![1.; degree + 1],
        periodic: false,
    }
}
#[test]
fn even_multiplicity_is_proved_without_a_transverse_jacobian() {
    for degree in [2, 4] {
        let c = tangent(degree);
        let r = affine_plane_contacts(&c, &plane(), 1e-9, 1000, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1, "{r:?}");
        let contact = &r.contacts[0];
        for (i, t) in [0.5, 1., 12.].into_iter().enumerate() {
            assert!(contact.parameter_box[i][0] <= t && t <= contact.parameter_box[i][1]);
        }
        for &i in &contact.scalar_cells {
            assert_eq!(r.scalar.cells[i].root_multiplicity, Some(degree));
        }
        assert!(r.scalar.work <= 1000);
        assert!(r.scalar.exact_predicate_work <= 100000);
    }
}
#[test]
fn contact_outside_patch_is_excluded_but_boundary_membership_stays_unresolved() {
    let mut c = tangent(2);
    for p in &mut c.control_points {
        p[1] += 10.;
    }
    let r = affine_plane_contacts(&c, &plane(), 1e-9, 1000, 100000).unwrap();
    assert!(r.complete);
    assert!(r.contacts.is_empty());
    for p in &mut c.control_points {
        p[1] = -1.;
    }
    let r = affine_plane_contacts(&c, &plane(), 1e-9, 1000, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert!(!r.unresolved.is_empty());
}
#[test]
fn coincident_curve_is_not_an_isolated_tangent_contact() {
    let mut c = tangent(2);
    for p in &mut c.control_points {
        p[0] = 0.;
    }
    let r = affine_plane_contacts(&c, &plane(), 1e-9, 1000, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert!(!r.unresolved.is_empty());
}
#[test]
fn nonaffine_target_is_not_replaced_by_an_approximate_plane() {
    let mut s = plane();
    s.control_points[1][1][0] = f64::from_bits(1);
    assert!(affine_plane_contacts(&tangent(2), &s, 1e-9, 1000, 100000).is_err());
}

#[test]
fn rational_weights_preserve_a_proved_non_dyadic_double_contact() {
    let mut c = tangent(2);
    c.weights = vec![1., 2., 4.];
    // Homogeneous x=(1/2-3t/2)^2, positive denominator (1+t)^2.
    let r = affine_plane_contacts(&c, &plane(), 1e-9, 1000, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    let contact = &r.contacts[0];
    let t = 1. / 3.;
    assert!(contact.parameter_box[0][0] <= t && t <= contact.parameter_box[0][1]);
    assert!(
        contact
            .scalar_cells
            .iter()
            .all(|&i| r.scalar.cells[i].root_multiplicity == Some(2))
    );
}
#[test]
fn oblique_original_affine_patch_uses_its_exact_plane() {
    let mut c = tangent(2);
    let mut s = plane();
    let transform = |p: &mut Vec<f64>| {
        let (x, y, z) = (p[0], p[1], p[2]);
        *p = vec![x + y, y + z, z];
    };
    for p in &mut c.control_points {
        transform(p);
    }
    for row in &mut s.control_points {
        for p in row {
            transform(p);
        }
    }
    let r = affine_plane_contacts(&c, &s, 1e-9, 1000, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    for (i, x) in [0.5, 1.5, 1.].into_iter().enumerate() {
        assert!(
            r.contacts[0].position_bounds[i][0] <= x && x <= r.contacts[0].position_bounds[i][1]
        );
    }
}

#[test]
fn exact_common_plane_scale_is_removed_before_binary64_admission() {
    let mut c = tangent(2);
    let mut s = plane();
    // (2*binary64(0.1))^2 is not itself a binary64 plane normal.
    // Its common factor cancels exactly from the whole plane equation.
    for p in &mut c.control_points {
        for x in p {
            *x *= 0.1;
        }
    }
    for row in &mut s.control_points {
        for p in row {
            for x in p {
                *x *= 0.1;
            }
        }
    }
    let r = affine_plane_contacts(&c, &s, 1e-9, 1000, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1);
    assert!(r.contacts[0].parameter_box[0][0] <= 0.5 && 0.5 <= r.contacts[0].parameter_box[0][1]);
    assert!(
        r.contacts[0]
            .scalar_cells
            .iter()
            .all(|&i| r.scalar.cells[i].root_multiplicity == Some(2))
    );
}

#[test]
fn common_scale_reduction_preserves_the_nonzero_plane_offset() {
    let mut s = plane();
    for row in &mut s.control_points {
        for p in row {
            for x in p.iter_mut() {
                *x *= 0.1;
            }
            p[0] = 0.1;
        }
    }
    let mut c = tangent(2);
    for (i, p) in c.control_points.iter_mut().enumerate() {
        p[0] = if i == 1 { 2. * 0.1 - 0.125 } else { 0.125 };
        p[1] *= 0.1;
        p[2] = 0.1;
    }
    let r = affine_plane_contacts(&c, &s, 1e-9, 1000, 100000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.contacts.len(), 1, "{r:?}");
    let contact = &r.contacts[0];
    assert!(contact.parameter_box[0][0] <= 0.5 && 0.5 <= contact.parameter_box[0][1]);
    assert!(contact.position_bounds[0][0] <= 0.1 && 0.1 <= contact.position_bounds[0][1]);
}

#[test]
fn irreducible_nonrepresentable_plane_coefficients_are_not_rounded() {
    let mut s = plane();
    s.control_points = vec![
        vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.]],
        vec![vec![1.1, 0., 0.1], vec![1.1, 1., 0.1]],
    ];
    // Exact 1.1 - 0.1 has an irreducible 56-bit mantissa. The offset
    // participates in the common gcd; independently rounding a normal is invalid.
    assert!(affine_plane_contacts(&tangent(2), &s, 1e-9, 1000, 100000).is_err());
}
