use super::*;
pub(in crate::editing) fn degrees(channels: &[Poly]) -> (usize, usize) {
    (
        channels
            .iter()
            .flat_map(|p| p.keys())
            .map(|x| x.0)
            .max()
            .unwrap_or(0)
            .max(1),
        channels
            .iter()
            .flat_map(|p| p.keys())
            .map(|x| x.1)
            .max()
            .unwrap_or(0)
            .max(1),
    )
}
pub(in crate::editing) fn bernstein(p: &Poly, pu: usize, pv: usize, w: &mut Work) -> PResult<Vec<Q>> {
    let mut temp = vec![vec![Q::zero(); pv + 1]; pu + 1];
    for j in 0..=pv {
        let column = (0..=pu)
            .map(|i| p.get(&(i, j)).cloned().unwrap_or_else(Q::zero))
            .collect::<Vec<_>>();
        let b = uni::bernstein(&column, pu, w)?;
        for i in 0..=pu {
            temp[i][j] = b[i].clone();
        }
    }
    let mut out = Vec::new();
    for row in temp {
        out.extend(uni::bernstein(&row, pv, w)?);
    }
    Ok(out)
}
pub(super) fn store(channels: &[Poly], domains: [[f64; 2]; 2], w: &mut Work) -> PResult<Surface> {
    let (pu, pv) = degrees(channels);
    if pu > 25 || pv > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let b = channels
        .iter()
        .map(|p| bernstein(p, pu, pv, w))
        .collect::<PResult<Vec<_>>>()?;
    // ONE gauge over the entire flat UV net, never one gauge per row/column.
    let (points, weights) = super::super::rational_edit::store(&b, w)?;
    let out = Surface {
        degree_u: pu,
        degree_v: pv,
        knots_u: [vec![domains[0][0]; pu + 1], vec![domains[0][1]; pu + 1]].concat(),
        knots_v: [vec![domains[1][0]; pv + 1], vec![domains[1][1]; pv + 1]].concat(),
        control_points: points.chunks(pv + 1).map(|r| r.to_vec()).collect(),
        weights: weights.chunks(pv + 1).map(|r| r.to_vec()).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    if out.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(out)
}
pub(in crate::editing) fn powers(s: &Surface, w: &mut Work) -> PResult<Vec<Poly>> {
    let pu = s.degree_u;
    let pv = s.degree_v;
    let mut rows = vec![vec![Vec::new(); pu + 1]; 4];
    for i in 0..=pu {
        let mut channels = vec![Vec::new(); 4];
        for j in 0..=pv {
            let weight = w.lift(s.weights[i][j])?;
            for axis in 0..3 {
                let x = w.lift(s.control_points[i][j][axis])?;
                channels[axis].push(w.op(&x, &weight, |x, w| x * w)?);
            }
            channels[3].push(weight);
        }
        for axis in 0..4 {
            rows[axis][i] = uni::power(&channels[axis], w)?;
        }
    }
    let mut out = vec![Poly::new(); 4];
    for axis in 0..4 {
        for j in 0..=pv {
            let column = (0..=pu)
                .map(|i| rows[axis][i].get(j).cloned().unwrap_or_else(Q::zero))
                .collect::<Vec<_>>();
            let p = uni::power(&column, w)?;
            for (i, x) in p.into_iter().enumerate() {
                if !x.is_zero() {
                    out[axis].insert((i, j), x);
                }
            }
        }
    }
    Ok(out)
}
pub(super) fn prove(s: &Surface, target: &[Poly], w: &mut Work) -> PResult<()> {
    let actual = powers(s, w)?;
    for axis in 0..3 {
        if polynomial::multiply(&actual[axis], &target[3], w)?
            != polynomial::multiply(&target[axis], &actual[3], w)?
        {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    Ok(())
}
