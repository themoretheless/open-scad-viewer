use super::*;
pub(super) struct Field {
    pub hull: [I; 12],
    pub witnesses: Vec<[I; 12]>,
}
pub(super) fn clip(
    pc: &Curve,
    s: &Surface,
    mut uv: [[f64; 2]; 2],
) -> Result<Option<[[f64; 2]; 2]>> {
    let d = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    for k in 0..2 {
        let lo = pc
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::INFINITY, f64::min);
        let hi = pc
            .control_points
            .iter()
            .map(|p| p[k])
            .fold(f64::NEG_INFINITY, f64::max);
        uv[k] = [uv[k][0].max(lo), uv[k][1].min(hi)];
        if lo >= d[k][0] && hi <= d[k][1] {
            uv[k] = [uv[k][0].max(d[k][0]), uv[k][1].min(d[k][1])];
        }
        if uv[k][0] < d[k][0] || uv[k][1] > d[k][1] || uv[k][0] > uv[k][1] {
            return Ok(None);
        }
    }
    Ok(Some(uv))
}
pub(super) fn get(
    pc: &Curve,
    s: &Surface,
    t: [f64; 2],
    reverse: bool,
    work: &mut usize,
    max: usize,
) -> Result<Option<Field>> {
    let Some(uv) = super::super::uv_box(pc, t, reverse, work, max)? else {
        return Ok(None);
    };
    let Some(uv) = clip(pc, s, uv)? else {
        return Ok(None);
    };
    let mut result: Option<Field> = None;
    for u in s.degree_u..s.control_points.len() {
        for v in s.degree_v..s.control_points[0].len() {
            let full = [
                [s.knots_u[u], s.knots_u[u + 1]],
                [s.knots_v[v], s.knots_v[v + 1]],
            ];
            let q = std::array::from_fn(|k| [uv[k][0].max(full[k][0]), uv[k][1].min(full[k][1])]);
            if (0..2).any(|k| full[k][0] == full[k][1] || q[k][0] > q[k][1]) {
                continue;
            }
            if *work == max {
                return Ok(None);
            }
            *work += 1;
            let r = crate::surface_shape_operator::span_rectangle_bounds(s, [u, v], q)?.shape;
            let (Some(n), Some(b)) = (r.normal, r.tensor) else {
                return Ok(None);
            };
            let mut value = [I::point(0.); 12];
            for k in 0..3 {
                value[k] = I::new(n[k][0], n[k][1])?;
                for j in 0..3 {
                    value[3 + 3 * k + j] = I::new(b[k][j][0], b[k][j][1])?;
                }
            }
            let witness =
                t[0] == t[1] && (0..2).all(|k| full[k][0] <= uv[k][0] && uv[k][1] <= full[k][1]);
            if let Some(out) = &mut result {
                for k in 0..12 {
                    out.hull[k].lo = out.hull[k].lo.min(value[k].lo);
                    out.hull[k].hi = out.hull[k].hi.max(value[k].hi);
                }
                if witness {
                    out.witnesses.push(value);
                }
            } else {
                result = Some(Field {
                    hull: value,
                    witnesses: if witness { vec![value] } else { vec![] },
                });
            }
        }
    }
    Ok(result)
}
pub(super) fn distances(a: &[I; 12], b: &[I; 12], sign: usize) -> Result<[[f64; 2]; 2]> {
    let mut d = [I::point(0.); 12];
    for k in 0..12 {
        let b = if sign == 0 {
            b[k]
        } else {
            I::new(-b[k].hi, -b[k].lo)?
        };
        d[k] = a[k].sub(b)?;
    }
    let n = box_distance(&d[..3], &[I::point(0.); 3])?;
    let b = box_distance(&d[3..], &[I::point(0.); 9])?;
    Ok([[n.0, n.1.min(2.)], [b.0, b.1]])
}
pub(super) fn pair(
    p: [&Curve; 2],
    s: [&Surface; 2],
    reverse: [bool; 2],
    t: [f64; 2],
    work: &mut usize,
    max: usize,
) -> Result<Option<([[f64; 2]; 2], [[f64; 2]; 2])>> {
    let Some(a) = get(p[0], s[0], t, reverse[0], work, max)? else {
        return Ok(None);
    };
    let Some(b) = get(p[1], s[1], t, reverse[1], work, max)? else {
        return Ok(None);
    };
    let mut upper = [[0.; 2]; 2];
    let mut lower = [[0_f64; 2]; 2];
    for sign in 0..2 {
        upper[sign] = distances(&a.hull, &b.hull, sign)?.map(|x| x[1]);
        for x in &a.witnesses {
            for y in &b.witnesses {
                let d = distances(x, y, sign)?;
                for k in 0..2 {
                    lower[sign][k] = lower[sign][k].max(d[k][0]);
                }
            }
        }
    }
    Ok(Some((upper, lower)))
}
