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
    assert!(boolean(
        &a,
        &a,
        Operation::Union,
        &Options {
            relative_tolerance: f64::NAN,
            ..Options::default()
        }
    )
    .is_err());
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
    assert!(boolean(
        &append(&outer, &reversed),
        &outer,
        Operation::Union,
        &Options::default()
    )
    .is_err());
    assert_eq!(
        boolean(
            &outer,
            &outer,
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
