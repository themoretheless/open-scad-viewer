//! Coincident-face routing, 2D CSG on a shared plane and post-Boolean
//! coplanar cleanup.
use brep_core::boolean::operations::{
    CarrierCoincidence, CarrierKind, boolean, classify_carrier_coincidence,
    coplanar_face_region_boolean, merge_coplanar_faces, route_face_pairs,
};
use brep_core::*;

fn map_points(m: &Model, f: impl Fn(&mut [f64])) -> Model {
    let mut m = m.clone();
    for v in &mut m.vertices {
        f(&mut v.point);
    }
    for e in &mut m.edges {
        for p in &mut e.curve.control_points {
            f(p);
        }
    }
    for fc in &mut m.faces {
        for row in &mut fc.surface.control_points {
            for p in row {
                f(p);
            }
        }
    }
    m
}
fn rot(m: &Model, ang: f64) -> Model {
    let (s, c) = ang.sin_cos();
    map_points(m, |p| {
        let (x, y) = (p[0], p[1]);
        p[0] = c * x - s * y;
        p[1] = s * x + c * y;
    })
}
fn shift(m: &Model, d: [f64; 3]) -> Model {
    map_points(m, |p| {
        for i in 0..3 {
            p[i] += d[i];
        }
    })
}
fn volume(m: &Model) -> f64 {
    analysis::mass_properties(m, 1e-7, 200_000)
        .unwrap()
        .signed_volume_mm3
}
fn check(m: &Model, faces: usize, vol: f64) {
    m.validate().unwrap();
    assert_eq!(m.faces.len(), faces, "face count");
    let loops: usize = m.faces.iter().map(|f| 1 + f.holes.len()).sum();
    let shells = m.shells.len() as i64;
    // Euler-Poincare for genus-0 shells: V - E + F - (L - F) = 2 S.
    assert_eq!(
        m.vertices.len() as i64 - m.edges.len() as i64 + m.faces.len() as i64
            - (loops as i64 - m.faces.len() as i64),
        2 * shells
    );
    assert!((volume(m) - vol).abs() < 1e-6, "{} vs {vol}", volume(m));
}

fn boxes() -> [Model; 5] {
    [
        cuboid([0.; 3], [12., 10., 10.]).unwrap(),
        cuboid([8., 0., 0.], [20., 10., 10.]).unwrap(),
        cuboid([12., 0., 0.], [20., 10., 10.]).unwrap(),
        cuboid([12., 2., 2.], [20., 8., 8.]).unwrap(),
        cuboid([6., 0., 0.], [12., 10., 10.]).unwrap(),
    ]
}

#[test]
fn axis_aligned_coplanar_boxes_give_minimal_faces() {
    let [a, b, t, s, d] = boxes();
    check(&boolean(&a, &b, "union").unwrap(), 6, 2000.);
    check(&boolean(&a, &t, "union").unwrap(), 6, 2000.);
    // A small box on one face: the shared face keeps one hole, no strips.
    check(&boolean(&a, &s, "union").unwrap(), 11, 1200. + 288.);
    check(&boolean(&a, &d, "difference").unwrap(), 6, 720. - 120.);
    check(&boolean(&a, &b, "intersection").unwrap(), 6, 400.);
}

#[test]
fn rotated_coplanar_boxes_no_longer_split_faces() {
    let [a, b, t, s, d] = boxes();
    let r = 0.3;
    let (a, b, t, s, d) = (rot(&a, r), rot(&b, r), rot(&t, r), rot(&s, r), rot(&d, r));
    // Previously 14, 10 and 22 faces: split coplanar facets survived.
    check(&boolean(&a, &b, "union").unwrap(), 6, 2000.);
    check(&boolean(&a, &t, "union").unwrap(), 6, 2000.);
    check(&boolean(&a, &s, "union").unwrap(), 11, 1488.);
    check(&boolean(&a, &d, "difference").unwrap(), 6, 600.);
}

