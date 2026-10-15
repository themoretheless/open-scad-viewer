//! Original-source proofs for hull separation, identity and rectangular charts.
use super::{
    ContinuationSample, CoverageProof, ExactBranch, ExactEndpointLocation, OverlapRegion, Surface,
    SurfaceSurfaceComponent as C, coedge_trim, cross3, surface_domain,
};
use crate::exact_curve_segments::{exact_add, exact_mul};
fn hull(s: &Surface) -> [[f64; 2]; 3] {
    std::array::from_fn(|k| {
        let values = s.control_points.iter().flatten().map(|p| p[k]);
        [
            values.clone().fold(f64::INFINITY, f64::min),
            values.fold(f64::NEG_INFINITY, f64::max),
        ]
    })
}
fn uv_box(s: &Surface) -> [f64; 4] {
    let d = surface_domain(s);
    [d[0][0], d[0][1], d[1][0], d[1][1]]
}
struct Rect {
    axes: [usize; 2],
    fixed: usize,
    origin: [f64; 2],
    end: [f64; 2],
}
fn rect(s: &Surface) -> Option<Rect> {
    if s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || s.periodic_u
        || s.periodic_v
    {
        return None;
    }
    if s.knots_u[0] != s.knots_u[1]
        || s.knots_u[2] != s.knots_u[3]
        || s.knots_v[0] != s.knots_v[1]
        || s.knots_v[2] != s.knots_v[3]
    {
        return None;
    }
    if !s.weights.iter().flatten().all(|w| *w == s.weights[0][0]) {
        return None;
    }
    let p = &s.control_points;
    for u in 0..3 {
        for v in 0..3 {
            if u == v {
                continue;
            }
            let k = 3 - u - v;
            if p[1][0][u] != p[0][0][u]
                && p[0][1][v] != p[0][0][v]
                && p[0][1][u] == p[0][0][u]
                && p[1][1][u] == p[1][0][u]
                && p[1][0][v] == p[0][0][v]
                && p[1][1][v] == p[0][1][v]
                && p.iter().flatten().all(|q| q[k] == p[0][0][k])
            {
                return Some(Rect {
                    axes: [u, v],
                    fixed: k,
                    origin: [p[0][0][u], p[0][0][v]],
                    end: [p[1][0][u], p[0][1][v]],
                });
            }
        }
    }
    None
}
fn parameter(x: f64, a: f64, b: f64, domain: [f64; 2]) -> Option<f64> {
    if x == a {
        return Some(domain[0]);
    }
    if x == b {
        return Some(domain[1]);
    }
    let n = exact_add(x, -a)?;
    let d = exact_add(b, -a)?;
    let q = n / d;
    if exact_mul(q, d)? != n {
        return None;
    }
    exact_add(domain[0], exact_mul(q, exact_add(domain[1], -domain[0])?)?)
}
impl Rect {
    fn inverse(&self, s: &Surface, p: [f64; 3]) -> Option<[f64; 2]> {
        let d = surface_domain(s);
        Some([
            parameter(p[self.axes[0]], self.origin[0], self.end[0], d[0])?,
            parameter(p[self.axes[1]], self.origin[1], self.end[1], d[1])?,
        ])
    }
    fn normal(&self) -> [f64; 3] {
        let mut u = [0.; 3];
        let mut v = [0.; 3];
        u[self.axes[0]] = if self.end[0] > self.origin[0] {
            1.
        } else {
            -1.
        };
        v[self.axes[1]] = if self.end[1] > self.origin[1] {
            1.
        } else {
            -1.
        };
        cross3(u, v)
    }
}
/// Some(Unproven) retains unsupported contacts/non-binary parameter maps as
/// unknown instead of handing them to a tolerance-based coincidence shortcut.
pub(super) fn reduce(a: &Surface, b: &Surface) -> Option<(Vec<C>, CoverageProof)> {
    let ha = hull(a);
    let hb = hull(b);
    if (0..3).any(|k| ha[k][1] < hb[k][0] || hb[k][1] < ha[k][0]) {
        return Some((vec![], CoverageProof::ControlHullDisjoint));
    }
    if a == b {
        let ua = uv_box(a);
        let ub = uv_box(b);
        return Some((
            vec![C::Overlap(OverlapRegion {
                first_uv_box: ua,
                second_uv_box: ub,
                geometry_enclosure: ha,
                coedge_trim: coedge_trim([ua[0], ua[1]], [ub[0], ub[1]], [[0, 0], [0, 0]]),
            })],
            CoverageProof::SameParameterIdentity,
        ));
    }
    let (Some(ar), Some(br)) = (rect(a), rect(b)) else {
        return None;
    };
    let bounds: [[f64; 2]; 3] =
        std::array::from_fn(|k| [ha[k][0].max(hb[k][0]), ha[k][1].min(hb[k][1])]);
    let varying: Vec<_> = (0..3).filter(|&k| bounds[k][0] < bounds[k][1]).collect();
    let mut start = std::array::from_fn(|k| bounds[k][0]);
    let mut end = std::array::from_fn(|k| bounds[k][1]);
    if varying.len() == 1 && ar.fixed != br.fixed {
        let direction = cross3(ar.normal(), br.normal());
        if direction[varying[0]] < 0. {
            std::mem::swap(&mut start, &mut end);
        }
    } else if !(varying.len() == 2 && ar.fixed == br.fixed) {
        return Some((vec![], CoverageProof::Unproven));
    }
    let mapped = (
        ar.inverse(a, start),
        ar.inverse(a, end),
        br.inverse(b, start),
        br.inverse(b, end),
    );
    let (Some(a0), Some(a1), Some(b0), Some(b1)) = mapped else {
        return Some((vec![], CoverageProof::Unproven));
    };
    if varying.len() == 2 {
        let box_of = |p: [f64; 2], q: [f64; 2]| {
            [
                p[0].min(q[0]),
                p[0].max(q[0]),
                p[1].min(q[1]),
                p[1].max(q[1]),
            ]
        };
        return Some((
            vec![C::Overlap(OverlapRegion {
                first_uv_box: box_of(a0, a1),
                second_uv_box: box_of(b0, b1),
                geometry_enclosure: bounds,
                coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
            })],
            CoverageProof::RectilinearCharts,
        ));
    }
    Some((
        vec![C::Exact(ExactBranch {
            orientation: 1,
            samples: vec![
                ContinuationSample {
                    point: start,
                    uv_first: a0,
                    uv_second: b0,
                    parameter: 0.,
                    seam_wrap: None,
                },
                ContinuationSample {
                    point: end,
                    uv_first: a1,
                    uv_second: b1,
                    parameter: 1.,
                    seam_wrap: None,
                },
            ],
            first_trace: [a0, a1],
            second_trace: [b0, b1],
            endpoint_uv: [[a0, b0], [a1, b1]],
            location: ExactEndpointLocation::Boundary,
            geometry_enclosure: bounds,
            coedge_trim: coedge_trim([0., 1.], [0., 1.], [[0, 0], [0, 0]]),
            junction: None,
        })],
        CoverageProof::RectilinearCharts,
    ))
}

