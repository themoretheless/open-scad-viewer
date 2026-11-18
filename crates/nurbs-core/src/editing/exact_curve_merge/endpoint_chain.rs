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
    assemble(sources, Anchor::Open(start), max_work, |_, _| Ok(()))
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
        |_, _| Ok(()),
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
        |ordered, _| prove_simple(ordered, tolerance_uv, max_pairs, max_cells),
    )
}

fn prove_simple(
    curves: &[Curve],
    tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<()> {
    let proof = crate::trim_simplicity::inspect(curves, tolerance, max_pairs, max_cells)?;
    if !proof.proven_simple {
        return Err(crate::input(format!(
            "Projected loop simplicity unproven: {:?}",
            proof.outcome()
        )));
    }
    Ok(())
}

/// Certify a closed spatial wire using a simple coordinate projection.
/// Dropping one Cartesian coordinate commutes with rational evaluation.
/// A simple projection proves spatial simplicity even on nonplanar wires;
/// an inconclusive projection can be retried along another axis.
pub fn assemble_simple_spatial_loop(
    sources: &[Curve],
    first_segment: usize,
    reversed: bool,
    dropped_axis: usize,
    max_work: u64,
    projection_tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<GaugeMergeReport> {
    crate::check(dropped_axis < 3, "Expected a Cartesian projection axis")?;
    assemble(
        sources,
        Anchor::Closed(first_segment, reversed),
        max_work,
        |ordered, _| {
            let mut projected = ordered.to_vec();
            for curve in &mut projected {
                crate::check(
                    curve.control_points[0].len() == 3,
                    "Spatial wire requires 3D segments",
                )?;
                for point in &mut curve.control_points {
                    point.remove(dropped_axis);
                }
            }
            prove_simple(&projected, projection_tolerance, max_pairs, max_cells)
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
    verify: impl FnOnce(&[Curve], &mut Work) -> Result<()>,
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
    verify(&ordered, &mut work)?;
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

/// Prove spatial simplicity using two supplied linear coordinate forms.
/// Each projected pole is computed as an exact rational dot product and must
/// fit binary64 before being passed to the planar proof. The output keeps
/// the original 3D poles' geometry; projection shares the edit work budget.
pub fn assemble_oblique_simple_loop(
    sources: &[Curve],
    first_segment: usize,
    reversed: bool,
    forms: [[f64; 3]; 2],
    max_work: u64,
    projection_tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<GaugeMergeReport> {
    crate::check(
        forms.iter().flatten().all(|v| v.is_finite()),
        "Expected finite projection coefficients",
    )?;
    assemble(
        sources,
        Anchor::Closed(first_segment, reversed),
        max_work,
        |ordered, work| {
            let mut projected = ordered.to_vec();
            for curve in &mut projected {
                crate::check(curve.control_points[0].len() == 3, "Expected spatial curve")?;
                for point in &mut curve.control_points {
                    let mut xy = Vec::with_capacity(2);
                    for row in &forms {
                        let mut dot = || -> std::result::Result<f64, Exact> {
                            let mut sum = work.lift(0.)?;
                            for (&coefficient, &coordinate) in row.iter().zip(point.iter()) {
                                let a = work.lift(coefficient)?;
                                let b = work.lift(coordinate)?;
                                let product = work.op(&a, &b, |x, y| x * y)?;
                                sum = work.op(&sum, &product, |x, y| x + y)?;
                            }
                            Ok(work.float(&sum)?)
                        };
                        xy.push(dot().map_err(|reason| {
                            crate::input(format!("Exact projection failed: {reason:?}"))
                        })?);
                    }
                    *point = xy;
                }
            }
            prove_simple(&projected, projection_tolerance, max_pairs, max_cells)
        },
    )
}

/// Partition multi-span edges exactly for proof, preserving original edges for output.
/// Certify each adjacent Bezier pair in a locally chosen coordinate plane.
/// Every other pair must have a positive spatial distance lower bound.
/// Pair and subdivision limits are shared across the complete wire.
pub fn assemble_pairwise_simple_spatial_loop(
    sources: &[Curve],
    first_segment: usize,
    reversed: bool,
    max_work: u64,
    tolerance: f64,
    max_pairs: usize,
    max_cells: usize,
) -> Result<GaugeMergeReport> {
    crate::check(
        tolerance.is_finite()
            && tolerance > 0.
            && (1..=100000).contains(&max_cells)
            && (1..=100000).contains(&max_pairs),
        "Invalid spatial separation limits",
    )?;
    assemble(
        sources,
        Anchor::Closed(first_segment, reversed),
        max_work,
        |ordered, work| {
            crate::check(
                ordered.iter().all(|c| c.control_points[0].len() == 3),
                "Expected 3D segments",
            )?;
            let span_count: usize = ordered
                .iter()
                .map(|c| c.knots.windows(2).filter(|p| p[0] < p[1]).count())
                .sum();
            crate::check(
                span_count * (span_count - 1) / 2 <= max_pairs,
                "Spatial pair budget exhausted",
            )?;
            let mut spans = Vec::with_capacity(span_count);
            for c in ordered {
                let mut cuts = c.knots[c.degree + 1..c.control_points.len()].to_vec();
                cuts.dedup();
                if cuts.is_empty() {
                    spans.push(c.clone());
                    continue;
                }
                crate::check(
                    work.used < work.limit,
                    "Exact span partition budget exhausted",
                )?;
                let partition = crate::editing::exact_knot_insertion::partition(
                    c,
                    &cuts,
                    work.limit - work.used,
                )?;
                work.used += partition.work_used;
                spans.extend(partition.pieces.ok_or_else(|| {
                    crate::input(format!(
                        "Spatial proof partition refused: {:?}",
                        partition.refusal
                    ))
                })?);
            }
            let ordered = spans.as_slice();
            let n = ordered.len();
            let projections: Vec<Vec<Curve>> = (0..3)
                .map(|axis| {
                    let mut curves = ordered.to_vec();
                    for curve in &mut curves {
                        for point in &mut curve.control_points {
                            point.remove(axis);
                        }
                    }
                    curves
                })
                .collect();
            let mut remaining = max_cells;
            for i in 0..n {
                for j in i + 1..n {
                    if j == i + 1 || (i == 0 && j == n - 1) {
                        let mut proved = false;
                        for curves in &projections {
                            let a = &curves[i];
                            let b = &curves[j];
                            // Refuse views that collapse distinct unshared ends into
                            // an additional apparent join.
                            let contacts = [&a.control_points[0], a.control_points.last().unwrap()]
                                .into_iter()
                                .filter(|p| {
                                    *p == &b.control_points[0]
                                        || *p == b.control_points.last().unwrap()
                                })
                                .count();
                            if contacts == if n == 2 { 2 } else { 1 }
                                && crate::trim_simplicity::prove_endpoint_contacts_only(a, b)?
                            {
                                proved = true;
                                break;
                            }
                        }
                        crate::check(
                            proved,
                            "Adjacent spatial contact or segment injectivity unproven",
                        )?;
                    } else {
                        crate::check(remaining > 0, "Spatial subdivision budget exhausted")?;
                        let proof = crate::curve_distance::prove_separation(
                            &ordered[i],
                            &ordered[j],
                            tolerance,
                            remaining,
                        )?;
                        remaining -= proof.cells;
                        crate::check(
                            proof.distance_interval_mm[0] > 0.,
                            "Nonadjacent spatial separation unproven",
                        )?;
                    }
                }
            }
            Ok(())
        },
    )
}
