use super::*;

#[test]
fn analytic_fillet_af01_cuboid_edge() {
    let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            (a[0] - b[0]).abs() <= 1e-12
                && (a[1] - b[1]).abs() <= 1e-12
                && (a[0] - 10.).abs() <= 1e-9
                && (a[1] - 10.).abs() <= 1e-9
        })
        .expect("vertical +X/+Y edge");
    let (out, cert) = analytic_fillet(&model, edge, 1.).unwrap();
    assert!(cert.complete);
    assert!(
        out.faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    );
}

#[test]
fn curved_fillet_refuses_af_n1() {
    let model = cylinder(2., 4.).unwrap();
    assert_eq!(
        analytic_fillet(&model, 0, 0.5).unwrap_err().code,
        "BREP_ANALYTIC_FILLET_REFUSED"
    );
}

#[test]
fn planar_shell_as01() {
    let model = cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let (out, cert) = analytic_shell(&model, 0.4).unwrap();
    assert!(cert.complete);
    out.validate().unwrap();
}

#[test]
#[cfg(feature = "codec")]
fn exact_shell_offsets_planar_inward_outward_and_nonadjacent_openings() {
    let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    for direction in ["inward", "outward"] {
        let closed = exact_analytic_shell(&source, &[], 0.5, direction).unwrap();
        assert_eq!(closed.feature.capability, EXACT_ANALYTIC_SHELL_CAPABILITY);
        assert!(closed.audit.ok && closed.naming_complete);
        assert_eq!(closed.model.bodies.len(), 1);
        assert_eq!(closed.model.bodies[0].inner_shells.len(), 1);
        let opened = exact_analytic_shell(&source, &[0, 1], 0.5, direction).unwrap();
        assert!(opened.audit.ok && opened.naming_complete);
    }
    assert_eq!(before, value_codec::to_string(&source).unwrap());
}

#[test]
fn exact_shell_has_independent_box_volume_area_thickness_oracles() {
    let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let thickness = 0.5;
    let inward = exact_analytic_shell(&source, &[], thickness, "inward")
        .unwrap()
        .model;
    let properties = crate::analysis::mass_properties(&inward, 1e-10, 300_000).unwrap();
    let expected_volume = 10. * 8. * 6. - 9. * 7. * 5.;
    let expected_area =
        2. * (10. * 8. + 10. * 6. + 8. * 6.) + 2. * (9. * 7. + 9. * 5. + 7. * 5.);
    assert!((properties.signed_volume_mm3 - expected_volume).abs() < 1e-6);
    assert!((properties.surface_area_mm2 - expected_area).abs() < 1e-6);
    let xs = inward
        .vertices
        .iter()
        .map(|vertex| vertex.point[0])
        .collect::<Vec<_>>();
    assert!(xs.iter().any(|x| (*x - thickness).abs() < 1e-9));
    assert!(xs.iter().any(|x| (*x - (10. - thickness)).abs() < 1e-9));
}

#[test]
fn exact_cylinder_shell_is_rigid_stable_and_refuses_unowned_rims() {
    let source = crate::cylinder(4., 6.).unwrap();
    let placed = crate::transform::affine(
        &source,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let caps = placed
        .faces
        .iter()
        .enumerate()
        .filter_map(|(index, face)| {
            (face.surface.degree_u == 1 && face.surface.degree_v == 1).then_some(index)
        })
        .collect::<Vec<_>>();
    let opened = exact_analytic_shell(&placed, &caps, 0.5, "outward").unwrap();
    assert!(opened.audit.ok && opened.naming_complete);
    assert_eq!(opened.model.faces.len(), 10);
    assert_eq!(
        exact_analytic_shell(&placed, &caps[..1], 0.5, "inward")
            .unwrap_err()
            .code,
        "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED"
    );
}

#[test]
fn exact_tube_offsets_preserve_topology_under_rigid_placement() {
    let source = tube(5., 2., 8.).unwrap();
    let placed = crate::transform::affine(
        &source,
        [
            [0., -1., 0., 4.],
            [0., 0., 1., -7.],
            [-1., 0., 0., 3.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for direction in ["inward", "outward"] {
        let result = exact_analytic_shell(&placed, &[], 0.25, direction).unwrap();
        assert!(result.audit.ok && result.naming_complete);
        assert_eq!(
            (
                result.model.vertices.len(),
                result.model.edges.len(),
                result.model.faces.len(),
            ),
            (16, 24, 10)
        );
    }
}

#[test]
#[cfg(feature = "codec")]
fn exact_shell_refusals_are_atomic_and_typed() {
    let source = cuboid([0.; 3], [4.; 3]).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    assert_eq!(
        exact_analytic_shell(&source, &[0, 2], 0.25, "inward")
            .unwrap_err()
            .code,
        "BREP_ANALYTIC_SHELL_TRANSITION_REFUSED"
    );
    assert_eq!(
        exact_analytic_shell(&source, &[], 2.1, "inward")
            .unwrap_err()
            .code,
        "BREP_ANALYTIC_SHELL_FEASIBILITY_REFUSED"
    );
    assert_eq!(before, value_codec::to_string(&source).unwrap());
}

#[test]
fn fillet_chain_remaps_two_vertical_corners() {
    let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edges: Vec<usize> = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            if (a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12 {
                Some(i)
            } else {
                None
            }
        })
        .take(2)
        .collect();
    assert_eq!(edges.len(), 2);
    let (out, cert) = analytic_fillet_chain(&model, &edges, 0.8).unwrap();
    assert!(cert.notes.contains(&"af01_fillet_chain_remapped"));
    out.validate().unwrap();
}

#[test]
fn successor_multi_edge_fillet_is_context_audit_and_naming_bound() {
    let model = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let edges = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, edge)| {
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12).then_some(i)
        })
        .take(2)
        .collect::<Vec<_>>();
    let certified = audited_multi_edge_fillet(&model, &edges, 0.5).unwrap();
    assert_eq!(
        certified.feature.capability,
        AUDITED_MULTI_EDGE_FILLET_CAPABILITY
    );
    assert_eq!(certified.context, certified.evidence.context);
    assert!(certified.audit.ok && certified.naming_complete);
    assert!(!certified.change_set.changes.is_empty());
}

