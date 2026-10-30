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
