//! Column-pivoted proposal solves with retained physical correction.
use super::*;

// Column-pivoted Householder QR, not normal equations. The small diagonal
// regularizer chooses unconstrained interior modes; no geometric proof uses it.
pub(super) fn least_squares(
    a: Vec<Vec<f64>>,
    b: Vec<f64>,
    extra: &[Vec<(usize, f64)>],
) -> Result<Vec<f64>> {
    least_squares_constrained(a, b, extra, &[])
}

pub(super) fn least_squares_constrained(
    a: Vec<Vec<f64>>,
    b: Vec<f64>,
    extra: &[Vec<(usize, f64)>],
    constraints: &[(Vec<f64>, f64)],
) -> Result<Vec<f64>> {
    least_squares_metric(a, b, extra, constraints, false)
}

pub(super) fn least_squares_metric(
    mut a: Vec<Vec<f64>>,
    mut b: Vec<f64>,
    extra: &[Vec<(usize, f64)>],
    constraints: &[(Vec<f64>, f64)],
    physical_correction: bool,
) -> Result<Vec<f64>> {
    let m = a.len();
    let n = a[0].len();
    if constraints.is_empty() && b.iter().all(|&x| x == 0.) {
        return Ok(vec![0.; n]);
    }
    // Normalize columns before rank-revealing QR. Clustered knots otherwise
    // mix tiny value columns with enormous second-derivative columns.
    let column_scale: Vec<f64> = (0..n)
        .map(|j| a.iter().map(|r| r[j] * r[j]).sum::<f64>().sqrt())
        .collect();
    if column_scale.iter().any(|&s| !s.is_finite() || s == 0.) {
        return Err(refuse("Invalid Hermite column scale"));
    }
    for row in &mut a {
        for j in 0..n {
            row[j] /= column_scale[j];
        }
    }
    let mut rank = n;
    let mut order: Vec<usize> = (0..n).collect();
    for k in 0..n {
        // Accumulate every remaining column norm in one row-major pass.
        // Recomputing two strided column scans for each pivot comparison
        // made large fairing fits spend minutes on cache misses alone.
        let mut norms = vec![0.; n - k];
        for row in a.iter().skip(k) {
            for (sum, &x) in norms.iter_mut().zip(&row[k..]) {
                *sum += x * x;
            }
        }
        let col = k
            + (0..norms.len())
                .max_by(|&i, &j| norms[i].total_cmp(&norms[j]))
                .unwrap();
        let norm = norms[col - k].sqrt();
        for row in &mut a {
            row.swap(k, col);
        }
        order.swap(k, col);
        if !norm.is_finite() {
            return Err(refuse(
                "Hermite fitting system is numerically rank deficient",
            ));
        }
        if norm <= 1e-13 && (physical_correction || (extra.is_empty() && constraints.is_empty())) {
            rank = k;
            break;
        }
        let alpha = -norm.copysign(a[k][k]);
        let mut v: Vec<f64> = (k..m).map(|i| a[i][k]).collect();
        v[0] -= alpha;
        let beta = 2. / v.iter().map(|x| x * x).sum::<f64>();
        // Each column dot retains ascending row order; only independent
        // columns are interleaved. Apply the same Householder reflector in
        // contiguous rows. This is still pivoted QR, not normal equations.
        let mut dots = vec![0.; n - k - 1];
        for (i, row) in a.iter().enumerate().skip(k) {
            for (sum, &x) in dots.iter_mut().zip(&row[k + 1..]) {
                *sum += v[i - k] * x;
            }
        }
        for d in &mut dots {
            *d *= beta;
        }
        for (i, row) in a.iter_mut().enumerate().skip(k) {
            for (x, &d) in row[k + 1..].iter_mut().zip(&dots) {
                *x -= d * v[i - k];
            }
        }
        let d = beta * (k..m).map(|i| v[i - k] * b[i]).sum::<f64>();
        for i in k..m {
            b[i] -= d * v[i - k];
        }
        a[k][k] = alpha;
        for row in a.iter_mut().skip(k + 1) {
            row[k] = 0.;
        }
    }
    if rank < n && physical_correction {
        // Complete only the resolved row space; never invert numeric noise.
        // Soft fairing is superseded by bounded physical norm in this fallback.
        let base = minimum_norm::complete(&a, &b, &order, &column_scale, rank)?;
        let mut x = order
            .iter()
            .map(|&j| base[j] * column_scale[j])
            .collect::<Vec<_>>();
        correct_physical_poles(&order, &column_scale, &mut x, constraints)?;
        let mut out = vec![0.; n];
        for i in 0..n {
            out[order[i]] = x[i] / column_scale[order[i]];
        }
        if out.iter().any(|x| !x.is_finite()) {
            return Err(refuse("Nonfinite resolved-rank junction proposal"));
        }
        return Ok(out);
    }
    // Append sparse fairing rows to the completed QR factor, rather than
    // expanding thousands of five-entry rows into another dense matrix.
    // Positive original Tikhonov rows retain full rank for this path.
    for row in extra {
        let mut values = vec![0.; n];
        let mut inverse_order = vec![0; n];
        for (j, &original) in order.iter().enumerate() {
            inverse_order[original] = j;
        }
        for &(j, x) in row {
            values[inverse_order[j]] = x / column_scale[j];
        }
        let mut rhs = 0.;
        for j in 0..n {
            if values[j] == 0. {
                continue;
            }
            let radius = a[j][j].hypot(values[j]);
            if !radius.is_finite() || radius == 0. {
                return Err(refuse("Sparse fairing QR rotation is singular"));
            }
            let c = a[j][j] / radius;
            let s = values[j] / radius;
            a[j][j] = radius;
            values[j] = 0.;
            for k in j + 1..n {
                let x = a[j][k];
                let y = values[k];
                a[j][k] = c * x + s * y;
                values[k] = -s * x + c * y;
            }
            let x = b[j];
            b[j] = c * x + s * rhs;
            rhs = -s * x + c * rhs;
        }
    }
    if rank < n {
        return minimum_norm::complete(&a, &b, &order, &column_scale, rank);
    }
    let mut x = vec![0.; n];
    for i in (0..rank).rev() {
        x[i] = (b[i] - (i + 1..rank).map(|j| a[i][j] * x[j]).sum::<f64>()) / a[i][i];
    }
    if physical_correction {
        // In near-null fitting modes, preserving the QR objective can require
        // enormous pole excursions. Junction edits instead minimize the
        // change in ORIGINAL physical poles; legacy world anchors keep QR's
        // objective metric. This is a proposal, never material admission.
        correct_physical_poles(&order, &column_scale, &mut x, constraints)?;
    } else {
        constrained::correct(&a, &order, &column_scale, &mut x, constraints)?;
    }
    let mut out = vec![0.; n];
    for i in 0..n {
        out[order[i]] = x[i] / column_scale[order[i]];
    }
    if out.iter().any(|x| !x.is_finite()) {
        return Err(refuse("Hermite fit exceeded finite range"));
    }
    Ok(out)
}
