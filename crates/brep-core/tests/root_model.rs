#[path = "support/closed_solid_policy.rs"]
mod solid_policy;
use brep_core::root_model::RootBrep;
#[path = "../../nurbs-core/tests/support/ss_root_circle.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "../../nurbs-core/tests/support/ss_closed_fiber.rs"]
mod atlas_fixture;
#[test]
fn paired_brep_keeps_shared_original_world_edges_and_both_uv_projections() {
    let contour = fixture::wire();
    let edge = &contour.edges()[2];
    let clock = edge.original_inner_clock_with_projection().unwrap();
    let endpoint = |value| num_rational::BigRational::from_float(value).unwrap();
    assert_eq!(clock, [endpoint(0.3125), endpoint(0.6875)]);
    let model = RootBrep::paired_contours(&contour, &[], 1e-7).unwrap();
    model.reverify().unwrap();
    let topology = model.topology();
    assert_eq!(topology.vertices.len(), 8);
    assert_eq!(topology.edges.len(), 8);
    assert_eq!(topology.loops.len(), 2);
    assert_eq!(topology.faces.len(), 2);
    assert_eq!(topology.shells.len(), 1);
    assert!(topology.shells[0].closed);
    assert_eq!(topology.euler_characteristic(), 2);
    assert!(topology.boundary_edges().is_empty());
    for i in 0..8 {
        assert_eq!(
            topology.loops[0].coedges[i].edge,
            topology.loops[1].coedges[i].edge
        );
        assert_eq!(topology.loops[0].coedges[i].pcurve.support, 0);
        assert_eq!(topology.loops[1].coedges[i].pcurve.support, 1);
        let a = model.pcurve_bounds(0, 0, i, Default::default()).unwrap();
        let b = model.pcurve_bounds(1, 0, i, Default::default()).unwrap();
        assert!(a.iter().chain(&b).all(|d| d[0] <= d[1]));
        let world = topology.edges[i]
            .curve
            .enclose(Default::default())
            .unwrap()
            .world;
        assert!(world[2][0] <= 0. && 0. <= world[2][1]);
    }
    assert!(model.pcurve_bounds(2, 0, 0, Default::default()).is_err());
}
#[test]
fn forged_surface_projection_vertex_and_rounded_trim_are_rejected() {
    let model = RootBrep::paired_contours(&fixture::wire(), &[], 1e-7).unwrap();
    let mut swapped = model.topology().clone();
    swapped.loops[0].coedges[0].pcurve.support = 1;
    assert!(RootBrep::from_topology(swapped).is_err());
    let mut wrong = model.topology().clone();
    wrong.faces[0].surface.control_points[0][0][2] += 1.;
    assert!(RootBrep::from_topology(wrong).is_err());
    let mut vertex = model.topology().clone();
    vertex.vertices[0].point = vertex.vertices[1].point.clone();
    assert!(RootBrep::from_topology(vertex).is_err());
    let mut trim = model.topology().clone();
    trim.loops[0].coedges[0] = trim.loops[0].coedges[0]
        .clone()
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [0., 1.],
            pcurve_parameter: [0., 1.],
            periodic_lift: [[0; 2]; 2],
        })
        .unwrap();
    assert!(RootBrep::from_topology(trim).is_err());
    let mut degenerate = model.topology().clone();
    degenerate.edges[0].degenerate = true;
    assert!(RootBrep::from_topology(degenerate).is_err());
}
#[test]
fn irrational_vertex_flows_through_common_ports_wire_and_paired_brep() {
    use nurbs_core::ss_intersection as ss;
    use std::sync::Arc;
    let contour = fixture::wire();
    let first = &contour.edges()[0];
    let clock = Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(
        &[-1., 0., 8.], [0.34375, 0.375], Default::default(),
    ).unwrap());
    let algebraic = ss::OriginalSsAlgebraicPoint::new(first.curve().clone(), clock).unwrap();
    let point = Arc::new(ss::OriginalSsRootPoint::from_algebraic(algebraic.clone(), Default::default()).unwrap());
    assert!(point.binary_parameter().is_none());
    point.reverify().unwrap();
    let again = ss::OriginalSsRootPoint::from_algebraic(algebraic, Default::default()).unwrap();
    assert!(point.same_selector(&again));
    let independent = ss::OriginalSsAlgebraicPoint::new(first.curve().clone(), Arc::new(
        ss::QualifiedSsAlgebraicClock::from_binary(&[-1., 0., 8.], [0.34375, 0.375], Default::default()).unwrap(),
    )).unwrap();
    assert!(!point.same_selector(&ss::OriginalSsRootPoint::from_algebraic(independent, Default::default()).unwrap()));
    let middle = ss::bind_original_ss_port(point.clone(), first.curve().clone(), [0;4], Default::default()).unwrap();
    let independent=ss::OriginalSsAlgebraicPoint::new(first.curve().clone(),Arc::new(
        ss::QualifiedSsAlgebraicClock::from_binary(&[2.,-1.,-16.,8.],[0.34,0.38],Default::default()).unwrap(),
    )).unwrap();
    let canonical=Arc::new(ss::OriginalSsRootPoint::from_algebraic(independent,Default::default()).unwrap());
    let other_middle=ss::bind_original_ss_port(canonical.clone(),first.curve().clone(),[0;4],Default::default()).unwrap();
    assert!(!middle.point().same_selector(other_middle.point()));
    assert!(middle.point().same_source_point(other_middle.point(),Default::default()).unwrap());
    let mut edges = vec![
        ss::OriginalSsRootEdge::new(first.start().clone(), middle.clone()).unwrap(),
        ss::OriginalSsRootEdge::new(other_middle, first.end().clone()).unwrap(),
    ];
    edges.extend_from_slice(&contour.edges()[1..]);
    assert!(ss::OriginalSsRootWire::new(edges.clone()).is_err());
    assert!(ss::OriginalSsRootWire::reconcile_selectors(edges.clone(),ss::SsAlgebraicClockLimits {max_operations:1,..Default::default()}).is_err());
    let wire = ss::OriginalSsRootWire::reconcile_selectors(edges,Default::default()).unwrap();
    wire.reverify().unwrap();
    let model = RootBrep::paired_contours(&wire, &[], 1e-7).unwrap();
    model.reverify().unwrap();
    assert_eq!(model.topology().vertices.len(), 9);
    assert_eq!(model.topology().edges.len(), 9);
    assert_eq!(model.topology().euler_characteristic(), 2);
    assert!(model.topology().boundary_edges().is_empty());
    assert!(model.topology().vertices.iter().any(|v| v.point.selector.same_selector(&canonical)));
    for face in 0..2 {
        let uv = model.pcurve_bounds(face,0,0,Default::default()).unwrap();
        assert!(uv.iter().all(|d| d[0]<=d[1]));
    }
}
#[test]
fn cross_axis_algebraic_junction_is_preserved_by_paired_brep() {
    use nurbs_core::ss_intersection as ss;
    use std::sync::Arc;
    let contour=fixture::wire();
    let edge=&contour.edges()[1];
    let make=|curve,p:&[f64],d| {
        let clock=Arc::new(ss::QualifiedSsAlgebraicClock::from_binary(p,d,Default::default()).unwrap());
        Arc::new(ss::OriginalSsRootPoint::from_algebraic(ss::OriginalSsAlgebraicPoint::new(curve,clock).unwrap(),Default::default()).unwrap())
    };
    let x=make(edge.curve().clone(),&[-257.,0.,4096.],[0.25,0.251]);
    let y=Arc::new(x.reparameterize_to(contour.edges()[2].curve().clone(),Default::default(),Default::default()).unwrap());
    let bind=|point|ss::bind_original_ss_port(point,edge.curve().clone(),[0;4],Default::default()).unwrap();
    let mut edges=vec![contour.edges()[0].clone(),
        ss::OriginalSsRootEdge::new(edge.start().clone(),bind(x)).unwrap(),
        ss::OriginalSsRootEdge::new(bind(y.clone()),edge.end().clone()).unwrap()];
    edges.extend_from_slice(&contour.edges()[2..]);
    assert!(ss::OriginalSsRootWire::new(edges.clone()).is_err());
    let wire=ss::OriginalSsRootWire::reconcile_selectors(edges,Default::default()).unwrap();
    wire.reverify().unwrap();
    let brep=RootBrep::paired_contours(&wire,&[],1e-7).unwrap();
    brep.reverify().unwrap();
    assert_eq!(brep.topology().vertices.len(),9);
    assert_eq!(brep.topology().edges.len(),9);
    assert!(brep.topology().boundary_edges().is_empty());
    assert!(brep.topology().vertices.iter().any(|v|v.point.selector.same_selector(&y)));
    for support in 0..2 {
        let uv=brep.pcurve_bounds(support,0,1,Default::default()).unwrap();
        assert!(uv.iter().all(|d|d[0]<=d[1]));
    }
}

