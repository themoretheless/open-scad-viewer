use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        affine_plane_contacts, original_affine_patch_contacts,
    },
    surface::Surface,
};
fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-2., -2., 2., 2.],
        knots_v: vec![10., 10., 14., 14.],
        control_points: vec![
            vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.]],
            vec![vec![1.1, 0., 0.1], vec![1.1, 1., 0.1]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn curve(p: usize) -> Curve {
    Curve {
        degree: p,
        knots: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        control_points: (0..=p)
            .map(|i| {
                vec![
                    if i % 2 == 0 { 0.1 } else { 1.1 },
                    i as f64 / p as f64,
                    if i % 2 == 0 { 0.1 } else { 0. },
                ]
            })
            .collect(),
        weights: vec![1.; p + 1],
        periodic: false,
    }
}
#[test]
fn unrepresentable_plane_contacts_are_admitted_inside_the_original_patch() {
    for order in [2, 4] {
        let c = curve(order);
        let s = patch();
        assert!(affine_plane_contacts(&c, &s, 1e-9, 1000, 100000).is_err());
        let r = original_affine_patch_contacts(&c, &s, 1e-9, 10, 100000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.contacts.len(), 1, "{r:?}");
        let x = &r.contacts[0];
        assert_eq!(x.multiplicities, vec![order]);
        for (i, t) in [0.5, 0., 12.].into_iter().enumerate() {
            assert!(
                x.parameter_box[i][0] <= t && t <= x.parameter_box[i][1],
                "{x:?}"
            );
        }
        assert!(r.exact_work <= 100000);
        assert_eq!(r.scalar_proofs.len(), 1);
    }
}
#[test]
fn finite_membership_excludes_outside_and_preserves_boundary_ambiguity() {
    let mut c = curve(2);
    for p in &mut c.control_points {
        p[1] = 2.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(r.complete);
    assert!(r.contacts.is_empty());
    for p in &mut c.control_points {
        p[1] = 0.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert!(!r.ambiguous_parameters.is_empty());
}
#[test]
fn one_global_exact_budget_and_span_budget_preserve_missing_coverage() {
    let mut c = curve(2);
    let row = c.control_points.clone();
    c.control_points.extend([row[1].clone(), row[2].clone()]);
    c.weights = vec![1.; 5];
    c.knots = vec![0., 0., 0., 1., 1., 2., 2., 2.];
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 1, 100000).unwrap();
    assert!(!r.complete);
    assert_eq!(r.spans, 1);
    assert_eq!(r.contacts.len(), 1);
    assert_eq!(r.unresolved_spans.len(), 1);
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 0).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.exact_work, 0);
    assert_eq!(r.unresolved_spans.len(), 2);
}
#[test]
fn coincident_span_and_nonaffine_target_do_not_authorize_isolated_contacts() {
    let mut c = curve(2);
    for p in &mut c.control_points {
        p[0] = 0.1;
        p[2] = 0.;
    }
    let r = original_affine_patch_contacts(&c, &patch(), 1e-9, 10, 100000).unwrap();
    assert!(!r.complete);
    assert!(r.contacts.is_empty());
    assert_eq!(r.unresolved_spans.len(), 1);
    let mut s = patch();
    s.control_points[1][1][0] = 1.1f64.next_up();
    assert!(original_affine_patch_contacts(&curve(2), &s, 1e-9, 10, 100000).is_err());
}
