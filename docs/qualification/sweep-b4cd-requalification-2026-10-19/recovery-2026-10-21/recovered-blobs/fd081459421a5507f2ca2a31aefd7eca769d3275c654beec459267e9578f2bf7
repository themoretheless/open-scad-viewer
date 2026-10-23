use brep_core::{
    Model,
    edge_partition::{self, PcurveCut},
};
fn cuts(model: &Model, edge: usize, fraction: f64) -> Vec<PcurveCut> {
    model
        .loops
        .iter()
        .enumerate()
        .flat_map(|(wire, l)| {
            l.coedges
                .iter()
                .enumerate()
                .filter(move |(_, c)| c.edge == edge)
                .map(move |(coedge, c)| {
                    let [a, b] = c.pcurve.domain();
                    let t = if c.reversed { 1. - fraction } else { fraction };
                    PcurveCut {
                        wire,
                        coedge,
                        parameter: a + (b - a) * t,
                    }
                })
        })
        .collect()
}
fn check_shared(model: &Model, edge: usize, fraction: f64) {
    let before = model.clone();
    let [a, b] = model.edges[edge].curve.domain();
    let out = edge_partition::partition(
        model,
        edge,
        a + (b - a) * fraction,
        &cuts(model, edge, fraction),
        10000,
    )
    .unwrap();
    assert_eq!(model, &before);
    assert_eq!(out.model.edges.len(), model.edges.len() + 1);
    assert_eq!(out.model.vertices.len(), model.vertices.len() + 1);
    assert_eq!(out.model.edges[out.child_edges[0]].vertices[1], out.vertex);
    assert_eq!(out.model.edges[out.child_edges[1]].vertices[0], out.vertex);
    assert_eq!(out.source_identity, model.1.edges[edge]);
    assert!(
        out.child_identities
            .iter()
            .all(|id| *id != out.source_identity)
    );
    assert!(out.model.1.lineage.iter().any(|r| r.operation == "split"
        && r.parents == [out.source_identity]
        && r.children == out.child_identities));
    for i in 0..model.edges.len() {
        if i != edge {
            assert_eq!(model.1.edges[i], out.model.1.edges[i]);
        }
    }
    for id in out.child_edges {
        assert_eq!(
            out.model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == id)
                .count(),
            2
        );
    }
    assert!(out.cells <= 10000);
    out.model.validate().unwrap();
    let closure = brep_core::watertight::inspect(&out.model, 100000).unwrap();
    assert_eq!(
        closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
}
#[test]
fn shared_partition_updates_both_faces_and_persistent_lineage_at_point_one() {
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    check_shared(&model, 0, 0.1);
}
#[test]
fn shared_partition_retains_rational_edge_and_nonunit_original_domain() {
    let mut model = brep_core::cylinder(4., 12.).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| e.curve.degree == 2)
        .unwrap();
    // Reparameterize without changing any original pole or weight.
    for t in &mut model.edges[edge].curve.knots {
        *t = 0.3 + *t * 1.4;
    }
    model.rebuild_topology_ids();
    check_shared(&model, edge, 0.1);
}
#[test]
fn shared_partition_rejects_missing_duplicate_or_wrong_pcurve_cuts_and_budget() {
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let before = model.clone();
    let selected = cuts(&model, 0, 0.1);
    assert_eq!(selected.len(), 2);
    for (cut, work) in [
        (vec![selected[0]], 10000),
        (vec![selected[0], selected[0]], 10000),
        (
            {
                let mut c = selected.clone();
                c[1].parameter = 0.5;
                c
            },
            10000,
        ),
        (selected.clone(), 1),
    ] {
        assert!(edge_partition::partition(&model, 0, 0.1, &cut, work).is_err());
        assert_eq!(model, before);
    }
}

#[test]
fn repeated_shared_partition_preserves_closed_incidence_for_twenty_cuts() {
    let initial = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let mut current = initial.clone();
    for _ in 0..20 {
        let [a, b] = current.edges[0].curve.domain();
        let use_cuts = cuts(&current, 0, 0.5);
        let next =
            edge_partition::partition(&current, 0, a + (b - a) * 0.5, &use_cuts, 10000).unwrap();
        assert!(next.model.persistent_naming_complete());
        current = next.model;
    }
    assert_eq!(current.edges.len(), initial.edges.len() + 20);
    assert_eq!(current.vertices.len(), initial.vertices.len() + 20);
    assert_eq!(current.faces.len(), initial.faces.len());
    assert_eq!(current.bodies.len(), initial.bodies.len());
    let closure = brep_core::watertight::inspect(&current, 100000).unwrap();
    assert_eq!(
        closure.status,
        brep_core::watertight::Closure::WithinTolerance
    );
}