#[test]
fn explicit_face_retention_keeps_only_complete_bodies() {
    use brep_core::root_model::RootFaceRetention;
    use std::sync::Arc;
    let source=Arc::new(RootBrep::paired_contours(&fixture::wire(),&[],1e-7).unwrap());
    let all=RootFaceRetention::retain(source.clone(),vec![1,0]).unwrap();
    assert_eq!(all.body_map(),&[Some(0)]);assert!(all.output().topology().shells[0].closed);
    all.reverify().unwrap();
    let partial=RootFaceRetention::retain(source.clone(),vec![0]).unwrap();
    assert_eq!(partial.body_map(),&[None]);assert!(partial.output().topology().bodies.is_empty());
    assert!(!partial.output().topology().shells[0].closed);partial.reverify().unwrap();
    assert!(RootFaceRetention::retain(source,vec![2]).is_err());
}
#[test]
fn root_geometry_roundtrips_into_existing_mixed_pipeline_without_new_owners() {
    use brep_core::mixed_root_model::{MixedBrep,MixedEdge,MixedVertex,MixedPcurve};
    use std::sync::Arc;
    let source=RootBrep::paired_contours(&fixture::wire(),&[],1e-7).unwrap();
    let mixed=MixedBrep::from_root(&source).unwrap();
    let result=mixed.to_root().unwrap();result.reverify().unwrap();
    let joined=mixed.concatenate(&mixed).unwrap();
    let joined_root=joined.model().to_root().unwrap();joined_root.reverify().unwrap();
    assert_eq!(joined_root.topology().faces.len(),4);
    assert_eq!(joined_root.topology().bodies.len(),2);
    let a=source.topology();let b=result.topology();
    assert_eq!(a.vertices.len(),b.vertices.len());assert_eq!(a.faces.len(),b.faces.len());
    assert_eq!(a.euler_characteristic(),b.euler_characteristic());
    for (i,v) in a.vertices.iter().enumerate() {
        let MixedVertex::Root(p)=&mixed.topology().vertices[i].point else {panic!("root vertex lost")};
        assert!(Arc::ptr_eq(p,&v.point.selector));assert!(Arc::ptr_eq(&b.vertices[i].point.selector,&v.point.selector));
    }
    for (i,e) in a.edges.iter().enumerate() {
        let MixedEdge::Root(root)=&mixed.topology().edges[i].curve else {panic!("root edge lost")};
        assert!(Arc::ptr_eq(root.curve(),e.curve.curve()));assert!(Arc::ptr_eq(b.edges[i].curve.curve(),e.curve.curve()));
        assert_eq!(e.vertices,b.edges[i].vertices);
    }
    for (l,wire) in a.loops.iter().enumerate() {for (i,c) in wire.coedges.iter().enumerate() {
        let MixedPcurve::Root(p)=&mixed.topology().loops[l].coedges[i].pcurve else {panic!("root pcurve lost")};
        let out=&b.loops[l].coedges[i];assert_eq!((p.world_edge,p.support,p.reversed),(c.edge,c.pcurve.support,c.reversed));
        assert_eq!((out.edge,out.pcurve.support,out.reversed),(c.edge,c.pcurve.support,c.reversed));
    }}
}

