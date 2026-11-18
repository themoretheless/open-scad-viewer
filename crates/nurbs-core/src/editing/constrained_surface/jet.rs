//! Scalar linear equations in original first/second rational surface jets.
use super::{Result, Surface, basis, error};
#[derive(Clone, Debug)]
pub struct JetEquation {
    /// Coefficients of (Su,Sv), each a Cartesian dot product.
    pub first: [[f64; 3]; 2],
    /// Coefficients of (Suu,Suv,Svv), each a Cartesian dot product.
    pub second: [[f64; 3]; 3],
    pub value: f64,
}
fn second(e: &JetEquation) -> bool {
    e.second.iter().flatten().any(|x| *x != 0.)
}
pub(super) fn row(s: &Surface, uv: [f64; 2], e: &JetEquation) -> Result<Vec<f64>> {
    if !e.value.is_finite()
        || e.first
            .iter()
            .chain(&e.second)
            .flatten()
            .any(|x| !x.is_finite())
    {
        return Err(error(
            "Scalar jet equation needs finite original coefficients",
        ));
    }
    let n = s.control_points.len() * s.control_points[0].len();
    let jets = coefficient_jets(s, uv, second(e))?;
    let mut row = vec![0.; 3 * n];
    for (i, r) in jets.iter().enumerate() {
        for k in 0..3 {
            row[k * n + i] = (0..2).map(|a| e.first[a][k] * r[a + 1]).sum::<f64>()
                + (0..3).map(|a| e.second[a][k] * r[a + 3]).sum::<f64>();
        }
    }
    Ok(row)
}
pub(super) fn coefficient_jets(
    s: &Surface,
    uv: [f64; 2],
    second_order: bool,
) -> Result<Vec<[f64; 6]>> {
    let (nu, nv) = (s.control_points.len(), s.control_points[0].len());
    let u = basis(s.degree_u, &s.knots_u, nu, uv[0], s.periodic_u)?;
    let v = basis(s.degree_v, &s.knots_v, nv, uv[1], s.periodic_v)?;
    if u.continuity.unwrap_or(2) < if second_order { 2 } else { 1 }
        || v.continuity.unwrap_or(2) < if second_order { 2 } else { 1 }
    {
        return Err(error(
            "Scalar jet equation has an unresolved original knot jet",
        ));
    }
    let mut poles = Vec::new();
    let mut weight = [0.; 6];
    for i in 0..nu {
        for j in 0..nv {
            let w = s.weights[i][j];
            let n = [
                u.basis[i] * v.basis[j] * w,
                u.d1[i] * v.basis[j] * w,
                u.basis[i] * v.d1[j] * w,
                u.d2[i] * v.basis[j] * w,
                u.d1[i] * v.d1[j] * w,
                u.basis[i] * v.d2[j] * w,
            ];
            for k in 0..6 {
                weight[k] += n[k];
            }
            poles.push(n);
        }
    }
    if weight[0] <= 0. || weight.iter().any(|w| !w.is_finite()) {
        return Err(error("Unresolved scalar-jet denominator"));
    }
    let mut output = Vec::new();
    for n in &poles {
        let mut r = [0.; 6];
        r[0] = n[0] / weight[0];
        r[1] = (n[1] - weight[1] * r[0]) / weight[0];
        r[2] = (n[2] - weight[2] * r[0]) / weight[0];
        r[3] = (n[3] - weight[3] * r[0] - 2. * weight[1] * r[1]) / weight[0];
        r[4] = (n[4] - weight[4] * r[0] - weight[1] * r[2] - weight[2] * r[1]) / weight[0];
        r[5] = (n[5] - weight[5] * r[0] - 2. * weight[2] * r[2]) / weight[0];
        output.push(r);
    }
    Ok(output)
}
pub(super) fn value(s: &Surface, uv: [f64; 2], e: &JetEquation) -> Result<f64> {
    let at = s.evaluate(uv[0], uv[1])?;
    let (u, v) = at
        .first_derivatives()
        .ok_or_else(|| error("Actual scalar first jet unresolved"))?;
    let mut value = (0..3)
        .map(|k| e.first[0][k] * u[k] + e.first[1][k] * v[k])
        .sum::<f64>();
    if second(e) {
        let (uu, uv, vv) = at
            .second_derivatives()
            .ok_or_else(|| error("Actual scalar second jet unresolved"))?;
        value += (0..3)
            .map(|k| e.second[0][k] * uu[k] + e.second[1][k] * uv[k] + e.second[2][k] * vv[k])
            .sum::<f64>();
    }
    if !value.is_finite() {
        return Err(error("Actual scalar jet overflow"));
    }
    Ok(value)
}
