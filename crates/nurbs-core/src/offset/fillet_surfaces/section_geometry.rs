//! Section geometry: preserves the original arithmetic and proof budgets.
use super::*;

/// G2 quintic Bézier section: position, tangent and curvature vector matched
/// to both support normal sections at the contacts.
pub(super) fn g2_section(m: &Marcher<'_>, sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let Some(section_normal) = unit(cross(a_vec, b_vec)) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let Some((_, na)) = point_normal(m.a, sample.uv_a[0], sample.uv_a[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let Some((_, nb)) = point_normal(m.b, sample.uv_b[0], sample.uv_b[1])? else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let chord = sub(sample.contact_b, sample.contact_a);
    let chord_len = norm(chord);
    if chord_len <= 1e-12 {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let tangent_at = |n: [f64; 3]| -> Option<[f64; 3]> {
        let mut t = unit(cross(section_normal, n))?;
        if dot(t, chord) < 0. {
            t = scale(t, -1.);
        }
        Some(t)
    };
    let (Some(t0), Some(t1)) = (tangent_at(na), tangent_at(nb)) else {
        return Ok(Err(FailureReason::AssemblyFailed));
    };
    let k0 = normal_curvature(m.a, sample.uv_a, t0)?.unwrap_or(0.);
    let k1 = normal_curvature(m.b, sample.uv_b, t1)?.unwrap_or(0.);
    let handle = 0.35 * chord_len;
    // Bézier end condition: curvature vector K = (4/5)·(P2−P1)⊥/a² — solve
    // for the inner controls with K matched to k_n·n of each support.
    let k0_vec = scale(na, k0);
    let k1_vec = scale(nb, k1);
    let p0 = sample.contact_a;
    let p5 = sample.contact_b;
    let p1 = add(p0, scale(t0, handle));
    let p4 = sub(p5, scale(t1, handle));
    let p2 = add(add(p1, scale(t0, handle)), scale(k0_vec, 1.25 * handle * handle));
    let p3 = add(sub(p4, scale(t1, handle)), scale(k1_vec, 1.25 * handle * handle));
    let curve = Curve {
        degree: 5,
        knots: vec![0.; 6].into_iter().chain(vec![1.; 6]).collect(),
        control_points: vec![p0, p1, p2, p3, p4, p5]
            .iter()
            .map(|p| p.to_vec())
            .collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    match curve.validate() {
        Ok(()) => Ok(Ok(curve)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
}

/// Exact rational circular arc section from contact A to contact B.
pub(super) fn arc_section(sample: &SpineSample) -> Result<std::result::Result<Curve, FailureReason>> {
    let a_vec = sub(sample.contact_a, sample.center);
    let b_vec = sub(sample.contact_b, sample.center);
    let phi = angle_between(a_vec, b_vec);
    if !(1e-6..std::f64::consts::PI - 1e-9).contains(&phi) || !phi.is_finite() {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    }
    let Some(a_hat) = unit(a_vec) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    let perp = sub(b_vec, scale(a_hat, dot(b_vec, a_hat)));
    let Some(v_hat) = unit(perp) else {
        return Ok(Err(FailureReason::SectionSelfIntersection));
    };
    // Use one coherent two-arc parameterization across all stations. Choosing
    // one/two arcs at 90 degrees makes tiny angular roundoff change the rational
    // parameter map; lofting those mismatched maps deforms even a cylinder.
    let weight = (phi * 0.25).cos();
    let mut points = Vec::with_capacity(5);
    for i in 0..5 {
        let angle = phi * i as f64 * 0.25;
        let radius = if i % 2 == 0 { sample.radius } else { sample.radius / weight };
        let radial = add(scale(a_hat, angle.cos()), scale(v_hat, angle.sin()));
        points.push(add(sample.center, scale(radial, radius)).to_vec());
    }
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
        control_points: points,
        weights: vec![1., weight, 1., weight, 1.],
        periodic: false,
    };
    match curve.validate() {
        Ok(()) => Ok(Ok(curve)),
        Err(_) => Ok(Err(FailureReason::AssemblyFailed)),
    }
}

/// March one direction from the seed with a curvature-adaptive step.
pub(super) fn march_direction(
    m: &Marcher<'_>,
    seed: [f64; 4],
    sign: f64,
    radius_at: &dyn Fn(f64) -> f64,
    max_steps: usize,
    closed_at: [f64; 3],
) -> Result<(Vec<RawPoint>, bool)> {
    let mut points = Vec::with_capacity(max_steps.min(64));
    let mut x = seed;
    let r0 = radius_at(0.);
    let Some(mut tangent) = m.tangent(&x, r0)? else {
        return Ok((points, false));
    };
    let h_min = 1e-4 * r0;
    let h_max = r0;
    let mut h = 0.5 * r0;
    let mut s = 0.;
    let mut closed = false;
    let mut budget = Budget::with_iterations(max_steps + 1)?.guard("fillet_marching");
    for _ in 0..max_steps {
        budget.tick()?;
        let r_here = radius_at(s);
        let h_min = h_min.min(1e-4 * r_here);
        let h_max = h_max.max(2. * r_here);
        let mut accepted = None;
        let mut step = h;
        for _ in 0..6 {
            let mut candidate = x;
            for i in 0..4 {
                candidate[i] += sign * step * tangent[i];
            }
            if let Some((refined, center, pa, pb)) = m.refine_with_contacts(candidate, radius_at(s + sign * step))? {
                if m.inside(&refined) {
                    accepted = Some((refined, center, pa, pb));
                    break;
                }
            }
            step *= 0.5;
            if step < h_min * 1e-3 {
                break;
            }
        }
        let Some((next, center, pa, pb)) = accepted else { break };
        let s_next = s + sign * step;
        let r_next = radius_at(s_next);
        // Curvature-adaptive next step: chord error κ h²/8 ≤ chord tolerance.
        let mut h_next = (h * 1.5).min(h_max);
        if let Some(t_next) = m.tangent(&next, r_next)? {
            let d = dot4(t_next, tangent);
            let t_next = if d >= 0. { t_next } else { t_next.map(|v| -v) };
            let kappa = d.abs().clamp(-1., 1.).acos() / step.max(1e-300);
            if kappa > 1e-12 {
                let chord_tol = (0.02 * r_next).max(10. * m.tol);
                h_next = h_next.min((8. * chord_tol / kappa).sqrt()).clamp(h_min, h_max);
            }
            tangent = t_next;
        }
        points.push(RawPoint {
            x: next,
            center,
            contact_a: pa,
            contact_b: pb,
            s: s_next,
        });
        x = next;
        s = s_next;
        h = h_next;
        // Loop closure back at the seed after enough travel.
        if points.len() > 8 && norm(sub(center, closed_at)) < 0.25 * r_next {
            closed = true;
            break;
        }
    }
    Ok((points, closed))
}

/// Full two-direction spine trace with a radius law evaluated on the
/// normalized arc-length parameter of a previous pass (`law_length`), or a
/// constant radius when `law` is None and `seed_r` is used.
pub(super) fn trace_spine(
    m: &Marcher<'_>,
    seed_r: f64,
    max_steps: usize,
) -> Result<(Vec<RawPoint>, Option<[f64; 4]>, bool)> {
    let Some(seed) = find_seed(m, seed_r)? else {
        return Ok((Vec::new(), None, false));
    };
    let (Some((pa, na)), Some((pb, nb))) = (
        point_normal(m.a, seed[0], seed[1])?,
        point_normal(m.b, seed[2], seed[3])?,
    ) else {
        return Ok((Vec::new(), None, false));
    };
    let seed_center = scale(add(add(pa, scale(na, seed_r * m.sa)), add(pb, scale(nb, seed_r * m.sb))), 0.5);
    let radius_at = |_: f64| seed_r;
    let (mut backward, closed_b) = march_direction(m, seed, -1., &radius_at, max_steps / 2, seed_center)?;
    let (forward, closed_f) = march_direction(m, seed, 1., &radius_at, max_steps / 2, seed_center)?;
    backward.reverse();
    let mut points = backward;
    points.reserve(1 + forward.len());
    points.push(RawPoint { x: seed, center: seed_center, contact_a: pa, contact_b: pb, s: 0. });
    points.extend(forward);
    // Re-base arc lengths to start at zero.
    let s0 = points[0].s;
    for p in &mut points {
        p.s -= s0;
    }
    Ok((points, Some(seed), closed_b || closed_f))
}
