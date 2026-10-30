use super::*;
#[test]
fn coefficient_plane_roots_keep_original_domains_and_refusals() {
    let net: Vec<Vec<_>> = [0., 1.]
        .into_iter()
        .map(|u| {
            [0., 1.]
                .into_iter()
                .map(|v| [I::point(u), I::point(v), I::point(0.), I::point(1.)])
                .collect()
        })
        .collect();
    let out = intersections_coefficients(
        &net,
        [[2., 4.], [3., 7.]],
        [0.25, 0.75, -1.],
        [0., 0., 2.],
        1e-6,
        100,
    )
    .unwrap();
    assert!(out.complete);
    assert_eq!(out.roots.len(), 1);
    let root = &out.roots[0];
    for (a, expected) in [2.5, 6.].into_iter().enumerate() {
        assert!(root.uv[a][0] <= expected && root.uv[a][1] >= expected);
    }
    assert!(root.parameter[0] <= 0.5 && root.parameter[1] >= 0.5);
    let miss = intersections_coefficients(
        &net,
        [[0., 1.]; 2],
        [2., 0.5, -1.],
        [0., 0., 2.],
        1e-6,
        100,
    )
    .unwrap();
    assert!(miss.complete && miss.roots.is_empty());
    let coincident =
        intersections_coefficients(&net, [[0., 1.]; 2], [0., 0.5, 0.], [1., 0., 0.], 1e-6, 100)
            .unwrap();
    assert!(!coincident.complete && !coincident.unresolved.is_empty());
    let zero = intersections_coefficients(
        &net,
        [[0., 1.]; 2],
        [0.25, 0.75, -1.],
        [0., 0., 2.],
        1e-6,
        0,
    )
    .unwrap();
    assert!(!zero.complete && zero.cells == 0 && zero.unresolved.len() == 1);
}
#[test]
fn finite_line_box_exclusion_preserves_crossings_and_tangencies() {
    let bounds = [[0., 1.]; 3];
    let distant = infinite_line_enclosure_parameters(bounds, [10., 0.5, 0.5], [1., 0., 0.])
        .unwrap()
        .unwrap();
    assert!(distant[0] <= -10. && distant[1] >= -9.);
    assert!(
        finite_line_enclosure_parameters(bounds, [10., 0.5, 0.5], [1., 0., 0.])
            .unwrap()
            .is_none()
    );
    assert!(
        infinite_line_enclosure_parameters(bounds, [-1., -3., 0.5], [1., 1., 0.])
            .unwrap()
            .is_none()
    );

    let t = finite_line_enclosure_parameters(bounds, [-1., 0.5, 0.5], [3., 0., 0.])
        .unwrap()
        .unwrap();
    assert!(t[0] <= 1. / 3. && t[1] >= 2. / 3.);
    let reverse = finite_line_enclosure_parameters(bounds, [2., 0.5, 0.5], [-3., 0., 0.])
        .unwrap()
        .unwrap();
    assert!(reverse[0] <= 1. / 3. && reverse[1] >= 2. / 3.);
    assert!(
        finite_line_enclosure_parameters(bounds, [-1., 2., 0.5], [3., 0., 0.])
            .unwrap()
            .is_none()
    );
    assert!(
        finite_line_enclosure_parameters(bounds, [-1., -3., 0.5], [3., 3., 0.])
            .unwrap()
            .is_none()
    );
    assert!(
        finite_line_enclosure_parameters(bounds, [-1., 1., 0.5], [1., 0., 0.])
            .unwrap()
            .is_some()
    );
    assert!(finite_line_enclosure_parameters(bounds, [0.; 3], [0.; 3]).is_err());
}
#[test]
fn rational_quintic_ray_isolation_returns_original_parameter_enclosures() {
    let surface = Surface {
        degree_u: 1,
        degree_v: 5,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 0., 0., 1., 1., 1., 1., 1., 1.],
        control_points: (0..2)
            .map(|u| (0..6).map(|v| vec![u as f64, v as f64 / 5., 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 6], vec![2.; 6]],
        periodic_u: false,
        periodic_v: false,
    };
    let before = surface.clone();
    let result = intersections(&surface, [0.25, 0.37, -1.], [0., 0., 1.], 1e-8, 10000).unwrap();
    assert!(result.complete && result.profile_parameterization.is_some());
    assert_eq!(result.roots.len(), 1);
    let root = &result.roots[0];
    assert!(root.uv[0][0] <= 1. / 7. && root.uv[0][1] >= 1. / 7.);
    assert!(root.parameter[0] <= 1. && root.parameter[1] >= 1.);
    assert!(root.uv[0][1] - root.uv[0][0] <= 1e-8);
    assert_eq!(surface, before);
    let mut folded = surface.clone();
    for row in &mut folded.control_points {
        for (p, y) in row.iter_mut().zip([0., 1., 1., -1., -1., 0.]) {
            p[1] = y;
        }
    }
    let denied = intersections(&folded, [0.25, 0.2, -1.], [0., 0., 1.], 1e-8, 1).unwrap();
    assert!(!denied.complete && !denied.unresolved.is_empty() && denied.cells <= 1);
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 2.], vec![0., 1., 2.]],
            vec![vec![1., 0., 2.], vec![1., 1., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn transverse_plane_root_and_exclusion() {
    let s = plane();
    let r = intersections(&s, [0.37, 0.62, 0.], [0., 0., 1.], 1e-8, 1000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 1);
    for (k, t) in [0.37, 0.62].into_iter().enumerate() {
        assert!(r.roots[0].uv[k][0] <= t && r.roots[0].uv[k][1] >= t)
    }
    assert!(r.roots[0].parameter[0] <= 2. && r.roots[0].parameter[1] >= 2.);
    let r = intersections(&s, [2., 0.62, 0.], [0., 0., 1.], 1e-8, 1000).unwrap();
    assert!(r.complete && r.roots.is_empty());
}
fn folded() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..2)
                    .map(|j| vec![[0.16, -0.34, 0.16][i], j as f64, i as f64 * 0.5])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn covers_two_separate_roots_without_seed_sampling() {
    let s = folded();
    let r = intersections(&s, [0., 0.37, -1.], [0., 0., 1.], 1e-7, 10000).unwrap();
    assert!(r.complete, "{} {:?}", r.cells, r.unresolved);
    assert_eq!(r.roots.len(), 2);
    for (root, u) in r.roots.iter().zip([0.2, 0.8]) {
        assert!(root.uv[0][0] <= u && root.uv[0][1] >= u);
        assert!(root.parameter[0] <= 1. + u && root.parameter[1] >= 1. + u)
    }
}
#[test]
fn mid_plane_crossings_survive_subdivision_and_rounding_stays_explicit() {
    let r = intersections(&folded(), [0., 0.5, -1.], [0., 0., 1.], 1e-7, 10000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 2);
    for root in &r.roots {
        assert!(root.uv[1][0] <= 0.5 && root.uv[1][1] >= 0.5);
    }
    assert_eq!(exact_split(0., 1.), Some(0.375));
    assert_eq!(exact_split(0.5, 0.5_f64.next_up()), None);
}
#[test]
fn tangency_coincidence_and_limits_never_claim_empty_coverage() {
    let mut tangent = folded();
    for (i, row) in tangent.control_points.iter_mut().enumerate() {
        for p in row {
            p[0] = [0.25, -0.25, 0.25][i]
        }
    }
    let r = intersections(&tangent, [0., 0.37, -1.], [0., 0., 1.], 1e-7, 100).unwrap();
    assert!(!r.complete);
    assert!(r.roots.is_empty());
    assert!(!r.unresolved.is_empty());
    let r = intersections(&plane(), [0.37, 0.62, 2.], [1., 0., 0.], 1e-7, 100).unwrap();
    assert!(!r.complete && !r.unresolved.is_empty());
    let r = intersections(&folded(), [0., 0.37, -1.], [0., 0., 1.], 1e-7, 1).unwrap();
    assert!(!r.complete);
    assert_eq!(r.cells, 1);
}
#[test]
fn nonbinary_knots_and_negative_direction_keep_parameter_enclosures() {
    let mut s = folded();
    s.knots_u = vec![0.13, 0.13, 0.13, 3.17, 3.17, 3.17];
    s.knots_v = vec![-0.71, -0.71, 0.29, 0.29];
    let r = intersections(&s, [0., 0.37, 2.], [0., 0., -2.], 1e-7, 10000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 2);
    for (root, u) in r.roots.iter().zip([0.8, 0.2]) {
        let uv = 0.13 + (3.17 - 0.13) * u;
        let t = (2. - u) / 2.;
        assert!(root.uv[0][0] <= uv && root.uv[0][1] >= uv);
        assert!(root.parameter[0] <= t && root.parameter[1] >= t)
    }
}
#[test]
fn rational_weights_and_multiple_spans_are_covered() {
    let mut rational = plane();
    rational.weights = vec![vec![1., 2.], vec![3., 4.]];
    let p = rational.evaluate(0.31, 0.67).unwrap().point;
    let r = intersections(&rational, [p[0], p[1], 0.], [0., 0., 1.], 1e-7, 10000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 1);
    for (k, t) in [0.31, 0.67].into_iter().enumerate() {
        assert!(r.roots[0].uv[k][0] <= t && r.roots[0].uv[k][1] >= t)
    }
    let mut multi = plane();
    multi.knots_u = vec![0., 0., 0.3, 1., 1.];
    multi.control_points = vec![
        vec![vec![0., 0., 2.], vec![0., 1., 2.]],
        vec![vec![0.3, 0., 2.], vec![0.3, 1., 2.]],
        vec![vec![1., 0., 2.], vec![1., 1., 2.]],
    ];
    multi.weights = vec![vec![1.; 2]; 3];
    let r = intersections(&multi, [0.2, 0.63, 0.], [0., 0., 1.], 1e-7, 1000).unwrap();
    assert!(r.complete);
    assert_eq!(r.roots.len(), 1);
    assert!(r.cells >= 2);
}
#[test]
fn outer_boundary_roots_cannot_be_counted_as_isolated_interior_crossings() {
    let r = intersections(&plane(), [0., 0.63, 0.], [0., 0., 1.], 1e-7, 100).unwrap();
    assert!(!r.complete);
    assert!(r.roots.is_empty());
}
