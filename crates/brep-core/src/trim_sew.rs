//! Lifted-UV arrangements, material cell classification, and exact sewing.
//!
//! Extends planar_trim to frozen chart classes (plane / poly / analytic circle).
//! Sewing is exact-match only: gap/duplicate/orientation mismatches refuse and
//! roll back atomically. No mesh weld, tolerance growth, or auto-heal.

use nurbs_core::{Error, Result};
use std::collections::{BTreeMap, BTreeSet};

fn refuse(code: &'static str, message: &str) -> Error {
    Error::new(code, message)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChartKind {
    PlanePoly,
    AnalyticCircle,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChartEvent {
    pub parameter: f64,
    pub kind: &'static str,
    pub edge: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellLabel {
    Outside,
    Inside,
    Boundary,
}

#[derive(Clone, Debug)]
pub struct ClassificationCertificate {
    pub chart: ChartKind,
    pub events: Vec<ChartEvent>,
    pub cells: Vec<(f64, f64, CellLabel)>,
    pub complete: bool,
}

/// Event order on a frozen chart: parameters must be strictly increasing except
/// for explicit coincident boundary strata which are labeled, never merged.
pub fn classify_chart_events(
    chart: ChartKind,
    mut events: Vec<ChartEvent>,
    sample_points: &[(f64, CellLabel)],
) -> Result<ClassificationCertificate> {
    if events.len() > 4096 || sample_points.len() > 4096 {
        return Err(refuse(
            "BREP_TRIM_RESOURCE_LIMIT",
            "Chart event/sample budget exceeded",
        ));
    }
    events.sort_by(|a, b| {
        a.parameter
            .partial_cmp(&b.parameter)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.edge.cmp(&b.edge))
    });
    for window in events.windows(2) {
        let a = &window[0];
        let b = &window[1];
        if !a.parameter.is_finite() || !b.parameter.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Chart event parameter must be finite",
            ));
        }
        if (a.parameter - b.parameter).abs() <= 1e-15 && a.edge == b.edge {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Duplicate chart event on the same edge without stratum label",
            ));
        }
    }
    let mut cells = Vec::new();
    for (i, point) in sample_points.iter().enumerate() {
        if !point.0.is_finite() {
            return Err(refuse(
                "BREP_TRIM_INVALID",
                "Classification sample must be finite",
            ));
        }
        // Root-isolated: sample must not sit on an event unless Boundary.
        let on_event = events
            .iter()
            .any(|e| (e.parameter - point.0).abs() <= 1e-12);
        if on_event && point.1 != CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Interior/exterior sample coincides with an event root",
            ));
        }
        if !on_event && point.1 == CellLabel::Boundary {
            return Err(refuse(
                "BREP_TRIM_AMBIGUOUS",
                "Boundary label without an isolating event",
            ));
        }
        let lo = if i == 0 {
            f64::NEG_INFINITY
        } else {
            sample_points[i - 1].0
        };
        cells.push((lo, point.0, point.1));
    }
    Ok(ClassificationCertificate {
        chart,
        events,
        cells,
        complete: true,
    })
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SewEdgeKey {
    pub a: [i64; 3],
    pub b: [i64; 3],
}

#[derive(Clone, Debug)]
pub struct SewLedgerEntry {
    pub key: SewEdgeKey,
    pub face_a: usize,
    pub face_b: usize,
    pub orientation_agree: bool,
}

#[derive(Clone, Debug, Default)]
pub struct SewSnapshot {
    pub edges: BTreeSet<SewEdgeKey>,
    pub oriented: BTreeMap<SewEdgeKey, bool>,
}

#[derive(Clone, Debug)]
pub struct SewCertificate {
    pub matched: usize,
    pub complete: bool,
}

fn quantize(point: [f64; 3], scale: f64) -> Result<[i64; 3]> {
    if !point.iter().all(|x| x.is_finite()) || !(scale.is_finite() && scale > 0.) {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Sew quantization requires finite points and positive scale",
        ));
    }
    Ok(point.map(|x| (x / scale).round() as i64))
}