#[test]
fn coaxial_cylinder_union_merges_split_lateral_faces() {
    let c1 = analytic::cylinder(5., 10.).unwrap();
    let c2 = shift(&c1, [0., 0., 6.]);
    let u = boolean(&c1, &c2, "union").unwrap();
    u.validate().unwrap();
    assert_eq!(u.faces.len(), c1.faces.len());
    let expected = std::f64::consts::PI * 25. * 16.;
    assert!((volume(&u) - expected).abs() < 1e-6);
}

#[test]
fn merge_is_idempotent_on_minimal_results() {
    let [a, _, _, s, _] = boxes();
    let split = boolean(&rot(&a, 0.3), &rot(&s, 0.3), "union").unwrap();
    // Cleanup is idempotent on the already minimal result.
    let err = merge_coplanar_faces(&split).unwrap_err();
    assert_eq!(err.code, "BREP_UNSUPPORTED_OPERATION");
    assert_eq!(split.1.faces.len(), split.faces.len());
}

#[test]
fn classification_routes_coincident_pairs_away_from_marching() {
    let [a, _, t, _, _] = boxes();
    let routing = route_face_pairs(&a, &t).unwrap();
    // x=12 faces touch with opposite normals; y/z side faces are coplanar.
    let opposite: Vec<_> = routing
        .coincident
        .iter()
        .filter(|(_, _, c)| {
            matches!(c, CarrierCoincidence::Coincident { opposite_normals: true, .. })
        })
        .collect();
    assert_eq!(opposite.len(), 1);
    assert_eq!(routing.coincident.len(), 5);
    assert!(routing.near_coincident.is_empty());
    assert!(routing.coincident.iter().all(|(i, j, _)| !routing.marching.contains(&(*i, *j))));

    // A plane displaced by 2x tolerance is near-coincident, never marched.
    let tol = a.tolerance_mm;
    let near = cuboid([12. + 2. * tol, 0., 0.], [20., 10., 10.]).unwrap();
    let routing = route_face_pairs(&a, &near).unwrap();
    assert!(!routing.near_coincident.is_empty());
}

#[test]
fn analytic_recognition_classifies_curved_carriers() {
    for (name, m) in [
        ("sphere", analytic::sphere(4.).unwrap()),
        ("torus", analytic::torus(6., 2.).unwrap()),
        ("cone", analytic::frustum(5., 2., 6.).unwrap()),
        ("cylinder", analytic::cylinder(5., 6.).unwrap()),
    ] {
        // A rigidly rotated copy has a different control net on the same carrier.
        let r = rot(&m, 0.7);
        let mut found = false;
        for i in 0..m.faces.len() {
            for j in 0..r.faces.len() {
                if let CarrierCoincidence::Coincident { kind, opposite_normals, .. } =
                    classify_carrier_coincidence(&m, i, &r, j).unwrap()
                {
                    if kind != CarrierKind::Plane {
                        assert!(!opposite_normals, "{name}");
                        found = true;
                    }
                }
            }
        }
        assert!(found, "{name}: curved carrier not recognized as coincident");
        // A scaled copy is a different carrier.
        let scaled = map_points(&m, |p| p.iter_mut().for_each(|x| *x *= 1.1));
        for i in 0..m.faces.len() {
            for j in 0..scaled.faces.len() {
                let c = classify_carrier_coincidence(&m, i, &scaled, j).unwrap();
                if let CarrierCoincidence::Coincident { kind, .. } = c {
                    assert_eq!(kind, CarrierKind::Plane, "{name}: {c:?}");
                }
            }
        }
    }
}

fn top_face(m: &Model, z: f64) -> usize {
    (0..m.faces.len())
        .find(|&f| {
            m.faces[f]
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|p| (p[2] - z).abs() < 1e-9)
        })
        .unwrap()
}
fn region_area(r: &boolean::operations::PlanarRegion) -> f64 {
    let ring = |l: &[[f64; 3]]| {
        let n = (0..l.len()).fold([0.; 3], |s, i| {
            let (p, q) = (l[i], l[(i + 1) % l.len()]);
            [
                s[0] + p[1] * q[2] - p[2] * q[1],
                s[1] + p[2] * q[0] - p[0] * q[2],
                s[2] + p[0] * q[1] - p[1] * q[0],
            ]
        });
        (n[0] * r.normal[0] + n[1] * r.normal[1] + n[2] * r.normal[2]) / 2.
    };
    ring(&r.outer) + r.holes.iter().map(|h| ring(h)).sum::<f64>()
}

