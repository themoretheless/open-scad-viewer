//! iges interchange v2 algorithm: surface128.
use super::*;
pub fn surface128(entities: &BTreeMap<usize, Entity>, de: usize, scale: f64) -> Result<Surface> {
    let v = &entity(entities, de, 128)?.values;
    let ku = u(v, 1, "surface U upper index")?;
    let kv = u(v, 2, "surface V upper index")?;
    let du = u(v, 3, "surface U degree")?;
    let dv = u(v, 4, "surface V degree")?;
    let nu = ku + 1;
    let nv = kv + 1;
    let n = nu
        .checked_mul(nv)
        .ok_or_else(|| refuse("Surface size overflow"))?;
    if n == 0 || n > 16384 {
        return Err(refuse("IGES surface control net exceeds 16384"));
    }
    let ku_count = nu + du + 1;
    let kv_count = nv + dv + 1;
    let mut at = 10;
    let knots_u = (0..ku_count)
        .map(|i| f(v, at + i, "surface U knot"))
        .collect::<Result<Vec<_>>>()?;
    at += ku_count;
    let knots_v = (0..kv_count)
        .map(|i| f(v, at + i, "surface V knot"))
        .collect::<Result<Vec<_>>>()?;
    at += kv_count;
    let flat_w = (0..n)
        .map(|i| f(v, at + i, "surface weight"))
        .collect::<Result<Vec<_>>>()?;
    at += n;
    let mut flat_p = Vec::with_capacity(n);
    for i in 0..n {
        flat_p.push(vec![
            f(v, at + 3 * i, "surface X")? * scale,
            f(v, at + 3 * i + 1, "surface Y")? * scale,
            f(v, at + 3 * i + 2, "surface Z")? * scale,
        ]);
    }
    let control_points = flat_p.chunks(nv).map(|r| r.to_vec()).collect();
    let weights = flat_w.chunks(nv).map(|r| r.to_vec()).collect();
    let s = Surface {
        degree_u: du,
        degree_v: dv,
        knots_u,
        knots_v,
        control_points,
        weights,
        periodic_u: u(v, 7, "surface U periodic flag")? != 0,
        periodic_v: u(v, 8, "surface V periodic flag")? != 0,
    };
    s.validate()?;
    Ok(s)
}
