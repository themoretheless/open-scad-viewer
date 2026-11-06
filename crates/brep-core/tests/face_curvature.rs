use brep_core::{Model, face_curvature as c};
use c::{Decision, Limits};
use nurbs_core::surface::Surface;
fn work() -> Limits {
    Limits {
        cells: 4000,
        domain_cells: 1000000,
        shape_cells: 100000,
    }
}
fn cylinder() -> Model {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let w = 0.5f64.sqrt();
    m.faces[0].surface = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
            vec![vec![1., 1., 0.], vec![1., 1., 1.]],
            vec![vec![0., 1., 0.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1.; 2], vec![w; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    m
}
#[test]
fn full_plane_cover_admits_but_a_hint_does_not_replace_cover() {
    let m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let before = m.clone();
    let out = c::inspect(&m, 0, 1e-6, 1e-8, work(), &[]).unwrap();
    assert_eq!(out.decision, Decision::Within);
    assert!(out.covered_cells > 0);
    assert!(out.absolute_curvature_upper.unwrap() <= 1e-6);
    assert_eq!(m, before);
    let mut zero = work();
    zero.cells = 0;
    assert_eq!(
        c::inspect(&m, 0, 1e-6, 1e-8, zero, &[[0.5, 0.5]])
            .unwrap()
            .decision,
        Decision::Unresolved
    );
}
#[test]
fn rational_cylinder_has_a_whole_face_bound_and_a_certified_excess() {
    let m = cylinder();
    let before = m.clone();
    let out = c::inspect(&m, 0, 1.1, 1e-8, work(), &[]).unwrap();
    assert_eq!(out.translation_invariant_axes, [false, true]);
    assert_eq!(
        out.decision,
        Decision::Within,
        "{} cells={} shape={} region={:?} bounds={:?}",
        out.reason,
        out.cells,
        out.shape_cells,
        out.unresolved_region,
        out.unresolved_curvature
    );
    assert!(
        out.absolute_curvature_upper.unwrap() >= 1. && out.absolute_curvature_upper.unwrap() <= 1.1
    );
    let out = c::inspect(&m, 0, 0.5, 1e-8, work(), &[[0.5, 0.5]]).unwrap();
    assert_eq!(out.decision, Decision::Exceeds);
    assert!(out.witness_curvature.unwrap()[0] > 0.5);
    assert_eq!(out.witness_uv, Some([0.5, 0.5]));
    assert_eq!(m, before);
}
#[test]
fn an_excess_hint_in_a_hole_never_becomes_a_face_witness() {
    let mut m = cylinder();
    let points = [[0.4, 0.4], [0.4, 0.6], [0.6, 0.6], [0.6, 0.4]];
    let coedges = (0..4)
        .map(|i| {
            brep_core::Coedge::new(
                0,
                false,
                nurbs_core::curve::Curve::from_polyline(vec![
                    points[i].to_vec(),
                    points[(i + 1) % 4].to_vec(),
                ])
                .unwrap(),
            )
        })
        .collect();
    let hole = m.loops.len();
    m.loops.push(brep_core::Loop { coedges });
    m.faces[0].holes.push(hole);
    let mut only_hint = work();
    only_hint.cells = 0;
    let out = c::inspect(&m, 0, 0.5, 1e-8, only_hint, &[[0.5, 0.5]]).unwrap();
    assert_eq!(out.decision, Decision::Unresolved);
    assert!(out.witness_uv.is_none());
}

#[test]
fn loops_outside_the_covered_support_cannot_receive_a_vacuous_curvature_pass() {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let outer = m.faces[0].outer;
    for c in &mut m.loops[outer].coedges {
        for p in &mut c.pcurve.control_points {
            p[0] += 2.;
            p[1] += 2.;
        }
    }
    assert!(c::inspect(&m, 0, 1e-6, 1e-8, work(), &[]).is_err());
}

#[test]
fn almost_extruded_controls_do_not_receive_an_exact_invariance_shortcut() {
    let mut m = cylinder();
    m.faces[0].surface.control_points[1][1][0] += 2. * f64::EPSILON;
    let mut zero = work();
    zero.cells = 0;
    let out = c::inspect(&m, 0, 1.1, 1e-8, zero, &[]).unwrap();
    assert_eq!(out.translation_invariant_axes, [false, false]);
    assert_eq!(out.decision, Decision::Unresolved);
}

#[test]
fn transposed_rational_extrusion_has_the_same_curvature_limit() {
    let mut m = cylinder();
    let old = m.faces[0].surface.clone();
    m.faces[0].surface = Surface {
        degree_u: old.degree_v,
        degree_v: old.degree_u,
        knots_u: old.knots_v,
        knots_v: old.knots_u,
        control_points: (0..2)
            .map(|j| (0..3).map(|i| old.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..3).map(|i| old.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let out = c::inspect(&m, 0, 1.1, 1e-8, work(), &[]).unwrap();
    assert_eq!(out.translation_invariant_axes, [true, false]);
    assert_eq!(out.decision, Decision::Within);
    assert!(out.absolute_curvature_upper.unwrap() <= 1.1);
}

#[test]
fn exhausted_domain_budget_is_unresolved_even_with_a_hint() {
    let m = cylinder();
    let mut limits = work();
    limits.domain_cells = 0;
    let out = c::inspect(&m, 0, 0.5, 1e-8, limits, &[[0.5, 0.5]]).unwrap();
    assert_eq!(out.decision, Decision::Unresolved);
    assert_eq!(out.domain_cells, 0);
    assert!(out.witness_uv.is_none());
}

#[test]
fn exact_original_c2_jets_cover_redundant_knots_without_borrowing_other_supports() {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let source = m.faces[0].surface.clone();
    assert_eq!(source.degree_u, 1);
    let middle = source.control_points[0]
        .iter()
        .zip(&source.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x + y) * 0.5).collect())
        .collect();
    m.faces[0].surface.control_points.insert(1, middle);
    m.faces[0].surface.weights.insert(1, vec![1.; 2]);
    m.faces[0].surface.knots_u = vec![0., 0., 0.5, 1., 1.];
    m.validate().unwrap();
    let before = m.clone();
    let old = c::inspect(&m, 0, 10., 1e-8, work(), &[]).unwrap();
    assert_eq!(old.decision, Decision::Unresolved);
    assert_eq!(old.reason, "continuity_not_proven");
    let exact =
        nurbs_core::analysis::surface_continuity::qualify(&m.faces[0].surface, 2, 100000, 1000000)
            .unwrap();
    let proof = exact.certificate.unwrap();
    let shape = nurbs_core::surface_shape_operator::rectangle_bounds_with_continuity(
        &m.faces[0].surface,
        [[0., 1.], [0., 1.]],
        1000,
        &proof,
    )
    .unwrap();
    assert!(shape.shape.tensor.is_some());
    assert!(
        nurbs_core::surface_shape_operator::rectangle_bounds_with_continuity(
            &before.faces[0].surface,
            [[0., 1.], [0., 1.]],
            1000,
            &proof
        )
        .is_err()
    );
    let r = c::inspect_with_continuity(&m, 0, 10., 1e-8, work(), &[], &proof).unwrap();
    assert_eq!(r.decision, Decision::Within, "{}", r.reason);
    assert!(r.absolute_curvature_upper.unwrap() < 1e-8);
    assert_eq!(m, before);
    let copy = m.faces[0].surface.clone();
    let foreign =
        nurbs_core::analysis::surface_continuity::qualify(&copy, 2, 100000, 1000000).unwrap();
    assert!(
        c::inspect_with_continuity(&m, 0, 10., 1e-8, work(), &[], &foreign.certificate.unwrap())
            .is_err()
    );
    let c1 =
        nurbs_core::analysis::surface_continuity::qualify(&m.faces[0].surface, 1, 100000, 1000000)
            .unwrap();
    assert!(
        c::inspect_with_continuity(&m, 0, 10., 1e-8, work(), &[], &c1.certificate.unwrap())
            .is_err()
    );
    let zero =
        c::inspect_with_continuity(&m, 0, 10., 1e-8, Limits { cells: 0, ..work() }, &[], &proof)
            .unwrap();
    assert_eq!(zero.decision, Decision::Unresolved);
}

#[test]
fn original_span_curvature_requires_exact_closed_cell_ownership() {
    let mut m = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let source = m.faces[0].surface.clone();
    let row = source.control_points[0]
        .iter()
        .zip(&source.control_points[1])
        .map(|(a, b)| a.iter().zip(b).map(|(x, y)| (x + y) * 0.5).collect())
        .collect();
    m.faces[0].surface.control_points.insert(1, row);
    m.faces[0].surface.weights.insert(1, vec![1.; 2]);
    m.faces[0].surface.knots_u = vec![0., 0., 0.5, 1., 1.];
    m.validate().unwrap();
    let s = &m.faces[0].surface;
    use nurbs_core::surface_shape_operator as shape;
    assert!(shape::rectangle_span_bounds(s, [1, 1], [[0., 1.], [0., 1.]], 1000).is_err());
    assert!(shape::rectangle_span_bounds(s, [0, 1], [[0., 0.], [0., 1.]], 1000).is_err());
    for u in [1, 2] {
        let r = shape::rectangle_span_bounds(s, [u, 1], [[0.5, 0.5], [0., 1.]], 1000).unwrap();
        assert!(r.shape.tensor.is_some() && r.shape.normal.is_some());
    }
    let one = c::inspect_one_sided(&m, 0, 10., 1e-8, work()).unwrap();
    assert_eq!(one.original_spans, 2);
    assert_eq!(one.field.decision, Decision::Within);
    let zero = c::inspect_one_sided(&m, 0, 10., 1e-8, Limits { cells: 0, ..work() }).unwrap();
    assert_eq!(zero.field.decision, Decision::Unresolved);
}

#[test]
fn closed_original_span_limits_do_not_borrow_the_neighboring_curvature() {
    // A parabolic cylinder joins its tangent plane with G1 but different
    // one-sided curvature at exactly the SAME original U knot.
    let xs = [0., 0.25, 0.5, 0.75, 1.];
    let zs = [0., 0., 0.25, 0.5, 0.75];
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 2., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..5)
            .map(|i| {
                [0., 1.]
                    .into_iter()
                    .map(|y| vec![xs[i], y, zs[i]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    use nurbs_core::surface_shape_operator as shape;
    let query = [[1., 1.], [0.5, 0.5]];
    let left = shape::rectangle_span_bounds(&s, [2, 1], query, 1000).unwrap();
    let right = shape::rectangle_span_bounds(&s, [4, 1], query, 1000).unwrap();
    let l = shape::absolute_spectral_bounds(left.shape.tensor.unwrap())
        .unwrap()
        .unwrap();
    let r = shape::absolute_spectral_bounds(right.shape.tensor.unwrap())
        .unwrap()
        .unwrap();
    assert!(l[0] > 0.7 && l[1] < 0.72, "{l:?}");
    assert!(r[1] < 1e-8, "{r:?}");
    assert!(
        shape::rectangle_bounds(&s, query, 1000)
            .unwrap()
            .shape
            .tensor
            .is_none()
    );
}
