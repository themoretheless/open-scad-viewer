//! Candidate parameters come from exact ORIGINAL coefficients. The full
//! original identity checker independently admits the discovered point.
use super::*;
use num_traits::ToPrimitive;
fn power(net: &[Q]) -> Vec<Q> {
    let n = net.len() - 1;
    (0..=n)
        .map(|k| {
            (0..=k).fold(Q::zero(), |sum, i| {
                let term = &net[i] * Q::from_integer((choose(n, i) * choose(n - i, k - i)).into());
                if (k - i) % 2 == 0 {
                    sum + term
                } else {
                    sum - term
                }
            })
        })
        .collect()
}
fn term(net: &[Q]) -> Option<(usize, f64, f64)> {
    let p = power(net);
    let n = p.iter().rposition(|x| !x.is_zero())?;
    if n < 2 || n % 2 != 0 {
        return None;
    }
    let c = &p[n];
    let center = -&p[n - 1] / (c * Q::from_integer(n.into()));
    if center <= Q::zero() || center >= Q::from_integer(1.into()) {
        return None;
    }
    for k in 1..=n {
        if p[k] != c * Q::from_integer(choose(n, k).into()) * (-&center).pow((n - k) as i32) {
            return None;
        }
    }
    // The public specification currently stores binary64. Refuse a rounded
    // candidate rather than replacing the exact original contact with it.
    let (x, a) = (center.to_f64()?, c.to_f64()?);
    if q(x).ok()? != center || q(a).ok()? != *c {
        return None;
    }
    Some((n, x, a))
}
/// Discover an isolated even-power graph contact without caller-supplied center,
/// order or coefficients. None is unsupported, not proof of disjoint surfaces.
/// Original nonconstant/mixed residuals and nonzero offsets cannot be hidden.
pub fn discover_isolated_graph_contact(
    source: &FiberSources,
) -> crate::Result<Option<QualifiedIsolatedGraphContact>> {
    let (a, b) = (source.first(), source.second());
    if admit(a).is_err()
        || admit(b).is_err()
        || a.degree_u != b.degree_u
        || a.degree_v != b.degree_v
    {
        return Ok(None);
    }
    let mut gap = vec![vec![Q::zero(); a.degree_v + 1]; a.degree_u + 1];
    for i in 0..=a.degree_u {
        for j in 0..=a.degree_v {
            gap[i][j] = q(b.control_points[i][j][2])? - q(a.control_points[i][j][2])?;
        }
    }
    for i in 0..=a.degree_u {
        for j in 0..=a.degree_v {
            if gap[i][j] != &gap[i][0] + &gap[0][j] - &gap[0][0] {
                return Ok(None);
            }
        }
    }
    let u = term(&gap.iter().map(|row| row[0].clone()).collect::<Vec<_>>());
    let v = term(&gap[0]);
    let (Some((pu, u, cu)), Some((pv, v, cv))) = (u, v) else {
        return Ok(None);
    };
    let spec = IsolatedGraphContactSpec {
        center: [u, v],
        powers: [pu, pv],
        coefficients: [cu, cv],
    };
    match qualify_isolated_graph_contact(source, spec) {
        Ok(owner) => Ok(Some(owner)),
        Err(error) if error.code == crate::RESOURCE_LIMIT => Err(error),
        Err(_) => Ok(None),
    }
}
