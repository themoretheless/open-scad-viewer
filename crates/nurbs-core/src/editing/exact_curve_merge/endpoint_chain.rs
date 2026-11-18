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
    let mut vertex = key(start);
    crate::check(
        vertices.values().all(|edges| edges.len() <= 2)
            && vertices.values().filter(|edges| edges.len() == 1).count() == 2
            && vertices.get(&vertex).is_some_and(|edges| edges.len() == 1),
        "Expected unbranched open wire and endpoint anchor",
    )?;
    let mut used = vec![false; sources.len()];
    let mut ordered = Vec::with_capacity(sources.len());
    let mut cursor = 0.;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    while ordered.len() < sources.len() {
        let Some(&(i, backward)) = vertices[&vertex].iter().find(|(i, _)| !used[*i]) else {
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
    let mut report = compress_mixed_degree_chain(&ordered, max_work - work.used)?;
    report.work_used += work.used;
    Ok(report)
}
