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

#[test]
fn independent_degrees_and_refined_knots_certify_original_whole_spans() {
    // Same x=2*t geometry, three independently authored representations.
    let strip = |degree, knots: Vec<f64>, x: &[f64], y| Surface {
        degree_u: degree,
        degree_v: 2,
        knots_u: knots,
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| (0..3).map(|j| vec![x, y + j as f64, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; x.len()],
        periodic_u: false,
        periodic_v: false,
    };
    let a = strip(1, vec![0., 0., 1., 1.], &[0., 2.], 0.);
    let cases = [
        strip(2, vec![0., 0., 0., 1., 1., 1.], &[0., 1., 2.], 2.),
        strip(2, vec![4., 4., 4., 5., 6., 6., 6.], &[0., 0.5, 1.5, 2.], 2.),
        // Nonclamped, different degree, original full active domain [0,2].
        strip(
            2,
            vec![-2., -1., 0., 1., 2., 3., 4.],
            &[-0.5, 0.5, 1.5, 2.5],
            2.,
        ),
    ];
    for b in cases {
        let saved = (a.clone(), b.clone());
        let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert!(
            !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
                .unwrap()
                .certified
        );
        assert!(!audit(&a, &b, "vMax", "vMin", 2, 1., 0).unwrap().certified);
        let mut mismatch = b.clone();
        mismatch.control_points[1][0][2] = f64::EPSILON;
        assert!(
            !audit(&a, &mismatch, "vMax", "vMin", 1, 1., 1000000)
                .unwrap()
                .certified
        );
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
        assert_eq!((a.clone(), b), saved);
    }
}

#[test]
fn independent_rational_degree_elevation_binds_every_homogeneous_coefficient() {
    let make = |p, x: &[f64], weights: &[f64], y| Surface {
        degree_u: p,
        degree_v: 1,
        knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, y, 0.], vec![x, y + 1., 0.]])
            .collect(),
        weights: weights.iter().map(|&w| vec![w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(1, &[0., 3.], &[1., 2.], 0.);
    let b = make(2, &[0., 2., 3.], &[3., 4.5, 6.], 1.);
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    let mut changed = b.clone();
    changed.weights[1][0] = f64::from_bits(4.5f64.to_bits() + 1);
    assert!(
        !audit(&a, &changed, "vMax", "vMin", 2, 1., 1000000)
            .unwrap()
            .certified
    );
    let plane = make(1, &[0., 1.], &[1., 1.], 0.);
    let mut hidden = make(4, &[0., 0.25, 0.5, 0.75, 1.], &[1.; 5], 1.);
    hidden.control_points[2][0][2] = 0.125;
    // Endpoints and endpoint tangents agree; the original interior does not.
    assert!(
        !audit(&plane, &hidden, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
}

#[test]
fn independent_along_basis_keeps_periodic_cross_closure_mandatory() {
    let a = cylinder(2, 0.);
    let mut b = a.clone();
    b.degree_u = 2;
    b.knots_u = vec![0., 0., 0., 1., 1., 1.];
    let middle = a.control_points[0]
        .iter()
        .map(|p| vec![p[0], p[1], 0.5])
        .collect();
    b.control_points.insert(1, middle);
    b.weights.insert(1, b.weights[0].clone());
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    for row in &mut b.control_points {
        row[0][0] += 0.125;
        row[4][0] += 0.125;
    }
    let refused = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(!refused.certified, "{refused:?}");
}

#[test]
fn fractional_linear_original_knots_propose_but_do_not_authorize_seams_or_pcurves() {
    use crate::curve::Curve;
    let make = |knots: Vec<f64>, weights: &[f64], y| Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: knots,
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..5)
            .map(|x| vec![vec![x as f64, y, 0.], vec![x as f64, y + 1., 0.]])
            .collect(),
        weights: weights.iter().map(|&w| vec![w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let a = make(vec![0., 0., 0., 0.5, 0.75, 1., 1., 1.], &[1.; 5], 0.);
    let b = make(
        vec![0., 0., 0., 0.25, 0.5, 1., 1., 1.],
        &[1., 1.5, 3., 6., 9.],
        1.,
    );
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    let mut forged = b.clone();
    forged.weights[2][0] = f64::from_bits(3f64.to_bits() + 1);
    assert!(
        !audit(&a, &forged, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    // The SAME point set under phi(t)=3t/(1+2t) is not the SAME
    // parameterized trace under an authored LINEAR pcurve. Keep this refusal.
    let c = Curve {
        degree: 2,
        knots: a.knots_u.clone(),
        control_points: a.control_points.iter().map(|r| r[1].clone()).collect(),
        weights: vec![1.; 5],
        periodic: false,
    };
    let p = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let binding = crate::curve_surface_agreement::verify_exact(&c, &p, &b, false, 1000000)
        .unwrap()
        .unwrap();
    assert_ne!(binding.outcome, BezierIdentity::Equal);
}

#[test]
fn nonlinear_polynomial_and_rational_graph_seams_own_all_coefficients_and_maps() {
    // A(s)=(s,s*s, z), B(t)=A((t+t*t)/2). All coefficients are
    // authored exactly (scaled by 48 to keep Bernstein poles dyadic).
    let surface = |p, rows: Vec<[f64; 2]>, weights: Vec<f64>, z| Surface {
        degree_u: p,
        degree_v: 2,
        knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: rows
            .iter()
            .map(|p| {
                vec![
                    vec![p[0], p[1], z],
                    vec![p[0], p[1], z + 0.5],
                    vec![p[0], p[1], z + 1.],
                ]
            })
            .collect(),
        weights: weights.iter().map(|&w| vec![w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let a = surface(2, vec![[0., 0.], [24., 0.], [48., 48.]], vec![1.; 3], 0.);
    let b = surface(
        4,
        vec![[0., 0.], [6., 0.], [16., 2.], [30., 12.], [48., 48.]],
        vec![1.; 5],
        1.,
    );
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    let collection =
        crate::continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
            &[a.clone(), b.clone()],
            &[crate::continuity::ProjectiveSeam {
                patches: [0, 1],
                boundaries: ["vMax".into(), "vMin".into()],
                order: 2,
                normal_scale: 1.,
            }],
            1000000,
        )
        .unwrap();
    assert!(collection.certified, "{:?}", collection.seams.first());

    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    assert!(!audit(&a, &b, "vMax", "vMin", 2, 1., 0).unwrap().certified);
    let mut curvature = b.clone();
    for row in &mut curvature.control_points {
        row[2][1] += 0.125;
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
    let mut hidden = b.clone();
    hidden.control_points[2][0][1] = 2f64.next_up();
    assert!(
        !audit(&a, &hidden, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
    // Rational speed map: degree-one homogeneous source mapped by unequal
    // positive weights, with no interior knots available to propose a Mobius map.
    let line = surface(1, vec![[0., 0.], [3., 0.]], vec![1.; 2], 0.);
    let rational = surface(2, vec![[0., 0.], [1., 0.], [3., 0.]], vec![1., 1., 2.], 1.);
    let proof = audit(&line, &rational, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    let same_degree = surface(1, vec![[0., 0.], [3., 0.]], vec![1., 2.], 1.);
    let proof = audit(&line, &same_degree, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "same-degree nonlinear fallback: {proof:?}");
    // A stationary/reversing map cannot borrow geometric set equality.
    let folded = surface(
        3,
        vec![[0., 0.], [6., 0.], [-3., 0.], [3., 0.]],
        vec![1.; 4],
        1.,
    );
    let rejected = audit(&line, &folded, "vMax", "vMin", 1, 1., 1000000).unwrap();
    assert!(
        !rejected.exact_identity && !rejected.certified,
        "{rejected:?}"
    );
}

#[test]
fn periodic_nonlinear_chart_cover_preserves_one_lap_and_refuses_double_cover() {
    let xy = [[1., 0.], [0., 1.], [-1., 0.], [0., -1.]];
    let a = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: (0..9).map(|i| i as f64).collect(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..6)
            .map(|i| {
                (0..3)
                    .map(|j| vec![24. * xy[i % 4][0], 24. * xy[i % 4][1], 0.5 * j as f64])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 6],
        periodic_u: true,
        periodic_v: false,
    };
    let chain = |laps: usize| {
        let mut points = Vec::new();
        for j in 0..4 * laps {
            let a = xy[j % 4];
            let b = xy[(j + 1) % 4];
            let c = xy[(j + 2) % 4];
            if j == 0 {
                points.push([12. * (a[0] + b[0]), 12. * (a[1] + b[1])]);
            }
            points.push([9. * a[0] + 15. * b[0], 9. * a[1] + 15. * b[1]]);
            points.push([
                4.5 * a[0] + 19. * b[0] + 0.5 * c[0],
                4.5 * a[1] + 19. * b[1] + 0.5 * c[1],
            ]);
            points.push([21. * b[0] + 3. * c[0], 21. * b[1] + 3. * c[1]]);
            points.push([12. * (b[0] + c[0]), 12. * (b[1] + c[1])]);
        }
        let mut knots = vec![0.; 5];
        for j in 1..4 * laps {
            knots.extend(vec![j as f64; 4]);
        }
        knots.extend(vec![(4 * laps) as f64; 5]);
        Surface {
            degree_u: 4,
            degree_v: 2,
            knots_u: knots,
            knots_v: a.knots_v.clone(),
            control_points: points
                .iter()
                .map(|p| {
                    (0..3)
                        .map(|j| vec![p[0], p[1], 1. + 0.5 * j as f64])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; points.len()],
            periodic_u: false,
            periodic_v: false,
        }
    };
    let b = chain(1);
    let saved = (a.clone(), b.clone());
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    let twice = chain(2);
    assert_eq!(twice.control_points.len(), 33);
    let refusal = audit(&a, &twice, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(!refusal.exact_identity && !refusal.certified, "{refusal:?}");
    assert_eq!((a, b), saved);
}

#[test]
fn nonlinear_rational_conic_has_a_projective_chart_without_an_affine_cartesian_axis() {
    let make = |p, xy: &[[f64; 2]], w: &[f64], z| Surface {
        degree_u: p,
        degree_v: 2,
        knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: xy
            .iter()
            .map(|p| {
                (0..3)
                    .map(|j| vec![p[0], p[1], z + 0.5 * j as f64])
                    .collect()
            })
            .collect(),
        weights: w.iter().map(|&w| vec![w; 3]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    // Exact homogeneous composition of a rational conic with phi=(t+t*t)/2.
    // Scaling by 665 makes every Cartesian pole exactly representable binary64.
    let a = make(
        2,
        &[[665., 0.], [665., 665.], [0., 665.]],
        &[12., 24., 12.],
        0.,
    );
    let b = make(
        4,
        &[
            [665., 0.],
            [665., 266.],
            [647.5, 507.5],
            [570., 665.],
            [0., 665.],
        ],
        &[12., 15., 19., 21., 12.],
        1.,
    );
    let proof = audit(&a, &b, "vMax", "vMin", 2, 1., 1000000).unwrap();
    assert!(proof.certified, "{proof:?}");
    assert!(
        !audit(&a, &b, "vMax", "vMin", 2, 1., proof.work - 1)
            .unwrap()
            .certified
    );
    let mut bad = b.clone();
    bad.weights[2][0] = 19f64.next_up();
    assert!(
        !audit(&a, &bad, "vMax", "vMin", 1, 1., 1000000)
            .unwrap()
            .certified
    );
}
