use super::*;
fn cube(low: Point, high: Point) -> Mesh {
    let mut positions = Vec::new();
    for [x, y, z] in [
        [0, 0, 0],
        [1, 0, 0],
        [1, 1, 0],
        [0, 1, 0],
        [0, 0, 1],
        [1, 0, 1],
        [1, 1, 1],
        [0, 1, 1],
    ] {
        positions.extend([
            if x == 0 { low[0] } else { high[0] },
            if y == 0 { low[1] } else { high[1] },
            if z == 0 { low[2] } else { high[2] },
        ]);
    }
    Mesh {
        positions,
        indices: vec![
            0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7,
            6, 3, 0, 4, 3, 4, 7,
        ],
        uv: None,
    }
}
fn run(a: &Mesh, b: &Mesh, op: Operation, volume: f64) -> BuiltMesh {
    let out = boolean(a, b, op, &Options::default()).unwrap_or_else(|e| panic!("{op:?}: {e:?}"));
    assert!(
        (out.report.signed_volume_mm3 - volume).abs() < 1e-8,
        "{op:?}: {} vs {volume}",
        out.report.signed_volume_mm3
    );
    if volume > 0. {
        assert!(out.report.closed);
        assert_eq!(out.report.boundary_edges, 0);
        assert_eq!(out.report.non_manifold_edges, 0);
        assert_eq!(out.report.orientation_conflicts, 0);
        vertex_manifold(&out.mesh).unwrap();
    } else {
        assert!(out.mesh.indices.is_empty());
        assert!(out.mesh.positions.is_empty());
    }
    assert!(out.mesh.uv.is_none());
    out
}
#[test]
fn overlapping_boxes_all_operations() {
    let a = cube([0., 0., 0.], [2., 2., 2.]);
    let b = cube([1., 1., 1.], [3., 3., 3.]);
    let before = a.positions.clone();
    run(&a, &b, Operation::Union, 15.);
    run(&a, &b, Operation::Intersection, 1.);
    run(&a, &b, Operation::Difference, 7.);
    assert_eq!(a.positions, before);
}
#[test]
fn coincident_coplanar_and_empty_cases() {
    let a = cube([0.; 3], [2.; 3]);
    let empty = Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    };
    run(&a, &a, Operation::Union, 8.);
    run(&a, &a, Operation::Intersection, 8.);
    run(&a, &a, Operation::Difference, 0.);
    run(&empty, &a, Operation::Union, 8.);
    run(&empty, &a, Operation::Difference, 0.);
    run(&a, &empty, Operation::Difference, 8.);
    run(&empty, &a, Operation::Intersection, 0.);
    run(&empty, &empty, Operation::Union, 0.);
}
#[test]
fn disjoint_and_nested_solids_with_cavity() {
    let a = cube([0.; 3], [3.; 3]);
    let b = cube([1.; 3], [2.; 3]);
    let c = cube([4.; 3], [5.; 3]);
    run(&a, &c, Operation::Union, 28.);
    run(&a, &c, Operation::Difference, 27.);
    run(&a, &c, Operation::Intersection, 0.);
    run(&a, &b, Operation::Union, 27.);
    run(&a, &b, Operation::Intersection, 1.);
    let shell = run(&a, &b, Operation::Difference, 26.);
    run(&b, &a, Operation::Difference, 0.);
    run(&shell.mesh, &b, Operation::Union, 27.);
    run(&shell.mesh, &b, Operation::Intersection, 0.);
}
#[test]
fn face_contact_is_regularized_and_edge_contact_fails_closed() {
    let a = cube([0.; 3], [1.; 3]);
    let face = cube([1., 0., 0.], [2., 1., 1.]);
    run(&a, &face, Operation::Union, 2.);
    run(&a, &face, Operation::Intersection, 0.);
    run(&a, &face, Operation::Difference, 1.);
    let edge = cube([1., 1., 0.], [2., 2., 1.]);
    run(&a, &edge, Operation::Intersection, 0.);
    assert_eq!(
        boolean(&a, &edge, Operation::Union, &Options::default())
            .unwrap_err()
            .code,
        "POLYGON_BOOLEAN_NON_MANIFOLD_RESULT"
    );
}
#[test]
fn coplanar_partial_overlap_and_rotated_boxes() {
    let a = cube([0.; 3], [2.; 3]);
    let b = cube([1., 0., 0.], [3., 2., 2.]);
    run(&a, &b, Operation::Union, 12.);
    run(&a, &b, Operation::Intersection, 4.);
    run(&a, &b, Operation::Difference, 4.);
    let q = cube([-1.; 3], [1.; 3]);
    let s = std::f64::consts::FRAC_1_SQRT_2;
    let rotated = q
        .transform([
            [s, -s, 0., 0.],
            [s, s, 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ])
        .unwrap();
    let overlap = 16. * (2_f64.sqrt() - 1.);
    run(&q, &rotated, Operation::Intersection, overlap);
    run(&q, &rotated, Operation::Union, 16. - overlap);
    run(&q, &rotated, Operation::Difference, 8. - overlap);
}
#[test]
fn coordinate_normalization_preserves_scale_and_translation() {
    let a = cube([0.; 3], [2.; 3]);
    let b = cube([1.; 3], [3.; 3]);
    for size in [0.01, 1000.] {
        let m = [
            [size, 0., 0., 1e6],
            [0., size, 0., -1e6],
            [0., 0., size, 2e6],
            [0., 0., 0., 1.],
        ];
        let out = boolean(
            &a.transform(m).unwrap(),
            &b.transform(m).unwrap(),
            Operation::Intersection,
            &Options::default(),
        )
        .unwrap();
        assert!((out.report.signed_volume_mm3 / size.powi(3) - 1.).abs() < 1e-6);
    }
}
#[test]
fn rejects_invalid_solids_and_exhausted_budget() {
    let a = cube([0.; 3], [1.; 3]);
    let mut open = a.clone();
    open.indices.truncate(33);
    assert_eq!(
        boolean(&open, &a, Operation::Union, &Options::default())
            .unwrap_err()
            .code,
        "POLYGON_BOOLEAN_INVALID_SOLID"
    );
    let mut inward = a.clone();
    inward.reverse_winding();
    assert!(boolean(&inward, &a, Operation::Union, &Options::default()).is_err());
    assert_eq!(
        boolean(
            &a,
            &a,
            Operation::Union,
            &Options {
                max_work: 1,
                ..Options::default()
            }
        )
        .unwrap_err()
        .code,
        "POLYGON_BOOLEAN_RESOURCE_LIMIT"
    );
    assert!(
        boolean(
            &a,
            &a,
            Operation::Union,
            &Options {
                relative_tolerance: f64::NAN,
                ..Options::default()
            }
        )
        .is_err()
    );
}

#[test]
fn analytic_box_sweep_and_chained_nonconvex_result() {
    let a = cube([0.; 3], [2.; 3]);
    for i in 0..24 {
        let t = i as f64;
        let low = [
            -0.7 + t * 0.13,
            -0.4 + (i % 7) as f64 * 0.31,
            -0.2 + (i % 5) as f64 * 0.37,
        ];
        let high = low.map(|x| x + 1.3);
        let b = cube(low, high);
        let intersection = (0..3)
            .map(|axis| (2_f64.min(high[axis]) - 0_f64.max(low[axis])).max(0.))
            .product::<f64>();
        run(
            &a,
            &b,
            Operation::Union,
            8. + 1.3_f64.powi(3) - intersection,
        );
        run(&a, &b, Operation::Intersection, intersection);
        run(&a, &b, Operation::Difference, 8. - intersection);
    }
    let b = cube([1., 1., 0.], [3., 3., 2.]);
    let joined = run(&a, &b, Operation::Union, 14.);
    let cut = cube([0.5, 0.5, -1.], [1.5, 1.5, 3.]);
    let result = run(&joined.mesh, &cut, Operation::Difference, 12.);
    run(&result.mesh, &cut, Operation::Intersection, 0.);
    assert_eq!(
        result.mesh.indices,
        boolean(
            &joined.mesh,
            &cut,
            Operation::Difference,
            &Options::default()
        )
        .unwrap()
        .mesh
        .indices
    );
}

#[test]
fn rejects_crossing_shells_and_incorrect_component_orientation() {
    fn append(a: &Mesh, b: &Mesh) -> Mesh {
        let mut m = a.clone();
        let offset = m.positions.len() / 3;
        m.positions.extend(&b.positions);
        m.indices.extend(b.indices.iter().map(|i| i + offset));
        m
    }
    let outer = cube([0.; 3], [3.; 3]);
    for bad in [
        append(&outer, &cube([2.; 3], [4.; 3])),
        append(&outer, &cube([1.; 3], [2.; 3])),
    ] {
        assert_eq!(
            boolean(&bad, &outer, Operation::Union, &Options::default())
                .unwrap_err()
                .code,
            "POLYGON_BOOLEAN_INVALID_SOLID"
        );
    }
    let mut reversed = cube([4.; 3], [5.; 3]);
    reversed.reverse_winding();
    assert!(
        boolean(
            &append(&outer, &reversed),
            &outer,
            Operation::Union,
            &Options::default()
        )
        .is_err()
    );
    // The output cap bounds clipped results; identical or separated operands
    // return whole input surfaces and are bounded by the inputs instead.
    assert_eq!(
        boolean(
            &outer,
            &cube([1.; 3], [4.; 3]),
            Operation::Union,
            &Options {
                max_output_triangles: 1,
                ..Options::default()
            }
        )
        .unwrap_err()
        .code,
        "POLYGON_BOOLEAN_RESOURCE_LIMIT"
    );
    assert!(
        boolean(
            &outer,
            &outer,
            Operation::Union,
            &Options {
                max_output_triangles: 1,
                ..Options::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn coincident_solids_with_different_triangulations() {
    let a = cube([0.; 3], [2.; 3]);
    let mut b = a.clone();
    b.indices[..6].copy_from_slice(&[0, 3, 1, 1, 3, 2]);
    run(&a, &b, Operation::Union, 8.);
    run(&a, &b, Operation::Intersection, 8.);
    run(&a, &b, Operation::Difference, 0.);
}

fn translated(mesh: &Mesh, offset: [f64; 3]) -> Mesh {
    mesh.transform([
        [1., 0., 0., offset[0]],
        [0., 1., 0., offset[1]],
        [0., 0., 1., offset[2]],
        [0., 0., 0., 1.],
    ])
    .unwrap()
}

#[test]
fn separated_operands_above_the_bsp_cap_take_the_exact_fast_path() {
    let a = crate::solid::primitives::sphere(10., 96).unwrap();
    let far = translated(&a, [30., 0., 0.]);
    assert!(
        a.indices.len() / 3 + far.indices.len() / 3 > BSP_INPUT_TRIANGLES,
        "fixture must exceed the BSP admission cap"
    );
    let out = boolean(&a, &far, Operation::Union, &Options::default()).unwrap();
    assert_eq!(out.mesh.indices.len(), a.indices.len() + far.indices.len());
    assert!(out.report.closed);
    assert_eq!(out.report.self_intersection_status, "not_checked");
    let kept = boolean(&a, &far, Operation::Difference, &Options::default()).unwrap();
    assert_eq!(kept.mesh.indices.len(), a.indices.len());
    let none = boolean(&a, &far, Operation::Intersection, &Options::default()).unwrap();
    assert!(none.mesh.indices.is_empty());
    // Overlapping operands above the cap still need BSP clipping and are refused.
    let near = translated(&a, [5., 0., 0.]);
    assert_eq!(
        boolean(&a, &near, Operation::Union, &Options::default())
            .unwrap_err()
            .code,
        "POLYGON_BOOLEAN_RESOURCE_LIMIT"
    );
    // Below the cap the fast path is still audited, as before.
    let small = cube([0.; 3], [1.; 3]);
    let apart = cube([3.; 3], [4.; 3]);
    let audited = boolean(&small, &apart, Operation::Union, &Options::default()).unwrap();
    assert_eq!(
        audited.report.self_intersection_status,
        "checked_with_tolerance"
    );
}

#[test]
fn union_many_joins_separated_groups_and_folds_touching_ones() {
    let a = cube([0.; 3], [1.; 3]);
    let overlapping = cube([0.5, 0., 0.], [1.5, 1., 1.]);
    let apart = cube([5.; 3], [6.; 3]);
    let calls = std::cell::Cell::new(0);
    let mut pairwise = |x: &Mesh, y: &Mesh| {
        calls.set(calls.get() + 1);
        Ok(boolean(x, y, Operation::Union, &Options::default())?.mesh)
    };
    let out = union_many(
        &[a.clone(), apart.clone(), overlapping.clone()],
        &mut pairwise,
    )
    .unwrap();
    assert_eq!(calls.get(), 1, "only the overlapping pair needs CSG");
    let report = out.inspect().unwrap();
    assert!(report.closed);
    assert!((report.signed_volume_mm3 - 2.5).abs() < 1e-8);
    vertex_manifold(&out).unwrap();

    // Face contact is connectivity, never separation.
    let touching = cube([1., 0., 0.], [2., 1., 1.]);
    calls.set(0);
    let fused = union_many(&[a.clone(), touching], &mut pairwise).unwrap();
    assert_eq!(calls.get(), 1);
    assert!((fused.inspect().unwrap().signed_volume_mm3 - 2.).abs() < 1e-8);

    calls.set(0);
    assert_eq!(
        union_many(&[a.clone()], &mut pairwise).unwrap().indices,
        a.indices
    );
    assert!(union_many(&[], &mut pairwise).unwrap().indices.is_empty());
    assert_eq!(calls.get(), 0);
}

#[test]
fn difference_many_subtracts_separated_cutters_in_batches() {
    let plate = cube([0., 0., 0.], [8., 8., 1.]);
    let mut cutters: Vec<Mesh> = (0..4)
        .flat_map(|i| {
            (0..4).map(move |j| {
                let (x, y) = (1. + 2. * i as f64, 1. + 2. * j as f64);
                cube([x, y, -1.], [x + 1., y + 1., 2.])
            })
        })
        .collect();
    cutters.push(cube([20.; 3], [21.; 3]));
    let calls = std::cell::Cell::new(0);
    let mut pairwise = |x: &Mesh, y: &Mesh| {
        calls.set(calls.get() + 1);
        Ok(boolean(x, y, Operation::Difference, &Options::default())?.mesh)
    };
    let out = difference_many(&plate, &cutters, &mut pairwise, 8).unwrap();
    assert_eq!(
        calls.get(),
        2,
        "16 separated cutters in batches of 8; the far cube is skipped"
    );
    let report = out.inspect().unwrap();
    assert!(report.closed);
    assert!((report.signed_volume_mm3 - 48.).abs() < 1e-8);
    vertex_manifold(&out).unwrap();

    // One cutter per step gives the same volume. It is kept short on purpose:
    // BSP clipping planes are infinite, so every step re-splits the whole cap
    // and the intermediate triangle count grows quadratically when coplanar
    // simplification cannot retriangulate a multi-hole cap (measured: 14,700
    // triangles after 14 single-cutter steps on this plate). Batching is the
    // mitigation until simplification handles such caps.
    calls.set(0);
    let sequential = difference_many(&plate, &cutters[..8], &mut pairwise, 1).unwrap();
    assert_eq!(calls.get(), 8);
    assert!((sequential.inspect().unwrap().signed_volume_mm3 - 56.).abs() < 1e-8);

    // Overlapping cutters never share a batch, so the join stays exact.
    let stacked = [
        cube([1., 1., -1.], [3., 3., 2.]),
        cube([2., 2., -1.], [4., 4., 2.]),
    ];
    calls.set(0);
    let pocketed = difference_many(&plate, &stacked, &mut pairwise, 8).unwrap();
    assert_eq!(calls.get(), 2);
    assert!((pocketed.inspect().unwrap().signed_volume_mm3 - 57.).abs() < 1e-8);

    assert!(
        difference_many(
            &crate::solid::primitives::empty(),
            &cutters,
            &mut pairwise,
            8
        )
        .unwrap()
        .indices
        .is_empty()
    );
    assert_eq!(
        difference_many(&plate, &[], &mut pairwise, 8)
            .unwrap()
            .indices,
        plate.indices
    );
}
