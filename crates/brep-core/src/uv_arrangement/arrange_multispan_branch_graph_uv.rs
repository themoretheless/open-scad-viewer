use super::*;

fn validate_breaks(breaks: &[f64]) -> Result<()> {
    if breaks.len() < 2
        || breaks.iter().any(|value| !value.is_finite())
        || breaks.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(refuse(
            "Tensor knot partition must be finite and strictly increasing",
        ));
    }
    Ok(())
}

/// Arrange multiple disjoint certified branches over all tensor knot cells.
/// Branches are required to cover the complete chart in their varying
/// parameter. Duplicate, crossing, partial, missed, or certificate-mutated
/// branches refuse before DCEL construction.
pub fn arrange_multispan_branch_graph_uv(
    context: &ToleranceContext,
    knot_breaks: [&[f64]; 2],
    graph: &nurbs_intersect::nurbs_ss::BranchGraph,
    support_index: usize,
    resource_limit: usize,
) -> Result<MultiSpanUvArrangement> {
    validate_breaks(knot_breaks[0])?;
    validate_breaks(knot_breaks[1])?;
    if support_index > 1
        || !graph.permits_topology_authorship()
        || graph.certificate.context != context.spec_identity()
    {
        return Err(refuse(
            "Tensor arrangement requires an intact context-bound BranchGraph",
        ));
    }
    let domain = [
        [knot_breaks[0][0], *knot_breaks[0].last().unwrap()],
        [knot_breaks[1][0], *knot_breaks[1].last().unwrap()],
    ];
    let tensor_cell_count = (knot_breaks[0].len() - 1)
        .checked_mul(knot_breaks[1].len() - 1)
        .ok_or_else(|| Error::new("BREP_TRIM_RESOURCE_LIMIT", "Tensor cell count overflow"))?;
    if tensor_cell_count > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Tensor cell budget exceeded",
        ));
    }
    let mut branch_keys = Vec::new();
    for component in &graph.components {
        if component.closed || component.fragments.is_empty() {
            return Err(refuse(
                "Closed or empty branch needs periodic topology outside this tensor chart",
            ));
        }
        let mut traces = component
            .fragments
            .iter()
            .map(|fragment| exact_line_trace(&fragment.pcurves[support_index]))
            .collect::<Result<Vec<_>>>()?;
        let axis = traces[0].0;
        let fixed = traces[0].1;
        if traces
            .iter()
            .any(|trace| trace.0 != axis || trace.1.to_bits() != fixed.to_bits())
        {
            return Err(refuse(
                "Joined component changed iso axis or fixed parameter",
            ));
        }
        for trace in &mut traces {
            if trace.2[0] > trace.2[1] {
                trace.2.reverse();
            }
        }
        traces.sort_by(|a, b| a.2[0].total_cmp(&b.2[0]));
        let varying_domain = match axis {
            TensorAxis::U => domain[1],
            TensorAxis::V => domain[0],
        };
        if traces[0].2[0].to_bits() != varying_domain[0].to_bits()
            || traces.last().unwrap().2[1].to_bits() != varying_domain[1].to_bits()
            || traces
                .windows(2)
                .any(|pair| pair[0].2[1].to_bits() != pair[1].2[0].to_bits())
        {
            return Err(refuse(
                "Certified branch has missed or duplicate tensor-cell coverage",
            ));
        }
        branch_keys.push((axis, fixed.to_bits(), component.component_id));
    }
    branch_keys.sort();
    if branch_keys
        .windows(2)
        .any(|pair| pair[0].0 == pair[1].0 && pair[0].1 == pair[1].1)
    {
        return Err(refuse("Duplicate certified tensor branch"));
    }
    if branch_keys
        .iter()
        .enumerate()
        .any(|(i, branch)| branch_keys[i + 1..].iter().any(|other| branch.0 != other.0))
    {
        return Err(refuse("Certified tensor branches cross inside the chart"));
    }
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
    let mut edge_id = 4;
    for fixed in &knot_breaks[0][1..knot_breaks[0].len() - 1] {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: TensorAxis::U,
                fixed: *fixed,
                interval: domain[1],
            },
        });
        edge_id += 1;
    }
    for fixed in &knot_breaks[1][1..knot_breaks[1].len() - 1] {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: TensorAxis::V,
                fixed: *fixed,
                interval: domain[0],
            },
        });
        edge_id += 1;
    }
    for (axis, fixed, _) in &branch_keys {
        primitives.push(LiftedUvPrimitive {
            edge_id,
            geometry: LiftedUvGeometry::TensorIsoLine {
                axis: *axis,
                fixed: f64::from_bits(*fixed),
                interval: match axis {
                    TensorAxis::U => domain[1],
                    TensorAxis::V => domain[0],
                },
            },
        });
        edge_id += 1;
    }
    if primitives.len() > resource_limit {
        return Err(Error::new(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Global tensor primitive budget exceeded",
        ));
    }
    let expected_iso = primitives.len() - 4;
    let arrangement =
        arrange_tensor_bezier_graph_uv(context, domain, &primitives, expected_iso, resource_limit)?;
    let material_cell_count = arrangement
        .cells
        .iter()
        .filter(|cell| matches!(cell.label, WindingLabel::Material(_)))
        .count();
    let authority = MultiSpanUvAuthority {
        context: context.spec_identity(),
        knot_bits: [
            knot_breaks[0].iter().map(|value| value.to_bits()).collect(),
            knot_breaks[1].iter().map(|value| value.to_bits()).collect(),
        ],
        branch_keys,
        primitive_count: primitives.len(),
        cell_count: arrangement.cells.len(),
        tensor_cell_count,
        material_cell_count,
        hole_cell_count: 0,
    };
    let result = MultiSpanUvArrangement {
        tensor_cell_count,
        branch_count: authority.branch_keys.len(),
        material_cell_count,
        hole_cell_count: 0,
        global_coverage_complete: true,
        context: context.spec_identity(),
        arrangement,
        authority,
    };
    if !result.permits_trim_classification() {
        return Err(refuse(
            "Global tensor arrangement failed its authority check",
        ));
    }
    Ok(result)
}

