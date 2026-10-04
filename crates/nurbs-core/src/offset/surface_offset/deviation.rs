//! Certified sampled deviation of an offset surface against the true offset
//! (split from `surface_offset`, byte for byte).
use super::*;

/// Dense sampled deviation of `offset` against the true offset of `original`.
/// Also records small UV rectangles around samples whose normal degenerated.
pub(super) fn sampled_deviation(
    original: &Surface,
    offset: &Surface,
    distance: f64,
    degenerate: &mut Vec<[f64; 4]>,
) -> Result<f64> {
    let ([u0, u1], [v0, v1]) = domain(original);
    let nu = (2 * original.control_points.len()).clamp(4, 64);
    let nv = (2 * original.control_points[0].len()).clamp(4, 64);
    let mut worst = 0_f64;
    for i in 0..=nu {
        for j in 0..=nv {
            let u = u0 + (u1 - u0) * i as f64 / nu as f64;
            let v = v0 + (v1 - v0) * j as f64 / nv as f64;
            let e = original.evaluate_validated(u, v)?;
            // Skip (and record) samples whose normal is None or numerically
            // unreliable: |S_u x S_v| / (|S_u||S_v|) below threshold means the
            // normal direction is dominated by roundoff (e.g. at poles).
            let reliable = e.unit_normal().is_some()
                && e.first_derivatives().is_some_and(|(du, dv)| {
                    let su = norm(du);
                    let sv = norm(dv);
                    su > 0.
                        && sv > 0.
                        && su.min(sv) > 1e-9 * su.max(sv)
                        && norm([
                            du[1] * dv[2] - du[2] * dv[1],
                            du[2] * dv[0] - du[0] * dv[2],
                            du[0] * dv[1] - du[1] * dv[0],
                        ]) > 1e-6 * su * sv
                });
            let n = e.unit_normal();
            if !reliable {
                let du = (u1 - u0) / nu as f64;
                let dv = (v1 - v0) / nv as f64;
                degenerate.push([
                    (u - du / 2.).max(u0),
                    (u + du / 2.).min(u1),
                    (v - dv / 2.).max(v0),
                    (v + dv / 2.).min(v1),
                ]);
                continue;
            }
            let Some(n) = n else { continue };
            let q = offset.evaluate_validated(u, v)?.point;
            let delta: [f64; 3] = std::array::from_fn(|k| q[k] - e.point[k]);
            let along = dot(delta, n);
            let lateral: [f64; 3] = std::array::from_fn(|k| delta[k] - along * n[k]);
            let deviation = (along - distance).abs().hypot(norm(lateral));
            numeric(deviation.is_finite(), "Surface offset deviation overflowed")?;
            worst = worst.max(deviation);
        }
    }
    Ok(next_up(worst))
}
