use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveAngleBandsOutcome, PcurvePullbackRefusal, isolate_pcurve_seam_angle,
        },
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

#[test]
fn isolates_an_interior_crease_with_certified_nonbinary_bounds() {
    let flat = face([1.; 3], [0.; 3]);
    let bend = face([1.; 3], [0., 2., 0.]);
    let uv = trace();
    let width = 1e-7;
    let report = isolate_pcurve_seam_angle((&flat, &uv), (&bend, &uv), 0.8, width, WORK).unwrap();
    let PcurveAngleBandsOutcome::Bands(bands) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(bands.len(), 1);
    let t = |s: f64| 3. + 4. * s / (2. - s);
    for (b, root) in [
        (bands[0].start, t((1. - 0.5_f64.sqrt()) / 2.)),
        (bands[0].end, t((1. + 0.5_f64.sqrt()) / 2.)),
    ] {
        let [lo, hi] = b.bracket;
        assert!(lo < root && root < hi && hi - lo <= width);
        assert!(!b.included);
    }
}

#[test]
fn equality_at_the_maximum_has_no_strict_violation_band() {
    let flat = face([1.; 3], [0.; 3]);
    let bend = face([1.; 3], [0., 2., 0.]);
    let uv = trace();
    let result = isolate_pcurve_seam_angle((&flat, &uv), (&bend, &uv), 0.5, 1e-9, WORK).unwrap();
    assert_eq!(result.outcome, PcurveAngleBandsOutcome::Bands(vec![]));
}

#[test]
fn singular_normals_and_exhaustion_withhold_bands() {
    let flat = face([1.; 3], [0.; 3]);
    let singular = face([-1., 0., 1.], [0.; 3]);
    let uv = trace();
    let result =
        isolate_pcurve_seam_angle((&flat, &uv), (&singular, &uv), 0.5, 1e-9, WORK).unwrap();
    assert_eq!(
        result.outcome,
        PcurveAngleBandsOutcome::Singular {
            parameter: [3., 7.],
            face: 1
        }
    );
    let result = isolate_pcurve_seam_angle((&flat, &uv), (&singular, &uv), 0.5, 1e-9, 1).unwrap();
    assert_eq!(
        result.outcome,
        PcurveAngleBandsOutcome::Refused(PcurvePullbackRefusal::Geometry(Refusal::ResourceLimit))
    );
}
