use super::*;
use crate::editing::{rational_edit::Work, rational_factor::Refusal as Exact};
use std::collections::BTreeMap;

fn key(point: &[f64]) -> Vec<u64> {
    point
        .iter()
        .map(|&x| if x == 0. { 0 } else { x.to_bits() })
        .collect()
}

/// Assemble an unordered open wire from exact clamped endpoint incidence,
/// starting at the specified degree-one vertex. The resulting clock starts
/// at zero and retains the exact duration of each local parameter interval.
/// Crossings in the interior of segments do not create junctions.
pub fn assemble_open_endpoint_chain(
    sources: &[Curve],
    start: &[f64],
    max_work: u64,
) -> Result<GaugeMergeReport> {
    assemble(sources, Anchor::Open(start), max_work, |_| Ok(()))
}

/// Assemble one closed clamped curve, preserving each local span duration.
/// The chosen starting edge and orientation determine the seam and traversal.
/// Endpoint closure alone does not certify simplicity or periodic derivatives.
pub fn assemble_closed_endpoint_chain(
    sources: &[Curve],
    first_segment: usize,
    reversed: bool,
    max_work: u64,
) -> Result<GaugeMergeReport> {
    assemble(
        sources,
        Anchor::Closed(first_segment, reversed),
        max_work,
        |_| Ok(()),
    )
}

/// Assemble an unordered planar contour only after proving each oriented
/// segment injective and every pair separated except for its required joins.
/// The simplicity proof uses separate pair/cell limits. An inconclusive
/// proof returns an error; exact compression preserves the proved image.
pub fn assemble_simple_planar_loop(
    sources: &[Curve],
    first_segment: usize,
    reversed: bool,
    max_work: u64,
    tolerance_uv: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<GaugeMergeReport> {
    assemble(
        sources,
        Anchor::Closed(first_segment, reversed),
        max_work,
        |ordered| {
            let proof =
                crate::trim_simplicity::inspect(ordered, tolerance_uv, max_pairs, max_cells)?;
            if !proof.proven_simple {
                return Err(crate::input(format!(
                    "Planar loop simplicity unproven: {:?}",
                    proof.outcome()
                )));
            }
            Ok(())
        },
    )
}

enum Anchor<'a> {
    Open(&'a [f64]),
    Closed(usize, bool),
}

fn assemble(
    sources: &[Curve],
    anchor: Anchor<'_>,
    max_work: u64,
    verify: impl FnOnce(&[Curve]) -> Result<()>,
) -> Result<GaugeMergeReport> {
    crate::check(
        (2..=256).contains(&sources.len()),
        "Expected 2..=256 wire segments",
    )?;
    crate::check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Invalid wire proof budget",
    )?;
    let mut vertices = BTreeMap::<_, Vec<(usize, bool)>>::new();
    for (i, curve) in sources.iter().enumerate() {
        curve.validate()?;
        crate::check(
            !curve.periodic && clamped(curve),
            "Expected clamped nonperiodic segments",
        )?;
        vertices
            .entry(key(&curve.control_points[0]))
            .or_default()
            .push((i, false));
        vertices
            .entry(key(curve.control_points.last().unwrap()))
            .or_default()
            .push((i, true));
    }
    let (mut vertex, mut first) = match anchor {
        Anchor::Open(start) => {
            let vertex = key(start);
            crate::check(
                vertices.values().all(|edges| edges.len() <= 2)
                    && vertices.values().filter(|edges| edges.len() == 1).count() == 2
                    && vertices.get(&vertex).is_some_and(|edges| edges.len() == 1),
                "Expected unbranched open wire and endpoint anchor",
            )?;
            (vertex, None)
        }
        Anchor::Closed(index, reversed) => {
            crate::check(
                vertices.values().all(|edges| edges.len() == 2),
                "Expected a closed wire without branches",
            )?;
            let curve = sources
                .get(index)
                .ok_or_else(|| crate::input("Invalid starting edge"))?;
            let point = if reversed {
                curve.control_points.last().unwrap()
            } else {
                &curve.control_points[0]
            };
            (key(point), Some((index, reversed)))
        }
    };
    let mut used = vec![false; sources.len()];
    let mut ordered = Vec::with_capacity(sources.len());
    let mut cursor = 0.;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    while ordered.len() < sources.len() {
        let Some((i, backward)) = first
            .take()
            .or_else(|| vertices[&vertex].iter().copied().find(|(i, _)| !used[*i]))
        else {
            return Err(crate::input("Disconnected endpoint graph"));
        };
        used[i] = true;
        let mut curve = sources[i].clone();
        let [a, b] = curve.domain();
        let mut translate = || -> std::result::Result<(), Exact> {
            let origin = work.lift(cursor)?;
            let anchor = work.lift(if backward { b } else { a })?;
            for knot in &mut curve.knots {
                let old = work.lift(*knot)?;
                let offset = work.op(&old, &anchor, |x, y| if backward { y - x } else { x - y })?;
                let new = work.op(&origin, &offset, |x, y| x + y)?;
                *knot = work.float(&new)?;
            }
            Ok(())
        };
        if let Err(reason) = translate() {
            return Ok(GaugeMergeReport {
                curve: None,
                work_used: work.used,
                refusal: Some(GaugeMergeRefusal::Exact(reason)),
            });
        }
        if backward {
            curve.knots.reverse();
            curve.control_points.reverse();
            curve.weights.reverse();
        }
        vertex = key(curve.control_points.last().unwrap());
        cursor = curve.domain()[1];
        ordered.push(curve);
    }
    if work.used == max_work {
        return Ok(GaugeMergeReport {
            curve: None,
            work_used: work.used,
            refusal: Some(GaugeMergeRefusal::Exact(Exact::ResourceLimit)),
        });
    }
    verify(&ordered)?;
    let mut report = compress_mixed_degree_chain(&ordered, max_work - work.used)?;
    report.work_used += work.used;
    Ok(report)
}

/// Assemble a proved simple planar boundary at the selected source endpoint
/// and enforce the requested winding without moving the seam. Area sign alone
/// suffices even if the caller's requested area interval width was not reached.
pub fn assemble_wound_planar_loop(
    sources: &[Curve],
    first_segment: usize,
    clockwise: bool,
    max_work: u64,
    tolerance_uv: f64,
    tolerance_area: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<GaugeMergeReport> {
    let mut result = assemble_simple_planar_loop(
        sources,
        first_segment,
        false,
        max_work,
        tolerance_uv,
        max_pairs,
        max_cells,
    )?;
    let Some(curve) = result.curve.as_mut() else {
        return Ok(result);
    };
    let area = crate::planar_area::measure(std::slice::from_ref(curve), tolerance_area, max_cells)?;
    let [lo, hi] = area.area_interval_mm2;
    crate::check(
        lo > 0. || hi < 0.,
        "Boundary orientation unproven within area budget",
    )?;
    if (hi < 0.) != clockwise {
        let mut work = Work {
            used: result.work_used,
            limit: max_work,
        };
        if let Err(reason) = reflect_segment(curve, &mut work) {
            result.curve = None;
            result.refusal = Some(GaugeMergeRefusal::Exact(reason));
        }
        result.work_used = work.used;
    }
    Ok(result)
}
