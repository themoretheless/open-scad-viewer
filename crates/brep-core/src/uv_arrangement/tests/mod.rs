use super::*;
use crate::cuboid;

#[test]
fn imprint_curves_classify_with_complete_certificate() {
    let curves = [UvImprintCurve {
        chart: ChartKind::PlanePoly,
        parameter_intervals: vec![[0.2, 0.8]],
        hole_intervals: vec![],
        edge_id: 0,
        periodic: false,
    }];
    let arr = arrange_imprint_curves(ChartKind::PlanePoly, &curves).unwrap();
    assert!(arr.classification.complete);
    assert_missed_branch_detected(&arr).unwrap();
}

#[test]
fn empty_arrangement_is_outside() {
    let arr = arrange_imprint_curves(ChartKind::AnalyticCircle, &[]).unwrap();
    assert!(arr.classification.complete);
    assert!(
        arr.classification
            .cells
            .iter()
            .any(|c| c.2 == CellLabel::Outside)
    );
}

#[test]
fn monotone_nesting_and_multiple_holes() {
    let curves = [
        UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.1, 0.9]],
            hole_intervals: vec![[0.3, 0.4], [0.6, 0.7]],
            edge_id: 1,
            periodic: false,
        },
        UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.2, 0.5]],
            hole_intervals: vec![],
            edge_id: 2,
            periodic: false,
        },
    ];
    let arr = arrange_imprint_curves(ChartKind::PlanePoly, &curves).unwrap();
    assert!(arr.classification.complete);
    assert_eq!(arr.hole_count, 2);
    assert!(!arr.nesting.is_empty());
    assert!(arr.nesting.iter().any(|n| n.nested_edges.contains(&2)));
    assert_missed_branch_detected(&arr).unwrap();
}

#[test]
fn non_monotone_overlap_refuses() {
    let curves = [
        UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.1, 0.5]],
            hole_intervals: vec![],
            edge_id: 1,
            periodic: false,
        },
        UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.3, 0.8]],
            hole_intervals: vec![],
            edge_id: 2,
            periodic: false,
        },
    ];
    assert_eq!(
        arrange_imprint_curves(ChartKind::PlanePoly, &curves)
            .unwrap_err()
            .code,
        "BREP_UV_ARRANGEMENT_REFUSED"
    );
}

#[test]
fn periodic_wrap_across_seam_expands() {
    let curves = [UvImprintCurve {
        chart: ChartKind::AnalyticCircle,
        // Wrap: from 5.5 through TAU to 0.4
        parameter_intervals: vec![[5.5, 0.4]],
        hole_intervals: vec![],
        edge_id: 3,
        periodic: true,
    }];
    let arr = arrange_imprint_curves(ChartKind::AnalyticCircle, &curves).unwrap();
    assert!(arr.classification.complete);
    assert!(arr.events.len() >= 4);
    assert_missed_branch_detected(&arr).unwrap();
}

#[test]
fn interval_only_freeform_chart_touch_refuses_complete() {
    let curves = [UvImprintCurve {
        chart: ChartKind::Freeform,
        parameter_intervals: vec![[0.15, 0.85]],
        hole_intervals: vec![[0.4, 0.55]],
        edge_id: 7,
        periodic: false,
    }];
    assert!(arrange_imprint_curves(ChartKind::Freeform, &curves).is_err());
}

#[test]
fn freeform_empty_refuses() {
    assert!(arrange_imprint_curves(ChartKind::Freeform, &[]).is_err());
}

#[test]
fn plane_segments_build_context_bound_dcel_and_detect_missed_branch() {
    let context = ToleranceContext::default_valid();
    let points = [[0., 0.], [2., 0.], [2., 1.], [0., 1.]];
    let primitives = (0..4)
        .map(|edge_id| LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::PlaneSegment {
                start: points[edge_id],
                end: points[(edge_id + 1) % 4],
            },
        })
        .collect::<Vec<_>>();
    let arrangement =
        arrange_lifted_uv(&context, ChartKind::PlanePoly, &primitives, 16).unwrap();
    assert!(arrangement.coverage.complete);
    assert_eq!(arrangement.vertices.len(), 4);
    assert_eq!(arrangement.halfedges.len(), 8);
    crate::uv_coverage::verify_lifted_uv_arrangement_coverage(&arrangement, &context)
        .unwrap();

    let mut missed = arrangement.clone();
    missed.halfedges.pop();
    assert!(
        crate::uv_coverage::verify_lifted_uv_arrangement_coverage(&missed, &context)
            .is_err()
    );
}

