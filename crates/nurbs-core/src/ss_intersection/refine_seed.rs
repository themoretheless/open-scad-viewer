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
    // Unified guard backing the fixed 16-step Newton polish (item 1065).
    let mut guard = Budget::with_iterations(16)?.guard("ssi_seed_newton");
    for _ in 0..16 {
        guard.tick()?;
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
    let point = std::array::from_fn(|i| (pa[i] + pb[i]) * 0.5);
    // Result control point must be finite before it enters a branch (1093).
    require_finite_point(&point, "ssi refined point")?;
    Ok(Some(([u, v], [s, t], point, contact)))
}

pub(super) fn near_seed(existing: &[([f64; 2], [f64; 2])], uv: [f64; 2], st: [f64; 2], floor: f64) -> bool {
    existing.iter().any(|(a, b)| {
        (a[0] - uv[0]).abs() <= floor * 8.
            && (a[1] - uv[1]).abs() <= floor * 8.
            && (b[0] - st[0]).abs() <= floor * 8.
            && (b[1] - st[1]).abs() <= floor * 8.
    })
}
