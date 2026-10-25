#[test]
fn cuboid_continuous_validation_succeeds_without_mutating_or_changing_legacy_status() {
    let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let before = model.clone();

    let report = model.validate_continuous(10000).unwrap();
    assert!(report.topology_valid);
    assert_eq!(report.geometry_agreement, "continuous_with_tolerance");
    assert_eq!(report.solid_geometry_status, "not_certified");
    assert_eq!(model, before);

    assert_eq!(
        model.validate().unwrap().geometry_agreement,
        "sampled_with_tolerance"
    );
    assert_eq!(model, before);
}

#[test]
fn continuous_validation_detects_interior_edge_mismatch() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let mut curve = model.edges[0].curve.elevate(3).unwrap();
    curve.control_points[1][2] += 0.1;
    curve.control_points[2][2] += 0.1;
    model.edges[0].curve = curve;
    model.rebuild_topology_ids();

    let error = model.validate_continuous(10000).unwrap_err();
    assert_eq!(error.code, "BREP_GEOMETRY_MISMATCH");
}

#[test]
fn continuous_validation_rejects_invalid_cell_budgets() {
    let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    for max_cells in [0, 1_000_001] {
        assert!(model.validate_continuous(max_cells).is_err());
    }
}

#[test]
fn continuous_validation_reports_tiny_budget_as_unresolved() {
    let model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let error = model.validate_continuous(1).unwrap_err();
    assert_eq!(error.code, "BREP_GEOMETRY_UNRESOLVED");
}

#[test]
fn continuous_validation_consumes_trim_and_checks_periodic_lift_metadata() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let edge_domains: Vec<_> = model.edges.iter().map(|edge| edge.curve.domain()).collect();
    for wire in &mut model.loops {
        for coedge in &mut wire.coedges {
            let trim = brep_topology::CoedgeTrim {
                curve_parameter: edge_domains[coedge.edge],
                pcurve_parameter: coedge.pcurve.domain(),
                periodic_lift: [[0, 0], [0, 0]],
            };
            *coedge = coedge.clone().with_trim(trim).unwrap();
        }
    }
    assert_eq!(
        model.validate_continuous(10000).unwrap().geometry_agreement,
        "continuous_with_tolerance"
    );
    let exact = brep_core::boundary_agreement::verify_exact(&model, 10000).unwrap();
    assert!(!exact.all_equal);
    assert!(exact.uses.iter().all(|entry| entry.decision.is_none()));

    let outer = model.faces[0].outer;
    let coedge = model.loops[outer].coedges[0].clone();
    let mut trim = coedge.trim().unwrap();
    trim.periodic_lift[0][0] = 1;
    model.loops[outer].coedges[0] = coedge.with_trim(trim).unwrap();
    let error = model.validate_continuous(10000).unwrap_err();
    assert_eq!(error.code, "BREP_TRIM_PERIODIC_LIFT");
}

#[test]
fn continuous_validation_applies_reversed_edge_sense_exactly_once() {
    let mut model = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let (wire, slot) = model
        .loops
        .iter()
        .enumerate()
        .find_map(|(wire, loop_)| {
            loop_.coedges.iter().position(|coedge| {
                coedge.reversed
                    && model.edges[coedge.edge].curve.domain() == [0., 1.]
                    && coedge.pcurve.domain() == [0., 1.]
            })
            .map(|slot| (wire, slot))
        })
        .unwrap();
    let coedge = model.loops[wire].coedges[slot].clone();
    model.loops[wire].coedges[slot] = coedge
        .with_trim(brep_topology::CoedgeTrim {
            curve_parameter: [0.1, 0.8],
            pcurve_parameter: [0.1, 0.8],
            periodic_lift: [[0, 0], [0, 0]],
        })
        .unwrap();
    assert_eq!(
        model.validate_continuous(10000).unwrap().geometry_agreement,
        "continuous_with_tolerance"
    );
}
