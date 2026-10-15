//! Bounded retained-carrier agreement, including degree elevation and positive
//! projective reparameterization. No sampled or fitted replacement is created.
use super::planar_scalar::{Relation, admit, invert, monotone};
use crate::{Result, curve::Curve, interval_eval::Interval as I};
fn choose(n: usize, k: usize) -> f64 {
    (0..k).fold(1usize, |v, i| v * (n - i) / (i + 1)) as f64
}
fn leading(c: &Curve) -> Option<(usize, f64)> {
    for i in 1..c.control_points.len() {
        let a = &c.control_points[0];
        let b = &c.control_points[i];
        let length = (b[0] - a[0]).hypot(b[1] - a[1]);
        if length > 0. {
            return Some((
                i,
                length * choose(c.degree, i) * (c.weights[i] / c.weights[0]),
            ));
        }
    }
    None
}
/// Every point of A has a corresponding point of B, and vice versa, within
/// the returned L1 bound under an orientation-preserving projective map.
/// A numerical lambda only proposes the map; interval coefficients admit it.
pub fn agreement(a: &Curve, b: &Curve) -> Result<Option<f64>> {
    a.validate()?;
    b.validate()?;
    if !admit(a) || !admit(b) {
        return Ok(None);
    }
    let (Some((ka, va)), Some((kb, vb))) = (leading(a), leading(b)) else {
        return Ok(None);
    };
    if ka != kb {
        return Ok(None);
    }
    let lambda = (va / vb).powf(1. / ka as f64);
    if !lambda.is_finite() || lambda <= 0. {
        return Ok(None);
    }
    let weights = |c: &Curve, l: f64| -> Result<Vec<I>> {
        let max = c.weights.iter().copied().fold(0., f64::max);
        let mut factor = I::point(1.);
        let mut out = Vec::new();
        for w in &c.weights {
            out.push(I::point(*w).div(I::point(max))?.mul(factor)?);
            factor = factor.mul(I::point(l))?;
        }
        Ok(out)
    };
    let wa = weights(a, 1.)?;
    let wb = weights(b, lambda)?;
    let lower = I::point(wa.iter().map(|w| w.lo).fold(f64::INFINITY, f64::min)).mul(I::point(
        wb.iter().map(|w| w.lo).fold(f64::INFINITY, f64::min),
    ))?;
    if lower.lo <= 0. {
        return Ok(None);
    }
    let n = a.degree;
    let m = b.degree;
    let mut bound = I::point(0.);
    for axis in 0..2 {
        let mut coefficients = vec![I::point(0.); n + m + 1];
        for i in 0..=n {
            for j in 0..=m {
                let factor =
                    I::point(choose(n, i) * choose(m, j)).div(I::point(choose(n + m, i + j)))?;
                let term = I::point(a.control_points[i][axis])
                    .sub(I::point(b.control_points[j][axis]))?
                    .mul(wa[i])?
                    .mul(wb[j])?
                    .mul(factor)?;
                coefficients[i + j] = coefficients[i + j].add(term)?;
            }
        }
        let error = coefficients
            .iter()
            .map(|v| v.lo.abs().max(v.hi.abs()))
            .fold(0., f64::max);
        bound = bound.add(I::point(error).div(lower)?)?;
    }
    Ok(Some(bound.hi))
}
pub fn overlap(a: &Curve, b: &Curve, tolerance: f64) -> Result<Option<Relation>> {
    a.validate()?;
    b.validate()?;
    crate::check(
        tolerance.is_finite() && tolerance > 0.,
        "Positive coincidence tolerance required",
    )?;
    if !admit(a) || !admit(b) {
        return Ok(None);
    }
    for axis in 0..2 {
        if !monotone(a, axis) || !monotone(b, axis) {
            continue;
        }
        let aa = a.control_points[0][axis];
        let az = a.control_points.last().unwrap()[axis];
        let ba = b.control_points[0][axis];
        let bz = b.control_points.last().unwrap()[axis];
        let lo = aa.min(az).max(ba.min(bz));
        let hi = aa.max(az).min(ba.max(bz));
        if hi <= lo {
            continue;
        }
        let (Some(t0), Some(t1), Some(u0), Some(u1)) = (
            invert(a, axis, lo)?,
            invert(a, axis, hi)?,
            invert(b, axis, lo)?,
            invert(b, axis, hi)?,
        ) else {
            continue;
        };
        let first = [t0.min(t1), t0.max(t1)];
        let second = [u0.min(u1), u0.max(u1)];
        if first[1] <= first[0] || second[1] <= second[0] {
            continue;
        }
        let x = a.trim(first[0], first[1])?;
        let mut y = b.trim(second[0], second[1])?;
        let reversed = (az > aa) != (bz > ba);
        if reversed {
            y = y.reverse()?;
        }
        let scale = x
            .control_points
            .iter()
            .chain(&y.control_points)
            .flatten()
            .map(|v| v.abs())
            .fold(1., f64::max);
        // Carrier coalescence is confined to roundoff, far below the user's
        // geometric tolerance. Nearby distinct contacts are not overlap guesses.
        let limit = (4096. * f64::EPSILON * scale).min(tolerance * 0.001);
        if agreement(&x, &y)
            .ok()
            .flatten()
            .is_some_and(|error| error <= limit)
        {
            return Ok(Some(Relation::Overlap {
                points: Vec::new(),
                first,
                second,
                reversed,
                linear: false,
            }));
        }
    }
    endpoint_overlap(a, b, tolerance)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cubic() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 1.], vec![3., 3.]],
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn partial_curved_overlap_keeps_original_parameter_domains_and_reversal() {
        let a = cubic();
        let b = a.trim(0.25, 0.75).unwrap().reverse().unwrap();
        let Some(Relation::Overlap {
            first,
            second,
            reversed,
            ..
        }) = overlap(&a, &b, 1e-7).unwrap()
        else {
            panic!("overlap refused")
        };
        assert!((first[0] - 0.25).abs() < 1e-12 && (first[1] - 0.75).abs() < 1e-12);
        assert_eq!(second, b.domain());
        assert!(reversed);
    }
    #[test]
    fn projective_parameter_map_is_admitted_by_coefficients_and_not_endpoint_guessing() {
        let a = cubic();
        let mut b = a.trim(0.25, 0.75).unwrap();
        for (i, w) in b.weights.iter_mut().enumerate() {
            *w *= 2_f64.powi(i as i32);
        }
        assert!(overlap(&a, &b, 1e-7).unwrap().is_some());
        b.control_points[1][1] += 1e-5;
        assert!(overlap(&a, &b, 1e-7).unwrap().is_none());
    }
    #[test]
    fn different_degrees_and_near_distinct_carriers_do_not_use_endpoint_only_matching() {
        let a = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![1., 2.], vec![2., 3.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let b = a.elevate(3).unwrap();
        assert!(agreement(&a, &b).unwrap().unwrap() < 1e-12);
        let mut different = b.clone();
        for p in &mut different.control_points {
            p[1] += 1e-9;
        }
        assert!(overlap(&a, &different, 1e-7).unwrap().is_none());
    }
}