/// Full original graph coverage against a rectangular plane containing the
/// entire source projection. No clipping or ownership conclusion is inferred.
pub(super) fn graph_plane(a: &Surface, b: &Surface) -> Option<(Vec<C>, CoverageProof)> {
    use super::{ContinuedBranch, EndpointLocation};
    if a.periodic_u || a.periodic_v { return None; }
    let br = rect(b)?;
    let ha = hull(a);
    let hb = hull(b);
    // Positive-weight convex hull proves inclusion in the ENTIRE planar chart.
    // If any possible graph point could leave it, this reduction is unavailable.
    if br.axes.iter().any(|&k|ha[k][0]<hb[k][0] || ha[k][1]>hb[k][1]) {return None;}
    let mut normal=[0.;3];normal[br.fixed]=1.;
    let proof=crate::parallel_sections::prove_graph_coverage(a,normal,b.control_points[0][0][br.fixed],100000).ok()??;
    let mut samples=Vec::new();
    for (i,uv) in proof.endpoints.into_iter().enumerate() {
        let p=a.evaluate(uv[0],uv[1]).ok()?.point;
        let d=surface_domain(b);
        let st=std::array::from_fn(|k|d[k][0]+(d[k][1]-d[k][0])*(p[br.axes[k]]-br.origin[k])/(br.end[k]-br.origin[k]));
        samples.push(ContinuationSample {point:[p[0],p[1],p[2]],uv_first:uv,uv_second:st,parameter:i as f64,seam_wrap:None});
    }
    let first=&samples[0];let last=&samples[1];
    let branch=ContinuedBranch {closed:false,junction:false,contact:super::ContactClass::Transverse,
        first_uv:first.uv_first,last_uv:last.uv_first,first_st:first.uv_second,last_st:last.uv_second,
        location:EndpointLocation::Boundary,geometry_enclosure:ha,
        coedge_trim:coedge_trim([0.,1.],[0.,1.],[[0,0],[0,0]]),samples};
    Some((vec![C::Continued(branch)],CoverageProof::MonotoneGraphInRectangle))
}

pub(crate) fn rectangle_chart(s:&Surface)->Option<([usize;2],usize,[f64;2],[f64;2])> {
    let r=rect(s)?;Some((r.axes,r.fixed,r.origin,r.end))
}
