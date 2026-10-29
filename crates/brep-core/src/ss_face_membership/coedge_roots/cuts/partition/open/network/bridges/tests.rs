use super::super::tests::{combined, request};
use super::*;
use crate::ss_face_membership::tests::{curves, polygon_region};
fn fixture(c: &ss::QualifiedGraphCurves, mixed: bool) -> [Model; 2] {
    let outer = vec![
        [0.375, 0.],
        [1., 0.],
        [1., 1.],
        [0.375, 1.],
        [0.375, 0.75],
        [0.375, 0.5625],
        [0.375, 0.4375],
        [0.375, 0.25],
    ];
    let hole = vec![
        [0.5625, 0.4375],
        [0.875, 0.4375],
        [0.875, 0.5625],
        [0.5625, 0.5625],
    ];
    let other_outer = if mixed {
        vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
    } else {
        outer.clone()
    };
    let other_hole = if mixed {
        vec![
            [0.125, 0.25],
            [0.375, 0.25],
            [0.375, 0.4375],
            [0.5625, 0.4375],
            [0.875, 0.4375],
            [0.875, 0.5625],
            [0.5625, 0.5625],
            [0.375, 0.5625],
            [0.375, 0.75],
            [0.125, 0.75],
        ]
    } else {
        hole.clone()
    };
    [
        polygon_region(c.original().source().first(), outer, hole),
        polygon_region(c.original().source().second(), other_outer, other_hole),
    ]
}
fn check(
    paths: [&QualifiedOpenSSPath<'_, '_, '_, '_>; 2],
    r: &QualifiedOpenSSNetwork<'_, '_, '_, '_, '_>,
    mixed: bool,
) {
    assert_eq!(r.faces()[0].len(), 2);
    assert_eq!(r.faces()[1].len(), if mixed { 3 } else { 2 });
    assert_eq!(r.cuts().len(), 2);
    for cut in r.cuts() {
        assert_eq!(
            cut.bridge_groups,
            [Some(0), if mixed { None } else { Some(0) }]
        );
        let path = paths[cut.path];
        for side in 0..2 {
            let o = &cut.sides[side];
            let m = &r.models()[o.model];
            for (i, &e) in o.edges.iter().enumerate() {
                assert_eq!(m.edges[e].curve, path.segments()[i].curve);
                let uses: Vec<_> = m
                    .faces
                    .iter()
                    .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
                    .flat_map(|l| &m.loops[l].coedges)
                    .filter(|c| c.edge == e)
                    .collect();
                assert_eq!(uses.len(), 2);
                assert_ne!(uses[0].reversed, uses[1].reversed);
                assert_eq!(
                    m.1.local_tolerances_mm[&m.1.edges[e]],
                    path.world_tolerance().max(1e-10)
                );
            }
        }
    }
    assert_eq!(r.ports().len(), 2);
    r.reverify().unwrap();
}
#[test]
fn source_ellipse_bridges_author_actual_both_side_contours_and_mixed_topology() {
    for mixed in [false, true] {
        let c = curves(0.0625);
        let m = fixture(&c, mixed);
        let before = m.clone();
        let roots = qualify_coedge_roots(
            c.original(),
            [(&m[0], 0), (&m[1], 0)],
            1e-8,
            Default::default(),
        )
        .unwrap();
        let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
        assert_eq!(reps.nodes().len(), 4);
        let input = partition_cut_representatives(&reps, Default::default()).unwrap();
        let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
        let bottom =
            qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
        let result = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
        check([&top, &bottom], &result, mixed);
        let mut result = result;
        let groups = result.cuts[0].bridge_groups;
        result.cuts[0].bridge_groups = [None; 2];
        assert!(result.reverify().is_err());
        result.cuts[0].bridge_groups = groups;
        result.reverify().unwrap();
        let swapped = partition_open_ss_network(&[&bottom, &top], Default::default()).unwrap();
        check([&bottom, &top], &swapped, mixed);
        let mut reverse = top.request().clone();
        std::mem::swap(&mut reverse.start_node, &mut reverse.end_node);
        reverse.steps.reverse();
        for s in &mut reverse.steps {
            s.reversed = !s.reversed;
        }
        let reverse = qualify_open_ss_path(&input, reverse, Default::default()).unwrap();
        let reversed = partition_open_ss_network(&[&reverse, &bottom], Default::default()).unwrap();
        check([&reverse, &bottom], &reversed, mixed);
        for limits in [
            OpenSSFaceLimits {
                cells: 1,
                ..Default::default()
            },
            OpenSSFaceLimits {
                pairs: 1,
                ..Default::default()
            },
            OpenSSFaceLimits {
                operations: result.operations() - 1,
                ..Default::default()
            },
        ] {
            assert!(partition_open_ss_network(&[&top, &bottom], limits).is_err());
        }
        assert_eq!(input.models()[0].faces.len(), 1);
        assert_eq!(input.models()[1].faces.len(), 1);
        assert_eq!(m, before);
    }
}
#[test]
fn shared_model_keeps_both_bridge_and_mixed_operand_edits() {
    for mixed in [false, true] {
        let c = curves(0.0625);
        let source = fixture(&c, mixed);
        let m = combined(&source[0], &source[1]);
        let before = m.clone();
        let roots =
            qualify_coedge_roots(c.original(), [(&m, 0), (&m, 1)], 1e-8, Default::default())
                .unwrap();
        let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
        let input = partition_cut_representatives(&reps, Default::default()).unwrap();
        let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
        let bottom =
            qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
        let r = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
        assert_eq!(r.models().len(), 1);
        assert_eq!(r.models()[0].faces.len(), if mixed { 5 } else { 4 });
        assert_eq!(r.models()[0].shells[0].faces.len(), 2);
        assert_eq!(
            r.models()[0].shells[1].faces.len(),
            if mixed { 3 } else { 2 }
        );
        check([&top, &bottom], &r, mixed);
        assert_eq!(m, before);
    }
}

