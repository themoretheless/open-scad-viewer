use brep_core::{Model, TopoId, cuboid, cylinder, extrude_polygon, extrude_polygon_with_holes};
use std::collections::BTreeSet;

fn ids(values: &[TopoId]) -> BTreeSet<TopoId> {
    values.iter().copied().collect()
}

fn cap(model: &Model, z: f64) -> usize {
    model
        .faces
        .iter()
        .position(|face| {
            face.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == z)
        })
        .unwrap()
}

#[test]
fn an_added_hole_changes_cap_identity_but_preserves_unchanged_walls() {
    let outline = [[0., 0.], [6., 0.], [6., 6.], [0., 6.]];
    let source = extrude_polygon(&outline, 0., 2.).unwrap();
    let mut drilled = extrude_polygon_with_holes(
        &outline,
        &[vec![[2., 2.], [2., 4.], [4., 4.], [4., 2.]]],
        0.,
        2.,
    )
    .unwrap();
    drilled.inherit_topology_ids(&[&source]);
    drilled.validate().unwrap();
    for z in [0., 2.] {
        assert_ne!(
            drilled.1.faces[cap(&drilled, z)],
            source.1.faces[cap(&source, z)]
        );
    }
    assert_eq!(
        ids(&source.1.faces)
            .intersection(&ids(&drilled.1.faces))
            .count(),
        4
    );
}

#[test]
fn matching_boundaries_do_not_make_distinct_support_surfaces_identical() {
    let source = cuboid([0.; 3], [2.; 3]).unwrap();
    let mut curved = source.clone();
    let face = cap(&source, 2.);
    let support = &source.faces[face].surface;
    let mut control = vec![];
    for i in 0..3 {
        let mut row = vec![];
        for j in 0..3 {
            let mut p = support
                .evaluate(i as f64 / 2., j as f64 / 2.)
                .unwrap()
                .point;
            if i == 1 && j == 1 {
                p[2] += 0.5;
            }
            row.push(p.to_vec());
        }
        control.push(row);
    }
    let patch = &mut curved.faces[face].surface;
    patch.degree_u = 2;
    patch.degree_v = 2;
    patch.knots_u = vec![0., 0., 0., 1., 1., 1.];
    patch.knots_v = patch.knots_u.clone();
    patch.control_points = control;
    patch.weights = vec![vec![1.; 3]; 3];
    curved.inherit_topology_ids(&[&source]);
    curved.validate().unwrap();
    assert_ne!(curved.1.faces[face], source.1.faces[face]);
    assert_eq!(curved.1.edges, source.1.edges);
    assert_eq!(
        ids(&curved.1.faces)
            .intersection(&ids(&source.1.faces))
            .count(),
        5
    );
}

#[test]
fn every_rational_curve_definition_contributes_to_edge_identity() {
    let source = cylinder(2., 3.).unwrap();
    let mut changed = source.clone();
    let edge = source
        .edges
        .iter()
        .position(|edge| edge.curve.degree == 2)
        .unwrap();
    let endpoints = changed.edges[edge].vertices;
    changed.edges[edge].curve.weights[1] *= 0.8;
    // Identity generation also runs while builders assemble a candidate, before
    // its face trims are repaired; the distinct valid curves share endpoints.
    changed.edges[edge].curve.validate().unwrap();
    changed.inherit_topology_ids(&[&source]);
    assert_eq!(changed.edges[edge].vertices, endpoints);
    assert_ne!(changed.1.edges[edge], source.1.edges[edge]);
    assert!(!changed.1.lineage.iter().any(|relation| {
        relation.entity_kind == "edge"
            && relation.parents.contains(&source.1.edges[edge])
            && relation.children.contains(&changed.1.edges[edge])
    }));
}

#[test]
fn material_inside_a_hole_has_no_face_lineage_from_the_surrounding_cap() {
    let source = extrude_polygon_with_holes(
        &[[0., 0.], [6., 0.], [6., 6.], [0., 6.]],
        &[vec![[2., 2.], [2., 4.], [4., 4.], [4., 2.]]],
        0.,
        2.,
    )
    .unwrap();
    let mut island =
        extrude_polygon(&[[2.5, 2.5], [3.5, 2.5], [3.5, 3.5], [2.5, 3.5]], 0., 2.).unwrap();
    island.inherit_topology_ids(&[&source]);
    for z in [0., 2.] {
        assert!(!island.1.lineage.iter().any(|relation| {
            relation.entity_kind == "face"
                && relation.parents.contains(&source.1.faces[cap(&source, z)])
                && relation.children.contains(&island.1.faces[cap(&island, z)])
        }));
    }
}

