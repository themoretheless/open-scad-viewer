use brep_core::{
    Model,
    region_replacement::{self as r, InteriorTileLimits},
};
use nurbs_core::surface::Axis;
fn limits() -> InteriorTileLimits {
    InteriorTileLimits {
        domain_cells: 10000,
        agreement_cells: 30000,
        seam_cells: 30000,
        exact_work: 1000000,
        graph_coefficients: None,
    }
}
fn fixture() -> Model {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    m.faces[0].surface = m.faces[0]
        .surface
        .edit_axis(Axis::U, |c| c.elevate(6))
        .unwrap()
        .edit_axis(Axis::V, |c| c.elevate(6))
        .unwrap();
    m.rebuild_topology_ids();
    m
}
#[test]
fn compact_tile_keeps_a_closed_shell_and_qualifies_four_whole_g2_seams() {
    let source = fixture();
    let before = source.clone();
    let out = r::propose_interior_tile(
        &source,
        0,
        [[0.25, 0.75]; 2],
        [0.001; 3],
        1e-8,
        [1e-7, 1e-5, 1e-4],
        limits(),
    )
    .unwrap();
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(source, before);
    assert_eq!(out.model.faces.len(), 7);
    assert_eq!(out.model.edges.len(), 16);
    assert_eq!(out.model.vertices.len(), 12);
    assert_eq!(out.model.faces[0].holes.len(), 1);
    assert_eq!(out.model.faces[0].surface, source.faces[0].surface);
    assert_eq!(&out.model.edges[..source.edges.len()], &source.edges[..]);
    assert_eq!(
        &out.model.1.edges[..source.edges.len()],
        &source.1.edges[..]
    );
    for i in 1..6 {
        assert_eq!(out.model.faces[i], source.faces[i]);
        assert_eq!(out.model.1.faces[i], source.1.faces[i]);
    }
    assert!(out.seams.iter().all(|s| s.positions.proven
        && s.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance));
    let audit =
        brep_core::face_domain::audit_trim_regions(&out.model, 1e-8, 10000, 30000, 30000).unwrap();
    assert!(audit.all_valid);
    assert!(out.model.persistent_naming_complete());
    assert!(out.model.1.lineage.iter().any(|r| r.operation == "split"
        && r.entity_kind == "face"
        && r.parents == [source.1.faces[0]]
        && r.children.contains(&out.model.1.faces[out.tile_face])));
    assert_eq!(
        brep_topology::explorer::euler_characteristic(&out.model.0),
        2
    );
    let centre = out.model.faces[out.tile_face]
        .surface
        .evaluate(0.5, 0.5)
        .unwrap()
        .point;
    let old = source.faces[0].surface.evaluate(0.5, 0.5).unwrap().point;
    assert!((0..3).all(|k| centre[k] > old[k]));
}
#[test]
fn outside_rectangle_and_exhausted_ownership_do_not_change_the_source() {
    let source = fixture();
    let before = source.clone();
    for (rect, budget) in [([[-0.1, 0.5]; 2], 10000), ([[0.25, 0.75]; 2], 0)] {
        let mut work = limits();
        work.domain_cells = budget;
        assert!(
            r::propose_interior_tile(&source, 0, rect, [0.001; 3], 1e-8, [1e-7, 1e-5, 1e-4], work)
                .is_err()
        );
        assert_eq!(source, before);
    }
}

