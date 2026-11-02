use subdivision_core::{Cage, quad_surface::QuadSurface};

#[test]
fn valence_five_control_vertex_has_limit_geometry_and_regular_descendants() {
    let mut vertices = vec![[0., 0., 0.4]];
    for i in 0..5 {
        let a = std::f64::consts::TAU * i as f64 / 5.;
        vertices.push([a.cos(), a.sin(), 0.]);
        let b = a + std::f64::consts::PI / 5.;
        vertices.push([1.7 * b.cos(), 1.7 * b.sin(), 0.]);
    }
    let faces = (0..5)
        .map(|i| vec![0, 1 + 2 * i, 2 + 2 * i, 1 + 2 * ((i + 1) % 5)])
        .collect();
    let surface = QuadSurface::new(Cage { vertices, faces }).unwrap();
    let center = surface.evaluate(0, [0., 0.], 0).unwrap();
    assert!((center[2] - 0.2).abs() < 1e-12);
    for face in 0..5 {
        let located = surface.locate(face, [0.6, 0.7], 4).unwrap();
        assert!(located.refinement_depth > 0);
        let p = surface.evaluate(face, [0.6, 0.7], 4).unwrap();
        assert!(p.iter().all(|x| x.is_finite()));
        assert!(p[2] > 0. && p[2] < 0.4);
    }
}
fn cube() -> Cage {
    Cage::extrude(
        &[[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        [0., 0., 2.],
    )
    .unwrap()
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).fold(0f64, |n, (a, b)| n.hypot(a - b))
}
#[test]
fn extraordinary_cube_queries_are_retained_geometry_and_edits_are_immutable() {
    let cage = cube();
    let surface = QuadSurface::new(cage.clone()).unwrap();
    let limits = cage.evaluate_limit_points().unwrap();
    for (fi, f) in cage.faces.iter().enumerate() {
        for (uv, vi) in [
            ([0., 0.], f[0]),
            ([1., 0.], f[1]),
            ([1., 1.], f[2]),
            ([0., 1.], f[3]),
        ] {
            assert!(distance(surface.evaluate(fi, uv, 0).unwrap(), limits[vi]) < 1e-12);
        }
        for uv in [[0.25, 0.25], [0.2, 0.7], [0.65, 0.3], [0.75, 0.75]] {
            let p = surface.evaluate(fi, uv, 4).unwrap();
            let regular = surface.locate(fi, uv, 4).unwrap();
            assert!(distance(p, regular.patch.evaluate(regular.uv[0], regular.uv[1])) < 1e-12);
            // Symmetric convex cube stays strictly inside its original cage.
            assert!(p.iter().all(|x| *x >= 0. && *x <= 2.));
        }
    }
    let edited = surface.with_vertex(0, [-0.5, 0., 0.]).unwrap();
    assert_eq!(surface.cage().vertices, cage.vertices);
    assert_ne!(
        edited.evaluate(0, [0.25, 0.25], 4).unwrap(),
        surface.evaluate(0, [0.25, 0.25], 4).unwrap()
    );
}
#[test]
fn shared_edges_have_one_limit_image_in_both_face_charts() {
    let cage = cube();
    let surface = QuadSurface::new(cage.clone()).unwrap();
    let chart = |edge: usize, t: f64| match edge {
        0 => [t, 0.],
        1 => [1., t],
        2 => [1. - t, 1.],
        _ => [0., 1. - t],
    };
    for a in 0..cage.faces.len() {
        for b in a + 1..cage.faces.len() {
            for i in 0..4 {
                for j in 0..4 {
                    if cage.faces[a][i] == cage.faces[b][(j + 1) % 4]
                        && cage.faces[a][(i + 1) % 4] == cage.faces[b][j]
                    {
                        for t in [0., 0.25, 0.5, 0.75, 1.] {
                            let p = surface.evaluate(a, chart(i, t), 4).unwrap();
                            let q = surface.evaluate(b, chart(j, 1. - t), 4).unwrap();
                            assert!(
                                distance(p, q) < 1e-12,
                                "{a}/{b}, {i}/{j}, {t}: {p:?} != {q:?}"
                            );
                        }
                    }
                }
            }
        }
    }
}
#[test]
fn open_boundary_uses_limit_curve_and_invalid_queries_refuse() {
    let surface = QuadSurface::new(Cage {
        vertices: vec![[0., 0., 0.], [2., 0., 0.], [2., 2., 0.], [0., 2., 0.]],
        faces: vec![vec![0, 1, 2, 3]],
    })
    .unwrap();
    assert!(
        distance(
            surface.evaluate(0, [0.5, 0.], 0).unwrap(),
            [1., 1. / 12., 0.]
        ) < 1e-12
    );
    assert!(surface.evaluate(8, [0.5; 2], 3).is_err());
    assert!(surface.evaluate(0, [f64::NAN, 0.], 3).is_err());
    assert!(surface.locate(0, [0.5; 2], 0).is_err());
    assert!(surface.with_vertex(0, [f64::NAN; 3]).is_err());
    assert!(surface.locate(0, [0.5; 2], 6).is_err());
}