#[test]
fn concave_collinear_contours_use_material_overlap_instead_of_centroids() {
    let source = extrude_polygon(
        &[
            [0., 0.],
            [3., 0.],
            [6., 0.],
            [6., 1.],
            [1., 1.],
            [1., 6.],
            [0., 6.],
        ],
        0.,
        2.,
    )
    .unwrap();
    let mut smaller = extrude_polygon(
        &[
            [0., 0.],
            [2., 0.],
            [4., 0.],
            [4., 0.5],
            [0.5, 0.5],
            [0.5, 4.],
            [0., 4.],
        ],
        0.,
        2.,
    )
    .unwrap();
    smaller.inherit_topology_ids(&[&source]);
    smaller.validate().unwrap();
    for z in [0., 2.] {
        assert!(smaller.1.lineage.iter().any(|relation| {
            relation.entity_kind == "face"
                && relation.parents.contains(&source.1.faces[cap(&source, z)])
                && relation
                    .children
                    .contains(&smaller.1.faces[cap(&smaller, z)])
        }));
    }
}

#[test]
fn nearby_coordinates_are_not_rounded_into_the_same_authored_entity() {
    let source = cuboid([0.; 3], [2.; 3]).unwrap();
    let mut shifted = source.clone();
    let shift = 1e-8;
    for vertex in &mut shifted.vertices {
        vertex.point[0] += shift;
    }
    for edge in &mut shifted.edges {
        for point in &mut edge.curve.control_points {
            point[0] += shift;
        }
    }
    for face in &mut shifted.faces {
        for point in face.surface.control_points.iter_mut().flatten() {
            point[0] += shift;
        }
    }
    shifted.inherit_topology_ids(&[&source]);
    shifted.validate().unwrap();
    assert!(ids(&source.1.vertices).is_disjoint(&ids(&shifted.1.vertices)));
    assert!(ids(&source.1.edges).is_disjoint(&ids(&shifted.1.edges)));
}

#[test]
fn reindexing_and_cyclic_loop_rotation_preserve_geometry_identities() {
    let source = cylinder(2., 3.).unwrap();
    let mut reordered = source.clone();
    let vertices = reordered.vertices.len();
    reordered.vertices.reverse();
    for edge in &mut reordered.edges {
        edge.vertices = edge.vertices.map(|index| vertices - 1 - index);
    }
    let edges = reordered.edges.len();
    reordered.edges.reverse();
    for wire in &mut reordered.loops {
        for coedge in &mut wire.coedges {
            coedge.edge = edges - 1 - coedge.edge;
        }
        wire.coedges.rotate_left(1);
    }
    reordered.rebuild_topology_ids();
    reordered.validate().unwrap();
    assert_eq!(ids(&reordered.1.vertices), ids(&source.1.vertices));
    assert_eq!(ids(&reordered.1.edges), ids(&source.1.edges));
    assert_eq!(ids(&reordered.1.loops), ids(&source.1.loops));
    assert_eq!(ids(&reordered.1.faces), ids(&source.1.faces));
}

fn reindex_clone(source: &Model) -> Model {
    let mut reordered = source.clone();
    let vertices = reordered.vertices.len();
    reordered.vertices.reverse();
    for edge in &mut reordered.edges {
        edge.vertices = edge.vertices.map(|index| vertices - 1 - index);
    }
    let edges = reordered.edges.len();
    reordered.edges.reverse();
    for wire in &mut reordered.loops {
        for coedge in &mut wire.coedges {
            coedge.edge = edges - 1 - coedge.edge;
        }
        if !wire.coedges.is_empty() {
            wire.coedges.rotate_left(1);
        }
    }
    reordered.rebuild_topology_ids();
    reordered
}

#[test]
fn cone_apex_edge_and_vertex_ids_survive_reindex() {
    for model in [
        brep_core::frustum(3., 0., 5.).unwrap(),
        brep_core::frustum(0., 3., 5.).unwrap(),
    ] {
        assert!(model.edges.iter().any(|edge| edge.degenerate));
        let reordered = reindex_clone(&model);
        reordered.validate().unwrap();
        assert_eq!(ids(&reordered.1.vertices), ids(&model.1.vertices));
        assert_eq!(ids(&reordered.1.edges), ids(&model.1.edges));
        let apex_edges: BTreeSet<_> = model
            .edges
            .iter()
            .zip(model.1.edges.iter())
            .filter(|(edge, _)| edge.degenerate)
            .map(|(_, id)| *id)
            .collect();
        let reordered_apex: BTreeSet<_> = reordered
            .edges
            .iter()
            .zip(reordered.1.edges.iter())
            .filter(|(edge, _)| edge.degenerate)
            .map(|(_, id)| *id)
            .collect();
        assert_eq!(apex_edges, reordered_apex);
        assert!(!apex_edges.is_empty());
    }
}

#[test]
fn sphere_ordinary_poles_survive_reindex_without_degenerate_edges() {
    let source = brep_core::sphere(3.).unwrap();
    assert!(source.edges.iter().all(|edge| !edge.degenerate));
    let poles: Vec<_> = source
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| v.point[0].abs() < 1e-12 && v.point[1].abs() < 1e-12)
        .map(|(i, v)| (source.1.vertices[i], v.point[2].signum()))
        .collect();
    assert_eq!(poles.len(), 2);
    let reordered = reindex_clone(&source);
    reordered.validate().unwrap();
    assert!(reordered.edges.iter().all(|edge| !edge.degenerate));
    assert_eq!(ids(&reordered.1.vertices), ids(&source.1.vertices));
    assert_eq!(ids(&reordered.1.edges), ids(&source.1.edges));
}

