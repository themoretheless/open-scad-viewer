use subdivision_core::{
    Cage,
    quad_surface::{
        QuadSurface,
        editing::{DeletionOptions, VertexDeletion},
    },
};
fn grid(n: usize, m: usize) -> QuadSurface {
    QuadSurface::new(Cage {
        vertices: (0..m)
            .flat_map(|j| (0..n).map(move |i| [i as f64, j as f64, 0.1 * (i * j) as f64]))
            .collect(),
        faces: (0..m - 1)
            .flat_map(|j| {
                (0..n - 1).map(move |i| {
                    vec![
                        j * n + i,
                        j * n + i + 1,
                        (j + 1) * n + i + 1,
                        (j + 1) * n + i,
                    ]
                })
            })
            .collect(),
    })
    .unwrap()
}
fn star(n: usize) -> QuadSurface {
    let mut vertices = vec![[0., 0., 0.4]];
    for i in 0..n {
        let a = std::f64::consts::TAU * i as f64 / n as f64;
        vertices.push([a.cos(), a.sin(), 0.1 * a.cos()]);
        let b = a + std::f64::consts::PI / n as f64;
        vertices.push([1.7 * b.cos(), 1.7 * b.sin(), 0.1 * b.cos()]);
    }
    QuadSurface::new(Cage {
        vertices,
        faces: (0..n)
            .map(|i| vec![0, 1 + 2 * i, 2 + 2 * i, 1 + 2 * ((i + 1) % n)])
            .collect(),
    })
    .unwrap()
}
fn check(q: &VertexDeletion) {
    q.surface().cage().validate().unwrap();
    let r = q.reverify(5000000).unwrap();
    assert!(r.topology_preserved && r.projected_fill_proven);
    for (i, j) in q.vertex_map().iter().enumerate() {
        if let Some(j) = j {
            assert_eq!(
                q.source().cage().vertices[i],
                q.surface().cage().vertices[*j]
            );
        }
    }
    assert!(q.surface().cage().faces.iter().all(|f| f.len() == 4));
    for &f in q.replacement_faces() {
        let p = q.surface().evaluate(f, [0.37, 0.61], 4).unwrap();
        assert!(p.iter().all(|v| v.is_finite()));
    }
}
#[test]
fn interior_vertex_retiles_regular_quad_stars_without_moving_other_controls() {
    let source = grid(5, 5);
    let before = source.cage().clone();
    let q = source
        .delete_vertex(12, DeletionOptions::default())
        .unwrap();
    assert_eq!(q.surface().cage().vertices.len(), 24);
    assert_eq!(q.surface().cage().faces.len(), 15);
    assert_eq!(q.vertex_map()[12], None);
    assert_eq!(q.removed_faces().len(), 4);
    assert_eq!(q.replacement_faces().len(), 3);
    for &f in q.removed_faces() {
        assert!(q.face_map()[f].is_none());
    }
    for (f, mapped) in q.face_map().iter().enumerate() {
        if let Some(j) = mapped {
            assert_eq!(
                q.surface().cage().faces[*j],
                source.cage().faces[f]
                    .iter()
                    .map(|v| q.vertex_map()[*v].unwrap())
                    .collect::<Vec<_>>()
            );
        }
    }
    assert!(
        q.edge_map()
            .iter()
            .all(|((a, b), m)| if *a == 12 || *b == 12 {
                m.is_none()
            } else {
                true
            })
    );
    check(&q);
    assert_eq!(source.cage().vertices, before.vertices);
    assert_eq!(source.cage().faces, before.faces);
}
#[test]
fn extraordinary_three_five_six_and_eight_valent_stars_get_retained_quad_geometry() {
    for n in [3, 5, 6, 8] {
        let source = star(n);
        let q = source.delete_vertex(0, DeletionOptions::default()).unwrap();
        assert_eq!(q.surface().cage().vertices.len(), 2 * n);
        assert_eq!(q.replacement_faces().len(), n - 1);
        assert_eq!(q.deleted_vertices(), [0]);
        check(&q);
    }
}
#[test]
fn oblique_closed_cube_deletion_keeps_closed_genus_zero_topology() {
    let mut cage = Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap();
    for p in &mut cage.vertices {
        let q = *p;
        *p = [
            11. + 0.36 * q[0] - 0.8 * q[1] + 0.48 * q[2],
            -7. + 0.48 * q[0] + 0.6 * q[1] + 0.64 * q[2],
            5. - 0.8 * q[0] + 0.6 * q[2],
        ];
    }
    let source = QuadSurface::new(cage).unwrap();
    let q = source.delete_vertex(0, DeletionOptions::default()).unwrap();
    assert_eq!(q.surface().cage().vertices.len(), 7);
    assert_eq!(q.surface().cage().faces.len(), 5);
    check(&q);
}
#[test]
fn paired_boundary_vertices_respect_quad_parity_and_retain_all_other_controls() {
    let source = grid(4, 4);
    assert!(source.delete_vertex(1, DeletionOptions::default()).is_err());
    let q = source
        .delete_vertices(&[1, 2], DeletionOptions::default())
        .unwrap();
    assert_eq!(q.deleted_vertices(), [1, 2]);
    assert_eq!(q.surface().cage().vertices.len(), 14);
    assert_eq!(q.surface().cage().faces.len(), 8);
    check(&q);
}
#[test]
fn invalid_disconnected_or_unbudgeted_stars_publish_no_replacement() {
    let source = grid(6, 6);
    let before = source.cage().clone();
    for ids in [vec![], vec![999], vec![7, 7], vec![7, 28], vec![1]] {
        assert!(
            source
                .delete_vertices(&ids, DeletionOptions::default())
                .is_err()
        );
    }
    let mut o = DeletionOptions::default();
    o.max_candidates = 1;
    assert!(source.delete_vertex(14, o).is_err());
    let mut o = DeletionOptions::default();
    o.predicate_work = 1;
    assert!(source.delete_vertex(14, o).is_err());
    let mut o = DeletionOptions::default();
    o.projection = Some([1., 0., 0.]);
    // Use an exactly planar cage so its edge-on projection cannot be admitted.
    let mut c = source.cage().clone();
    for p in &mut c.vertices {
        p[2] = 0.;
    }
    assert!(QuadSurface::new(c).unwrap().delete_vertex(14, o).is_err());
    let mut crossing = grid(3, 3).cage().clone();
    for p in &mut crossing.vertices {
        p[2] = 0.;
    }
    crossing.vertices.swap(0, 2);
    assert!(
        QuadSurface::new(crossing)
            .unwrap()
            .delete_vertex(4, DeletionOptions::default())
            .is_err()
    );
    let q = source
        .delete_vertex(14, DeletionOptions::default())
        .unwrap();
    assert!(q.reverify(1).is_err());
    assert_eq!(source.cage().vertices, before.vertices);
    assert_eq!(source.cage().faces, before.faces);
}

