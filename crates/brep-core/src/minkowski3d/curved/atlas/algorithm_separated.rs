//! atlas algorithm: separated.
use super::*;
/// A deliberately small set of exact separating axes. Failure only requests
/// subdivision; no axis or floating-point proximity is treated as evidence.
pub fn separated(a: &[P], b: &[P], work: &mut Work) -> Result<bool> {
    for k in 0..3 {
        if separates(
            a,
            b,
            &std::array::from_fn(|i| if i == k { Q::one() } else { Q::zero() }),
        ) {
            return Ok(true);
        }
    }
    let center = |p: &[P]| {
        std::array::from_fn(|i| p.iter().map(|p| p[i].clone()).sum::<Q>() / integer(p.len() as i64))
    };
    if separates(a, b, &sub(&center(a), &center(b))) {
        return Ok(true);
    }
    // Corner triangle face normals and crossed edge directions are cheap
    // exact witnesses for the usual near-linear chart hulls.
    if a.len() >= 3 && b.len() >= 3 {
        for p in [a, b] {
            let n = cross(&sub(&p[1], &p[0]), &sub(&p[2], &p[0]));
            if separates(a, b, &n) {
                return Ok(true);
            }
            for k in 0..3 {
                work.charge(1)?;
                if separates(a, b, &cross(&n, &sub(&p[(k + 1) % 3], &p[k]))) {
                    return Ok(true);
                }
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                work.charge(1)?;
                if separates(
                    a,
                    b,
                    &cross(&sub(&a[(i + 1) % 3], &a[i]), &sub(&b[(j + 1) % 3], &b[j])),
                ) {
                    return Ok(true);
                }
            }
        }
    }
    // Convex hull face normals are useful even for spatially rotated charts.
    for p in [a, b] {
        for i in 0..p.len().min(6) {
            for j in i + 1..p.len().min(6) {
                for k in j + 1..p.len().min(6) {
                    work.charge(1)?;
                    if separates(a, b, &cross(&sub(&p[j], &p[i]), &sub(&p[k], &p[i]))) {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
