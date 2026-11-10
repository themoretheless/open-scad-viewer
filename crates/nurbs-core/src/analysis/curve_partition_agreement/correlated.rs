//! Common-cell homogeneous Bernstein residual A_xyz B_w - B_xyz A_w.
//! Both original definitions are restricted outward; no rounded proposal is
//! substituted for source data, and independent weight gauges cancel exactly.
use super::*;
pub(super) fn upper(a: &Curve, b: &Curve, domain: [f64; 2]) -> Result<Option<f64>> {
    let sa = span(a, domain);
    let sb = span(b, domain);
    upper_between(
        a,
        sa,
        [I::point(domain[0]), I::point(domain[1])],
        b,
        sb,
        [I::point(domain[0]), I::point(domain[1])],
    )
}
pub(crate) fn upper_between(
    a: &Curve,
    sa: usize,
    ad: [I; 2],
    b: &Curve,
    sb: usize,
    bd: [I; 2],
) -> Result<Option<f64>> {
    let dim = a.control_points[0].len();
    let ac = crate::curve_distance::restricted_controls_between(a, sa, ad)?;
    let mut bc = crate::curve_distance::restricted_controls_between(b, sb, bd)?;
    let oa = &a.control_points[sa - a.degree];
    let ob = &b.control_points[sb - b.degree];
    for p in &mut bc {
        for k in 0..dim {
            if oa[k] != ob[k] {
                p[k] = p[k].add(I::point(ob[k]).sub(I::point(oa[k]))?.mul(p[dim])?)?;
            }
        }
    }
    let aw = ac.iter().map(|p| p[dim]).collect::<Vec<_>>();
    let bw = bc.iter().map(|p| p[dim]).collect::<Vec<_>>();
    let weight = crate::curve_surface_composition::mul(&aw, &bw)?;
    let lower = weight.iter().map(|v| v.lo).fold(f64::INFINITY, f64::min);
    if lower <= 0. {
        return Ok(None);
    }
    let mut total = I::point(0.);
    for k in 0..dim {
        let x = ac.iter().map(|p| p[k]).collect::<Vec<_>>();
        let y = bc.iter().map(|p| p[k]).collect::<Vec<_>>();
        let x = crate::curve_surface_composition::mul(&x, &bw)?;
        let y = crate::curve_surface_composition::mul(&y, &aw)?;
        let mut max = 0f64;
        for (x, y) in x.into_iter().zip(y) {
            let r = x.sub(y)?;
            max = max.max(r.lo.abs().max(r.hi.abs()));
        }
        total = total.add(I::point(max).div(I::point(lower))?)?;
    }
    Ok(Some(total.hi))
}
