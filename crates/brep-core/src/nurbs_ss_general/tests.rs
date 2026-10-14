use super::*;
use nurbs_core::surface::Surface;

fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 1., z]],
            vec![vec![1., 0., z], vec![1., 1., z]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]

#[cfg(feature = "codec")]
fn general_ss_branch_graph_without_graph_patch() {
    let xy = plane(0.);
    let xz = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 1.]],
            vec![vec![1., 0., 0.], vec![1., 0., 1.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let report = nurbs_core::ss_intersection::intersect_surface_surface(&xy, &xz, None).unwrap();
    let context = ToleranceContext::default_valid();
    let graph = branch_graph_from_ss_report(&report, &context).unwrap();
    let native =
        nurbs_core::ss_intersection::intersect_surface_surface_report(&xy, &xz, None).unwrap();
    assert_native_wire_parity(&native);
    assert!(graph.complete);
    assert!(!graph.boolean_mutation_authority);
    // Query-grade topology flag may be true only when complete; Boolean stays false.
    let audit = verify_general_ss_branch_graph(&graph, &context).unwrap();
    assert!(audit.notes.contains(&"no_graph_patch_iso_fixture"));
    let traces = rational_traces_from_ss_report(&report).unwrap();
    assert!(!traces.is_empty());
}


fn assert_native_wire_parity(report: &SurfaceSurfaceIntersection) {
    let context = &report.tolerance;
    let native = branch_graph_from_surface_intersection(report, context).unwrap();
    verify_general_ss_branch_graph(&native, context).unwrap();
    #[cfg(feature = "codec")]
    {
    let wire = value_codec::to_value(report).unwrap();
    let decoded = branch_graph_from_ss_report(&wire, context).unwrap();
    assert_eq!(native, decoded);
    assert_eq!(
        rational_traces_from_branch_graph(&native),
        rational_traces_from_ss_report(&wire).unwrap()
    );
    assert_eq!(
        native.permits_topology_authorship(),
        decoded.permits_topology_authorship()
    );
    }
}
#[test]
fn native_and_wire_paths_agree_for_overlap_and_separated_surfaces() {
    let first = plane(0.);
    for second in [plane(0.), plane(2.)] {
        let report =
            nurbs_core::ss_intersection::intersect_surface_surface_report(&first, &second, None)
                .unwrap();
        assert_native_wire_parity(&report);
    }
}
#[test]
fn native_and_wire_paths_agree_for_continuation_tangency_and_unresolved_coverage() {
    use nurbs_core::ss_intersection::{
        ContinuationSample, ContinuedBranch, EndpointLocation, TangencyBranch,
        UnresolvedSurfaceIntersection,
    };
    let trim = brep_topology::CoedgeTrim {
        curve_parameter: [0., 1.],
        pcurve_parameter: [0., 1.],
        periodic_lift: [[0, 0], [0, 0]],
    };
    let sample = |t| ContinuationSample {
        point: [t, 0., 0.],
        uv_first: [t, 0.],
        uv_second: [0., t],
        parameter: t,
        seam_wrap: None,
    };
    let mut report = SurfaceSurfaceIntersection {
        components: vec![
            SurfaceSurfaceComponent::Continued(ContinuedBranch {
                closed: true,
                junction: true,
                contact: ContactClass::Transverse,
                samples: vec![sample(0.), sample(1.)],
                first_uv: [0., 0.],
                last_uv: [1., 0.],
                first_st: [0., 0.],
                last_st: [0., 1.],
                location: EndpointLocation::Closed,
                geometry_enclosure: [[0., 1.]; 3],
                coedge_trim: trim,
            }),
            SurfaceSurfaceComponent::Tangency(TangencyBranch {
                contact: ContactClass::EvenTangency,
                sample: sample(0.5),
                geometry_enclosure: [[0., 1.]; 3],
                coedge_trim: trim,
                junction: true,
            }),
        ],
        unresolved: vec![],
        boxes_visited: 1,
        bernstein_excluded: 0,
        krawczyk_isolated: 1,
        tolerance: ToleranceContext::default_valid(),
    };
    assert_native_wire_parity(&report);
    report.unresolved.push(UnresolvedSurfaceIntersection {
        parameter_box: None,
        reason: nurbs_core::intersection::UnresolvedReason::ResourceBoundary,
        classification: None,
    });
    assert_native_wire_parity(&report);
    let graph = branch_graph_from_surface_intersection(&report, &report.tolerance).unwrap();
    assert!(!graph.permits_topology_authorship());
    report.unresolved.clear();
    let SurfaceSurfaceComponent::Continued(branch) = &mut report.components[0] else {
        unreachable!()
    };
    branch.contact = ContactClass::PoleOrSingular;
    assert_native_wire_parity(&report);
    assert!(
        !branch_graph_from_surface_intersection(&report, &report.tolerance)
            .unwrap()
            .permits_topology_authorship()
    );
}
#[test]
fn native_graph_refuses_a_different_tolerance_context() {
    let report =
        nurbs_core::ss_intersection::intersect_surface_surface_report(&plane(0.), &plane(0.), None)
            .unwrap();
    let mut spec = report.tolerance.specification().clone();
    spec.policy.push_str("-different");
    let other = ToleranceContext::new(spec).unwrap();
    assert!(branch_graph_from_surface_intersection(&report, &other).is_err());
}
#[test]

#[cfg(feature = "codec")]
fn legacy_trace_adapter_retains_empty_sample_and_headerless_line_behavior() {
    let mut report = value_codec::json!({"components":[{"kind":"empty"},{"kind":"curve","pcurveFirst":{"kind":"line","start":[0,0],"end":[1,1],"correspondence":"exact_affine"}}]});
    let traces = rational_traces_from_ss_report(&report).unwrap();
    assert_eq!(traces.len(), 1);
    assert_eq!(traces[0].edge_id, 2);
    report["components"][1]["samples"] = value_codec::json!([]);
    assert!(rational_traces_from_ss_report(&report).unwrap().is_empty());
    report["components"][1]["pcurveFirst"]["start"] = value_codec::json!([0]);
    assert!(rational_traces_from_ss_report(&report).is_err());
}
