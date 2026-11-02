use brep_core::{
    ChangeKind, Model,
    feature_ops::edge_partition::{self, Limits},
};
fn cube() -> Model {
    brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap()
}
fn volume(m: &Model) -> f64 {
    brep_core::analysis::mass_properties(m, 1e-6, 2000000)
        .unwrap()
        .signed_volume_mm3
}
#[test]
fn shared_linear_edge_splits_both_uses_and_keeps_face_body_and_local_tolerance_ownership() {
    let mut s = cube();
    let edge = 0;
    let parent = s.1.edges[edge];
    s.1.local_tolerances_mm.insert(parent, 1e-7);
    s.1.sewed_edges.insert(parent);
    let before = s.clone();
    let p = edge_partition::split(&s, edge, &[0.25, 0.75], "linear", Limits::default()).unwrap();
    assert_eq!(p.model().vertices.len(), s.vertices.len() + 2);
    assert_eq!(p.model().edges.len(), s.edges.len() + 2);
    assert_eq!(p.model().faces, s.faces);
    assert_eq!(p.model().1.faces, s.1.faces);
    assert_eq!(p.model().1.bodies, s.1.bodies);
    assert!((volume(p.model()) - 24.).abs() < 1e-6);
    for &e in p.edges() {
        assert_eq!(
            p.model()
                .loops
                .iter()
                .flat_map(|w| &w.coedges)
                .filter(|c| c.edge == e)
                .count(),
            2
        );
        assert_eq!(
            p.model().1.local_tolerances_mm.get(&p.model().1.edges[e]),
            Some(&1e-7)
        );
        assert!(!p.model().1.sewed_edges.contains(&p.model().1.edges[e]));
    }
    assert!(!p.model().1.edges.contains(&parent));
    assert!(
        p.model()
            .1
            .change_set
            .changes
            .iter()
            .any(|c| c.kind == ChangeKind::Split && c.parents == [parent] && c.children.len() == 3)
    );
    assert!(p.reverify().unwrap().boundary_complete);
    assert_eq!(s, before);
}
#[test]
fn rational_circle_segments_keep_shared_candidate_endpoints_and_original_geometry() {
    let s = brep_core::cylinder(2., 3.).unwrap();
    let edge = s
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2 && e.curve.weights.iter().any(|w| *w != e.curve.weights[0])
        })
        .unwrap();
    let d = s.edges[edge].curve.domain();
    let cuts = [d[0] + 0.2 * (d[1] - d[0]), d[0] + 0.6 * (d[1] - d[0])];
    let p = edge_partition::split(&s, edge, &cuts, "circle", Limits::default()).unwrap();
    assert_eq!(p.edges().len(), 3);
    for pair in p.edges().windows(2) {
        assert_eq!(
            p.model().edges[pair[0]].curve.control_points.last(),
            Some(&p.model().edges[pair[1]].curve.control_points[0])
        );
    }
    assert!((volume(p.model()) - 12. * std::f64::consts::PI).abs() < 1e-5);
    assert!(p.reverify().unwrap().trim_regions_valid);
    let child = p.edges()[1];
    let d = p.model().edges[child].curve.domain();
    let second = edge_partition::split(
        p.model(),
        child,
        &[(d[0] + d[1]) / 2.],
        "nested",
        Limits::default(),
    )
    .unwrap();
    assert!((volume(second.model()) - volume(&s)).abs() < 1e-5);
    second.model().1.change_set.validate().unwrap();
}
#[test]
fn independent_nonunit_pcurve_clocks_and_full_descending_metadata_follow_local_pieces() {
    let mut s = cube();
    let edge = 0;
    for e in &mut s.edges {
        for k in &mut e.curve.knots {
            *k = -3. + 10. * *k;
        }
    }
    for w in &mut s.loops {
        for c in &mut w.coedges {
            for k in &mut c.pcurve.knots {
                *k = -11. + 40. * *k;
            }
            if c.edge == edge {
                let mut curve = [-3., 7.];
                let mut pcurve = [-11., 29.];
                if c.reversed {
                    curve.reverse();
                    pcurve.reverse();
                }
                *c = c
                    .clone()
                    .with_trim(brep_topology::CoedgeTrim {
                        curve_parameter: curve,
                        pcurve_parameter: pcurve,
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    s.validate().unwrap();
    let before = s.clone();
    let p = edge_partition::split(&s, edge, &[-1., 4.], "clock", Limits::default()).unwrap();
    assert_eq!(p.model().edges[p.edges()[0]].curve.domain(), [-3., -1.]);
    assert_eq!(p.model().edges[p.edges()[2]].curve.domain(), [4., 7.]);
    for c in p
        .model()
        .loops
        .iter()
        .flat_map(|w| &w.coedges)
        .filter(|c| p.edges().contains(&c.edge))
    {
        assert!(c.trim().is_some());
    }
    assert!(p.reverify().unwrap().boundary_complete);
    assert_eq!(s, before);
}
#[test]
fn split_names_follow_operation_and_parent_identity_across_face_storage_and_cut_edits() {
    let s = cube();
    let a = edge_partition::split(&s, 0, &[0.25], "stable", Limits::default()).unwrap();
    let b = edge_partition::split(&s, 0, &[0.4], "stable", Limits::default()).unwrap();
    assert_eq!(
        a.edges()
            .iter()
            .map(|e| a.model().1.edges[*e])
            .collect::<Vec<_>>(),
        b.edges()
            .iter()
            .map(|e| b.model().1.edges[*e])
            .collect::<Vec<_>>()
    );
    let mut reordered = s.clone();
    let n = reordered.faces.len();
    reordered.faces.reverse();
    reordered.1.faces.reverse();
    for shell in &mut reordered.shells {
        for u in &mut shell.faces {
            u.face = n - 1 - u.face;
        }
    }
    let c = edge_partition::split(&reordered, 0, &[0.25], "stable", Limits::default()).unwrap();
    assert_eq!(a.model().1.edges, c.model().1.edges);
    assert_eq!(a.model().1.vertices, c.model().1.vertices);
}
#[test]
fn invalid_cuts_and_exhausted_work_cannot_publish_partial_edits() {
    let s = cube();
    let before = s.clone();
    for cuts in [
        vec![],
        vec![0.],
        vec![1.],
        vec![0.5, 0.5],
        vec![0.75, 0.25],
        vec![f64::NAN],
    ] {
        assert!(edge_partition::split(&s, 0, &cuts, "invalid", Limits::default()).is_err());
    }
    assert!(edge_partition::split(&s, 999, &[0.5], "invalid", Limits::default()).is_err());
    assert!(edge_partition::split(&s, 0, &[0.5], "", Limits::default()).is_err());
    assert!(
        edge_partition::split(
            &s,
            0,
            &[0.5],
            "scarce",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    let mut partial = s.clone();
    for w in &mut partial.loops {
        for c in &mut w.coedges {
            if c.edge == 0 {
                *c = c
                    .clone()
                    .with_trim(brep_topology::CoedgeTrim {
                        curve_parameter: [0.25, 0.75],
                        pcurve_parameter: [0.25, 0.75],
                        periodic_lift: [[0; 2]; 2],
                    })
                    .unwrap();
            }
        }
    }
    let refined = edge_partition::split(&partial, 0, &[0.5], "partial", Limits::default()).unwrap();
    assert_eq!(refined.relations().len(), 2);
    assert!(refined.reverify().unwrap().boundary_complete);
    assert_eq!(s, before);
}
#[test]
fn inner_cavity_edge_refinement_keeps_shell_roles_and_an_explicit_uv_allowance() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let inner = brep_core::cuboid([1.; 3], [3.; 3]).unwrap();
    let s = brep_core::operations::boolean(&outer, &inner, "difference").unwrap();
    let shell = s.bodies[0].inner_shells[0];
    let face = s.shells[shell].faces[0].face;
    let edge = s.loops[s.faces[face].outer].coedges[0].edge;
    let d = s.edges[edge].curve.domain();
    let p = edge_partition::split_with_uv_tolerance(
        &s,
        edge,
        &[d[0] + 0.3 * (d[1] - d[0])],
        "inner-shell",
        1e-12,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(p.model().bodies, s.bodies);
    assert_eq!(p.model().shells, s.shells);
    assert!((volume(p.model()) - 56.).abs() < 1e-6);
    assert!(p.reverify().unwrap().boundary_complete);
    let f = p.model().shells[p.model().bodies[0].outer_shell].faces[0].face;
    let e = p.model().loops[p.model().faces[f].outer].coedges[0].edge;
    let d = p.model().edges[e].curve.domain();
    let outer_split = edge_partition::split(
        p.model(),
        e,
        &[(d[0] + d[1]) / 2.],
        "outer-shell",
        Limits::default(),
    )
    .unwrap();
    assert!((volume(outer_split.model()) - 56.).abs() < 1e-6);
    assert!(outer_split.reverify().unwrap().boundary_complete);
    for tolerance in [0., -1., f64::NAN] {
        assert!(
            edge_partition::split_with_uv_tolerance(
                &s,
                edge,
                &[(d[0] + d[1]) / 2.],
                "invalid-uv",
                tolerance,
                Limits::default()
            )
            .is_err()
        );
    }
}

#[test]
fn concave_outer_bounds_cannot_authorize_a_cavity_in_missing_material() {
    let outer = brep_core::cuboid([0.; 3], [4.; 3]).unwrap();
    let notch = brep_core::cuboid([2.; 3], [5.; 3]).unwrap();
    let concave = brep_core::operations::boolean(&outer, &notch, "difference").unwrap();
    let inner = brep_core::cuboid([0.5; 3], [1.5; 3]).unwrap();
    let mut invalid = brep_core::operations::boolean(&concave, &inner, "difference").unwrap();
    let shell = invalid.bodies[0].inner_shells[0];
    let faces: Vec<_> = invalid.shells[shell].faces.iter().map(|u| u.face).collect();
    let mut edges = std::collections::BTreeSet::new();
    for f in faces {
        for p in invalid.faces[f].surface.control_points.iter_mut().flatten() {
            for x in p.iter_mut() {
                *x += 2.;
            }
        }
        for c in &invalid.loops[invalid.faces[f].outer].coedges {
            edges.insert(c.edge);
        }
    }
    let mut vertices = std::collections::BTreeSet::new();
    for &e in &edges {
        vertices.insert(invalid.edges[e].vertices[0]);
        vertices.insert(invalid.edges[e].vertices[1]);
        for p in &mut invalid.edges[e].curve.control_points {
            for x in p.iter_mut() {
                *x += 2.;
            }
        }
    }
    for v in vertices {
        for x in &mut invalid.vertices[v].point {
            *x += 2.;
        }
    }
    invalid.validate().unwrap();
    assert!(brep_core::solid_audit::audit_solid(&invalid).is_err());
    let before = invalid.clone();
    let e = *edges.first().unwrap();
    let d = invalid.edges[e].curve.domain();
    assert!(
        edge_partition::split(
            &invalid,
            e,
            &[(d[0] + d[1]) / 2.],
            "outside-material",
            Limits::default()
        )
        .is_err()
    );
    assert_eq!(invalid, before);
}
