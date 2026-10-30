//! normal alignment algorithm: restrict_unit_net.
use super::*;
/// Restrict all coefficients together by two de Casteljau splits per axis.
/// This keeps closed endpoints and avoids a separate blossom for every cell.
pub fn restrict_unit_net(net: &Net, domain: [[f64; 2]; 2]) -> Result<Net> {
    check(
        domain.iter().all(|d| {
            d[0].is_finite() && d[1].is_finite() && 0. <= d[0] && d[0] <= d[1] && d[1] <= 1.
        }),
        "Closed unit polynomial rectangle",
    )?;
    fn split(row: &[[I; 4]], t: I) -> Result<(Vec<[I; 4]>, Vec<[I; 4]>)> {
        let t = t.intersect(0., 1.)?;
        let beta = I::point(1.).sub(t)?.intersect(0., 1.)?;
        let mut current = row.to_vec();
        let mut left = vec![current[0]];
        let mut right = vec![*current.last().unwrap()];
        while current.len() > 1 {
            let mut next = Vec::new();
            for pair in current.windows(2) {
                let mut h = [I::point(0.); 4];
                for k in 0..4 {
                    h[k] = pair[0][k].mul(beta)?.add(pair[1][k].mul(t)?)?.intersect(
                        pair[0][k].lo.min(pair[1][k].lo),
                        pair[0][k].hi.max(pair[1][k].hi),
                    )?;
                }
                next.push(h);
            }
            current = next;
            left.push(current[0]);
            right.push(*current.last().unwrap());
        }
        right.reverse();
        Ok((left, right))
    }
    fn row_restrict(row: &[[I; 4]], d: [f64; 2]) -> Result<Vec<[I; 4]>> {
        if d == [0., 1.] {
            return Ok(row.to_vec());
        }
        if d[0] == d[1] {
            let value = *split(row, I::point(d[0]))?.0.last().unwrap();
            return Ok(vec![value; row.len()]);
        }
        let left = if d[1] == 1. {
            row.to_vec()
        } else {
            split(row, I::point(d[1]))?.0
        };
        if d[0] == 0. {
            return Ok(left);
        }
        Ok(split(&left, I::point(d[0]).div(I::point(d[1]))?)?.1)
    }
    let mut out = net.clone();
    for v in 0..net[0].len() {
        let row = net.iter().map(|r| r[v]).collect::<Vec<_>>();
        let restricted = row_restrict(&row, domain[0])?;
        for u in 0..net.len() {
            out[u][v] = restricted[u];
        }
    }
    for row in &mut out {
        *row = row_restrict(row, domain[1])?;
    }
    Ok(out)
}