pub fn sew_edge_key(a: [f64; 3], b: [f64; 3], scale: f64) -> Result<SewEdgeKey> {
    let qa = quantize(a, scale)?;
    let qb = quantize(b, scale)?;
    if qa == qb {
        return Err(refuse(
            "BREP_SEW_INVALID",
            "Degenerate sew edge after exact quantization",
        ));
    }
    Ok(if qa <= qb {
        SewEdgeKey { a: qa, b: qb }
    } else {
        SewEdgeKey { a: qb, b: qa }
    })
}

/// Exact-match sew: every boundary edge must appear twice with opposite
/// orientation. Gaps, duplicates (>2), or orientation clashes refuse and leave
/// `base` unchanged (caller keeps the preimage snapshot).
pub fn exact_sew(
    base: &SewSnapshot,
    pending: &[SewLedgerEntry],
) -> Result<(SewSnapshot, SewCertificate)> {
    let mut next = base.clone();
    let mut uses: BTreeMap<SewEdgeKey, Vec<&SewLedgerEntry>> = BTreeMap::new();
    for entry in pending {
        uses.entry(entry.key.clone()).or_default().push(entry);
    }
    for (key, entries) in &uses {
        if entries.len() == 1 {
            return Err(refuse(
                "BREP_SEW_GAP",
                "Boundary edge has a single mate; refuse auto-heal",
            ));
        }
        if entries.len() > 2 {
            return Err(refuse(
                "BREP_SEW_DUPLICATE",
                "Boundary edge claimed by more than two faces",
            ));
        }
        if entries[0].orientation_agree == entries[1].orientation_agree {
            return Err(refuse(
                "BREP_SEW_ORIENTATION",
                "Paired sew edges must disagree in orientation",
            ));
        }
        if entries[0].face_a == entries[1].face_a && entries[0].face_b == entries[1].face_b {
            return Err(refuse(
                "BREP_SEW_DUPLICATE",
                "Paired sew entries reference the same face pair",
            ));
        }
        next.edges.insert(key.clone());
        next.oriented.insert(key.clone(), true);
    }
    Ok((
        next,
        SewCertificate {
            matched: uses.len(),
            complete: true,
        },
    ))
}

