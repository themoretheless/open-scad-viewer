//! Exact tensor iso-traces under their authored trim clocks.
use super::*;
use cad_predicates::{AuthoredScalar, Limits, PredicateContext, SourceArena, ToleranceContext};
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
fn exact_float(value: Q) -> Option<f64> {
    value.to_f64().filter(|&x| Q::from_float(x) == Some(value))
}

/// Extend the exact cyclic basis through adjacent periods, then keep the
/// controls supporting its nonempty spans. The caller proves seam identity.
fn lifted_basis(
    knots: &[f64],
    degree: usize,
    count: usize,
    range: [f64; 2],
) -> Option<(Vec<usize>, Vec<f64>)> {
    let q = |v| Q::from_float(v).unwrap();
    let low = q(knots[degree]);
    let period = q(knots[count]) - &low;
    let tile = |u| ((q(u) - &low) / &period).floor().to_i64();
    let [start, end] = [tile(range[0])?, tile(range[1])?];
    if !(-1_000_000..=1_000_000).contains(&start)
        || !(-1_000_000..=1_000_000).contains(&end)
        || end - start > 3
    {
        return None;
    }
    let cycle = (count - degree) as i64;
    let first = (degree..count).find(|&i| knots[i] < knots[i + 1])? as i64;
    let last = (degree..count).rfind(|&i| knots[i] < knots[i + 1])? as i64;
    let first_control = first - degree as i64 + start * cycle;
    let last_control = last + end * cycle;
    let indices = (first_control..=last_control)
        .map(|i| i.rem_euclid(cycle) as usize)
        .collect();
    let knots = (first_control..=last_control + degree as i64 + 1)
        .map(|i| {
            exact_float(
                q(knots[i.rem_euclid(cycle) as usize])
                    + &period * Q::from_integer(i.div_euclid(cycle).into()),
            )
        })
        .collect::<Option<Vec<_>>>()?;
    Some((indices, knots))
}

pub(super) fn uv_clamped(p: &Curve) -> bool {
    let [lo, hi] = p.domain();
    p.knots[..=p.degree].iter().all(|&k| k == lo)
        && p.knots[p.control_points.len()..].iter().all(|&k| k == hi)
}