/// Build a UV arrangement from imprint curves on a frozen / admitted chart.
pub fn arrange_imprint_curves(
    chart: ChartKind,
    curves: &[UvImprintCurve],
) -> Result<UvArrangement> {
    if curves.len() > 256 {
        return Err(refuse("UV imprint curve budget exceeded"));
    }
    let mut events = Vec::new();
    let mut hole_count = 0usize;
    let mut expanded_for_nesting: Vec<UvImprintCurve> = Vec::new();
    for curve in curves {
        if curve.chart != chart {
            return Err(refuse("Imprint curve chart kind mismatch"));
        }
        if curve.periodic && chart != ChartKind::AnalyticCircle {
            return Err(refuse(
                "Periodic UV wrap is only admitted on AnalyticCircle charts",
            ));
        }
        if chart == ChartKind::Freeform {
            return Err(refuse(
                "Interval-only Freeform input cannot publish Complete coverage",
            ));
        }
        let material = if curve.periodic {
            expand_periodic_intervals(&curve.parameter_intervals, std::f64::consts::TAU)?
        } else {
            curve.parameter_intervals.clone()
        };
        let holes = if curve.periodic {
            expand_periodic_intervals(&curve.hole_intervals, std::f64::consts::TAU)?
        } else {
            curve.hole_intervals.clone()
        };
        expanded_for_nesting.push(UvImprintCurve {
            chart: curve.chart,
            parameter_intervals: material.clone(),
            hole_intervals: holes.clone(),
            edge_id: curve.edge_id,
            periodic: false,
        });
        for interval in &material {
            if !interval[0].is_finite() || !interval[1].is_finite() || interval[1] < interval[0] {
                return Err(refuse("Imprint interval must be finite and ordered"));
            }
            events.push(ChartEvent {
                parameter: interval[0],
                kind: "enter",
                edge: curve.edge_id,
            });
            events.push(ChartEvent {
                parameter: interval[1],
                kind: "exit",
                edge: curve.edge_id,
            });
        }
        for hole in &holes {
            if !hole[0].is_finite() || !hole[1].is_finite() || hole[1] < hole[0] {
                return Err(refuse("Hole interval must be finite and ordered"));
            }
            hole_count += 1;
            events.push(ChartEvent {
                parameter: hole[0],
                kind: "hole_enter",
                edge: curve.edge_id.saturating_add(10_000),
            });
            events.push(ChartEvent {
                parameter: hole[1],
                kind: "hole_exit",
                edge: curve.edge_id.saturating_add(10_000),
            });
        }
    }
    let nesting = compute_monotone_nesting(&expanded_for_nesting)?;
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    let mut samples = Vec::new();
    if events.is_empty() {
        if chart == ChartKind::Freeform {
            return Err(refuse(
                "Empty freeform arrangement cannot publish Complete classification",
            ));
        }
        samples.push((0.5, CellLabel::Outside));
    } else {
        let first = events[0].parameter;
        if first > 0. {
            samples.push((first * 0.5, CellLabel::Outside));
        }
        for window in events.windows(2) {
            samples.push((window[0].parameter, CellLabel::Boundary));
            let mid = 0.5 * (window[0].parameter + window[1].parameter);
            if (mid - window[0].parameter).abs() > 1e-12 {
                let label = if window[0].kind.starts_with("hole") {
                    CellLabel::Outside
                } else {
                    CellLabel::Inside
                };
                samples.push((mid, label));
            }
        }
        if let Some(last) = events.last() {
            samples.push((last.parameter, CellLabel::Boundary));
            samples.push((last.parameter + 0.5, CellLabel::Outside));
        }
    }
    let classification = classify_chart_events(chart, events.clone(), &samples)?;
    Ok(UvArrangement {
        chart,
        events,
        classification,
        hole_count,
        nesting,
    })
}

/// Missed-branch mutation: dropping any imprint edge's events must refuse Complete.
pub fn assert_missed_branch_detected(arrangement: &UvArrangement) -> Result<()> {
    if arrangement.events.is_empty() {
        return Ok(());
    }
    let dropped_edge = arrangement.events.last().map(|e| e.edge).unwrap();
    let truncated: Vec<_> = arrangement
        .events
        .iter()
        .filter(|e| e.edge != dropped_edge)
        .cloned()
        .collect();
    // Replay the original samples against the truncated event set. Boundary
    // labels that sat only on the dropped edge must no longer isolate → refuse.
    let samples: Vec<(f64, CellLabel)> = arrangement
        .classification
        .cells
        .iter()
        .map(|(_lo, p, label)| (*p, *label))
        .collect();
    match classify_chart_events(arrangement.chart, truncated, &samples) {
        Err(_) => Ok(()),
        Ok(cert) if !cert.complete => Ok(()),
        Ok(_) => Err(refuse(
            "Missed-branch mutation did not invalidate classification completeness",
        )),
    }
}

/// Exact sew of a model with transactional rollback on refusal.
pub fn sew_model_atomic(model: &Model) -> Result<(Model, SewCertificate)> {
    model.validate()?;
    let certificate = sew_closed_model_edges(model)?;
    Ok((model.clone(), certificate))
}
