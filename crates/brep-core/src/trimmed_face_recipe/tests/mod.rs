use super::*;
fn limits() -> Limits {
    Limits {
        pairs: 10000,
        region_cells: 100000,
        domain_cells: 100000,
        agreement_cells: 100000,
    }
}
fn fixture() -> (Surface, Vec<Vec<Boundary>>) {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![0., 1., 1.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let arc = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., 0.5f64.sqrt(), 1.],
        periodic: false,
    };
    let uv = vec![
        arc,
        Curve::from_polyline(vec![vec![0., 1.], vec![0., 0.]]).unwrap(),
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap(),
    ];
    let wire = uv
        .into_iter()
        .map(|pcurve| {
            let mut curve = pcurve.clone();
            for p in &mut curve.control_points {
                p.push(1.);
            }
            Boundary {
                curve,
                pcurve,
                reversed: false,
            }
        })
        .collect();
    (surface, vec![wire])
}
#[test]
fn split_rational_boundary_can_be_reassembled_without_changing_surface() {
    let (surface, mut wires) = fixture();
    let context = ToleranceContext::default_valid();
    let split = split_boundary(
        &context,
        &surface,
        &wires[0][0],
        &[0.25, 0.75],
        1e-8,
        100000,
        100000,
    )
    .unwrap();
    assert!(
        split.report.qualified,
        "world={} uv={} joins={}/{} visited={}/{} lift={:?}",
        split.report.world.geometry_preserved,
        split.report.uv.geometry_preserved,
        split.report.world.endpoint_joins_exact,
        split.report.uv.endpoint_joins_exact,
        split.report.world.visited,
        split.report.uv.visited,
        split
            .report
            .pieces
            .iter()
            .map(|p| p.agreement.as_ref().map(|a| (a.status, a.cells)))
            .collect::<Vec<_>>()
    );
    let boundaries = split.boundaries.unwrap();
    assert_eq!(boundaries.len(), 3);
    wires[0].splice(0..1, boundaries);
    let r = assemble(&context, &surface, &wires, 1e-8, limits()).unwrap();
    let face = r.face.expect(r.reason);
    assert_eq!(face.edges().len(), 5);
    assert_eq!(face.face().surface, surface);
}
#[test]
fn unqualified_split_does_not_expose_usable_boundaries() {
    let (surface, mut wires) = fixture();
    wires[0][0].curve.control_points[1][2] += 0.1;
    let r = split_boundary(
        &ToleranceContext::default_valid(),
        &surface,
        &wires[0][0],
        &[0.25, 0.75],
        1e-8,
        100000,
        1,
    )
    .unwrap();
    assert!(!r.report.qualified);
    assert!(r.boundaries.is_none());
    assert_eq!(r.report.pieces.len(), 3);
}
#[test]
fn corner_replacement_preserves_support_and_kept_world_edges() {
    let (surface, wire) = fixture();
    let contact = &wire[0][0];
    let points = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
    let original = vec![(0..4)
        .map(|i| {
            let pcurve =
                Curve::from_polyline(vec![points[i].clone(), points[(i + 1) % 4].clone()])
                    .unwrap();
            let mut curve = pcurve.clone();
            for p in &mut curve.control_points {
                p.push(1.);
            }
            Boundary {
                curve,
                pcurve,
                reversed: false,
            }
        })
        .collect::<Vec<_>>()];
    let r = replace_boundary_arc(
        &ToleranceContext::default_valid(),
        &surface,
        &original,
        0,
        0,
        2,
        contact,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.contour.as_ref().unwrap().region_subset_proven);
    let result = r.replacement.unwrap();
    let face = result.face.expect(result.reason);
    assert_eq!(face.edges().len(), 3);
    assert_eq!(face.face().surface, surface);
    assert_eq!(face.edges()[1].curve, original[0][2].curve);
    assert_eq!(face.edges()[2].curve, original[0][3].curve);
    let path = [
        [vec![1., 0.], vec![0.5, 0.5]],
        [vec![0.5, 0.5], vec![0., 1.]],
    ]
    .into_iter()
    .map(|points| {
        let pcurve = Curve::from_polyline(points.to_vec()).unwrap();
        let mut curve = pcurve.clone();
        for point in &mut curve.control_points {
            point.push(1.);
        }
        Boundary {
            curve,
            pcurve,
            reversed: false,
        }
    })
    .collect::<Vec<_>>();
    let r = replace_boundary_path(
        &ToleranceContext::default_valid(),
        &surface,
        &original,
        0,
        0,
        2,
        &path,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.contour.as_ref().unwrap().region_subset_proven);
    let face = r.replacement.unwrap().face.unwrap();
    assert_eq!(face.edges().len(), 4);
    assert_eq!(face.edges()[0].curve, path[0].curve);
    assert_eq!(face.edges()[1].curve, path[1].curve);
    assert_eq!(face.edges()[2].curve, original[0][2].curve);
    let mut bad_path = path.clone();
    bad_path[1].curve.control_points[1][2] += 0.1;
    let r = replace_boundary_path(
        &ToleranceContext::default_valid(),
        &surface,
        &original,
        0,
        0,
        2,
        &bad_path,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.replacement.unwrap().face.is_none());
    let mut wrong = Boundary {
        curve: contact.curve.clone(),
        pcurve: contact.pcurve.clone(),
        reversed: false,
    };
    wrong.curve.control_points[1][2] += 0.1;
    let r = replace_boundary_arc(
        &ToleranceContext::default_valid(),
        &surface,
        &original,
        0,
        0,
        2,
        &wrong,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.contour.as_ref().unwrap().region_subset_proven);
    assert!(r.replacement.unwrap().face.is_none());
}
#[test]
fn canonical_rational_face_preserves_definitions_and_closed_world_incidence() {
    let (surface, mut wires) = fixture();
    // A reversed canonical edge retains a forward loop pcurve.
    wires[0][1].curve.control_points.reverse();
    wires[0][1].curve.weights.reverse();
    wires[0][1].reversed = true;
    let report = assemble(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        1e-8,
        limits(),
    )
    .unwrap();
    let face = report.face.as_ref().expect(report.reason);
    assert_eq!(face.vertices().len(), 3);
    assert_eq!(face.edges().len(), 3);
    assert_eq!(face.face().surface, surface);
    for (i, e) in face.edges().iter().enumerate() {
        assert_eq!(e.curve, wires[0][i].curve);
    }
    assert!(face.loops()[0].coedges[1].reversed);
    assert_eq!(face.edges()[1].vertices, [2, 1]);
    assert!(report
        .agreements
        .iter()
        .all(|r| r.status == agreement::Status::WithinTolerance));
}
#[test]
fn world_gap_and_wrong_source_lift_never_produce_a_face() {
    let (surface, mut wires) = fixture();
    wires[0][0].curve.control_points[0][0] += 1e-9;
    let r = assemble(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.face.is_none());
    assert_eq!(r.reason, "world-joins-not-identical");
    let (surface, mut wires) = fixture();
    wires[0][0].curve.control_points[1][2] += 0.1;
    let r = assemble(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        1e-8,
        limits(),
    )
    .unwrap();
    assert!(r.face.is_none());
    assert_eq!(r.reason, "face-source-agreement-unqualified");
}
#[test]
fn exhausted_region_budget_cannot_hide_malformed_world_geometry() {
    let (surface, mut wires) = fixture();
    wires[0][2].curve.weights[1] = -1.;
    let work = Limits {
        pairs: 1,
        region_cells: 1,
        domain_cells: 1,
        agreement_cells: 1,
    };
    assert!(assemble(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        1e-8,
        work
    )
    .is_err());
}
#[test]
fn source_endpoint_partitions_feed_face_replacement_with_reversed_world_edge() {
    let (surface, _) = fixture();
    let points = [vec![1., 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]];
    let make = |points: Vec<Vec<f64>>| {
        let pcurve = Curve::from_polyline(points).unwrap();
        let mut curve = pcurve.clone();
        for p in &mut curve.control_points {
            p.push(1.);
        }
        Boundary {
            curve,
            pcurve,
            reversed: false,
        }
    };
    let mut wires = vec![(0..4)
        .map(|i| make(vec![points[i].clone(), points[(i + 1) % 4].clone()]))
        .collect::<Vec<_>>()];
    wires[0][0].curve.control_points.reverse();
    wires[0][0].reversed = true;
    let contact = make(vec![vec![1., 0.5], vec![0.5, 1.]]);
    let r = partition_and_replace_path(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        0,
        0,
        0.5,
        1,
        0.5,
        &[contact.clone()],
        1e-8,
        100000,
        limits(),
    )
    .unwrap();
    assert!(r.splits.iter().all(|s| s.report.qualified));
    let face = r.replacement.unwrap().replacement.unwrap().face.unwrap();
    assert_eq!(face.edges().len(), 5);
    assert_eq!(face.face().surface, surface);
    assert_eq!(face.edges()[0].curve, contact.curve);
    assert_eq!(face.edges()[2].curve, wires[0][2].curve);
    let wrapped = make(vec![vec![0.5, 0.], vec![1., 0.5]]);
    let r = partition_and_replace_path(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        0,
        3,
        0.5,
        0,
        0.5,
        &[wrapped.clone()],
        1e-8,
        100000,
        limits(),
    )
    .unwrap();
    let face = r.replacement.unwrap().replacement.unwrap().face.unwrap();
    assert_eq!(face.edges().len(), 5);
    assert_eq!(face.edges()[0].curve, wrapped.curve);
    let mut wrong = contact;
    wrong.curve.control_points[0][0] -= 1e-9;
    let r = partition_and_replace_path(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        0,
        0,
        0.5,
        1,
        0.5,
        &[wrong],
        1e-8,
        100000,
        limits(),
    )
    .unwrap();
    assert!(r.replacement.unwrap().replacement.unwrap().face.is_none());
}
#[test]
fn source_face_crossing_search_retains_original_addresses_and_shared_budget() {
    let (surface, wires) = fixture();
    let contact = Curve::from_polyline(vec![vec![-0.2, 0.3], vec![1.2, 0.3]]).unwrap();
    let r = locate_contact_boundaries(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        &contact,
        1e-8,
        limits(),
        10000,
        [1e-7, 1e-7],
    )
    .unwrap();
    assert!(r.complete && r.precision_proven);
    assert_eq!(r.crossings.len(), 3);
    let roots = r
        .crossings
        .iter()
        .flat_map(|c| {
            c.report
                .cells
                .iter()
                .filter_map(move |cell| cell.root.map(|root| (c.boundary_index, root)))
        })
        .collect::<Vec<_>>();
    assert_eq!(roots.len(), 2);
    let mapped = locate_source_contact_points(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        &contact,
        1e-8,
        limits(),
        10000,
        [1e-7, 1e-7],
        16,
        16,
    )
    .unwrap();
    assert!(mapped.complete);
    assert_eq!(mapped.points.len(), 2);
    assert_eq!(mapped.point_checks, 2);
    assert_eq!(mapped.mapping_cells, 2);
    assert!(mapped
        .points
        .iter()
        .all(|p| p.report.as_ref().unwrap().point.is_some()));
    let stopped = locate_source_contact_points(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        &contact,
        1e-8,
        limits(),
        10000,
        [1e-7, 1e-7],
        16,
        1,
    )
    .unwrap();
    assert!(!stopped.complete);
    assert_eq!(stopped.points.len(), 2);
    assert_eq!(stopped.mapping_cells, 1);
    assert!(stopped.points[1].report.is_none());

    assert!(roots.iter().any(|x| x.0 == 0));
    assert!(roots.iter().any(|x| x.0 == 1));
    let r = locate_contact_boundaries(
        &ToleranceContext::default_valid(),
        &surface,
        &wires,
        &contact,
        1e-8,
        limits(),
        1,
        [1e-7, 1e-7],
    )
    .unwrap();
    assert!(!r.complete && !r.precision_proven);
    assert_eq!(r.visited, 1);
    assert_eq!(r.crossings.len(), 3);
    assert!(r.crossings[1..]
        .iter()
        .all(|c| c.report.visited == 0 && !c.report.complete));
}
