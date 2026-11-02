use brep_core::{Model, feature_ops::full_round};
use nurbs_core::curve::Curve;

fn wire(points: &[[f64; 2]]) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % points.len()].to_vec(),
            ])
            .unwrap()
        })
        .collect()
}
fn selected(m: &Model) -> [usize; 3] {
    [(0, 0.), (1, 8.), (0, 4.)].map(|(axis, value)| {
        m.faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| (p[axis] - value).abs() < 1e-9)
                    && f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .any(|p| p[2] == 6.)
                    && f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .any(|p| p[2] == 0.)
            })
            .unwrap()
    })
}

#[test]
fn full_round_preserves_nonrectangular_rear_and_through_hole() {
    let outer = wire(&[[0., 0.], [1., -1.], [3., -1.], [4., 0.], [4., 8.], [0., 8.]]);
    let hole = wire(&[[1., 1.], [1., 2.], [2., 2.], [2., 1.]]);
    let source = brep_core::prism::extrude(&[outer, hole], 0., 6.).unwrap();
    let before = source.clone();
    let r = full_round(&source, selected(&source)).unwrap();
    assert_eq!(source, before);
    assert!((r.radius_mm - 2.).abs() < 1e-10);
    assert!(r.audit.ok);
    assert_eq!(r.tangency.model(), &r.model);
    assert_eq!(r.tangency.seams().len(), 3);
    assert_eq!(r.tangency.envelopes().len(), 2);
    assert!(
        r.tangency
            .envelopes()
            .iter()
            .all(|(_, p)| p.radius().error_upper() <= r.model.tolerance_mm)
    );
    let p = brep_core::prism::recognize(&r.model).unwrap().unwrap();
    assert_eq!(p.loops.len(), 2);
    assert_eq!(p.loops[1].len(), 4);
    for point in [[1., -1., 0.], [3., -1., 6.], [1., 1., 0.], [2., 2., 6.]] {
        assert!(r.model.vertices.iter().any(|v| v.point == point));
    }
    let volume = brep_core::analysis::mass_properties(&r.model, 1e-6, 2_000_000)
        .unwrap()
        .signed_volume_mm3;
    // Original area: rectangle 32 + rear trapezoid 3 - hole 1.
    let expected = (34. - 8. + 2. * std::f64::consts::PI) * 6.;
    assert!((volume - expected).abs() < 1e-4, "{volume} vs {expected}");
    let f = selected(&source);
    let swapped = full_round(&source, [f[2], f[1], f[0]]).unwrap();
    assert!(swapped.audit.ok);
}

#[test]
fn full_round_refuses_rear_profile_intruding_into_removed_strip() {
    let outer = wire(&[
        [0., 0.],
        [1., 0.],
        [1., 7.],
        [3., 7.],
        [3., 0.],
        [4., 0.],
        [4., 8.],
        [0., 8.],
    ]);
    let source = brep_core::prism::extrude(&[outer], 0., 6.).unwrap();
    let before = source.clone();
    let err = full_round(&source, selected(&source)).unwrap_err();
    assert!(err.to_string().contains("replacement strip"), "{err}");
    assert_eq!(source, before);
}

#[test]
fn full_round_retains_rational_curved_rear_carriers() {
    let mut outer = Vec::new();
    for start in [180., 270.] {
        let mut arc =
            nurbs_core::primitives::circle_arc([2., 0., 0.], [0., 0., 1.], 2., start, 90.).unwrap();
        for p in &mut arc.control_points {
            p.truncate(2);
        }
        outer.push(arc);
    }
    let retained = outer.clone();
    for (a, b) in [
        ([4., 0.], [4., 8.]),
        ([4., 8.], [0., 8.]),
        ([0., 8.], [0., 0.]),
    ] {
        outer.push(Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap());
    }
    let source = brep_core::prism::extrude(&[outer], 0., 6.).unwrap();
    let r = full_round(&source, selected(&source)).unwrap();
    let profile = brep_core::prism::recognize(&r.model).unwrap().unwrap();
    for curve in retained {
        assert!(profile.loops[0].iter().any(|c| c.degree == curve.degree
            && c.control_points == curve.control_points
            && c.weights == curve.weights));
    }
    let volume = brep_core::analysis::mass_properties(&r.model, 1e-6, 2_000_000)
        .unwrap()
        .signed_volume_mm3;
    assert!((volume - (24. + 4. * std::f64::consts::PI) * 6.).abs() < 1e-4);
}

#[test]
fn full_round_selection_is_bound_to_actual_face_not_only_its_plane() {
    let outer = wire(&[
        [0., 0.],
        [1., -1.],
        [3., -1.],
        [4., 0.],
        [4., 8.],
        [0., 8.],
        [0., 3.],
    ]);
    let source = brep_core::prism::extrude(&[outer], 0., 6.).unwrap();
    let faces = selected(&source);
    let wrong = source
        .faces
        .iter()
        .position(|f| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[0] == 0. && p[1] <= 3.)
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|p| p[2] == 6.)
                && f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .any(|p| p[2] == 0.)
        })
        .unwrap();
    assert_ne!(wrong, faces[0]);
    let before = source.clone();
    let error = full_round(&source, [wrong, faces[1], faces[2]]).unwrap_err();
    assert!(error.to_string().contains("does not own"), "{error}");
    assert_eq!(source, before);
    assert!(full_round(&source, faces).unwrap().audit.ok);
}
