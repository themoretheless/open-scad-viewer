use nurbs_core::{curve::Curve, curve_pullback, surface::Surface};
fn graph() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64,
                            j as f64,
                            if i == 2 { 1. } else { 0. } + if j == 2 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn curved_graph_retains_rational_circle_contact_instead_of_uv_chords() {
    // x=cos(t), y=sin(t), z=(x²+y²)/4=1/4.
    let w = 0.5f64.sqrt();
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.25], vec![1., 1., 0.25], vec![0., 1., 0.25]],
        weights: vec![1., w, 1.],
        periodic: false,
    };
    let s = graph();
    let before = (c.clone(), s.clone());
    let r = curve_pullback::project(
        &c,
        &s,
        curve_pullback::Limits {
            tolerance: 1e-7,
            max_segments: 1,
            max_iterations: 8,
            agreement_cells: 10000,
        },
    )
    .unwrap();
    assert!(r.within_tolerance);
    let pc = r.pcurve.unwrap();
    assert_eq!(pc.degree, 2);
    assert_eq!(pc.weights, c.weights);
    assert_eq!(
        pc.control_points,
        vec![vec![0.5, 0.], vec![0.5, 0.5], vec![0., 0.5]]
    );
    assert_eq!((c, s), before);
}
#[test]
fn graph_routing_never_admits_wrong_height_from_projected_coordinates_alone() {
    let c = Curve::from_polyline(vec![vec![0., 0., 10.], vec![1., 1., 10.]]).unwrap();
    let r = curve_pullback::project(
        &c,
        &graph(),
        curve_pullback::Limits {
            tolerance: 1e-7,
            max_segments: 2,
            max_iterations: 8,
            agreement_cells: 10000,
        },
    )
    .unwrap();
    assert!(!r.within_tolerance);
}

#[test]
fn graph_edge_roundoff_is_requalified_but_real_out_of_chart_error_is_refused() {
    let make = |x| Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![x, 0., 1.], vec![2., 1., 1.], vec![1., 1., 0.5]],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    };
    let limits = || curve_pullback::Limits {
        tolerance: 1e-8,
        max_segments: 2,
        max_iterations: 8,
        agreement_cells: 10000,
    };
    let tiny = make(2f64.next_up());
    let before = tiny.clone();
    let r = curve_pullback::project(&tiny, &graph(), limits()).unwrap();
    assert!(r.within_tolerance);
    assert!(r.pcurve.unwrap().control_points.iter().all(|p| p[0] <= 1.));
    assert_eq!(tiny, before);
    assert!(
        !curve_pullback::project(&make(2.01), &graph(), limits())
            .unwrap()
            .within_tolerance
    );
}

#[test]
fn oblique_height_graph_retains_the_original_rational_parameter_clock() {
    let transform = |p: &[f64]| {
        vec![
            0.36 * p[0] - 0.8 * p[1] + 0.48 * p[2] + 11.,
            0.48 * p[0] + 0.6 * p[1] + 0.64 * p[2] - 7.,
            -0.8 * p[0] + 0.6 * p[2] + 5.,
        ]
    };
    let mut s = graph();
    for p in s.control_points.iter_mut().flatten() {
        *p = transform(p);
    }
    let mut c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.25], vec![1., 1., 0.25], vec![0., 1., 0.25]],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    };
    for p in &mut c.control_points {
        *p = transform(p);
    }
    let out = curve_pullback::project(
        &c,
        &s,
        curve_pullback::Limits {
            tolerance: 1e-7,
            max_segments: 1,
            max_iterations: 8,
            agreement_cells: 10000,
        },
    )
    .unwrap();
    assert!(out.within_tolerance);
    let pc = out.pcurve.unwrap();
    assert_eq!(pc.degree, c.degree);
    assert_eq!(pc.weights, c.weights);
    for (p, q) in pc
        .control_points
        .iter()
        .zip([[0.5, 0.], [0.5, 0.5], [0., 0.5]])
    {
        assert!((p[0] - q[0]).hypot(p[1] - q[1]) < 1e-12);
    }
}
