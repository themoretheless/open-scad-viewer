use planar_geometry::rings as cad;
use polygon_core::{
    Mesh,
    solid::{
        modeling, primitives as mesh,
        section::{MeshSection, MeshSectionIndex},
    },
};

fn area(section: &MeshSection) -> f64 {
    section
        .contours
        .iter()
        .map(|c| {
            let origin = c.points[0];
            let local: Vec<_> = c
                .points
                .iter()
                .map(|p| [p[0] - origin[0], p[1] - origin[1]])
                .collect();
            cad::area(&local)
        })
        .sum()
}

fn shifted(mut mesh: Mesh, delta: [f64; 3]) -> Mesh {
    for p in mesh.positions.chunks_exact_mut(3) {
        for axis in 0..3 {
            p[axis] += delta[axis];
        }
    }
    mesh
}

fn join(meshes: &[Mesh]) -> Mesh {
    let mut result = mesh::empty();
    for mesh in meshes {
        let offset = result.positions.len() / 3;
        result.positions.extend(&mesh.positions);
        result
            .indices
            .extend(mesh.indices.iter().map(|i| i + offset));
    }
    result
}

#[test]
fn box_uses_documented_half_open_rule_at_horizontal_faces() {
    let mesh = mesh::cube([2., 3., 4.], false).unwrap();
    let index = MeshSectionIndex::new(&mesh).unwrap();
    for z in [0., 1e-12, 2., 4. - 1e-12] {
        let section = index.section(z).unwrap();
        assert_eq!(section.contours.len(), 1, "z={z}");
        assert!((area(&section) - 6.).abs() < 1e-12);
        assert_eq!(section.candidate_triangles, 8);
        for contour in &section.contours {
            assert_eq!(contour.points.len(), contour.source_triangles.len());
            assert!(
                contour
                    .source_triangles
                    .iter()
                    .all(|&t| t < mesh.indices.len() / 3)
            );
        }
    }
    for z in [-1., -1e-12, 4., 5.] {
        let section = index.section(z).unwrap();
        assert!(section.contours.is_empty());
        assert_eq!(section.candidate_triangles, 0);
    }
}

#[test]
fn hollow_extrusion_preserves_hole_winding() {
    let outer = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
    let hole = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
    let rings = cad::planar(&outer, &hole, "difference").unwrap();
    let mesh = modeling::extrude_rings(&rings, 2., 1, 0., [1., 1.], false).unwrap();
    let index = MeshSectionIndex::new(&mesh).unwrap();
    for z in [0., 0.5, 1., 1.999] {
        let section = index.section(z).unwrap();
        assert_eq!(section.contours.len(), 2);
        let mut areas: Vec<_> = section
            .contours
            .iter()
            .map(|c| cad::area(&c.points))
            .collect();
        areas.sort_by(f64::total_cmp);
        assert_eq!(areas, vec![-4., 16.]);
        assert!((area(&section) - 12.).abs() < 1e-12);
    }
}

#[test]
fn cylinder_section_matches_its_polygon_not_the_analytic_circle() {
    let mesh = mesh::cylinder(3., 2., 2., 32, false).unwrap();
    let section = MeshSectionIndex::new(&mesh).unwrap().section(1.5).unwrap();
    let expected = 0.5 * 32. * 4. * (std::f64::consts::TAU / 32.).sin();
    assert_eq!(section.contours.len(), 1);
    assert!((area(&section) - expected).abs() < 1e-10);
}

#[test]
fn shared_plane_vertices_and_edges_do_not_create_double_segments() {
    // Outward octahedron: the equator lies exactly on four edges and vertices.
    let mesh = Mesh {
        positions: vec![
            1., 0., 0., 0., 1., 0., -1., 0., 0., 0., -1., 0., 0., 0., 1., 0., 0., -1.,
        ],
        indices: vec![
            0, 1, 4, 1, 2, 4, 2, 3, 4, 3, 0, 4, 1, 0, 5, 2, 1, 5, 3, 2, 5, 0, 3, 5,
        ],
        uv: None,
    };
    let index = MeshSectionIndex::new(&mesh).unwrap();
    let section = index.section(0.).unwrap();
    assert_eq!(section.contours.len(), 1);
    assert_eq!(section.contours[0].points.len(), 4);
    assert!((area(&section) - 2.).abs() < 1e-12);
    for z in [-1e-8, 1e-8] {
        let section = index.section(z).unwrap();
        assert!((area(&section) - 2. * (1. - z.abs()).powi(2)).abs() < 1e-12);
    }
    assert!(index.section(-1.).unwrap().contours.is_empty());
    assert!(index.section(1.).unwrap().contours.is_empty());
}

#[test]
fn gap_smaller_than_old_welding_epsilon_is_preserved() {
    let left = mesh::cube([1., 1., 1.], false).unwrap();
    let right = shifted(left.clone(), [1. + 1e-10, 0., 0.]);
    let section = MeshSectionIndex::new(&join(&[left, right]))
        .unwrap()
        .section(0.5)
        .unwrap();
    assert_eq!(section.contours.len(), 2);
    assert!((area(&section) - 2.).abs() < 1e-12);
}