#[test]
fn step_exports_the_actual_mixed_ss_result_with_its_new_curves_and_native_clocks() {
    let c = curves(0.0625);
    let m = fixture(&c, true);
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let bottom = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let r = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
    for side in 0..2 {
        let actual = &r.models()[side];
        let (text, export, _) = crate::export_step_v9(actual).unwrap();
        assert!(export.complete);
        let (imported, certificate, _) = crate::import_step_v9(&text).unwrap();
        assert!(certificate.complete);
        assert_eq!(imported.faces.len(), actual.faces.len());
        assert_eq!(imported.edges.len(), actual.edges.len());
        assert!(imported.faces.len() > m[side].faces.len());
        for cut in r.cuts() {
            for &edge in &cut.sides[side].edges {
                let curve = &actual.edges[edge].curve;
                let reversed = curve.reverse().unwrap();
                let restored = imported
                    .edges
                    .iter()
                    .position(|e| e.curve == *curve || e.curve == reversed)
                    .expect("Edited original SS bridge curve must survive export/import");
                let clocks = |m: &Model, e: usize| {
                    let mut c: Vec<_> = m
                        .faces
                        .iter()
                        .flat_map(|f| std::iter::once(f.outer).chain(f.holes.iter().copied()))
                        .flat_map(|w| &m.loops[w].coedges)
                        .filter(|c| c.edge == e)
                        .map(|c| c.pcurve.domain())
                        .collect();
                    c.sort_by(|a, b| a[0].total_cmp(&b[0]).then_with(|| a[1].total_cmp(&b[1])));
                    c
                };
                assert_eq!(clocks(&imported, restored), clocks(actual, edge));
            }
        }
    }
}

#[test]
fn original_ss_bridges_between_two_holes_author_a_cap_and_merged_hole_on_both_sides() {
    use crate::ss_face_membership::tests::polygon_regions;
    let c = curves(0.0625);
    let outer = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let holes = vec![
        vec![
            [0.125, 0.4375],
            [0.4375, 0.4375],
            [0.4375, 0.5625],
            [0.125, 0.5625],
        ],
        vec![
            [0.5625, 0.4375],
            [0.875, 0.4375],
            [0.875, 0.5625],
            [0.5625, 0.5625],
        ],
    ];
    let m = [
        polygon_regions(c.original().source().first(), outer.clone(), holes.clone()),
        polygon_regions(c.original().source().second(), outer, holes),
    ];
    let before = m.clone();
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 4);
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let bottom = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let r = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
    check([&top, &bottom], &r, false);
    for side in 0..2 {
        assert_eq!(r.models()[side].faces[r.faces()[side][0]].holes.len(), 1);
        assert!(r.models()[side].faces[r.faces()[side][1]].holes.is_empty());
    }
    partition_open_ss_network(&[&bottom, &top], Default::default())
        .unwrap()
        .reverify()
        .unwrap();
    assert_eq!(m, before);
}
