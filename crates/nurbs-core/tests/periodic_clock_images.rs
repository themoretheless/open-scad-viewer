use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
fn source() -> Curve {
    Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![5., 0.], vec![0., 5.], vec![0., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    }
}
fn mapped() -> Curve {
    Curve {
        degree: 6,
        knots: [vec![0.; 7], vec![1.; 7]].concat(),
        control_points: vec![
            vec![0., 0.],
            vec![0., 0.],
            vec![1., 0.],
            vec![3., 0.],
            vec![4., 1.],
            vec![0., 5.],
            vec![0., 0.],
        ],
        weights: vec![1.; 7],
        periodic: false,
    }
}
fn clock() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
fn parts(c: &Curve, cut: f64) -> Report {
    Report {
        pieces: c
            .split(cut)
            .unwrap()
            .into_iter()
            .map(|curve| Piece {
                source_domain: curve.domain(),
                curve,
                translation: [0.; 2],
                tile: [0; 2],
            })
            .collect(),
        candidate_hulls_contained: false,
    }
}
fn limits() -> p::ChartAssemblyLimits {
    p::ChartAssemblyLimits {
        max_segments: 64,
        max_links: 32,
        max_edges: 32,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}
#[test]
fn exact_nonprojective_clock_binds_original_curves_and_orientation() {
    let a = source();
    let b = mapped();
    let c = clock();
    let proof = p::verify_curve_clock_correspondence(&a, &b, &c, 1_000_000)
        .unwrap()
        .unwrap();
    proof.reverify(1_000_000).unwrap();
    assert_eq!(proof.orientation(), 1);
    for t in [0_f64, 0.125, 0.5, 0.875, 1.] {
        let x = a.evaluate(t * t).unwrap().point;
        let y = b.evaluate(t).unwrap().point;
        assert!(x.iter().zip(y).all(|(&a, b)| (a - b).abs() < 1e-12));
    }
    let mut reverse = b;
    reverse.control_points.reverse();
    let mut reverse_clock = c;
    reverse_clock.control_points.reverse();
    let reverse = p::verify_curve_clock_correspondence(&a, &reverse, &reverse_clock, 1_000_000)
        .unwrap()
        .unwrap();
    assert_eq!(reverse.orientation(), -1);
    reverse.reverify(1_000_000).unwrap();
    assert!(
        p::verify_curve_clock_correspondence(&a, &mapped(), &clock(), proof.work_used() - 1)
            .unwrap()
            .is_none()
    );
    let mut damaged = mapped();
    damaged.control_points[2][0] = 1_f64.next_up();
    assert!(
        p::verify_curve_clock_correspondence(&a, &damaged, &clock(), 1_000_000)
            .unwrap()
            .is_none()
    );
}
#[test]
fn actual_split_clock_images_compact_complete_original_multiplicity() {
    let sources = vec![source(), mapped()];
    let partitions = vec![parts(&sources[0], 0.25), parts(&sources[1], 0.5)];
    let clocks = clock().split(0.5).unwrap();
    let specs = vec![
        p::ClockAliasSpec {
            edges: [0, 2],
            clock: clocks[0].clone(),
        },
        p::ClockAliasSpec {
            edges: [1, 3],
            clock: clocks[1].clone(),
        },
    ];
    let out = p::assemble_clock_mapped_source_chart(
        &sources,
        &partitions,
        &[[0, 0]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &specs,
        1_000_000,
    )
    .unwrap();
    let graph = out.graph().expect(out.reason().unwrap_or("mapped chart"));
    assert_eq!(graph.original_edges, 4);
    assert_eq!(graph.edges.len(), 2);
    assert!(
        graph
            .edges
            .iter()
            .all(|e| e.coefficient == 2 && e.contributions.len() == 2)
    );
    assert!(out.contact_ownership().unwrap().complete);
    out.reverify().unwrap();
    assert_eq!(out.source().calibration().unwrap().source.winding, Some(2));
    let tight = p::assemble_clock_mapped_source_chart(
        &sources,
        &partitions,
        &[[0, 0]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &specs,
        out.work_used(),
    )
    .unwrap();
    tight.reverify().unwrap();
    let short = p::assemble_clock_mapped_source_chart(
        &sources,
        &partitions,
        &[[0, 0]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &specs,
        out.work_used() - 1,
    )
    .unwrap();
    assert!(short.graph().is_none());
    assert!(short.work_used() <= out.work_used() - 1);
}

#[test]
fn opposite_nonlinear_source_images_cancel_without_duplicate_contact_rays() {
    let a = source();
    let mut b = mapped();
    b.control_points.reverse();
    let mut c = clock();
    c.control_points.reverse();
    let sources = vec![a, b];
    let partitions = vec![parts(&sources[0], 0.25), parts(&sources[1], 0.5)];
    let clocks = c.split(0.5).unwrap();
    let specs = vec![
        p::ClockAliasSpec {
            edges: [1, 2],
            clock: clocks[0].clone(),
        },
        p::ClockAliasSpec {
            edges: [0, 3],
            clock: clocks[1].clone(),
        },
    ];
    let out = p::assemble_clock_mapped_source_chart(
        &sources,
        &partitions,
        &[[0, 0]; 2],
        [[0., 8.]; 2],
        [1., 1.],
        limits(),
        &specs,
        1_000_000,
    )
    .unwrap();
    let g = out
        .graph()
        .expect(out.reason().unwrap_or("cancelled graph"));
    assert!(g.edges.is_empty());
    assert_eq!(g.cancellations.len(), 2);
    assert!(out.contact_ownership().is_none());
    assert_eq!(out.source().calibration().unwrap().source.winding, Some(0));
    out.reverify().unwrap();
}