#[test]
fn exact_triangle_soup_seams_and_signed_zero_share_identity() {
    let mesh = mesh::cube([1., 1., 1.], false).unwrap();
    let mut soup = mesh::empty();
    for &i in &mesh.indices {
        let mut p = mesh.point(i).unwrap();
        if soup.indices.len() % 2 == 0 {
            for v in &mut p {
                if *v == 0. {
                    *v = -0.;
                }
            }
        }
        soup.positions.extend(p);
        soup.indices.push(soup.indices.len());
    }
    let section = MeshSectionIndex::new(&soup).unwrap().section(0.5).unwrap();
    assert_eq!(section.contours.len(), 1);
    assert!((area(&section) - 1.).abs() < 1e-12);
}

#[test]
fn open_boundary_is_an_error_instead_of_a_repaired_ring() {
    let mut mesh = mesh::cube([1., 1., 1.], false).unwrap();
    let triangle = mesh
        .indices
        .chunks_exact(3)
        .position(|t| {
            let z: Vec<_> = t.iter().map(|&i| mesh.positions[i * 3 + 2]).collect();
            z.contains(&0.) && z.contains(&1.)
        })
        .unwrap();
    mesh.indices.drain(triangle * 3..triangle * 3 + 3);
    let error = MeshSectionIndex::new(&mesh)
        .unwrap()
        .section(0.5)
        .unwrap_err();
    assert_eq!(error.code, "SECTION_OPEN_BOUNDARY");
}

#[test]
fn duplicate_and_touching_boundaries_are_explicitly_ambiguous() {
    let cube = mesh::cube([1., 1., 1.], false).unwrap();
    for other in [cube.clone(), shifted(cube.clone(), [1., 1., 0.])] {
        let index = MeshSectionIndex::new(&join(&[cube.clone(), other])).unwrap();
        assert_eq!(
            index.section(0.5).unwrap_err().code,
            "SECTION_AMBIGUOUS_BOUNDARY"
        );
    }
}

#[test]
fn large_translation_does_not_saturate_coordinate_quantization() {
    let mesh = shifted(
        mesh::cube([2., 3., 4.], false).unwrap(),
        [1e12, -1e12, 1e12],
    );
    let section = MeshSectionIndex::new(&mesh)
        .unwrap()
        .section(1e12 + 2.)
        .unwrap();
    assert_eq!(section.contours.len(), 1);
    assert!((area(&section) - 6.).abs() < 1e-12);
}

#[test]
fn reusable_index_restricts_candidates_to_the_current_height() {
    let cube = mesh::cube([1., 1., 1.], false).unwrap();
    let meshes: Vec<_> = (0..100)
        .map(|i| shifted(cube.clone(), [0., 0., i as f64 * 2.]))
        .collect();
    let index = MeshSectionIndex::new(&join(&meshes)).unwrap();
    for z in [0.5, 84.5, 198.5, 0.5] {
        let section = index.section(z).unwrap();
        assert_eq!(section.candidate_triangles, 8);
        assert_eq!(section.contours.len(), 1);
        assert!((area(&section) - 1.).abs() < 1e-12);
    }
    assert_eq!(index.section(85.5).unwrap().candidate_triangles, 0);
}

#[test]
fn invalid_inputs_fail_before_slicing_and_empty_mesh_is_supported() {
    let index = MeshSectionIndex::new(&mesh::empty()).unwrap();
    assert!(index.section(0.).unwrap().contours.is_empty());
    for z in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(index.section(z).unwrap_err().code, "SECTION_INVALID_HEIGHT");
    }
    let mesh = Mesh {
        positions: vec![0., 0., 0.],
        indices: vec![0, 1, 2],
        uv: None,
    };
    assert!(MeshSectionIndex::new(&mesh).is_err());
}

#[test]
fn island_inside_a_hole_remains_a_separate_oriented_contour() {
    let outer = vec![vec![[0., 0.], [6., 0.], [6., 6.], [0., 6.]]];
    let hole = vec![vec![[1., 1.], [5., 1.], [5., 5.], [1., 5.]]];
    let rings = cad::planar(&outer, &hole, "difference").unwrap();
    let shell = modeling::extrude_rings(&rings, 2., 1, 0., [1., 1.], false).unwrap();
    let island = shifted(mesh::cube([2., 2., 2.], false).unwrap(), [2., 2., 0.]);
    let section = MeshSectionIndex::new(&join(&[shell, island]))
        .unwrap()
        .section(1.)
        .unwrap();
    let mut areas: Vec<_> = section
        .contours
        .iter()
        .map(|c| cad::area(&c.points))
        .collect();
    areas.sort_by(f64::total_cmp);
    assert_eq!(areas, vec![-16., 4., 36.]);
}

#[test]
fn overflowing_interpolation_fails_with_a_numeric_diagnostic() {
    let mesh = Mesh {
        positions: vec![0., 0., -f64::MAX, 1., 0., f64::MAX, 0., 1., f64::MAX],
        indices: vec![0, 1, 2],
        uv: None,
    };
    assert_eq!(
        MeshSectionIndex::new(&mesh)
            .unwrap()
            .section(0.)
            .unwrap_err()
            .code,
        "SECTION_NUMERIC_RANGE"
    );
}
