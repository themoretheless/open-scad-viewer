use num_rational::BigRational;
use nurbs_core::{
    editing::exact_surface_derivative::{
        Axis, CurvatureGapGraph, Refusal, curvature_gap_graph_slope,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn sheared_parabola(shear: f64, v_end: f64) -> Surface {
    // S=(x,s,x²) with x = a + b*t + c*s. The squared gap depends
    // only on x, so a threshold contour has constant t'(s) = -c/b.
    let (a, b, c) = (0.25, 0.5, shear);
    let control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let (t, s) = (i as f64 * 0.5, j as f64 * 0.5);
                    let x = a + b * t + c * s;
                    let mut z = a * a + 2. * a * b * t + 2. * a * c * s + 2. * b * c * t * s;
                    if i == 2 {
                        z += b * b;
                    }
                    if j == 2 {
                        z += c * c;
                    }
                    vec![x, s, z]
                })
                .collect()
        })
        .collect();
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 6., 6., 6.],
        knots_v: vec![7., 7., 7., v_end, v_end, v_end],
        control_points,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn absolute_knot_clocks_scale_the_slope_and_reversal_changes_its_sign() {
    let mut s = sheared_parabola(0.125, 9.);
    let r = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., WORK).unwrap();
    assert_eq!(
        r.graph,
        Some(CurvatureGapGraph::Unique { increasing: false })
    );
    assert_eq!(r.slope, Some([-0.5, -0.5]));
    s.control_points.reverse();
    let r = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., WORK).unwrap();
    assert_eq!(
        r.graph,
        Some(CurvatureGapGraph::Unique { increasing: true })
    );
    assert_eq!(r.slope, Some([0.5, 0.5]));
}

#[test]
fn nonbinary_negative_slope_rounds_outward_in_both_directions() {
    let s = sheared_parabola(0.125, 10.);
    let r = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., WORK).unwrap();
    let [lo, hi] = r.slope.unwrap();
    let expected = BigRational::new((-1).into(), 3.into());
    assert!(BigRational::from_float(lo).unwrap() < expected);
    assert!(expected < BigRational::from_float(hi).unwrap());
    assert_eq!(lo.next_up(), hi);
}

#[test]
fn zero_slope_is_exact_and_unproved_graph_yields_no_bounds() {
    let s = sheared_parabola(0., 9.);
    let axial = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., WORK).unwrap();
    assert_eq!(axial.slope, Some([0., 0.]));
    let transverse = curvature_gap_graph_slope(&s, [2, 2], Axis::V, 1., WORK).unwrap();
    assert_eq!(transverse.graph, Some(CurvatureGapGraph::Unresolved));
    assert!(transverse.slope.is_none());
}

#[test]
fn late_budget_refusal_discards_the_graph_and_slope() {
    let s = sheared_parabola(0.125, 9.);
    let full = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., WORK).unwrap();
    let short = curvature_gap_graph_slope(&s, [2, 2], Axis::U, 1., full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.graph.is_none() && short.slope.is_none());
}

#[test]
fn affine_contour_has_exact_zero_polyline_error_at_dyadic_samples() {
    use nurbs_core::editing::exact_surface_derivative::certify_curvature_gap_polyline;
    let s = sheared_parabola(0.125, 9.);
    let r =
        certify_curvature_gap_polyline(&s, [2, 2], Axis::U, 0.5, &[7., 8., 9.], 16, WORK).unwrap();
    assert_eq!(r.segment_error_upper, Some(vec![0., 0.]));
    let samples = r.samples.unwrap();
    assert_eq!(samples[0].root_interval, [0.5; 2]);
    assert_eq!(samples[2].root_interval, [0.25; 2]);
}

fn quadratic_transverse_chart() -> Surface {
    // S=(x,s,x²), x = 1/4 + t/2 + 3s²/32. Degree four in s retains
    // the quartic z term without rounding any poles.
    let mut surface = sheared_parabola(0., 9.);
    surface.degree_v = 4;
    surface.knots_v = [vec![7.; 5], vec![9.; 5]].concat();
    let (a, b, c) = (0.25, 0.5, 3. / 32.);
    surface.control_points = (0..3)
        .map(|i| {
            (0..5)
                .map(|j| {
                    let t = i as f64 * 0.5;
                    let c_s2 = c * j as f64 * (j as f64 - 1.) / 12.;
                    let x = a + b * t + c_s2;
                    let mut z = a * a + 2. * a * b * t + 2. * (a + b * t) * c_s2;
                    if i == 2 {
                        z += b * b;
                    }
                    if j == 4 {
                        z += c * c;
                    }
                    vec![x, j as f64 * 0.25, z]
                })
                .collect()
        })
        .collect();
    surface.weights = vec![vec![1.; 5]; 3];
    surface
}

