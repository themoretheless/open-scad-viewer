use nurbs_core::{ss_intersection as ss, surface::Surface};
#[test]
fn complete_original_control_grids_use_caller_work_instead_of_a_fixed_replay_cap() {
    let knots = std::iter::once(0.)
        .chain((0..32).map(|i| i as f64 / 31.))
        .chain(std::iter::once(1.))
        .collect::<Vec<_>>();
    let surface = |second: bool| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: knots.clone(),
        knots_v: knots.clone(),
        control_points: (0..32)
            .map(|i| {
                (0..32)
                    .map(|j| {
                        let (u, v) = (i as f64 / 31., j as f64 / 31.);
                        if second {
                            vec![u, 0.5, -1. + 2. * v]
                        } else {
                            vec![0.25 + 0.25 * u, v, 0.]
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 32]; 32],
        periodic_u: false,
        periodic_v: false,
    };
    let sources = ss::FiberSources::new(&surface(false), &surface(true)).unwrap();
    let original = sources
        .certify(
            [[0., 1.]; 4],
            ss::FiberAxis::FirstU,
            ss::FiberLimits::default(),
        )
        .unwrap()
        .into_fiber()
        .unwrap();
    assert!(original.report().rectangles > 1024);
    let limits = ss::FiberTraceBuildLimits::default();
    let trace = ss::construct_fiber_trace(&original, limits).unwrap();
    assert!(trace.cells <= limits.max_cells);
    let pieces = trace.trace.as_ref().expect(trace.reason);
    assert_eq!(
        pieces.len(),
        1,
        "An already qualified straight original branch needs one cubic proposal"
    );
    pieces[0].reverify().unwrap();
}
