use super::*;
use nurbs_core::{curve::Curve, intersection::certified_curve_surface::first_in_window};
pub(super) fn source() -> Surface {
    let mut s = super::within_span::loop_surface();
    for (i, row) in s.control_points.iter_mut().enumerate() {
        let z = if i == 3 { 0.25 } else { 0. };
        row[0][2] = z;
        row[1][2] = 2. + z;
    }
    s
}
fn iso(s: &Surface) -> Curve {
    Curve {
        degree: 3,
        knots: s.knots_u.clone(),
        control_points: s
            .control_points
            .iter()
            .map(|row| vec![row[0][0], row[0][1], (row[0][2] + row[1][2]) * 0.5])
            .collect(),
        weights: vec![1.; 4],
        periodic: false,
    }
}
#[test]
fn nonplanar_contact_with_different_transverse_parameters_has_3d_root() {
    let s = source();
    let c = iso(&s);
    let r = first_in_window(&c, &s, [[0., 0.4375], [0.4375, 1.], [0., 1.]], 10000).unwrap();
    assert_eq!(r.roots.len(), 1, "{r:?}");
    let root = &r.roots[0];
    for (i, t) in [0.25, 0.75, 0.44921875].into_iter().enumerate() {
        assert!(
            root.parameter_box[i][0] <= t && t <= root.parameter_box[i][1],
            "{root:?}"
        );
    }
    for (i, x) in [0., 0., 1.00390625].into_iter().enumerate() {
        assert!(root.position_bounds[i][0] <= x && x <= root.position_bounds[i][1]);
    }
    assert!(root.existence_domain[0][1] <= root.existence_domain[1][0]);
    assert!(r.cells <= 10000);
}
#[test]
fn invalid_windows_and_exhausted_cells_do_not_admit_roots() {
    let s = source();
    let c = iso(&s);
    assert!(first_in_window(&c, &s, [[0., 0.], [0., 1.], [0., 1.]], 100).is_err());
    let r = first_in_window(&c, &s, [[0., 0.4375], [0.4375, 1.], [0., 1.]], 1).unwrap();
    assert!(r.roots.is_empty());
    assert!(!r.complete);
    assert_eq!(r.cells, 1);
}

#[test]
fn default_surface_audit_finds_a_nonplanar_different_fiber_contact() {
    let r = surface_self_intersection::inspect(&source(), 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert_eq!(
        c.proof,
        nurbs_core::surface_self_intersection::CrossingProof::CurveSurface
    );
    for (i, t) in [0.25, 0.75].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1]);
    }
    assert_eq!(c.parameter_boxes[0][1], [0.5, 0.5]);
    assert!(c.parameter_boxes[1][1][0] <= 0.44921875 && 0.44921875 <= c.parameter_boxes[1][1][1]);
    assert!(c.parameter_boxes[1][1][1] < 0.5);
    assert!(r.probe_search.cells <= 10000);
    assert!(
        r.probe_search
            .attempts
            .iter()
            .any(|p| p.kind == "curve-surface-forward")
    );
}

#[test]
fn projected_xy_crossing_with_separated_height_bands_is_not_a_3d_root() {
    let mut s = source();
    for row in &mut s.control_points {
        row[1][2] = row[0][2] + 1. / 64.;
    }
    let c = iso(&s);
    let r = first_in_window(&c, &s, [[0., 0.4375], [0.4375, 1.], [0., 1.]], 1000).unwrap();
    assert!(r.roots.is_empty(), "{r:?}");
    assert!(r.cells <= 1000);
}

