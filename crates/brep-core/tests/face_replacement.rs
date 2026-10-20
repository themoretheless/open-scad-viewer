use brep_core::{Model, face_replacement};
use nurbs_core::{curve::Curve, surface::Surface};
fn source() -> (Model, usize) {
    let m = brep_core::cuboid([0.; 3], [8.; 3]).unwrap();
    let f = m
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 8.)
        })
        .unwrap();
    (m, f)
}
fn pcs(m: &Model, f: usize) -> Vec<Vec<Curve>> {
    std::iter::once(m.faces[f].outer)
        .chain(m.faces[f].holes.iter().copied())
        .map(|w| {
            m.loops[w]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect()
}
#[test]
fn changed_surface_keeps_owned_edges_and_rejects_mismatched_boundaries() {
    let (m, f) = source();
    let before = m.clone();
    let original = &m.faces[f].surface;
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let mut p = original
                            .evaluate(i as f64 * 0.5, j as f64 * 0.5)
                            .unwrap()
                            .point
                            .to_vec();
                        if i == 1 && j == 1 {
                            p[2] -= 0.4;
                        }
                        p
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let out = face_replacement::replace(&m, f, &s, &pcs(&m, f), 1e-8, 1000000).unwrap();
    assert_eq!(out.model.edges, m.edges);
    assert_eq!(out.model.vertices, m.vertices);
    assert_eq!(out.model.1.edges, m.1.edges);
    assert_eq!(out.model.1.vertices, m.1.vertices);
    assert_ne!(out.face_identity, out.source_identity);
    assert!((out.model.faces[f].surface.evaluate(0.5, 0.5).unwrap().point[2] - 7.9).abs() < 1e-12);
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    let mut bad = s.clone();
    bad.control_points[0][1][2] -= 0.01;
    assert!(face_replacement::replace(&m, f, &bad, &pcs(&m, f), 1e-8, 1000000).is_err());
    assert!(face_replacement::replace(&m, f, &s, &[], 1e-8, 1000000).is_err());
    assert!(face_replacement::replace(&m, f, &s, &pcs(&m, f), 1e-8, 1).is_err());
    assert_eq!(m, before);
}
#[test]
fn split_sector_gets_a_nonplanar_replacement_without_changing_the_retained_sector() {
    let (m, f) = source();
    let outer = &m.loops[m.faces[f].outer].coedges;
    let endpoints = [0, 2].map(|i| m.edges[outer[i].edge].vertices[usize::from(outer[i].reversed)]);
    let pc = Curve::from_polyline(vec![
        outer[0].pcurve.control_points[0].clone(),
        outer[2].pcurve.control_points[0].clone(),
    ])
    .unwrap();
    let curve = Curve {
        control_points: pc
            .control_points
            .iter()
            .map(|p| {
                m.faces[f]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        ..pc.clone()
    };
    let split = brep_core::face_cut::partition(
        &m,
        f,
        endpoints,
        &[brep_core::face_cut::Segment { curve, pcurve: pc }],
        1e-8,
        1000000,
    )
    .unwrap();
    let old = split.model.clone();
    let selected = split.faces[0];
    let retained = split.faces[1];
    // Tensor Bernstein coefficients of u(1-u)v(1-v)(u-v): the
    // authored nonplanar sector preserves the diagonal and all four sides.
    let choose = |n: usize, k: usize| -> f64 {
        match (n, k) {
            (3, 1) | (3, 2) => 3.,
            (2, 1) => 2.,
            (_, 0) => 1.,
            _ if n == k => 1.,
            _ => 0.,
        }
    };
    let terms = [
        (2, 1, 1.),
        (3, 1, -1.),
        (3, 2, 1.),
        (1, 2, -1.),
        (1, 3, 1.),
        (2, 3, -1.),
    ];
    let original = &old.faces[selected].surface;
    let control_points = (0..4)
        .map(|i| {
            (0..4)
                .map(|j| {
                    let mut p = original
                        .evaluate(i as f64 / 3., j as f64 / 3.)
                        .unwrap()
                        .point
                        .to_vec();
                    let z: f64 = terms
                        .iter()
                        .filter(|&&(a, b, _)| a <= i && b <= j)
                        .map(|&(a, b, c)| {
                            c * choose(i, a) / choose(3, a) * choose(j, b) / choose(3, b)
                        })
                        .sum();
                    p[2] -= z;
                    p
                })
                .collect()
        })
        .collect();
    let s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let out =
        face_replacement::replace(&old, selected, &s, &pcs(&old, selected), 1e-8, 1000000).unwrap();
    assert_eq!(out.model.faces[retained], old.faces[retained]);
    assert_eq!(out.model.1.faces[retained], old.1.faces[retained]);
    assert_eq!(out.model.edges, old.edges);
    assert_eq!(out.model.vertices, old.vertices);
    assert!((s.evaluate(0.75, 0.25).unwrap().point[2] - 8.).abs() > 0.01);
    assert_eq!(
        brep_core::watertight::inspect_with_exact(&out.model, 100000, 1000000)
            .unwrap()
            .closure
            .status,
        brep_core::watertight::Closure::WithinTolerance
    );
    assert_eq!(split.model, old);
}
