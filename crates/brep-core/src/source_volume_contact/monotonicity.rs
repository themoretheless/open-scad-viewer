//! Outward SPD proof on a complete dyadic partition of a trivariate cube.
use nurbs_core::{interval_eval::Interval as I, Result};
type Net = [[Vec<[I; 3]>; 2]; 2];
pub(super) struct Report {
    pub certified: bool,
    pub cells: usize,
    pub projection: [[f64; 3]; 3],
    pub principal_minor_lower: [f64; 3],
}
fn midpoint(points: &[[f64; 3]]) -> [f64; 3] {
    let mut values = points.to_vec();
    for n in (1..values.len()).rev() {
        for i in 0..n {
            for k in 0..3 {
                values[i][k] = 0.5 * values[i][k] + 0.5 * values[i + 1][k];
            }
        }
    }
    values[0]
}
fn proposal(net: &[[Vec<[f64; 3]>; 2]; 2]) -> [[f64; 3]; 3] {
    let rows = net
        .each_ref()
        .map(|row| row.each_ref().map(|p| midpoint(p)));
    let mut out = [[0.; 3]; 3];
    for k in 0..3 {
        out[0][k] = 0.5 * (rows[1][0][k] - rows[0][0][k]) + 0.5 * (rows[1][1][k] - rows[0][1][k]);
        out[1][k] = 0.5 * (rows[0][1][k] - rows[0][0][k]) + 0.5 * (rows[1][1][k] - rows[1][0][k]);
    }
    let degree = (net[0][0].len() - 1) as f64;
    for row in net {
        for points in row {
            let derivative = points
                .windows(2)
                .map(|p| std::array::from_fn(|k| degree * (p[1][k] - p[0][k])))
                .collect::<Vec<_>>();
            let v = midpoint(&derivative);
            for k in 0..3 {
                out[2][k] += 0.25 * v[k];
            }
        }
    }
    out
}
fn split_curve(points: &[[I; 3]]) -> Result<(Vec<[I; 3]>, Vec<[I; 3]>)> {
    let n = points.len();
    let mut work = points.to_vec();
    let mut left = vec![work[0]];
    let mut right = vec![work[n - 1]];
    for count in (1..n).rev() {
        for i in 0..count {
            for k in 0..3 {
                work[i][k] = work[i][k]
                    .mul(I::point(0.5))?
                    .add(work[i + 1][k].mul(I::point(0.5))?)?;
            }
        }
        left.push(work[0]);
        right.push(work[count - 1]);
    }
    right.reverse();
    Ok((left, right))
}
fn split(net: &Net, axis: usize) -> Result<(Net, Net)> {
    let mut left = net.clone();
    let mut right = net.clone();
    if axis == 2 {
        for a in 0..2 {
            for b in 0..2 {
                (left[a][b], right[a][b]) = split_curve(&net[a][b])?;
            }
        }
    } else {
        for other in 0..2 {
            for v in 0..net[0][0].len() {
                for k in 0..3 {
                    let p = if axis == 0 {
                        [net[0][other][v][k], net[1][other][v][k]]
                    } else {
                        [net[other][0][v][k], net[other][1][v][k]]
                    };
                    let mid = p[0].mul(I::point(0.5))?.add(p[1].mul(I::point(0.5))?)?;
                    if axis == 0 {
                        left[1][other][v][k] = mid;
                        right[0][other][v][k] = mid;
                    } else {
                        left[other][1][v][k] = mid;
                        right[other][0][v][k] = mid;
                    }
                }
            }
        }
    }
    Ok((left, right))
}
fn jacobian(net: &Net, width: [f64; 3]) -> Result<[[I; 3]; 3]> {
    let mut out = [[I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3]; 3];
    for axis in 0..3 {
        for a in 0..2 {
            for b in 0..2 {
                for v in 0..net[0][0].len() {
                    if (axis == 0 && a == 1)
                        || (axis == 1 && b == 1)
                        || (axis == 2 && v + 1 == net[0][0].len())
                    {
                        continue;
                    }
                    let next = match axis {
                        0 => net[1][b][v],
                        1 => net[a][1][v],
                        _ => net[a][b][v + 1],
                    };
                    let factor = I::point(if axis == 2 {
                        (net[0][0].len() - 1) as f64
                    } else {
                        1.
                    })
                    .div(I::point(width[axis]))?;
                    for k in 0..3 {
                        let d = next[k].sub(net[a][b][v][k])?.mul(factor)?;
                        out[k][axis].lo = out[k][axis].lo.min(d.lo);
                        out[k][axis].hi = out[k][axis].hi.max(d.hi);
                    }
                }
            }
        }
    }
    Ok(out)
}
fn minors(j: [[I; 3]; 3], projection: [[f64; 3]; 3]) -> Result<[I; 3]> {
    let mut a = [[I::point(0.); 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            for k in 0..3 {
                a[r][c] = a[r][c].add(I::point(projection[r][k]).mul(j[k][c])?)?;
            }
        }
    }
    for r in 0..3 {
        for c in r + 1..3 {
            let sym = a[r][c].add(a[c][r])?.mul(I::point(0.5))?;
            a[r][c] = sym;
            a[c][r] = sym;
        }
    }
    let first = a[0][0];
    let second = a[0][0].mul(a[1][1])?.sub(a[0][1].mul(a[0][1])?)?;
    let third = a[0][0]
        .mul(a[1][1].mul(a[2][2])?.sub(a[1][2].mul(a[1][2])?)?)?
        .sub(a[0][1].mul(a[0][1].mul(a[2][2])?.sub(a[1][2].mul(a[0][2])?)?)?)?
        .add(a[0][2].mul(a[0][1].mul(a[1][2])?.sub(a[1][1].mul(a[0][2])?)?)?)?;
    Ok([first, second, third])
}
pub(super) fn inspect(source: &[[Vec<[f64; 3]>; 2]; 2], max_cells: usize) -> Result<Report> {
    let projection = proposal(source);
    let mut out = Report {
        certified: false,
        cells: 0,
        projection,
        principal_minor_lower: [f64::INFINITY; 3],
    };
    if projection.iter().flatten().any(|v| !v.is_finite()) {
        return Ok(out);
    }
    let net: Net = source.each_ref().map(|row| {
        row.each_ref()
            .map(|points| points.iter().map(|p| p.map(I::point)).collect())
    });
    let mut pending = vec![(net, [1.; 3], 0usize)];
    while let Some((net, width, depth)) = pending.pop() {
        if out.cells == max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let m = minors(jacobian(&net, width)?, projection)?;
        if m.iter().all(|m| m.lo > 0.) {
            for k in 0..3 {
                out.principal_minor_lower[k] = out.principal_minor_lower[k].min(m[k].lo);
            }
            continue;
        }
        if depth == 14 || m.iter().any(|m| m.hi <= 0.) {
            return Ok(out);
        }
        let axis = if depth < 8 { 2 } else { depth % 3 };
        let (left, right) = split(&net, axis)?;
        let mut width = width;
        width[axis] *= 0.5;
        pending.push((right, width, depth + 1));
        pending.push((left, width, depth + 1));
    }
    out.certified = true;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn cube(curved: bool) -> [[Vec<[f64; 3]>; 2]; 2] {
        std::array::from_fn(|a| {
            std::array::from_fn(|b| {
                (0..=3)
                    .map(|v| {
                        [
                            a as f64 + if curved && v == 1 { 0.125 } else { 0. },
                            b as f64,
                            v as f64 / 3.,
                        ]
                    })
                    .collect()
            })
        })
    }
    #[test]
    fn complete_cube_and_curved_cell_require_full_budget() {
        for curved in [false, true] {
            let net = cube(curved);
            let full = inspect(&net, 10000).unwrap();
            assert!(full.certified && full.cells > 0);
            assert!(full.principal_minor_lower.iter().all(|x| *x > 0.));
            for budget in [0, full.cells - 1] {
                let denied = inspect(&net, budget).unwrap();
                assert!(!denied.certified && denied.cells <= budget);
            }
        }
    }
    #[test]
    fn singular_and_folded_volumes_are_not_certified() {
        let mut singular = cube(false);
        for row in &mut singular {
            for points in row {
                for p in points {
                    p[2] = 0.;
                }
            }
        }
        assert!(!inspect(&singular, 10000).unwrap().certified);
        let mut folded = cube(false);
        for row in &mut folded {
            for points in row {
                points[2][2] = -1.;
            }
        }
        assert!(!inspect(&folded, 10000).unwrap().certified);
    }
}
