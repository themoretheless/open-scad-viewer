use super::*;

/// Classification from imprint circle strata on an AnalyticCircle chart.
pub fn classify_imprint_circle_events(
    mut events: Vec<ChartEvent>,
) -> Result<ClassificationCertificate> {
    if events.is_empty() {
        return classify_chart_events(
            ChartKind::AnalyticCircle,
            vec![],
            &[(0.5, CellLabel::Outside)],
        );
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    // Deduplicate near-equal parameters into Boundary strata.
    let mut unique = Vec::new();
    for event in events {
        if unique
            .last()
            .is_some_and(|e: &ChartEvent| (e.parameter - event.parameter).abs() <= 1e-12)
        {
            continue;
        }
        unique.push(event);
    }
    let mut samples = Vec::new();
    for (i, event) in unique.iter().enumerate() {
        samples.push((event.parameter, CellLabel::Boundary));
        let next = unique
            .get(i + 1)
            .map(|e| e.parameter)
            .unwrap_or(event.parameter + 1.);
        let mid = 0.5 * (event.parameter + next);
        if (mid - event.parameter).abs() > 1e-12 {
            samples.push((mid, CellLabel::Inside));
        }
    }
    if let Some(first) = unique.first()
        && first.parameter > 0.
    {
        samples.insert(0, (first.parameter * 0.5, CellLabel::Outside));
    }
    classify_chart_events(ChartKind::AnalyticCircle, unique, &samples)
}

/// Build chart events from an authored face outer loop (parameter along coedges).
pub fn classify_face_outer_loop(
    model: &crate::Model,
    face_index: usize,
    chart: ChartKind,
) -> Result<ClassificationCertificate> {
    model.validate()?;
    let face = model.faces.get(face_index).ok_or_else(|| {
        refuse(
            "BREP_TRIM_INVALID",
            "Face index out of range for chart classification",
        )
    })?;
    let wire = model.loops.get(face.outer).ok_or_else(|| {
        refuse(
            "BREP_TRIM_INVALID",
            "Face outer loop missing for chart classification",
        )
    })?;
    if wire.coedges.is_empty() {
        return Err(refuse(
            "BREP_TRIM_INVALID",
            "Empty outer loop cannot be classified",
        ));
    }
    let mut events = Vec::new();
    let mut samples = Vec::new();
    let n = wire.coedges.len();
    for (i, coedge) in wire.coedges.iter().enumerate() {
        let t = i as f64 / n as f64;
        events.push(ChartEvent {
            parameter: t,
            kind: "coedge",
            edge: coedge.edge,
        });
        samples.push((t, CellLabel::Boundary));
        let mid = (i as f64 + 0.5) / n as f64;
        // Material between coedge knots is treated as Inside for a closed outer
        // loop on the frozen chart classes; exterior is not claimed here.
        samples.push((mid, CellLabel::Inside));
    }
    classify_chart_events(chart, events, &samples)
}

/// Exact sew of a closed shell's unique edge pairs from authored vertex points.
/// Returns a certificate or typed refuse; never mutates the model.
pub fn sew_closed_model_edges(model: &crate::Model) -> Result<SewCertificate> {
    model.validate()?;
    let context = model
        .tolerance_context()
        .map_err(|_| refuse("BREP_SEW_INVALID", "Model tolerance context is invalid"))?;
    let scale = model.tolerance_mm.max(1e-9);
    let mut face_usage = vec![None; model.faces.len()];
    for (shell_id, shell) in model.shells.iter().enumerate() {
        if !shell.closed {
            return Err(refuse(
                "BREP_SEW_GAP",
                "Solid sew requires every audited shell to be closed",
            ));
        }
        for use_ in &shell.faces {
            if face_usage[use_.face]
                .replace((use_.reversed, shell_id))
                .is_some()
            {
                return Err(refuse(
                    "BREP_SEW_DUPLICATE",
                    "Face is owned by more than one shell",
                ));
            }
        }
    }
    if face_usage.iter().any(Option::is_none) {
        return Err(refuse(
            "BREP_SEW_GAP",
            "Solid sew found a face not owned by a shell",
        ));
    }
    let mut edge_uses: BTreeMap<usize, Vec<EdgeUse>> = BTreeMap::new();
    for (face_a, face) in model.faces.iter().enumerate() {
        for &wire_id in std::iter::once(&face.outer).chain(face.holes.iter()) {
            let wire = &model.loops[wire_id];
            for (cyclic_index, coedge) in wire.coedges.iter().enumerate() {
                // A collapsed parametric boundary has zero geometric measure
                // and is not a sew seam. Several cone patches may reference
                // the same pole edge; manifold incidence is carried by their
                // nondegenerate radial edges and the shared pole vertex.
                if model.edges[coedge.edge].degenerate {
                    continue;
                }
                let (shell_reversed, shell_id) = face_usage[face_a].unwrap_or((false, 0));
                edge_uses.entry(coedge.edge).or_default().push((
                    face_a,
                    wire_id,
                    cyclic_index,
                    coedge.reversed ^ shell_reversed,
                    shell_id,
                ));
            }
        }
    }
    let mut proven = Vec::new();
    for (edge_id, uses) in edge_uses {
        if uses.len() != 2 {
            return Err(refuse(
                if uses.len() < 2 {
                    "BREP_SEW_GAP"
                } else {
                    "BREP_SEW_DUPLICATE"
                },
                "Closed model edge incidence is not a unique pair",
            ));
        }
        if uses[0].4 != uses[1].4 {
            return Err(refuse(
                "BREP_SEW_SHELL_OWNERSHIP",
                "Boundary mates are not owned by the same shell",
            ));
        }
        let edge = &model.edges[edge_id];
        let endpoints = [
            model.vertices[edge.vertices[0]].point,
            model.vertices[edge.vertices[1]].point,
        ];
        let traversal = |reversed: bool| {
            if reversed {
                [endpoints[1], endpoints[0]]
            } else {
                endpoints
            }
        };
        let seam_shift = if edge.curve.periodic && uses[0].0 == uses[1].0 {
            if uses[0].3 { -1 } else { 1 }
        } else {
            0
        };
        let correspondence = prove_boundary_correspondence(
            &context,
            BoundaryCorrespondenceInput {
                curve_a: &edge.curve,
                curve_b: &edge.curve,
                endpoints_a: traversal(uses[0].3),
                endpoints_b: traversal(uses[1].3),
                orientation: ParameterOrientation::Reversed,
                seam_shift,
                shell: uses[0].4,
                uses: [
                    BoundaryUse {
                        face: uses[0].0,
                        wire: uses[0].1,
                        cyclic_index: uses[0].2,
                        reversed: uses[0].3,
                    },
                    BoundaryUse {
                        face: uses[1].0,
                        wire: uses[1].1,
                        cyclic_index: uses[1].2,
                        reversed: uses[1].3,
                    },
                ],
            },
        )?;
        let mut key = sew_edge_key(endpoints[0], endpoints[1], scale)?;
        key.shell = Some(uses[0].4);
        key.definition = Some(authority_key(&correspondence.authority));
        proven.push(CorrespondenceLedgerEntry {
            key,
            correspondence,
        });
    }
    let (_snap, cert) = exact_sew_correspondences(&SewSnapshot::default(), &context, &proven)?;
    Ok(cert)
}
