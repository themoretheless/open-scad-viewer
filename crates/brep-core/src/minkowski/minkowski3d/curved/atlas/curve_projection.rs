//! Exact projected boundary curves and monotone seam images.
use super::*;

pub(super) fn projected(p: &P, n: &P) -> P {
    let t = dot(p, n) / dot(n, n);
    std::array::from_fn(|i| &p[i] - &t * &n[i])
}

pub(super) fn curve_points(c: &Curve, n: &P) -> Vec<P> {
    c.iter()
        .flatten()
        .filter(|h| h[3] > Q::zero())
        .map(|h| projected(&sb::point(h), n))
        .collect()
}

pub(super) fn plane_separated(a: &[P], b: &[P], n: &P, work: &mut Work) -> Result<bool> {
    for p in [a, b] {
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                work.charge(1)?;
                let edge = sub(&p[j], &p[i]);
                if separates(a, b, &edge) || separates(a, b, &cross(n, &edge)) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

pub(super) fn monotone(c: &Curve, n: &P, work: &mut Work) -> Result<bool> {
    monotone_parameter(c, n, work, false)
}

pub(super) fn monotone_parameter(
    c: &Curve,
    n: &P,
    work: &mut Work,
    stationary: bool,
) -> Result<bool> {
    let h = &c[0];
    let d = h.len() - 1;
    let a = projected(&sb::point(&h[0]), n);
    let b = projected(&sb::point(h.last().unwrap()), n);
    let axis = sub(&b, &a);
    if zero(&axis) {
        return Ok(false);
    }
    let x = h
        .iter()
        .map(|p| (0..3).map(|k| &p[k] * &axis[k]).sum::<Q>())
        .collect::<Vec<_>>();
    let w = h.iter().map(|p| p[3].clone()).collect::<Vec<_>>();
    let deriv = |v: &[Q]| {
        v.windows(2)
            .map(|v| (&v[1] - &v[0]) * integer(d as i64))
            .collect::<Vec<_>>()
    };
    let product = |a: &[Q], b: &[Q], work: &mut Work| -> Result<Vec<Q>> {
        work.charge(a.len() * b.len())?;
        let product = sb::multiply(a, b);
        if product
            .iter()
            .any(|q| q.numer().bits() > 8192 || q.denom().bits() > 8192)
        {
            return Err(fail("atlas edge derivative bit budget exceeded"));
        }
        Ok(product)
    };
    let a = product(&deriv(&x), &w, work)?;
    let b = product(&x, &deriv(&w), work)?;
    // Nonnegative Bernstein coefficients with one positive coefficient give
    // a strictly positive derivative in (0,1), permitting stationary endpoints.
    Ok(if stationary {
        a.iter().zip(&b).all(|(x, y)| x >= y) && a.iter().zip(&b).any(|(x, y)| x > y)
    } else {
        a.iter().zip(&b).all(|(x, y)| x > y)
    })
}

pub(super) fn curve_pair(
    a: &Curve,
    b: &Curve,
    shared: Option<&P>,
    n: &P,
    work: &mut Work,
) -> Result<bool> {
    let (pa, pb) = (curve_points(a, n), curve_points(b, n));
    if let Some(origin) = shared {
        let origin = projected(origin, n);
        let v = pa
            .iter()
            .map(|p| sub(p, &origin))
            .chain(pb.iter().map(|p| sub(&origin, p)))
            .filter(|p| !zero(p))
            .collect::<Vec<_>>();
        // The curve endpoints and strict Bernstein positivity ensure that
        // dropping zero endpoint controls cannot hide an interior return.
        return super::super::rounding::pointed(&v, work);
    }
    plane_separated(&pa, &pb, n, work)
}

/// Both complete time homotopies have the same oriented segment image. Their
/// strictly monotone clocks give a continuous time-dependent edge gluing, which
/// extends over each disk. Projected star proofs depend on these images, not on
/// an arbitrary speed chosen along the common segment.
pub(super) fn segment_seam(a: &[sb::H], b: &[sb::H], work: &mut Work) -> Result<bool> {
    let p = sb::point(&a[0]);
    let q = sb::point(a.last().unwrap());
    let axis = sub(&q, &p);
    if zero(&axis) || sb::point(&b[0]) != p || sb::point(b.last().unwrap()) != q {
        return Ok(false);
    }
    if a.iter()
        .chain(b)
        .filter(|h| h[3] > Q::zero())
        .any(|h| !zero(&cross(&sub(&sb::point(h), &p), &axis)))
    {
        return Ok(false);
    }
    let n = (0..3)
        .map(|k| {
            cross(
                &axis,
                &std::array::from_fn(|j| if j == k { Q::one() } else { Q::zero() }),
            )
        })
        .find(|n| !zero(n))
        .unwrap();
    Ok(monotone(&[a.to_vec(), a.to_vec()], &n, work)?
        && monotone(&[b.to_vec(), b.to_vec()], &n, work)?)
}
