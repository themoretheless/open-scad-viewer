use super::*;

pub(super) fn krawczyk_cc(
    first: &Curve,
    second: &Curve,
    ta: [f64; 2],
    tb: [f64; 2],
    floor: f64,
) -> Result<Option<(f64, f64)>> {
    let width_t = ta[1] - ta[0];
    let width_u = tb[1] - tb[0];
    if width_t.max(width_u) > floor.max(2_f64.powi(-24)) {
        return Ok(None);
    }
    let (mut t, mut u) = ((ta[0] + ta[1]) * 0.5, (tb[0] + tb[1]) * 0.5);
    // Unified guard backing the fixed 12-step Newton polish (item 1065).
    let mut guard = Budget::with_iterations(12)?.guard("cc_krawczyk_newton");
    for _ in 0..12 {
        guard.tick()?;
        let ja = first.evaluate(t)?;
        let jb = second.evaluate(u)?;
        let Some(ref a1) = ja.d1 else {
            return Ok(None);
        };
        let Some(ref b1) = jb.d1 else {
            return Ok(None);
        };
        let va = point3(a1)?;
        let vb = point3(b1)?;
        let r = [
            ja.point[0] - jb.point[0],
            ja.point[1] - jb.point[1],
            ja.point[2] - jb.point[2],
        ];
        // Project residual onto the two dominant axes of va×vb plane.
        let n = cross3(va, vb);
        let nn = norm3(n);
        if nn <= TRANSVERSE_SINE * norm3(va) * norm3(vb) {
            return Ok(None);
        }
        let e1 = va;
        let e2 = cross3(n, va);
        let ne2 = norm3(e2);
        if !ne2.is_finite() || ne2 <= 0. {
            return Ok(None);
        }
        let e2 = e2.map(|x| x / ne2);
        let e1n = norm3(e1).max(f64::from_bits(1));
        let e1 = e1.map(|x| x / e1n);
        let ft = [dot3(r, e1), dot3(r, e2)];
        let j00 = dot3(va, e1);
        let j01 = -dot3(vb, e1);
        let j10 = dot3(va, e2);
        let j11 = -dot3(vb, e2);
        let det = j00 * j11 - j01 * j10;
        if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
            return Ok(None);
        }
        let dt = (j11 * ft[0] - j01 * ft[1]) / det;
        let du = (-j10 * ft[0] + j00 * ft[1]) / det;
        t -= dt;
        u -= du;
        if !(ta[0] <= t && t <= ta[1] && tb[0] <= u && u <= tb[1]) {
            return Ok(None);
        }
        if dt.abs().max(du.abs()) <= floor {
            break;
        }
    }
    // Krawczyk contraction: image of the box under Newton stays inside.
    let radius_t = width_t * 0.45;
    let radius_u = width_u * 0.45;
    if (t - (ta[0] + ta[1]) * 0.5).abs() <= radius_t
        && (u - (tb[0] + tb[1]) * 0.5).abs() <= radius_u
    {
        // Isolated parameters must be finite before they enter a report (1093).
        require_finite_point(&[t, u], "krawczyk parameters")?;
        Ok(Some((t, u)))
    } else {
        Ok(None)
    }
}

