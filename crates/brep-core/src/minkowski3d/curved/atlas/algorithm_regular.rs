//! atlas algorithm: regular.
use super::*;
pub fn regular(p: &Patch, n: &P, work: &mut Work) -> Result<bool> {
    let c = bernstein::corners(&p.h);
    let (e, f) = (sub(&c[1], &c[0]), sub(&c[2], &c[0]));
    let det = dot(n, &cross(&e, &f));
    if det <= Q::zero() {
        return Ok(false);
    }
    // Both duals share the positive factor det. Keeping them unnormalized
    // preserves every sign/interval criterion and avoids a large denominator.
    let a = cross(&f, n);
    let b = cross(n, &e);
    let au = scalar_dot(&p.derivatives[0], &a);
    let av = scalar_dot(&p.derivatives[1], &a);
    let bu = scalar_dot(&p.derivatives[0], &b);
    let bv = scalar_dot(&p.derivatives[1], &b);
    // For J_t=(1-t)J+tI, det J_t=(1-t)^2 det J+t(1-t)tr J+t².
    // Either exact positive diagonal dominance or positive determinant and
    // trace therefore prove the complete time homotopy, including skew charts.
    if positive_sum(&au, &av, 1)
        && positive_sum(&au, &av, -1)
        && positive_sum(&bv, &bu, 1)
        && positive_sum(&bv, &bu, -1)
    {
        return Ok(true);
    }
    if !positive_sum(&au, &bv, 1) && !p.h.iter().flatten().any(|p| p[3].is_zero()) {
        return Ok(false);
    }
    let range = |p: &Scalar| {
        [
            p.iter().flatten().min().unwrap().clone(),
            p.iter().flatten().max().unwrap().clone(),
        ]
    };
    let times = |a: [Q; 2], b: [Q; 2]| {
        let products = [&a[0] * &b[0], &a[0] * &b[1], &a[1] * &b[0], &a[1] * &b[1]];
        [
            products.iter().min().unwrap().clone(),
            products.iter().max().unwrap().clone(),
        ]
    };
    if times(range(&au), range(&bv))[0] > times(range(&av), range(&bu))[1] {
        return Ok(true);
    }
    if 2 * (au.len() - 1) > 33 {
        return Ok(false);
    }
    let det = bernstein::difference(
        &bernstein::product(&au, &bv, work)?,
        &bernstein::product(&av, &bu, work)?,
    );
    if p.h.iter().flatten().any(|p| p[3].is_zero()) {
        // At a certified removable homogeneous vertex, strict coefficient
        // positivity is impossible. Positivity in the open triangle plus the
        // separately proved Jordan boundary still gives projected degree one.
        let trace = au
            .iter()
            .zip(&bv)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| a + b))
            .collect::<Vec<_>>();
        if !(trace.iter().all(|q| *q >= Q::zero())
            && det.iter().flatten().all(|q| *q >= Q::zero())
            && det.iter().flatten().any(|q| *q > Q::zero()))
        {
            return Ok(false);
        }
        // Prove the boundary of each singular chart, including internal star
        // edges. Interior positivity alone must not authorize an edge collapse.
        let edges = [
            bernstein::edges(&p.motion[0]),
            bernstein::edges(&p.motion[1]),
        ];
        let curves = (0..3)
            .map(|i| [edges[0][i].clone(), edges[1][i].clone()])
            .collect::<Vec<_>>();
        for curve in &curves {
            if !monotone(curve, n, work)? {
                return Ok(false);
            }
        }
        for i in 0..3 {
            for j in i + 1..3 {
                let shared = if (i + 1) % 3 == j { &c[j] } else { &c[i] };
                if !curve_pair(&curves[i], &curves[j], Some(shared), n, work)? {
                    return Ok(false);
                }
            }
        }
        return Ok(true);
    }
    Ok(det.iter().flatten().all(|q| *q > Q::zero()))
}
