use super::*;
fn panel(kind: usize, transpose: bool) -> Model {
    let xs = [0., 0.25, 0.5, 0.6, 1.];
    let zs = match kind {
        1 => [0., 0., 0., 0.1, 0.5],
        2 => [0., 0., 0., 0., 0.5],
        _ => [0.; 5],
    };
    let mut surface = Surface {
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
    if kind == 3 {
        surface.control_points.remove(2);
        surface.weights.remove(2);
        surface.knots_u = vec![0., 0., 0., 1., 2., 2., 2.];
    }
    if transpose {
        surface = transposed(&surface);
    }
    let u = [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
    ];
    let v = [
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ];
    use nurbs_core::surface::Axis;
    let curves = [
        surface.iso(Axis::V, v[0]).unwrap(),
        surface.iso(Axis::U, u[1]).unwrap(),
        surface.iso(Axis::V, v[1]).unwrap().reverse().unwrap(),
        surface.iso(Axis::U, u[0]).unwrap().reverse().unwrap(),
    ];
    let mut b = crate::region_replacement::corner_patch_body::Build::empty(1e-7);
    let nodes = curves
        .each_ref()
        .map(|c| b.node(c.control_points[0].clone().try_into().unwrap()));
    let uv = [[u[0], v[0]], [u[1], v[0]], [u[1], v[1]], [u[0], v[1]]];
    let segments = curves
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            (
                nodes[i],
                nodes[(i + 1) % 4],
                c,
                Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap(),
            )
        })
        .collect();
    let face = b.face(surface, segments).unwrap();
    b.m.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face,
            reversed: false,
        }],
        closed: false,
    });
    b.m.rebuild_topology_ids();
    b.m.validate().unwrap();
    b.m
}
#[test]
fn speed_change_is_geometrically_g2_without_raw_parameter_c2() {
    for transpose in [false, true] {
        let m = panel(0, transpose);
        let before = m.clone();
        let raw = nurbs_core::analysis::surface_continuity::qualify(
            &m.faces[0].surface,
            2,
            100000,
            1000000,
        )
        .unwrap();
        assert_eq!(raw.outcome, cad_predicates::BezierIdentity::Different);
        let report = inspect(&m, 1e-5, 1e-4, 10000).unwrap();
        assert!(report.within_tolerance);
        assert_eq!(report.required_joins, 1);
        assert_eq!(report.joins.len(), 1);
        assert_eq!(report.joins[0].axis, usize::from(transpose));
        assert!(report.cells <= 10000);
        assert_eq!(m, before);
        let empty = inspect(&m, 1e-5, 1e-4, 0).unwrap();
        assert!(!empty.within_tolerance);
        assert_eq!(empty.next_join, Some(0));
    }
}
#[test]
fn real_shape_jumps_are_not_excused_as_parameter_speed_changes() {
    for transpose in [false, true] {
        for kind in [1, 2] {
            let m = panel(kind, transpose);
            let report = inspect(&m, 1e-5, 1e-4, 10000).unwrap();
            assert!(!report.within_tolerance);
            let r = report.joins[0].report.as_ref().unwrap();
            assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
            assert!(r.normal[0] > 1e-5 || r.tensor[0] > 1e-4);
        }
    }
}

#[test]
fn unsupported_original_basis_and_invalid_tolerances_do_not_grant_network_admission() {
    let m = panel(3, false);
    let r = inspect(&m, 1e-5, 1e-4, 10000).unwrap();
    assert_eq!(r.required_joins, 1);
    assert_eq!(r.joins.len(), 1);
    assert!(r.joins[0].report.is_err());
    assert!(!r.within_tolerance);
    assert!(inspect(&m, f64::NAN, 1e-4, 10000).is_err());
    assert!(inspect(&m, 1e-5, -1., 10000).is_err());
}

#[test]
fn one_sided_bounds_cannot_replace_a_missing_geometric_join() {
    let m = panel(1, false);
    let limits = crate::face_curvature::Limits {
        cells: 5000,
        domain_cells: 100000,
        shape_cells: 100000,
    };
    let one = crate::face_curvature::inspect_one_sided(&m, 0, 10., 1e-8, limits).unwrap();
    assert_eq!(one.original_spans, 2);
    assert_eq!(one.field.decision, crate::face_curvature::Decision::Within);
    assert!(!inspect(&m, 1e-5, 1e-4, 10000).unwrap().within_tolerance);
    let curved = panel(2, false);
    let bad = crate::face_curvature::inspect_one_sided(&curved, 0, 10., 1e-8, limits).unwrap();
    assert_eq!(bad.field.decision, crate::face_curvature::Decision::Exceeds);
    assert!(bad.field.witness_curvature.unwrap()[0] > 10.);
}
