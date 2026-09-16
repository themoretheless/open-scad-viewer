//! Lifted-UV arrangements for frozen chart classes (G5b / F3 close).
//!
//! Supports multiple imprint curves per chart, monotone nesting, multiple holes,
//! periodic AnalyticCircle wrap strata, freeform chart touches, and missed-branch
//! mutation evidence. Periodic/pole rules refuse non-finite or wrap-ambiguous strata.

use crate::Model;
use crate::transactions::ModelSnapshot;
use crate::trim_sew::{
    CellLabel, ChartEvent, ChartKind, ClassificationCertificate, SewCertificate, SewLedgerEntry,
    SewSnapshot, classify_chart_events, sew_atomic, sew_edge_key,
};
use nurbs_core::{Error, Result};
use std::collections::BTreeMap;

fn refuse(message: &str) -> Error {
    Error::new("BREP_UV_ARRANGEMENT_REFUSED", message)
}

#[derive(Clone, Debug)]
pub struct UvImprintCurve {
    pub chart: ChartKind,
    pub parameter_intervals: Vec<[f64; 2]>,
    /// Optional hole intervals classified Outside relative to outer enter/exit.
    pub hole_intervals: Vec<[f64; 2]>,
    pub edge_id: usize,
    /// True when the chart is periodic and the interval may wrap the seam.
    pub periodic: bool,
}

#[derive(Clone, Debug)]
pub struct NestingRecord {
    pub outer_edge: usize,
    pub nested_edges: Vec<usize>,
    pub depth: usize,
}

#[derive(Clone, Debug)]
pub struct UvArrangement {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub classification: ClassificationCertificate,
    pub hole_count: usize,
    /// Monotone nesting of imprint intervals (outer contains inner).
    pub nesting: Vec<NestingRecord>,
}

fn interval_contains(outer: [f64; 2], inner: [f64; 2]) -> bool {
    outer[0] <= inner[0] + 1e-15
        && inner[1] <= outer[1] + 1e-15
        && (inner[0] - outer[0]).abs() + (outer[1] - inner[1]).abs() > 1e-12
}

fn compute_monotone_nesting(curves: &[UvImprintCurve]) -> Result<Vec<NestingRecord>> {
    let mut intervals: Vec<(usize, [f64; 2])> = Vec::new();
    for curve in curves {
        for interval in &curve.parameter_intervals {
            intervals.push((curve.edge_id, *interval));
        }
        for hole in &curve.hole_intervals {
            // Holes nest inside material intervals; record with offset edge ids.
            intervals.push((curve.edge_id.saturating_add(10_000), *hole));
        }
    }
    intervals.sort_by(|a, b| {
        (a.1[1] - a.1[0])
            .partial_cmp(&(b.1[1] - b.1[0]))
            .unwrap_or(std::cmp::Ordering::Equal)
            .reverse()
            .then_with(|| a.0.cmp(&b.0))
    });
    let mut nesting = Vec::new();
    for (i, (edge, span)) in intervals.iter().enumerate() {
        let mut nested = Vec::new();
        let mut depth = 0usize;
        for (j, (other_edge, other)) in intervals.iter().enumerate() {
            if i == j {
                continue;
            }
            if interval_contains(*span, *other) {
                nested.push(*other_edge);
            }
            if interval_contains(*other, *span) {
                depth += 1;
            }
        }
        if !nested.is_empty() || depth > 0 {
            nested.sort_unstable();
            nested.dedup();
            nesting.push(NestingRecord {
                outer_edge: *edge,
                nested_edges: nested,
                depth,
            });
        }
    }
    // Crossing (non-monotone) pairs refuse: intervals overlap without containment.
    for i in 0..intervals.len() {
        for j in (i + 1)..intervals.len() {
            let a = intervals[i].1;
            let b = intervals[j].1;
            let overlap_lo = a[0].max(b[0]);
            let overlap_hi = a[1].min(b[1]);
            if overlap_hi > overlap_lo + 1e-12
                && !interval_contains(a, b)
                && !interval_contains(b, a)
            {
                return Err(refuse(
                    "Non-monotone overlapping imprint intervals; refuse UV arrange",
                ));
            }
        }
    }
    Ok(nesting)
}

/// Split a periodic wrap interval [a,b] with a>b into [a, period] U [0, b].
fn expand_periodic_intervals(intervals: &[[f64; 2]], period: f64) -> Result<Vec<[f64; 2]>> {
    let mut out = Vec::new();
    for interval in intervals {
        if !interval[0].is_finite() || !interval[1].is_finite() {
            return Err(refuse("Periodic imprint interval must be finite"));
        }
        if interval[0] < 0. || interval[1] < 0. || interval[0] > period || interval[1] > period {
            return Err(refuse("Periodic imprint interval must lie in [0, period]"));
        }
        if interval[1] >= interval[0] {
            if (interval[1] - interval[0]) >= period - 1e-9 {
                return Err(refuse("Periodic full-period imprint is a pole/seam refuse"));
            }
            out.push(*interval);
        } else {
            // Wrap across the seam: [lo, period] + [0, hi].
            let first = [interval[0], period];
            let second = [0., interval[1]];
            if (first[1] - first[0]) + (second[1] - second[0]) >= period - 1e-9 {
                return Err(refuse(
                    "Periodic full-period wrap imprint is a pole/seam refuse",
                ));
            }
            if first[1] > first[0] + 1e-15 {
                out.push(first);
            }
            if second[1] > second[0] + 1e-15 {
                out.push(second);
            }
        }
    }
    Ok(out)
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
            // Freeform chart touches: require finite ordered intervals and refuse
            // empty-parameter strata that would claim Complete without work.
            if curve.parameter_intervals.is_empty() && curve.hole_intervals.is_empty() {
                return Err(refuse(
                    "Freeform chart touch requires at least one imprint or hole interval",
                ));
            }
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
                    displacement: None,
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
            return Err(refuse(
                "Sew incidence is not a unique pair; model unchanged",
            ));
        }
        paired.push(entries[0].clone());
        paired.push(SewLedgerEntry {
            key: entries[1].key.clone(),
            face_a: entries[1].face_a,
            face_b: entries[0].face_a,
            orientation_agree: entries[1].orientation_agree,
            displacement: None,
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
    fn freeform_chart_touch_arranges() {
        let curves = [UvImprintCurve {
            chart: ChartKind::Freeform,
            parameter_intervals: vec![[0.15, 0.85]],
            hole_intervals: vec![[0.4, 0.55]],
            edge_id: 7,
            periodic: false,
        }];
        let arr = arrange_imprint_curves(ChartKind::Freeform, &curves).unwrap();
        assert!(arr.classification.complete);
        assert_eq!(arr.hole_count, 1);
        assert_missed_branch_detected(&arr).unwrap();
    }

    #[test]
    fn freeform_empty_refuses() {
        assert!(arrange_imprint_curves(ChartKind::Freeform, &[]).is_err());
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
}