#[test]
fn exact_root_patches_sew_with_original_face_use_orientation() {
    check_exact_root_patch_flux(false);
}

#[test]
fn root_sewing_preserves_disconnected_closed_components() {
    use std::sync::Arc;
    let wire = fixture::wire();
    let mut plane = wire.edges()[0].curve().source().first().clone();
    for point in plane.control_points.iter_mut().flatten() {
        point[2] += 2.;
    }
    let other = fixture::wire_on(plane);
    let first = Arc::new(RootBrep::paired_contours(&wire, &[], 1e-7).unwrap());
    let second = Arc::new(RootBrep::paired_contours(&other, &[], 1e-7).unwrap());
    let patches = [first, second];
    let result = RootBrep::sew_root_shells(&patches, 1e-7).unwrap();
    assert_eq!(result.topology().shells.len(), 2);
    assert!(result.topology().shells.iter().all(|shell| shell.closed && shell.faces.len() == 2));
    assert!(result.topology().boundary_edges().is_empty());
    assert!(RootBrep::sew_root_shell(&patches, 1e-7).is_err());
}

#[test]
#[ignore = "Deep interval refinement of exact trim curves takes several minutes in debug builds"]
fn exact_root_patches_sew_with_positive_signed_flux() {
    check_exact_root_patch_flux(true);
}