#[test]
fn nonlinear_contour_error_encloses_the_analytic_chord_remainder() {
    use nurbs_core::editing::exact_surface_derivative::certify_curvature_gap_polyline;
    let s = quadratic_transverse_chart();
    let sample = |grid: &[f64]| {
        certify_curvature_gap_polyline(&s, [2, 4], Axis::U, 0.5, grid, 16, WORK).unwrap()
    };
    let coarse = sample(&[7., 9.]);
    let errors = coarse
        .segment_error_upper
        .unwrap_or_else(|| panic!("{:?} {:?}", coarse.graph, coarse.refusal));
    // u_abs = 2 + 4(1/2 - 3s²/16). Its full-span chord error is 3/16.
    assert!(errors[0] >= 3. / 16.);
    let fine = sample(&[7., 8., 9.]).segment_error_upper.unwrap();
    assert!(
        fine.iter()
            .all(|&error| error >= 3. / 64. && error <= errors[0] * 0.5)
    );
}

#[test]
fn polyline_gap_accounts_for_vertex_uncertainty_and_work_is_atomic() {
    use nurbs_core::editing::exact_surface_derivative::certify_curvature_gap_polyline;
    let s = sheared_parabola(0.125, 9.);
    let certify = |params: &[f64], work| {
        certify_curvature_gap_polyline(&s, [2, 2], Axis::U, 1., params, 10, work)
    };
    assert!(certify(&[7.25, 9.], WORK).is_err());
    assert!(certify(&[7., 8., 8., 9.], WORK).is_err());
    let full = certify(&[7., 8., 9.], WORK).unwrap();
    assert_eq!(full.segment_error_upper.unwrap(), vec![1. / 512.; 2]);
    let short = certify(&[7., 8., 9.], full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(Refusal::ResourceLimit));
    assert!(short.samples.is_none() && short.segment_error_upper.is_none());
}

#[test]
fn adaptive_polyline_meets_requested_tolerance_with_complete_ordered_cover() {
    use nurbs_core::editing::exact_surface_derivative::{
        AdaptiveCurvaturePolylineOptions, CurvaturePolylineStop, adapt_curvature_gap_polyline,
    };
    let s = quadratic_transverse_chart();
    let options = AdaptiveCurvaturePolylineOptions {
        max_error: 0.1,
        max_vertices: 32,
        root_depth: 16,
        max_work: WORK,
    };
    let r = adapt_curvature_gap_polyline(&s, [2, 4], Axis::U, 0.5, options).unwrap();
    assert_eq!(r.stop, Some(CurvaturePolylineStop::ToleranceMet), "{r:?}");
    let samples = r.polyline.samples.unwrap();
    let errors = r.polyline.segment_error_upper.unwrap();
    assert!(samples.len() > 2);
    assert_eq!(samples.first().unwrap().transverse, 7.);
    assert_eq!(samples.last().unwrap().transverse, 9.);
    assert_eq!(samples.len(), errors.len() + 1);
    for (pair, error) in samples.windows(2).zip(errors) {
        let h = pair[1].transverse - pair[0].transverse;
        assert!(h > 0. && error <= options.max_error);
        assert!(error >= (3. / 64.) * h * h);
    }
}

#[test]
fn vertex_limit_retains_bounds_but_exact_work_refuses_atomically() {
    use nurbs_core::editing::exact_surface_derivative::{
        AdaptiveCurvaturePolylineOptions, CurvaturePolylineStop, adapt_curvature_gap_polyline,
    };
    let s = quadratic_transverse_chart();
    let mut options = AdaptiveCurvaturePolylineOptions {
        max_error: 0.1,
        max_vertices: 2,
        root_depth: 16,
        max_work: WORK,
    };
    let full = adapt_curvature_gap_polyline(&s, [2, 4], Axis::U, 0.5, options).unwrap();
    assert_eq!(full.stop, Some(CurvaturePolylineStop::VertexLimit));
    assert_eq!(full.polyline.samples.unwrap().len(), 2);
    assert!(full.polyline.segment_error_upper.unwrap()[0] >= 3. / 16.);
    options.max_work = full.polyline.work_used - 1;
    let short = adapt_curvature_gap_polyline(&s, [2, 4], Axis::U, 0.5, options).unwrap();
    assert_eq!(short.polyline.refusal, Some(Refusal::ResourceLimit));
    assert!(short.stop.is_none() && short.polyline.samples.is_none());
}

#[test]
fn adjacent_transverse_floats_report_parameter_precision_limit() {
    use nurbs_core::editing::exact_surface_derivative::{
        AdaptiveCurvaturePolylineOptions, CurvaturePolylineStop, adapt_curvature_gap_polyline,
    };
    let mut s = sheared_parabola(0., 9.);
    s.knots_v[3..].fill(7f64.next_up());
    let options = AdaptiveCurvaturePolylineOptions {
        max_error: 1e-6,
        max_vertices: 8,
        root_depth: 2,
        max_work: WORK,
    };
    let result = adapt_curvature_gap_polyline(&s, [2, 2], Axis::U, 1., options).unwrap();
    assert_eq!(
        result.stop,
        Some(CurvaturePolylineStop::ParameterPrecision),
        "{result:?}"
    );
    assert_eq!(result.polyline.samples.unwrap().len(), 2);
    assert!(result.polyline.segment_error_upper.unwrap()[0] > options.max_error);
}
