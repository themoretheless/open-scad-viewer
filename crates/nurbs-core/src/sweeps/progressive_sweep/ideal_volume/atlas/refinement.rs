//! Source-paid refinement proposals. Neither point probes nor width scores
//! authorize geometry; every selected subcell still needs the same full SPD
//! proof under ONE fixed metric over the complete joint hull.
use super::*;
pub(super) fn promising(
    pieces: &[Box3],
    projection: [[f64; 3]; 3],
    scale: [f64; 3],
    metric: [[f64; 3]; 3],
    query: &mut impl FnMut(Box3, &mut IdealVolumeReport) -> Result<Enclosed>,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<bool> {
    for b in pieces {
        // Keep cheap corner rejection first. Rational carrier jets can be
        // benign at every corner but violate this candidate metric on a face
        // or in the interior. Extra original-point queries ONLY reject a
        // proposal; all successful candidates still need the complete cover.
        let corners = (0..8).map(|mask| std::array::from_fn(|i| [b[i][(mask >> i) & 1]; 2]));
        let mids = (0usize..27).filter(|&code|
            (0..3).any(|i| (code / 3usize.pow(i)) % 3 == 1)
        ).map(|code| std::array::from_fn(|i| {
            let position = (code / 3usize.pow(i as u32)) % 3;
            let value = match position {
                0 => b[i][0], 2 => b[i][1],
                _ => crate::distance_bounds::midpoint(b[i][0], b[i][1]),
            };
            [value; 2]
        }));
        for point in corners.chain(mids) {
            if out.cells == max {
                return Ok(false);
            }
            out.cells += 1;
            let Enclosed::Value(e) = query(point, out)? else {
                return Ok(false);
            };
            let positive = projected(e.columns, projection)
                .and_then(|c| scaled(c, scale))
                .and_then(|c| symmetric_positive(c, metric))
                .unwrap_or(false);
            if !positive {
                #[cfg(test)]
                if std::env::var_os("OSV_TRACE_ACTUAL_METRIC").is_some() {
                    eprintln!("METRIC rejected point={point:?} columns={:?}",e.columns);
                }
                return Ok(false);
            }
        }
    }
    Ok(true)
}
fn score(e: Enclosed, projection: [[f64; 3]; 3], scale: [f64; 3], metric: [[f64; 3]; 3]) -> f64 {
    let Enclosed::Value(e) = e else {
        return f64::INFINITY;
    };
    let Ok(c) = projected(e.columns, projection)
        .and_then(|c| scaled(c, scale))
        .and_then(|c| projected(c, metric))
    else {
        return f64::INFINITY;
    };
    let sum = c.iter().flatten().map(|x| x.hi - x.lo).sum::<f64>();
    if sum.is_finite() { sum } else { f64::INFINITY }
}
pub(super) fn axis(
    b: Box3,
    projection: [[f64; 3]; 3],
    scale: [f64; 3],
    metric: [[f64; 3]; 3],
    query: &mut impl FnMut(Box3, &mut IdealVolumeReport) -> Result<Enclosed>,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<usize>> {
    let Some((fallback, _)) = widest(b) else {
        return Ok(None);
    };
    let mut best = (f64::INFINITY, fallback);
    for axis in 0..3 {
        let Some(children) = split_axis(b, axis) else {
            continue;
        };
        if out.cells == max {
            return Ok(None);
        }
        out.cells += 1;
        let a = query(children[0], out)?;
        if out.cells == max {
            return Ok(None);
        }
        let z = query(children[1], out)?;
        let candidate =
            score(a, projection, scale, metric).max(score(z, projection, scale, metric));
        if candidate < best.0 {
            best = (candidate, axis);
        }
    }
    Ok(Some(best.1))
}