#[test]
fn torus_shared_seam_edge_ids_stable_under_cyclic_rotation() {
    let source = brep_core::torus(4., 1.).unwrap();
    let reordered = reindex_clone(&source);
    reordered.validate().unwrap();
    assert_eq!(ids(&reordered.1.edges), ids(&source.1.edges));
    assert_eq!(ids(&reordered.1.faces), ids(&source.1.faces));
}

#[test]
fn degenerate_flag_participates_in_edge_identity() {
    let mut model = brep_core::frustum(3., 0., 5.).unwrap();
    let apex = model
        .edges
        .iter()
        .position(|edge| edge.degenerate)
        .expect("cone apex");
    let before = model.1.edges[apex];
    model.edges[apex].degenerate = false;
    model.rebuild_topology_ids();
    assert_ne!(model.1.edges[apex], before);
}

#[test]
fn cone_apex_lineage_does_not_false_persist_across_height_change() {
    let short = brep_core::frustum(3., 0., 4.).unwrap();
    let mut tall = brep_core::frustum(3., 0., 6.).unwrap();
    tall.inherit_topology_ids(&[&short]);
    tall.validate().unwrap();
    let short_apex: BTreeSet<_> = short
        .edges
        .iter()
        .zip(short.1.edges.iter())
        .filter(|(edge, _)| edge.degenerate)
        .map(|(_, id)| *id)
        .collect();
    let tall_apex: BTreeSet<_> = tall
        .edges
        .iter()
        .zip(tall.1.edges.iter())
        .filter(|(edge, _)| edge.degenerate)
        .map(|(_, id)| *id)
        .collect();
    assert!(short_apex.is_disjoint(&tall_apex));
}

#[test]
fn tube_inner_and_outer_walls_keep_distinct_face_identities() {
    let tube = brep_core::tube(4., 2., 5.).unwrap();
    tube.validate().unwrap();
    assert!(tube.faces.len() >= 8);
    let face_ids = ids(&tube.1.faces);
    assert_eq!(face_ids.len(), tube.faces.len());
    let reordered = reindex_clone(&tube);
    reordered.validate().unwrap();
    assert_eq!(ids(&reordered.1.faces), face_ids);
    assert_eq!(ids(&reordered.1.edges), ids(&tube.1.edges));
}

#[test]
fn revolve_axis_touching_profile_poles_survive_reindex() {
    let profile = [[0., 0.], [2., 0.], [2., 3.], [0., 3.]];
    let source = brep_core::revolve(&profile).unwrap();
    source.validate().unwrap();
    let poles: Vec<_> = source
        .vertices
        .iter()
        .enumerate()
        .filter(|(_, v)| v.point[0].abs() < 1e-12 && v.point[1].abs() < 1e-12)
        .map(|(i, _)| source.1.vertices[i])
        .collect();
    assert!(!poles.is_empty(), "axis-touching revolve must author poles");
    let reordered = reindex_clone(&source);
    reordered.validate().unwrap();
    assert_eq!(ids(&reordered.1.vertices), ids(&source.1.vertices));
    assert_eq!(ids(&reordered.1.faces), ids(&source.1.faces));
}

#[test]
fn frozen_constructor_matrix_validates_and_has_stable_identity_cardinality() {
    // Frozen Phase A constructor admission matrix (positive cases only).
    let cases: Vec<(&str, Model)> = vec![
        ("cylinder", cylinder(2., 4.).unwrap()),
        ("frustum", brep_core::frustum(3., 1., 5.).unwrap()),
        ("cone", brep_core::frustum(3., 0., 5.).unwrap()),
        ("tube", brep_core::tube(3., 1., 4.).unwrap()),
        ("sphere", brep_core::sphere(2.5).unwrap()),
        ("torus", brep_core::torus(4., 1.).unwrap()),
        (
            "revolve",
            brep_core::revolve(&[[1., 0.], [2., 0.], [2., 2.], [1., 2.]]).unwrap(),
        ),
    ];
    for (name, model) in cases {
        model
            .validate()
            .unwrap_or_else(|e| panic!("{name} validate: {e}"));
        assert_eq!(
            model.1.vertices.len(),
            model.vertices.len(),
            "{name} vertex ids"
        );
        assert_eq!(model.1.edges.len(), model.edges.len(), "{name} edge ids");
        assert_eq!(model.1.faces.len(), model.faces.len(), "{name} face ids");
        let again = reindex_clone(&model);
        again
            .validate()
            .unwrap_or_else(|e| panic!("{name} reindex: {e}"));
        assert_eq!(
            ids(&again.1.faces),
            ids(&model.1.faces),
            "{name} face stability"
        );
    }
}
