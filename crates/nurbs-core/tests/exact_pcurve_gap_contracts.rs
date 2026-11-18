use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveGapOutcome, PcurvePullbackRefusal, certify_pcurve_seam_gap},
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face(x_bulge: f64, z_bulge: f64) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 4., 4., 4.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [0., 2., 4.]
            .into_iter()
            .map(|u| {
                let (x, z) = if u == 2. {
                    (x_bulge, z_bulge)
                } else {
                    (0., 0.)
                };
                vec![vec![x, u, z], vec![1. + x, u, z]]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn uv(weights: Vec<f64>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights,
        periodic: false,
    }
}

#[test]
fn verifies_equality_at_an_interior_rational_gap_maximum() {
    let a = face(0., 0.);
    let b = face(0., 0.25);
    let curve = uv(vec![1., 2.]);
    let exact = certify_pcurve_seam_gap((&a, &curve), (&b, &curve), 0.125, WORK).unwrap();
    assert_eq!(exact.outcome, PcurveGapOutcome::WithinTolerance);
    let too_small = f64::from_bits(0.125_f64.to_bits() - 1);
    let below = certify_pcurve_seam_gap((&a, &curve), (&b, &curve), too_small, WORK).unwrap();
    assert_eq!(
        below.outcome,
        PcurveGapOutcome::ExceedsTolerance {
            parameter: [3., 7.]
        }
    );
}

#[test]
fn uses_the_euclidean_gap_across_all_three_coordinates() {
    let a = face(0., 0.);
    let b = face(0.25, 0.25);
    let curve = uv(vec![1.; 2]);
    let result = certify_pcurve_seam_gap((&a, &curve), (&b, &curve), 0.125, WORK).unwrap();
    assert_eq!(
        result.outcome,
        PcurveGapOutcome::ExceedsTolerance {
            parameter: [3., 7.]
        }
    );
    let result = certify_pcurve_seam_gap((&a, &curve), (&b, &curve), 0.1875, WORK).unwrap();
    assert_eq!(result.outcome, PcurveGapOutcome::WithinTolerance);
}

#[test]
fn handles_zero_tolerance_and_distinguishes_exhaustion() {
    let a = face(0., 0.);
    let curve = uv(vec![1.; 2]);
    let result = certify_pcurve_seam_gap((&a, &curve), (&a, &curve), 0., WORK).unwrap();
    assert_eq!(result.outcome, PcurveGapOutcome::WithinTolerance);
    let result = certify_pcurve_seam_gap((&a, &curve), (&a, &curve), 0., 1).unwrap();
    assert_eq!(
        result.outcome,
        PcurveGapOutcome::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
    assert!(result.work_used <= 1);
    assert!(certify_pcurve_seam_gap((&a, &curve), (&a, &curve), -1., WORK).is_err());
}
