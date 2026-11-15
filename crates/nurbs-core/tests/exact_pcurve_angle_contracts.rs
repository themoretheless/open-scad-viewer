use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveAngleOutcome, PcurvePullbackRefusal, certify_pcurve_seam_angle},
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn face(x: [f64; 3], z: [f64; 3]) -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 4., 4., 4.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                let u = 2. * i as f64;
                vec![vec![0., u, 0.], vec![x[i], u, z[i]]]
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn trace() -> Curve {
    Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![vec![0., 0.], vec![4., 0.]],
        weights: vec![1., 2.],
        periodic: false,
    }
}

fn test(a: &Surface, b: &Surface, cosine: f64, work: u64) -> PcurveAngleOutcome {
    let uv = trace();
    certify_pcurve_seam_angle((a, &uv), (b, &uv), cosine, work)
        .unwrap()
        .outcome
}

#[test]
fn proves_an_exact_interior_angular_maximum_with_rational_clock() {
    let flat = face([1.; 3], [0.; 3]);
    let bent = face([1.; 3], [0., 2., 0.]);
    assert_eq!(
        test(&flat, &bent, 0.5, WORK),
        PcurveAngleOutcome::WithinTolerance
    );
    let tighter = f64::from_bits(0.5_f64.to_bits() + 1);
    assert_eq!(
        test(&flat, &bent, tighter, WORK),
        PcurveAngleOutcome::ExceedsTolerance {
            parameter: [3., 7.]
        }
    );
}

#[test]
fn handles_reversed_normals_and_perpendicular_planes() {
    let flat = face([1.; 3], [0.; 3]);
    assert_eq!(
        test(&flat, &face([-1.; 3], [0.; 3]), 1., WORK),
        PcurveAngleOutcome::WithinTolerance
    );
    let vertical = face([0.; 3], [1.; 3]);
    assert_eq!(
        test(&flat, &vertical, 0., WORK),
        PcurveAngleOutcome::WithinTolerance
    );
    assert_eq!(
        test(&flat, &vertical, 0.25, WORK),
        PcurveAngleOutcome::ExceedsTolerance {
            parameter: [3., 7.]
        }
    );
}

#[test]
fn rejects_an_isolated_singular_plane_even_at_the_largest_angle() {
    let flat = face([1.; 3], [0.; 3]);
    let singular = face([-1., 0., 1.], [0.; 3]);
    assert_eq!(
        test(&flat, &singular, 0., WORK),
        PcurveAngleOutcome::Singular {
            parameter: [3., 7.],
            face: 1
        }
    );
    assert_eq!(
        test(&flat, &singular, 0., 1),
        PcurveAngleOutcome::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}
