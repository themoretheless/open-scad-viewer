//! Common source cut ownership for admitted overlap carriers.
use super::{Curve, Result, refused};
pub(super) fn synchronize(
    curves: &[Curve],
    aliases: &[usize],
    cuts: &mut [Vec<f64>],
    tolerance: f64,
    work: &mut usize,
) -> Result<()> {
    let mut points = Vec::new();
    for (i, c) in curves.iter().enumerate() {
        for t in &cuts[i] {
            points.push((aliases[i], c.evaluate(*t)?.point));
        }
        if points.len() > 8192 {
            return Err(refused("Shared Bézier cut budget exceeded"));
        }
    }
    for (i, c) in curves.iter().enumerate() {
        for (alias, p) in &points {
            if *alias != aliases[i] {
                continue;
            }
            *work += 1;
            if *work > 100000 {
                return Err(refused("Shared Bézier cut projection budget exceeded"));
            }
            let mut best: Option<(f64, f64)> = None;
            for axis in 0..2 {
                if !nurbs_core::intersection::planar_scalar::monotone(c, axis) {
                    continue;
                }
                if let Some(t) = nurbs_core::intersection::planar_scalar::invert(c, axis, p[axis])?
                {
                    let q = c.evaluate(t)?.point;
                    let residual = (p[0] - q[0]).hypot(p[1] - q[1]);
                    if residual <= tolerance && best.is_none_or(|(r, _)| residual < r) {
                        best = Some((residual, t));
                    }
                }
            }
            if let Some((_, t)) = best {
                cuts[i].push(t)
            }
        }
    }
    Ok(())
}

/// Parameter distance has no geometric units. Coalesce cuts only when an
/// outward enclosure of the *original* source bounds the entire intervening
/// image within the endpoint roundoff budget. Keep both source-domain ends.
pub(super) fn coalesce(
    c: &Curve,
    cuts: &mut Vec<f64>,
    tolerance: f64,
    work: &mut usize,
) -> Result<()> {
    use nurbs_core::interval_eval::{Interval, evaluate_interval};
    let domain = c.domain();
    if cuts
        .iter()
        .any(|t| !t.is_finite() || *t < domain[0] || *t > domain[1])
    {
        return Err(refused("Bézier cut is outside its source domain"));
    }
    cuts.extend(domain);
    cuts.sort_by(f64::total_cmp);
    cuts.dedup();
    let scale = c
        .control_points
        .iter()
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    let limit = (4096. * f64::EPSILON * scale).min(tolerance * 0.001);
    let mut retained = Vec::new();
    for t in cuts.iter().copied() {
        if let Some(&previous) = retained.last() {
            *work += 1;
            if *work > 100000 {
                return Err(refused("Bézier cut enclosure budget exceeded"));
            }
            if previous != domain[0] || t != domain[1] {
                let enclosure = evaluate_interval(c, Interval::new(previous, t)?)?;
                let diameter = enclosure
                    .iter()
                    .try_fold(0f64, |sum, i| {
                        let width = (i.hi - i.lo).next_up();
                        (sum + width).is_finite().then_some((sum + width).next_up())
                    })
                    .unwrap_or(f64::INFINITY);
                if diameter <= limit {
                    if t == domain[1] {
                        *retained.last_mut().unwrap() = t;
                    }
                    continue;
                }
            }
        }
        retained.push(t);
    }
    *cuts = retained;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_cuts_use_another_coordinate_when_the_first_inverse_is_flat() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 1.], vec![-1., 2.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let mut b = a.clone();
        b.knots = vec![2., 2., 2., 2., 5., 5., 5., 5.];
        let mut cuts = vec![vec![0., 1. / 3., 1.], vec![2., 5.]];
        synchronize(&[a, b], &[0, 0], &mut cuts, 1e-7, &mut 0).unwrap();
        assert!(cuts[1].iter().any(|t| (t - 3.).abs() < 1e-12));
    }
}

#[cfg(test)]
mod cut_tests {
    use super::*;
    #[test]
    fn cuts_are_coalesced_in_world_units_and_never_by_absolute_parameter_distance() {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0., 0.], vec![1., 0.], vec![2., 4.]],
            weights: vec![1.; 4],
            periodic: false,
        };
        let mut cuts = vec![0., 1e-8, 1.];
        coalesce(&c, &mut cuts, 1e-7, &mut 0).unwrap();
        assert_eq!(cuts, vec![0., 1.]);
        let line = Curve::from_polyline(vec![vec![0., 0.], vec![1e6, 0.]]).unwrap();
        let mut cuts = vec![0., 5e-13, 1.];
        coalesce(&line, &mut cuts, 1e-8, &mut 0).unwrap();
        assert_eq!(cuts.len(), 3);
        let mut tiny = line.clone();
        tiny.knots = vec![0., 0., 1e-15, 1e-15];
        let mut cuts = vec![0., 5e-16, 1e-15];
        coalesce(&tiny, &mut cuts, 1e-8, &mut 0).unwrap();
        assert_eq!(cuts.len(), 3);
    }
}
