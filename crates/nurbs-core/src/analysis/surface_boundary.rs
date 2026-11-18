//! Seam boundary identifiers, verdicts and outward boundary enclosures shared by
//! analysis and the surface join certificates.
use crate::distance_bounds::Interval as I;
use crate::{Result, surface::Surface, surface_distance::rectangle_bounds, surface_measure::jets};
#[derive(Clone, Copy, Debug)]
pub enum Boundary {
    UMin,
    UMax,
    VMin,
    VMax,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    WithinTolerance,
    CorrespondenceExceedsTolerance,
    Unresolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Orientation {
    Same,
    Opposite,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
pub(crate) fn boundary_box(s: &Surface, edge: Boundary, t: [f64; 2], reverse: bool) -> Result<Vec<I>> {
    let d = domain(s);
    let (fixed, max) = match edge {
        Boundary::UMin => (0, false),
        Boundary::UMax => (0, true),
        Boundary::VMin => (1, false),
        Boundary::VMax => (1, true),
    };
    let free = 1 - fixed;
    let t = I::new(t[0], t[1])?;
    let t = if reverse {
        I::point(1.).sub(t)?.intersect(0., 1.)?
    } else {
        t
    };
    let x = I::point(d[free][0])
        .add(I::point(d[free][1]).sub(I::point(d[free][0]))?.mul(t)?)?
        .intersect(d[free][0], d[free][1])?;
    let mut rectangle = d;
    rectangle[free] = [x.lo, x.hi];
    let endpoint = d[fixed][usize::from(max)];
    rectangle[fixed] = [endpoint, endpoint];
    rectangle_bounds(s, rectangle)?
        .into_iter()
        .map(|b| I::new(b[0], b[1]))
        .collect()
}

pub(crate) fn edge(e: Boundary) -> (usize, bool) {
    match e {
        Boundary::UMin => (0, false),
        Boundary::UMax => (0, true),
        Boundary::VMin => (1, false),
        Boundary::VMax => (1, true),
    }
}
pub(crate) fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}

pub(crate) fn boundary_jets(
    s: &Surface,
    e: Boundary,
    t: [f64; 2],
    reverse: bool,
) -> Result<Vec<jets::Jets>> {
    boundary_jets_impl(s,e,t,reverse,None)
}
pub(crate) fn boundary_graph_jets(s:&Surface,e:Boundary,t:[f64;2],reverse:bool,frame:([usize;3],[f64;2]))->Result<Vec<jets::Jets>> {
    boundary_jets_impl(s,e,t,reverse,Some(frame))
}
fn boundary_jets_impl(s:&Surface,e:Boundary,t:[f64;2],reverse:bool,frame:Option<([usize;3],[f64;2])>)->Result<Vec<jets::Jets>> {
    let (fixed, max) = edge(e);
    let free = 1 - fixed;
    let d = domains(s);
    let mut t = I::new(t[0], t[1])?;
    if reverse {
        t = I::point(1.).sub(t)?.intersect(0., 1.)?;
    }
    let x = I::point(d[free][0])
        .add(I::point(d[free][1]).sub(I::point(d[free][0]))?.mul(t)?)?
        .intersect(d[free][0], d[free][1])?;
    let (p, k, n) = if fixed == 0 {
        (s.degree_u, &s.knots_u, s.control_points.len())
    } else {
        (s.degree_v, &s.knots_v, s.control_points[0].len())
    };
    let fixed_span = (p..n).filter(|&i| k[i] < k[i + 1]);
    let fs = if max {
        fixed_span.last()
    } else {
        fixed_span.into_iter().next()
    }
    .unwrap();
    let (p, k, n) = if free == 0 {
        (s.degree_u, &s.knots_u, s.control_points.len())
    } else {
        (s.degree_v, &s.knots_v, s.control_points[0].len())
    };
    let fixed_knots = if fixed == 0 { &s.knots_u } else { &s.knots_v };
    let mut out = Vec::new();
    for i in p..n {
        let lo = x.lo.max(k[i]);
        let hi = x.hi.min(k[i + 1]);
        if k[i] >= k[i + 1] || lo > hi {
            continue;
        }
        let mut domain = d;
        let mut span = [0; 2];
        span[fixed] = fs;
        span[free] = i;
        domain[fixed] = [fixed_knots[fs], fixed_knots[fs + 1]];
        let mut corner = [None; 2];
        corner[fixed] = Some(usize::from(max));
        if lo == hi {
            if lo - k[i] >= k[i + 1] - lo {
                domain[free] = [k[i], lo];
                corner[free] = Some(1);
            } else {
                domain[free] = [lo, k[i + 1]];
                corner[free] = Some(0);
            }
        } else {
            domain[free] = [lo, hi];
        }
        let j = if let Some(frame)=frame {jets::calculate_partial_stable_graph(s,span,domain,corner,frame)?}else{jets::calculate_partial_stable(s, span, domain, corner)?};
        out.push(j);
    }
    Ok(out)
}
