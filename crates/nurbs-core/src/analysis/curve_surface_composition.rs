//! Correlated Bernstein bound for a rational Bezier curve/surface composition.
//! Every polynomial operation uses outward interval arithmetic. Unsupported
//! degrees or knot layouts leave the general interval verifier in charge.
use crate::{Result, curve::Curve, distance_bounds::Interval, surface::Surface};
type Poly = Vec<Interval>;
fn choose(n: usize, k: usize) -> f64 {
    // n <= 48: all integer intermediates and results fit u64, and the result
    // is below 2^53, hence exactly representable as binary64.
    let mut v = 1u64;
    for i in 0..k.min(n - k) {
        v = v * (n - i) as u64 / (i + 1) as u64;
    }
    v as f64
}
fn mul(a: &Poly, b: &Poly) -> Result<Poly> {
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
    if degree > 32
        || c.degree > 16
        || du + dv > 8
        || p.degree > 8
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
fn restricted(c: &Curve, t: [f64; 2], reversed: bool, source_interval: [f64;2]) -> Result<Option<Vec<Poly>>> {
    use crate::curve_surface_agreement::mapped_range;
    let range = mapped_range(source_interval, Interval::new(t[0], t[1])?, reversed)?;
    let Some(span) = span(&c.knots, c.degree, c.control_points.len(), range) else {
        return Ok(None);
    };
    let ends = [
        mapped_range(source_interval, Interval::point(t[0]), reversed)?,
        mapped_range(source_interval, Interval::point(t[1]), reversed)?,
    ];
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
pub(crate) fn surface_net_on(
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
/// Optional bound on one normalized interval. Every intersected surface chart
/// contributes an extension bound; unsupported cases retain general coverage.
pub(crate) fn upper_cell(
    c: &Curve,
    p: &Curve,
    s: &Surface,
    reversed: bool,
    t: [f64; 2],
) -> Result<Option<f64>> {
    upper_cell_on(c,p,s,reversed,t,p.domain())
}
pub(crate) fn upper_cell_on(
    c: &Curve, p: &Curve, s: &Surface, reversed: bool,
    t: [f64;2], source_interval: [f64;2],
)->Result<Option<f64>> {
    let (du, dv) = (s.degree_u, s.degree_v);
    if p.degree.saturating_mul(du.saturating_add(dv)) > 32
        || c.degree > 16
        || du + dv > 8
        || p.degree > 8
    {
        return Ok(None);
    }
    let Some(curve) = restricted(c, t, reversed, c.domain())? else {
        return Ok(None);
    };
    let Some(h) = restricted(p, t, false, source_interval)? else {
        return Ok(None);
    };
    let range = crate::curve_surface_agreement::mapped_range(source_interval, Interval::new(t[0], t[1])?, false)?;
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
    let (du, dv) = (s.degree_u, s.degree_v);
    if p.control_points[0].len() != 2
        || p.degree != 1
        || p.control_points.len() != 2
        || p.periodic
        || s.periodic_u
        || s.periodic_v
        || du + dv > 8
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
