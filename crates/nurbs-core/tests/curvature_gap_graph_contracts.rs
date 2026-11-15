use nurbs_core::{
    editing::exact_surface_derivative::{
        Axis, CurvatureGapGraph, Refusal, certify_curvature_gap_graph,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn parabola() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![2., 2., 2., 6., 6., 6.],
        knots_v: vec![7., 7., 9., 9.],
        // S=(x,v,x²), x = 1/4 + (3/4)t. All poles are exact dyadics.
        control_points: [(0.25, 0.0625), (0.625, 0.25), (1., 1.)]
            .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn proves_the_single_contour_and_its_slope_in_both_orientations() {
    let mut s = parabola();
    let r = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 1., WORK).unwrap();
    assert_eq!(r.domain, [[2., 6.], [7., 9.]]);
    assert_eq!(
        r.graph,
        Some(CurvatureGapGraph::Unique { increasing: false })
    );
    s.control_points.reverse();
    let r = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 1., WORK).unwrap();
    assert_eq!(
        r.graph,
        Some(CurvatureGapGraph::Unique { increasing: true })
    );
}

#[test]
fn respects_the_chosen_root_axis_in_a_transposed_chart() {
    let mut s = parabola();
    let r = certify_curvature_gap_graph(&s, [2, 1], Axis::V, 1., WORK).unwrap();
    assert_eq!(r.graph, Some(CurvatureGapGraph::Unresolved));
    s.control_points = (0..2)
        .map(|v| s.control_points.iter().map(|row| row[v].clone()).collect())
        .collect();
    s.weights = vec![vec![1.; 3]; 2];
    std::mem::swap(&mut s.degree_u, &mut s.degree_v);
    std::mem::swap(&mut s.knots_u, &mut s.knots_v);
    let r = certify_curvature_gap_graph(&s, [1, 2], Axis::V, 1., WORK).unwrap();
    assert_eq!(
        r.graph,
        Some(CurvatureGapGraph::Unique { increasing: false })
    );
}

#[test]
fn distinguishes_an_empty_contour_and_an_identically_equal_cell() {
    let mut s = parabola();
    let r = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 4., WORK).unwrap();
    assert_eq!(r.graph, Some(CurvatureGapGraph::NoContour));
    s.control_points = [(1., 0.), (1., 1.), (0., 1.)]
        .map(|(x, y)| vec![vec![x, y, 0.], vec![x, y, 1.]])
        .to_vec();
    s.weights[2] = vec![2.; 2];
    let r = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 1., WORK).unwrap();
    assert_eq!(r.graph, Some(CurvatureGapGraph::EverywhereAtLevel));
}

#[test]
fn validates_original_span_and_returns_no_certificate_on_short_work() {
    let s = parabola();
    assert!(certify_curvature_gap_graph(&s, [0, 1], Axis::U, 1., WORK).is_err());
    let full = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 1., WORK).unwrap();
    let short = certify_curvature_gap_graph(&s, [2, 1], Axis::U, 1., full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.graph.is_none());
}

#[test]
fn samples_in_order_with_checked_brackets_in_both_orientations() {
    use nurbs_core::editing::exact_surface_derivative::sample_curvature_gap_graph;
    let mut s = parabola();
    let parameters = [9., 7.25, 8., 7.];
    let x = ((4f64.cbrt() - 1.) / 4.).sqrt();
    let t = (x - 0.25) / 0.75;
    for reverse in [false, true] {
        if reverse {
            s.control_points.reverse();
        }
        let r = sample_curvature_gap_graph(&s, [2, 1], Axis::U, 1., &parameters, 14, WORK).unwrap();
        let samples = r.samples.unwrap_or_else(|| panic!("{:?}", r.refusal));
        for (sample, expected) in samples.iter().zip(parameters) {
            assert_eq!(sample.transverse, expected);
            let [lo, hi] = sample.root_interval;
            let root = if reverse { 1. - t } else { t };
            assert!(lo < root && root < hi);
            assert_eq!(hi - lo, 1. / 16384.);
        }
        assert_eq!(samples.len(), parameters.len());
    }
}

#[test]
fn exact_midpoint_is_retained_as_a_single_parameter() {
    use nurbs_core::editing::exact_surface_derivative::sample_curvature_gap_graph;
    let mut s = parabola();
    s.control_points = [(0.25, 0.0625), (0.5, 0.1875), (0.75, 0.5625)]
        .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
        .to_vec();
    let r = sample_curvature_gap_graph(&s, [2, 1], Axis::U, 0.5, &[8.], 16, WORK).unwrap();
    assert_eq!(r.samples.unwrap()[0].root_interval, [0.5, 0.5]);
}

#[test]
fn sample_batch_validates_inputs_and_refuses_atomically() {
    use nurbs_core::editing::exact_surface_derivative::sample_curvature_gap_graph;
    let s = parabola();
    let sample = |axis, parameters: &[f64], work| {
        sample_curvature_gap_graph(&s, [2, 1], axis, 1., parameters, 8, work)
    };
    assert!(sample(Axis::U, &[6.99], WORK).is_err());
    assert!(sample(Axis::U, &[f64::NAN], WORK).is_err());
    assert!(sample(Axis::U, &[], WORK).is_err());
    let unresolved = sample(Axis::V, &[3.], WORK).unwrap();
    assert_eq!(unresolved.graph, Some(CurvatureGapGraph::Unresolved));
    assert!(unresolved.samples.is_none());
    let full = sample(Axis::U, &[7., 8.], WORK).unwrap();
    assert_eq!(full.samples.unwrap().len(), 2);
    let short = sample(Axis::U, &[7., 8.], full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.samples.is_none() && short.graph.is_none());
}
