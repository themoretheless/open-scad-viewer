//! class a algorithm: chain_segments.
use super::*;
/// Сцепка сегментов в полилинии по близости концов.
pub fn chain_segments(
    segments: Vec<([f64; 2], [f64; 2])>,
    tolerance: f64,
    limits: &Limits,
) -> Result<Vec<Vec<[f64; 2]>>> {
    let tol2 = tolerance * tolerance;
    let near = |a: [f64; 2], b: [f64; 2]| {
        let du = a[0] - b[0];
        let dv = a[1] - b[1];
        du * du + dv * dv <= tol2
    };
    let mut pool: Vec<[f64; 2]> = Vec::with_capacity(segments.len() * 2);
    let mut free: Vec<(usize, usize)> = Vec::with_capacity(segments.len());
    for (a, b) in segments {
        pool.push(a);
        pool.push(b);
        free.push((pool.len() - 2, pool.len() - 1));
    }
    let mut lines: Vec<Vec<[f64; 2]>> = Vec::new();
    // Единый бюджет точек сцепленной полилинии (стадия class-a.chaining).
    let mut guard = Budget::with_iterations(limits.max_polyline_points)?
        .guard("class-a.chaining");
    while let Some((ia, ib)) = free.pop() {
        let mut line = vec![pool[ia], pool[ib]];
        loop {
            if line.len() >= limits.max_polyline_points {
                return Err(resource("class_a polyline point budget exceeded"));
            }
            let mut grown = false;
            let mut k = 0;
            while k < free.len() {
                let (ca, cb) = free[k];
                let (head, tail) = (line[0], line[line.len() - 1]);
                // Кандидат (присоединяемая точка, идёт ли на хвост).
                let candidate: Option<(usize, bool)> = if near(pool[ca], tail) {
                    Some((cb, true))
                } else if near(pool[cb], tail) {
                    Some((ca, true))
                } else if near(pool[ca], head) {
                    Some((cb, false))
                } else if near(pool[cb], head) {
                    Some((ca, false))
                } else {
                    None
                };
                let Some((ipoint, at_tail)) = candidate else {
                    k += 1;
                    continue;
                };
                let p = pool[ipoint];
                // Отбрасываем мгновенный возврат назад (A-B-A): дубликат узла
                // марширующих квадратов, а не реальный излом на pi.
                let backtrack = if at_tail && line.len() >= 2 {
                    near(p, line[line.len() - 2])
                } else if !at_tail && line.len() >= 2 {
                    near(p, line[1])
                } else {
                    false
                };
                if backtrack {
                    free.swap_remove(k);
                    continue;
                }
                if at_tail {
                    guard.tick()?;
                    line.push(p);
                } else {
                    guard.tick()?;
                    line.insert(0, p);
                }
                free.swap_remove(k);
                grown = true;
            }
            if !grown {
                break;
            }
        }
        lines.push(line);
    }
    Ok(lines)
}
