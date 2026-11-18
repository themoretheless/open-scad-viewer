//! Joint knot and degree compression of a clamped curve.
use super::{Refusal, simplify};
use crate::{Result, check, curve::Curve, editing::exact_degree_reduction::reduce_smooth};

#[derive(Clone, Debug)]
pub struct CurveCompression {
    pub curve: Curve,
    /// Knots removed directly, excluding removals during degree reduction.
    pub removed_knots: usize,
    pub reduced_degrees: usize,
    pub work_used: u64,
    /// A full pass found no further provable edit.
    pub completed: bool,
}

/// Preserve the exact parameter-to-point map through every accepted edit.
/// A partial result contains only proved changes. The shared arithmetic budget
/// excludes storage validation and copying, as in the underlying operations.
pub fn compress_curve(source: &Curve, max_work: u64) -> Result<CurveCompression> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Invalid curve compression work limit",
    )?;
    let mut report = CurveCompression {
        curve: source.clone(),
        removed_knots: 0,
        reduced_degrees: 0,
        work_used: 0,
        completed: false,
    };
    while report.work_used < max_work {
        let knots = simplify(&report.curve, max_work - report.work_used)?;
        report.work_used += knots.work_used;
        report.removed_knots += knots.removed;
        report.curve = knots.curve;
        if !knots.completed_pass {
            break;
        }
        if report.curve.degree > 1 {
            if report.work_used == max_work {
                break;
            }
            let degree = reduce_smooth(&report.curve, max_work - report.work_used)?;
            report.work_used += degree.work_used;
            if degree.reduced {
                report.reduced_degrees += 1;
                report.curve = degree.curve;
                continue;
            }
            if degree.refusal == Some(Refusal::ResourceLimit) {
                break;
            }
        }
        if knots.removed == 0 {
            report.completed = true;
            break;
        }
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct CurveCompressionBatch {
    /// Complete segment reports in input order, or None on budget exhaustion.
    pub segments: Option<Vec<CurveCompression>>,
    pub work_used: u64,
}

/// Compress a collection of clamped curve segments as one transaction.
/// Charges one unit per input control point plus the shared arithmetic budget.
/// Individual parameter domains, orientation and segment indices are preserved.
pub fn compress_curve_batch(sources: &[Curve], max_work: u64) -> Result<CurveCompressionBatch> {
    check(!sources.is_empty(), "Expected at least one curve segment")?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Invalid curve compression work limit",
    )?;
    let mut work_used = 0;
    let mut segments = Vec::new();
    for source in sources {
        let controls = source.control_points.len() as u64;
        if controls >= max_work - work_used {
            return Ok(CurveCompressionBatch {
                segments: None,
                work_used,
            });
        }
        work_used += controls;
        let result = compress_curve(source, max_work - work_used)?;
        work_used += result.work_used;
        if !result.completed {
            return Ok(CurveCompressionBatch {
                segments: None,
                work_used,
            });
        }
        segments.push(result);
    }
    Ok(CurveCompressionBatch {
        segments: Some(segments),
        work_used,
    })
}
