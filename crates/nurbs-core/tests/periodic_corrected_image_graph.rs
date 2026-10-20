use nurbs_core::{curve::Curve, periodic_trim as p};
#[test]
fn image_compaction_retains_actual_nonzero_perimeter_correction() {
    for reverse in [false, true] {
        let c = Curve::from_polyline(vec![
            vec![1.25, 1.],
            vec![1., 1.25],
            vec![0.75, 1.],
            vec![1., 0.75],
            vec![1.25, 1.],
        ])
        .unwrap();
        let c = if reverse { c.reverse().unwrap() } else { c };
        let tiles = if reverse {
            [[1, 0], [0, 0], [0, 1], [1, 1]]
        } else {
            [[1, 1], [0, 1], [0, 0], [1, 0]]
        };
        let part = p::split_candidates(&c, [[0., 1.]; 2], &[1., 2., 3.], &tiles, 16).unwrap();
        let limits = p::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 64,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let owner = p::assemble_clock_mapped_source_chart(
            &[c],
            &[part],
            &[[0; 2]],
            [[0., 1.]; 2],
            [0.5, 0.5],
            limits,
            &[],
            1_000_000,
        )
        .unwrap();
        owner.reverify().unwrap();
        let source = owner.source();
        let expected = if reverse { -1 } else { 1 };
        assert_eq!(source.calibration().unwrap().offset, Some(expected));
        assert_ne!(
            source.materialized_graph().unwrap(),
            &source.calibration().unwrap().raw_graph
        );
        assert_eq!(owner.graph(), source.graph());
        for (q, w) in [
            ([0.0625, 0.0625], expected),
            ([0.9375, 0.9375], expected),
            ([0.5, 0.5], 0),
        ] {
            let r = p::inspect_weighted_winding(
                owner.graph().unwrap(),
                [[0., 1.]; 2],
                q,
                1e-8,
                256,
                30000,
            )
            .unwrap();
            assert_eq!(r.winding, Some(w));
        }
    }
}