#[test]
fn coplanar_region_boolean_on_shared_plane() {
    let a = cuboid([0.; 3], [10., 10., 10.]).unwrap();
    let b = cuboid([5., 5., 0.], [15., 15., 10.]).unwrap();
    let inner = cuboid([2., 2., 0.], [4., 4., 10.]).unwrap();
    let touching = cuboid([10., 0., 0.], [20., 10., 10.]).unwrap();
    let (fa, fb) = (top_face(&a, 10.), top_face(&b, 10.));
    let area = |m: &Model, f, n: &Model, g, op| -> (usize, f64) {
        let r = coplanar_face_region_boolean(m, f, n, g, op).unwrap();
        (r.len(), (r.iter().map(region_area).sum::<f64>() * 1e6).round() / 1e6)
    };
    assert_eq!(area(&a, fa, &b, fb, "union"), (1, 175.));
    assert_eq!(area(&a, fa, &b, fb, "intersection"), (1, 25.));
    assert_eq!(area(&a, fa, &b, fb, "difference"), (1, 75.));
    // Contained square: difference keeps one region with a hole.
    let fi = top_face(&inner, 10.);
    let r = coplanar_face_region_boolean(&a, fa, &inner, fi, "difference").unwrap();
    assert_eq!((r.len(), r[0].holes.len()), (1, 1));
    assert!((region_area(&r[0]) - 96.).abs() < 1e-9);
    // Edge-touching squares: union is one region, intersection is empty.
    let ft = top_face(&touching, 10.);
    assert_eq!(area(&a, fa, &touching, ft, "union"), (1, 200.));
    assert_eq!(area(&a, fa, &touching, ft, "intersection").0, 0);
    // Opposite-normal coincident faces: the bottom of a box stacked on top.
    let stacked = cuboid([5., 5., 10.], [15., 15., 20.]).unwrap();
    let bottom = (0..stacked.faces.len())
        .find(|&f| {
            stacked.faces[f]
                .surface
                .control_points
                .iter()
                .flatten()
                .all(|p| (p[2] - 10.).abs() < 1e-9)
        })
        .unwrap();
    assert_eq!(area(&a, fa, &stacked, bottom, "intersection"), (1, 25.));
    // Non-coplanar faces are refused.
    let other = top_face(&b, 0.);
    assert!(coplanar_face_region_boolean(&a, fa, &b, other, "union").is_err());
}

#[test]
fn convex_overlap_keeps_exact_vertices_on_a_tilted_plane() {
    use brep_core::boolean::operations::exact_convex_coplanar_overlap;
    use num_rational::BigRational as Q;
    let lift = |[x, y]: [f64; 2]| [x, y, x + 2. * y];
    let first = [[0., 0.], [2., 0.], [2., 1.], [0., 1.]].map(lift);
    let mut second = [[0., 0.], [2., 0.], [2., 3.]].map(lift);
    second.reverse();
    let overlap = exact_convex_coplanar_overlap(&first, &second, 100).unwrap();
    let rational = |n: i32, d: i32| Q::new(n.into(), d.into());
    assert_eq!(overlap, vec![
        [rational(0, 1), rational(0, 1), rational(0, 1)],
        [rational(2, 1), rational(0, 1), rational(2, 1)],
        [rational(2, 1), rational(1, 1), rational(4, 1)],
        [rational(2, 3), rational(1, 1), rational(8, 3)],
    ]);
    let touching = [[2., 0.], [3., 0.], [3., 1.], [2., 1.]].map(lift);
    assert!(exact_convex_coplanar_overlap(&first, &touching, 100).unwrap().is_empty());
    assert!(exact_convex_coplanar_overlap(&first, &second, 8).is_err());
    let mut displaced = second;
    displaced[0][2] += 1e-12;
    assert!(exact_convex_coplanar_overlap(&first, &displaced, 100).is_err());
    let concave = [[0., 0.], [2., 0.], [1., 0.5], [2., 1.], [0., 1.]].map(lift);
    assert!(exact_convex_coplanar_overlap(&first, &concave, 100).is_err());
}
