//! Common-axis polynomial cancellation for complete rational tensor nets.
pub use super::rational_factor::Refusal;
use super::{
    exact_tensor_edit::row,
    rational_edit::{Work, store},
    rational_factor::{
        common_quotients,
        polynomial::{self, Poly},
    },
};
use crate::{
    Result, check,
    surface::{Axis, Surface},
};
mod proof;
type PResult<T> = std::result::Result<T, Refusal>;
#[derive(Clone, Debug)]
pub struct Report {
    /// Literal source on every refusal, including a later-row proof failure.
    pub surface: Surface,
    pub simplified: bool,
    pub factor_degree: usize,
    pub rows_verified: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn quotients(source: &Surface, axis: Axis, w: &mut Work) -> PResult<(usize, Vec<Poly>)> {
    let count = match axis {
        Axis::U => source.control_points[0].len(),
        Axis::V => source.control_points.len(),
    };
    let mut channels = Vec::new();
    for index in 0..count {
        let c = row(source, axis, index);
        let mut h = vec![Vec::new(); 4];
        for (point, &weight) in c.control_points.iter().zip(&c.weights) {
            let weight = w.lift(weight)?;
            for coordinate in 0..3 {
                let x = w.lift(point[coordinate])?;
                h[coordinate].push(w.op(&x, &weight, |x, y| x * y)?);
            }
            h[3].push(weight);
        }
        channels.extend(h);
    }
    // A divisor common to ALL coordinates and denominators, not one per row.
    common_quotients(&channels, 3, w)
}
fn candidate(
    source: &Surface,
    axis: Axis,
    h: &[Poly],
    degree: usize,
    w: &mut Work,
) -> PResult<Surface> {
    let mut channels = vec![Vec::new(); 4];
    for row in h.chunks_exact(4) {
        for coordinate in 0..4 {
            channels[coordinate].extend(polynomial::bernstein(&row[coordinate], degree, w)?);
        }
    }
    // Exactly one gauge for the complete tensor net; no independent row scale.
    let (points, weights) = store(&channels, w)?;
    let count = h.len() / 4;
    let n = degree + 1;
    let mut output = source.clone();
    let domain = row(source, axis, 0).domain();
    let knots = [vec![domain[0]; n], vec![domain[1]; n]].concat();
    match axis {
        Axis::U => {
            output.degree_u = degree;
            output.knots_u = knots;
            output.control_points = (0..n)
                .map(|i| (0..count).map(|j| points[j * n + i].clone()).collect())
                .collect();
            output.weights = (0..n)
                .map(|i| (0..count).map(|j| weights[j * n + i]).collect())
                .collect();
        }
        Axis::V => {
            output.degree_v = degree;
            output.knots_v = knots;
            output.control_points = points.chunks_exact(n).map(|r| r.to_vec()).collect();
            output.weights = weights.chunks_exact(n).map(|r| r.to_vec()).collect();
        }
    }
    if output.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(output)
}
/// Cancel a common polynomial factor on one nonperiodic clamped Bezier axis.
/// The other axis keeps its complete original basis, including periodic storage.
/// All literal row laws and their shared denominator ratio are re-proved for
/// the actual output. One ledger covers proposal, storage and complete proof.
/// A primitive constant uses valid degree1 storage. No sampling or tolerance.
pub fn cancel(source: &Surface, axis: Axis, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Rational tensor factor work must be in1..=1000000",
    )?;
    let first = row(source, axis, 0);
    let p = first.degree;
    let domain = first.domain();
    check(
        !first.periodic
            && first.control_points.len() == p + 1
            && first.knots[..=p].iter().all(|&u| u == domain[0])
            && first.knots[p + 1..].iter().all(|&u| u == domain[1]),
        "Rational tensor factor requires one nonperiodic clamped Bezier axis",
    )?;
    let refused = |reason, used, factor, verified| Report {
        surface: source.clone(),
        simplified: false,
        factor_degree: factor,
        rows_verified: verified,
        work_used: used,
        refusal: Some(reason),
    };
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (factor, h) = match quotients(source, axis, &mut work) {
        Ok(v) => v,
        Err(r) => return Ok(refused(r, work.used, 0, 0)),
    };
    let minimum = h
        .iter()
        .map(|p| p.len().saturating_sub(1))
        .max()
        .unwrap_or(0)
        .max(1);
    let mut last = Refusal::NotRepresentable;
    for degree in minimum..=p.saturating_sub(1).max(1) {
        let output = match candidate(source, axis, &h, degree, &mut work) {
            Ok(s) => s,
            Err(Refusal::ResourceLimit) => {
                return Ok(refused(Refusal::ResourceLimit, work.used, factor, 0));
            }
            Err(r) => {
                last = r;
                continue;
            }
        };
        let proof = proof::verify(source, &output, axis, &mut work);
        if let Some(reason) = proof.refusal {
            return Ok(refused(reason, work.used, factor, proof.verified));
        }
        return Ok(Report {
            surface: output,
            simplified: true,
            factor_degree: factor,
            rows_verified: proof.verified,
            work_used: work.used,
            refusal: None,
        });
    }
    Ok(refused(last, work.used, factor, 0))
}
