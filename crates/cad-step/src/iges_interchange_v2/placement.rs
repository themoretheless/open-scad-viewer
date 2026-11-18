use super::*;

type Matrix = [[f64; 4]; 3];

pub(super) fn read(entities: &BTreeMap<usize, Entity>, de: usize) -> Result<Matrix> {
    let e = entity(entities, de, 124)?;
    if e.directory.form != 0 || e.directory.transform != 0 || e.values.len() != 13 {
        return Err(refuse(
            "Expected one unchained form-zero IGES placement matrix",
        ));
    }
    let mut matrix = [[0.; 4]; 3];
    for (i, row) in matrix.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = f(&e.values, 1 + 4 * i + j, "placement coefficient")?;
        }
    }
    for (i, row) in matrix.iter().enumerate() {
        for (j, other) in matrix.iter().enumerate().take(i + 1) {
            let dot: f64 = (0..3).map(|k| row[k] * other[k]).sum();
            if !dot.is_finite() || (dot - f64::from(i == j)).abs() > 1e-12 {
                return Err(refuse("IGES placement requires an orthonormal rotation"));
            }
        }
    }
    let [a, b, c] = matrix;
    let det = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if det <= 0. {
        return Err(refuse("IGES form-zero placement must preserve handedness"));
    }
    Ok(matrix)
}

pub(super) fn apply(matrix: &Matrix, point: &[f64], scale: f64, dim: usize) -> Result<Vec<f64>> {
    let xyz = matrix
        .map(|row| row[0] * point[0] + row[1] * point[1] + row[2] * point[2] + row[3] * scale);
    if !xyz.iter().all(|x| x.is_finite()) || (dim == 2 && xyz[2] != 0.) {
        return Err(refuse(
            "IGES placement exceeds the destination coordinate domain",
        ));
    }
    Ok(xyz[..dim].to_vec())
}
