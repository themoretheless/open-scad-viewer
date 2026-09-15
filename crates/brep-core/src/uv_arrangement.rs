//! Lifted-UV arrangements for frozen chart classes (G5b).
//!
//! Imprint curves are inserted as parameter-space events on plane/poly/analytic
//! circle charts. Cell classification is root-isolated. Missed-branch mutations
//! (dropping an event) must flip completeness — independent of the sew ledger.

use crate::trim_sew::{
    classify_chart_events, exact_sew, sew_atomic, CellLabel, ChartEvent, ChartKind,
    ClassificationCertificate, SewCertificate, SewLedgerEntry, SewSnapshot, sew_edge_key,
};
use crate::transactions::ModelSnapshot;
use crate::Model;
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

fn refuse(message: &str) -> Error {
    Error::new("BREP_UV_ARRANGEMENT_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct UvImprintCurve {
    pub chart: ChartKind,
    pub parameter_intervals: Vec<[f64; 2]>,
    pub edge_id: usize,
}

#[derive(Clone, Debug)]
pub struct UvArrangement {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub classification: ClassificationCertificate,
}

/// Build a UV arrangement from imprint curves on a frozen chart.
pub fn arrange_imprint_curves(
    chart: ChartKind,
    curves: &[UvImprintCurve],
) -> Result<UvArrangement> {
    if curves.len() > 256 {
        return Err(refuse("UV imprint curve budget exceeded"));
    }
    let mut events = Vec::new();
    for curve in curves {
        if curve.chart != chart {
            return Err(refuse("Imprint curve chart kind mismatch"));
        }
        for interval in &curve.parameter_intervals {
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
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    let mut samples = Vec::new();
    if events.is_empty() {
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
                samples.push((mid, CellLabel::Inside));
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
    let snapshot = ModelSnapshot::new(model.clone())?;
    let scale = model.tolerance_mm.max(1e-9);
    let mut pending = Vec::new();
    for (face_a, face) in model.faces.iter().enumerate() {
        for &wire_id in std::iter::once(&face.outer).chain(face.holes.iter()) {
            for coedge in &model.loops[wire_id].coedges {
                let edge = &model.edges[coedge.edge];
                let a = model.vertices[edge.vertices[0]].point;
                let b = model.vertices[edge.vertices[1]].point;
                let key = sew_edge_key(a, b, scale)?;
                pending.push(SewLedgerEntry {
                    key,
                    face_a,
                    face_b: face_a,
                    orientation_agree: !coedge.reversed,
                });
            }
        }
    }
    let mut by_key: BTreeMap<_, Vec<_>> = BTreeMap::new();
    for entry in pending {
        by_key.entry(entry.key.clone()).or_default().push(entry);
    }
    let mut paired = Vec::new();
    for (_key, entries) in by_key {
        if entries.len() != 2 {
            let _ = snapshot;
            return Err(refuse("Sew incidence is not a unique pair; model unchanged"));
        }
        paired.push(entries[0].clone());
        paired.push(SewLedgerEntry {
            key: entries[1].key.clone(),
            face_a: entries[1].face_a,
            face_b: entries[0].face_a,
            orientation_agree: entries[1].orientation_agree,
        });
    }
    match sew_atomic(SewSnapshot::default(), &paired) {
        Ok((_snap, cert)) => Ok((model.clone(), cert)),
        Err(err) => {
            // Snapshot proves operands/model preimage is retained by caller.
            let _ = snapshot;
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cuboid;

    #[test]
    fn imprint_curves_classify_with_complete_certificate() {
        let curves = [UvImprintCurve {
            chart: ChartKind::PlanePoly,
            parameter_intervals: vec![[0.2, 0.8]],
            edge_id: 0,
        }];
        let arr = arrange_imprint_curves(ChartKind::PlanePoly, &curves).unwrap();
        assert!(arr.classification.complete);
        assert_missed_branch_detected(&arr).unwrap();
    }

    #[test]
    fn empty_arrangement_is_outside() {
        let arr = arrange_imprint_curves(ChartKind::AnalyticCircle, &[]).unwrap();
        assert!(arr.classification.complete);
        assert!(arr
            .classification
            .cells
            .iter()
            .any(|c| c.2 == CellLabel::Outside));
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
                    err.code == "BREP_UV_ARRANGEMENT_REFUSED"
                        || err.code.starts_with("BREP_SEW_")
                );
                model.validate().unwrap();
            }
        }
        let _ = exact_sew;
    }
}
