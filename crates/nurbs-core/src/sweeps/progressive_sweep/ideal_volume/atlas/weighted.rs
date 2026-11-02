//! Whole interval-matrix invertibility from a verified positive row scaling.
use super::*;
/// R and the positive weights are proposals only. Every A in `columns` must
/// satisfy |I-R*A| d < d, verified by outward arithmetic for every row.
/// Then ||I-R*A||_(infinity,d)<1 for ALL such matrices, including averaged
/// source Jacobians. A large acyclic coupling can be harmless in this norm.
pub(super) fn certify(columns: [V3; 3], r: [[f64; 3]; 3]) -> Result<bool> {
    if r.iter().flatten().any(|x| !x.is_finite()) {
        return Ok(false);
    }
    match checked(columns, r) {
        Err(e) if e.code == "NURBS_NUMERIC_ERROR" => Ok(false),
        result => result,
    }
}
fn checked(columns: [V3; 3], r: [[f64; 3]; 3]) -> Result<bool> {
    let mut m = [[0.; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            let mut product = I::point(0.);
            for k in 0..3 {
                product = product.add(I::point(r[i][k]).mul(columns[j][k])?)?;
            }
            let b = I::point(if i == j { 1. } else { 0. }).sub(product)?;
            m[i][j] = b.lo.abs().max(b.hi.abs());
        }
    }
    // Proposal: (I-M)d=1. Its floating inverse or determinant is NEVER proof.
    let proposal = std::array::from_fn(|j| {
        std::array::from_fn(|i| I::point((if i == j { 1. } else { 0. }) - m[i][j]))
    });
    let Some(inv) = inverse_proposal(proposal) else {
        return Ok(false);
    };
    let mut d = inv.map(|row| row.into_iter().sum::<f64>());
    if d.iter().any(|x| !x.is_finite() || *x <= 0.) {
        return Ok(false);
    }
    let max = d.iter().copied().fold(0_f64, f64::max);
    for x in &mut d {
        *x /= max;
    }
    if d.iter().any(|x| !x.is_finite() || *x <= 0.) {
        return Ok(false);
    }
    for i in 0..3 {
        let mut sum = I::point(0.);
        for j in 0..3 {
            sum = sum.add(I::point(m[i][j]).mul(I::point(d[j]))?)?;
        }
        if sum.hi >= d[i] {
            return Ok(false);
        }
    }
    Ok(true)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn positive_scaling_proves_large_triangular_coupling_but_not_a_focal_cycle() {
        let z = I::point(0.);
        let one = I::point(1.);
        let triangular = [
            [one, z, z],
            [I::new(-32., 32.).unwrap(), one, z],
            [z, I::new(-16., 16.).unwrap(), one],
        ];
        let identity = [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        assert!(certify(triangular, identity).unwrap());
        assert!(!frobenius_invertible(triangular, identity).unwrap());
        let cycle = [
            [one, I::new(-0.0625, 0.0625).unwrap(), z],
            [I::new(-32., 32.).unwrap(), one, z],
            [z, z, one],
        ];
        assert!(!certify(cycle, identity).unwrap());
        let singular = [[one, z, z], [z, one, z], [z, z, z]];
        assert!(!certify(singular, identity).unwrap());
    }
    #[test]
    fn dense_whole_interval_matrices_pass_without_determinant_or_frobenius_credit() {
        let columns = [
            [
                [-1.0696, -0.5944],
                [0.49199999999999977, 1.0999999999999999],
                [-1.468, -1.468],
            ],
            [
                [-1.212, -1.212],
                [-0.049199999999999855, 0.47320000000000023],
                [-0.6568, -0.027200000000000057],
            ],
            [
                [-1.724, -1.724],
                [1.5779999999999998, 1.5779999999999998],
                [-0.984, -0.984],
            ],
        ]
        .map(|col| col.map(|x| I::new(x[0], x[1]).unwrap()));
        let det = iv::dot_tight(columns[0], iv::cross(columns[1], columns[2]).unwrap()).unwrap();
        assert!(det.lo <= 0. && det.hi >= 0.);
        let a = scaled(columns, coordinate_scale(columns).unwrap()).unwrap();
        let r = inverse_proposal(a).unwrap();
        assert!(!frobenius_invertible(a, r).unwrap());
        assert!(certify(a, r).unwrap());
        assert!(regular(columns, [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]]).unwrap());
        assert!(!certify(a, [[0.; 3]; 3]).unwrap());
        let mut bad = r;
        bad[0][0] = f64::NAN;
        assert!(!certify(a, bad).unwrap());
    }
}

#[cfg(test)]
mod source_tests;
