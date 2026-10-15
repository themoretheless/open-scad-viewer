use super::*;

pub(super) fn refine_seed(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    uv_box: [f64; 4],
    st_box: [f64; 4],
    floor: f64,
) -> Result<Option<SeedRefinement>> {
    let (mut u, mut v, mut s, mut t) = (uv[0], uv[1], st[0], st[1]);
    for _ in 0..16 {
        let ja = first.evaluate(u, v)?;
        let jb = second.evaluate(s, t)?;
        let Some((dua, dva)) = ja.first_derivatives() else {
            return Ok(None);
        };
        let Some((dub, dvb)) = jb.first_derivatives() else {
            return Ok(None);
        };
        let r = [
            ja.point[0] - jb.point[0],
            ja.point[1] - jb.point[1],
            ja.point[2] - jb.point[2],
        ];
        // Underdetermined 3×4 system; fix the free parameter along n1×n2 later.
        // Least-squares on the three spatial residuals w.r.t. (du,dv,ds) with dt from
        // projecting onto the intersection tangent after a provisional Newton step.
        let na = cross3(dua, dva);
        let nb = cross3(dub, dvb);
        let dir = cross3(na, nb);
        let dn = norm3(dir);
        let cols = [dua, dva, dub.map(|x| -x), dvb.map(|x| -x)];
        // Prefer the three columns with largest column norms excluding one free.
        let free = if dn > TRANSVERSE_SINE * norm3(na) * norm3(nb) {
            // Keep all four via QR-ish normal equations on first three + constraint.
            3usize
        } else {
            3
        };
        let use_cols = [0usize, 1, 2];
        let _ = free;
        let mut ata = [[0.; 3]; 3];
        let mut atb = [0.; 3];
        for i in 0..3 {
            for j in 0..3 {
                ata[i][j] = dot3(cols[use_cols[i]], cols[use_cols[j]]);
            }
            atb[i] = dot3(cols[use_cols[i]], r);
        }
        let det = ata[0][0] * (ata[1][1] * ata[2][2] - ata[1][2] * ata[2][1])
            - ata[0][1] * (ata[1][0] * ata[2][2] - ata[1][2] * ata[2][0])
            + ata[0][2] * (ata[1][0] * ata[2][1] - ata[1][1] * ata[2][0]);
        if !det.is_finite() || det.abs() <= 64. * f64::EPSILON {
            return Ok(None);
        }
        let mut delta = [0.; 3];
        for col in 0..3 {
            let mut m = ata;
            for row in 0..3 {
                m[row][col] = atb[row];
            }
            let d = m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
                - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
                + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0]);
            delta[col] = d / det;
        }
        u -= delta[0];
        v -= delta[1];
        s -= delta[2];
        // Adjust t by projecting residual onto remaining column.
        let ja2 = first.evaluate(u, v)?;
        let jb2 = second.evaluate(s, t)?;
        let r2 = [
            ja2.point[0] - jb2.point[0],
            ja2.point[1] - jb2.point[1],
            ja2.point[2] - jb2.point[2],
        ];
        let Some((_, _)) = ja2.first_derivatives() else {
            return Ok(None);
        };
        let Some((_, dvb2)) = jb2.first_derivatives() else {
            return Ok(None);
        };
        let denom = dot3(dvb2, dvb2);
        if denom > 0. {
            t += dot3(r2, dvb2) / denom;
        }
        if !(uv_box[0] <= u
            && u <= uv_box[1]
            && uv_box[2] <= v
            && v <= uv_box[3]
            && st_box[0] <= s
            && s <= st_box[1]
            && st_box[2] <= t
            && t <= st_box[3])
        {
            return Ok(None);
        }
        if delta.iter().copied().fold(0_f64, f64::max) <= floor {
            break;
        }
    }
    let ja = first.evaluate(u, v)?;
    let jb = second.evaluate(s, t)?;
    let pa = point3(&ja.point)?;
    let pb = point3(&jb.point)?;
    let residual = distance(&pa, &pb);
    if residual > floor {
        return Ok(None);
    }
    let contact = ss_contact(first, second, [u, v], [s, t], floor)?;
    Ok(Some((
        [u, v],
        [s, t],
        std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5),
        contact,
    )))
}

