//! Exact affine dimension of the nominal binary64 control net.
//!
//! A lower-dimensional control hull contains every positive-weight rational
//! image. This read-only query neither replaces a NURBS definition nor proves
//! regularity, trimmed-domain coverage, or equivalence to an analytic patch.
use crate::{Result, check, curve::Curve, surface::Surface};
use cad_predicates::{
    AuthoredScalar, Outcome, PredicateContext, Reason, Sign, SourceArena, ToleranceContext,
};

/// Result over the complete validated control net, with original flat indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Report {
    /// Exact affine dimension when complete; proved lower bound otherwise.
    pub dimension: usize,
    /// Affinely independent source controls establishing that lower bound.
    pub witness_indices: Vec<usize>,
    pub complete: bool,
    pub reason: Option<Reason>,
    /// Charged predicate work, summed across every temporary source context.
    pub work_used: u64,
    pub control_count: usize,
}

struct Work {
    remaining: u64,
    used: u64,
}
impl Work {
    fn orientation(&mut self, p: &[[f64; 3]], axes: Option<[usize; 2]>) -> Result<Outcome> {
        if self.remaining == 0 {
            return Ok(Outcome::Indeterminate(Reason::ResourceLimit));
        }
        let values = match axes {
            Some(a) => p
                .iter()
                .flat_map(|p| a.map(|i| AuthoredScalar::Binary64Bits(p[i].to_bits())))
                .collect(),
            None => p
                .iter()
                .flat_map(|p| p.map(|x| AuthoredScalar::Binary64Bits(x.to_bits())))
                .collect(),
        };
        let source = SourceArena::authored("nurbs-affine-control-support", 0, values)
            .map_err(|e| crate::input(format!("Affine support source admission: {e}")))?;
        let tolerance = ToleranceContext::default_valid();
        let limits = cad_predicates::Limits {
            max_work: self.remaining,
            ..Default::default()
        };
        let mut ctx = PredicateContext::new(&source, &tolerance, limits, None);
        let leaf = |i| source.leaf(i).expect("bounded local source index");
        let decision = if axes.is_some() {
            cad_predicates::orient2d(
                &mut ctx,
                [leaf(0), leaf(1)],
                [leaf(2), leaf(3)],
                [leaf(4), leaf(5)],
            )
        } else {
            cad_predicates::orient3d(
                &mut ctx,
                [leaf(0), leaf(1), leaf(2)],
                [leaf(3), leaf(4), leaf(5)],
                [leaf(6), leaf(7), leaf(8)],
                [leaf(9), leaf(10), leaf(11)],
            )
        }
        .map_err(|e| crate::input(format!("Affine support predicate: {e}")))?;
        let charged = ctx.work_used();
        self.remaining = self
            .remaining
            .checked_sub(charged)
            .ok_or_else(|| crate::input("Affine support work accounting overflow"))?;
        self.used += charged;
        Ok(decision.outcome)
    }
}

fn inspect(points: &[[f64; 3]], max_work: u64) -> Result<Report> {
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Affine support work must lie in 1..=1000000",
    )?;
    check(
        !points.is_empty() && points.len() <= 65_536,
        "Affine support requires 1..65536 controls",
    )?;
    check(
        points.iter().flatten().all(|x| x.is_finite()),
        "Affine support controls must be finite",
    )?;
    let mut work = Work {
        remaining: max_work,
        used: 0,
    };
    let mut report = Report {
        dimension: 0,
        witness_indices: vec![0],
        complete: true,
        reason: None,
        work_used: 0,
        control_count: points.len(),
    };
    for (index, &point) in points.iter().enumerate().skip(1) {
        let independent = match report.dimension {
            0 => point != points[0],
            1 => {
                let a = points[report.witness_indices[0]];
                let b = points[report.witness_indices[1]];
                let mut nonzero = false;
                for axes in [[0, 1], [1, 2], [2, 0]] {
                    match work.orientation(&[a, b, point], Some(axes))? {
                        Outcome::Sign(Sign::Zero) => {}
                        Outcome::Sign(_) => {
                            nonzero = true;
                            break;
                        }
                        Outcome::Indeterminate(reason) => {
                            report.complete = false;
                            report.reason = Some(reason);
                            report.work_used = work.used;
                            return Ok(report);
                        }
                    }
                }
                nonzero
            }
            2 => match work.orientation(
                &[
                    points[report.witness_indices[0]],
                    points[report.witness_indices[1]],
                    points[report.witness_indices[2]],
                    point,
                ],
                None,
            )? {
                Outcome::Sign(Sign::Zero) => false,
                Outcome::Sign(_) => true,
                Outcome::Indeterminate(reason) => {
                    report.complete = false;
                    report.reason = Some(reason);
                    report.work_used = work.used;
                    return Ok(report);
                }
            },
            _ => unreachable!("rank-three hull terminates immediately"),
        };
        if independent {
            report.dimension += 1;
            report.witness_indices.push(index);
            // R^3 supplies the matching upper bound regardless of remaining controls.
            if report.dimension == 3 {
                break;
            }
        }
    }
    report.work_used = work.used;
    Ok(report)
}

/// Recognize the affine support of an original validated 3D curve control net.
/// A complete line/plane result is containment only, not image-surjectivity.
pub fn curve(source: &Curve, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        source.control_points.iter().all(|p| p.len() == 3),
        "Affine support requires 3D curve controls",
    )?;
    let controls: Vec<[f64; 3]> = source
        .control_points
        .iter()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    inspect(&controls, max_work)
}

/// Recognize the affine support of a validated surface control net.
/// Witness indices use U-major flattening. No regularity or plane replacement
/// authority follows from the report; a negative plane test concerns controls.
pub fn surface(source: &Surface, max_work: u64) -> Result<Report> {
    source.validate()?;
    let controls: Vec<[f64; 3]> = source
        .control_points
        .iter()
        .flatten()
        .map(|p| [p[0], p[1], p[2]])
        .collect();
    inspect(&controls, max_work)
}
