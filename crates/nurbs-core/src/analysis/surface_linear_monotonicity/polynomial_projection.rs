//! Global strong-monotonicity enclosure for a constant-weight B-spline.
//! Project authored poles BEFORE differentiating. This retains cancellations
//! between coordinates without replacing their exact sum by a rounded pole.
use super::*;
pub(super) fn certify(
    s: &Surface,
    projection: [[i8; 3]; 2],
    metric: [f64; 2],
    rates: [[f64; 2]; 2],
) -> Result<bool> {
    let weight = s.weights[0][0];
    if s.weights.iter().flatten().any(|&w| w != weight) {
        return Ok(false);
    }
    // A common positive weight cancels by B-spline partition of unity.
    let sizes = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    let mut coordinates: Vec<Vec<[I; 2]>> = Vec::with_capacity(sizes[0]);
    for row in &s.control_points {
        let mut out = Vec::with_capacity(sizes[1]);
        for p in row {
            let mut q = [I::point(0.); 2];
            for r in 0..2 {
                for k in 0..3 {
                    if projection[r][k] != 0 {
                        q[r] = q[r].add(I::point(p[k]).mul(I::point(projection[r][k] as f64))?)?;
                    }
                }
            }
            out.push(q);
        }
        coordinates.push(out);
    }
    let mut j = [[I::point(0.); 2]; 2];
    for axis in 0..2 {
        let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for i in 0..sizes[0] - usize::from(axis == 0) {
            for k in 0..sizes[1] - usize::from(axis == 1) {
                let t = if axis == 0 { i } else { k };
                let width = I::point(knots[axis][t + degrees[axis] + 1])
                    .sub(I::point(knots[axis][t + 1]))?;
                if width.lo <= 0. {
                    return Ok(false);
                }
                let next = coordinates[i + usize::from(axis == 0)][k + usize::from(axis == 1)];
                for r in 0..2 {
                    let d = next[r]
                        .sub(coordinates[i][k][r])?
                        .mul(I::point(degrees[axis] as f64))?
                        .div(width)?;
                    ranges[r][0] = ranges[r][0].min(d.lo);
                    ranges[r][1] = ranges[r][1].max(d.hi);
                }
            }
        }
        // Nonnegative derivative bases sum to one on the unchanged natural
        // rectangle, so the complete coefficient hull covers every span.
        for r in 0..2 {
            j[r][axis] = I::new(ranges[r][0], ranges[r][1])?
                .mul(I::point(metric[r]))?
                .mul(I::new(rates[axis][0], rates[axis][1])?)?;
        }
    }
    if j[0][0].lo <= 0. || j[1][1].lo <= 0. {
        return Ok(false);
    }
    let off = j[0][1].add(j[1][0])?.mul(I::point(0.5))?;
    let magnitude = off.lo.abs().max(off.hi.abs());
    let square = I::point(magnitude).mul(I::point(magnitude))?;
    Ok(j[0][0].mul(j[1][1])?.sub(square)?.lo > 0.)
}