fn check_exact_root_patch_flux(prove_sign: bool) {
    use brep_core::root_model::RootFaceRetention;
    use std::sync::Arc;
    let source = Arc::new(RootBrep::paired_contours(&fixture::wire(), &[], 1e-7).unwrap());
    let left = RootFaceRetention::retain(source.clone(), vec![0]).unwrap();
    let right = RootFaceRetention::retain(source, vec![1]).unwrap();
    let sewn = RootBrep::sew_root_shell(&[left.output().clone(), right.output().clone()], 1e-7).unwrap();
    let topology = sewn.topology();
    assert_eq!(topology.edges.len(), 8);
    assert!(topology.shells[0].closed);
    assert!(topology.boundary_edges().is_empty());
    assert!(topology.bodies.is_empty());
    let charts = brep_core::root_model::QualifiedRootShellCharts::qualify(
        Arc::new(sewn.clone()), 64, Default::default(),
    ).unwrap();
    charts.reverify(Default::default()).unwrap();
    let (max_cells, max_depth) = if prove_sign { (127, 6) } else { (32, 2) };
    let flux = charts.signed_flux(brep_core::root_model::RootShellFluxLimits {
        axis: 2, origin: 0., max_cells, max_spans: max_cells, max_depth,
        winding: nurbs_core::ss_intersection::OriginalSsWindingLimits {
            max_segments: 32, ..Default::default()
        },
    }).unwrap();
    // The bowl and planar disk enclose pi * (5/16)^4 / 2, up to sign.
    let expected = std::f64::consts::PI * (5_f64 / 16.).powi(4) / 2.;
    let [lo, hi] = flux.signed_bounds;
    assert!((lo <= expected && expected <= hi) || (lo <= -expected && -expected <= hi));
    if prove_sign {
        assert!(flux.has_positive_lower_bound(), "{lo}..{hi}");
        assert!(lo <= expected && expected <= hi, "{lo}..{hi}");
        use nurbs_core::ss_intersection as ss;
        let carriers = topology.edges[0].curve.curve().source();
        let fibers: Vec<_> =
            atlas_fixture::family(carriers, &[([0.5; 2], [0.3125; 2])])
                .iter()
                .map(|chart| {
                    chart.source()
                        .certify_quotient(chart.domain(), chart.free_axis(), Default::default())
                        .unwrap()
                        .into_fiber()
                        .expect("quotient fiber on the bowl rim")
                })
                .collect();
        let census = ss::cover_quotient_fiber_surfaces(&fibers, Default::default())
            .unwrap()
            .into_coverage()
            .expect("complete bowl and disk carrier intersections");
        // The two native V intervals meet only at 5/16. The shared
        // projection leaves a line, whose only root is at its U endpoint.
        let corners = ss::shared_frame_graph_contact_corners(
            carriers,
            [[0.21875, 0.25], [0.3125, 0.353], [0.21875, 0.25], [0.31, 0.3125]],
        ).unwrap().unwrap();
        let expected_corner = [0.25, 0.3125, 0.25, 0.3125]
            .map(|value| num_rational::BigRational::from_float(value).unwrap());
        assert_eq!(corners, vec![expected_corner]);
        let proof = brep_core::root_model::QualifiedRootShellPositiveFlux::from_patches(
            &[left.output().clone(), right.output().clone()],
            1e-7,
            64,
            vec![([0, 1], census)],
            128,
            brep_core::root_model::RootShellFluxLimits {
                axis: 2,
                origin: 0.,
                max_cells,
                max_spans: max_cells,
                max_depth,
                winding: ss::OriginalSsWindingLimits {
                    max_segments: 32,
                    ..Default::default()
                },
            },
        )
        .unwrap();
        assert_eq!(proof.report().signed_bounds, flux.signed_bounds);
        for (point, expected) in [([0.5, 0.5, -0.04], true), ([0.5, 0.5, 0.1], false)] {
            assert_eq!(
                proof.classify_point(point, [0., 0., 1.], 1e-8, 100_000, 8192).unwrap(),
                Some(expected),
            );
        }
        assert!(proof.source().topology().boundary_edges().is_empty());
        let (assembled, volume) =
            brep_core::root_model::QualifiedRootShellPositiveFlux::assemble_disjoint_bodies(
                std::slice::from_ref(&proof),
            ).unwrap();
        assert_eq!(assembled.topology().bodies.len(), 1);
        assert!(volume[0] <= expected && expected <= volume[1]);
        assert!(
            brep_core::root_model::QualifiedRootShellPositiveFlux::assemble_disjoint_bodies(
                &[proof.clone(), proof.clone()],
            ).is_err()
        );
        let qualify_placed = |plane, origin| {
            let wire = fixture::wire_on(plane);
            let carriers = wire.edges()[0].curve().source();
            let fibers = atlas_fixture::family(carriers, &[([0.5; 2], [0.3125; 2])])
                .iter()
                .map(|chart| chart.source()
                    .certify_quotient(chart.domain(), chart.free_axis(), Default::default())
                    .unwrap()
                    .into_fiber()
                    .unwrap())
                .collect::<Vec<_>>();
            let census = ss::cover_quotient_fiber_surfaces(&fibers, Default::default())
                .unwrap().into_coverage().unwrap();
            let shell = Arc::new(RootBrep::paired_contours(&wire, &[], 1e-7).unwrap());
            brep_core::root_model::QualifiedRootShellPositiveFlux::from_patches(
                &[shell], 1e-7, 64, vec![([0, 1], census)], 128,
                brep_core::root_model::RootShellFluxLimits {
                    axis: 2, origin, max_cells, max_spans: max_cells, max_depth,
                    winding: ss::OriginalSsWindingLimits { max_segments: 32, ..Default::default() },
                },
            ).unwrap()
        };
        let mut shifted_plane = carriers.first().clone();
        for point in shifted_plane.control_points.iter_mut().flatten() {
            point[2] += 2.;
        }
        let shifted_proof = qualify_placed(shifted_plane, 2.);
        let mut inner_plane = carriers.first().clone();
        for point in inner_plane.control_points.iter_mut().flatten() {
            point[0] = 0.375 + 0.25 * point[0];
            point[1] = 0.375 + 0.25 * point[1];
            point[2] = -0.03125;
        }
        let inner_proof = qualify_placed(inner_plane, -0.03125);
        let rays = [[0., 0., 1.], [1., 0., 0.], [0.137, 0.283, 1.]];
        use brep_core::mixed_root_model::RootRelationKind;
        let hollow = proof
            .subtract_root_shell(&inner_proof, &rays, 4096, Default::default(), 0.02)
            .unwrap();
        assert_eq!(hollow.model().topology().bodies.len(), 1);
        assert_eq!(hollow.model().topology().bodies[0].inner_shells, [1]);
        assert_eq!(hollow.lineage()[1].shell_orientation_reversed(0), Some(true));
        assert!(hollow.lineage()[1].target_matches(hollow.model()));
        let expected_hollow = expected * (1. - 0.25_f64.powi(4));
        let [lo, hi] = hollow.volume();
        assert!(lo <= expected_hollow && expected_hollow <= hi, "{lo}..{hi}");
        assert_eq!(
            proof.classify_shell_relation_with_rays(&inner_proof, &rays, 4096, Default::default()).unwrap(),
            RootRelationKind::SecondInsideFirst,
        );
        assert_eq!(
            inner_proof.classify_shell_relation_with_rays(&proof, &rays, 4096, Default::default()).unwrap(),
            RootRelationKind::FirstInsideSecond,
        );
        use brep_core::closed_solid::BooleanMaterialOp as Op;
        for (op, target, bodies, voids) in [
            (Op::Union, expected, 1, 0),
            (Op::Intersection, expected * 0.25_f64.powi(4), 1, 0),
            (Op::Difference, expected_hollow, 1, 1),
            (Op::Xor, expected_hollow, 1, 1),
        ] {
            let (model, [lo, hi]) = proof.boolean_with_separated_shell(
                &inner_proof, op, &rays, 4096, Default::default(), 0.02,
            ).unwrap();
            let model = model.unwrap();
            assert_eq!(model.topology().bodies.len(), bodies);
            assert_eq!(model.topology().bodies[0].inner_shells.len(), voids);
            assert!(lo <= target && target <= hi, "{op:?}: {lo}..{hi}");
        }
        let swapped = inner_proof.boolean_with_separated_shell_and_lineage(
            &proof, Op::Xor, &rays, 4096, Default::default(), 0.02,
        ).unwrap();
        let model = swapped.model.as_ref().unwrap();
        for (index, map) in swapped.lineage.iter().enumerate() {
            let map = map.as_ref().unwrap();
            assert!(map.source_matches(&swapped.sources[index]) && map.target_matches(model));
            assert_eq!(map.shell_orientation_reversed(0), Some(index == 0));
        }
        let (empty, volume) = inner_proof.boolean_with_separated_shell(
            &proof, Op::Difference, &rays, 4096, Default::default(), 0.02,
        ).unwrap();
        assert!(empty.is_none());
        assert_eq!(volume, [0.; 2]);
        proof.prove_boundary_separation(&shifted_proof, 4).unwrap();
        assert_eq!(
            proof.classify_shell_relation_with_rays(
                &shifted_proof, &[[0., 0., 1.], [1., 0., 0.]], 128, Default::default(),
            ).unwrap(),
            brep_core::mixed_root_model::RootRelationKind::Disjoint,
        );
        assert!(proof.prove_boundary_separation(&shifted_proof, 3).is_err());
        assert!(proof.prove_boundary_separation(&proof, 4).is_err());
        let disjoint_rays = [[1., 0., 0.]];
        for (op, body_count) in [
            (Op::Union, 2),
            (Op::Intersection, 0),
            (Op::Difference, 1),
            (Op::Xor, 2),
        ] {
            let (result, [lo, hi]) = proof.boolean_with_separated_shell(
                &shifted_proof, op, &disjoint_rays, 128, Default::default(), 0.02,
            ).unwrap();
            assert_eq!(result.as_ref().map_or(0, |m| m.topology().bodies.len()), body_count);
            assert_eq!(result.is_none(), body_count == 0);
            let analytical = expected * body_count as f64;
            assert!(lo <= analytical && analytical <= hi, "{op:?}: {lo}..{hi}");
        }
        assert!(proof.boolean_with_separated_shell(
            &proof, Op::Union, &disjoint_rays, 128, Default::default(), 0.02,
        ).is_err());
        let combined =
            brep_core::root_model::QualifiedRootShellPositiveFlux::assemble_disjoint_bodies_with_lineage(
                &[proof.clone(), shifted_proof],
            ).unwrap();
        let two = &combined.model;
        let total = combined.volume;
        for (id, (source, map)) in combined.sources.iter().zip(&combined.lineage).enumerate() {
            assert!(map.source_matches(source) && map.target_matches(two));
            assert_eq!(map.body(0), Some(id));
            assert_eq!(map.shell(0), Some(id));
            assert!(map.edge(0).is_some());
        }
        assert_eq!(two.topology().bodies.len(), 2);
        assert!(two.topology().bodies.iter().all(|body| body.inner_shells.is_empty()));
        assert!(total[0] <= 2. * expected && 2. * expected <= total[1]);
        let body = proof.to_body().unwrap();
        assert_eq!(body.topology().bodies.len(), 1);
        assert_eq!(body.topology().bodies[0].outer_shell, 0);
        assert!(body.topology().bodies[0].inner_shells.is_empty());
        let host = brep_core::closed_solid::qualify(
            brep_core::cuboid([-1.; 3], [2.; 3]).unwrap(), solid_policy::policy(),
        ).unwrap().into_solid().unwrap();
        let result = proof.subtract_from_closed_solid_with_lineage(
            &host, &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
            brep_core::mixed_root_model::RootCavityLimits { absolute_error: 0.02, ..Default::default() },
        ).unwrap();
        let cavity = result.model();
        let volume = result.volume();
        let [host_map, inner_map] = result.lineage();
        assert!(host_map.source_matches(&result.sources()[0]));
        assert!(inner_map.source_matches(&result.sources()[1]));
        assert!(result.lineage().iter().all(|map| map.target_matches(cavity)));
        assert_eq!(inner_map.body(0), Some(0));
        assert_eq!(inner_map.shell_orientation_reversed(0), Some(true));
        assert_eq!(inner_map.shell(0), Some(cavity.topology().bodies[0].inner_shells[0]));
        for (id, source) in result.sources()[1].topology().edges.iter().enumerate() {
            let target = &cavity.topology().edges[inner_map.edge(id).unwrap()];
            let (
                brep_core::mixed_root_model::MixedEdge::Root(source),
                brep_core::mixed_root_model::MixedEdge::Root(target),
            ) = (&source.curve, &target.curve) else {
                panic!("cavity edge lost its root curve");
            };
            assert!(Arc::ptr_eq(source.curve(), target.curve()));
        }
        assert_eq!(cavity.topology().bodies[0].inner_shells.len(), 1);
        assert!(volume[0] <= 27. - expected && 27. - expected <= volume[1]);
        let outer = brep_core::cuboid([-1.; 3], [2.; 3]).unwrap();
        let void = brep_core::cuboid([0.01, 0.25, -0.0625], [0.125, 0.75, -0.03125]).unwrap();
        let host_with_void = brep_core::minkowski3d::curved::certify_enclosed_difference(
            &outer, &void, 0,
        ).unwrap().into_model();
        let host_with_void = brep_core::closed_solid::qualify(
            host_with_void, solid_policy::policy(),
        ).unwrap().into_solid().unwrap();
        let (two_voids, remaining) = proof.subtract_from_closed_solid(
            &host_with_void,
            &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
            brep_core::mixed_root_model::RootCavityLimits { absolute_error: 0.02, ..Default::default() },
        ).unwrap();
        assert_eq!(two_voids.topology().bodies[0].inner_shells.len(), 2);
        assert!(remaining[0] <= 27. - (0.125 - 0.01) * 0.5 * 0.03125 - expected && 27. - (0.125 - 0.01) * 0.5 * 0.03125 - expected <= remaining[1]);
        // This entire box lies between the paraboloid and its planar cap.
        // Subtracting the enclosing root shell must not subtract its volume twice.
        let enclosed_void = brep_core::cuboid(
            [0.46875, 0.46875, -0.0625], [0.53125, 0.53125, -0.03125],
        ).unwrap();
        let nested_host = brep_core::minkowski3d::curved::certify_enclosed_difference(
            &outer, &enclosed_void, 0,
        ).unwrap().into_model();
        let nested_host = brep_core::closed_solid::qualify(
            nested_host, solid_policy::policy(),
        ).unwrap().into_solid().unwrap();
        let refusal = proof.subtract_from_closed_solid_with_lineage(
            &nested_host,
            &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
            brep_core::mixed_root_model::RootCavityLimits {
                absolute_error: 0.02, ..Default::default()
            },
        ).err().expect("subtraction must refuse an unproved nested cavity");
        assert!(refusal.to_string().contains("separation from existing void"));
        let mixed = brep_core::mixed_root_model::MixedBrep::from_root(&body).unwrap();
        let roundtrip = mixed.to_root().unwrap();
        assert_eq!(roundtrip.topology().bodies, body.topology().bodies);
        for (original, imported) in body.topology().edges.iter().zip(&roundtrip.topology().edges) {
            assert!(Arc::ptr_eq(original.curve.curve(), imported.curve.curve()));
        }
    } else {
        assert!(flux.cells < 14, "Zero-density face should skip subdivision");
    }
    assert!(hi - lo < 0.15, "Contour bounds should tighten flux: {lo}..{hi}");
    let coarse = charts.signed_flux(brep_core::root_model::RootShellFluxLimits {
        axis: 2, origin: 0., max_cells: 2, max_spans: 32, max_depth: 2,
        winding: nurbs_core::ss_intersection::OriginalSsWindingLimits {
            max_segments: 32, ..Default::default()
        },
    }).unwrap();
    assert_eq!(coarse.cells, 2);
    assert!(coarse.signed_bounds[1] - coarse.signed_bounds[0] >= hi - lo);
    let missing = brep_core::root_model::QualifiedRootShellContacts::qualify(
        charts, vec![], 128,
    ).unwrap_err();
    assert!(missing.to_string().contains("incident face pairs"));
    let missing_after_sew = brep_core::root_model::QualifiedRootShellPositiveFlux::from_patches(
        &[left.output().clone(), right.output().clone()], 1e-7, 64, vec![], 128,
        brep_core::root_model::RootShellFluxLimits {
            axis: 2, origin: 0., max_cells: 32, max_spans: 32, max_depth: 2,
            winding: Default::default(),
        },
    ).unwrap_err();
    assert!(missing_after_sew.to_string().contains("incident face pairs"));

    assert!(brep_core::root_model::QualifiedRootShellCharts::qualify(
        left.output().clone(), 64, Default::default(),
    ).is_err());
    sewn.reverify().unwrap();
    assert!(RootBrep::sew_root_shell(&[left.output().clone()], 1e-7).is_err());
    assert!(RootBrep::sew_root_shell(&[left.output().clone(), left.output().clone()], 1e-7).is_err());
}