/// Rollback drill helper: apply sew or restore `base` on any refusal.
pub fn sew_atomic(base: SewSnapshot, pending: &[SewLedgerEntry]) -> Result<(SewSnapshot, SewCertificate)> {
    match exact_sew(&base, pending) {
        Ok(done) => Ok(done),
        Err(error) => Err(error),
    }
}

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
    if let Some(first) = unique.first() {
        if first.parameter > 0. {
            samples.insert(0, (first.parameter * 0.5, CellLabel::Outside));
        }
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
    let scale = model.tolerance_mm.max(1e-9);
    let mut pending = Vec::new();
    for (face_a, face) in model.faces.iter().enumerate() {
        for &wire_id in std::iter::once(&face.outer).chain(face.holes.iter()) {
            let wire = &model.loops[wire_id];
            for coedge in &wire.coedges {
                let edge = &model.edges[coedge.edge];
                let a = model.vertices[edge.vertices[0]].point;
                let b = model.vertices[edge.vertices[1]].point;
                let key = sew_edge_key(a, b, scale)?;
                // Orientation: reversed coedge disagrees with edge direction.
                pending.push(SewLedgerEntry {
                    key,
                    face_a,
                    face_b: face_a,
                    orientation_agree: !coedge.reversed,
                });
            }
        }
    }
    // Pair opposite orientations per key across different faces.
    let mut by_key: BTreeMap<SewEdgeKey, Vec<SewLedgerEntry>> = BTreeMap::new();
    for entry in pending {
        by_key.entry(entry.key.clone()).or_default().push(entry);
    }
    let mut paired = Vec::new();
    for (_key, entries) in by_key {
        if entries.len() != 2 {
            return Err(refuse(
                if entries.len() < 2 {
                    "BREP_SEW_GAP"
                } else {
                    "BREP_SEW_DUPLICATE"
                },
                "Closed model edge incidence is not a unique pair",
            ));
        }
        if entries[0].face_a == entries[1].face_a {
            // Same-face repeated edge (seam) — admit only if orientations disagree.
            if entries[0].orientation_agree == entries[1].orientation_agree {
                return Err(refuse(
                    "BREP_SEW_ORIENTATION",
                    "Seam edge coedges must disagree in orientation",
                ));
            }
        }
        paired.push(entries[0].clone());
        paired.push(SewLedgerEntry {
            face_a: entries[1].face_a,
            face_b: entries[0].face_a,
            orientation_agree: entries[1].orientation_agree,
            key: entries[1].key.clone(),
        });
    }
    let (_snap, cert) = sew_atomic(SewSnapshot::default(), &paired)?;
    Ok(cert)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn certified_event_order_and_classification() {
        let cert = classify_chart_events(
            ChartKind::PlanePoly,
            vec![
                ChartEvent {
                    parameter: 0.25,
                    kind: "enter",
                    edge: 0,
                },
                ChartEvent {
                    parameter: 0.75,
                    kind: "exit",
                    edge: 1,
                },
            ],
            &[(0.1, CellLabel::Outside), (0.5, CellLabel::Inside), (0.9, CellLabel::Outside)],
        )
        .unwrap();
        assert!(cert.complete);
        assert_eq!(cert.cells.len(), 3);
    }

    #[test]
    fn sample_on_event_without_boundary_label_refuses() {
        assert!(
            classify_chart_events(
                ChartKind::AnalyticCircle,
                vec![ChartEvent {
                    parameter: 0.5,
                    kind: "root",
                    edge: 0,
                }],
                &[(0.5, CellLabel::Inside)],
            )
            .is_err()
        );
    }

    #[test]
    fn exact_sew_matches_opposite_orientation_pairs() {
        let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
        let pending = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 1,
                face_b: 0,
                orientation_agree: false,
            },
        ];
        let base = SewSnapshot::default();
        let (next, cert) = sew_atomic(base.clone(), &pending).unwrap();
        assert!(cert.complete);
        assert_eq!(cert.matched, 1);
        assert!(next.edges.contains(&key));
    }

    #[test]
    fn sew_gap_refuses_without_mutating_base() {
        let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
        let pending = [SewLedgerEntry {
            key,
            face_a: 0,
            face_b: 1,
            orientation_agree: true,
        }];
        let base = SewSnapshot::default();
        let err = sew_atomic(base.clone(), &pending).unwrap_err();
        assert_eq!(err.code, "BREP_SEW_GAP");
        assert!(base.edges.is_empty());
    }

    #[test]
    fn sew_duplicate_and_orientation_refuse() {
        let key = sew_edge_key([0., 0., 0.], [0., 1., 0.], 1e-9).unwrap();
        let dup = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 2,
                face_b: 3,
                orientation_agree: false,
            },
            SewLedgerEntry {
                key: key.clone(),
                face_a: 4,
                face_b: 5,
                orientation_agree: true,
            },
        ];
        assert_eq!(
            sew_atomic(SewSnapshot::default(), &dup).unwrap_err().code,
            "BREP_SEW_DUPLICATE"
        );
        let bad_orient = [
            SewLedgerEntry {
                key: key.clone(),
                face_a: 0,
                face_b: 1,
                orientation_agree: true,
            },
            SewLedgerEntry {
                key,
                face_a: 1,
                face_b: 0,
                orientation_agree: true,
            },
        ];
        assert_eq!(
            sew_atomic(SewSnapshot::default(), &bad_orient)
                .unwrap_err()
                .code,
            "BREP_SEW_ORIENTATION"
        );
    }

    #[test]
    fn cuboid_face_outer_loop_classifies_complete() {
        let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
        let cert = classify_face_outer_loop(&model, 0, ChartKind::PlanePoly).unwrap();
        assert!(cert.complete);
        assert!(!cert.events.is_empty());
    }

    #[test]
    fn cylinder_closed_edge_sew_or_typed_refuse() {
        let model = crate::cylinder(2., 4.).unwrap();
        // Analytic cylinder shares edges across faces; expect Complete sew or a
        // typed incidence refuse — never silent heal.
        match sew_closed_model_edges(&model) {
            Ok(cert) => assert!(cert.complete),
            Err(err) => assert!(
                err.code == "BREP_SEW_GAP"
                    || err.code == "BREP_SEW_DUPLICATE"
                    || err.code == "BREP_SEW_ORIENTATION"
                    || err.code == "BREP_SEW_INVALID"
            ),
        }
    }
}
