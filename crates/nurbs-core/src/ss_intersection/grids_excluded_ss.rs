use super::*;
pub(super) fn grids_excluded_ss(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>]) -> bool {
    let flat_a: Vec<[f64; 4]> = a.iter().flatten().copied().collect();
    let flat_b: Vec<[f64; 4]> = b.iter().flatten().copied().collect();
    let ra = {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &flat_a {
            for axis in 0..3 {
                let x = p[axis] / p[3];
                lo[axis] = lo[axis].min(next_down(x));
                hi[axis] = hi[axis].max(next_up(x));
            }
        }
        [lo, hi]
    };
    let rb = {
        let mut lo = [f64::INFINITY; 3];
        let mut hi = [f64::NEG_INFINITY; 3];
        for p in &flat_b {
            for axis in 0..3 {
                let x = p[axis] / p[3];
                lo[axis] = lo[axis].min(next_down(x));
                hi[axis] = hi[axis].max(next_up(x));
            }
        }
        [lo, hi]
    };
    (0..3).any(|axis| ra[1][axis] < rb[0][axis] || rb[1][axis] < ra[0][axis])
}

pub(super) fn proportional_grids(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>], tol: f64) -> bool {
    if a.len() != b.len() || a.is_empty() || a[0].len() != b[0].len() {
        return false;
    }
    let mut scale = None;
    for (ra, rb) in a.iter().zip(b) {
        for (pa, pb) in ra.iter().zip(rb) {
            for axis in 0..4 {
                if pa[axis].abs() <= tol && pb[axis].abs() <= tol {
                    continue;
                }
                if pa[axis].abs() <= tol || pb[axis].abs() <= tol {
                    return false;
                }
                let ratio = pa[axis] / pb[axis];
                match scale {
                    None => scale = Some(ratio),
                    Some(s) if (ratio - s).abs() > tol.max(s.abs() * 1e-9) => return false,
                    _ => {}
                }
            }
        }
    }
    scale.is_some()
}

pub(super) type PlaneCarrier = ([f64; 3], f64, [f64; 3], [f64; 3]);
pub(super) type SeedRefinement = ([f64; 2], [f64; 2], [f64; 3], ContactClass);
pub(super) type SurfaceCellPending = (
    [f64; 4],
    [f64; 4],
    Option<Vec<Vec<[f64; 4]>>>,
    Option<Vec<Vec<[f64; 4]>>>,
    usize,
);

pub(super) fn affine_plane(surface: &Surface) -> Result<Option<PlaneCarrier>> {
    // Any positive-weight coplanar chart is an affine plane carrier, not only deg 1×1.
    let o = point3(&surface.control_points[0][0])?;
    let mut u_dir = None;
    let mut v_dir = None;
    for row in &surface.control_points {
        for corner in row {
            let p = point3(corner)?;
            let d = [p[0] - o[0], p[1] - o[1], p[2] - o[2]];
            if norm3(d) <= 1e-15 {
                continue;
            }
            if u_dir.is_none() {
                u_dir = Some(d);
                continue;
            }
            let u = u_dir.unwrap();
            let c = cross3(u, d);
            if norm3(c) > 1e-12 {
                v_dir = Some(d);
                break;
            }
        }
        if v_dir.is_some() {
            break;
        }
    }
    let (Some(u_dir), Some(v_dir)) = (u_dir, v_dir) else {
        return Ok(None);
    };
    let n = cross3(u_dir, v_dir);
    let nn = norm3(n);
    if !nn.is_finite() || nn <= 0. {
        return Ok(None);
    }
    let normal = n.map(|x| x / nn);
    let offset = dot3(normal, o);
    for row in &surface.control_points {
        for corner in row {
            let p = point3(corner)?;
            if (dot3(normal, p) - offset).abs() > 1e-9 {
                return Ok(None);
            }
        }
    }
    Ok(Some((normal, offset, u_dir, v_dir)))
}

pub(super) fn invert_uv(
    origin: [f64; 3],
    u_dir: [f64; 3],
    v_dir: [f64; 3],
    domain: [[f64; 2]; 2],
    point: [f64; 3],
) -> Result<[f64; 2]> {
    let d = [
        point[0] - origin[0],
        point[1] - origin[1],
        point[2] - origin[2],
    ];
    let guu = dot3(u_dir, u_dir);
    let guv = dot3(u_dir, v_dir);
    let gvv = dot3(v_dir, v_dir);
    let det = guu * gvv - guv * guv;
    check(det.abs() > 0., "Degenerate planar frame")?;
    let ru = dot3(d, u_dir);
    let rv = dot3(d, v_dir);
    let su = (gvv * ru - guv * rv) / det;
    let sv = (-guv * ru + guu * rv) / det;
    Ok([
        domain[0][0] + su * (domain[0][1] - domain[0][0]),
        domain[1][0] + sv * (domain[1][1] - domain[1][0]),
    ])
}

pub(super) fn surface_domain(surface: &Surface) -> [[f64; 2]; 2] {
    [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.knots_u.len() - surface.degree_u - 1],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.knots_v.len() - surface.degree_v - 1],
        ],
    ]
}

pub(super) fn ss_contact(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    floor: f64,
) -> Result<ContactClass> {
    let ja = first.evaluate(uv[0], uv[1])?;
    let jb = second.evaluate(st[0], st[1])?;
    let Some((dua, dva)) = ja.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let Some((dub, dvb)) = jb.first_derivatives() else {
        return Ok(ContactClass::PoleOrSingular);
    };
    let na = cross3(dua, dva);
    let nb = cross3(dub, dvb);
    let la = norm3(na);
    let lb = norm3(nb);
    if !la.is_finite() || !lb.is_finite() || la <= floor || lb <= floor {
        return Ok(ContactClass::PoleOrSingular);
    }
    let na = na.map(|x| x / la);
    let nb = nb.map(|x| x / lb);
    let sine = norm3(cross3(na, nb));
    if sine > TRANSVERSE_SINE {
        Ok(ContactClass::Transverse)
    } else if sine <= floor {
        Ok(ContactClass::EvenTangency)
    } else {
        Ok(ContactClass::OddTangency)
    }
}
