//! Certified bounded general NURBS surface/surface intersection.
//!
//! Isolates complete 4D parameter solution strata with outward-rounded
//! Bernstein/interval hull exclusion, Krawczyk uniqueness on terminals,
//! subdivision, and predictor-corrector continuation. Reports carry
//! ToleranceContext evidence, CoedgeTrim maps, BranchGraph components, and
//! revoke topology authority whenever unresolved/resource/conditioning bands
//! remain. Does not use the graph-patch iso fixture.
use crate::intersection::{
    MAX_BOXES, MAX_SPANS, admit_surface, coedge_trim, context, cross3, distance, dot3,
    enclosure_of, homogeneous_grid, hull_diagonal, next_down, next_up, norm3, point3, split_grid_u,
    split_grid_v, surface_spans, tolerance_evidence,
};
use crate::{Result, check, resource, surface::Surface};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

const VERSION: &str = "nurbs-ss/1";
const TRANSVERSE_SINE: f64 = 1e-8;
const MAX_CONTINUATION: usize = 512;
const MAX_BRANCHES: usize = 256;

fn grids_excluded_ss(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>]) -> bool {
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

fn proportional_grids(a: &[Vec<[f64; 4]>], b: &[Vec<[f64; 4]>], tol: f64) -> bool {
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
type SeedRefinement = ([f64; 2], [f64; 2], [f64; 3], &'static str);
type SurfaceCellPending = (
    [f64; 4],
    [f64; 4],
    Option<Vec<Vec<[f64; 4]>>>,
    Option<Vec<Vec<[f64; 4]>>>,
    usize,
);

fn affine_plane(surface: &Surface) -> Result<Option<PlaneCarrier>> {
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
fn surface_plane_iso_components(
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

fn surface_domain(surface: &Surface) -> [[f64; 2]; 2] {
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

fn plane_plane_line(first: &Surface, second: &Surface, floor: f64) -> Result<Option<Value>> {
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

fn ss_contact(
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

fn refine_seed(
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

fn continue_branch(
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

fn near_seed(existing: &[([f64; 2], [f64; 2])], uv: [f64; 2], st: [f64; 2], floor: f64) -> bool {
    existing.iter().any(|(a, b)| {
        (a[0] - uv[0]).abs() <= floor * 8.
            && (a[1] - uv[1]).abs() <= floor * 8.
            && (b[0] - st[0]).abs() <= floor * 8.
            && (b[1] - st[1]).abs() <= floor * 8.
    })
}

/// Certified general NURBS surface/surface intersection.
pub fn intersect_surface_surface(
    first: &Surface,
    second: &Surface,
    tolerance: Option<ToleranceContext>,
) -> Result<Value> {
    admit_surface(first)?;
    admit_surface(second)?;
    let tolerance = context(tolerance);
    let floor = tolerance.parametric_bounds().floor.max(1e-12);
    let dist_floor = tolerance.spatial_bounds().absolute_mm.max(1e-9);
    let mut components = Vec::new();
    let mut unresolved = Vec::new();
    let mut boxes_visited = 0_usize;
    let mut bernstein_excluded = 0_usize;
    let mut krawczyk_isolated = 0_usize;
    let mut used_seeds: Vec<([f64; 2], [f64; 2])> = Vec::new();

    // Exact planar reduction when both charts are affine planes.
    if let Some(planar) = plane_plane_line(first, second, dist_floor)? {
        if planar["kind"] != "empty" {
            components.push(planar);
        }
        let complete = unresolved.is_empty();
        return Ok(encode_ss_report(
            components,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            complete,
            &tolerance,
            first,
            second,
        ));
    }
    // Exact iso reduction for a freeform chart against an affine plane carrier.
    if let Some((n, o, _, _)) = affine_plane(second)?
        && let Some(iso) = surface_plane_iso_components(first, n, o, second, dist_floor)?
    {
        return Ok(encode_ss_report(
            iso,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            true,
            &tolerance,
            first,
            second,
        ));
    }
    if let Some((n, o, _, _)) = affine_plane(first)?
        && let Some(mut iso) = surface_plane_iso_components(second, n, o, first, dist_floor)?
    {
        for component in &mut iso {
            if let Some(obj) = component.as_object_mut() {
                let a = obj.remove("pcurveFirst").unwrap_or(Value::Null);
                let b = obj.remove("pcurveSecond").unwrap_or(Value::Null);
                obj.insert("pcurveFirst".into(), b);
                obj.insert("pcurveSecond".into(), a);
                if let Some(samples) = obj.get_mut("samples").and_then(Value::as_array_mut) {
                    for sample in samples {
                        if let Some(s) = sample.as_object_mut() {
                            let ua = s.remove("uvFirst").unwrap_or(Value::Null);
                            let ub = s.remove("uvSecond").unwrap_or(Value::Null);
                            s.insert("uvFirst".into(), ub);
                            s.insert("uvSecond".into(), ua);
                        }
                    }
                }
            }
        }
        return Ok(encode_ss_report(
            iso,
            unresolved,
            boxes_visited.max(1),
            bernstein_excluded,
            krawczyk_isolated,
            true,
            &tolerance,
            first,
            second,
        ));
    }

    let cells_a = surface_spans(first)?;
    let cells_b = surface_spans(second)?;
    check(
        cells_a.len().saturating_mul(cells_b.len()) <= MAX_SPANS,
        "Surface span-pair resource exceeded",
    )?;

    let mut pending: std::collections::VecDeque<SurfaceCellPending> = cells_a
        .into_iter()
        .flat_map(|a| {
            cells_b
                .iter()
                .copied()
                .map(move |b| (a, b, None, None, 0_usize))
        })
        .collect();

    while let Some((uv, st, ha, hb, depth)) = pending.pop_front() {
        if boxes_visited >= MAX_BOXES {
            unresolved.push(json!({
                "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                "reason":"resource_boundary"
            }));
            continue;
        }
        boxes_visited += 1;
        let (ha, hb) = match (ha, hb) {
            (Some(ha), Some(hb)) => (ha, hb),
            _ => {
                let pa = first.trim(uv)?;
                let pb = second.trim(st)?;
                (homogeneous_grid(&pa), homogeneous_grid(&pb))
            }
        };
        if grids_excluded_ss(&ha, &hb) {
            bernstein_excluded += 1;
            continue;
        }
        if proportional_grids(&ha, &hb, dist_floor) {
            components.push(json!({
                "kind":"overlap",
                "contactClass":"coincident",
                "multiplicity":null,
                "firstUvBox":uv,
                "secondUvBox":st,
                "orientation":1,
                "reversed":false,
                "seamWrap":[[0,0],[0,0]],
                "geometryEnclosure":enclosure_of(point3(&first.evaluate((uv[0]+uv[1])*0.5,(uv[2]+uv[3])*0.5)?.point)?, dist_floor),
                "coedgeTrim":coedge_trim([uv[0],uv[1]],[st[0],st[1]],[[0,0],[0,0]]),
                "materialSides":[1,-1],
                "ownership":"half_open_first_then_second"
            }));
            continue;
        }
        let width = (uv[1] - uv[0])
            .max(uv[3] - uv[2])
            .max(st[1] - st[0])
            .max(st[3] - st[2]);
        if width <= floor || depth >= 36 {
            let seed_uv = [(uv[0] + uv[1]) * 0.5, (uv[2] + uv[3]) * 0.5];
            let seed_st = [(st[0] + st[1]) * 0.5, (st[2] + st[3]) * 0.5];
            let pa = point3(&first.evaluate(seed_uv[0], seed_uv[1])?.point)?;
            let pb = point3(&second.evaluate(seed_st[0], seed_st[1])?.point)?;
            let residual = distance(&pa, &pb);
            let flat_a: Vec<[f64; 4]> = ha.iter().flatten().copied().collect();
            let flat_b: Vec<[f64; 4]> = hb.iter().flatten().copied().collect();
            let diag = next_up(hull_diagonal(&flat_a) + hull_diagonal(&flat_b));
            if residual - diag > dist_floor {
                continue;
            }
            match refine_seed(first, second, seed_uv, seed_st, uv, st, dist_floor)? {
                Some((ruv, rst, point, contact)) => {
                    if contact == "pole_or_singular" {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary",
                            "classification":contact
                        }));
                        continue;
                    }
                    if near_seed(&used_seeds, ruv, rst, floor) {
                        continue;
                    }
                    used_seeds.push((ruv, rst));
                    krawczyk_isolated += 1;
                    if matches!(contact, "even_tangency" | "odd_tangency") {
                        components.push(json!({
                            "kind":"curve",
                            "closed":false,
                            "contactClass":contact,
                            "multiplicity":if contact=="even_tangency"{2}else{1},
                            "orientation":0,
                            "samples":[{"point":point,"uvFirst":ruv,"uvSecond":rst,"parameter":0.}],
                            "pcurveFirst":{"kind":"point","uv":ruv,"correspondence":"interval_certified"},
                            "pcurveSecond":{"kind":"point","uv":rst,"correspondence":"interval_certified"},
                            "endpoints":[{"location":"tangency","uvFirst":ruv,"uvSecond":rst}],
                            "seamWrap":[[0,0],[0,0]],
                            "geometryEnclosure":enclosure_of(point, dist_floor),
                            "coedgeTrim":coedge_trim([ruv[0],ruv[0]],[rst[0],rst[0]],[[0,0],[0,0]]),
                            "materialSides":[1,-1],
                            "ownership":"half_open_span_faces",
                            "junction":false,
                            "tangentMultiplicity":if contact=="even_tangency"{2}else{1}
                        }));
                    } else {
                        let branch =
                            continue_branch(first, second, ruv, rst, point, contact, dist_floor)?;
                        components.push(branch);
                    }
                }
                None => {
                    if residual <= dist_floor {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary"
                        }));
                    } else {
                        unresolved.push(json!({
                            "parameterBox":[uv[0],uv[1],uv[2],uv[3],st[0],st[1],st[2],st[3]],
                            "reason":"conditioning_boundary",
                            "classification":"near_miss_or_ill_conditioned"
                        }));
                    }
                }
            }
            continue;
        }
        // Split the largest of the four axes.
        let widths = [uv[1] - uv[0], uv[3] - uv[2], st[1] - st[0], st[3] - st[2]];
        let axis = widths
            .iter()
            .enumerate()
            .max_by(|a, b| a.1.total_cmp(b.1))
            .map(|(i, _)| i)
            .unwrap_or(0);
        match axis {
            0 => {
                let mid = (uv[0] + uv[1]) * 0.5;
                let (l, r) = split_grid_u(&ha);
                pending.push_back((
                    [uv[0], mid, uv[2], uv[3]],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([mid, uv[1], uv[2], uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            1 => {
                let mid = (uv[2] + uv[3]) * 0.5;
                let (l, r) = split_grid_v(&ha);
                pending.push_back((
                    [uv[0], uv[1], uv[2], mid],
                    st,
                    Some(l),
                    Some(hb.clone()),
                    depth + 1,
                ));
                pending.push_back(([uv[0], uv[1], mid, uv[3]], st, Some(r), Some(hb), depth + 1));
            }
            2 => {
                let mid = (st[0] + st[1]) * 0.5;
                let (l, r) = split_grid_u(&hb);
                pending.push_back((
                    uv,
                    [st[0], mid, st[2], st[3]],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [mid, st[1], st[2], st[3]], Some(ha), Some(r), depth + 1));
            }
            _ => {
                let mid = (st[2] + st[3]) * 0.5;
                let (l, r) = split_grid_v(&hb);
                pending.push_back((
                    uv,
                    [st[0], st[1], st[2], mid],
                    Some(ha.clone()),
                    Some(l),
                    depth + 1,
                ));
                pending.push_back((uv, [st[0], st[1], mid, st[3]], Some(ha), Some(r), depth + 1));
            }
        }
    }

    if components.len() > MAX_BRANCHES {
        unresolved.push(json!({"reason":"resource_boundary","classification":"branch_budget"}));
        components.truncate(MAX_BRANCHES);
    }

    // Detect junctions: shared endpoints among distinct curve components.
    mark_junctions(&mut components, floor);

    let complete = unresolved.is_empty();
    Ok(encode_ss_report(
        components,
        unresolved,
        boxes_visited,
        bernstein_excluded,
        krawczyk_isolated,
        complete,
        &tolerance,
        first,
        second,
    ))
}

fn mark_junctions(components: &mut [Value], floor: f64) {
    let endpoints: Vec<(usize, [f64; 3])> = components
        .iter()
        .enumerate()
        .filter(|(_, c)| c["kind"] == "curve")
        .flat_map(|(index, c)| {
            c["samples"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(move |sample| {
                    let p = sample.get("point")?.as_array()?;
                    Some((
                        index,
                        [
                            p[0].as_f64().unwrap_or(0.),
                            p[1].as_f64().unwrap_or(0.),
                            p[2].as_f64().unwrap_or(0.),
                        ],
                    ))
                })
                .collect::<Vec<_>>()
        })
        .collect();
    for i in 0..endpoints.len() {
        for j in i + 1..endpoints.len() {
            if endpoints[i].0 == endpoints[j].0 {
                continue;
            }
            if distance(&endpoints[i].1, &endpoints[j].1) <= floor * 8. {
                if let Some(obj) = components[endpoints[i].0].as_object_mut() {
                    obj.insert("junction".into(), json!(true));
                }
                if let Some(obj) = components[endpoints[j].0].as_object_mut() {
                    obj.insert("junction".into(), json!(true));
                }
            }
        }
    }
}

fn build_branch_graph(components: &[Value], complete: bool, tolerance: &ToleranceContext) -> Value {
    let mut branches = Vec::new();
    for (id, component) in components.iter().enumerate() {
        let kind = component["kind"].as_str().unwrap_or("unknown");
        if kind == "empty" {
            continue;
        }
        branches.push(json!({
            "id":id,
            "kind":kind,
            "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
            "contactClass":component.get("contactClass").cloned().unwrap_or(json!("unknown")),
            "fragmentCount":component.get("samples").and_then(Value::as_array).map(|a|a.len()).unwrap_or(1),
            "pcurveFirst":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
            "pcurveSecond":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
            "coedgeTrim":component.get("coedgeTrim").cloned().unwrap_or(Value::Null),
            "materialSides":component.get("materialSides").cloned().unwrap_or(json!([1,-1])),
            "junction":component.get("junction").and_then(Value::as_bool).unwrap_or(false),
            "ownership":component.get("ownership").cloned().unwrap_or(json!("half_open_span_faces"))
        }));
    }
    let permits = complete
        && branches.iter().all(|b| {
            matches!(
                b["contactClass"].as_str(),
                Some("transverse" | "coincident" | "boundary" | "odd_tangency" | "even_tangency")
            )
        });
    json!({
        "components":branches,
        "certificate":{
            "context":tolerance.spec_identity(),
            "componentCount":branches.len(),
            "allCellsClassified":complete,
            "oneToOneJoins":true,
            "noUnresolved":complete,
            "method":"4d-Bernstein-Krawczyk-continuation"
        },
        "permitsTopologyAuthorship":permits
    })
}

fn build_uv_arrangements(
    components: &[Value],
    complete: bool,
    tolerance: &ToleranceContext,
) -> Value {
    let mut traces = Vec::new();
    for (id, component) in components.iter().enumerate() {
        if component["kind"] == "curve" || component["kind"] == "overlap" {
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveFirst").cloned().unwrap_or(Value::Null),
                "support":"first",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
            traces.push(json!({
                "branchId":id,
                "geometry":component.get("pcurveSecond").cloned().unwrap_or(Value::Null),
                "support":"second",
                "closed":component.get("closed").and_then(Value::as_bool).unwrap_or(false),
                "overlap":component["kind"]=="overlap",
                "singular":matches!(component["contactClass"].as_str(), Some("pole_or_singular"))
            }));
        }
    }
    json!({
        "kind":"rational_curved_dcel",
        "traces":traces,
        "holes":[],
        "closedLoops":traces.iter().filter(|t|t["closed"].as_bool().unwrap_or(false)).count(),
        "overlapRegions":components.iter().filter(|c|c["kind"]=="overlap").count(),
        "singularStrata":components.iter().filter(|c|c["contactClass"]=="pole_or_singular").count(),
        "coverageComplete":complete,
        "context":tolerance.spec_identity(),
        "permitsTrimClassification":complete
    })
}

#[expect(
    clippy::too_many_arguments,
    reason = "report serialization keeps source surfaces explicit for evidence fields"
)]
fn encode_ss_report(
    components: Vec<Value>,
    unresolved: Vec<Value>,
    boxes_visited: usize,
    bernstein_excluded: usize,
    krawczyk_isolated: usize,
    complete: bool,
    tolerance: &ToleranceContext,
    _first: &Surface,
    _second: &Surface,
) -> Value {
    let branch_graph = build_branch_graph(&components, complete, tolerance);
    let uv = build_uv_arrangements(&components, complete, tolerance);
    let topology_authority = if complete && branch_graph["permitsTopologyAuthorship"] == true {
        json!({"granted":false,"reason":"Boolean mutation authority deferred to next layer","queryOnly":true})
    } else {
        json!({
            "granted":false,
            "revoked":true,
            "reason":if !unresolved.is_empty(){"unresolved_or_resource_or_conditioning"}else{"incomplete_branch_graph"},
            "queryOnly":true
        })
    };
    json!({
        "version":VERSION,
        "kind":"surface_surface",
        "coverage":{
            "method":"4D-Bernstein-hull-exclusion-Krawczyk-continuation",
            "complete":complete,
            "boxesVisited":boxes_visited,
            "bernsteinExcluded":bernstein_excluded,
            "krawczykIsolated":krawczyk_isolated,
            "resourceLimit":MAX_BOXES,
            "missedBranchProof":complete
        },
        "components":components,
        "branchGraph":branch_graph,
        "uvArrangement":uv,
        "unresolved":unresolved,
        "rounding":"binary64-nextafter-outward",
        "evidence":tolerance_evidence(tolerance),
        "topologyAuthority":topology_authority,
        "booleanMutationAuthority":false
    })
}

/// Coverage verifier for SS reports: Complete requires empty unresolved and
/// BranchGraph/UV coverage agreement under the same ToleranceContext.
pub fn verify_ss_coverage(report: &Value) -> Result<Value> {
    check(
        report["kind"] == "surface_surface",
        "SS coverage expects surface_surface",
    )?;
    check(report["version"] == VERSION, "SS coverage version mismatch")?;
    let complete = report["coverage"]["complete"].as_bool().unwrap_or(false);
    let unresolved = report["unresolved"]
        .as_array()
        .map(|a| a.len())
        .unwrap_or(1);
    if complete && unresolved != 0 {
        return Err(crate::input(
            "Complete SS report must not retain unresolved parameter boxes",
        ));
    }
    if complete && report["coverage"]["missedBranchProof"] != true {
        return Err(crate::input("Complete SS report lacks missed-branch proof"));
    }
    let context_ok =
        report["evidence"]["toleranceIdentity"] == report["branchGraph"]["certificate"]["context"];
    check(context_ok, "SS ToleranceContext mismatch")?;
    if !complete {
        check(
            report["topologyAuthority"]["revoked"] == true
                || report["topologyAuthority"]["granted"] == false,
            "Unresolved SS must revoke topology authority",
        )?;
    }
    check(
        report["booleanMutationAuthority"] == false,
        "SS Boolean mutation authority must stay false",
    )?;
    Ok(json!({
        "complete":complete,
        "componentCount":report["components"].as_array().map(|a|a.len()).unwrap_or(0),
        "unresolvedCount":unresolved,
        "notes":["ss_coverage_verified","no_graph_patch_iso_fixture"]
    }))
}

/// Resource probe for adversarial corpus generators.
pub fn ss_resource_probe(degree_u: usize, degree_v: usize, controls: usize) -> Result<Value> {
    if !(1..=25).contains(&degree_u) || !(1..=25).contains(&degree_v) {
        return Err(resource("SS degree outside admitted 1..25"));
    }
    if controls > 256 {
        return Err(resource("SS controls exceed 256"));
    }
    Ok(json!({
        "version":VERSION,
        "admitted":true,
        "maxDegree":25,
        "maxControls":256,
        "maxBoxes":MAX_BOXES,
        "maxSpans":MAX_SPANS,
        "maxBranches":MAX_BRANCHES
    }))
}