#[test]
fn shared_partition_qualifies_quintic_nurbs_transition_on_deformed_patch_network() {
    let mut model = brep_core::analytic::g2_annular::cylinder(8., 4., 16., 0.1, 1e-6)
        .unwrap()
        .model;
    for vertex in &mut model.vertices {
        vertex.point[0] *= 2.;
        vertex.point[1] *= 0.5;
    }
    for edge in &mut model.edges {
        for p in &mut edge.curve.control_points {
            p[0] *= 2.;
            p[1] *= 0.5;
        }
    }
    for face in &mut model.faces {
        for row in &mut face.surface.control_points {
            for p in row {
                p[0] *= 2.;
                p[1] *= 0.5;
            }
        }
    }
    model.rebuild_topology_ids();
    let edge = model
        .edges
        .iter()
        .position(|e| e.curve.degree == 5)
        .expect("quintic transition edge");
    check_shared(&model, edge, 0.1);
}

fn yz_plane(x: f64) -> nurbs_core::surface::Surface {
    nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![x, -20., -20.], vec![x, -20., 20.]],
            vec![vec![x, 20., -20.], vec![x, 20., 20.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn automatic_plane_root_partitions_one_shared_vertex_on_both_faces() {
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            a[1] == 0. && b[1] == 0. && a[2] == 0. && b[2] == 0. && a[0] != b[0]
        })
        .unwrap();
    let before = model.clone();
    let out = edge_partition::partition_at_plane(&model, edge, &yz_plane(1.), 0, 10000).unwrap();
    assert_eq!(model, before);
    assert!(
        out.root_parameter[0] <= out.partition.source_parameter
            && out.partition.source_parameter <= out.root_parameter[1]
    );
    assert!(out.root_position_error_upper <= model.tolerance_mm);
    assert!(
        (out.partition.model.vertices[out.partition.vertex].point[0] - 1.).abs()
            <= model.tolerance_mm
    );
    assert!(out.partition.cells <= 10000);
    let exact = brep_core::boundary_agreement::verify_exact(&out.partition.model, 1000000).unwrap();
    assert!(
        exact.all_equal && exact.all_joins_exact,
        "root representative must retain exact authored face lifts"
    );
    let half = edge_partition::partition_at_plane(&model, edge, &yz_plane(5.), 0, 10000).unwrap();
    let exact_half =
        brep_core::boundary_agreement::verify_exact(&half.partition.model, 1000000).unwrap();
    assert!(exact_half.all_equal && exact_half.all_joins_exact);
    for id in out.partition.child_edges {
        assert_eq!(
            out.partition
                .model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == id)
                .count(),
            2
        );
    }
    assert!(edge_partition::partition_at_plane(&model, edge, &yz_plane(100.), 0, 10000).is_err());
    assert!(edge_partition::partition_at_plane(&model, edge, &yz_plane(1.), 0, 1).is_err());
}
#[test]
fn automatic_plane_root_retains_original_rational_curve_root_band() {
    let model = brep_core::cylinder(4., 12.).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            if e.curve.degree != 2 {
                return false;
            }
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            a[0].min(b[0]) < 2. && a[0].max(b[0]) > 2.
        })
        .unwrap();
    let out = edge_partition::partition_at_plane(&model, edge, &yz_plane(2.), 0, 10000).unwrap();
    assert!(out.root_position_error_upper <= model.tolerance_mm);
    assert!(
        (out.partition.model.vertices[out.partition.vertex].point[0] - 2.).abs()
            <= model.tolerance_mm
    );
    assert!(out.root_cells > 0);
    out.partition.model.validate().unwrap();
    assert_eq!(
        brep_core::watertight::inspect(&out.partition.model, 100000)
            .unwrap()
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
}

#[test]
fn automatic_curved_surface_root_partitions_both_incident_faces() {
    let model = brep_core::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            a[1] == 0. && b[1] == 0. && a[2] == 0. && b[2] == 0. && a[0] != b[0]
        })
        .unwrap();
    let mut support = yz_plane(1.);
    support.control_points[1][1][0] = 1.2;
    let before = model.clone();
    let result = edge_partition::partition_at_surface(&model, edge, &support, 0, 10000).unwrap();
    assert_eq!(model, before);
    assert!(result.root_position_error_upper <= model.tolerance_mm);
    assert!(
        (result.partition.model.vertices[result.partition.vertex].point[0] - 1.05).abs()
            <= model.tolerance_mm
    );
    for uv in result.support_uv_bounds {
        assert!(uv[0] <= 0.5 && 0.5 <= uv[1]);
    }
    for child in result.partition.child_edges {
        assert_eq!(
            result
                .partition
                .model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == child)
                .count(),
            2
        );
    }
    assert_eq!(
        brep_core::watertight::inspect(&result.partition.model, 100000)
            .unwrap()
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert!(edge_partition::partition_at_surface(&model, edge, &support, 0, 1).is_err());
    assert!(edge_partition::partition_at_surface(&model, edge, &support, 1, 10000).is_err());
}
