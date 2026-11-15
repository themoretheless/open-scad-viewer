use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveSeamContact, UvAxis, classify_pcurve_seam},
    },
};

const WORK: u64 = 1_000_000;

fn bezier(v: &[f64], weights: Vec<f64>) -> Curve {
    Curve {
        degree: v.len() - 1,
        knots: [vec![2.; v.len()], vec![5.; v.len()]].concat(),
        control_points: v.iter().map(|&y| vec![1., y]).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn separates_real_crossing_and_touch_on_one_rational_span() {
    // Homogeneous V - 2W = 3(2t - 1)(4t - 1)^2.
    let source = bezier(&[-1., 5.5, -1.75, 29.], vec![1., 2., 4., 1.]);
    let report = classify_pcurve_seam(&source, UvAxis::V, 2., WORK).unwrap();
    let spans = report.spans.unwrap();
    assert_eq!(spans[0].parameter, [2., 5.]);
    assert_eq!(
        spans[0].contact,
        PcurveSeamContact::Discrete {
            crossings: 1,
            touches: 1,
            at_start: false,
            at_end: false,
        }
    );
}

#[test]
fn retains_one_sided_endpoints_and_an_entire_coincident_span() {
    let source = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 4., 4.],
        control_points: vec![vec![0., 0.], vec![1., 1.], vec![1., 2.], vec![2., 4.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let report = classify_pcurve_seam(&source, UvAxis::U, 1., WORK).unwrap();
    let spans = report.spans.unwrap();
    assert_eq!(spans.len(), 3);
    assert_eq!(
        spans[0].contact,
        PcurveSeamContact::Discrete {
            crossings: 0,
            touches: 0,
            at_start: false,
            at_end: true,
        }
    );
    assert_eq!(spans[1].parameter, [1., 2.]);
    assert_eq!(spans[1].contact, PcurveSeamContact::Coincident);
    assert_eq!(
        spans[2].contact,
        PcurveSeamContact::Discrete {
            crossings: 0,
            touches: 0,
            at_start: true,
            at_end: false,
        }
    );
}

#[test]
fn endpoint_roots_are_not_counted_as_interior_tangencies() {
    let source = bezier(&[0., 3., -8., 12., 0.], vec![1.; 5]);
    let report = classify_pcurve_seam(&source, UvAxis::V, 0., WORK).unwrap();
    assert_eq!(
        report.spans.unwrap()[0].contact,
        PcurveSeamContact::Discrete {
            crossings: 0,
            touches: 1,
            at_start: true,
            at_end: true,
        }
    );

    let report = classify_pcurve_seam(&source, UvAxis::V, 0., 1).unwrap();
    assert!(report.spans.is_none());
    assert_eq!(report.refusal, Some(Refusal::ResourceLimit));
    assert!(report.work_used <= 1);
}
