use nurbs_core::thread::{self, Spec};

fn qualification_limits() -> brep_core::volume_validity::Limits {
    brep_core::volume_validity::Limits {
        boundary: brep_core::boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 1000,
            trim_cells: 10000,
            trim_domain_cells: 100000,
            spans: 100,
            contacts: brep_core::face_contacts::Limits {
                pairs: 1000,
                cells: 10000,
                domain_cells: 100000,
                cells_per_pair: 16,
                domain_cells_per_pair: 1000,
            },
        },
        nesting_pairs: 100,
        nesting_cells: 10000,
        nesting_domain_cells: 100000,
        orientation_cells: 10000,
        orientation_domain_cells: 100000,
        orientation_spans: 100,
    }
}

#[test]
fn closed_candidate_corrects_inward_orientation_without_claiming_solid_validity() {
    let mut source = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    source.bodies.clear();
    source.shells[0].closed = false;
    for u in &mut source.shells[0].faces {
        u.reversed = !u.reversed;
    }
    source.rebuild_topology_ids();
    let before = source.clone();
    let result = brep_core::thread::closed_body_candidate(&source, 1e-6, 200000).unwrap();
    assert!(result.orientation_reversed);
    assert!((result.mass.signed_volume_mm3 - 1.).abs() < 1e-9);
    assert!(!result.solid_geometry_certified);
    let qualified =
        brep_core::thread::qualify_closed_candidate(&result, 1e-8, 100000, qualification_limits())
            .unwrap();
    assert!(qualified.continuous_agreement.complete);
    assert!(qualified.solid_geometry_certified);
    assert_eq!(source, before);
    assert!(brep_core::thread::closed_body_candidate(&source, 1e-6, 0).is_err());
}

#[test]
fn interior_clipped_network_connects_across_turn_and_profile_wrap() {
    let spec = Spec {
        turns: 2,
        ..Spec::default()
    };
    let limits = [1.13, 1.87];
    let source = thread::trim_candidates(spec, limits, 1e-3).unwrap();
    let network =
        brep_core::thread::clipped_thread_network(spec, limits, 1e-12, 1e-3, 128, 1e-8, 4_000_000)
            .unwrap();
    assert_eq!(network.model.faces.len(), source.candidates.len());
    assert!(network.model.validate().unwrap().topology_valid);
    assert!(network.verification_cells <= 4_000_000);
    assert!(!network.model.shells[0].closed);
    assert!(network.model.bodies.is_empty());
    let before = network.model.clone();
    let (lower, lower_cells) =
        brep_core::thread::cap_patch_network_plane(&network.model, limits[0], 100000).unwrap();
    let (closed, upper_cells) =
        brep_core::thread::cap_patch_network_plane(&lower, limits[1], 100000).unwrap();
    assert_eq!(closed.faces.len(), network.model.faces.len() + 2);
    assert_eq!(closed.validate().unwrap().boundary_edge_count, 0);
    assert!(lower_cells <= 100000 && upper_cells <= 100000);
    assert!(closed.bodies.is_empty());
    assert_eq!(network.model, before);
    let candidate = brep_core::thread::closed_body_candidate(&closed, 1e-4, 2_000_000).unwrap();
    assert!(candidate.model.shells[0].closed);
    assert_eq!(candidate.model.bodies.len(), 1);
    assert!(!candidate.solid_geometry_certified);
    assert!(candidate.mass.signed_volume_mm3 > candidate.mass.volume_error_estimate_mm3);
    assert!(candidate.evaluations <= 2_000_000);
    let profile = thread::patches(spec).unwrap();
    let c = profile.crest_radius;
    let r = profile.root_radius;
    let ideal_volume = std::f64::consts::PI
        * (0.125 * c * c + 0.25 * r * r + 0.625 * (c * c + c * r + r * r) / 3.)
        * (limits[1] - limits[0]);
    assert!(
        (candidate.mass.signed_volume_mm3 - ideal_volume).abs() < ideal_volume * 0.005,
        "{} versus {ideal_volume}",
        candidate.mass.signed_volume_mm3
    );
    let qualified = brep_core::thread::qualify_closed_candidate(
        &candidate,
        1e-8,
        1_000_000,
        qualification_limits(),
    )
    .unwrap();
    assert!(qualified.continuous_agreement.complete);
    assert!(!qualified.solid_geometry_certified);
    assert!(!qualified.volume.boundary.agreement.all_equal);
}