fn tensor_primitives() -> Vec<LiftedUvPrimitive> {
    let domain = [[0., 1.], [0., 1.]];
    let mut primitives = [
        TensorBoundary::UMin,
        TensorBoundary::UMax,
        TensorBoundary::VMin,
        TensorBoundary::VMax,
    ]
    .into_iter()
    .enumerate()
    .map(|(edge_id, side)| LiftedUvPrimitive {
        edge_id,
        geometry: LiftedUvGeometry::TensorRectangleBoundary { side, domain },
    })
    .collect::<Vec<_>>();
    primitives.push(LiftedUvPrimitive {
        edge_id: 4,
        geometry: LiftedUvGeometry::TensorIsoLine {
            axis: TensorAxis::U,
            fixed: 0.5,
            interval: [0., 1.],
        },
    });
    primitives
}

#[test]
fn tensor_rectangle_iso_chart_builds_bounded_dcel() {
    let context = ToleranceContext::default_valid();
    let arrangement = arrange_tensor_bezier_graph_uv(
        &context,
        [[0., 1.], [0., 1.]],
        &tensor_primitives(),
        1,
        16,
    )
    .unwrap();
    assert_eq!(arrangement.chart, ChartKind::TensorBezierGraph);
    assert!(arrangement.coverage.complete);
    assert_eq!(arrangement.coverage.primitive_count, 5);
    crate::uv_coverage::verify_lifted_uv_arrangement_coverage(&arrangement, &context)
        .unwrap();
}

#[test]
fn tensor_chart_refuses_diagonal_missed_branch_and_resource_mutations() {
    let context = ToleranceContext::default_valid();
    let mut missed = tensor_primitives();
    missed.pop();
    assert!(
        arrange_tensor_bezier_graph_uv(&context, [[0., 1.], [0., 1.]], &missed, 1, 16).is_err()
    );
    let mut partial = tensor_primitives();
    if let LiftedUvGeometry::TensorIsoLine { interval, .. } =
        &mut partial.last_mut().unwrap().geometry
    {
        *interval = [0.1, 0.9];
    }
    assert!(
        arrange_tensor_bezier_graph_uv(&context, [[0., 1.], [0., 1.]], &partial, 1, 16)
            .is_err()
    );
    assert_eq!(
        arrange_tensor_bezier_graph_uv(
            &context,
            [[0., 1.], [0., 1.]],
            &tensor_primitives(),
            1,
            4
        )
        .unwrap_err()
        .code,
        "BREP_TRIM_RESOURCE_LIMIT"
    );
    assert!(
        arrange_lifted_uv(
            &context,
            ChartKind::TensorBezierGraph,
            &[LiftedUvPrimitive {
                edge_id: 9,
                geometry: LiftedUvGeometry::PlaneSegment {
                    start: [0., 0.],
                    end: [1., 1.]
                }
            }],
            8
        )
        .is_err()
    );
}

#[test]
fn lifted_uv_context_overlap_and_resource_mutations_refuse() {
    let context = ToleranceContext::default_valid();
    let overlapping = [
        LiftedUvPrimitive {
            edge_id: 1,
            geometry: LiftedUvGeometry::PlaneSegment {
                start: [0., 0.],
                end: [2., 0.],
            },
        },
        LiftedUvPrimitive {
            edge_id: 2,
            geometry: LiftedUvGeometry::PlaneSegment {
                start: [1., 0.],
                end: [3., 0.],
            },
        },
    ];
    assert!(arrange_lifted_uv(&context, ChartKind::PlanePoly, &overlapping, 8).is_err());
    assert_eq!(
        arrange_lifted_uv(&context, ChartKind::PlanePoly, &overlapping[..1], 0)
            .unwrap_err()
            .code,
        "BREP_TRIM_RESOURCE_LIMIT"
    );

    let square = [
        ([0., 0.], [1., 0.]),
        ([1., 0.], [1., 1.]),
        ([1., 1.], [0., 1.]),
        ([0., 1.], [0., 0.]),
    ]
    .into_iter()
    .enumerate()
    .map(|(edge_id, (start, end))| LiftedUvPrimitive {
        edge_id,
        geometry: LiftedUvGeometry::PlaneSegment { start, end },
    })
    .collect::<Vec<_>>();
    let arrangement = arrange_lifted_uv(&context, ChartKind::PlanePoly, &square, 8).unwrap();
    let mut spec = context.specification().clone();
    spec.policy = "foreign-uv-context".into();
    let foreign = ToleranceContext::new(spec).unwrap();
    assert!(
        crate::uv_coverage::verify_lifted_uv_arrangement_coverage(&arrangement, &foreign)
            .is_err()
    );
}

#[test]
fn periodic_circle_arcs_close_across_lifted_seam() {
    let context = ToleranceContext::default_valid();
    let quarter = std::f64::consts::FRAC_PI_2;
    let arcs = (0..4)
        .map(|edge_id| LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::AnalyticCircleArc {
                center: [0., 0.],
                radius: 2.,
                interval: [edge_id as f64 * quarter, (edge_id + 1) as f64 * quarter],
            },
        })
        .collect::<Vec<_>>();
    let arrangement =
        arrange_lifted_uv(&context, ChartKind::AnalyticCircle, &arcs, 16).unwrap();
    assert_eq!(arrangement.vertices.len(), 4);
    assert!(
        arrangement
            .vertices
            .iter()
            .any(|vertex| matches!(vertex.origin, UvVertexOrigin::PeriodicSeam { .. }))
    );
    assert!(
        arrangement
            .cells
            .iter()
            .any(|cell| cell.label == WindingLabel::Material(1))
    );
}

