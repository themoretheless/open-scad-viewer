use num_rational::BigRational as Q;
use nurbs_core::{
    curve::Curve,
    editing::exact_derivative::{ExactPlanarAreaOutcome, Refusal, exact_planar_boundary_area},
};

const WORK: u64 = 1_000_000;

fn curve(points: &[[f64; 2]], weights: Vec<f64>) -> Curve {
    Curve {
        degree: points.len() - 1,
        knots: [vec![2.; points.len()], vec![6.; points.len()]].concat(),
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights,
        periodic: false,
    }
}

#[test]
fn integrates_a_quadratic_arch_and_nonuniform_rational_closing_segment() {
    let arch = curve(&[[0., 0.], [1., 2.], [2., 0.]], vec![2.; 3]);
    let base = curve(&[[2., 0.], [0., 0.]], vec![1., 4.]);
    let report = exact_planar_boundary_area(&[arch, base], WORK).unwrap();
    assert_eq!(
        report.outcome,
        ExactPlanarAreaOutcome::Area(Q::new((-4).into(), 3.into()))
    );
}

#[test]
fn integrates_a_rational_multispan_polygon_with_no_sampling() {
    let polygon = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 3., 4., 6., 6.],
        control_points: vec![
            vec![5., -3.],
            vec![7., -3.],
            vec![7., -2.],
            vec![5., -2.],
            vec![5., -3.],
        ],
        weights: vec![1., 4., 2., 8., 1.],
        periodic: false,
    };
    assert_eq!(
        exact_planar_boundary_area(&[polygon], WORK)
            .unwrap()
            .outcome,
        ExactPlanarAreaOutcome::Area(Q::from_integer(2.into()))
    );
}

#[test]
fn distinguishes_unsupported_rational_spans_open_loops_and_budget() {
    let arch = curve(&[[0., 0.], [1., 2.], [2., 0.]], vec![1., 2., 1.]);
    let base = curve(&[[2., 0.], [0., 0.]], vec![1., 4.]);
    assert_eq!(
        exact_planar_boundary_area(&[arch.clone()], WORK)
            .unwrap()
            .outcome,
        ExactPlanarAreaOutcome::UnprovedClosure
    );
    let loop_curves = [arch, base];
    assert_eq!(
        exact_planar_boundary_area(&loop_curves, WORK)
            .unwrap()
            .outcome,
        ExactPlanarAreaOutcome::UnsupportedRationalSpan { curve: 0, span: 2 }
    );
    let report = exact_planar_boundary_area(&loop_curves, 1).unwrap();
    assert_eq!(
        report.outcome,
        ExactPlanarAreaOutcome::Refused(Refusal::ResourceLimit)
    );
    assert!(report.work_used <= 1);
}