#[test]
fn automatic_clipped_network_retains_every_candidate() {
    let spec = Spec::default();
    let limits = [0.1, 0.4];
    let source = thread::trim_candidates(spec, limits, 1e-3).unwrap();
    let network =
        brep_core::thread::clipped_thread_network(spec, limits, 1e-12, 1e-3, 128, 1e-8, 2_000_000)
            .unwrap();
    assert_eq!(network.model.faces.len(), source.candidates.len());
    assert!(network.model.validate().unwrap().topology_valid);
    assert!(network.verification_cells <= 2_000_000);
    assert!(network.max_endpoint_adjustment_upper_bound <= 1e-8);
    assert!(network.model.bodies.is_empty());
    for candidate in &source.candidates {
        assert!(
            network
                .model
                .faces
                .iter()
                .any(|f| f.surface == candidate.patch.surface)
        );
    }
    assert!(
        brep_core::thread::clipped_thread_network(spec, limits, 1e-12, 1e-3, 128, 1e-8, 0).is_err()
    );
}

#[test]
fn clipped_thread_network_preserves_planar_end_boundaries() {
    use brep_core::thread as author;
    let limits = [0.1, 0.4];
    let trims = thread::trim_candidates(Spec::default(), limits, 1e-10).unwrap();
    let candidate = |q, s| {
        trims
            .candidates
            .iter()
            .find(|c| c.patch.quarter == q && c.patch.profile_segment == s)
            .unwrap()
    };
    let sheet = |q, s| {
        author::trimmed_patch_sheet_with_planar_ends(
            candidate(q, s),
            &limits,
            1e-12,
            1e-3,
            128,
            1e-8,
            100000,
        )
        .unwrap()
        .model
    };
    let boundary = |model: &brep_core::Model, wire: usize, axis: usize, value: f64| {
        model.loops[wire]
            .coedges
            .iter()
            .position(|c| c.pcurve.control_points.iter().all(|p| p[axis] == value))
            .unwrap()
    };
    let a = sheet(0, 0);
    let b = sheet(0, 1);
    let si = boundary(&a, 0, 0, 1.);
    let ti = boundary(&b, 0, 0, 0.);
    let b = author::align_patch_endpoints(
        &b,
        ti,
        &a.edges[a.loops[0].coedges[si].edge].curve,
        1e-8,
        100000,
    )
    .unwrap()
    .model;
    let ab = author::join_patch_sheets(&a, &b, si, ti, 100000).unwrap().0;
    let c = sheet(1, 0);
    let si = boundary(&ab, 0, 1, 1.);
    let ti = boundary(&c, 0, 1, 0.);
    let c = author::align_patch_endpoints(
        &c,
        ti,
        &ab.edges[ab.loops[0].coedges[si].edge].curve,
        1e-8,
        100000,
    )
    .unwrap()
    .model;
    let abc = author::attach_patch_sheet(&ab, &c, 0, si, ti, 100000)
        .unwrap()
        .0;
    let d = sheet(1, 1);
    let su = boundary(&abc, 2, 0, 1.);
    let sv = boundary(&abc, 1, 1, 1.);
    let tu = boundary(&d, 0, 0, 0.);
    let tv = boundary(&d, 0, 1, 0.);
    let d = author::align_patch_endpoints_many(
        &d,
        &[
            (tu, &abc.edges[abc.loops[2].coedges[su].edge].curve),
            (tv, &abc.edges[abc.loops[1].coedges[sv].edge].curve),
        ],
        1e-8,
        100000,
    )
    .unwrap()
    .model;
    let network =
        author::attach_patch_sheet_edges(&abc, &d, &[([2, su], tu), ([1, sv], tv)], 100000)
            .unwrap()
            .0;
    assert_eq!(network.faces.len(), 4);
    assert!(network.validate().unwrap().topology_valid);
    for (wire, (q, s)) in [(0, (0, 0)), (1, (0, 1)), (2, (1, 0)), (3, (1, 1))] {
        for &z in &limits {
            for i in candidate(q, s).plane_boundary_indices(z).unwrap() {
                let edge = &network.edges[network.loops[wire].coedges[i].edge];
                assert!(edge.curve.control_points.iter().all(|p| p[2] == z));
            }
        }
    }
    assert!(network.bodies.is_empty());
}