pub(super) fn verify(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    cr: [f64; 2],
    pr: [f64; 2],
    max_work: u64,
) -> Result<Option<cad_predicates::BezierIdentityDecision>> {
    let pd = p.domain();
    let clamped = uv_clamped(p);
    let count = p.control_points.len();
    if s.control_points
        .len()
        .saturating_mul(s.control_points[0].len())
        > 4096
        || c.degree > 32
        || c.control_points.len() > 4096
        || s.degree_u > 32
        || s.degree_v > 32
        || (c.periodic && (cr != c.domain() || !(s.periodic_u || s.periodic_v)))
        || p.periodic
        || !(1..=5).contains(&p.degree)
        || count > 4096
        || c.control_points.iter().any(|p| p.len() != 3)
        || p.control_points.iter().any(|p| p.len() != 2)
    {
        return Ok(None);
    }
    let mut affine_work = 0;
    let active;
    let endpoints = if clamped {
        p
    } else {
        if max_work == 0 {
            return Ok(None);
        }
        let report = crate::core::exact_curve_arithmetic::clamp_active(p, None, max_work)?;
        let Some(curve) = report.curve else {
            return Ok(None);
        };
        if report.work_used >= max_work {
            return Ok(None);
        }
        affine_work = report.work_used;
        active = curve;
        &active
    };
    let a = &endpoints.control_points[0];
    let z = endpoints.control_points.last().unwrap();
    if !(0..2).any(|axis| a[axis] == z[axis] && a[1 - axis] != z[1 - axis]) {
        return Ok(None);
    }
    if !clamped || p.degree != 1 || count != 2 || p.weights[0] != p.weights[1] {
        let affine = Curve {
            degree: 1,
            knots: vec![pd[0], pd[0], pd[1], pd[1]],
            control_points: vec![a.clone(), z.clone()],
            weights: vec![1.; 2],
            periodic: false,
        };
        let Some(proof) = crate::curve_partition_agreement::verify_exact_bspline_restriction(
            p,
            &affine,
            max_work - affine_work,
        )?
        else {
            return Ok(None);
        };
        if proof.outcome != cad_predicates::BezierIdentity::Equal
            || proof.work_used >= max_work - affine_work
        {
            return Ok(None);
        }
        affine_work += proof.work_used;
    }
    if c.periodic {
        // The affine-clock proof above binds these exact active endpoints
        // to the original UV carrier. Use the periodic trace owner on the
        // complete world clock rather than extracting a nonperiodic strip.
        let q = |v: f64| Q::from_float(v).unwrap();
        let [lo, hi] = pd.map(q);
        let active = |t: f64| {
            let f = (q(t) - &lo) / (&hi - &lo);
            a.iter()
                .zip(z)
                .map(|(&a, &z)| exact_float(q(a) + &f * (q(z) - q(a))))
                .collect::<Option<Vec<_>>>()
        };
        let (Some(start), Some(end)) = (active(pr[0]), active(pr[1])) else {
            return Ok(None);
        };
        let side = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![start, end],
            weights: vec![1.; 2],
            periodic: false,
        };
        let used = affine_work;
        return super::bspline_trace::verify_active(
            c,
            &side,
            s,
            reversed,
            max_work - used,
            Some((p, pr)),
        )
        .map(|decision| {
            decision.map(|mut proof| {
                proof.work_used += used;
                proof
            })
        });
    }
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let degrees = [s.degree_u, s.degree_v];
    let knots = [&s.knots_u, &s.knots_v];
    for fixed in 0..2 {
        let free = 1 - fixed;
        if a[fixed] != z[fixed] || a[free] == z[free] {
            continue;
        }
        let fixed_knots = knots[fixed];
        let degree = degrees[fixed];
        let count = counts[fixed];
        let mut at = a[fixed];
        let low = fixed_knots[degree];
        let high = fixed_knots[count];
        let periodic = [s.periodic_u, s.periodic_v];
        if periodic[fixed] && (at < low || at >= high) {
            if crate::periodic_trim::verify_surface_seam_identity(s, fixed)?.is_none() {
                continue;
            }
            let q = |v| Q::from_float(v).unwrap();
            let period = q(high) - q(low);
            let tile = ((q(at) - q(low)) / &period).floor();
            let Some(native) = exact_float(q(at) - tile * period) else {
                continue;
            };
            at = native;
        }
        if at < low || at > high {
            continue;
        }
        let index = if at == low
            && fixed_knots[..=degree].iter().all(|k| *k == low)
            && fixed_knots[degree + 1] > low
        {
            Some(0)
        } else if at == high
            && fixed_knots[count..].iter().all(|k| *k == high)
            && fixed_knots[count - 1] < high
        {
            Some(count - 1)
        } else if at < high
            && (at > low || periodic[fixed])
            && fixed_knots.iter().filter(|&&k| k == at).count() == degree
        {
            Some(fixed_knots.partition_point(|&k| k <= at) - 1 - degree)
        } else {
            None
        };
        let station = index.is_none()
            && degree == 1
            && count == 2
            && low < at
            && at < high
            && fixed_knots == &[low, low, high, high];
        if index.is_none() && !station {
            continue;
        }
        let free_knots = knots[free];
        let free_degree = degrees[free];
        let mut active = free_degree..counts[free];
        let first = active.find(|&i| free_knots[i] < free_knots[i + 1]).unwrap();
        let last = active
            .rfind(|&i| free_knots[i] < free_knots[i + 1])
            .unwrap_or(first);
        let active = first - free_degree..last + 1;
        let mut trace_knots = free_knots[active.start..active.end + free_degree + 1].to_vec();
        let mut controls = active.collect::<Vec<_>>();
        if periodic[free] {
            if crate::periodic_trim::verify_surface_seam_identity(s, free)?.is_none() {
                continue;
            }
            let range = [a[free].min(z[free]), a[free].max(z[free])];
            if range[0] < free_knots[free_degree] || range[1] > free_knots[counts[free]] {
                let Some((lifted, lifted_knots)) =
                    lifted_basis(free_knots, free_degree, counts[free], range)
                else {
                    continue;
                };
                controls = lifted;
                trace_knots = lifted_knots;
            }
        }
        let mut values = Vec::new();
        for (p, w) in c.control_points.iter().zip(&c.weights) {
            values.extend(p);
            values.push(*w);
        }
        let q = |v| Q::from_float(v).unwrap();
        let t = station.then(|| (q(at) - q(low)) / (q(high) - q(low)));
        let row = |i: usize| -> Option<[f64; 4]> {
            let control = |j| {
                let (u, v) = if fixed == 0 { (j, i) } else { (i, j) };
                let p = &s.control_points[u][v];
                [p[0], p[1], p[2], s.weights[u][v]]
            };
            if let Some(index) = index {
                return Some(control(index));
            }
            let [a, b] = [control(0), control(1)];
            if a[3] != b[3] {
                return None;
            }
            let t = t.as_ref()?;
            let mut point = a;
            for k in 0..3 {
                let exact = q(a[k]) + t * (q(b[k]) - q(a[k]));
                point[k] = exact
                    .to_f64()
                    .filter(|&x| Q::from_float(x) == Some(exact))?;
            }
            Some(point)
        };
        let Some(trace) = controls.into_iter().map(row).collect::<Option<Vec<_>>>() else {
            continue;
        };
        if trace.len() > 4096 {
            continue;
        }
        values.extend(trace.iter().flatten().copied());
        let ck = values.len();
        values.extend(&c.knots);
        let bk = values.len();
        values.extend_from_slice(&trace_knots);
        let line = values.len();
        values.extend([a[free], z[free]]);
        values.extend(pd);
        values.extend(pr);
        values.extend(cr);
        values.extend(s.control_points.iter().flatten().flatten().copied());
        values.extend(s.weights.iter().flatten().copied());
        values.extend(s.knots_u.iter().chain(&s.knots_v).copied());
        if affine_work > 0 {
            values.extend(p.control_points.iter().flatten().copied());
            values.extend(p.weights.iter().copied());
            values.extend(p.knots.iter().copied());
        }
        let arena = SourceArena::authored(
            "original-trimmed-tensor-boundary",
            1,
            values
                .iter()
                .map(|v: &f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .map_err(|_| crate::input("Invalid original tensor boundary source"))?;
        let rows = |start: usize, n: usize| {
            (0..n)
                .map(|i| std::array::from_fn(|k| arena.leaf(4 * (start + i) + k).unwrap()))
                .collect::<Vec<_>>()
        };
        let refs = |start: usize, n: usize| {
            (start..start + n)
                .map(|i| arena.leaf(i).unwrap())
                .collect::<Vec<_>>()
        };
        let aa = rows(0, c.control_points.len());
        let bb = rows(c.control_points.len(), trace.len());
        let ak = refs(ck, c.knots.len());
        let bkk = refs(bk, trace_knots.len());
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: max_work - affine_work,
                ..Limits::default()
            },
            None,
        );
        return cad_predicates::rational_bspline_trimmed_boundary_identity(
            &mut ctx,
            [
                cad_predicates::SplineLeafRefs {
                    controls: &aa,
                    knots: &ak,
                    degree: c.degree,
                },
                cad_predicates::SplineLeafRefs {
                    controls: &bb,
                    knots: &bkk,
                    degree: degrees[free],
                },
            ],
            [arena.leaf(line).unwrap(), arena.leaf(line + 1).unwrap()],
            std::array::from_fn(|i| arena.leaf(line + 2 + i).unwrap()),
            [arena.leaf(line + 6).unwrap(), arena.leaf(line + 7).unwrap()],
            reversed,
        )
        .map(|mut proof| {
            proof.work_used += affine_work;
            Some(proof)
        })
        .map_err(|_| crate::input("Invalid original tensor boundary clock proof"));
    }
    Ok(None)
}
