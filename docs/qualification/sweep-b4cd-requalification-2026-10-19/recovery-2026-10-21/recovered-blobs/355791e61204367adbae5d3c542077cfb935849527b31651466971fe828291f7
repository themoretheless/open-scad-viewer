//! Native predictor/corrector continuation branch geometry.
use super::{
    ContactClass, ContinuationSample, MAX_CONTINUATION, Result, Surface, TRANSVERSE_SINE,
    coedge_trim, cross3, distance, dot3, enclosure_of, norm3, refine_seed, surface_domain,
};
use crate::foundation::guards::{Budget, require_finite_point};

pub enum EndpointLocation {
    Boundary,
    Closed,
    InteriorOrPole,
}
pub struct ContinuedBranch {
    pub closed: bool,
    pub junction: bool,
    pub contact: ContactClass,
    pub samples: Vec<ContinuationSample>,
    pub first_uv: [f64; 2],
    pub last_uv: [f64; 2],
    pub first_st: [f64; 2],
    pub last_st: [f64; 2],
    pub location: EndpointLocation,
    pub geometry_enclosure: [[f64; 2]; 3],
    pub coedge_trim: brep_topology::CoedgeTrim,
}

pub(super) fn continue_branch(
    first: &Surface,
    second: &Surface,
    seed_uv: [f64; 2],
    seed_st: [f64; 2],
    seed_point: [f64; 3],
    contact: ContactClass,
    floor: f64,
) -> Result<ContinuedBranch> {
    let d1 = surface_domain(first);
    let d2 = surface_domain(second);
    // Boundary validation of the marching seed (item 1093).
    require_finite_point(&seed_uv, "ssi seed uv")?;
    require_finite_point(&seed_st, "ssi seed st")?;
    require_finite_point(&seed_point, "ssi seed point")?;
    let step = floor.clamp(1e-3, 0.05);
    let mut samples = vec![ContinuationSample {
        point: seed_point,
        uv_first: seed_uv,
        uv_second: seed_st,
        parameter: 0.,
        seam_wrap: None,
    }];
    let mut uv = seed_uv;
    let mut st = seed_st;
    let mut closed = false;
    let mut hit_boundary = false;
    // Unified guard backing MAX_CONTINUATION on the forward march (1065).
    let mut guard = Budget::with_iterations(MAX_CONTINUATION)?.guard("ssi_marching");
    for i in 1..=MAX_CONTINUATION {
        guard.tick()?;
        guard.check()?;
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
                samples.push(ContinuationSample {
                    point: seed_point,
                    uv_first: seed_uv,
                    uv_second: seed_st,
                    parameter: i as f64 * step,
                    seam_wrap: Some([wrap_a, wrap_b]),
                });
                break;
            }
            samples.push(ContinuationSample {
                point,
                uv_first: ruv,
                uv_second: rst,
                parameter: i as f64 * step,
                seam_wrap: Some([wrap_a, wrap_b]),
            });
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
        let mut guard = Budget::with_iterations(MAX_CONTINUATION)?.guard("ssi_marching_reverse");
        for i in 1..=MAX_CONTINUATION {
            guard.tick()?;
            guard.check()?;
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
                prefix.push(ContinuationSample {
                    point,
                    uv_first: ruv,
                    uv_second: rst,
                    parameter: -(i as f64) * step,
                    seam_wrap: None,
                });
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
        .map(|sample| sample.uv_first)
        .unwrap_or(seed_uv);
    let last_uv = samples
        .last()
        .map(|sample| sample.uv_first)
        .unwrap_or(seed_uv);
    let first_st = samples
        .first()
        .map(|sample| sample.uv_second)
        .unwrap_or(seed_st);
    let last_st = samples
        .last()
        .map(|sample| sample.uv_second)
        .unwrap_or(seed_st);
    Ok(ContinuedBranch {
        junction: false,
        closed,
        contact,
        samples,
        first_uv,
        last_uv,
        first_st,
        last_st,
        location: if hit_boundary {
            EndpointLocation::Boundary
        } else if closed {
            EndpointLocation::Closed
        } else {
            EndpointLocation::InteriorOrPole
        },
        geometry_enclosure: enclosure_of(seed_point, floor),
        coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
    })
}