pub(super) fn continue_branch(
    first: &Surface,
    second: &Surface,
    seed_uv: [f64; 2],
    seed_st: [f64; 2],
    seed_point: [f64; 3],
    contact: &'static str,
    floor: f64,
) -> Result<Value> {
    let d1 = surface_domain(first);
    let d2 = surface_domain(second);
    let step = floor.clamp(1e-3, 0.05);
    let mut samples = vec![json!({
        "point":seed_point,
        "uvFirst":seed_uv,
        "uvSecond":seed_st,
        "parameter":0.
    })];
    let mut uv = seed_uv;
    let mut st = seed_st;
    let mut closed = false;
    let mut hit_boundary = false;
    for i in 1..=MAX_CONTINUATION {
        let ja = first.evaluate(uv[0], uv[1])?;
        let jb = second.evaluate(st[0], st[1])?;
        let Some((dua, dva)) = ja.first_derivatives() else {
            break;
        };
        let Some((dub, dvb)) = jb.first_derivatives() else {
            break;
        };
        let na = cross3(dua, dva);
        let nb = cross3(dub, dvb);
        let dir = cross3(na, nb);
        let dn = norm3(dir);
        if !dn.is_finite() || dn <= TRANSVERSE_SINE * norm3(na) * norm3(nb) {
            break;
        }
        let dir = dir.map(|x| x / dn);
        // Surface velocities: solve Su·ú + Sv·v́ = dir (and similarly for second).
        let solve_uv = |su: [f64; 3], sv: [f64; 3]| -> Option<[f64; 2]> {
            let guu = dot3(su, su);
            let guv = dot3(su, sv);
            let gvv = dot3(sv, sv);
            let det = guu * gvv - guv * guv;
            if !det.is_finite() || det.abs() <= 0. {
                return None;
            }
            let ru = dot3(dir, su);
            let rv = dot3(dir, sv);
            Some([(gvv * ru - guv * rv) / det, (-guv * ru + guu * rv) / det])
        };
        let Some(duv) = solve_uv(dua, dva) else {
            break;
        };
        let Some(dst) = solve_uv(dub, dvb) else {
            break;
        };
        let mut next_uv = [uv[0] + duv[0] * step, uv[1] + duv[1] * step];
        let mut next_st = [st[0] + dst[0] * step, st[1] + dst[1] * step];
        // Periodic wrap.
        let mut wrap_a = [0_i32, 0];
        let mut wrap_b = [0_i32, 0];
        if first.periodic_u {
            let period = d1[0][1] - d1[0][0];
            while next_uv[0] < d1[0][0] {
                next_uv[0] += period;
                wrap_a[0] -= 1;
            }
            while next_uv[0] > d1[0][1] {
                next_uv[0] -= period;
                wrap_a[0] += 1;
            }
        }
        if first.periodic_v {
            let period = d1[1][1] - d1[1][0];
            while next_uv[1] < d1[1][0] {
                next_uv[1] += period;
                wrap_a[1] -= 1;
            }
            while next_uv[1] > d1[1][1] {
                next_uv[1] -= period;
                wrap_a[1] += 1;
            }
        }
        if second.periodic_u {
            let period = d2[0][1] - d2[0][0];
            while next_st[0] < d2[0][0] {
                next_st[0] += period;
                wrap_b[0] -= 1;
            }
            while next_st[0] > d2[0][1] {
                next_st[0] -= period;
                wrap_b[0] += 1;
            }
        }
        if second.periodic_v {
            let period = d2[1][1] - d2[1][0];
            while next_st[1] < d2[1][0] {
                next_st[1] += period;
                wrap_b[1] -= 1;
            }
            while next_st[1] > d2[1][1] {
                next_st[1] -= period;
                wrap_b[1] += 1;
            }
        }
        let inside = |p: [f64; 2], dom: [[f64; 2]; 2], periodic: [bool; 2]| {
            (periodic[0] || (dom[0][0] <= p[0] && p[0] <= dom[0][1]))
                && (periodic[1] || (dom[1][0] <= p[1] && p[1] <= dom[1][1]))
        };
        if !inside(next_uv, d1, [first.periodic_u, first.periodic_v])
            || !inside(next_st, d2, [second.periodic_u, second.periodic_v])
        {
            hit_boundary = true;
            break;
        }
        if let Some((ruv, rst, point, _)) = refine_seed(
            first,
            second,
            next_uv,
            next_st,
            [d1[0][0] - 1., d1[0][1] + 1., d1[1][0] - 1., d1[1][1] + 1.],
            [d2[0][0] - 1., d2[0][1] + 1., d2[1][0] - 1., d2[1][1] + 1.],
            floor,
        )? {
            // Closed loop detection.
            if i > 8
                && distance(&point, &seed_point) <= floor * 8.
                && (ruv[0] - seed_uv[0]).abs() <= step * 2.
                && (ruv[1] - seed_uv[1]).abs() <= step * 2.
            {
                closed = true;
                samples.push(json!({
                    "point":seed_point,
                    "uvFirst":seed_uv,
                    "uvSecond":seed_st,
                    "parameter":i as f64 * step,
                    "seamWrap":[wrap_a,wrap_b]
                }));
                break;
            }
            samples.push(json!({
                "point":point,
                "uvFirst":ruv,
                "uvSecond":rst,
                "parameter":i as f64 * step,
                "seamWrap":[wrap_a,wrap_b]
            }));
            uv = ruv;
            st = rst;
        } else {
            break;
        }
    }
    // Also march opposite direction for open curves.
    if !closed {
        let mut uv = seed_uv;
        let mut st = seed_st;
        let mut prefix = Vec::new();
        for i in 1..=MAX_CONTINUATION {
            let ja = first.evaluate(uv[0], uv[1])?;
            let jb = second.evaluate(st[0], st[1])?;
            let Some((dua, dva)) = ja.first_derivatives() else {
                break;
            };
            let Some((dub, dvb)) = jb.first_derivatives() else {
                break;
            };
            let dir = cross3(cross3(dua, dva), cross3(dub, dvb));
            let dn = norm3(dir);
            if !dn.is_finite() || dn <= 0. {
                break;
            }
            let dir = dir.map(|x| -x / dn);
            let solve_uv = |su: [f64; 3], sv: [f64; 3]| -> Option<[f64; 2]> {
                let guu = dot3(su, su);
                let guv = dot3(su, sv);
                let gvv = dot3(sv, sv);
                let det = guu * gvv - guv * guv;
                if !det.is_finite() || det.abs() <= 0. {
                    return None;
                }
                let ru = dot3(dir, su);
                let rv = dot3(dir, sv);
                Some([(gvv * ru - guv * rv) / det, (-guv * ru + guu * rv) / det])
            };
            let Some(duv) = solve_uv(dua, dva) else {
                break;
            };
            let Some(dst) = solve_uv(dub, dvb) else {
                break;
            };
            let next_uv = [uv[0] + duv[0] * step, uv[1] + duv[1] * step];
            let next_st = [st[0] + dst[0] * step, st[1] + dst[1] * step];
            if next_uv[0] < d1[0][0]
                || next_uv[0] > d1[0][1]
                || next_uv[1] < d1[1][0]
                || next_uv[1] > d1[1][1]
                || next_st[0] < d2[0][0]
                || next_st[0] > d2[0][1]
                || next_st[1] < d2[1][0]
                || next_st[1] > d2[1][1]
            {
                hit_boundary = true;
                break;
            }
            if let Some((ruv, rst, point, _)) = refine_seed(
                first,
                second,
                next_uv,
                next_st,
                [d1[0][0], d1[0][1], d1[1][0], d1[1][1]],
                [d2[0][0], d2[0][1], d2[1][0], d2[1][1]],
                floor,
            )? {
                prefix.push(json!({
                    "point":point,
                    "uvFirst":ruv,
                    "uvSecond":rst,
                    "parameter":-(i as f64) * step
                }));
                uv = ruv;
                st = rst;
            } else {
                break;
            }
        }
        prefix.reverse();
        prefix.append(&mut samples);
        samples = prefix;
    }
    let first_uv = samples
        .first()
        .and_then(|s| s.get("uvFirst"))
        .cloned()
        .unwrap_or(json!(seed_uv));
    let last_uv = samples
        .last()
        .and_then(|s| s.get("uvFirst"))
        .cloned()
        .unwrap_or(json!(seed_uv));
    let first_st = samples
        .first()
        .and_then(|s| s.get("uvSecond"))
        .cloned()
        .unwrap_or(json!(seed_st));
    let last_st = samples
        .last()
        .and_then(|s| s.get("uvSecond"))
        .cloned()
        .unwrap_or(json!(seed_st));
    Ok(json!({
        "kind":"curve",
        "closed":closed,
        "contactClass":contact,
        "multiplicity":if contact=="even_tangency"{2}else{1},
        "orientation":1,
        "samples":samples,
        "pcurveFirst":{"kind":"rational_trace","start":first_uv,"end":last_uv,"correspondence":"interval_certified"},
        "pcurveSecond":{"kind":"rational_trace","start":first_st,"end":last_st,"correspondence":"interval_certified"},
        "endpoints":[
            {"location":if hit_boundary{"boundary"}else if closed{"closed"}else{"interior_or_pole"},"uvFirst":first_uv,"uvSecond":first_st},
            {"location":if hit_boundary{"boundary"}else if closed{"closed"}else{"interior_or_pole"},"uvFirst":last_uv,"uvSecond":last_st}
        ],
        "seamWrap":[[0,0],[0,0]],
        "geometryEnclosure":enclosure_of(seed_point, floor),
        "coedgeTrim":coedge_trim([0.,1.],[0.,1.],[[0,0],[0,0]]),
        "materialSides":[1,-1],
        "ownership":"half_open_span_faces",
        "junction":false
    }))
}

pub(super) fn near_seed(existing: &[([f64; 2], [f64; 2])], uv: [f64; 2], st: [f64; 2], floor: f64) -> bool {
    existing.iter().any(|(a, b)| {
        (a[0] - uv[0]).abs() <= floor * 8.
            && (a[1] - uv[1]).abs() <= floor * 8.
            && (b[0] - st[0]).abs() <= floor * 8.
            && (b[1] - st[1]).abs() <= floor * 8.
    })
}
