//! Candidate parameters come from exact ORIGINAL coefficients. The full
//! original identity checker independently admits the discovered point.
use super::*;
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
fn leading(poly: &[Q]) -> Option<(usize, Q, Q)> {
    let n = poly.iter().rposition(|x| !x.is_zero())?;
    if n < 2 || n % 2 != 0 {
        return None;
    }
    let c = &poly[n];
    let center = -&poly[n - 1] / (c * Q::from_integer(n.into()));
    Some((n, center, c.clone()))
}

fn evaluate(poly: &[Q], parameter: &Q) -> Q {
    poly.iter().rev().fold(Q::zero(), |value, coefficient| {
        value * parameter + coefficient
    })
}

fn specialize(rows: &[Vec<Q>], parameter: &Q) -> Vec<Q> {
    rows.iter().map(|row| evaluate(row, parameter)).collect()
}

fn candidate(gap: &[Vec<Q>]) -> Option<(ExactIsolatedGraphContactSpec, Q)> {
    let v_power: Vec<_> = gap.iter().map(|row| power(row)).collect();
    let uv: Vec<_> = (0..gap[0].len())
        .map(|j| power(&v_power.iter().map(|row| row[j].clone()).collect::<Vec<_>>()))
        .collect();
    let vu: Vec<Vec<Q>> = (0..gap.len())
        .map(|i| uv.iter().map(|row| row[i].clone()).collect())
        .collect();
    let (pu, mut u, cu) = leading(&uv[0])?;
    let (pv, mut v, cv) = leading(&vu[0])?;
    // For order > 2 the mixed term has lower degree than the coefficient
    // used to recover the center. Quadratic terms need a stationary solve
    // or specialization at the already recovered center of the other axis.
    if pu == 2 && pv == 2 {
        let cross = &uv[1][1];
        let two = Q::from_integer(2.into());
        let det = &two * &two * &cu * &cv - cross.pow(2);
        if det <= Q::zero() {
            return None;
        }
        u = (cross * &vu[0][1] - &two * &cv * &uv[0][1]) / &det;
        v = (cross * &uv[0][1] - &two * &cu * &vu[0][1]) / det;
    } else if pu == 2 {
        u = leading(&specialize(&vu, &v))?.1;
    } else if pv == 2 {
        v = leading(&specialize(&uv, &u))?.1;
    }
    if [&u, &v]
        .into_iter()
        .any(|p| p <= &Q::zero() || p >= &Q::one())
    {
        return None;
    }
    let mixed = (&gap[0][0] - &cu * u.pow(pu as i32) - &cv * v.pow(pv as i32))
        / ((-&u).pow((pu / 2) as i32) * (-&v).pow((pv / 2) as i32));
    Some((
        ExactIsolatedGraphContactSpec {
            center: [u, v],
            powers: [pu, pv],
            coefficients: [cu, cv],
        },
        mixed,
    ))
}
/// Discover an isolated even-power graph contact without caller-supplied center,
/// order or coefficients. None is unsupported, not proof of disjoint surfaces.
/// The candidate is checked coefficient by coefficient on the whole domain.
pub fn discover_isolated_graph_contact(
    source: &FiberSources,
) -> crate::Result<Option<QualifiedIsolatedGraphContact>> {
    let gap = match common_gap(source) {
        Ok(gap) => gap,
        Err(error) if error.code == crate::RESOURCE_LIMIT => return Err(error),
        Err(_) => return Ok(None),
    };
    let Some((spec, mixed)) = candidate(&gap.coefficients) else {
        return Ok(None);
    };
    match qualify_gap(source, spec, mixed, &gap) {
        Ok(owner) => Ok(Some(owner)),
        Err(error) if error.code == crate::RESOURCE_LIMIT => Err(error),
        Err(_) => Ok(None),
    }
}