#[test]
fn adjacent_clipped_thread_faces_share_a_planar_end_vertex() {
    use brep_core::thread as author;
    let limits = [1. / 32., 1. / 8.];
    let trims = thread::trim_candidates(Spec::default(), limits, 1e-10).unwrap();
    let candidate = |segment| {
        trims
            .candidates
            .iter()
            .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == segment)
            .unwrap()
    };
    let a = author::trimmed_patch_sheet_with_planar_ends(
        candidate(0),
        &limits,
        1e-12,
        1e-3,
        128,
        1e-8,
        100000,
    )
    .unwrap();
    let b = author::trimmed_patch_sheet_with_planar_ends(
        candidate(1),
        &limits,
        1e-12,
        1e-3,
        128,
        1e-8,
        100000,
    )
    .unwrap();
    let si = a.model.loops[0]
        .coedges
        .iter()
        .position(|c| c.pcurve.control_points.iter().all(|p| p[0] == 1.))
        .unwrap();
    let ti = b.model.loops[0]
        .coedges
        .iter()
        .position(|c| c.pcurve.control_points.iter().all(|p| p[0] == 0.))
        .unwrap();
    let source_edge = &a.model.edges[a.model.loops[0].coedges[si].edge].curve;
    let aligned = author::align_patch_endpoints(&b.model, ti, source_edge, 1e-8, 100000).unwrap();
    let joined = author::join_patch_sheets(&a.model, &aligned.model, si, ti, 100000)
        .unwrap()
        .0;
    assert_eq!(joined.faces.len(), 2);
    assert!(joined.validate().unwrap().topology_valid);
    let shared_edge = &joined.edges[joined.loops[0].coedges[si].edge];
    assert!(
        shared_edge
            .vertices
            .iter()
            .any(|&v| joined.vertices[v].point[2] == limits[1])
    );
    for (wire, c) in [(0, candidate(0)), (1, candidate(1))] {
        for &z in &limits {
            for i in c.plane_boundary_indices(z).unwrap() {
                let edge = &joined.edges[joined.loops[wire].coedges[i].edge];
                assert!(edge.curve.control_points.iter().all(|p| p[2] == z));
            }
        }
    }
    let mut invalid = candidate(0).clone();
    invalid.exact_uv_vertices[0] = thread::ExactUvVertex::Corner { corner: 4 };
    assert!(invalid.plane_boundary_indices(limits[0]).is_err());
    assert!(
        author::trimmed_patch_sheet_with_planar_ends(
            candidate(0),
            &limits,
            1e-12,
            1e-3,
            128,
            1e-8,
            0
        )
        .is_err()
    );
}

