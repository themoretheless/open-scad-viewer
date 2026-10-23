use nurbs_core::{
    surface::{Axis, Surface},
    surface_tessellation::{self, Report, StopReason},
};
use std::collections::BTreeMap;
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 2.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn oracle(uv: [f64; 2]) -> [f64; 3] {
    let u = (uv[0] - 2.) / 5.;
    let v = (uv[1] + 3.) / 4.;
    let w = (1. + 2. * u) * (1. + v);
    [12. * u / (1. + 2. * u), 8. * v / (1. + v), 12. * u * v / w]
}
fn verify(r: &Report) {
    let mut edges = BTreeMap::new();
    let mut area = 0.;
    for triangle in &r.triangles {
        let t = triangle.vertices;
        let p = t.map(|i| r.parameters[i]);
        let a = ((p[1][0] - p[0][0]) * (p[2][1] - p[0][1])
            - (p[1][1] - p[0][1]) * (p[2][0] - p[0][0]))
            / 2.;
        assert!(a > 0.);
        area += a;
        for [a, b] in [[t[0], t[1]], [t[1], t[2]], [t[2], t[0]]] {
            *edges.entry([a.min(b), a.max(b)]).or_insert(0) += 1;
        }
        for i in 0..=5 {
            for j in 0..=5 - i {
                let bary = [i as f64 / 5., j as f64 / 5., (5 - i - j) as f64 / 5.];
                let mut uv = [0.; 2];
                let mut mesh = [0.; 3];
                for k in 0..3 {
                    for axis in 0..2 {
                        uv[axis] += bary[k] * p[k][axis];
                    }
                    for axis in 0..3 {
                        mesh[axis] += bary[k] * r.points[t[k]][axis];
                    }
                }
                let distance = mesh
                    .into_iter()
                    .zip(oracle(uv))
                    .map(|(a, b)| (a - b).powi(2))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= triangle.error_upper + 1e-12);
            }
        }
    }
    assert!((area - 20.).abs() < 1e-10);
    for ([a, b], count) in edges {
        assert!(count == 1 || count == 2);
        if count == 1 {
            let p = r.parameters[a];
            let q = r.parameters[b];
            assert!(
                (p[0] == q[0] && (p[0] == 2. || p[0] == 7.))
                    || (p[1] == q[1] && (p[1] == -3. || p[1] == 1.)),
                "Unmatched interior edge {p:?} {q:?}"
            );
        }
    }
}
#[test]
fn rational_surface_has_continuous_bounds_and_conforming_shared_edges() {
    let s = panel();
    let before = s.clone();
    let r = surface_tessellation::tessellate(&s, 1., 10000).unwrap();
    assert!(r.within_tolerance);
    assert_eq!(r.reason, StopReason::Tolerance);
    assert!(r.error_upper <= 1. && r.cells <= 10000);
    assert!(r.triangles.len() > 2);
    assert!(r.triangles.iter().all(|t| t.within_tolerance));
    verify(&r);
    assert_eq!(s, before);
}
#[test]
fn exhausted_budget_retains_complete_domain_and_honest_flags() {
    let r = surface_tessellation::tessellate(&panel(), 1e-12, 1).unwrap();
    assert!(!r.within_tolerance);
    assert_eq!(r.reason, StopReason::WorkLimit);
    assert_eq!(r.cells, 1);
    assert_eq!(r.triangles.len(), 2);
    assert!(r.triangles.iter().all(|t| !t.within_tolerance));
    verify(&r);
    let s = nurbs_core::surface_edit::insert(
        &nurbs_core::surface_edit::insert(&panel(), Axis::U, 4., 1).unwrap(),
        Axis::V,
        -1.,
        1,
    )
    .unwrap();
    assert!(surface_tessellation::tessellate(&s, 1., 3).is_err());
    let r = surface_tessellation::tessellate(&s, 100., 4).unwrap();
    assert!(r.within_tolerance);
    verify(&r);
}
#[test]
fn invalid_inputs_and_discontinuous_basis_are_refused() {
    for tolerance in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(surface_tessellation::tessellate(&panel(), tolerance, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(surface_tessellation::tessellate(&panel(), 1., budget).is_err());
    }
    let mut s = panel();
    s.knots_u = vec![2., 2., 4., 4., 7., 7.];
    s.control_points.extend(s.control_points.clone());
    s.weights.extend(s.weights.clone());
    assert!(surface_tessellation::tessellate(&s, 1., 100).is_err());
}
#[test]
fn periodic_surface_is_covered_with_explicitly_cut_seam() {
    let c = nurbs_core::curve::Curve {
        degree: 1,
        knots: vec![0., 1., 2., 3., 4., 5.],
        control_points: vec![
            vec![1., 0., 0.],
            vec![0., 1., 0.],
            vec![-1., 0., 0.],
            vec![1., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: true,
    };
    let s = nurbs_core::surface::extrude(&c, [0., 0., 2.]).unwrap();
    let r = surface_tessellation::tessellate(&s, 100., 3).unwrap();
    assert!(r.within_tolerance);
    assert_eq!(r.triangles.len(), 6);
    assert_eq!(r.points.len(), 8);
    let a = r.parameters.iter().position(|p| *p == [1., 0.]).unwrap();
    let b = r.parameters.iter().position(|p| *p == [4., 0.]).unwrap();
    assert_ne!(a, b);
    assert_eq!(r.points[a], r.points[b]);
}
