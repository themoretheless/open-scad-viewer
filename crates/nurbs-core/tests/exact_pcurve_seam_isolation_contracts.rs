use nurbs_core::{
    curve::Curve,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{PcurveSeamIsolation, PcurveSeamRoot, UvAxis, isolate_pcurve_seam},
    },
};

const WORK: u64 = 1_000_000;

fn bezier(v: &[f64], weights: Vec<f64>) -> Curve {
    Curve {
        degree: v.len() - 1,
        knots: [vec![2.; v.len()], vec![6.; v.len()]].concat(),
        control_points: v.iter().map(|&y| vec![1., y]).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn finds_exact_binary_parameters_and_preserves_parity() {
    let source = bezier(&[-1., 5.5, -1.75, 29.], vec![1., 2., 4., 1.]);
    let result = isolate_pcurve_seam(&source, UvAxis::V, 2., 1e-9, WORK).unwrap();
    let spans = result
        .spans
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    assert_eq!(spans[0].parameter, [2., 6.]);
    assert_eq!(
        spans[0].contact,
        PcurveSeamIsolation::Discrete {
            at_start: false,
            at_end: false,
            roots: vec![
                PcurveSeamRoot {
                    parameter: [3.; 2],
                    crosses: false
                },
                PcurveSeamRoot {
                    parameter: [4.; 2],
                    crosses: true
                },
            ],
        }
    );
}

#[test]
fn encloses_a_nonbinary_contact_and_retains_both_span_endpoints() {
    // V = 12t(1-t)(3t-1)^2, with normalized t in [0,1].
    let source = bezier(&[0., 3., -8., 12., 0.], vec![1.; 5]);
    let tol = 1e-7;
    let result = isolate_pcurve_seam(&source, UvAxis::V, 0., tol, WORK).unwrap();
    let spans = result.spans.unwrap();
    let PcurveSeamIsolation::Discrete {
        at_start: true,
        at_end: true,
        roots,
    } = &spans[0].contact
    else {
        panic!("endpoint ownership was lost");
    };
    assert_eq!(roots.len(), 1);
    let [lo, hi] = roots[0].parameter;
    assert!(lo < 10. / 3. && 10. / 3. < hi);
    assert!(hi - lo <= tol);
    assert!(!roots[0].crosses);
}

#[test]
fn coincident_spans_and_unresolvable_width_have_distinct_outcomes() {
    let source = bezier(&[-1., 2.], vec![1.; 2]);
    let coincident = isolate_pcurve_seam(&source, UvAxis::U, 1., 1e-10, WORK).unwrap();
    assert_eq!(
        coincident.spans.unwrap()[0].contact,
        PcurveSeamIsolation::Coincident
    );
    let narrow = isolate_pcurve_seam(&source, UvAxis::V, 0., 1e-20, WORK).unwrap();
    assert!(narrow.spans.is_none());
    assert_eq!(narrow.refusal, Some(Refusal::NotRepresentable));
    let limited = isolate_pcurve_seam(&source, UvAxis::V, 0., 1e-10, 1).unwrap();
    assert!(limited.spans.is_none());
    assert_eq!(limited.refusal, Some(Refusal::ResourceLimit));
    assert!(limited.work_used <= 1);
}
