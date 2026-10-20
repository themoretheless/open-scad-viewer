//! Correlated Bernstein bound for a rational Bezier curve/surface composition.
//! Every polynomial operation uses outward interval arithmetic. Unsupported
//! degrees or knot layouts leave the general interval verifier in charge.
use crate::{Result, curve::Curve, distance_bounds::Interval, surface::Surface};
type Poly = Vec<Interval>;
fn choose(n: usize, k: usize) -> f64 {
    // n <= 50: all integer intermediates and results fit u64, and the result
    // is below 2^53, hence exactly representable as binary64.
    let mut v = 1u64;
    for i in 0..k.min(n - k) {
        v = v * (n - i) as u64 / (i + 1) as u64;
    }
    v as f64
}
pub(crate) fn mul(a: &[Interval], b: &[Interval]) -> Result<Poly> {
    let (p, q) = (a.len() - 1, b.len() - 1);
    let mut out = vec![Interval::point(0.); p + q + 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            let factor = Interval::point(choose(p, i))
                .mul(Interval::point(choose(q, j)))?
                .div(Interval::point(choose(p + q, i + j)))?;
            out[i + j] = out[i + j].add(x.mul(y)?.mul(factor)?)?;
        }
    }
    Ok(out)
}
fn powers(p: &Poly, n: usize) -> Result<Vec<Poly>> {
    let mut out = vec![vec![Interval::point(1.)]];
    for i in 0..n {
        out.push(mul(&out[i], p)?);
    }
    Ok(out)
}
fn bezier(knots: &[f64], degree: usize, n: usize) -> bool {
    n == degree + 1
        && knots[..=degree].iter().all(|&x| x == knots[degree])
        && knots[degree + 1..].iter().all(|&x| x == knots[degree + 1])
}
fn homogeneous(c: &Curve, reversed: bool) -> Result<Vec<Poly>> {
    let dimension = c.control_points[0].len();
    let mut out = vec![Vec::new(); dimension + 1];
    for i in 0..c.control_points.len() {
        let j = if reversed {
            c.control_points.len() - 1 - i
        } else {
            i
        };
        let w = Interval::point(c.weights[j]);
        for k in 0..dimension {
            out[k].push(Interval::point(c.control_points[j][k]).mul(w)?);
        }
        out[dimension].push(w);
    }
    Ok(out)
}
/// Inputs have already passed curve/surface validation. This is an optional
/// bound over their complete natural domains, in normalized traversal order.
pub(crate) fn upper(c: &Curve, p: &Curve, s: &Surface, reversed: bool) -> Result<Option<f64>> {
    let (du, dv) = (s.degree_u, s.degree_v);
    let degree = p.degree.saturating_mul(du.saturating_add(dv));
    // The composed lift has degree <=32 and the source degree <=16, so
    // every cross-multiplied polynomial remains within choose()'s exact <=48
    // binomial range. Tensor degree alone need not exclude a 6x6 surface.
    if degree > 32
        || c.degree > 16
        || !bezier(&c.knots, c.degree, c.control_points.len())
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, du, s.control_points.len())
        || !bezier(&s.knots_v, dv, s.control_points[0].len())
    {
        return Ok(None);
    }
    let domains = [
        [s.knots_u[du], s.knots_u[du + 1]],
        [s.knots_v[dv], s.knots_v[dv + 1]],
    ];
    // Positive rational convex hull proves that the lift stays in this chart.
    if p.control_points
        .iter()
        .any(|cp| (0..2).any(|k| cp[k] < domains[k][0] || cp[k] > domains[k][1]))
    {
        return Ok(None);
    }
    let net = (0..=du)
        .map(|i| {
            (0..=dv)
                .map(|j| {
                    let w = Interval::point(s.weights[i][j]);
                    Ok([
                        Interval::point(s.control_points[i][j][0]).mul(w)?,
                        Interval::point(s.control_points[i][j][1]).mul(w)?,
                        Interval::point(s.control_points[i][j][2]).mul(w)?,
                        w,
                    ])
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    bound(
        homogeneous(c, reversed)?,
        homogeneous(p, false)?,
        net,
        [du, dv],
        domains,
    )
}
fn bound(
    curve: Vec<Poly>,
    h: Vec<Poly>,
    net: Vec<Vec<[Interval; 4]>>,
    degrees: [usize; 2],
    domains: [[f64; 2]; 2],
) -> Result<Option<f64>> {
    let [du, dv] = degrees;
    let lift = lift_coefficients(&h, &net, [du, dv], domains)?;
    let denominator = mul(&curve[3], &lift[3])?;
    let minimum = denominator
        .iter()
        .map(|x| x.lo)
        .fold(f64::INFINITY, f64::min);
    if minimum <= 0. {
        return Ok(None);
    }
    let mut squared = Interval::point(0.);
    for k in 0..3 {
        let a = mul(&curve[k], &lift[3])?;
        let b = mul(&lift[k], &curve[3])?;
        let mut maximum = 0_f64;
        for (&x, &y) in a.iter().zip(&b) {
            let d = x.sub(y)?;
            maximum = maximum.max(d.lo.abs()).max(d.hi.abs());
        }
        let ratio = Interval::point(maximum).div(Interval::point(minimum))?;
        squared = squared.add(ratio.mul(ratio)?)?;
    }
    Ok(Some(squared.hi.max(0.).sqrt().next_up()))
}

fn span(knots: &[f64], degree: usize, n: usize, range: Interval) -> Option<usize> {
    (degree..n)
        .find(|&i| knots[i] < knots[i + 1] && range.lo >= knots[i] && range.hi <= knots[i + 1])
}
/// Homogeneous blossom of the original curve, with interval affine endpoint
/// mapping. Never round a newly extracted control point before certification.
fn restricted_ends(c: &Curve, ends: [Interval; 2]) -> Result<Option<Vec<Poly>>> {
    let range = Interval::new(ends[0].lo.min(ends[1].lo), ends[0].hi.max(ends[1].hi))?;
    let Some(span) = span(&c.knots, c.degree, c.control_points.len(), range) else {
        return Ok(None);
    };
    let dim = c.control_points[0].len();
    let mut source = Vec::new();
    for i in span - c.degree..=span {
        let w = Interval::point(c.weights[i]);
        let mut row = c.control_points[i]
            .iter()
            .map(|&x| Interval::point(x).mul(w))
            .collect::<Result<Vec<_>>>()?;
        row.push(w);
        source.push(row);
    }
    let mut out = vec![Vec::new(); dim + 1];
    for k in 0..=c.degree {
        let mut d = source.clone();
        for r in 1..=c.degree {
            let parameter = ends[usize::from(r > c.degree - k)];
            for j in (r..=c.degree).rev() {
                let i = span - c.degree + j;
                let alpha = parameter
                    .sub(Interval::point(c.knots[i]))?
                    .div(
                        Interval::point(c.knots[i + c.degree - r + 1])
                            .sub(Interval::point(c.knots[i]))?,
                    )?
                    .intersect(0., 1.)?;
                let beta = Interval::point(1.).sub(alpha)?.intersect(0., 1.)?;
                for axis in 0..=dim {
                    let (a, b) = (d[j - 1][axis], d[j][axis]);
                    d[j][axis] = a
                        .mul(beta)?
                        .add(b.mul(alpha)?)?
                        .intersect(a.lo.min(b.lo), a.hi.max(b.hi))?;
                }
            }
        }
        for axis in 0..=dim {
            out[axis].push(d[c.degree][axis]);
        }
    }
    Ok(Some(out))
}
pub(crate) fn surface_net(s: &Surface, span: [usize; 2]) -> Result<Vec<Vec<[Interval; 4]>>> {
    surface_net_on(
        s,
        span,
        [
            [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
            [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
        ],
    )
}
fn surface_homogeneous_controls(s: &Surface, span: [usize;2]) -> Result<(Vec<Vec<[Interval;4]>>, [f64;2])> {
    let [iu, iv] = span; let (p,q) = (s.degree_u,s.degree_v);
    let mut controls = Vec::new();
    let mut weights = [f64::INFINITY, 0_f64];
    for i in iu - p..=iu {
        let mut row = Vec::new();
        for j in iv - q..=iv {
            let w = Interval::point(s.weights[i][j]);
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let cp = &s.control_points[i][j];
            row.push([
                Interval::point(cp[0]).mul(w)?,
                Interval::point(cp[1]).mul(w)?,
                Interval::point(cp[2]).mul(w)?,
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
pub(crate) fn surface_net_on_bernstein(s: &Surface, span: [usize;2], domain: [[f64;2];2]) -> Result<Option<Vec<Vec<[Interval;4]>>>> {
    let [iu,iv]=span; let (p,q)=(s.degree_u,s.degree_v);
    if !crate::surface_distance::complete_bernstein_span(p,&s.knots_u,iu)
        || !crate::surface_distance::complete_bernstein_span(q,&s.knots_v,iv) { return Ok(None); }
    let spans=[[s.knots_u[iu],s.knots_u[iu+1]],[s.knots_v[iv],s.knots_v[iv+1]]];
    if (0..2).any(|axis| domain[axis].iter().any(|x|!x.is_finite())
        ||domain[axis][0]<spans[axis][0]||domain[axis][1]>spans[axis][1]||domain[axis][0]>domain[axis][1]) { return Ok(None); }
    let (controls,weights)=surface_homogeneous_controls(s,span)?;
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
pub(crate) fn surface_net_on(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Vec<[Interval; 4]>>> {
    let [iu, iv] = span;
    let (p, q) = (s.degree_u, s.degree_v);
    let (controls, weights) = surface_homogeneous_controls(s, span)?;
    let mut out = vec![vec![[Interval::point(0.); 4]; q + 1]; p + 1];
    let u_plans = (0..=p).map(|u| {
        let us = (0..p).map(|k| domain[0][usize::from(k >= p - u)]).collect::<Vec<_>>();
        crate::surface_distance::interpolation_plan(p, &s.knots_u, iu, &us)
    }).collect::<Result<Vec<_>>>()?;
    for v in 0..=q {
        let vs = (0..q).map(|k| domain[1][usize::from(k >= q - v)]).collect::<Vec<_>>();
        let v_plan = crate::surface_distance::interpolation_plan(q, &s.knots_v, iv, &vs)?;
        let rows = controls.iter().map(|r| {
            crate::surface_distance::interpolate_planned(r.clone(), q, &v_plan, weights)
        }).collect::<Result<Vec<_>>>()?;
        for (u, plan) in u_plans.iter().enumerate() {
            out[u][v] = crate::surface_distance::interpolate_planned(rows.clone(), p, plan, weights)?;
        }
    }
    Ok(out)
}
/// Optional bound on one normalized interval. Every intersected surface chart
/// contributes an extension bound; unsupported cases retain general coverage.
pub(crate) fn upper_cell(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    t: [f64; 2],
) -> Result<Option<f64>> {
    use crate::curve_surface_agreement::mapped;
    upper_mapped_cell(
        c, p, s,
        [
            mapped(c, Interval::point(t[0]), reversed)?,
            mapped(c, Interval::point(t[1]), reversed)?,
        ],
        [
            mapped(p, Interval::point(t[0]), false)?,
            mapped(p, Interval::point(t[1]), false)?,
        ],
    )
}
/// Endpoint intervals describe an affine relation on the ORIGINAL curves.
pub(crate) fn upper_mapped_cell(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    edge_ends: [Interval; 2],
    pcurve_ends: [Interval; 2],
) -> Result<Option<f64>> {
    let (du, dv) = (s.degree_u, s.degree_v);
    if p.degree.saturating_mul(du.saturating_add(dv)) > 32
        || c.degree > 16
    {
        return Ok(None);
    }
    let Some(curve) = restricted_ends(c, edge_ends)? else {
        return Ok(None);
    };
    let Some(h) = restricted_ends(p, pcurve_ends)? else {
        return Ok(None);
    };
    let range = Interval::new(
        pcurve_ends[0].lo.min(pcurve_ends[1].lo),
        pcurve_ends[0].hi.max(pcurve_ends[1].hi),
    )?;
    let uv = crate::curve_surface_agreement::curve_bounds(p, range)?;
    let domains = [
        [s.knots_u[du], s.knots_u[s.control_points.len()]],
        [s.knots_v[dv], s.knots_v[s.control_points[0].len()]],
    ];
    let Some(us) = crate::periodic_chart::charts(uv[0], domains[0], s.periodic_u)? else {
        return Ok(None);
    };
    let Some(vs) = crate::periodic_chart::charts(uv[1], domains[1], s.periodic_v)? else {
        return Ok(None);
    };
    let mut maximum = 0_f64;
    let mut branches = 0;
    for uc in &us {
        for vc in &vs {
            let (Some(ushift), Some(vshift)) = (uc.shift, vc.shift) else {
                return Ok(None);
            };
            let mut lifted = h.clone();
            for (k, shift) in [ushift, vshift].into_iter().enumerate() {
                if shift.lo != 0. || shift.hi != 0. {
                    for j in 0..lifted[k].len() {
                        lifted[k][j] = lifted[k][j].sub(shift.mul(h[2][j])?)?;
                    }
                }
            }
            for u in du..s.control_points.len() {
                for v in dv..s.control_points[0].len() {
                    if s.knots_u[u] >= s.knots_u[u + 1]
                        || s.knots_v[v] >= s.knots_v[v + 1]
                        || uc.range.hi < s.knots_u[u]
                        || uc.range.lo > s.knots_u[u + 1]
                        || vc.range.hi < s.knots_v[v]
                        || vc.range.lo > s.knots_v[v + 1]
                    {
                        continue;
                    }
                    branches += 1;
                    if branches > 64 {
                        return Ok(None);
                    }
                    // Bound each chart's polynomial extension over the whole cell.
                    // It therefore covers its actual subset even at a seam/knot. All
                    // possible charts must pass; no branch is selected heuristically.
                    let Some(upper) = bound(
                        curve.clone(),
                        lifted.clone(),
                        surface_net(s, [u, v])?,
                        [du, dv],
                        [
                            [s.knots_u[u], s.knots_u[u + 1]],
                            [s.knots_v[v], s.knots_v[v + 1]],
                        ],
                    )?
                    else {
                        return Ok(None);
                    };
                    maximum = maximum.max(upper);
                }
            }
        }
    }
    Ok((branches > 0).then_some(maximum))
}

fn lift_coefficients(
    h: &[Poly],
    net: &[Vec<[Interval; 4]>],
    degrees: [usize; 2],
    domains: [[f64; 2]; 2],
) -> Result<Vec<Poly>> {
    let [du, dv] = degrees;
    let degree = (h[0].len() - 1) * (du + dv);
    let mut basis = Vec::new();
    for (k, n) in [(0, du), (1, dv)] {
        let a = Interval::point(domains[k][0]);
        let width = Interval::point(domains[k][1]).sub(a)?;
        let u = h[k]
            .iter()
            .zip(&h[2])
            .map(|(&x, &w)| x.sub(a.mul(w)?)?.div(width))
            .collect::<Result<Poly>>()?;
        let complement = h[2]
            .iter()
            .zip(&u)
            .map(|(&w, &x)| w.sub(x))
            .collect::<Result<Poly>>()?;
        let up = powers(&u, n)?;
        let down = powers(&complement, n)?;
        let mut row = Vec::new();
        for i in 0..=n {
            let factor = Interval::point(choose(n, i));
            row.push(
                mul(&up[i], &down[n - i])?
                    .into_iter()
                    .map(|x| x.mul(factor))
                    .collect::<Result<Poly>>()?,
            );
        }
        basis.push(row);
    }
    // Homogeneous surface evaluated at U/W,V/W. The common W^(du+dv)
    // cancels between numerator and denominator, including rational pcurves.
    let mut lift = vec![vec![Interval::point(0.); degree + 1]; 4];
    for i in 0..=du {
        for j in 0..=dv {
            let coefficient = mul(&basis[0][i], &basis[1][j])?;
            for k in 0..4 {
                let cp = net[i][j][k];
                for (out, &b) in lift[k].iter_mut().zip(&coefficient) {
                    *out = out.add(b.mul(cp)?)?;
                }
            }
        }
    }
    Ok(lift)
}

/// Rounded rational Bernstein trace candidate for one Bezier surface chart.
/// Admission still requires the independent full-interval agreement verifier.
pub(crate) fn trace_candidate(p: &Curve, s: &Surface) -> Result<Option<Curve>> {
    p.validate()?;
    s.validate()?;
    if s.periodic_u || s.periodic_v {
        // Candidate-only view of the same natural-domain basis. Admission
        // still checks the complete original periodic support in lift().
        let mut chart=s.clone();chart.periodic_u=false;chart.periodic_v=false;
        return trace_candidate(p,&chart);
    }
    if p.control_points.len()>p.degree+1 && !p.periodic {
        return multi_trace::piecewise(p,s);
    }
    if p.degree>1 || p.weights.iter().any(|w|*w!=p.weights[0])
        || s.weights.iter().flatten().any(|w|*w!=s.weights[0][0]) {
        if let Some(c)=cell_trace::candidate(p,s)? {return Ok(Some(c));}
    }
    let (du, dv) = (s.degree_u, s.degree_v);
    if !bezier(&s.knots_u,du,s.control_points.len()) || !bezier(&s.knots_v,dv,s.control_points[0].len()) {
        return multi_trace::trace(p,s);
    }

    if p.control_points[0].len() != 2
        || p.degree != 1
        || p.control_points.len() != 2
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        // Match the existing full-original verifier's source-curve degree
        // range. Cross products stay at degree <=32, within exact choose<=48.
        || du + dv > 16
        || !bezier(&p.knots, p.degree, p.control_points.len())
        || !bezier(&s.knots_u, du, s.control_points.len())
        || !bezier(&s.knots_v, dv, s.control_points[0].len())
    {
        return Ok(None);
    }
    if p.weights[0]==p.weights[1] && s.weights.iter().flatten().all(|&w|w==s.weights[0][0]) {
        return multi_trace::trace(p,s);
    }
    let domains = [
        [s.knots_u[du], s.knots_u[du + 1]],
        [s.knots_v[dv], s.knots_v[dv + 1]],
    ];
    if p.control_points
        .iter()
        .any(|cp| (0..2).any(|k| cp[k] < domains[k][0] || cp[k] > domains[k][1]))
    {
        return Ok(None);
    }
    let net = s
        .control_points
        .iter()
        .zip(&s.weights)
        .map(|(row, weights)| {
            row.iter()
                .zip(weights)
                .map(|(p, &w)| {
                    let w = Interval::point(w);
                    Ok([
                        Interval::point(p[0]).mul(w)?,
                        Interval::point(p[1]).mul(w)?,
                        Interval::point(p[2]).mul(w)?,
                        w,
                    ])
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let lift = lift_coefficients(&homogeneous(p, false)?, &net, [du, dv], domains)?;
    let middle = |i: Interval| i.lo * 0.5 + i.hi * 0.5;
    let weights = lift[3].iter().map(|&w| middle(w)).collect::<Vec<_>>();
    if weights.iter().any(|&w| !w.is_finite() || w <= 0.) {
        return Ok(None);
    }
    let points = (0..weights.len())
        .map(|i| (0..3).map(|k| middle(lift[k][i]) / weights[i]).collect())
        .collect();
    let degree = du + dv;
    let c = Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    };
    c.validate()?;
    Ok(Some(c))
}

#[cfg(test)]
mod coefficient_plan_regression {
    use super::*;
fn original_surface_net_on(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
) -> Result<Vec<Vec<[Interval; 4]>>> {
    let [iu, iv] = span;
    let (p, q) = (s.degree_u, s.degree_v);
    let mut controls = Vec::new();
    let mut weights = [f64::INFINITY, 0_f64];
    for i in iu - p..=iu {
        let mut row = Vec::new();
        for j in iv - q..=iv {
            let w = Interval::point(s.weights[i][j]);
            weights[0] = weights[0].min(w.lo);
            weights[1] = weights[1].max(w.hi);
            let cp = &s.control_points[i][j];
            row.push([
                Interval::point(cp[0]).mul(w)?,
                Interval::point(cp[1]).mul(w)?,
                Interval::point(cp[2]).mul(w)?,
                w,
            ]);
        }
        controls.push(row);
    }
    let mut out = vec![vec![[Interval::point(0.); 4]; q + 1]; p + 1];
    for v in 0..=q {
        let vs = (0..q)
            .map(|k| domain[1][usize::from(k >= q - v)])
            .collect::<Vec<_>>();
        let rows = controls
            .iter()
            .map(|r| {
                crate::surface_distance::interpolate(r.clone(), q, &s.knots_v, iv, &vs, weights)
            })
            .collect::<Result<Vec<_>>>()?;
        for u in 0..=p {
            let us = (0..p)
                .map(|k| domain[0][usize::from(k >= p - u)])
                .collect::<Vec<_>>();
            out[u][v] = crate::surface_distance::interpolate(
                rows.clone(),
                p,
                &s.knots_u,
                iu,
                &us,
                weights,
            )?;
        }
    }
    Ok(out)
}

    #[test]
    fn retained_surface_net_coefficient_plans_keep_original_bits() {
        for p in 0..=8 { for q in 0..=8 {
            let s = Surface {
                degree_u:p, degree_v:q,
                knots_u:(0..2*p+4).map(|i|i as f64 * 0.25 - 2.).collect(),
                knots_v:(0..2*q+4).map(|i|i as f64 * 0.125 - 1.).collect(),
                control_points:(0..p+3).map(|i|(0..q+3).map(|j|
                    vec![i as f64 * 0.125 - 0.5,j as f64 * 0.25 - 1.,(i*j) as f64 * 0.0625]).collect()).collect(),
                weights:(0..p+3).map(|i|(0..q+3).map(|j|0.5+((i+2*j)%7) as f64*0.125).collect()).collect(),
                periodic_u:false, periodic_v:false,
            };
            for iu in p..p+3 { for iv in q..q+3 {
                let span=[iu,iv];
                for fractions in [[0.,1.],[0.125,0.875],[0.5,0.5]] {
                    let domain=[(&s.knots_u,iu),(&s.knots_v,iv)].map(|(knots,index)| {
                        fractions.map(|t|knots[index]+(knots[index+1]-knots[index])*t)
                    });
                    let expected=original_surface_net_on(&s,span,domain).unwrap();
                    let actual=surface_net_on(&s,span,domain).unwrap();
                    let bits=|net:Vec<Vec<[Interval;4]>>|net.into_iter().flatten()
                        .flat_map(|point|point.map(|x|[x.lo.to_bits(),x.hi.to_bits()])).collect::<Vec<_>>();
                    assert_eq!(bits(actual),bits(expected),"degrees={p},{q}, span={span:?}, domain={domain:?}");
                }
            }}
        }}
    }
}

#[cfg(test)]
mod bernstein_section_tests {
    use super::*;
    #[test]
    fn restricted_rational_tensor_coefficients_enclose_independent_monomials() {
        let mut comparisons=0;
        for (p,q) in [(1,1),(2,13),(13,2),(13,13)] {
            let mut s=Surface{degree_u:p,degree_v:q,knots_u:[vec![2.;p+1],vec![6.;p+1]].concat(),
                knots_v:[vec![-2.;q+1],vec![2.;q+1]].concat(),control_points:vec![vec![vec![0.;3];q+1];p+1],
                weights:vec![vec![1.;q+1];p+1],periodic_u:false,periodic_v:false};
            s.weights[p][q]=2.;s.control_points[p][q]=vec![0.5,1.,-0.5];s.validate().unwrap();
            for u in [[0.,1.],[0.25,0.75],[0.,0.5],[0.5,1.],[0.5,0.5],[0.,0.],[1.,1.]] {
                for v in [[0.,1.],[0.25,0.75],[0.5,0.5],[0.,0.],[1.,1.]] {
                    let domain=[[2.+4.*u[0],2.+4.*u[1]],[-2.+4.*v[0],-2.+4.*v[1]]];
                    let net=surface_net_on_bernstein(&s,[p,q],domain).unwrap().unwrap();
                    assert_eq!(net.len(),p+1);assert!(net.iter().all(|row|row.len()==q+1));
                    for i in 0..=p {for j in 0..=q {
                        // Closed-form Bernstein coefficients of u^p v^q.
                        // All powers/products here are exactly representable dyadics.
                        let m=u[0].powi((p-i) as i32)*u[1].powi(i as i32)*v[0].powi((q-j) as i32)*v[1].powi(j as i32);
                        for (k,exact) in [m,2.*m,-m,1.+m].into_iter().enumerate() {
                            assert!(net[i][j][k].lo<=exact&&exact<=net[i][j][k].hi,"p={p},q={q},i={i},j={j},k={k},domain={domain:?}");comparisons+=1;
                        }
                    }}
                }
            }
            if p>0 {s.knots_u[p+1]=4.;assert!(surface_net_on_bernstein(&s,[p,q],[[2.,4.],[-2.,2.]]).unwrap().is_none());}
        }
        assert_eq!(comparisons,39760);
        let invalid=Surface{degree_u:0,degree_v:0,knots_u:vec![0.,1.],knots_v:vec![0.,1.],
            control_points:vec![vec![vec![0.;3]]],weights:vec![vec![1.]],periodic_u:false,periodic_v:false};
        assert!(invalid.validate().is_err());
    }
}

#[path = "curve_surface_composition/multi_trace.rs"]
mod multi_trace;
mod cell_trace;