fn ss_wave(swap: bool) -> nurbs_core::surface::Surface {
    let mut control_points = vec![vec![vec![0.; 3]; 3]; 3];
    for (u, row) in control_points.iter_mut().enumerate().take(3) {
        for (v, point) in row.iter_mut().enumerate().take(3) {
            let signed = if (if swap { v } else { u }) % 2 == 0 {
                -1.
            } else {
                1.
            };
            *point = vec![signed, u as f64, v as f64];
        }
    }
    nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 2.],
        knots_v: vec![0., 0., 1., 2., 2.],
        control_points,
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn ss_plane() -> nurbs_core::surface::Surface {
    nurbs_core::surface::Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0., 2.]],
            vec![vec![0., 2., 0.], vec![0., 2., 2.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn multispan_branch_graph_builds_global_tensor_cells() {
    let context = ToleranceContext::default_valid();
    let graph = crate::nurbs_ss_g6::certify_multispan_ss(
        &ss_wave(false),
        &ss_plane(),
        [1, 2],
        &context,
        32,
    )
    .unwrap();
    let arrangement = arrange_multispan_branch_graph_uv(
        &context,
        [&[0., 1., 2.], &[0., 1., 2.]],
        &graph,
        0,
        32,
    )
    .unwrap();
    assert!(arrangement.permits_trim_classification());
    assert_eq!(arrangement.tensor_cell_count, 4);
    assert_eq!(arrangement.branch_count, 2);
    assert_eq!(arrangement.hole_cell_count, 0);
    assert!(arrangement.material_cell_count >= 6);
    crate::uv_coverage::verify_lifted_uv_arrangement_coverage(
        &arrangement.arrangement,
        &context,
    )
    .unwrap();
}

#[test]
fn multispan_uv_rejects_missed_crossing_resource_and_certificate_mutations() {
    let context = ToleranceContext::default_valid();
    let graph = crate::nurbs_ss_g6::certify_multispan_ss(
        &ss_wave(false),
        &ss_plane(),
        [1, 2],
        &context,
        32,
    )
    .unwrap();
    assert_eq!(
        arrange_multispan_branch_graph_uv(
            &context,
            [&[0., 1., 2.], &[0., 1., 2.]],
            &graph,
            0,
            4
        )
        .unwrap_err()
        .code,
        "BREP_TRIM_RESOURCE_LIMIT"
    );
    let mut missed = graph.clone();
    missed.components[0].fragments.pop();
    assert!(
        arrange_multispan_branch_graph_uv(
            &context,
            [&[0., 1., 2.], &[0., 1., 2.]],
            &missed,
            0,
            32
        )
        .is_err()
    );

    let vertical = crate::nurbs_ss_g6::certify_multispan_ss(
        &ss_wave(true),
        &ss_plane(),
        [3, 4],
        &context,
        32,
    )
    .unwrap();
    let fragments = graph
        .components
        .iter()
        .chain(&vertical.components)
        .flat_map(|component| component.fragments.clone())
        .collect();
    let crossing = crate::nurbs_ss_g6::join_certified_multispan_fragments(
        fragments,
        &context,
        [8, 2],
        16,
        1.,
        32,
    )
    .unwrap();
    assert!(
        arrange_multispan_branch_graph_uv(
            &context,
            [&[0., 1., 2.], &[0., 1., 2.]],
            &crossing,
            0,
            32
        )
        .is_err()
    );

    let arrangement = arrange_multispan_branch_graph_uv(
        &context,
        [&[0., 1., 2.], &[0., 1., 2.]],
        &graph,
        0,
        32,
    )
    .unwrap();
    let mut duplicate_mutation = arrangement.clone();
    duplicate_mutation.branch_count += 1;
    assert!(!duplicate_mutation.permits_trim_classification());
    let mut coverage_mutation = arrangement;
    coverage_mutation.global_coverage_complete = false;
    assert!(!coverage_mutation.permits_trim_classification());
}

#[test]
fn cuboid_sew_atomic_preserves_model_on_success_or_typed_refuse() {
    let model = cuboid([0.; 3], [1.; 3]).unwrap();
    match sew_model_atomic(&model) {
        Ok((out, cert)) => {
            assert!(cert.complete);
            out.validate().unwrap();
        }
        Err(err) => {
            assert!(
                err.code == "BREP_UV_ARRANGEMENT_REFUSED" || err.code.starts_with("BREP_SEW_")
            );
            model.validate().unwrap();
        }
    }
}
