//! Joint exact knot and degree compression with shared work accounting.
use super::*;
use crate::editing::exact_surface_degree_reduction;

#[derive(Clone, Debug)]
pub struct TensorCompression {
    pub surface: Surface,
    /// Direct removals by axis, excluding knots removed during degree reduction.
    pub removed_knots: [usize; 2],
    pub reduced_degrees: [usize; 2],
    pub work_used: u64,
    /// A complete pass found no further accepted proposal.
    pub completed: bool,
}

/// Alternate exact knot removal and smooth degree reduction on both axes.
/// Every committed operation preserves the original UV map and its
/// denominator. Work exhaustion retains only fully proved steps.
/// The completed flag describes the search, without claiming global minimality.
pub fn compress_tensor_surface(source: &Surface, max_work: u64) -> Result<TensorCompression> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact tensor compression work must be in [1,1000000]",
    )?;
    validate_chart(source)?;
    compress_validated(source, max_work)
}

fn validate_chart(source: &Surface) -> Result<()> {
    source.validate()?;
    for (degree, knots, count, periodic) in [
        (
            source.degree_u,
            &source.knots_u,
            source.control_points.len(),
            source.periodic_u,
        ),
        (
            source.degree_v,
            &source.knots_v,
            source.control_points[0].len(),
            source.periodic_v,
        ),
    ] {
        check(
            !periodic
                && knots[..=degree].iter().all(|&k| k == knots[degree])
                && knots[count..].iter().all(|&k| k == knots[count]),
            "Exact tensor compression requires both axes clamped and nonperiodic",
        )?;
    }
    Ok(())
}

fn compress_validated(source: &Surface, max_work: u64) -> Result<TensorCompression> {
    let mut report = TensorCompression {
        surface: source.clone(),
        removed_knots: [0; 2],
        reduced_degrees: [0; 2],
        work_used: 0,
        completed: false,
    };
    loop {
        let mut changed = false;
        for (axis_index, axis) in [Axis::U, Axis::V].into_iter().enumerate() {
            let remaining = max_work - report.work_used;
            if remaining == 0 {
                return Ok(report);
            }
            let knots = simplify(&report.surface, axis, remaining)?;
            report.work_used += knots.work_used;
            if knots.removed > 0 {
                changed = true;
                report.removed_knots[axis_index] += knots.removed;
                report.surface = knots.surface;
            }
            if !knots.completed_pass {
                return Ok(report);
            }
            let degree = if axis_index == 0 {
                report.surface.degree_u
            } else {
                report.surface.degree_v
            };
            if degree == 1 {
                continue;
            }
            let remaining = max_work - report.work_used;
            if remaining == 0 {
                return Ok(report);
            }
            let step =
                exact_surface_degree_reduction::reduce_smooth(&report.surface, axis, remaining)?;
            report.work_used += step.work_used;
            if step.reduced {
                changed = true;
                report.reduced_degrees[axis_index] += 1;
                report.surface = step.surface;
            }
            if matches!(
                step.refusal,
                Some(Refusal::ResourceLimit)
                    | Some(Refusal::Curve {
                        reason: exact_knot_removal::Refusal::ResourceLimit,
                        ..
                    })
                    | Some(Refusal::Denominator {
                        decision: cad_predicates::BezierIdentity::Indeterminate(
                            cad_predicates::Reason::ResourceLimit
                        ),
                        ..
                    })
            ) {
                return Ok(report);
            }
        }
        if !changed {
            report.completed = true;
            return Ok(report);
        }
    }
}

#[derive(Clone, Debug)]
pub struct TensorCompressionBatch {
    /// Complete results in input order, or None if the shared budget expires.
    pub patches: Option<Vec<TensorCompression>>,
    pub work_used: u64,
}

/// Compress independent tensor charts as one transaction. Each output
/// preserves the complete original UV law and denominator. The batch
/// charges one unit per input control point before each chart's edit
/// proofs, so even no-op charts consume a bounded share of the work
/// ledger. Work exhaustion discards all proposed charts. This operation
/// preserves supplied chart geometry and does not modify shell topology.
pub fn compress_tensor_surface_batch(
    sources: &[Surface],
    max_work: u64,
) -> Result<TensorCompressionBatch> {
    check(
        !sources.is_empty(),
        "Tensor compression requires at least one patch",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact tensor compression work must be in [1,1000000]",
    )?;
    let mut work_used = 0;
    let mut patches = Vec::new();
    for source in sources {
        validate_chart(source)?;
        let controls = (source.control_points.len() * source.control_points[0].len()) as u64;
        // Reserve at least one proof unit before creating a candidate.
        if controls >= max_work - work_used {
            return Ok(TensorCompressionBatch {
                patches: None,
                work_used,
            });
        }
        work_used += controls;
        let patch = compress_validated(source, max_work - work_used)?;
        work_used += patch.work_used;
        if !patch.completed {
            return Ok(TensorCompressionBatch {
                patches: None,
                work_used,
            });
        }
        patches.push(patch);
    }
    Ok(TensorCompressionBatch {
        patches: Some(patches),
        work_used,
    })
}
