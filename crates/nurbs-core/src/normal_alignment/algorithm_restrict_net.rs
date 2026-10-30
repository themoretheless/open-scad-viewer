//! normal alignment algorithm: restrict_net.
use super::*;
pub fn restrict_net(
    net: &Net,
    degrees: [usize; 2],
    span: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
) -> Result<Net> {
    let [p, q] = degrees;
    let ku = [vec![span[0][0]; p + 1], vec![span[0][1]; p + 1]].concat();
    let kv = [vec![span[1][0]; q + 1], vec![span[1][1]; q + 1]].concat();
    let weights = [
        net.iter()
            .flatten()
            .map(|h| h[3].lo)
            .fold(f64::INFINITY, f64::min),
        net.iter()
            .flatten()
            .map(|h| h[3].hi)
            .fold(f64::NEG_INFINITY, f64::max),
    ];
    let mut result = vec![vec![[I::point(0.); 4]; q + 1]; p + 1];
    for v in 0..=q {
        let vs = (0..q)
            .map(|k| domain[1][usize::from(k >= q - v)])
            .collect::<Vec<_>>();
        let rows = net
            .iter()
            .map(|row| crate::surface_distance::interpolate(row.clone(), q, &kv, q, &vs, weights))
            .collect::<Result<Vec<_>>>()?;
        for u in 0..=p {
            let us = (0..p)
                .map(|k| domain[0][usize::from(k >= p - u)])
                .collect::<Vec<_>>();
            let h = crate::surface_distance::interpolate(rows.clone(), p, &ku, p, &us, weights)?;
            result[u][v] = h;
        }
    }
    Ok(result)
}