#[test]
fn correlated_graph_proof_qualifies_the_rotated_source_and_four_new_seams() {
    let mut source = fixture();
    let s = &source.faces[0].surface;
    let o = s.evaluate(0., 0.).unwrap().point;
    let a = s.evaluate(1., 0.).unwrap().point;
    let b = s.evaluate(0., 1.).unwrap().point;
    let au = std::array::from_fn::<_, 3, _>(|k| a[k] - o[k]);
    let av = std::array::from_fn::<_, 3, _>(|k| b[k] - o[k]);
    let an = [
        au[1] * av[2] - au[2] * av[1],
        au[2] * av[0] - au[0] * av[2],
        au[0] * av[1] - au[1] * av[0],
    ];
    let f = [
        [1. / 2f64.sqrt(), -1. / 2f64.sqrt(), 0.],
        [1. / 6f64.sqrt(), 1. / 6f64.sqrt(), -2. / 6f64.sqrt()],
        [1. / 3f64.sqrt(); 3],
    ];
    let map = |p: [f64; 3]| {
        std::array::from_fn::<_, 3, _>(|k| {
            [au, av, an]
                .iter()
                .zip(f)
                .map(|(e, v)| v[k] * (0..3).map(|j| e[j] * (p[j] - o[j])).sum::<f64>())
                .sum::<f64>()
        })
    };
    for v in &mut source.vertices {
        v.point = map(v.point);
    }
    for e in &mut source.edges {
        for p in &mut e.curve.control_points {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    for face in &mut source.faces {
        for p in face.surface.control_points.iter_mut().flatten() {
            *p = map([p[0], p[1], p[2]]).to_vec();
        }
    }
    source.rebuild_topology_ids();
    let before = source.clone();
    let mut work = limits();
    work.graph_coefficients = Some(20_000_000);
    let out = r::propose_interior_tile(
        &source,
        0,
        [[0.25, 0.75]; 2],
        [0.001; 3],
        1e-8,
        [1e-7, 1e-5, 1e-4],
        work,
    )
    .unwrap();
    assert_eq!(source, before);
    assert!(out.coefficient_work > 0 && out.coefficient_work <= 20_000_000);
    assert_eq!(out.seams.len(), 4);
    assert!(
        out.seams
            .iter()
            .all(|s| s.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance)
    );
    assert_eq!(
        out.closure.closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
}
#[test]
fn late_seam_and_closure_exhaustion_never_return_a_partial_body() {
    let source = fixture();
    let before = source.clone();
    let mut no_shape = limits();
    no_shape.graph_coefficients = Some(0);
    let mut no_closure = limits();
    no_closure.agreement_cells = 0;
    no_closure.exact_work = 0;
    for work in [no_shape, no_closure] {
        assert!(
            r::propose_interior_tile(
                &source,
                0,
                [[0.25, 0.75]; 2],
                [0.001; 3],
                1e-8,
                [1e-7, 1e-5, 1e-4],
                work
            )
            .is_err()
        );
        assert_eq!(source, before);
    }
}

#[test]
fn distributed_tile_requires_a_complete_curvature_receipt_and_retains_g2() {
    let mut source = fixture();
    source.faces[0].surface = source.faces[0]
        .surface
        .edit_axis(Axis::U, |c| c.elevate(9))
        .unwrap()
        .edit_axis(Axis::V, |c| c.elevate(9))
        .unwrap();
    source.rebuild_topology_ids();
    let before = source.clone();
    let work = brep_core::face_curvature::Limits {
        cells: 4000,
        domain_cells: 100000,
        shape_cells: 100000,
    };
    let (out, curvature) = r::propose_curvature_limited_tile(
        &source,
        0,
        [[0.25, 0.75]; 2],
        [0.00001; 3],
        1e-8,
        [1e-7, 1e-5, 1e-4],
        limits(),
        1.,
        work,
    )
    .unwrap();
    assert_eq!(
        curvature.decision,
        brep_core::face_curvature::Decision::Within
    );
    assert!(out.lifted_poles.len() > 1);
    assert_eq!(out.seams.len(), 4);
    assert!(out.seams.iter().all(|r| r.positions.proven
        && r.geometric.decision == nurbs_core::surface_join::Decision::WithinTolerance));
    assert_eq!(source, before);
    assert!(
        r::propose_curvature_limited_tile(
            &source,
            0,
            [[0.25, 0.75]; 2],
            [0.00001; 3],
            1e-8,
            [1e-7, 1e-5, 1e-4],
            limits(),
            1e-6,
            work
        )
        .is_err()
    );
    assert_eq!(source, before);
    let no_work = brep_core::face_curvature::Limits {
        cells: 0,
        domain_cells: 0,
        shape_cells: 0,
    };
    assert!(
        r::propose_curvature_limited_tile(
            &source,
            0,
            [[0.25, 0.75]; 2],
            [0.00001; 3],
            1e-8,
            [1e-7, 1e-5, 1e-4],
            limits(),
            1.,
            no_work
        )
        .is_err()
    );
    assert_eq!(source, before);
}