#[test]
fn full_boundary_face_attachment_fills_four_edge_hole_atomically() {
    use brep_core::thread as author;
    use nurbs_core::curve::Curve;
    let square = |x: f64, y: f64| {
        let p = [
            vec![x, y, 0.],
            vec![x + 1., y, 0.],
            vec![x + 1., y + 1., 0.],
            vec![x, y + 1., 0.],
        ];
        let wire = (0..4)
            .map(|i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
            .collect();
        brep_core::planar_cap::build_sheet(&[wire], 0., 1e-8, 1000).unwrap()
    };
    let coords = [
        (0., 0.),
        (1., 0.),
        (2., 0.),
        (2., 1.),
        (2., 2.),
        (1., 2.),
        (0., 2.),
        (0., 1.),
    ];
    let mut ring = square(0., 0.);
    let steps = [
        (0, 1, 3),
        (1, 1, 3),
        (2, 2, 0),
        (3, 2, 0),
        (4, 3, 1),
        (5, 3, 1),
    ];
    for (i, &(wire, s, t)) in steps.iter().enumerate() {
        let (x, y) = coords[i + 1];
        ring = author::attach_patch_sheet_edges(&ring, &square(x, y), &[([wire, s], t)], 1000)
            .unwrap()
            .0;
    }
    ring =
        author::attach_patch_sheet_edges(&ring, &square(0., 1.), &[([6, 0], 2), ([0, 2], 0)], 1000)
            .unwrap()
            .0;
    let before = ring.clone();
    let cap = square(1., 1.);
    let pairs = [([1, 2], 0), ([3, 3], 1), ([5, 0], 2), ([7, 1], 3)];
    let (filled, cells) = author::attach_patch_sheet_edges(&ring, &cap, &pairs, 1000).unwrap();
    assert_eq!(filled.faces.len(), 9);
    assert_eq!(filled.edges.len(), 24);
    assert_eq!(filled.vertices.len(), 16);
    assert_eq!(filled.validate().unwrap().boundary_edge_count, 12);
    assert!(cells <= 1000);
    assert_eq!(ring, before);
    assert!(author::attach_patch_sheet_edges(&ring, &cap, &pairs, 0).is_err());
    assert!(author::attach_patch_sheet_edges(&ring, &cap, &[pairs[0], pairs[0]], 1000).is_err());
    assert_eq!(ring, before);
}

#[test]
fn finite_thread_patch_has_exact_planar_end_edges_after_bounded_projection() {
    use brep_core::thread as author;
    let trims = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let candidate = trims
        .candidates
        .iter()
        .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == 0)
        .unwrap();
    let sheet = author::trimmed_patch_sheet(candidate, 1e-12, 1e-3, 128, 100000).unwrap();
    let mut planes = Vec::new();
    for i in 0..candidate.exact_uv_vertices.len() {
        if let (
            thread::ExactUvVertex::EdgePlane { z_limit: a, .. },
            thread::ExactUvVertex::EdgePlane { z_limit: b, .. },
        ) = (
            &candidate.exact_uv_vertices[i],
            &candidate.exact_uv_vertices[(i + 1) % candidate.exact_uv_vertices.len()],
        ) {
            if a == b {
                planes.push((i, *a));
            }
        }
    }
    assert_eq!(planes.len(), 2);
    let original = sheet.model.clone();
    let result = author::planarize_patch_edges(&sheet.model, &planes, 1e-8, 100000).unwrap();
    assert!(result.max_curve_change_upper_bound <= 1e-8);
    assert!(
        result
            .agreements
            .iter()
            .all(|r| r.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    );
    assert_eq!(sheet.model, original);
    assert_eq!(result.model.faces[0].surface, original.faces[0].surface);
    for &(coedge, z) in &planes {
        let edge = &result.model.edges[result.model.loops[0].coedges[coedge].edge];
        assert!(edge.curve.control_points.iter().all(|p| p[2] == z));
        for &v in &edge.vertices {
            assert_eq!(result.model.vertices[v].point[2], z);
        }
    }
    assert!(result.model.validate().unwrap().topology_valid);
    assert!(author::planarize_patch_edges(&sheet.model, &planes, 1e-8, 0).is_err());
    assert!(author::planarize_patch_edges(&sheet.model, &[(0, 0.), (1, 1.)], 1., 100000).is_err());
    assert!(
        author::planarize_patch_edges(&sheet.model, &[(planes[0].0, 100.)], 1e-8, 100000).is_err()
    );
    assert_eq!(sheet.model, original);
}

#[test]
fn whole_thread_turn_assembles_twenty_retained_patches() {
    let result = brep_core::thread::untrimmed_turn_sheet(
        Spec::default(),
        0,
        0,
        1e-12,
        1e-3,
        128,
        1e-10,
        2_000_000,
    )
    .unwrap();
    assert_eq!(result.model.faces.len(), 20);
    assert_eq!(result.model.edges.len(), 49);
    assert_eq!(result.model.vertices.len(), 30);
    assert_eq!(result.model.validate().unwrap().boundary_edge_count, 18);
    assert!(result.verification_cells <= 2_000_000);
    assert!(result.max_endpoint_adjustment_upper_bound <= 1e-10);
    assert!(!result.model.shells[0].closed);
    assert!(result.model.bodies.is_empty());
    let patches = thread::patches(Spec::default()).unwrap();
    for (face, original) in result.model.faces.iter().zip(patches.patches) {
        assert_eq!(face.surface, original.surface);
    }
    assert!(
        brep_core::thread::untrimmed_turn_sheet(Spec::default(), 0, 0, 1e-12, 1e-3, 128, 1e-10, 0)
            .is_err()
    );
}

#[test]
fn real_thread_patches_form_a_two_by_two_network() {
    use brep_core::thread as author;
    let trims = thread::trim_candidates(Spec::default(), [0., 2.], 1e-10).unwrap();
    let sheet = |q, s| {
        let candidate = trims
            .candidates
            .iter()
            .find(|c| c.patch.quarter == q && c.patch.profile_segment == s)
            .unwrap();
        author::trimmed_patch_sheet(candidate, 1e-12, 1e-3, 128, 100000)
            .unwrap()
            .model
    };
    let a = sheet(0, 0);
    let b = sheet(0, 1);
    let ab = author::join_patch_sheets(&a, &b, 1, 3, 100000).unwrap().0;
    let c = sheet(1, 0);
    let source_c = &ab.edges[ab.loops[0].coedges[2].edge].curve;
    let aligned_c = author::align_patch_endpoints(&c, 0, source_c, 1e-10, 100000).unwrap();
    let abc = author::attach_patch_sheet(&ab, &aligned_c.model, 0, 2, 0, 100000)
        .unwrap()
        .0;
    let d = sheet(1, 1);
    let source_u = &abc.edges[abc.loops[2].coedges[1].edge].curve;
    let source_v = &abc.edges[abc.loops[1].coedges[2].edge].curve;
    let aligned_d =
        author::align_patch_endpoints_many(&d, &[(3, source_u), (0, source_v)], 1e-10, 100000)
            .unwrap();
    let network = author::attach_patch_sheet_two_edges(
        &abc,
        &aligned_d.model,
        [[2, 1], [0, 3]],
        [[1, 2], [0, 0]],
        100000,
    )
    .unwrap();
    assert_eq!(network.faces.len(), 4);
    assert_eq!(network.edges.len(), 12);
    assert_eq!(network.vertices.len(), 9);
    assert_eq!(network.validate().unwrap().boundary_edge_count, 8);
    assert!(!network.shells[0].closed);
    assert!(network.bodies.is_empty());
    let before = d.clone();
    let mut conflicting = source_v.clone();
    *conflicting.control_points.last_mut().unwrap() = vec![100., 100., 100.];
    assert!(
        author::align_patch_endpoints_many(&d, &[(3, source_u), (0, &conflicting)], 1e-10, 100000)
            .is_err()
    );
    assert_eq!(d, before);
}

#[test]
fn four_face_grid_sews_the_second_boundary_of_the_last_face() {
    use nurbs_core::curve::Curve;
    let sheet = |x: f64, y: f64| {
        let p = [
            vec![x, y, 0.],
            vec![x + 1., y, 0.],
            vec![x + 1., y + 1., 0.],
            vec![x, y + 1., 0.],
        ];
        let wire = (0..4)
            .map(|i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
            .collect();
        brep_core::planar_cap::build_sheet(&[wire], 0., 1e-8, 1000).unwrap()
    };
    let a = sheet(0., 0.);
    let b = sheet(1., 0.);
    let c = sheet(1., 1.);
    let d = sheet(0., 1.);
    let ab = brep_core::thread::join_patch_sheets(&a, &b, 1, 3, 1000)
        .unwrap()
        .0;
    let abc = brep_core::thread::attach_patch_sheet(&ab, &c, 1, 2, 0, 1000)
        .unwrap()
        .0;
    let before = abc.clone();
    let grid = brep_core::thread::attach_patch_sheet_two_edges(
        &abc,
        &d,
        [[2, 3], [0, 1]],
        [[0, 2], [0, 0]],
        1000,
    )
    .unwrap();
    assert_eq!(grid.faces.len(), 4);
    assert_eq!(grid.edges.len(), 12);
    assert_eq!(grid.vertices.len(), 9);
    assert_eq!(grid.validate().unwrap().boundary_edge_count, 8);
    assert_eq!(grid.loops[0].coedges[2].edge, grid.loops[3].coedges[0].edge);
    assert!(grid.loops[3].coedges[0].reversed);
    assert_eq!(abc, before);
    assert!(brep_core::thread::sew_patch_boundaries(&grid, [0, 2], [3, 0], 1000).is_err());
    assert!(
        brep_core::thread::attach_patch_sheet_two_edges(
            &abc,
            &d,
            [[2, 3], [0, 1]],
            [[0, 2], [0, 0]],
            0
        )
        .is_err()
    );
    assert_eq!(abc, before);
}

#[test]
fn quarter_sweep_boundary_reuses_one_world_edge_for_both_hands() {
    use nurbs_core::curve::Curve;
    for hand in [thread::Hand::Right, thread::Hand::Left] {
        let trims = thread::trim_candidates(
            Spec {
                hand,
                ..Spec::default()
            },
            [0., 2.],
            1e-10,
        )
        .unwrap();
        let find = |quarter| {
            trims
                .candidates
                .iter()
                .find(|c| c.patch.quarter == quarter && c.patch.profile_segment == 1)
                .unwrap()
        };
        let source = find(0);
        let target = find(1);
        let mapped = source.mapped_edges(1e-12, 1e-3, 128, 100000).unwrap();
        let edge = mapped
            .edges
            .iter()
            .find(|e| e.pcurve.control_points.iter().all(|p| p[1] == 1.))
            .unwrap();
        let uv = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
        let shared = brep_core::thread::share_patch_edge(
            edge,
            &source.patch.surface,
            &uv,
            &target.patch.surface,
            true,
            1e-3,
            100000,
        )
        .unwrap();
        assert_eq!(shared.curve, edge.curve);
        let a = brep_core::thread::trimmed_patch_sheet(source, 1e-12, 1e-3, 128, 100000).unwrap();
        let b = brep_core::thread::trimmed_patch_sheet(target, 1e-12, 1e-3, 128, 100000).unwrap();
        let si = a.model.loops[0]
            .coedges
            .iter()
            .position(|c| c.pcurve == edge.pcurve)
            .unwrap();
        let ti = b.model.loops[0]
            .coedges
            .iter()
            .position(|c| c.pcurve == uv)
            .unwrap();
        let before = b.model.clone();
        let aligned =
            brep_core::thread::align_patch_endpoints(&b.model, ti, &edge.curve, 1e-10, 100000)
                .unwrap();
        assert!(aligned.max_curve_change_upper_bound <= 1e-10);
        let mut displaced = edge.curve.clone();
        displaced.control_points[0][0] += 1e-4;
        assert!(
            brep_core::thread::align_patch_endpoints(&b.model, ti, &displaced, 1e-10, 100000)
                .is_err()
        );
        assert_eq!(aligned.model.faces[0].surface, b.model.faces[0].surface);
        assert_eq!(b.model, before);
        let joined = brep_core::thread::join_patch_sheets(&a.model, &aligned.model, si, ti, 100000)
            .unwrap()
            .0;
        assert_eq!(joined.faces.len(), 2);
        assert_eq!(joined.validate().unwrap().boundary_edge_count, 6);
        assert!(
            brep_core::thread::align_patch_endpoints(&b.model, ti, &edge.curve, 1e-10, 0).is_err()
        );
        assert!(
            shared
                .agreements
                .iter()
                .all(|a| a.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
        );
    }
}

#[test]
fn one_world_edge_is_verified_on_adjacent_profile_patches() {
    use nurbs_core::curve::Curve;
    let trims = thread::trim_candidates(Spec::default(), [0., 2.], 1e-10).unwrap();
    let find = |segment| {
        trims
            .candidates
            .iter()
            .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == segment)
            .unwrap()
    };
    let source = find(0);
    let target = find(1);
    let mapped = source.mapped_edges(1e-12, 1e-3, 128, 100000).unwrap();
    let edge = mapped
        .edges
        .iter()
        .find(|e| e.pcurve.control_points.iter().all(|p| p[0] == 1.))
        .unwrap();
    let uv = Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap();
    let shared = brep_core::thread::share_patch_edge(
        edge,
        &source.patch.surface,
        &uv,
        &target.patch.surface,
        true,
        1e-3,
        100000,
    )
    .unwrap();
    assert_eq!(shared.curve, edge.curve);
    assert_eq!(shared.target_pcurve, uv);
    assert!(shared.target_reversed);
    let a = brep_core::thread::trimmed_patch_sheet(source, 1e-12, 1e-3, 128, 100000).unwrap();
    let b = brep_core::thread::trimmed_patch_sheet(target, 1e-12, 1e-3, 128, 100000).unwrap();
    let before_a = a.model.clone();
    let before_b = b.model.clone();
    let si = a.model.loops[0]
        .coedges
        .iter()
        .position(|c| c.pcurve == edge.pcurve)
        .unwrap();
    let ti = b.model.loops[0]
        .coedges
        .iter()
        .position(|c| c.pcurve == uv)
        .unwrap();
    let (joined, _) =
        brep_core::thread::join_patch_sheets(&a.model, &b.model, si, ti, 100000).unwrap();
    assert_eq!(joined.faces.len(), 2);
    assert_eq!(joined.edges.len(), 7);
    assert_eq!(joined.vertices.len(), 6);
    assert_eq!(joined.validate().unwrap().boundary_edge_count, 6);
    assert_eq!(
        joined.loops[0].coedges[si].edge,
        joined.loops[1].coedges[ti].edge
    );
    assert!(joined.loops[1].coedges[ti].reversed);
    let third = brep_core::thread::trimmed_patch_sheet(find(2), 1e-12, 1e-3, 128, 100000).unwrap();
    let ni = joined.loops[1]
        .coedges
        .iter()
        .position(|c| c.pcurve.control_points.iter().all(|p| p[0] == 1.))
        .unwrap();
    let nti = third.model.loops[0]
        .coedges
        .iter()
        .position(|c| c.pcurve == uv)
        .unwrap();
    let (chain, _) =
        brep_core::thread::attach_patch_sheet(&joined, &third.model, 1, ni, nti, 100000).unwrap();
    assert_eq!(chain.faces.len(), 3);
    assert_eq!(chain.edges.len(), 10);
    assert_eq!(chain.vertices.len(), 8);
    assert_eq!(chain.validate().unwrap().boundary_edge_count, 8);
    assert_eq!(chain.shells[0].faces.len(), 3);
    assert!(chain.bodies.is_empty());
    let mut strip = chain;
    for segment in 3..5 {
        let next = brep_core::thread::trimmed_patch_sheet(find(segment), 1e-12, 1e-3, 128, 100000)
            .unwrap();
        let previous_loop = segment - 1;
        let si = strip.loops[previous_loop]
            .coedges
            .iter()
            .position(|c| c.pcurve.control_points.iter().all(|p| p[0] == 1.))
            .unwrap();
        let ti = next.model.loops[0]
            .coedges
            .iter()
            .position(|c| c.pcurve == uv)
            .unwrap();
        strip = brep_core::thread::attach_patch_sheet(
            &strip,
            &next.model,
            previous_loop,
            si,
            ti,
            100000,
        )
        .unwrap()
        .0;
    }
    assert_eq!(strip.faces.len(), 5);
    assert_eq!(strip.edges.len(), 16);
    assert_eq!(strip.vertices.len(), 12);
    assert_eq!(strip.validate().unwrap().boundary_edge_count, 12);
    assert!(!strip.shells[0].closed);
    assert!(
        brep_core::thread::attach_patch_sheet(&joined, &third.model, 0, si, nti, 100000).is_err()
    );
    assert!(
        brep_core::thread::attach_patch_sheet(&joined, &third.model, usize::MAX, ni, nti, 100000)
            .is_err()
    );
    assert_eq!(a.model, before_a);
    assert_eq!(b.model, before_b);
    assert!(brep_core::thread::join_patch_sheets(&a.model, &b.model, si, ti, 0).is_err());
    assert!(shared.verification_cells <= 100000);
    assert!(
        shared
            .agreements
            .iter()
            .all(|a| a.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    );
    assert!(
        brep_core::thread::share_patch_edge(
            edge,
            &source.patch.surface,
            &uv,
            &target.patch.surface,
            false,
            1e-3,
            100000
        )
        .is_err()
    );
    assert!(
        brep_core::thread::share_patch_edge(
            edge,
            &source.patch.surface,
            &uv,
            &target.patch.surface,
            true,
            1e-3,
            0
        )
        .is_err()
    );
    let mut wrong = target.patch.surface.clone();
    for p in wrong.control_points.iter_mut().flatten() {
        p[0] += 0.1;
    }
    assert!(
        brep_core::thread::share_patch_edge(
            edge,
            &source.patch.surface,
            &uv,
            &wrong,
            true,
            1e-3,
            100000
        )
        .is_err()
    );
}

#[test]
fn patch_sheets_support_both_thread_kinds_and_hands() {
    for kind in [thread::Kind::External, thread::Kind::Internal] {
        for hand in [thread::Hand::Right, thread::Hand::Left] {
            let trims = thread::trim_candidates(
                Spec {
                    kind,
                    hand,
                    ..Spec::default()
                },
                [1. / 32., 1. / 8.],
                1e-10,
            )
            .unwrap();
            let candidate = trims
                .candidates
                .iter()
                .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == 0)
                .unwrap();
            let sheet = brep_core::thread::trimmed_patch_sheet(candidate, 1e-12, 1e-3, 128, 100000)
                .unwrap();
            assert_eq!(sheet.model.faces[0].surface, candidate.patch.surface);
            assert!(
                sheet
                    .agreements
                    .iter()
                    .all(|a| a.status
                        == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
            );
            assert!(sheet.model.bodies.is_empty());
        }
    }
}

#[test]
fn trimmed_thread_patch_authors_retained_surface_and_verified_open_topology() {
    let trims = thread::trim_candidates(Spec::default(), [1. / 32., 1. / 8.], 1e-10).unwrap();
    let candidate = trims
        .candidates
        .iter()
        .find(|c| c.patch.quarter == 0 && c.patch.profile_segment == 0)
        .unwrap();
    let sheet =
        brep_core::thread::trimmed_patch_sheet(candidate, 1e-12, 1e-3, 128, 100000).unwrap();
    assert_eq!(sheet.model.faces[0].surface, candidate.patch.surface);
    assert_eq!(sheet.model.edges.len(), 5);
    assert!(sheet.model.validate().unwrap().topology_valid);
    assert!(!sheet.model.shells[0].closed);
    assert!(sheet.model.bodies.is_empty());
    assert!(
        sheet
            .agreements
            .iter()
            .all(|a| a.status == nurbs_core::curve_surface_agreement::Status::WithinTolerance)
    );
    assert!(sheet.verification_cells <= 100000);
    for edge in &sheet.model.edges {
        for end in 0..2 {
            let p = if end == 0 {
                edge.curve.control_points.first()
            } else {
                edge.curve.control_points.last()
            }
            .unwrap();
            assert_eq!(
                p.as_slice(),
                sheet.model.vertices[edge.vertices[end]].point.as_slice()
            );
        }
    }
    assert!(brep_core::thread::trimmed_patch_sheet(candidate, 1e-12, 1e-3, 128, 0).is_err());
}

#[test]
fn single_bezier_thread_network_accepts_curved_general_caps() {
    let spec = Spec {
        turns: 2,
        error_budget: 1e-2,
        ..Spec::default()
    };
    let limits = [1.13, 1.87];
    let network =
        brep_core::thread::clipped_thread_network(spec, limits, 1e-12, 1e-3, 128, 1e-8, 4_000_000)
            .unwrap();
    assert!(network.model.edges.iter().any(|e| e.curve.degree == 6));
    let before = network.model.clone();
    assert!(
        brep_core::thread::cap_patch_network_plane_general(
            &network.model,
            limits[0],
            0,
            0,
            0,
            100000
        )
        .is_err()
    );
    let (lower, _) = brep_core::thread::cap_patch_network_plane_general(
        &network.model,
        limits[0],
        10000,
        100000,
        1000000,
        100000,
    )
    .unwrap();
    let (closed, _) = brep_core::thread::cap_patch_network_plane_general(
        &lower, limits[1], 10000, 100000, 1000000, 100000,
    )
    .unwrap();
    assert_eq!(closed.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(closed.faces.len(), network.model.faces.len() + 2);
    assert!(closed.bodies.is_empty());
    assert_eq!(network.model, before);
}

#[test]
fn finite_external_candidate_builds_closed_body_with_shared_budget() {
    let spec = Spec {
        turns: 2,
        error_budget: 1e-2,
        ..Spec::default()
    };
    let limits = brep_core::thread::FiniteThreadLimits {
        uv_error: 1e-12,
        tolerance_mm: 1e-3,
        max_spans: 128,
        max_curve_change: 1e-8,
        agreement_cells: 4_000_000,
        trim_pairs_per_cap: 10000,
        trim_cells_per_cap: 100000,
        domain_cells_per_cap: 1000000,
        mass_relative_tolerance: 1e-4,
        mass_evaluations: 2_000_000,
    };
    let result = brep_core::thread::finite_external_candidate(spec, [1.13, 1.87], limits).unwrap();
    let model = &result.body.model;
    assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
    assert_eq!(model.bodies.len(), 1);
    assert!(model.shells[0].closed);
    assert_eq!(model.faces.len(), result.side_faces + 2);
    assert!(result.agreement_cells <= limits.agreement_cells);
    assert!(!result.body.solid_geometry_certified);
    assert!(result.body.mass.signed_volume_mm3 > result.body.mass.volume_error_estimate_mm3);
    assert!(model.edges.iter().any(|e| e.curve.degree == 6));
    let internal = Spec {
        kind: thread::Kind::Internal,
        ..spec
    };
    assert!(brep_core::thread::finite_external_candidate(internal, [1.13, 1.87], limits).is_err());
    let no_work = brep_core::thread::FiniteThreadLimits {
        agreement_cells: 0,
        ..limits
    };
    assert!(brep_core::thread::finite_external_candidate(spec, [1.13, 1.87], no_work).is_err());
}
