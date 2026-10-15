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

type PlaneCarrier = ([f64; 3], f64, [f64; 3], [f64; 3]);
pub(super) type SeedRefinement = ([f64; 2], [f64; 2], [f64; 3], &'static str);
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

/// Exact iso branches of a general chart against an affine plane when signed
/// distance separates as a monotone function of one parameter on the control net.
pub(super) fn surface_plane_iso_components(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    plane: &Surface,
    floor: f64,
) -> Result<Option<Vec<Value>>> {
    let domain = surface_domain(surface);
    let nu = surface.control_points.len();
    let nv = surface.control_points[0].len();
    let signed_ctrl = |i: usize, j: usize| -> Result<f64> {
        let p = point3(&surface.control_points[i][j])?;
        Ok(dot3(normal, p) - offset)
    };
    let signed_at = |u: f64, v: f64| -> Result<f64> {
        let p = point3(&surface.evaluate(u, v)?.point)?;
        Ok(dot3(normal, p) - offset)
    };
    let greville = |knots: &[f64], degree: usize, i: usize| -> f64 {
        if degree == 0 {
            return knots[i];
        }
        knots[i + 1..i + 1 + degree].iter().sum::<f64>() / degree as f64
    };
    let origin = point3(&plane.control_points[0][0])?;
    let Some((_, _, u_dir, v_dir)) = affine_plane(plane)? else {
        return Ok(None);
    };
    let plane_domain = surface_domain(plane);
    let mut components = Vec::new();

    // Iso-U: control rows nearly constant in signed distance, adjacent rows change sign.
    let mut row_vals = Vec::with_capacity(nu);
    let mut row_ok = true;
    for i in 0..nu {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for j in 0..nv {
            let s = signed_ctrl(i, j)?;
            lo = lo.min(s);
            hi = hi.max(s);
        }
        if hi - lo > floor * 8. {
            row_ok = false;
            break;
        }
        row_vals.push(0.5 * (lo + hi));
    }
    if row_ok {
        for i in 0..nu.saturating_sub(1) {
            let a = row_vals[i];
            let b = row_vals[i + 1];
            if a * b > 0. && a.abs() > floor && b.abs() > floor {
                continue;
            }
            let mut lo =
                greville(&surface.knots_u, surface.degree_u, i).clamp(domain[0][0], domain[0][1]);
            let mut hi = greville(&surface.knots_u, surface.degree_u, i + 1)
                .clamp(domain[0][0], domain[0][1]);
            if hi < lo {
                std::mem::swap(&mut lo, &mut hi);
            }
            let v_mid = 0.5 * (domain[1][0] + domain[1][1]);
            let mut u = 0.5 * (lo + hi);
            for _ in 0..64 {
                let s = signed_at(u, v_mid)?;
                if s.abs() <= floor {
                    break;
                }
                let slo = signed_at(lo, v_mid)?;
                if slo * s <= 0. {
                    hi = u;
                } else {
                    lo = u;
                }
                u = 0.5 * (lo + hi);
            }
            if signed_at(u, v_mid)?.abs() > floor * 32. {
                continue;
            }
            if !(u > domain[0][0] + floor && u < domain[0][1] - floor) {
                continue;
            }
            let start_p = point3(&surface.evaluate(u, domain[1][0])?.point)?;
            let end_p = point3(&surface.evaluate(u, domain[1][1])?.point)?;
            if (dot3(normal, start_p) - offset).abs() > floor * 32.
                || (dot3(normal, end_p) - offset).abs() > floor * 32.
            {
                continue;
            }
            let uv_b0 = invert_uv(origin, u_dir, v_dir, plane_domain, start_p)?;
            let uv_b1 = invert_uv(origin, u_dir, v_dir, plane_domain, end_p)?;
            components.push(json!({
                "kind":"curve",
                "closed":false,
                "contactClass":"transverse",
                "multiplicity":1,
                "orientation":1,
                "samples":[
                    {"point":start_p,"uvFirst":[u,domain[1][0]],"uvSecond":uv_b0,"parameter":0.},
                    {"point":end_p,"uvFirst":[u,domain[1][1]],"uvSecond":uv_b1,"parameter":1.}
                ],
                "pcurveFirst":{"kind":"line","start":[u,domain[1][0]],"end":[u,domain[1][1]],"correspondence":"exact_affine"},
                "pcurveSecond":{"kind":"line","start":uv_b0,"end":uv_b1,"correspondence":"exact_affine"},
                "endpoints":[
                    {"location":"boundary","uvFirst":[u,domain[1][0]],"uvSecond":uv_b0},
                    {"location":"boundary","uvFirst":[u,domain[1][1]],"uvSecond":uv_b1}
                ],
                "seamWrap":[[0,0],[0,0]],
                "geometryEnclosure":enclosure_of(start_p, floor),
                "coedgeTrim":coedge_trim([0.,1.],[0.,1.],[[0,0],[0,0]]),
                "materialSides":[1,-1],
                "ownership":"half_open_span_faces",
                "junction":false
            }));
        }
    }

    // Iso-V candidates with Greville + bisection.
    let mut col_vals = Vec::with_capacity(nv);
    let mut col_ok = true;
    for j in 0..nv {
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for i in 0..nu {
            let s = signed_ctrl(i, j)?;
            lo = lo.min(s);
            hi = hi.max(s);
        }
        if hi - lo > floor * 8. {
            col_ok = false;
            break;
        }
        col_vals.push(0.5 * (lo + hi));
    }
    if col_ok {
        for j in 0..nv.saturating_sub(1) {
            let a = col_vals[j];
            let b = col_vals[j + 1];
            if a * b > 0. && a.abs() > floor && b.abs() > floor {
                continue;
            }
            let mut lo =
                greville(&surface.knots_v, surface.degree_v, j).clamp(domain[1][0], domain[1][1]);
            let mut hi = greville(&surface.knots_v, surface.degree_v, j + 1)
                .clamp(domain[1][0], domain[1][1]);
            if hi < lo {
                std::mem::swap(&mut lo, &mut hi);
            }
            let u_mid = 0.5 * (domain[0][0] + domain[0][1]);
            let mut v = 0.5 * (lo + hi);
            for _ in 0..64 {
                let s = signed_at(u_mid, v)?;
                if s.abs() <= floor {
                    break;
                }
                let slo = signed_at(u_mid, lo)?;
                if slo * s <= 0. {
                    hi = v;
                } else {
                    lo = v;
                }
                v = 0.5 * (lo + hi);
            }
            if signed_at(u_mid, v)?.abs() > floor * 32. {
                continue;
            }
            if !(v > domain[1][0] + floor && v < domain[1][1] - floor) {
                continue;
            }
            let start_p = point3(&surface.evaluate(domain[0][0], v)?.point)?;
            let end_p = point3(&surface.evaluate(domain[0][1], v)?.point)?;
            if (dot3(normal, start_p) - offset).abs() > floor * 32.
                || (dot3(normal, end_p) - offset).abs() > floor * 32.
            {
                continue;
            }
            let uv_b0 = invert_uv(origin, u_dir, v_dir, plane_domain, start_p)?;
            let uv_b1 = invert_uv(origin, u_dir, v_dir, plane_domain, end_p)?;
            components.push(json!({
                "kind":"curve",
                "closed":false,
                "contactClass":"transverse",
                "multiplicity":1,
                "orientation":1,
                "samples":[
                    {"point":start_p,"uvFirst":[domain[0][0],v],"uvSecond":uv_b0,"parameter":0.},
                    {"point":end_p,"uvFirst":[domain[0][1],v],"uvSecond":uv_b1,"parameter":1.}
                ],
                "pcurveFirst":{"kind":"line","start":[domain[0][0],v],"end":[domain[0][1],v],"correspondence":"exact_affine"},
                "pcurveSecond":{"kind":"line","start":uv_b0,"end":uv_b1,"correspondence":"exact_affine"},
                "endpoints":[
                    {"location":"boundary","uvFirst":[domain[0][0],v],"uvSecond":uv_b0},
                    {"location":"boundary","uvFirst":[domain[0][1],v],"uvSecond":uv_b1}
                ],
                "seamWrap":[[0,0],[0,0]],
                "geometryEnclosure":enclosure_of(start_p, floor),
                "coedgeTrim":coedge_trim([0.,1.],[0.,1.],[[0,0],[0,0]]),
                "materialSides":[1,-1],
                "ownership":"half_open_span_faces",
                "junction":false
            }));
        }
    }
    if components.is_empty() {
        return Ok(None);
    }
    Ok(Some(components))
}

fn invert_uv(
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

pub(super) fn plane_plane_line(first: &Surface, second: &Surface, floor: f64) -> Result<Option<Value>> {
    let (Some((n1, o1, u1, v1)), Some((n2, o2, u2, v2))) =
        (affine_plane(first)?, affine_plane(second)?)
    else {
        return Ok(None);
    };
    let direction = cross3(n1, n2);
    let dn = norm3(direction);
    if dn <= TRANSVERSE_SINE {
        // Parallel: either empty or coincident overlap.
        if (o1 - o2).abs() > floor {
            return Ok(Some(json!({
                "kind":"empty",
                "contactClass":"parallel_disjoint",
                "orientation":0
            })));
        }
        let d1 = surface_domain(first);
        let d2 = surface_domain(second);
        return Ok(Some(json!({
            "kind":"overlap",
            "contactClass":"coincident",
            "multiplicity":null,
            "firstUvBox":[d1[0][0],d1[0][1],d1[1][0],d1[1][1]],
            "secondUvBox":[d2[0][0],d2[0][1],d2[1][0],d2[1][1]],
            "orientation":1,
            "reversed":false,
            "seamWrap":[[0,0],[0,0]],
            "geometryEnclosure":enclosure_of(point3(&first.control_points[0][0])?, floor),
            "coedgeTrim":coedge_trim([d1[0][0],d1[0][1]],[d2[0][0],d2[0][1]],[[0,0],[0,0]]),
            "materialSides":[1,-1],
            "ownership":"half_open_first_then_second"
        })));
    }
    let dir = direction.map(|x| x / dn);
    // Line point solving: pick axis with largest |dir| component for stability.
    let abs_dir = [dir[0].abs(), dir[1].abs(), dir[2].abs()];
    let axis = abs_dir
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let mut point = [0.; 3];
    // Solve n1·x=o1, n2·x=o2 with x[axis]=0 then correct onto line.
    let mut a = [[0.; 2]; 2];
    let mut b = [0.; 2];
    let mut row = 0;
    for (n, off) in [(n1, o1), (n2, o2)] {
        let mut col = 0;
        for (i, value) in n.iter().enumerate() {
            if i == axis {
                continue;
            }
            a[row][col] = *value;
            col += 1;
        }
        b[row] = off;
        row += 1;
    }
    let det = a[0][0] * a[1][1] - a[0][1] * a[1][0];
    if !det.is_finite() || det.abs() <= 0. {
        return Ok(None);
    }
    let x0 = (b[0] * a[1][1] - a[0][1] * b[1]) / det;
    let x1 = (a[0][0] * b[1] - b[0] * a[1][0]) / det;
    let mut free = 0;
    for (i, value) in point.iter_mut().enumerate() {
        if i == axis {
            *value = 0.;
        } else {
            *value = if free == 0 { x0 } else { x1 };
            free += 1;
        }
    }
    // Project both surface domains onto the line parameter.
    let origin1 = point3(&first.control_points[0][0])?;
    let origin2 = point3(&second.control_points[0][0])?;
    let d1 = surface_domain(first);
    let d2 = surface_domain(second);
    let corners1 = [
        first.evaluate(d1[0][0], d1[1][0])?.point,
        first.evaluate(d1[0][1], d1[1][0])?.point,
        first.evaluate(d1[0][0], d1[1][1])?.point,
        first.evaluate(d1[0][1], d1[1][1])?.point,
    ];
    let corners2 = [
        second.evaluate(d2[0][0], d2[1][0])?.point,
        second.evaluate(d2[0][1], d2[1][0])?.point,
        second.evaluate(d2[0][0], d2[1][1])?.point,
        second.evaluate(d2[0][1], d2[1][1])?.point,
    ];
    let project = |p: &[f64]| -> f64 {
        let q = point3(p).unwrap_or([0.; 3]);
        dot3([q[0] - point[0], q[1] - point[1], q[2] - point[2]], dir)
    };
    let mut r1 = [f64::INFINITY, f64::NEG_INFINITY];
    for c in &corners1 {
        let s = project(c);
        r1[0] = r1[0].min(s);
        r1[1] = r1[1].max(s);
    }
    let mut r2 = [f64::INFINITY, f64::NEG_INFINITY];
    for c in &corners2 {
        let s = project(c);
        r2[0] = r2[0].min(s);
        r2[1] = r2[1].max(s);
    }
    let lo = r1[0].max(r2[0]);
    let hi = r1[1].min(r2[1]);
    if !hi.is_finite() || hi <= lo + floor {
        return Ok(Some(json!({
            "kind":"empty",
            "contactClass":"transverse_no_overlap",
            "orientation":0
        })));
    }
    let start = [
        point[0] + dir[0] * lo,
        point[1] + dir[1] * lo,
        point[2] + dir[2] * lo,
    ];
    let end = [
        point[0] + dir[0] * hi,
        point[1] + dir[1] * hi,
        point[2] + dir[2] * hi,
    ];
    let uv_a0 = invert_uv(origin1, u1, v1, d1, start)?;
    let uv_a1 = invert_uv(origin1, u1, v1, d1, end)?;
    let uv_b0 = invert_uv(origin2, u2, v2, d2, start)?;
    let uv_b1 = invert_uv(origin2, u2, v2, d2, end)?;
    let side = if dn > TRANSVERSE_SINE { 1_i8 } else { 0 };
    Ok(Some(json!({
        "kind":"curve",
        "closed":false,
        "contactClass":"transverse",
        "multiplicity":1,
        "orientation":side,
        "samples":[
            {"point":start,"uvFirst":uv_a0,"uvSecond":uv_b0,"parameter":0.},
            {"point":[(start[0]+end[0])*0.5,(start[1]+end[1])*0.5,(start[2]+end[2])*0.5],
             "uvFirst":[(uv_a0[0]+uv_a1[0])*0.5,(uv_a0[1]+uv_a1[1])*0.5],
             "uvSecond":[(uv_b0[0]+uv_b1[0])*0.5,(uv_b0[1]+uv_b1[1])*0.5],
             "parameter":0.5},
            {"point":end,"uvFirst":uv_a1,"uvSecond":uv_b1,"parameter":1.}
        ],
        "pcurveFirst":{"kind":"line","start":uv_a0,"end":uv_a1,"correspondence":"exact_affine"},
        "pcurveSecond":{"kind":"line","start":uv_b0,"end":uv_b1,"correspondence":"exact_affine"},
        "endpoints":[
            {"location":"boundary_or_interior","uvFirst":uv_a0,"uvSecond":uv_b0},
            {"location":"boundary_or_interior","uvFirst":uv_a1,"uvSecond":uv_b1}
        ],
        "seamWrap":[[0,0],[0,0]],
        "geometryEnclosure":[
            [next_down(start[0].min(end[0])-floor), next_up(start[0].max(end[0])+floor)],
            [next_down(start[1].min(end[1])-floor), next_up(start[1].max(end[1])+floor)],
            [next_down(start[2].min(end[2])-floor), next_up(start[2].max(end[2])+floor)]
        ],
        "coedgeTrim":coedge_trim([0.,1.],[0.,1.],[[0,0],[0,0]]),
        "materialSides":[1,-1],
        "ownership":"half_open_span_faces"
    })))
}

pub(super) fn ss_contact(
    first: &Surface,
    second: &Surface,
    uv: [f64; 2],
    st: [f64; 2],
    floor: f64,
) -> Result<&'static str> {
    let ja = first.evaluate(uv[0], uv[1])?;
    let jb = second.evaluate(st[0], st[1])?;
    let Some((dua, dva)) = ja.first_derivatives() else {
        return Ok("pole_or_singular");
    };
    let Some((dub, dvb)) = jb.first_derivatives() else {
        return Ok("pole_or_singular");
    };
    let na = cross3(dua, dva);
    let nb = cross3(dub, dvb);
    let la = norm3(na);
    let lb = norm3(nb);
    if !la.is_finite() || !lb.is_finite() || la <= floor || lb <= floor {
        return Ok("pole_or_singular");
    }
    let na = na.map(|x| x / la);
    let nb = nb.map(|x| x / lb);
    let sine = norm3(cross3(na, nb));
    if sine > TRANSVERSE_SINE {
        Ok("transverse")
    } else if sine <= floor {
        Ok("even_tangency")
    } else {
        Ok("odd_tangency")
    }
}