fn locate(c: &Curve, p: &[f64], limit: f64) -> Result<Option<f64>> {
    for axis in 0..2 {
        if !monotone(c, axis) {
            continue;
        }
        let a = c.control_points[0][axis];
        let b = c.control_points.last().unwrap()[axis];
        let target = p[axis].clamp(a.min(b), a.max(b));
        if (target - p[axis]).abs() > limit {
            continue;
        }
        let Some(t) = invert(c, axis, target)? else {
            continue;
        };
        let q = c.evaluate(t)?.point;
        if (q[0] - p[0]).hypot(q[1] - p[1]) <= limit {
            return Ok(Some(t));
        }
    }
    Ok(None)
}
fn outside(c: &Curve, range: [f64; 2]) -> Result<Vec<Curve>> {
    let [a, b] = c.domain();
    let mut result = Vec::new();
    if a < range[0] {
        result.push(c.trim(a, range[0])?);
    }
    if range[1] < b {
        result.push(c.trim(range[1], b)?);
    }
    Ok(result)
}
/// Each span may be injective on a different coordinate. Endpoint ownership
/// proposes overlap ranges; coefficient bounds admit the complete carriers.
/// The complement is audited so a nodal carrier cannot lose isolated contacts.
fn endpoint_overlap(a: &Curve, b: &Curve, tolerance: f64) -> Result<Option<Relation>> {
    // The complement audit relies on injectivity of both complete source
    // spans. Otherwise a point inside the overlap may also occur outside it.
    if ![a, b].iter().all(|c| (0..2).any(|axis| monotone(c, axis))) {
        return Ok(None);
    }
    let scale = a
        .control_points
        .iter()
        .chain(&b.control_points)
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    let limit = (4096. * f64::EPSILON * scale).min(tolerance * 0.001);
    let mut matches = Vec::new();
    for (t, p) in [
        (a.domain()[0], &a.control_points[0]),
        (a.domain()[1], a.control_points.last().unwrap()),
    ] {
        if let Some(u) = locate(b, p, limit)? {
            matches.push([t, u]);
        }
    }
    for (u, p) in [
        (b.domain()[0], &b.control_points[0]),
        (b.domain()[1], b.control_points.last().unwrap()),
    ] {
        if let Some(t) = locate(a, p, limit)? {
            matches.push([t, u]);
        }
    }
    matches.sort_by(|a, b| a[0].total_cmp(&b[0]));
    matches.dedup_by(|a, b| (a[0] - b[0]).abs() < 1e-12 && (a[1] - b[1]).abs() < 1e-12);
    let mut candidates = Vec::new();
    for i in 0..matches.len() {
        for j in i + 1..matches.len() {
            if matches[j][0] - matches[i][0] > 1e-12
                && (matches[j][1] - matches[i][1]).abs() > 1e-12
            {
                candidates.push((matches[i], matches[j]));
            }
        }
    }
    candidates.sort_by(|a, b| (b.1[0] - b.0[0]).total_cmp(&(a.1[0] - a.0[0])));
    for (start, end) in candidates {
        let first = [start[0], end[0]];
        let second = [start[1].min(end[1]), start[1].max(end[1])];
        let x = a.trim(first[0], first[1])?;
        let mut y = b.trim(second[0], second[1])?;
        let reversed = end[1] < start[1];
        if reversed {
            y = y.reverse()?;
        }
        if !agreement(&x, &y).ok().flatten().is_some_and(|e| e <= limit) {
            continue;
        }
        let mut points = Vec::new();
        let mut complete = true;
        for x in outside(a, first)? {
            for y in outside(b, second)? {
                let lift = |mut c: Curve| {
                    for p in &mut c.control_points {
                        p.push(0.);
                    }
                    c
                };
                let report = super::intersect_curve_curve_report(&lift(x.clone()), &lift(y), None)?;
                if !report.report.unresolved.is_empty() {
                    complete = false;
                    break;
                }
                for event in report.report.components {
                    if event.kind != super::CurveCurveComponentKind::Point
                        || event.residual > tolerance
                    {
                        complete = false;
                        break;
                    }
                    points.push([event.first, event.second]);
                }
                if !complete {
                    break;
                }
            }
            if !complete {
                break;
            }
        }
        if complete {
            return Ok(Some(Relation::Overlap {
                first,
                second,
                reversed,
                linear: false,
                points,
            }));
        }
    }
    Ok(None)
}
#[cfg(test)]
mod endpoint_tests {
    use super::*;
    #[test]
    fn flat_coordinate_does_not_prevent_endpoint_ownership() {
        let c = source(vec![
            vec![0., 0.],
            vec![1., 1.],
            vec![-1., 2.],
            vec![3., 3.],
        ]);
        let p = c.evaluate(1. / 3.).unwrap().point;
        let t = locate(&c, &p, 1e-12).unwrap().unwrap();
        assert!((t - 1. / 3.).abs() < 1e-12);
    }
    fn source(points: Vec<Vec<f64>>) -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: points,
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    #[test]
    fn curved_overlap_does_not_require_a_shared_global_monotone_axis() {
        let c = source(vec![
            vec![0., 0.],
            vec![6., -2.],
            vec![11., -1.],
            vec![7., 11.],
        ]);
        let a = c.trim(0., 0.625).unwrap();
        let b = c.trim(0.375, 1.).unwrap();
        assert!(monotone(&a, 0) && !monotone(&a, 1));
        assert!(monotone(&b, 1) && !monotone(&b, 0));
        let Some(Relation::Overlap {
            first,
            second,
            points,
            ..
        }) = overlap(&a, &b, 1e-7).unwrap()
        else {
            panic!("separate-axis overlap refused")
        };
        for range in [first, second] {
            assert!((range[0] - 0.375).abs() < 1e-12 && (range[1] - 0.625).abs() < 1e-12);
        }
        assert!(points.is_empty());
    }
    #[test]
    fn a_nodal_carrier_keeps_an_isolated_contact_outside_its_shared_subspan() {
        let c = source(vec![
            vec![0., 0.],
            vec![0., 8.],
            vec![-8., 0.],
            vec![0., 0.],
        ]);
        let a = c.trim(0., 0.625).unwrap();
        let b = c.trim(0.375, 1.).unwrap();
        assert!(overlap(&c, &b, 1e-7).unwrap().is_none());
        let Some(Relation::Overlap { first, points, .. }) = overlap(&a, &b, 1e-7).unwrap() else {
            panic!("nodal overlap refused")
        };
        assert!((first[0] - 0.375).abs() < 1e-12 && (first[1] - 0.625).abs() < 1e-12);
        assert!(
            points
                .iter()
                .any(|p| p[0].abs() < 1e-10 && (p[1] - 1.).abs() < 1e-10),
            "{points:?}"
        );
    }
}
#[cfg(test)]
mod interior_node_test {
    use super::*;
    #[test]
    fn independent_axis_overlap_retains_an_interior_nodal_contact() {
        let c = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![2.5, -17.5],
                vec![1.5, 29.5],
                vec![-29.5, -1.5],
                vec![17.5, -2.5],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let a = c.trim(1. / 12., 13. / 24.).unwrap();
        let b = c.trim(11. / 24., 11. / 12.).unwrap();
        let Some(Relation::Overlap { points, .. }) = overlap(&a, &b, 1e-7).unwrap() else {
            panic!("interior-node overlap refused")
        };
        assert!(
            points
                .iter()
                .any(|p| (p[0] - 1. / 6.).abs() < 1e-9 && (p[1] - 5. / 6.).abs() < 1e-9),
            "{points:?}"
        );
    }
}
