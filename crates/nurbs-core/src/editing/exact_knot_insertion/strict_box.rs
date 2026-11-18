//! Exact strict containment for a planar rational spline's complete active domain.
use super::*;
use num_traits::Signed;

/// Prove strict box containment on an original curve's active interval.
/// A periodic curve must have an exact stored wrap and is checked over one turn.
/// Spans and subdivisions stay homogeneous and share the caller's work limit.
pub fn strict_box_containment(
    curve: &Curve,
    bounds: [[f64; 2]; 2],
    depth: u8,
    max_work: u64,
) -> Result<(bool, u64)> {
    inspect(curve, None, bounds, depth, max_work)
}

/// Prove strict containment on a declared interval of the original clock.
/// Proper subintervals of periodic curves remain unsupported.
pub fn strict_box_containment_on(
    curve: &Curve,
    range: [f64; 2],
    bounds: [[f64; 2]; 2],
    depth: u8,
    max_work: u64,
) -> Result<(bool, u64)> {
    inspect(curve, Some(range), bounds, depth, max_work)
}

fn inspect(
    curve: &Curve,
    range: Option<[f64; 2]>,
    bounds: [[f64; 2]; 2],
    depth: u8,
    max_work: u64,
) -> Result<(bool, u64)> {
    curve.validate()?;
    let [start, end] = curve.domain();
    let range = range.unwrap_or([start, end]);
    if !(start <= range[0] && range[0] < range[1] && range[1] <= end)
        || (curve.periodic && range != [start, end])
    {
        return Ok((false, 0));
    }
    let mut work = Work {
        used: 0,
        limit: max_work.min(cad_predicates::MAX_WORK),
    };
    let proved = prove(curve, range, bounds, depth.min(16), &mut work).is_ok();
    Ok((proved, work.used))
}

fn prove(
    curve: &Curve,
    range: [f64; 2],
    bounds: [[f64; 2]; 2],
    depth: u8,
    work: &mut Work,
) -> PResult<()> {
    let p = curve.degree;
    let n = curve.control_points.len();
    let [start, end] = curve.domain();
    if p == 0 || curve.control_points[0].len() != 2 {
        return Err(Refusal::InvalidOutput);
    }
    if curve.periodic {
        work.charge((curve.knots.len() + p) as u64)?;
        if !crate::analysis::curve_partition_agreement::verify_periodic_source_closure(curve)
            .map_err(|_| Refusal::InvalidOutput)?
        {
            return Err(Refusal::InvalidOutput);
        }
    }

    let clamped = range == [start, end]
        && curve.knots[..=p].iter().all(|&k| k == start)
        && curve.knots[n..].iter().all(|&k| k == end);
    let (mut net, mut knots) = if clamped {
        homogeneous(curve, start, 0, work)?
    } else {
        // Extract the active interval without converting its endpoint
        // coefficients to a stored Cartesian curve.
        work.charge(((n + 2 * p) * 2 * p) as u64)?;
        super::super::exact_curve_merge::clamp_curve_net(curve, range, work)?
    };
    let internal = knots[p + 1..net.len()].to_vec();
    for block in internal.chunk_by(|a, b| a == b) {
        let count = p.checked_sub(block.len()).ok_or(Refusal::InvalidOutput)?;
        if count > 0 {
            work.charge(((net.len() + count) * count) as u64)?;
            refine_homogeneous(&mut net, &mut knots, p, block[0], count, work)?;
        }
    }
    let [lower, upper] = bounds;
    let lo = [work.lift(lower[0])?, work.lift(lower[1])?];
    let hi = [work.lift(upper[0])?, work.lift(upper[1])?];
    for span in p..net.len() {
        work.charge(1)?;
        if knots[span] < knots[span + 1] {
            let mut margins = Vec::with_capacity(p + 1);
            for row in &net[span - p..=span] {
                let mut bounds = std::array::from_fn(|_| Q::zero());
                for axis in 0..2 {
                    let lower = work.op(&lo[axis], &row[2], |a, w| a * w)?;
                    let upper = work.op(&hi[axis], &row[2], |a, w| a * w)?;
                    bounds[2 * axis] = work.op(&row[axis], &lower, |a, b| a - b)?;
                    bounds[2 * axis + 1] = work.op(&upper, &row[axis], |a, b| a - b)?;
                }
                margins.push(bounds);
            }
            positive_margins(margins, depth, work)?;
        }
    }
    Ok(())
}

fn positive_margins(mut net: Vec<[Q; 4]>, depth: u8, work: &mut Work) -> PResult<()> {
    work.charge(net.len() as u64)?;
    if net.iter().flatten().all(Signed::is_positive) {
        return Ok(());
    }
    let n = net.len();
    if depth == 0
        || [&net[0], &net[n - 1]]
            .into_iter()
            .flatten()
            .any(|v| !v.is_positive())
    {
        return Err(Refusal::InvalidOutput);
    }
    let half = Q::new(1.into(), 2.into());
    let mut left = vec![net[0].clone()];
    let mut right = vec![net[n - 1].clone()];
    for remaining in (1..n).rev() {
        for i in 0..remaining {
            for axis in 0..4 {
                net[i][axis] = work.op(&net[i][axis], &net[i + 1][axis], |a, b| (a + b) * &half)?;
            }
        }
        left.push(net[0].clone());
        right.push(net[remaining - 1].clone());
    }
    right.reverse();
    positive_margins(left, depth - 1, work)?;
    positive_margins(right, depth - 1, work)
}