#[test]
fn connected_interior_groups_do_not_discard_an_unselected_control() {
    let source = grid(5, 5);
    let q = source
        .delete_vertices(&[11, 12], DeletionOptions::default())
        .unwrap();
    assert_eq!(q.surface().cage().vertices.len(), 23);
    assert_eq!(q.surface().cage().faces.len(), 14);
    check(&q);
    let q = source
        .delete_vertices(&[7, 11, 13, 17], DeletionOptions::default())
        .unwrap();
    assert_eq!(q.surface().cage().vertices.len(), 21);
    assert_eq!(q.surface().cage().faces.len(), 12);
    assert_eq!(q.reconstruction_controls().last(), Some(&12));
    assert_eq!(
        q.vertex_map()[12].map(|v| q.surface().cage().vertices[v]),
        Some(source.cage().vertices[12])
    );
    check(&q);
}
#[test]
fn exact_original_projection_keeps_a_thin_star_erased_by_rounded_coordinates() {
    let mut cage = grid(3, 3).cage().clone();
    for (i, p) in cage.vertices.iter_mut().enumerate() {
        *p = [524288., (i / 3) as f64, (i % 3) as f64 * 1e-12];
    }
    assert_eq!(
        cage.vertices[0][0] + cage.vertices[0][2],
        cage.vertices[2][0] + cage.vertices[2][2]
    );
    let source = QuadSurface::new(cage).unwrap();
    let mut o = DeletionOptions::default();
    o.projection = Some([-1., 0., 1.]);
    let q = source.delete_vertex(4, o).unwrap();
    assert_eq!(q.replacement_faces().len(), 3);
    check(&q);
}

#[test]
fn multiple_original_interior_controls_survive_whole_disk_reconstruction() {
    let source = grid(7, 5);
    let selected = (1..4)
        .flat_map(|j| (1..6).map(move |i| j * 7 + i))
        .filter(|i| *i != 16 && *i != 18)
        .collect::<Vec<_>>();
    let q = source
        .delete_vertices(&selected, DeletionOptions::default())
        .unwrap();
    assert_eq!(q.removed_faces().len(), 24);
    assert_eq!(q.surface().cage().vertices.len(), 22);
    assert_eq!(q.replacement_faces().len(), 11);
    let controls = q.reconstruction_controls();
    assert_eq!(&controls[controls.len() - 2..], &[16, 18]);
    check(&q);
}
#[test]
fn outside_or_projectively_coincident_retained_controls_cannot_receive_fill_proof() {
    let source = grid(5, 5);
    let mut options = DeletionOptions::default();
    options.projection = Some([0., 0., 1.]);
    for position in [[100., 100., 0.], [1., 0., 5.]] {
        let mut cage = source.cage().clone();
        cage.vertices[12] = position;
        let source = QuadSurface::new(cage).unwrap();
        let before = source.cage().clone();
        assert!(source.delete_vertices(&[7, 11, 13, 17], options).is_err());
        assert_eq!(source.cage().vertices, before.vertices);
        assert_eq!(source.cage().faces, before.faces);
    }
}

#[test]
fn oblique_thin_interior_reconstruction_uses_original_coordinates() {
    let mut cage = grid(5, 5).cage().clone();
    for (i, p) in cage.vertices.iter_mut().enumerate() {
        // Orthogonal projection along [-1,0,1] retains a very thin disk;
        // rounding x+z first would collapse the whole original width.
        *p = [524288., (i / 5) as f64, (i % 5) as f64 * 1e-12];
    }
    let source = QuadSurface::new(cage).unwrap();
    let mut options = DeletionOptions::default();
    options.projection = Some([-1., 0., 1.]);
    let q = source.delete_vertices(&[7, 11, 13, 17], options).unwrap();
    assert_eq!(q.reconstruction_controls().last(), Some(&12));
    assert_eq!(
        q.vertex_map()[12].map(|v| q.surface().cage().vertices[v]),
        Some(source.cage().vertices[12])
    );
    check(&q);
    let used = q.work().1;
    assert!(used > 1 && used <= options.predicate_work);
    options.predicate_work = 1;
    assert!(source.delete_vertices(&[7, 11, 13, 17], options).is_err());
    assert!(q.reverify(1).is_err());
    check(&q);
}
