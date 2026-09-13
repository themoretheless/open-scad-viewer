use brep_core::{Model, cuboid, cylinder, extrude_polygon, extrude_polygon_with_holes};
use std::collections::BTreeSet;

fn ids(values: &[String]) -> BTreeSet<String> {
    values.iter().cloned().collect()
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
