//! Bounded numerical construction. Every proposed box is independently proved.
use super::*;
fn site(s: &Surface, p: [f64; 2], work: &mut Work) -> Option<([f64; 3], ([f64; 3], [f64; 3]))> {
    if work.report.evaluations == work.limits.max_evaluations {
        return None;
    }
    work.report.evaluations += 1;
    let value = s.evaluate(p[0], p[1]).ok()?;
    let derivatives = if let Some(j) = value.first_derivatives() {
        j
    } else {
        // A multiple knot can legitimately suppress evaluator derivatives.
        // A nearby one-sided Jacobian only proposes Newton steps. The native
        // common operator still proves all original pieces, including the knot.
        let mut nearby = p;
        for axis in 0..2 {
            let (knots, degree, n) = if axis == 0 {
                (&s.knots_u, s.degree_u, s.control_points.len())
            } else {
                (&s.knots_v, s.degree_v, s.control_points[0].len())
            };
            let span = (degree..n)
                .find(|&i| knots[i] < knots[i + 1] && knots[i] <= p[axis] && p[axis] < knots[i + 1])
                .or_else(|| {
                    (degree..n).rev().find(|&i| {
                        knots[i] < knots[i + 1] && knots[i] <= p[axis] && p[axis] <= knots[i + 1]
                    })
                })?;
            nearby[axis] = p[axis] + 1e-6 * (0.5 * knots[span] + 0.5 * knots[span + 1] - p[axis]);
        }
        if work.report.evaluations == work.limits.max_evaluations {
            return None;
        }
        work.report.evaluations += 1;
        s.evaluate(nearby[0], nearby[1]).ok()?.first_derivatives()?
    };
    Some((
        [value.point[0], value.point[1], value.point[2]],
        derivatives,
    ))
}
fn system(
    source: &FiberSources,
    x: [f64; 4],
    work: &mut Work,
) -> Result<Option<([f64; 3], [[f64; 4]; 3])>> {
    let n = natural(source);
    let p: [f64; 4] = std::array::from_fn(|k| {
        n[k][0]
            + (if periodic(source, k) {
                x[k].rem_euclid(1.)
            } else {
                x[k]
            }) * (n[k][1] - n[k][0])
    });
    let (Some((a, (au, av))), Some((b, (bu, bv)))) = (
        site(source.first(), [p[0], p[1]], work),
        site(source.second(), [p[2], p[3]], work),
    ) else {
        return Ok(None);
    };
    let cols = [au, av, bu.map(|v| -v), bv.map(|v| -v)];
    Ok(Some((
        std::array::from_fn(|k| a[k] - b[k]),
        std::array::from_fn(|k| std::array::from_fn(|j| cols[j][k] * (n[j][1] - n[j][0]))),
    )))
}
fn solve(j: [[f64; 4]; 3], rhs: [f64; 3], free: usize) -> Option<[f64; 3]> {
    let unknown = (0..4).filter(|&k| k != free).collect::<Vec<_>>();
    let r = inverse(std::array::from_fn(|i| {
        std::array::from_fn(|k| j[i][unknown[k]])
    }))?;
    let step = std::array::from_fn(|i| (0..3).map(|k| r[i][k] * rhs[k]).sum::<f64>());
    step.iter().all(|x| x.is_finite()).then_some(step)
}
pub(super) fn polish(
    source: &FiberSources,
    mut x: [f64; 4],
    free: usize,
    work: &mut Work,
) -> Result<Option<[f64; 4]>> {
    let unknown = (0..4).filter(|&k| k != free).collect::<Vec<_>>();
    for _ in 0..24 {
        let Some((f, j)) = system(source, x, work)? else {
            return Ok(None);
        };
        let Some(step) = solve(j, f, free) else {
            return Ok(None);
        };
        let mut scale = 1.;
        while scale >= 1. / 1024.
            && unknown.iter().enumerate().any(|(i, &k)| {
                !periodic(source, k) && !(0. ..=1.).contains(&(x[k] - scale * step[i]))
            })
        {
            scale *= 0.5;
        }
        if scale < 1. / 1024. {
            return Ok(None);
        }
        for (i, &k) in unknown.iter().enumerate() {
            x[k] -= scale * step[i];
            if periodic(source, k) {
                x[k] = x[k].rem_euclid(1.);
            }
        }
        if step.iter().all(|x| x.abs() * scale < 1e-12) {
            return Ok(Some(x));
        }
    }
    Ok(None)
}
pub(super) fn chart(
    source: &FiberSources,
    x: [f64; 4],
    free: usize,
    h: f64,
    work: &mut Work,
) -> Result<Option<[[f64; 2]; 4]>> {
    let Some((_, j)) = system(source, x, work)? else {
        return Ok(None);
    };
    let Some(slope) = solve(j, std::array::from_fn(|k| -j[k][free]), free) else {
        return Ok(None);
    };
    let mut tangent = [1.; 4];
    let mut i = 0;
    for k in 0..4 {
        if k != free {
            tangent[k] = slope[i];
            i += 1;
        }
    }
    let mut deviation = [0_f64; 4];
    for direction in [-1., 1.] {
        let mut seed = x;
        seed[free] = if periodic(source, free) {
            x[free] + direction * h
        } else {
            (x[free] + direction * h).clamp(0., 1.)
        };
        let delta = seed[free] - x[free];
        if let Some(p) = polish(source, seed, free, work)? {
            for k in 0..4 {
                let mut diff = p[k] - x[k];
                if periodic(source, k) && k != free {
                    diff -= diff.round();
                }
                deviation[k] = deviation[k].max((diff - delta * tangent[k]).abs());
            }
        }
    }
    let n = natural(source);
    Ok(Some(std::array::from_fn(|k| {
        let radius = if k == free {
            h
        } else {
            1.25 * h * tangent[k].abs() + (1.5 * deviation[k]).max(8. * h * h)
        };
        let (lo, hi) = if periodic(source, k) {
            ((x[k] - radius).max(-1.), (x[k] + radius).min(2.))
        } else {
            ((x[k] - radius).max(0.), (x[k] + radius).min(1.))
        };
        [
            n[k][0] + lo * (n[k][1] - n[k][0]),
            n[k][0] + hi * (n[k][1] - n[k][0]),
        ]
    })))
}
