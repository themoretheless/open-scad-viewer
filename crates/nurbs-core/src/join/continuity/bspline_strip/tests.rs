use super::*;
use crate::continuity::inspect_surface_projective_strip_jets as audit;

fn plane(y: f64) -> Surface {
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.],
        control_points: (0..5)
            .map(|x| (0..5).map(|j| vec![x as f64, y + j as f64, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn general_cross_bspline_jets_keep_g2_and_reject_kinks_curvature_and_short_work() {
    let a = plane(0.);
    let b = plane(4.);
    let saved = (a.clone(), b.clone());
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(proof.work > 0 && proof.work <= 1000000);
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    assert!(!audit(&a, &b, "vMax", "vMin", 2, 1., 0).unwrap().certified);
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][2] = 0.125;
    }
    assert!(
        audit(&a, &curvature, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert!(
        !audit(&a, &curvature, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut kink = b.clone();
    for row in &mut kink.control_points {
        row[1][2] = f64::EPSILON;
    }
    assert!(
        !audit(&a, &kink, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    assert_eq!((a, b), saved);
}
#[test]
fn rational_gauges_and_independent_nonuniform_natural_domains_are_exact() {
    let mut a = plane(0.);
    let mut b = plane(4.);
    for i in 0..5 {
        let w = if i % 2 == 0 { 1. } else { 2. };
        a.weights[i] = vec![w; 5];
        b.weights[i] = vec![3. * w; 5];
    }
    b.knots_u = b.knots_u.iter().map(|x| 7. + 3. * x).collect();
    b.knots_v = b.knots_v.iter().map(|x| -2. + 6. * x).collect();
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    b.knots_u[4] = f64::from_bits(b.knots_u[4].to_bits() + 1);
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
}
#[test]
fn unclamped_original_cross_basis_needs_no_rounded_extraction() {
    let strip = |offset| Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![-2., -1., 0., 1., 2., 3., 4.],
        control_points: (0..2)
            .map(|i| {
                (0..4)
                    .map(|j| vec![i as f64, offset + j as f64, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = strip(0.);
    let b = strip(2.);
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    let mut singular = [a, b];
    for s in &mut singular {
        for row in &mut s.control_points {
            for p in row {
                p[0] = 0.;
            }
        }
    }
    let proof = audit(&singular[0], &singular[1], "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(!proof.regularity_certified && !proof.certified, "{proof:?}");
}
fn cylinder(p: usize, z: f64) -> Surface {
    let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let n = 4 + p;
    Surface {
        degree_u: 1,
        degree_v: p,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: (0..n + p + 1).map(|i| i as f64).collect(),
        control_points: (0..2)
            .map(|i| {
                (0..n)
                    .map(|j| vec![xy[j % 4][0], xy[j % 4][1], z + i as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; n]; 2],
        periodic_u: false,
        periodic_v: true,
    }
}
fn transpose(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|j| s.control_points.iter().map(|r| r[j].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|j| s.weights.iter().map(|r| r[j]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
#[test]
fn periodic_cross_closure_is_proved_for_cubic_and_geometric_quadratic() {
    for degree in [2, 3] {
        let s = cylinder(degree, 0.);
        s.validate().unwrap();
        let proof = audit(&s, &s, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(proof.certified, "degree {degree}: {proof:?}");
        assert!(
            !audit(&s, &s, "vMax", "vMin", 2, 1., proof.work - 1)
                .unwrap()
                .certified
        );
    }
}
#[test]
fn periodic_along_basis_and_approximate_periodic_knot_flags_do_not_supply_credit() {
    for degree in [2, 3] {
        let a = transpose(&cylinder(degree, 0.));
        let b = transpose(&cylinder(degree, 1.));
        let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(proof.certified, "degree {degree}: {proof:?}");
        let mut changed = a.clone();
        changed.knots_u[1] = f64::from_bits(1f64.to_bits() + 1);
        changed.validate().unwrap(); // tolerant storage validation is NOT an exact closure proof
        assert!(
            !audit(&changed, &changed, "vMax", "vMin", 2, 1., 1000000)
                .unwrap()
                .certified
        );
    }
}

#[test]
fn nonuniform_linear_cross_speeds_are_geometric_and_interior_kinks_stay_refused() {
    let strip = |y: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0.25, 1., 1.],
        control_points: (0..2)
            .map(|i| (0..3).map(|j| vec![i as f64, y + j as f64, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = strip(0.);
    let b = strip(2.);
    // End/start normalized speeds are 4/3 and 4; scale 3 is a proposal.
    let proof = audit(&a, &b, "vMax", "vMin", 2, 3., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let mut kink = a.clone();
    for row in &mut kink.control_points {
        row[0][2] = 0.125;
    }
    assert!(
        !audit(&kink, &b, "vMax", "vMin", 1, 3., 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn unequal_cross_degrees_and_finite_weight_gauges_are_not_assumed_equal() {
    let mut a = plane(0.);
    a.degree_v = 1;
    a.knots_v = vec![0., 0., 0.5, 1., 1.];
    for row in &mut a.control_points {
        row.truncate(3);
    }
    a.weights = vec![vec![1.; 3]; 5];
    let b = plane(2.);
    let proof = audit(&a, &b, "vMax", "vMin", 2, 3., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    let mut weight = b.clone();
    weight.weights[2][0] = 2.;
    assert!(
        !audit(&a, &weight, "vMax", "vMin", 2, 3., 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn exact_rational_transition_proposal_keeps_explicit_binary_scale_semantics() {
    use crate::continuity::{
        ProjectiveSeam, inspect_projective_seam_collection_with_rational_scale_proposals,
    };
    let mut a = plane(0.);
    let mut b = plane(0.);
    for row in &mut a.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[1] = 3. * j as f64;
        }
    }
    for row in &mut b.control_points {
        for (j, p) in row.iter_mut().enumerate() {
            p[1] = 12. + 7. * j as f64;
        }
    }
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 7. / 3., 1000000)
            .unwrap()
            .certified
    );
    let patches = [a, b];
    let seams = [ProjectiveSeam {
        patches: [0, 1],
        boundaries: ["vMax".into(), "vMin".into()],
        order: 2,
        normal_scale: 7. / 3.,
    }];
    let proof =
        inspect_projective_seam_collection_with_rational_scale_proposals(&patches, &seams, 1000000)
            .unwrap();
    assert!(proof.certified && proof.certified_order == Some(2));
    assert!(proof.exact_work <= 1000000);
    assert!(
        !inspect_projective_seam_collection_with_rational_scale_proposals(
            &patches,
            &seams,
            proof.exact_work - 1
        )
        .unwrap()
        .certified
    );
}
