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
