//! normal alignment algorithm: jet_net_on.
use super::*;
pub fn jet_net_on(mut net: Net, degrees:[usize;2], span:[[f64;2];2],
    domain:[[f64;2];2], origin:&[f64]) -> Result<JetBounds> {
    for row in &mut net {
        for h in row {
            for k in 0..3 {
                h[k] = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
            }
        }
    }
    let mut h = [[I::point(0.); 4]; 6];
    for (i, order) in [[0, 0], [1, 0], [0, 1], [2, 0], [1, 1], [0, 2]]
        .into_iter()
        .enumerate()
    {
        h[i] = homogeneous_jet_hull(&net, degrees, span, domain, order)?;
    }
    let mut out = JetBounds {
        point: [I::point(0.); 3],
        first: [[I::point(0.); 3]; 2],
        second: [[I::point(0.); 3]; 3],
    };
    for k in 0..3 {
        let relative = h[0][k].div(h[0][3])?;
        out.point[k] = relative.add(I::point(origin[k]))?;
        for axis in 0..2 {
            out.first[axis][k] = h[axis + 1][k]
                .sub(relative.mul(h[axis + 1][3])?)?
                .div(h[0][3])?;
        }
        for (slot, a, b) in [(0, 0, 0), (1, 0, 1), (2, 1, 1)] {
            out.second[slot][k] = h[slot + 3][k]
                .sub(out.first[a][k].mul(h[b + 1][3])?)?
                .sub(out.first[b][k].mul(h[a + 1][3])?)?
                .sub(relative.mul(h[slot + 3][3])?)?
                .div(h[0][3])?;
        }
    }
    Ok(out)
}
