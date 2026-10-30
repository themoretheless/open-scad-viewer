//! iges interchange v2 algorithm: curve126.
use super::*;
pub fn curve126(
    entities: &BTreeMap<usize, Entity>,
    de: usize,
    dim: usize,
    scale: f64,
) -> Result<Curve> {
    let v = &entity(entities, de, 126)?.values;
    let k = u(v, 1, "curve upper index")?;
    let degree = u(v, 2, "curve degree")?;
    let n = k + 1;
    let knot_count = n + degree + 1;
    if n == 0 || n > 4096 || v.len() < 8 + knot_count + n + 3 * n {
        return Err(refuse("Malformed IGES rational B-spline curve"));
    }
    let mut knots = (0..knot_count)
        .map(|i| f(v, 7 + i, "curve knot"))
        .collect::<Result<Vec<_>>>()?;
    let weights = (0..n)
        .map(|i| f(v, 7 + knot_count + i, "curve weight"))
        .collect::<Result<Vec<_>>>()?;
    let cp_at = 7 + knot_count + n;
    let mut cps = Vec::with_capacity(n);
    for i in 0..n {
        let mut p = vec![
            f(v, cp_at + 3 * i, "curve X")?,
            f(v, cp_at + 3 * i + 1, "curve Y")?,
        ];
        if dim == 3 {
            p.push(f(v, cp_at + 3 * i + 2, "curve Z")?);
            for x in &mut p {
                *x *= scale;
            }
        }
        cps.push(p);
    }
    let domain_at = cp_at + 3 * n;
    let a = f(v, domain_at, "curve domain start")?;
    let b = f(v, domain_at + 1, "curve domain end")?;
    if !a.is_finite() || !b.is_finite() || a >= b {
        return Err(refuse("IGES curve domain is not increasing"));
    }
    let old = [knots[degree], knots[knots.len() - degree - 1]];
    if (old[0] - a).abs() > 1e-10 || (old[1] - b).abs() > 1e-10 {
        let factor = (b - a) / (old[1] - old[0]);
        for x in &mut knots {
            *x = a + (*x - old[0]) * factor;
        }
    }
    let c = Curve {
        degree,
        knots,
        control_points: cps,
        weights,
        periodic: u(v, 5, "curve periodic flag")? != 0,
    };
    c.validate()?;
    Ok(c)
}
