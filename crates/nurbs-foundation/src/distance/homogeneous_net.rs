//! Homogeneous Bernstein nets of rational surface spans: extraction from the original poles with
//! outward interval arithmetic, restriction to sub-rectangles, derivative nets and coordinate
//! hulls. Formerly split between `analysis::curve_surface_composition` and
//! `analysis::normal_alignment`; moved here because `distance` needs it (ADR 0012).
use crate::{check, distance_bounds::Interval, surface::Surface, Result};
use Interval as I;

pub type Net = Vec<Vec<[I; 4]>>;

#[path = "homogeneous_net/exact.rs"]
mod exact;
pub use exact::ExactIdentities;
#[path = "homogeneous_net/restrict_net.rs"]
mod restrict_net;
pub use restrict_net::restrict_net;
#[path = "homogeneous_net/restrict_hull.rs"]
mod restrict_hull;
pub use restrict_hull::restrict_hull;
#[path = "homogeneous_net/restrict_unit_net.rs"]
mod restrict_unit_net;
pub use restrict_unit_net::restrict_unit_net;

pub fn surface_net(s: &Surface, span: [usize; 2]) -> Result<Vec<Vec<[Interval; 4]>>> {
    surface_net_on(
        s,
        span,
        [
            [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
            [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
        ],
    )
}
fn surface_homogeneous_controls_with_origin(s: &Surface, span: [usize;2], origin:Option<[f64;3]>) -> Result<(Vec<Vec<[Interval;4]>>, [f64;2])> {
    surface_homogeneous_controls_weighted(s,span,origin,|i,j|Ok(Interval::point(s.weights[i][j])))
}
/// Extract from an original chart with an explicitly supplied homogeneous
/// weight expression. Callers own validation and weight/source correspondence.
fn surface_homogeneous_controls_weighted(s: &Surface, span: [usize;2], origin:Option<[f64;3]>, weight:impl Fn(usize,usize)->Result<Interval>) -> Result<(Vec<Vec<[Interval;4]>>, [f64;2])> {
    let [iu, iv] = span; let (p,q) = (s.degree_u,s.degree_v);
    let mut controls = Vec::new();
    let mut weights = [f64::INFINITY, 0_f64];
    for i in iu - p..=iu {
        let mut row = Vec::new();
        for j in iv - q..=iv {
            let w = weight(i,j)?;
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let cp = &s.control_points[i][j];
            let mut centered=[Interval::point(cp[0]),Interval::point(cp[1]),Interval::point(cp[2])];
            if let Some(origin)=origin {for k in 0..3 {centered[k]=centered[k].exact_sub(Interval::point(origin[k]))?;}}
            row.push([
                centered[0].exact_mul(w)?,
                centered[1].exact_mul(w)?,
                centered[2].exact_mul(w)?,
                w,
            ]);
        }
        controls.push(row);
    }
    Ok((controls, weights))
}
/// Exact Bernstein restriction identity with outward interval arithmetic.
/// Only complete original Bernstein spans qualify; general B-splines keep
/// the original blossom routine. No surface poles or knots are modified.
pub fn surface_net_on_bernstein(s: &Surface, span: [usize;2], domain: [[f64;2];2]) -> Result<Option<Vec<Vec<[Interval;4]>>>> {
    surface_net_on_bernstein_with_origin(s,span,domain,None)
}
fn surface_net_on_bernstein_with_origin(s:&Surface,span:[usize;2],domain:[[f64;2];2],origin:Option<[f64;3]>)->Result<Option<Vec<Vec<[Interval;4]>>>> {
    let [iu,iv]=span; let (p,q)=(s.degree_u,s.degree_v);
    if !crate::surface_distance::complete_bernstein_span(p,&s.knots_u,iu)
        || !crate::surface_distance::complete_bernstein_span(q,&s.knots_v,iv) { return Ok(None); }
    let spans=[[s.knots_u[iu],s.knots_u[iu+1]],[s.knots_v[iv],s.knots_v[iv+1]]];
    if (0..2).any(|axis| domain[axis].iter().any(|x|!x.is_finite())
        ||domain[axis][0]<spans[axis][0]||domain[axis][1]>spans[axis][1]||domain[axis][0]>domain[axis][1]) { return Ok(None); }
    let (controls,weights)=surface_homogeneous_controls_with_origin(s,span,origin)?;
    let mut rows=Vec::with_capacity(p+1);
    for row in controls {
        let mut restricted=crate::surface_distance::restrict_bernstein_controls(&row,spans[1],domain[1],weights)?;
        if restricted.len()==1 { restricted.resize(q+1,restricted[0]); }
        rows.push(restricted);
    }
    let mut out=vec![vec![[Interval::point(0.);4];q+1];p+1];
    for j in 0..=q {
        let column=rows.iter().map(|row|row[j]).collect::<Vec<_>>();
        let mut restricted=crate::surface_distance::restrict_bernstein_controls(&column,spans[0],domain[0],weights)?;
        if restricted.len()==1 { restricted.resize(p+1,restricted[0]); }
        for (i,h) in restricted.into_iter().enumerate() { out[i][j]=h; }
    }
    Ok(Some(out))
}
pub fn surface_net_on(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Vec<[Interval; 4]>>> {
    surface_net_on_with_origin(s,span,domain,None)
}
/// Translate the original homogeneous poles before interval restriction.
/// Subtraction and multiplication are outward; original surfaces stay intact.
pub fn surface_net_on_centered(s:&Surface,span:[usize;2],domain:[[f64;2];2],origin:[f64;3])->Result<Vec<Vec<[Interval;4]>>> {
    let mut net=match surface_net_on_bernstein_with_origin(s,span,domain,Some(origin))? {
        Some(net)=>net,None=>surface_net_on_with_origin(s,span,domain,Some(origin))?,
    };
    // Linear extraction preserves a literal zero coefficient row. Recognize
    // that identity on original controls before outward arithmetic can create
    // subnormal intervals around zero. No small nonzero value is discarded.
    for axis in 0..2 {
        let (degree,knots)=if axis==0 {(s.degree_u,&s.knots_u)}else{(s.degree_v,&s.knots_v)};
        if !crate::surface_distance::complete_bernstein_span(degree,knots,span[axis])
            || domain[axis]!=[knots[span[axis]],knots[span[axis]+1]] {continue;}
        for i in 0..=degree {for k in 0..3 {
            let fixed=span[axis]-degree+i;
            let other_degree=if axis==0 {s.degree_v}else{s.degree_u};
            let zero=(span[1-axis]-other_degree..=span[1-axis]).all(|j|{
                let (u,v)=if axis==0 {(fixed,j)}else{(j,fixed)};
                s.control_points[u][v][k]==origin[k]
            });
            if zero {for j in 0..=other_degree {
                let (u,v)=if axis==0 {(i,j)}else{(j,i)};net[u][v][k]=Interval::point(0.);
            }}
        }}
    }
    Ok(net)
}
fn surface_net_on_with_origin(s:&Surface,span:[usize;2],domain:[[f64;2];2],origin:Option<[f64;3]>)->Result<Vec<Vec<[Interval;4]>>> {
    surface_net_on_weighted(s,span,domain,origin,|i,j|Ok(Interval::point(s.weights[i][j])))
}
/// Extract from an original chart with an explicitly supplied homogeneous
/// weight expression. Callers own validation and weight/source correspondence.
pub fn surface_net_on_with_weights(
    s:&Surface,span:[usize;2],domain:[[f64;2];2],
    weight:impl Fn(usize,usize)->Result<Interval>,
)->Result<Vec<Vec<[Interval;4]>>> {
    surface_net_on_weighted(s,span,domain,None,weight)
}
/// Center original positions before homogeneous multiplication and extraction.
/// The interval subtraction encloses the exact translated control coordinates.
pub fn surface_net_on_with_weights_centered(
    s:&Surface,span:[usize;2],domain:[[f64;2];2],
    weight:impl Fn(usize,usize)->Result<Interval>,origin:[f64;3],
)->Result<Vec<Vec<[Interval;4]>>> {
    surface_net_on_weighted(s,span,domain,Some(origin),weight)
}
fn surface_net_on_weighted(s:&Surface,span:[usize;2],domain:[[f64;2];2],origin:Option<[f64;3]>,weight:impl Fn(usize,usize)->Result<Interval>)->Result<Vec<Vec<[Interval;4]>>> {
    let [iu, iv] = span;
    let (p, q) = (s.degree_u, s.degree_v);
    let (controls, weights) = surface_homogeneous_controls_weighted(s, span, origin, weight)?;
    let mut out = vec![vec![[Interval::point(0.); 4]; q + 1]; p + 1];
    let u_plans = (0..=p).map(|u| {
        let us = (0..p).map(|k| domain[0][usize::from(k >= p - u)]).collect::<Vec<_>>();
        crate::surface_distance::interpolation_plan(p, &s.knots_u, iu, &us)
    }).collect::<Result<Vec<_>>>()?;
    let mut rows = Vec::with_capacity(controls.len());
    for v in 0..=q {
        let vs = (0..q).map(|k| domain[1][usize::from(k >= q - v)]).collect::<Vec<_>>();
        let v_plan = crate::surface_distance::interpolation_plan(q, &s.knots_v, iv, &vs)?;
        rows.clear();
        for r in &controls {
            rows.push(crate::surface_distance::interpolate_planned(r, q, &v_plan, weights)?);
        }
        for (u, plan) in u_plans.iter().enumerate() {
            out[u][v] = crate::surface_distance::interpolate_planned(&rows, p, plan, weights)?;
        }
    }
    Ok(out)
}

/// Positive denominator Bernstein coefficients make a rational polynomial a
/// convex combination of coefficient quotients, including after restriction.
pub fn rational_quotient_hull(net: &Net, domain: [[f64; 2]; 2]) -> Result<[I; 3]> {
    let restricted = restrict_unit_net(net, domain)?;
    let mut out = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    for h in restricted.iter().flatten() {
        for k in 0..3 {
            let value = h[k].div(h[3])?;
            out[k].lo = out[k].lo.min(value.lo);
            out[k].hi = out[k].hi.max(value.hi);
        }
    }
    Ok(out)
}
/// Signed denominators are allowed only when all restricted coefficients have
/// the same separated sign, so their rational convex weights remain positive.
pub fn signed_rational_quotient_hull(
    net: &Net,
    domain: [[f64; 2]; 2],
) -> Result<Option<[I; 3]>> {
    let r = restrict_unit_net(net, domain)?;
    let positive = r.iter().flatten().all(|h| h[3].lo > 0.);
    let negative = r.iter().flatten().all(|h| h[3].hi < 0.);
    if !positive && !negative {
        return Ok(None);
    }
    let mut out = [I {
        lo: f64::INFINITY,
        hi: f64::NEG_INFINITY,
    }; 3];
    for h in r.iter().flatten() {
        for k in 0..3 {
            let x = h[k].div_signed(h[3])?;
            out[k].lo = out[k].lo.min(x.lo);
            out[k].hi = out[k].hi.max(x.hi);
        }
    }
    Ok(Some(out))
}
pub fn jacobian_on(
    s: &Surface,
    indices: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<[[I; 2]; 3]> {
    let [u, v] = indices;
    let (p, q) = (s.degree_u, s.degree_v);
    let span = [
        [s.knots_u[u], s.knots_u[u + 1]],
        [s.knots_v[v], s.knots_v[v + 1]],
    ];
    let origin = s.evaluate_validated(domain[0][0]*0.5+domain[0][1]*0.5,
        domain[1][0]*0.5+domain[1][1]*0.5)?.point;
    // Translate original controls before homogeneous multiplication/extraction.
    // The finite origin is only a numerical reference; interval subtraction
    // encloses the exact translated expression and its unchanged derivatives.
    let net = surface_net_on_with_weights_centered(
        s,indices,span,|i,j|Ok(I::point(s.weights[i][j])),
        std::array::from_fn(|k|origin[k]),
    )?;
    jacobian_net_on(net, [p,q], span, domain, &[0.;3])
}
/// Any finite translation cancels from the quotient derivative. The supplied
/// homogeneous coefficients must describe the actual surface being qualified.
pub fn jacobian_net_on(mut net: Net, [p,q]: [usize;2],
    span:[[f64;2];2], domain:[[f64;2];2], origin:&[f64]) -> Result<[[I;2];3]> {
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let values = restrict_hull(&net, [p, q], span, domain)?;
    let denominator = values[3].mul(values[3])?;
    let mut result = [[I::point(0.); 2]; 3];
    for axis in 0..2 {
        let degree = [p, q][axis];
        if degree == 0 {
            continue;
        }
        let derivatives = differentiate_net(&net, [p, q], axis, span)?;
        let d = restrict_hull(
            &derivatives,
            [p - usize::from(axis == 0), q - usize::from(axis == 1)],
            span,
            domain,
        )?;
        for k in 0..3 {
            result[k][axis] = d[k]
                .mul(values[3])?
                .sub(values[k].mul(d[3])?)?
                .div(denominator)?;
        }
    }
    Ok(result)
}
pub fn differentiate_net(
    net: &Net,
    degrees: [usize; 2],
    axis: usize,
    span: [[f64; 2]; 2],
) -> Result<Net> {
    let degree = degrees[axis];
    let mut out = vec![
        vec![[I::point(0.); 4]; degrees[1] + 1 - usize::from(axis == 1)];
        degrees[0] + 1 - usize::from(axis == 0)
    ];
    let width = I::point(span[axis][1]).sub(I::point(span[axis][0]))?;
    for i in 0..out.len() {
        for j in 0..out[0].len() {
            for k in 0..4 {
                out[i][j][k] = net[i + usize::from(axis == 0)][j + usize::from(axis == 1)][k]
                    .sub(net[i][j][k])?
                    .mul(I::point(degree as f64))?
                    .div(width)?;
            }
        }
    }
    Ok(out)
}