#[test]
#[cfg(feature = "codec")]
fn simple_prism_fillet_certifies_concave_bracket_and_refuses_inner_corner() {
    let profile = [
        [0., 0.],
        [40., 0.],
        [40., 5.],
        [5., 5.],
        [5., 30.],
        [0., 30.],
    ];
    let source = extrude_polygon(&profile, 0., 20.).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    let edge_at = |x: f64, y: f64| {
        source
            .edges
            .iter()
            .position(|e| {
                let [a, b] = e.vertices.map(|v| source.vertices[v].point);
                (a[0] - x).abs() < 1e-9
                    && (b[0] - x).abs() < 1e-9
                    && (a[1] - y).abs() < 1e-9
                    && (b[1] - y).abs() < 1e-9
                    && (a[2] - b[2]).abs() > 19.
            })
            .unwrap()
    };
    let outer = edge_at(0., 0.);
    assert!(exact_convex_prism_fillet(&source, &[outer], 1.).is_err());
    for radius in [0.25, 1., 16.] {
        let result = exact_simple_prism_fillet(&source, &[outer], radius).unwrap();
        assert_eq!(
            result.feature.capability,
            EXACT_SIMPLE_PRISM_FILLET_CAPABILITY
        );
        assert!(result.feature.complete && result.audit.ok && result.naming_complete);
        let volume = crate::analysis::mass_properties(&result.model, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        let expected = 6500. - radius * radius * (1. - std::f64::consts::PI / 4.) * 20.;
        assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
    }
    for (edge, radius) in [(outer, 20.), (edge_at(5., 5.), 1.)] {
        assert!(exact_simple_prism_fillet(&source, &[edge], radius).is_err());
    }
    let placed = crate::transform::affine(
        &source,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let result = exact_simple_prism_fillet(&placed, &[outer], 1.).unwrap();
    assert!(result.audit.ok && result.naming_complete);
    assert_eq!(value_codec::to_string(&source).unwrap(), before);
}

#[test]
fn tube_recognition_checks_rational_supports_not_only_vertices() {
    let source = crate::tube(20., 5., 6.).unwrap();
    assert!(recognize_analytic_tube(&source).is_some());
    let outer = crate::sketch::circle_wire(20.).unwrap();
    let mut inner = crate::sketch::circle_wire(5.).unwrap();
    inner.reverse();
    for curve in &mut inner {
        curve.control_points.reverse();
        curve.weights.reverse();
    }
    let extruded = crate::prism::extrude(&[outer, inner], 0., 6.).unwrap();
    assert!(
        recognize_analytic_tube(&extruded).is_some(),
        "Nested-circle extrusion must be admitted"
    );
    let placed = crate::transform::affine(
        &source,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert!(recognize_analytic_tube(&placed).is_some());
    let index = source
        .faces
        .iter()
        .position(|f| f.surface.degree_u == 2)
        .unwrap();
    let mut wrong_weight = source.clone();
    wrong_weight.faces[index].surface.weights[1][0] *= 0.95;
    assert!(recognize_analytic_tube(&wrong_weight).is_none());
    let mut wrong_support = source.clone();
    wrong_support.faces[index].surface.control_points[1][0][0] += 0.1;
    assert!(recognize_analytic_tube(&wrong_support).is_none());
    let elliptic = crate::transform::affine(
        &source,
        [
            [1.1, 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert!(recognize_analytic_tube(&elliptic).is_none());
}

#[test]
#[cfg(feature = "codec")]
fn annular_fillet_owns_complete_rims_and_refuses_partial_arcs() {
    let source = crate::tube(20., 5., 6.).unwrap();
    let before = value_codec::to_string(&source).unwrap();
    let mut rings: [Vec<usize>; 4] = std::array::from_fn(|_| Vec::new());
    for (i, e) in source.edges.iter().enumerate() {
        if e.curve.degree != 2 {
            continue;
        }
        let p = source.vertices[e.vertices[0]].point;
        let outer = p[0].hypot(p[1]) > 10.;
        let top = p[2] > 3.;
        rings[match (outer, top) {
            (false, false) => 0,
            (true, false) => 1,
            (true, true) => 2,
            (false, true) => 3,
        }]
        .push(i);
    }
    for mask in [1usize, 2, 4, 8, 5, 15] {
        let edges: Vec<_> = (0..4)
            .filter(|i| mask & (1 << i) != 0)
            .flat_map(|i| rings[i].iter().copied())
            .collect();
        let result = exact_annular_fillet(&source, &edges, 1.).unwrap();
        assert!(result.feature.complete && result.audit.ok && result.naming_complete);
        assert_eq!(result.feature.capability, EXACT_ANNULAR_FILLET_CAPABILITY);
        if mask == 4 {
            for edge in &rings[1] {
                assert!(
                    result.model.1.edges.contains(&source.1.edges[*edge]),
                    "Untouched opposite rim lost its identity"
                );
            }
            assert!(!result.change_set.changes.is_empty());
        }
        let area = 1. - std::f64::consts::PI / 4.;
        let moment: f64 = (0..4)
            .filter(|i| mask & (1 << i) != 0)
            .map(|i| {
                if i == 1 || i == 2 {
                    19. * area + 1. / 6.
                } else {
                    6. * area - 1. / 6.
                }
            })
            .sum();
        let expected = 2250. * std::f64::consts::PI - 2. * std::f64::consts::PI * moment;
        let volume = crate::analysis::mass_properties(&result.model, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        assert!(
            (volume - expected).abs() < 2e-5,
            "mask {mask}: {volume} != {expected}"
        );
    }
    assert!(exact_annular_fillet(&source, &rings[2][..1], 1.).is_err());
    let all: Vec<_> = rings.iter().flatten().copied().collect();
    assert!(exact_annular_fillet(&source, &all, 3.).is_err());
    let placed = crate::transform::affine(
        &source,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert!(
        exact_annular_fillet(&placed, &rings[2], 1.)
            .unwrap()
            .audit
            .ok
    );
    assert_eq!(value_codec::to_string(&source).unwrap(), before);
}

#[test]
#[cfg(feature = "codec")]
fn enclosure_outer_round_preserves_the_open_cavity() {
    let outer = cuboid([0., 0., 0.], [40., 30., 20.]).unwrap();
    let tool = cuboid([2., 2., 2.], [38., 28., 22.]).unwrap();
    let source = boolean(&outer, &tool, "difference").unwrap();
    let before = value_codec::to_string(&source).unwrap();
    let edge = outer
        .edges
        .iter()
        .position(|e| {
            e.vertices.iter().all(|i| {
                let p = outer.vertices[*i].point;
                p[0] == 0. && p[1] == 0.
            })
        })
        .unwrap();
    let source_edges: Vec<_> = source
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, e)| {
            e.vertices
                .iter()
                .all(|i| {
                    let p = source.vertices[*i].point;
                    p[0] == 0. && p[1] == 0.
                })
                .then_some(index)
        })
        .collect();
    let source_edge = source_edges[0];
    for radius in [1., 4.] {
        let public = exact_layered_prism_fillet(&source, &source_edges, radius).unwrap();
        assert!(public.naming_complete);
        let admitted =
            round_layered_prism_in_envelope(&source, &outer, &source_edges, radius).unwrap();
        audit_solid(&admitted).unwrap();
    }
    let angle: f64 = 0.37;
    let (sin, cos) = angle.sin_cos();
    let placed = crate::transform::affine(
        &source,
        [
            [cos, -sin, 0., 7.],
            [0., 0., -1., 11.],
            [sin, cos, 0., -3.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let placed_before = value_codec::to_string(&placed).unwrap();
    let rotated = exact_layered_prism_fillet(&placed, &source_edges, 1.).unwrap();
    assert!(rotated.naming_complete);
    let rotated_volume = crate::analysis::mass_properties(&rotated.model, 1e-9, 300_000)
        .unwrap()
        .signed_volume_mm3;
    assert!((rotated_volume - (7152. - (1. - std::f64::consts::PI / 4.) * 20.)).abs() < 2e-5);
    assert_eq!(value_codec::to_string(&placed).unwrap(), placed_before);
    assert!(exact_layered_prism_fillet(&source, &source_edges, 8.).is_err());
    assert!(exact_layered_prism_fillet(&source, &source_edges[..1], 1.).is_err());
    assert!(round_layered_prism_in_envelope(&source, &outer, &source_edges, 8.).is_err());
    assert!(
        round_layered_prism_in_envelope(&source, &outer, &[source_edge, source_edge], 1.)
            .is_err()
    );
    assert!(
        round_layered_prism_in_envelope(&source, &outer, &[source.edges.len()], 1.).is_err()
    );
    let rounded = exact_convex_prism_fillet(&outer, &[edge], 1.)
        .unwrap()
        .model;
    let result = boolean(&source, &rounded, "intersection").unwrap();
    audit_solid(&result).unwrap();
    let volume = crate::analysis::mass_properties(&result, 1e-9, 300_000)
        .unwrap()
        .signed_volume_mm3;
    let expected = 7152. - (1. - std::f64::consts::PI / 4.) * 20.;
    assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
    assert_eq!(result.bodies.len(), 1);
    let layers = crate::stepped_prism::recognize(&source).unwrap().unwrap();
    assert_eq!(layers.len(), 2);
    let envelope = crate::prism::recognize(&outer).unwrap().unwrap();
    for (radius, allowed) in [(1., true), (4., true), (8., false)] {
        let candidate = exact_convex_prism_fillet(&outer, &[edge], radius)
            .unwrap()
            .model;
        let profile = crate::prism::recognize(&candidate).unwrap().unwrap();
        let removed = crate::planar_trim::boolean(
            &envelope.loops,
            &profile.loops,
            "difference",
            source.tolerance_mm,
        )
        .unwrap();
        let mut inside_material = true;
        for layer in &layers {
            let outside = crate::planar_trim::boolean(
                &removed,
                &layer.profile,
                "difference",
                source.tolerance_mm,
            )
            .unwrap();
            inside_material &= outside.is_empty();
        }
        assert_eq!(inside_material, allowed, "radius {radius} cavity collision");
    }
    // Multiple outer rounds must retain the same cavity, including after
    // serialization removes any dependence on construction-time state.
    let restored: Model = value_codec::from_str(&before).unwrap();
    let corners: Vec<_> = outer
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let a = outer.vertices[e.vertices[0]].point;
            let b = outer.vertices[e.vertices[1]].point;
            (a[0] == b[0] && a[1] == b[1] && a[2] != b[2]).then_some(i)
        })
        .collect();
    assert_eq!(corners.len(), 4);
    for selected in [&corners[..2], &corners[..]] {
        let rounded = exact_convex_prism_fillet(&outer, selected, 4.)
            .unwrap()
            .model;
        let profile = crate::prism::recognize(&rounded).unwrap().unwrap();
        let removed = crate::planar_trim::boolean(
            &envelope.loops,
            &profile.loops,
            "difference",
            source.tolerance_mm,
        )
        .unwrap();
        for layer in &layers {
            assert!(
                crate::planar_trim::boolean(
                    &removed,
                    &layer.profile,
                    "difference",
                    source.tolerance_mm
                )
                .unwrap()
                .is_empty()
            );
        }
        let result = boolean(&restored, &rounded, "intersection").unwrap();
        audit_solid(&result).unwrap();
        let volume = crate::analysis::mass_properties(&result, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        let expected =
            7152. - selected.len() as f64 * (1. - std::f64::consts::PI / 4.) * 16. * 20.;
        assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
        let result_layers = crate::stepped_prism::recognize(&result).unwrap().unwrap();
        assert_eq!(result_layers.len(), 2);
        for (original, after) in layers.iter().zip(&result_layers) {
            assert_eq!(original.low, after.low);
            assert_eq!(original.high, after.high);
            assert!(
                crate::planar_trim::boolean(
                    &after.profile,
                    &original.profile,
                    "difference",
                    source.tolerance_mm
                )
                .unwrap()
                .is_empty()
            );
        }
    }
    assert_eq!(value_codec::to_string(&source).unwrap(), before);
}

#[test]
fn prism_cap_profile_follows_topology_through_a_concave_notch() {
    let expected = [
        [0., 0.],
        [10., 0.],
        [10., 2.],
        [2., 2.],
        [2., 8.],
        [10., 8.],
        [10., 10.],
        [0., 10.],
    ];
    let model = extrude_polygon(&expected, 0., 5.).unwrap();
    let vertices: Vec<_> = model.vertices.iter().map(|v| v.point).collect();
    let actual = ordered_prism_cap_profile(&model, &vertices, 0., 1e-6).unwrap();
    assert_eq!(actual.len(), expected.len());
    for (i, p) in actual.iter().enumerate() {
        let index = expected.iter().position(|q| q == p).unwrap();
        assert_eq!(
            actual[(i + 1) % actual.len()],
            expected[(index + 1) % expected.len()]
        );
    }
}

#[test]
fn exact_prism_fillet_supports_mixed_and_closed_profile_selections_under_rigid_placement() {
    let source = extrude_polygon(
        &[[-3., -2.], [4., -2.], [5., 1.], [2., 4.], [-2., 3.]],
        -1.,
        5.,
    )
    .unwrap();
    let placed = crate::transform::affine(
        &source,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 11.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let longitudinal: Vec<_> = placed
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, edge)| {
            let [a, b] = edge.vertices.map(|vertex| placed.vertices[vertex].point);
            ((a[0] - b[0]).abs() > 5.9
                && (a[1] - b[1]).abs() < 1e-9
                && (a[2] - b[2]).abs() < 1e-9)
                .then_some(i)
        })
        .collect();
    assert_eq!(longitudinal.len(), 5);
    let mixed =
        exact_convex_prism_fillet(&placed, &[longitudinal[0], longitudinal[2]], 0.25).unwrap();
    assert_eq!(
        mixed.feature.capability,
        EXACT_CONVEX_PRISM_FILLET_CAPABILITY
    );
    assert!(mixed.audit.ok && mixed.naming_complete);
    assert_eq!(
        mixed
            .model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
            .count(),
        2
    );
    let closed = exact_convex_prism_fillet(&placed, &longitudinal, 0.2).unwrap();
    assert_eq!(
        closed
            .model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
            .count(),
        5
    );
}

#[test]
fn exact_prism_fillet_has_independent_square_volume_and_radius_oracles() {
    let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let longitudinal: Vec<_> = source
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, edge)| {
            let [a, b] = edge.vertices.map(|vertex| source.vertices[vertex].point);
            ((a[0] - b[0]).abs() < 1e-9
                && (a[1] - b[1]).abs() < 1e-9
                && (a[2] - b[2]).abs() > 5.9)
                .then_some(i)
        })
        .collect();
    let radius = 0.5;
    let out = exact_convex_prism_fillet(&source, &longitudinal, radius)
        .unwrap()
        .model;
    let volume = crate::analysis::mass_properties(&out, 1e-9, 300_000)
        .unwrap()
        .signed_volume_mm3;
    let expected = (80. - 4. * radius * radius * (1. - std::f64::consts::PI / 4.)) * 6.;
    assert!((volume - expected).abs() < 2e-5, "{volume} != {expected}");
    for face in out
        .faces
        .iter()
        .filter(|face| face.surface.degree_u == 2 || face.surface.degree_v == 2)
    {
        let row = &face.surface.control_points;
        let endpoints = [&row[0][0], &row[row.len() - 1][0]];
        let chord = ((endpoints[0][0] - endpoints[1][0]).powi(2)
            + (endpoints[0][1] - endpoints[1][1]).powi(2))
        .sqrt();
        assert!((chord - radius * 2_f64.sqrt()).abs() < 1e-9);
    }
}

#[test]
#[cfg(feature = "codec")]
fn exact_convex_chamfer_handles_connected_chain_permutations_atomically() {
    let source = cuboid([0.; 3], [10.; 3]).unwrap();
    let connected = source.edges[0]
        .vertices
        .iter()
        .find_map(|vertex| {
            (1..source.edges.len()).find(|edge| source.edges[*edge].vertices.contains(vertex))
        })
        .unwrap();
    let before = value_codec::to_string(&source).unwrap();
    let a = exact_convex_chamfer(&source, &[0, connected], 0.75).unwrap();
    let b = exact_convex_chamfer(&source, &[connected, 0], 0.75).unwrap();
    assert_eq!(
        (
            a.model.vertices.len(),
            a.model.edges.len(),
            a.model.faces.len()
        ),
        (
            b.model.vertices.len(),
            b.model.edges.len(),
            b.model.faces.len()
        )
    );
    assert!(a.audit.ok && a.naming_complete);
    assert_eq!(before, value_codec::to_string(&source).unwrap());
    assert_eq!(
        exact_convex_chamfer(&source, &[0, 6], 0.75)
            .unwrap_err()
            .code,
        "BREP_EXACT_CHAMFER_REFUSED"
    );
    assert_eq!(before, value_codec::to_string(&source).unwrap());
}

#[test]
fn unsupported_transition_and_variable_radius_are_typed_refusals() {
    let source = cuboid([0.; 3], [10.; 3]).unwrap();
    let connected = source.edges[0]
        .vertices
        .iter()
        .find_map(|vertex| {
            (1..source.edges.len()).find(|edge| source.edges[*edge].vertices.contains(vertex))
        })
        .unwrap();
    assert_eq!(
        exact_convex_prism_fillet(&source, &[0, connected], 0.5)
            .unwrap_err()
            .code,
        "BREP_EXACT_FILLET_REFUSED"
    );
    // Constant-radius pair must not silently substitute.
    assert_eq!(
        exact_variable_radius_fillet(&source, &[0], &[[0.5, 0.5]])
            .unwrap_err()
            .code,
        "BREP_VARIABLE_RADIUS_FILLET_REFUSED"
    );
}

#[test]
fn variable_radius_frame_does_not_admit_non_cuboids_or_invalid_laws() {
    let bracket=extrude_polygon(&[[0.,0.],[10.,0.],[10.,3.],[3.,3.],[3.,8.],[0.,8.]],0.,6.).unwrap();
    let a=0.37_f64;
    let rotated=crate::transform::affine(&bracket,[
        [a.cos(),-a.sin(),0.,17.],[a.sin(),a.cos(),0.,-9.],
        [0.,0.,1.,23.],[0.,0.,0.,1.],
    ]).unwrap();
    // Some vertices have three orthogonal incident edges, but the entire
    // body is concave. Frame recovery must not authorize cuboid authorship.
    for edge in 0..rotated.edges.len() {
        assert_eq!(exact_variable_radius_fillet(&rotated,&[edge],&[[0.4,0.6]]).unwrap_err().code,"BREP_VARIABLE_RADIUS_FILLET_REFUSED");
    }
    let source=cuboid([0.;3],[10.,8.,6.]).unwrap();
    for edge in 0..source.edges.len() {
        for pair in [[0.,1.],[-1.,1.],[1.,1.],[f64::NAN,1.],[1.,f64::INFINITY],[1.,100.]] {
            assert_eq!(exact_variable_radius_fillet(&source,&[edge],&[pair]).unwrap_err().code,"BREP_VARIABLE_RADIUS_FILLET_REFUSED");
        }
    }
    for edges in [vec![],vec![0,1],vec![usize::MAX]] {
        assert_eq!(exact_variable_radius_fillet(&source,&edges,&[[0.4,0.6]]).unwrap_err().code,"BREP_VARIABLE_RADIUS_FILLET_REFUSED");
    }
}

#[test]
fn variable_radius_fillet_all_cuboid_edges_under_rigid_placement() {
    let source = cuboid([-7.,3.,-2.],[3.,11.,4.]).unwrap();
    let a=0.37_f64;let b=-0.61_f64;
    let matrix=[
        [a.cos()*b.cos(),-a.sin(),a.cos()*b.sin(),17.],
        [a.sin()*b.cos(),a.cos(),a.sin()*b.sin(),-9.],
        [-b.sin(),0.,b.cos(),23.],[0.,0.,0.,1.],
    ];
    let placed=crate::transform::affine(&source,matrix).unwrap();
    let before=value_codec::to_value(&placed).unwrap();
    for edge in 0..source.edges.len() {
        let [v0,v1]=source.edges[edge].vertices;
        let p=source.vertices[v0].point;let q=source.vertices[v1].point;
        let length=(p[0]-q[0]).hypot(p[1]-q[1]).hypot(p[2]-q[2]);
        for radii in [[0.5,1.5],[1.5,0.5]] {
            for model in [&source,&placed] {
                let out=exact_variable_radius_fillet(model,&[edge],&[radii]).unwrap();
                assert!(out.audit.ok && out.naming_complete);
                assert_eq!(out.model.validate().unwrap().boundary_edge_count,0);
                let volume=crate::analysis::mass_properties(&out.model,1e-9,300_000).unwrap().signed_volume_mm3;
                let expected=480.-(1.-std::f64::consts::PI/4.)*length
                    *(radii[0]*radii[0]+radii[0]*radii[1]+radii[1]*radii[1])/3.;
                assert!((volume-expected).abs()<2e-5,"edge {edge}, radii {radii:?}: {volume} != {expected}");
                // Volume is symmetric in the endpoint radii: additionally
                // measure each end of the rational conical patch to catch
                // accidental A/B reversal during frame recovery.
                let selected=&model.edges[edge];
                let origin=model.vertices[selected.vertices[0]].point;
                let end=model.vertices[selected.vertices[1]].point;
                let axis: [f64;3]=std::array::from_fn(|i|(end[i]-origin[i])/length);
                let patch=out.model.faces.iter().find(|f|f.surface.degree_u==2&&f.surface.degree_v==1).unwrap();
                for v in [0.,1.] {
                    let point=patch.surface.evaluate(0.5,v).unwrap().point;
                    let delta: [f64;3]=std::array::from_fn(|i|point[i]-origin[i]);
                    let along=(0..3).map(|i|delta[i]*axis[i]).sum::<f64>();
                    let expected_radius=if along.abs()<1e-7 {radii[0]} else {
                        assert!((along-length).abs()<1e-7);radii[1]
                    };
                    let perpendicular: [f64;3]=std::array::from_fn(|i|delta[i]-along*axis[i]);
                    let measured=perpendicular[0].hypot(perpendicular[1]).hypot(perpendicular[2])/(2_f64.sqrt()-1.);
                    assert!((measured-expected_radius).abs()<1e-7,"edge {edge}: endpoint radius {measured} != {expected_radius}");
                }
            }
        }
    }
    assert_eq!(value_codec::to_value(&placed).unwrap(),before);
    let sheared=crate::transform::affine(&placed,[[1.,0.2,0.,0.],[0.,1.,0.,0.],[0.,0.,1.,0.],[0.,0.,0.,1.]]).unwrap();
    for edge in 0..sheared.edges.len() {
        assert_eq!(exact_variable_radius_fillet(&sheared,&[edge],&[[0.5,1.5]]).unwrap_err().code,"BREP_VARIABLE_RADIUS_FILLET_REFUSED");
    }
}

#[test]
fn exact_variable_radius_fillet_authors_linear_law_on_vertical_cuboid_edge() {
    let source = cuboid([0.; 3], [10.; 3]).unwrap();
    let edge = source
        .edges
        .iter()
        .position(|edge| {
            let a = source.vertices[edge.vertices[0]].point;
            let b = source.vertices[edge.vertices[1]].point;
            (a[0] - b[0]).abs() <= 1e-12
                && (a[1] - b[1]).abs() <= 1e-12
                && (a[0] - 10.).abs() <= 1e-9
                && (a[1] - 10.).abs() <= 1e-9
        })
        .expect("vertical +X/+Y edge");
    let out = exact_variable_radius_fillet(&source, &[edge], &[[0.5, 1.5]]).unwrap();
    assert_eq!(
        out.feature.capability,
        EXACT_VARIABLE_RADIUS_FILLET_CAPABILITY
    );
    assert!(out.feature.complete);
    assert!(out.naming_complete);
    assert!(out.feature.notes.contains(&"exact_linear_radius_law"));
    assert!(
        out.feature
            .notes
            .contains(&"no_constant_radius_substitution")
    );
    out.model.validate().unwrap();
}

#[test]
fn exact_valence3_corner_blend_rotated_cuboid_and_shear_refusal() {
    let source = cuboid([-7., 3., -2.], [3., 11., 4.]).unwrap();
    let a = 0.37_f64;
    let b = -0.61_f64;
    let matrix = [
        [a.cos() * b.cos(), -a.sin(), a.cos() * b.sin(), 17.],
        [a.sin() * b.cos(), a.cos(), a.sin() * b.sin(), -9.],
        [-b.sin(), 0., b.cos(), 23.],
        [0., 0., 0., 1.],
    ];
    let placed = crate::transform::affine(&source, matrix).unwrap();
    for vertex in 0..8 {
        let edges: Vec<_> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.vertices.contains(&vertex).then_some(i))
            .collect();
        let out = exact_valence3_corner_blend(&placed, &edges, 1.).unwrap();
        assert!(out.audit.ok && out.naming_complete);
        let volume = crate::analysis::mass_properties(&out.model, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        let expected =
            480. - (1. - std::f64::consts::PI / 4.) * 21. - (1. - std::f64::consts::PI / 6.);
        assert!((volume - expected).abs() < 2e-5);
        let sheared = crate::transform::affine(
            &source,
            [
                [1., 0.2, 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        assert!(exact_valence3_corner_blend(&sheared, &edges, 1.).is_err());
    }
}

#[test]
fn exact_valence3_corner_blend_all_eight_corners_and_duplicate_refusal() {
    let min = [-7., 3., -2.];
    let max = [3., 11., 4.];
    let source = cuboid(min, max).unwrap();
    let before = format!("{source:?}");
    let mut reference_volume: Option<f64> = None;
    for mask in 0..8 {
        let corner =
            std::array::from_fn::<_, 3, _>(
                |i| if mask & (1 << i) == 0 { min[i] } else { max[i] },
            );
        let vertex = source
            .vertices
            .iter()
            .position(|v| v.point == corner)
            .unwrap();
        let edges: Vec<_> = source
            .edges
            .iter()
            .enumerate()
            .filter_map(|(i, e)| e.vertices.contains(&vertex).then_some(i))
            .collect();
        assert_eq!(edges.len(), 3);
        let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
        assert!(
            out.audit.ok && out.naming_complete && out.feature.complete,
            "corner {mask}"
        );
        let volume = crate::analysis::mass_properties(&out.model, 1e-9, 300_000)
            .unwrap()
            .signed_volume_mm3;
        let expected = 480. - (1. - std::f64::consts::PI / 4.) * 21.
            - (1. - std::f64::consts::PI / 6.);
        assert!((volume - expected).abs() < 2e-5, "corner {mask}: {volume} != {expected}");
        if let Some(reference) = reference_volume {
            assert!(
                (volume - reference).abs() < 2e-5,
                "corner {mask}: {volume} != {reference}"
            );
        } else {
            reference_volume = Some(volume);
        }
        let (actual_min, actual_max) = model_bounds(&out.model);
        for i in 0..3 {
            assert!((actual_min[i] - min[i]).abs() < 1e-9);
            assert!((actual_max[i] - max[i]).abs() < 1e-9);
        }
        assert!(
            exact_valence3_corner_blend(&source, &[edges[0], edges[0], edges[1]], 1.).is_err()
        );
        assert!(exact_valence3_corner_blend(&source, &edges, 3.).is_err());
    }
    assert_eq!(format!("{source:?}"), before);
}

#[test]
fn exact_valence3_corner_blend_authors_sphere_and_three_cylinders() {
    let source = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let (min, max) = model_bounds(&source);
    let edges: Vec<usize> = source
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let a = source.vertices[edge.vertices[0]].point;
            let b = source.vertices[edge.vertices[1]].point;
            let mid = [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ];
            let on_max_x = (mid[0] - max[0]).abs() <= 1e-9;
            let on_max_y = (mid[1] - max[1]).abs() <= 1e-9;
            let on_max_z = (mid[2] - max[2]).abs() <= 1e-9;
            let touches_corner = edge.vertices.iter().any(|&v| {
                let p = source.vertices[v].point;
                (0..3).all(|i| (p[i] - max[i]).abs() <= 1e-9)
            });
            let axis_edge =
                (on_max_x && on_max_y) || (on_max_x && on_max_z) || (on_max_y && on_max_z);
            (touches_corner && axis_edge).then_some(index)
        })
        .collect();
    assert_eq!(
        edges.len(),
        3,
        "expected three max-corner edges, got {edges:?}"
    );
    let _ = min;
    let out = exact_valence3_corner_blend(&source, &edges, 1.).unwrap();
    assert_eq!(
        out.feature.capability,
        EXACT_VALENCE3_CORNER_BLEND_CAPABILITY
    );
    assert!(out.feature.complete && out.naming_complete && out.audit.ok);
    assert!(
        out.feature
            .notes
            .contains(&"exact_equal_radius_sphere_octant")
    );
    let spheres = out
        .model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 2)
        .count();
    let cylinders = out
        .model
        .faces
        .iter()
        .filter(|f| f.surface.degree_u == 2 && f.surface.degree_v == 1)
        .count();
    assert_eq!((spheres, cylinders), (1, 3));
    out.model.validate().unwrap();
}

#[test]
fn cylinder_shell_offset() {
    let model = cylinder(4., 6.).unwrap();
    let (out, cert) = analytic_shell(&model, 0.5).unwrap();
    assert!(cert.notes.contains(&"as01_cylinder_wall_offset"));
    out.validate().unwrap();
}

#[test]
fn solid_loft_section_match() {
    let a = cuboid([0., 0., 0.], [2., 2., 1.]).unwrap();
    let b = cuboid([0., 0., 5.], [2., 2., 6.]).unwrap();
    let (out, cert) = analytic_solid_loft(&[a, b]).unwrap();
    assert!(cert.complete);
    assert!(!out.faces.is_empty());
}

#[test]
fn step_roundtrip_not_faceted() {
    let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let (text, cert) = crate::export_step(&model).unwrap();
    assert!(cert.complete);
    assert!(text.contains("ADVANCED_FACE"));
    assert!(text.contains("PLANE"));
    assert!(text.contains("VERTEX_POINT"));
    assert!(text.contains("EDGE_CURVE"));
    assert!(text.contains("AP242"));
    assert!(!text.contains("FACETED_BREP"));
    let (back, _) = crate::import_step(&text).unwrap();
    back.validate().unwrap();
}

#[test]
fn analytic_chamfer_af01_cuboid_edge() {
    let model = cuboid([0., 0., 0.], [10., 10., 10.]).unwrap();
    let edge = model
        .edges
        .iter()
        .position(|e| {
            let a = model.vertices[e.vertices[0]].point;
            let b = model.vertices[e.vertices[1]].point;
            (a[0] - b[0]).abs() <= 1e-12
                && (a[1] - b[1]).abs() <= 1e-12
                && (a[0] - 10.).abs() <= 1e-9
                && (a[1] - 10.).abs() <= 1e-9
        })
        .expect("vertical +X/+Y edge");
    let (out, cert) = analytic_chamfer(&model, edge, 1.).unwrap();
    assert!(cert.complete);
    assert!(cert.notes.contains(&"not_mesh_bevel"));
    out.validate().unwrap();
}

#[test]
fn analytic_chamfer_honors_each_selected_vertical_corner() {
    let model = cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    for [x, y] in [[0., 0.], [10., 0.], [10., 8.], [0., 8.]] {
        let edge = model
            .edges
            .iter()
            .position(|edge| {
                let a = model.vertices[edge.vertices[0]].point;
                let b = model.vertices[edge.vertices[1]].point;
                (a[0] - b[0]).abs() <= 1e-12
                    && (a[1] - b[1]).abs() <= 1e-12
                    && (a[0] - x).abs() <= 1e-9
                    && (a[1] - y).abs() <= 1e-9
            })
            .unwrap();
        let (out, _) = analytic_chamfer(&model, edge, 1.).unwrap();
        out.validate().unwrap();
        assert!(!out.edges.iter().any(|edge| {
            let a = out.vertices[edge.vertices[0]].point;
            let b = out.vertices[edge.vertices[1]].point;
            (a[0] - b[0]).abs() <= 1e-9
                && (a[1] - b[1]).abs() <= 1e-9
                && (a[0] - x).abs() <= 1e-9
                && (a[1] - y).abs() <= 1e-9
        }));
    }
}

#[test]
fn mesh_bevel_cannot_be_claimed_via_curved_chamfer() {
    let model = cylinder(2., 4.).unwrap();
    assert_eq!(
        analytic_chamfer(&model, 0, 0.5).unwrap_err().code,
        "BREP_ANALYTIC_CHAMFER_REFUSED"
    );
}

#[test]
fn frame_law_production_sweep() {
    let (out, cert) = frame_law_ruled_sweep(
        &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
        &[[0., 0., 0.], [0., 0., 2.], [0., 0., 4.]],
        "rotation-minimizing",
    )
    .unwrap();
    assert!(cert.complete);
    assert!(
        cert.notes
            .contains(&"frame_law_parallel_translation_walking_slice")
    );
    out.validate().unwrap();
}

#[test]
fn frame_law_refuses_bent_nonparallel_path() {
    assert_eq!(
        frame_law_ruled_sweep(
            &[[0., 0.], [1., 0.], [1., 1.], [0., 1.]],
            &[[0., 0., 0.], [0., 0., 2.], [0., 1., 4.]],
            "frenet",
        )
        .unwrap_err()
        .code,
        "BREP_FRAME_LAW_REFUSED"
    );
}

#[test]
fn successor_parallel_sweep_certifies_and_bent_rmf_refuses() {
    let profile = [[0., 0.], [2., 0.], [2., 1.], [0., 1.]];
    let certified =
        audited_parallel_frame_sweep(&profile, &[[3., -1., 0.], [3., -1., 4.]], "rmf").unwrap();
    assert_eq!(
        certified.feature.capability,
        EXACT_PARALLEL_FRAME_SWEEP_CAPABILITY
    );
    assert!(certified.audit.ok && certified.naming_complete);
    assert_eq!(certified.context, certified.evidence.context);
    assert_eq!(
        audited_parallel_frame_sweep(
            &profile,
            &[[0., 0., 0.], [0., 0., 2.], [0., 1., 4.]],
            "rmf",
        )
        .unwrap_err()
        .code,
        "BREP_FRAME_LAW_REFUSED"
    );
}

#[test]
fn exact_multi_section_loft_has_correspondence_topology_and_audit_oracles() {
    let sections = vec![
        vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
        vec![
            [-1.5, -1., 2.],
            [1.5, -1., 2.],
            [1.5, 1., 2.],
            [-1.5, 1., 2.],
        ],
        vec![
            [-1., -0.75, 5.],
            [1., -0.75, 5.],
            [1., 0.75, 5.],
            [-1., 0.75, 5.],
        ],
    ];
    let result = audited_multi_section_loft(&sections).unwrap();
    assert_eq!(
        result.feature.capability,
        EXACT_MULTI_SECTION_LOFT_CAPABILITY
    );
    assert!(result.audit.ok && result.naming_complete);
    assert_eq!(
        (
            result.model.vertices.len(),
            result.model.edges.len(),
            result.model.faces.len()
        ),
        (12, 20, 10)
    );
    assert_eq!(
        result
            .model
            .faces
            .iter()
            .filter(|face| face.surface.degree_u == 1 && face.surface.degree_v == 1)
            .count(),
        10
    );
    assert!(result.audit.self_intersection_pairs_candidate > 0);
    assert_eq!(result.audit.self_intersection_pairs_checked, 0);
    assert!(!result.audit.self_intersection_complete);
    // Independent Simpson integration of the quadratic section-area law.
    let expected_volume = (4. + 4. * 5. + 6.) * 2. / 6. + (6. + 4. * 4.375 + 3.) * 3. / 6.;
    let mass = crate::analysis::mass_properties(&result.model, 1e-6, 10_000).unwrap();
    assert!((mass.signed_volume_mm3.abs() - expected_volume).abs() < 1e-5);
    let prism = audited_multi_section_loft(&[
        vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
        vec![[-1., -1., 2.], [1., -1., 2.], [1., 1., 2.], [-1., 1., 2.]],
        vec![[-1., -1., 5.], [1., -1., 5.], [1., 1., 5.], [-1., 1., 5.]],
    ])
    .unwrap();
    let prism_mass = crate::analysis::mass_properties(&prism.model, 1e-6, 10_000).unwrap();
    assert!((prism_mass.signed_volume_mm3.abs() - 20.).abs() < 1e-6);
    assert!((prism_mass.surface_area_mm2 - 48.).abs() < 1e-6);
}

#[test]
fn exact_bent_rmf_sweep_certifies_path_frame_laws_and_reversal() {
    let profile = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
    let path = [[0., 0., 0.], [0., 0., 3.], [0., 1., 6.], [0., 3., 9.]];
    let twist = [0., 0.1, 0.2, 0.3];
    let scale = [1., 1.1, 1.2, 1.25];
    let result = audited_bent_rmf_sweep(&profile, &path, &twist, &scale).unwrap();
    assert_eq!(result.feature.capability, EXACT_BENT_RMF_SWEEP_CAPABILITY);
    assert!(result.audit.ok && result.naming_complete);
    assert_eq!(
        (
            result.model.vertices.len(),
            result.model.edges.len(),
            result.model.faces.len()
        ),
        (16, 28, 14)
    );
    let independent_path_length = path
        .windows(2)
        .map(|span| {
            let delta = sub3(span[1], span[0]);
            dot3(delta, delta).sqrt()
        })
        .sum::<f64>();
    assert!((independent_path_length - (3. + 10_f64.sqrt() + 13_f64.sqrt())).abs() < 1e-12);
    for (station, vertices) in path
        .iter()
        .zip(result.model.vertices.chunks_exact(profile.len()))
    {
        let centroid = (0..3)
            .map(|axis| {
                vertices
                    .iter()
                    .map(|vertex| vertex.point[axis])
                    .sum::<f64>()
                    / vertices.len() as f64
            })
            .collect::<Vec<_>>();
        assert!((0..3).all(|axis| (centroid[axis] - station[axis]).abs() < 1e-9));
        let section_u = sub3(vertices[1].point, vertices[0].point);
        let section_v = sub3(vertices[3].point, vertices[0].point);
        assert!(dot3(cross3(section_u, section_v), cross3(section_u, section_v)) > 1e-8);
    }
    // Four shared section edges per interior station prove exact C0
    // adjacency rather than duplicated, tolerance-sewn span boundaries.
    assert_eq!(
        result.model.edges.len(),
        path.len() * profile.len() + (path.len() - 1) * profile.len()
    );
    let mut reverse_path = path;
    reverse_path.reverse();
    let mut reverse_twist = twist;
    reverse_twist.reverse();
    let mut reverse_scale = scale;
    reverse_scale.reverse();
    let reversed =
        audited_bent_rmf_sweep(&profile, &reverse_path, &reverse_twist, &reverse_scale)
            .unwrap();
    assert_eq!(
        (
            reversed.model.vertices.len(),
            reversed.model.edges.len(),
            reversed.model.faces.len()
        ),
        (16, 28, 14)
    );
}

#[test]
fn loft_sweep_mutations_refuse_atomically_with_exact_codes() {
    let profile = [[-0.2, -0.2], [0.2, -0.2], [0.2, 0.2], [-0.2, 0.2]];
    let path = [[0., 0., 0.], [0., 0., 3.], [0., 1., 6.]];
    let before = profile;
    assert_eq!(
        audited_bent_rmf_sweep(&profile, &path, &[0.; 3], &[1., 0., 1.])
            .unwrap_err()
            .code,
        "BREP_SWEEP_SCALE_REFUSED"
    );
    assert_eq!(profile, before);
    assert_eq!(
        audited_bent_rmf_sweep(
            &profile,
            &[[0., 0., 0.], [0., 0., 3.], [0., 0., 0.]],
            &[0.; 3],
            &[1.; 3],
        )
        .unwrap_err()
        .code,
        "BREP_SWEEP_CLOSED_LOOP_REFUSED"
    );
    assert_eq!(
        audited_bent_rmf_sweep(&profile, &path, &[0., 2., 0.], &[1.; 3])
            .unwrap_err()
            .code,
        "BREP_SWEEP_TWIST_REFUSED"
    );
    assert_eq!(
        audited_bent_rmf_sweep(
            &profile,
            &[[0., 0., 0.], [0., 0., 3.], [0., 0., 3.]],
            &[0.; 3],
            &[1.; 3],
        )
        .unwrap_err()
        .code,
        "BREP_SWEEP_CUSP_REFUSED"
    );
    let mut mismatched = vec![
        vec![[-1., -1., 0.], [1., -1., 0.], [1., 1., 0.], [-1., 1., 0.]],
        vec![[-1., -1., 2.], [1., -1., 2.], [1., 1., 2.], [-1., 1., 2.]],
        vec![[-1., -1., 4.], [1., -1., 4.], [0., 1., 4.]],
    ];
    let snapshot = mismatched.clone();
    assert_eq!(
        audited_multi_section_loft(&mismatched).unwrap_err().code,
        "BREP_LOFT_CORRESPONDENCE_REFUSED"
    );
    assert_eq!(mismatched, snapshot);
    mismatched[2] = vec![[-1., -1., 4.], [1., 1., 4.], [1., -1., 4.], [-1., 1., 4.]];
    assert_eq!(
        audited_multi_section_loft(&mismatched).unwrap_err().code,
        "BREP_LOFT_SECTION_TOPOLOGY_REFUSED"
    );
}

#[test]
fn frame_law_refuses_unknown_law() {
    assert_eq!(
        frame_law_ruled_sweep(
            &[[0., 0.], [1., 0.], [1., 1.]],
            &[[0., 0., 0.], [0., 0., 1.]],
            "mesh"
        )
        .unwrap_err()
        .code,
        "BREP_FRAME_LAW_REFUSED"
    );
}

#[test]
fn cylinder_shell_preserves_rigid_placement() {
    let base = cylinder(4., 6.).unwrap();
    let placed = crate::transform::affine(
        &base,
        [
            [0., 0., 1., 7.],
            [1., 0., 0., -3.],
            [0., 1., 0., 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (shelled, _) = analytic_shell(&placed, 0.5).unwrap();
    let (a_min, a_max) = model_bounds(&placed);
    let (b_min, b_max) = model_bounds(&shelled);
    for axis in 0..3 {
        assert!((a_min[axis] - b_min[axis]).abs() < 1e-8);
        assert!((a_max[axis] - b_max[axis]).abs() < 1e-8);
    }
}

#[test]
fn solid_loft_refuses_nonrectangular_section_carriers() {
    let triangle = extrude_polygon(&[[0., 0.], [2., 0.], [0., 2.]], 0., 1.).unwrap();
    let box_section = cuboid([0., 0., 4.], [2., 2., 5.]).unwrap();
    assert_eq!(
        analytic_solid_loft(&[triangle, box_section])
            .unwrap_err()
            .code,
        "BREP_ANALYTIC_LOFT_REFUSED"
    );
}

#[test]
fn iges_roundtrip_entity_subset() {
    let model = cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let (text, cert) = export_iges(&model).unwrap();
    assert!(cert.complete);
    assert!(text.contains("116,") || text.contains("110,") || text.contains("190,"));
    assert!(text.contains("186,") && text.contains("514,"));
    let (back, _) = import_iges(&text).unwrap();
    back.validate().unwrap();
}

#[test]
fn iges_refuses_stl_payload() {
    let stl = "solid cube\nfacet normal 0 0 1\nouter loop\nvertex 0 0 0\nendloop\nendfacet\nendsolid cube\n";
    assert_eq!(import_iges(stl).unwrap_err().code, "BREP_IGES_REFUSED");
}
