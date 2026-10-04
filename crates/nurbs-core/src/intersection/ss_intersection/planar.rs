//! Native affine and iso-parametric intersection results.
use super::{
    ContinuationSample, Result, Surface, TRANSVERSE_SINE, affine_plane, coedge_trim, cross3, dot3,
    enclosure_of, invert_uv, next_down, next_up, norm3, point3, surface_domain,
};

pub(super) enum PlanarIntersection {
    Empty,
    Overlap(OverlapRegion),
    Curve(ExactBranch),
}
pub struct OverlapRegion {
    pub first_uv_box: [f64; 4],
    pub second_uv_box: [f64; 4],
    pub geometry_enclosure: [[f64; 2]; 3],
    pub coedge_trim: brep_topology::CoedgeTrim,
}
pub enum ExactEndpointLocation {
    Boundary,
    BoundaryOrInterior,
}
pub struct ExactBranch {
    pub orientation: i8,
    pub samples: Vec<ContinuationSample>,
    pub first_trace: [[f64; 2]; 2],
    pub second_trace: [[f64; 2]; 2],
    pub endpoint_uv: [[[f64; 2]; 2]; 2],
    pub location: ExactEndpointLocation,
    pub geometry_enclosure: [[f64; 2]; 3],
    pub coedge_trim: brep_topology::CoedgeTrim,
    pub junction: Option<bool>,
}
impl ExactBranch {
    pub fn swap_supports(&mut self) {
        std::mem::swap(&mut self.first_trace, &mut self.second_trace);
        for sample in &mut self.samples {
            std::mem::swap(&mut sample.uv_first, &mut sample.uv_second);
        }
    }
}

pub(super) fn plane_plane_line(
    first: &Surface,
    second: &Surface,
    floor: f64,
) -> Result<Option<PlanarIntersection>> {
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
            return Ok(Some(PlanarIntersection::Empty));
        }
        let d1 = surface_domain(first);
        let d2 = surface_domain(second);
        return Ok(Some(PlanarIntersection::Overlap(OverlapRegion {
            first_uv_box: [d1[0][0], d1[0][1], d1[1][0], d1[1][1]],
            second_uv_box: [d2[0][0], d2[0][1], d2[1][0], d2[1][1]],
            geometry_enclosure: enclosure_of(point3(&first.control_points[0][0])?, floor),
            coedge_trim: coedge_trim([d1[0][0], d1[0][1]], [d2[0][0], d2[0][1]], [[0, 0], [0, 0]]),
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
        return Ok(Some(PlanarIntersection::Empty));
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
    Ok(Some(PlanarIntersection::Curve(ExactBranch {
        orientation: side,
        samples: vec![
            ContinuationSample {
                point: start,
                uv_first: uv_a0,
                uv_second: uv_b0,
                parameter: 0.,
                seam_wrap: None,
            },
            ContinuationSample {
                point: std::array::from_fn(|i| (start[i] + end[i]) * 0.5),
                uv_first: std::array::from_fn(|i| (uv_a0[i] + uv_a1[i]) * 0.5),
                uv_second: std::array::from_fn(|i| (uv_b0[i] + uv_b1[i]) * 0.5),
                parameter: 0.5,
                seam_wrap: None,
            },
            ContinuationSample {
                point: end,
                uv_first: uv_a1,
                uv_second: uv_b1,
                parameter: 1.,
                seam_wrap: None,
            },
        ],
        first_trace: [uv_a0, uv_a1],
        second_trace: [uv_b0, uv_b1],
        endpoint_uv: [[uv_a0, uv_b0], [uv_a1, uv_b1]],
        location: ExactEndpointLocation::BoundaryOrInterior,
        geometry_enclosure: std::array::from_fn(|i| {
            [
                next_down(start[i].min(end[i]) - floor),
                next_up(start[i].max(end[i]) + floor),
            ]
        }),
        coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
        junction: None,
    })))
}

/// Exact iso branches of a general chart against an affine plane when signed
/// distance separates as a monotone function of one parameter on the control net.
pub(super) fn surface_plane_iso_components(
    surface: &Surface,
    normal: [f64; 3],
    offset: f64,
    plane: &Surface,
    floor: f64,
) -> Result<Option<Vec<ExactBranch>>> {
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
            let trace = [[u, domain[1][0]], [u, domain[1][1]]];
            components.push(ExactBranch {
                orientation: 1,
                samples: vec![
                    ContinuationSample {
                        point: start_p,
                        uv_first: trace[0],
                        uv_second: uv_b0,
                        parameter: 0.,
                        seam_wrap: None,
                    },
                    ContinuationSample {
                        point: end_p,
                        uv_first: trace[1],
                        uv_second: uv_b1,
                        parameter: 1.,
                        seam_wrap: None,
                    },
                ],
                first_trace: trace,
                second_trace: [uv_b0, uv_b1],
                endpoint_uv: [[trace[0], uv_b0], [trace[1], uv_b1]],
                location: ExactEndpointLocation::Boundary,
                geometry_enclosure: enclosure_of(start_p, floor),
                coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
                junction: Some(false),
            });
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
            let trace = [[domain[0][0], v], [domain[0][1], v]];
            components.push(ExactBranch {
                orientation: 1,
                samples: vec![
                    ContinuationSample {
                        point: start_p,
                        uv_first: trace[0],
                        uv_second: uv_b0,
                        parameter: 0.,
                        seam_wrap: None,
                    },
                    ContinuationSample {
                        point: end_p,
                        uv_first: trace[1],
                        uv_second: uv_b1,
                        parameter: 1.,
                        seam_wrap: None,
                    },
                ],
                first_trace: trace,
                second_trace: [uv_b0, uv_b1],
                endpoint_uv: [[trace[0], uv_b0], [trace[1], uv_b1]],
                location: ExactEndpointLocation::Boundary,
                geometry_enclosure: enclosure_of(start_p, floor),
                coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
                junction: Some(false),
            });
        }
    }
    if components.is_empty() {
        return Ok(None);
    }
    Ok(Some(components))
}
