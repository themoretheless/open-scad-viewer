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

pub(super) fn admit_coincidence(
    input: &CurveIntersectionBox<'_>,
    report: &mut CurveCurveReport,
) -> Result<bool> {
    let CurveIntersectionBox {
        first,
        second,
        ta,
        tb,
        ha,
        hb,
        floor,
    } = *input;
    if proportional_homogeneous(ha, hb, floor.max(1e-12)) {
        let pa = point3(&first.evaluate(ta[0])?.point)?;
        let pb = point3(&first.evaluate(ta[1])?.point)?;
        let qa = point3(&second.evaluate(tb[0])?.point)?;
        let qb = point3(&second.evaluate(tb[1])?.point)?;
        let forward = distance(&pa, &qa) + distance(&pb, &qb)
            <= distance(&pa, &qb) + distance(&pb, &qa) + floor;
        let reversed = !forward;
        let (sb0, sb1) = if reversed {
            (tb[1], tb[0])
        } else {
            (tb[0], tb[1])
        };
        report.components.push(CcComponent {
            kind: CurveCurveComponentKind::Overlap,
            first: ta[0],
            second: sb0,
            first_interval: ta,
            second_interval: if reversed { [tb[0], tb[1]] } else { tb },
            point: pa,
            residual: 0.,
            contact: ContactClass::Coincident,
            multiplicity: u32::MAX,
            orientation: if reversed { -1 } else { 1 },
            reversed,
            first_wrap: 0,
            second_wrap: 0,
            enclosure: enclosure_of(pa, floor),
            coedge_trim: Some(coedge_trim(ta, [sb0, sb1], [[0, 0], [0, 0]])),
        });
        return Ok(true);
    }
    let (Some(da), Some(db)) = (collinear_direction(ha), collinear_direction(hb)) else {
        return Ok(false);
    };
    if norm3(cross3(da, db)) > 1e-8 {
        return Ok(false);
    }
    let origin = [
        ha[0][0] / ha[0][3],
        ha[0][1] / ha[0][3],
        ha[0][2] / ha[0][3],
    ];
    let dir = da;
    let mut a_params: Vec<(f64, f64)> = ta
        .into_iter()
        .map(|t| {
            let p = point3(&first.evaluate(t).unwrap().point).unwrap();
            (t, project_line_parameter(p, origin, dir))
        })
        .collect();
    let mut b_params: Vec<(f64, f64)> = tb
        .into_iter()
        .map(|u| {
            let p = point3(&second.evaluate(u).unwrap().point).unwrap();
            (u, project_line_parameter(p, origin, dir))
        })
        .collect();
    a_params.sort_by(|a, b| a.1.total_cmp(&b.1));
    b_params.sort_by(|a, b| a.1.total_cmp(&b.1));
    let lo = a_params[0].1.max(b_params[0].1);
    let hi = a_params[1].1.min(b_params[1].1);
    if !hi.is_finite() || hi <= lo + floor {
        return Ok(false);
    }
    // Invert endpoints by linear blend in source parameter (degree-1 exact; higher monotone collinear).
    let invert = |params: &[(f64, f64)], s: f64| {
        let (t0, s0) = params[0];
        let (t1, s1) = params[1];
        if (s1 - s0).abs() <= floor {
            t0
        } else {
            t0 + (t1 - t0) * ((s - s0) / (s1 - s0))
        }
    };
    let t0 = invert(&a_params, lo);
    let t1 = invert(&a_params, hi);
    let u0 = invert(&b_params, lo);
    let u1 = invert(&b_params, hi);
    let reversed = (u1 - u0).signum() != (t1 - t0).signum() && (u1 - u0).abs() > floor;
    let point = point3(&first.evaluate(t0)?.point)?;
    report.components.push(CcComponent {
        kind: CurveCurveComponentKind::Overlap,
        first: t0,
        second: u0,
        first_interval: [t0.min(t1), t0.max(t1)],
        second_interval: [u0.min(u1), u0.max(u1)],
        point,
        residual: 0.,
        contact: ContactClass::Coincident,
        multiplicity: u32::MAX,
        orientation: if reversed { -1 } else { 1 },
        reversed,
        first_wrap: 0,
        second_wrap: 0,
        enclosure: enclosure_of(point, floor),
        coedge_trim: Some(coedge_trim(
            [t0.min(t1), t0.max(t1)],
            [u0.min(u1), u0.max(u1)],
            [[0, 0], [0, 0]],
        )),
    });
    Ok(true)
}
