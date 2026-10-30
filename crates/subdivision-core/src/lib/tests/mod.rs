use super::*;
fn square() -> Cage {
    Cage {
        vertices: vec![[0., 0., 0.], [1., 0., 0.], [1., 1., 0.], [0., 1., 0.]],
        faces: vec![vec![0, 1, 2, 3]],
    }
}
#[test]
fn boundary_rule_and_identity() {
    let r = square().subdivide(2).unwrap();
    assert_eq!(r.cage.faces.len(), 16);
    assert!(r.face_ids.iter().all(|x| *x == 0));
    let first = square().subdivide(1).unwrap();
    assert_eq!(first.cage.vertices[0], [0.125, 0.125, 0.]);
    assert_eq!(boundary_edges(&first.triangulate().unwrap().0), 8);
}
#[test]
fn invalid_inputs() {
    let mut c = square();
    c.faces.push(vec![0, 1, 2]);
    assert!(c.validate().is_err());
    assert!(square().subdivide(6).is_err());
}
#[test]
fn brush_edits_cage_vertices_locally_and_preserves_faces() {
    let cage = Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap();
    let b = geometry_ops::Brush {
        center: [2., 2., 2.],
        radius: 0.5,
        displacement: [0., 0., 1.],
    };
    let out = cage.brush(&b).unwrap();
    assert_eq!(out.faces, cage.faces);
    assert_eq!(out.vertices.len(), cage.vertices.len());
    let moved: Vec<_> = cage
        .vertices
        .iter()
        .zip(&out.vertices)
        .filter(|(a, b)| a != b)
        .collect();
    assert_eq!(moved.len(), 1);
    assert_eq!(*moved[0].0, [2., 2., 2.]);
    assert_eq!(*moved[0].1, [2., 2., 3.]);
    assert!(out.subdivide(2).unwrap().triangulate().is_ok());
}
#[test]
fn sculpt_uses_cage_normals_and_keeps_faces() {
    use geometry_ops::{Falloff, SculptBrush, SculptKind};
    let cage = Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap();
    let geometry_ops::SculptTarget {
        normals,
        adjacency: rings,
        ..
    } = cage.sculpt_target().unwrap();
    assert_eq!(normals.len(), 8);
    assert!(rings.iter().all(|r| r.len() == 3));
    for (p, n) in cage.vertices.iter().zip(&normals) {
        assert!(math_core::dot(*n, math_core::sub(*p, [1., 1., 1.])) > 0.);
    }
    let inflate = cage
        .sculpt(&SculptBrush {
            falloff: Falloff::Constant,
            ..SculptBrush::new(SculptKind::Inflate { strength: 1. }, [1., 1., 1.], 100.)
        })
        .unwrap();
    assert_eq!(inflate.faces, cage.faces);
    for (p, q) in cage.vertices.iter().zip(&inflate.vertices) {
        assert!((math_core::norm(math_core::sub(*q, *p)) - 1.).abs() < 1e-12);
    }
    let smooth = cage
        .sculpt(&SculptBrush::new(
            SculptKind::Smooth { strength: 1. },
            [2., 2., 2.],
            0.5,
        ))
        .unwrap();
    let mean = [
        (2. + 2. + 0.) / 3.,
        (0. + 2. + 2.) / 3.,
        (2. + 0. + 2.) / 3.,
    ];
    let moved = smooth
        .vertices
        .iter()
        .zip(&cage.vertices)
        .find(|(a, b)| a != b)
        .unwrap()
        .0;
    for k in 0..3 {
        assert!((moved[k] - mean[k]).abs() < 1e-12);
    }
    assert!(smooth.subdivide(2).unwrap().triangulate().is_ok());
    assert!(
        cage.sculpt(&SculptBrush::new(
            SculptKind::Pinch { strength: 2. },
            [0.; 3],
            1.
        ))
        .is_err()
    );
}
#[test]
fn brush_rejects_invalid_brush_and_invalid_cage() {
    let b = geometry_ops::Brush {
        center: [0.; 3],
        radius: 1.,
        displacement: [0., 0., 1.],
    };
    assert!(
        square()
            .brush(&geometry_ops::Brush {
                radius: 0.,
                ..b.clone()
            })
            .is_err()
    );
    assert!(
        square()
            .brush(&geometry_ops::Brush {
                displacement: [f64::NAN; 3],
                ..b.clone()
            })
            .is_err()
    );
    let mut broken = square();
    broken.faces.push(vec![0, 1, 2]);
    assert!(broken.brush(&b).is_err());
    assert_eq!(square().brush(&b).unwrap().vertices[0], [0., 0., 1.]);
}
fn boundary_edges(t: &geometry_ops::Triangles) -> usize {
    let mut edges = std::collections::BTreeMap::new();
    for tri in t.indices.as_chunks::<3>().0 {
        for (a, b) in [(tri[0], tri[1]), (tri[1], tri[2]), (tri[2], tri[0])] {
            let key = (a.min(b), a.max(b));
            *edges.entry(key).or_insert(0) += 1;
        }
    }
    edges.values().filter(|&&n| n == 1).count()
}
#[test]
fn semi_sharp_creases_pin_corners_and_edges() {
    let cage = Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap();
    let smooth = cage.subdivide(2).unwrap();
    let creased = cage
        .subdivide_with_creases(
            2,
            &[(0, 1, 10.0), (1, 2, 10.0), (2, 3, 10.0), (3, 0, 10.0)],
            &[(0, 10.0)],
        )
        .unwrap();
    // Vertex 0 has sharpness 10.0 so it stays pinned at [0., 0., 0.]
    assert_eq!(creased.cage.vertices[0], [0., 0., 0.]);
    assert_ne!(smooth.cage.vertices[0], [0., 0., 0.]);
}
#[test]
fn loop_subdivision_refines_triangle_cage() {
    let tetra = Cage {
        vertices: vec![
            [0., 0., 0.],
            [2., 0., 0.],
            [1., 2., 0.],
            [1., 0.8, 2.],
        ],
        faces: vec![
            vec![0, 2, 1],
            vec![0, 1, 3],
            vec![1, 2, 3],
            vec![2, 0, 3],
        ],
    };
    let refined = tetra.subdivide_loop(2).unwrap();
    assert_eq!(refined.cage.faces.len(), 64);
    assert!(refined.cage.faces.iter().all(|f| f.len() == 3));
    assert!(refined.triangulate().is_ok());

    let loop_cage = tetra.loop_subdivide(2).unwrap();
    assert_eq!(loop_cage.faces.len(), 64);
    let quad_cube = Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap();
    let loop_from_quads = quad_cube.loop_subdivide(1).unwrap();
    assert_eq!(loop_from_quads.faces.len(), 48); // 6 quads -> 12 tris -> 48 tris
}
