//! Exact strict containment for a planar rational spline's complete active domain.
use super::*;
use num_traits::Signed;

/// Prove strict box containment for an original clamped nonperiodic curve.
/// Spans and subdivisions stay homogeneous and share the caller's work limit.
pub fn strict_box_containment(
    curve: &Curve,
    bounds: [[f64; 2]; 2],
    depth: u8,
    max_work: u64,
) -> Result<(bool, u64)> {
    curve.validate()?;
    let mut work = Work {
        used: 0,
        limit: max_work.min(cad_predicates::MAX_WORK),
    };
    let proved = prove(curve, bounds, depth.min(16), &mut work).is_ok();
    Ok((proved, work.used))
}

fn prove(curve: &Curve, bounds: [[f64; 2]; 2], depth: u8, work: &mut Work) -> PResult<()> {
    let p = curve.degree;
    let n = curve.control_points.len();
    let [start, end] = curve.domain();
    if curve.periodic
        || p == 0
        || curve.control_points[0].len() != 2
        || curve.knots[..=p].iter().any(|&k| k != start)
        || curve.knots[n..].iter().any(|&k| k != end)
    {
        return Err(Refusal::InvalidOutput);
    }

    let [lower, upper] = bounds;
    let lo = [work.lift(lower[0])?, work.lift(lower[1])?];
    let hi = [work.lift(upper[0])?, work.lift(upper[1])?];
    let mut net = Vec::with_capacity(n);
    for (point, &weight) in curve.control_points.iter().zip(&curve.weights) {
        let w = work.lift(weight)?;
        let x = work.lift(point[0])?;
        let y = work.lift(point[1])?;
        let mut row = Vec::with_capacity(5);
        for (a, b) in [(&x, &lo[0]), (&hi[0], &x), (&y, &lo[1]), (&hi[1], &y)] {
            let margin = work.op(a, b, |a, b| a - b)?;
            row.push(work.op(&margin, &w, |a, b| a * b)?);
        }
        row.push(w);
        net.push(row);
    }

    let mut knots = curve.knots.clone();
    for block in curve.knots[p + 1..n].chunk_by(|a, b| a == b) {
        let count = p.checked_sub(block.len()).ok_or(Refusal::InvalidOutput)?;
        if count > 0 {
            work.charge(((net.len() + count) * count) as u64)?;
            refine_homogeneous(&mut net, &mut knots, p, block[0], count, work)?;
        }
    }
    for span in p..net.len() {
        work.charge(1)?;
        if knots[span] < knots[span + 1] {
            work.charge(((p + 1) * 4) as u64)?;
            let margins = net[span - p..=span]
                .iter()
                .map(|row| std::array::from_fn(|axis| row[axis].clone()))
                .collect();
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
