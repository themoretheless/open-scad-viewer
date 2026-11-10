//! Sufficient simplicity proof on original spatial rational Bezier curves.
use nurbs_core::{Error, Result, curve::Curve, interval_eval::Interval as I};
pub struct SpatialRimReport {
    pub injective: Vec<bool>,
    pub pairs: Vec<([usize; 2], bool)>,
    pub proven_simple: bool,
    pub cells: usize,
}
fn projection(p: &[f64], q: &[f64], direction: &[f64]) -> Result<I> {
    let mut sum = I::point(0.);
    for k in 0..3 {
        if p[k] == q[k] || direction[k] == 0. {
            continue;
        }
        sum = sum.add(
            I::point(p[k])
                .sub(I::point(q[k]))?
                .mul(I::point(direction[k]))?,
        )?;
    }
    Ok(sum)
}
fn monotone(c: &Curve) -> Result<bool> {
    let p = &c.control_points;
    if (0..3).any(|k| {
        (p[0][k] < p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] <= w[1][k]))
            || (p[0][k] > p[p.len() - 1][k] && p.windows(2).all(|w| w[0][k] >= w[1][k]))
    }) {
        return Ok(true);
    }
    let direction = (0..3)
        .map(|k| p[p.len() - 1][k] - p[0][k])
        .collect::<Vec<_>>();
    if direction.iter().any(|x| !x.is_finite()) {
        return Ok(false);
    }
    if projection(&p[p.len() - 1], &p[0], &direction)?.lo <= 0. {
        return Ok(false);
    }
    for w in p.windows(2) {
        if projection(&w[1], &w[0], &direction)?.lo < 0. {
            return Ok(false);
        }
    }
    Ok(true)
}
fn adjacent(a: &Curve, b: &Curve, join: &[f64]) -> Result<bool> {
    for pa in a.control_points.iter().filter(|p| p.as_slice() != join) {
        for pb in b.control_points.iter().filter(|p| p.as_slice() != join) {
            let va = (0..3).map(|k| pa[k] - join[k]).collect::<Vec<_>>();
            let vb = (0..3).map(|k| pb[k] - join[k]).collect::<Vec<_>>();
            let la = va.iter().map(|x| x * x).sum::<f64>().sqrt();
            let lb = vb.iter().map(|x| x * x).sum::<f64>().sqrt();
            if !(la > 0. && lb > 0. && la.is_finite() && lb.is_finite()) {
                continue;
            }
            let d = (0..3).map(|k| va[k] / la - vb[k] / lb).collect::<Vec<_>>();
            if d.iter().any(|x| !x.is_finite()) {
                continue;
            }
            let mut separate = true;
            for p in a.control_points.iter().filter(|p| p.as_slice() != join) {
                if projection(p, join, &d)?.lo <= 0. {
                    separate = false;
                    break;
                }
            }
            if !separate {
                continue;
            }
            for p in b.control_points.iter().filter(|p| p.as_slice() != join) {
                if projection(p, join, &d)?.hi >= 0. {
                    separate = false;
                    break;
                }
            }
            if separate {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
/// Every positive rational Bernstein basis is positive in the open interval.
/// Monotone original controls prove each curve injective; interval-certified
/// separating planes exclude extra adjacent contacts. Nonadjacent pairs use
/// whole-curve distance bounds. Failure is unproven, never an intersection.
pub fn inspect_spatial_rim(
    curves: &[Curve],
    tolerance_mm: f64,
    max_cells: usize,
) -> Result<SpatialRimReport> {
    let bad = || {
        Error::new(
            "BREP_SPATIAL_RIM_INPUT",
            "Need 3..32 clamped spatial Bezier curves, a positive tolerance and bounded work",
        )
    };
    if !(3..=32).contains(&curves.len())
        || !(1..=100000).contains(&max_cells)
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
    {
        return Err(bad());
    }
    for (i, c) in curves.iter().enumerate() {
        c.validate()?;
        let n = c.control_points.len();
        let d = c.domain();
        if c.periodic
            || c.degree + 1 != n
            || c.control_points[0].len() != 3
            || !c.knots[..=c.degree].iter().all(|&t| t == d[0])
            || !c.knots[n..].iter().all(|&t| t == d[1])
            || c.control_points.last() != curves[(i + 1) % curves.len()].control_points.first()
        {
            return Err(bad());
        }
    }
    let mut out = SpatialRimReport {
        injective: curves.iter().map(monotone).collect::<Result<Vec<_>>>()?,
        pairs: Vec::new(),
        proven_simple: false,
        cells: 0,
    };
    let n = curves.len();
    for a in 0..n {
        for b in a + 1..n {
            let proven = if b == a + 1 || (a == 0 && b == n - 1) {
                let join = if b == a + 1 {
                    &curves[b].control_points[0]
                } else {
                    &curves[0].control_points[0]
                };
                adjacent(&curves[a], &curves[b], join)?
            } else if out.cells < max_cells {
                let budget = (max_cells - out.cells).min(2048);
                let report = nurbs_core::curve_distance::distance(
                    &curves[a],
                    &curves[b],
                    tolerance_mm,
                    budget,
                )?;
                out.cells += report.cells;
                report.distance_interval_mm[0] > 0.
            } else {
                false
            };
            out.pairs.push(([a, b], proven));
        }
    }
    out.proven_simple = out.injective.iter().all(|x| *x) && out.pairs.iter().all(|(_, p)| *p);
    Ok(out)
}