#[test]
fn reverse_spatial_window_gets_budget_after_an_unresolved_forward_window() {
    let mut s = source();
    for (i, row) in s.control_points.iter_mut().enumerate() {
        row[0][2] = 0.;
        row[1][2] = [3., 1., 0.25, 0.25][i];
    }
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    assert_eq!(
        c.proof,
        nurbs_core::surface_self_intersection::CrossingProof::CurveSurface
    );
    for (i, t) in [0.75, 0.25].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1]);
    }
    let v = c.parameter_boxes[1][1];
    assert!(v[0] <= 3. / 26. && 3. / 26. <= v[1]);
    assert!(r.probe_search.cells <= 10000);
    assert!(
        r.probe_search
            .attempts
            .iter()
            .any(|p| p.kind == "curve-surface-reverse")
    );
}

#[test]
fn original_rational_iso_needs_no_binary64_curve_controls() {
    use nurbs_core::intersection::certified_curve_surface::first_iso_in_window;
    let mut s = source();
    for row in &mut s.weights {
        row[1] = 2.;
    }
    // The central iso has z=4/3+u^3/4. Its exact Cartesian
    // controls cannot be stored in a binary64 Curve.
    let r = first_iso_in_window(&s, 0, 0.5, [[0., 0.4375], [0.4375, 1.], [0., 1.]], 10000).unwrap();
    assert_eq!(r.roots.len(), 1, "{r:?}");
    let root = &r.roots[0];
    for (i, t) in [0.25, 0.75, 473. / 1063.].into_iter().enumerate() {
        assert!(
            root.parameter_box[i][0] <= t && t <= root.parameter_box[i][1],
            "{root:?}"
        );
    }
    let z = 4. / 3. + 1. / 256.;
    assert!(root.position_bounds[2][0] <= z && z <= root.position_bounds[2][1]);
    let audit = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(audit.status, Status::Present, "{audit:?}");
    assert_eq!(
        audit.certified_crossing.unwrap().proof,
        nurbs_core::surface_self_intersection::CrossingProof::CurveSurface
    );
}

#[test]
fn rational_spatial_iso_maps_both_axes_on_nonunit_domains() {
    use nurbs_core::intersection::certified_curve_surface::first_iso_in_window;
    for axis in 0..2 {
        let mut s = source();
        for row in &mut s.weights {
            row[1] = 2.;
        }
        if axis == 1 {
            s.control_points = (0..2)
                .map(|j| s.control_points.iter().map(|row| row[j].clone()).collect())
                .collect();
            s.weights = (0..2)
                .map(|j| s.weights.iter().map(|row| row[j]).collect())
                .collect();
            std::mem::swap(&mut s.degree_u, &mut s.degree_v);
            std::mem::swap(&mut s.knots_u, &mut s.knots_v);
        }
        // Exact normalized .5 maps to a rational natural parameter that need
        // not itself be representable. Original knot bit patterns own the map.
        for t in &mut s.knots_u {
            *t = 0.1 + 3. * *t;
        }
        for t in &mut s.knots_v {
            *t = 0.1 + 3. * *t;
        }
        let d = [0.1, 3.1];
        let map = |t: f64| d[0] + (d[1] - d[0]) * t;
        let mut w = [[d[0], map(0.4375)], d, d];
        w[axis + 1] = [map(0.4375), d[1]];
        let r = first_iso_in_window(&s, axis, 0.5, w, 10000).unwrap();
        assert_eq!(r.roots.len(), 1, "axis={axis}: {r:?}");
        let mut expected = [map(0.25), map(473. / 1063.), map(473. / 1063.)];
        expected[axis + 1] = map(0.75);
        for (i, t) in expected.into_iter().enumerate() {
            assert!(
                r.roots[0].parameter_box[i][0] <= t && t <= r.roots[0].parameter_box[i][1],
                "axis={axis}: {r:?}"
            );
        }
    }
}

#[test]
fn window_solver_rejects_non_3d_target_without_indexing_coordinates() {
    let mut s = source();
    let c = iso(&s);
    for row in &mut s.control_points {
        for p in row {
            p.pop();
        }
    }
    assert!(first_in_window(&c, &s, [[0., 0.4375], [0.4375, 1.], [0., 1.]], 100).is_err());
}
