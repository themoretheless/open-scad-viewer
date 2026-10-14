use nurbs_core::{
    surface::{Axis, Surface},
    surface_edit, surface_g1,
    surface_join::{Boundary, Decision},
};
fn plane(x: [f64; 2], slope: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: x
            .into_iter()
            .map(|x| vec![vec![x, 0., slope * x], vec![x, 1., slope * x]])
            .collect(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn g1_accepts_different_transverse_speeds_and_opposite_normals() {
    let a = plane([-1., 0.], 0.);
    let mut b = plane([0., 1.], 0.);
    b.weights[1] = vec![3.; 2];
    let before = b.clone();
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert!(r.regularity_proven);
    assert!(r.normal_chord_upper.unwrap() <= 0.02);
    assert!(r.position_bounds[1] <= 0.05);
    let reversed = surface_edit::reverse(&b, Axis::V).unwrap();
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &reversed,
        Boundary::UMin,
        true,
        0.05,
        0.02,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    assert_eq!(b, before);
    let at = surface_edit::transpose(&a).unwrap();
    let bt = surface_edit::transpose(&b).unwrap();
    let r = surface_g1::inspect(
        &at,
        Boundary::VMax,
        &bt,
        Boundary::VMin,
        false,
        0.05,
        0.02,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
}
#[test]
fn tangent_plane_gap_is_rejected_even_when_positions_match() {
    let a = plane([-1., 0.], 0.);
    let b = plane([0., 1.], 0.25);
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
    // Unit normals (0,0,1) and (-m,0,1)/sqrt(1+m²).
    let expected = (2. - 2. / (1.0625_f64).sqrt()).sqrt();
    assert!(r.normal_chord_lower <= expected);
    assert!(r.normal_chord_lower > 0.1);
    assert!(r.normal_chord_upper.unwrap() >= expected);
}
fn curved(left: bool) -> Surface {
    let x = if left { [-1., -0.5, 0.] } else { [0., 0.5, 1.] };
    let xx = if left { [1., 0., 0.] } else { [0., 0., 1.] };
    let y = [0., 0.5, 1.];
    let yy = [0., 0., 1.];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| (0..3).map(|j| vec![x[i], y[j], xx[i] + yy[j]]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn full_curved_seam_is_verified_by_adaptive_normal_bounds() {
    // z=x²+y² has the same normal (0,-2y,1) on the shared x=0 boundary.
    let a = curved(true);
    let b = curved(false);
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        10000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert!(r.cells > 1);
    assert!(r.normal_chord_upper.unwrap() <= 0.02);
}
#[test]
fn singularity_work_limits_and_invalid_contracts_are_explicit() {
    let a = plane([-1., 0.], 0.);
    let mut b = plane([0., 1.], 0.);
    b.control_points[1] = b.control_points[0].clone();
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        11,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
    assert!(r.normal_chord_upper.is_none());
    let b = plane([0., 1.], 0.);
    let r = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        1e-12,
        0.02,
        1,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert_eq!(r.cells, 1);
    assert!(
        surface_g1::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            -1.,
            0.02,
            100
        )
        .is_err()
    );
    assert!(
        surface_g1::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            f64::NAN,
            100
        )
        .is_err()
    );
    assert!(
        surface_g1::inspect(&a, Boundary::UMax, &b, Boundary::UMin, false, 0.1, 0.02, 0).is_err()
    );
    let b = surface_edit::insert(&b, Axis::V, -1., 1).unwrap();
    assert!(
        surface_g1::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            0.02,
            100
        )
        .is_err()
    );
}

#[test]
fn unsampled_interior_singularity_cannot_pass_a_planar_g1_check() {
    // x=u, y=(v-r)^2, z=0: identical tangent planes wherever regular,
    // but S_v vanishes at the non-dyadic interior point r=0.37.
    let r = 0.37_f64;
    let y = [r * r, r * r - r, (1. - r) * (1. - r)];
    let patch = |x: [f64; 2]| Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: x
            .into_iter()
            .map(|x| y.into_iter().map(|y| vec![x, y, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = patch([-1., 0.]);
    let b = patch([0., 1.]);
    let proof = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        125,
    )
    .unwrap();
    assert_eq!(proof.decision, Decision::Unresolved);
    assert!(!proof.regularity_proven);
    assert!(proof.normal_chord_upper.is_none());
}

#[test]
fn interior_tangent_plane_gap_between_initial_witnesses_is_rejected() {
    // B(u,v)=(u,v,u*f(v)), f(v)=8v(v-1/2)(v-1).
    // At u=0 the positions agree with a plane, and normals agree at
    // v=0,1/2,1. At v=1/4 the transverse slope is 3/8.
    let patch = |x: [f64; 2], bending: bool| Surface {
        degree_u: 1,
        degree_v: 3,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: x
            .into_iter()
            .map(|x| {
                [0., 4. / 3., -4. / 3., 0.]
                    .into_iter()
                    .enumerate()
                    .map(|(j, z)| vec![x, j as f64 / 3., if bending { x * z } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let a = patch([-1., 0.], false);
    let b = patch([0., 1.], true);
    for v in [0., 0.5, 1.] {
        let normal = b.evaluate(0., v).unwrap().unit_normal().unwrap();
        assert!(
            normal[0].abs() < 1e-12 && normal[1].abs() < 1e-12 && (normal[2] - 1.).abs() < 1e-12
        );
    }
    let proof = surface_g1::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(proof.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(proof.cells > 1);
    assert!(proof.normal_chord_lower > 0.1);
    let expected = (2. - 2. / (1. + (3_f64 / 8.).powi(2)).sqrt()).sqrt();
    assert!(proof.normal_chord_lower <= expected + 1e-12);
    assert!(proof.normal_chord_upper.unwrap() >= expected - 1e-12);
}
