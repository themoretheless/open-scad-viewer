use subdivision_core::{
    Cage,
    quad_surface::{
        QuadSurface,
        editing::{self, StripFace},
    },
};
fn tube(n: usize, z: f64) -> QuadSurface {
    let vertices = (0..2)
        .flat_map(|j| {
            (0..n).map(move |i| {
                let t = std::f64::consts::TAU * i as f64 / n as f64;
                [t.cos(), t.sin(), z + j as f64]
            })
        })
        .collect();
    let faces = (0..n)
        .map(|i| vec![i, (i + 1) % n, n + (i + 1) % n, n + i])
        .collect();
    QuadSurface::new(Cage { vertices, faces }).unwrap()
}
fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    (0..3).fold(0f64, |d, k| d.hypot(a[k] - b[k]))
}
#[test]
fn boundary_cycle_reconciliation_builds_one_retained_manifold_limit_surface() {
    let a = tube(8, 0.);
    let b = tube(8, 1.);
    let before = (a.cage().clone(), b.cage().clone());
    let out = editing::reconcile(
        &a,
        &(8..16).collect::<Vec<_>>(),
        &b,
        &(0..8).collect::<Vec<_>>(),
        0.,
    )
    .unwrap();
    assert_eq!(out.surface().cage().vertices.len(), 24);
    assert_eq!(out.surface().cage().faces.len(), 16);
    assert_eq!(out.maximum_control_displacement(), 0.);
    for i in 0..8 {
        assert_eq!(out.vertex_maps()[0][8 + i], out.vertex_maps()[1][i]);
    }
    for t in [0.1, 0.4, 0.7, 0.9] {
        let original = a.evaluate(0, [t, 1.], 0).unwrap();
        let joined = out.surface().evaluate(0, [t, 1.], 4).unwrap();
        assert!(distance(original, joined) < 1e-12);
    }
    assert_eq!(a.cage().vertices, before.0.vertices);
    assert_eq!(b.cage().vertices, before.1.vertices);
    assert!(editing::reconcile(&a, &[8, 9, 10], &b, &[0, 1, 2], 0.).is_err());
    let mut wrong = (0..8).collect::<Vec<_>>();
    wrong.reverse();
    assert!(editing::reconcile(&a, &(8..16).collect::<Vec<_>>(), &b, &wrong, 0.).is_err());
}
#[test]
fn reconciliation_displacement_allowance_is_enforced_before_publication() {
    let a = tube(8, 0.);
    let b = tube(8, 1.2);
    let la = (8..16).collect::<Vec<_>>();
    let lb = (0..8).collect::<Vec<_>>();
    assert!(editing::reconcile(&a, &la, &b, &lb, 0.1).is_err());
    let out = editing::reconcile(&a, &la, &b, &lb, 0.3).unwrap();
    assert!((out.maximum_control_displacement() - 0.1).abs() < 1e-12);
    out.surface().cage().validate().unwrap();
}
#[test]
fn deleting_a_closed_quad_strip_removes_control_ring_and_keeps_genus_one_topology() {
    let n = 6;
    let m = 6;
    let vertices = (0..m)
        .flat_map(|j| {
            (0..n).map(move |i| {
                let a = std::f64::consts::TAU * i as f64 / n as f64;
                let b = std::f64::consts::TAU * j as f64 / m as f64;
                [
                    (2. + 0.5 * b.cos()) * a.cos(),
                    (2. + 0.5 * b.cos()) * a.sin(),
                    0.5 * b.sin(),
                ]
            })
        })
        .collect();
    let faces = (0..m)
        .flat_map(|j| {
            (0..n).map(move |i| {
                vec![
                    j * n + i,
                    j * n + (i + 1) % n,
                    ((j + 1) % m) * n + (i + 1) % n,
                    ((j + 1) % m) * n + i,
                ]
            })
        })
        .collect();
    let source = QuadSurface::new(Cage { vertices, faces }).unwrap();
    let before = source.cage().clone();
    let strip = (0..n)
        .map(|i| StripFace {
            face: 2 * n + i,
            across_edge: 1,
        })
        .collect::<Vec<_>>();
    let out = source.delete_strip(&strip).unwrap();
    assert_eq!(out.surface().cage().faces.len(), 30);
    assert_eq!(out.surface().cage().vertices.len(), 30);
    for i in 0..n {
        assert_eq!(out.vertex_map()[2 * n + i], out.vertex_map()[3 * n + i]);
        assert!(out.face_map()[2 * n + i].is_none());
    }
    out.surface().cage().validate().unwrap();
    assert!(
        out.surface()
            .evaluate(0, [0.3, 0.6], 3)
            .unwrap()
            .iter()
            .all(|x| x.is_finite())
    );
    assert!((out.maximum_control_displacement() - 0.25).abs() < 1e-12);
    assert_eq!(source.cage().vertices, before.vertices);
    assert_eq!(source.cage().faces, before.faces);
    assert!(source.delete_strip(&strip[..1]).is_err());
    assert!(source.delete_strip(&[strip[0], strip[0]]).is_err());
    let mut bad = strip.clone();
    bad[2].across_edge = 0;
    assert!(source.delete_strip(&bad).is_err());
}
