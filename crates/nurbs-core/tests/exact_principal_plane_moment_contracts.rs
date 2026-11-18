use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_planar_face_mass::{
            ExactPrincipalPlaneOutcome, exact_projected_face_mass,
            exact_projected_face_principal_moments,
        },
        planar_trim_projection::{
            PlanarTrimOutcome, ProjectedTrimmedFace, project_coplanar_trim_loops,
        },
    },
    curve::Curve,
    surface::Surface,
};
const WORK: u64 = 1_000_000;
fn chart(tilted: bool) -> ProjectedTrimmedFace {
    let mut surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 2.], vec![0., 1., 3.]],
            vec![vec![1., 0., 3.], vec![1., 1., 4.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    if !tilted {
        for pole in surface.control_points.iter_mut().flatten() {
            pole[2] = 2.;
        }
    }
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points: vec![
            vec![0., 0.],
            vec![1., 0.],
            vec![1., 1.],
            vec![0., 1.],
            vec![0., 0.],
        ],
        weights: vec![1.; 5],
        periodic: false,
    };
    let report = project_coplanar_trim_loops(&[surface], &[vec![vec![curve]]], WORK).unwrap();
    let PlanarTrimOutcome::Projected(mut faces) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    faces.remove(0)
}

fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}

#[test]
fn detects_exact_isotropy_of_a_flat_square() {
    let result = exact_projected_face_principal_moments(&chart(false), WORK).unwrap();
    let ExactPrincipalPlaneOutcome::Moments(moments) = result.outcome else {
        panic!("{:?}", result.outcome)
    };
    assert_eq!(moments.midpoint, q(1, 12));
    assert_eq!(moments.half_gap_squared, q(0, 1));
    assert_eq!(moments.area_scale_squared, q(1, 1));
    assert!(moments.is_isotropic());
}

#[test]
fn retains_distinct_algebraic_moments_of_a_tilted_face() {
    // The signed eigenvalues before the metric factor are -1/4 and -1/12.
    let result = exact_projected_face_principal_moments(&chart(true), WORK).unwrap();
    let ExactPrincipalPlaneOutcome::Moments(moments) = result.outcome else {
        panic!("{:?}", result.outcome)
    };
    assert_eq!(moments.midpoint, q(-1, 6));
    assert_eq!(moments.half_gap_squared, q(1, 144));
    assert_eq!(moments.area_scale_squared, q(3, 1));
    assert!(!moments.is_isotropic());
}

#[test]
fn distinguishes_zero_signed_area_from_exhausted_spectrum_budget() {
    let mut face = chart(false);
    let budget = exact_projected_face_mass(&face, WORK).unwrap().work_used;
    let limited = exact_projected_face_principal_moments(&face, budget).unwrap();
    assert!(matches!(
        limited.outcome,
        ExactPrincipalPlaneOutcome::SpectrumRefused(_)
    ));
    let reverse = face.loops[0]
        .iter()
        .rev()
        .map(|curve| curve.reverse().unwrap())
        .collect();
    face.loops.push(reverse);
    assert_eq!(
        exact_projected_face_principal_moments(&face, WORK)
            .unwrap()
            .outcome,
        ExactPrincipalPlaneOutcome::ZeroSignedArea
    );
}

#[test]
fn algebraic_projectors_select_the_correct_tilted_principal_axes() {
    use nurbs_core::analysis::exact_planar_face_mass::{
        ExactPrincipalDirectionOutcome, exact_projected_face_principal_directions,
    };
    let report = exact_projected_face_principal_directions(&chart(true), WORK).unwrap();
    let ExactPrincipalDirectionOutcome::Directions(axes) = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(axes.moments.half_gap_squared, q(1, 144));
    let d = axes.deviator.unwrap();
    let small = [1, 1, 2];
    let large = [1, -1, 0];
    for i in 0..3 {
        for j in 0..3 {
            let half = q(1, 2);
            let signed = &d[i][j] * q(12, 1);
            assert_eq!(
                (&axes.plane_projector[i][j] - &signed) * &half,
                q(small[i] * small[j], 6)
            );
            assert_eq!(
                (&axes.plane_projector[i][j] + &signed) * &half,
                q(large[i] * large[j], 2)
            );
        }
    }
}

#[test]
fn retains_the_plane_without_choosing_axes_for_an_isotropic_face() {
    use nurbs_core::analysis::exact_planar_face_mass::{
        ExactPrincipalDirectionOutcome, exact_projected_face_principal_directions,
    };
    let report = exact_projected_face_principal_directions(&chart(false), WORK).unwrap();
    let ExactPrincipalDirectionOutcome::Directions(axes) = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(axes.moments.is_isotropic());
    assert!(axes.deviator.is_none());
    assert_eq!(
        axes.plane_projector,
        [
            [q(1, 1), q(0, 1), q(0, 1)],
            [q(0, 1), q(1, 1), q(0, 1)],
            [q(0, 1), q(0, 1), q(0, 1)],
        ]
    );
}
