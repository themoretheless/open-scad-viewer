//! Affine control-data transforms shared by native callers and viewer adapters.
use crate::{Result, check, curve::Curve, surface::Surface};
pub type Matrix = [[f64; 4]; 4];
fn validate(matrix: &Matrix) -> Result<()> {
    check(
        matrix.iter().flatten().all(|x| x.is_finite()),
        "Affine matrix must be finite",
    )?;
    check(matrix[3] == [0., 0., 0., 1.], "Transform must be affine.")
}
fn point(p: &[f64], matrix: &Matrix) -> Result<Vec<f64>> {
    check(p.len() == 3, "Affine transform requires 3D controls.")?;
    let q: Vec<f64> = (0..3)
        .map(|i| {
            matrix[i][3]
                + p.iter()
                    .enumerate()
                    .fold(0., |sum, (j, v)| sum + matrix[i][j] * v)
        })
        .collect();
    check(
        q.iter().all(|v| v.is_finite() && v.abs() <= 1e6),
        "Transformed coordinates exceed numeric limits.",
    )?;
    Ok(q)
}
/// Preserve weights, degrees, knots and periodic encoding. Singular affine maps
/// are permitted; no regularity or continuous rounding certificate is implied.
pub fn curve(source: &Curve, matrix: &Matrix) -> Result<Curve> {
    validate(matrix)?;
    source.validate()?;
    let mut result = source.clone();
    result.control_points = source
        .control_points
        .iter()
        .map(|p| point(p, matrix))
        .collect::<Result<_>>()?;
    result.validate()?;
    Ok(result)
}
pub fn surface(source: &Surface, matrix: &Matrix) -> Result<Surface> {
    validate(matrix)?;
    source.validate()?;
    let mut result = source.clone();
    result.control_points = source
        .control_points
        .iter()
        .map(|row| row.iter().map(|p| point(p, matrix)).collect::<Result<_>>())
        .collect::<Result<_>>()?;
    result.validate()?;
    Ok(result)
}
pub fn patches(sources: &[Surface], matrix: &Matrix) -> Result<Vec<Surface>> {
    check(
        !sources.is_empty() && sources.len() <= 2048,
        "Affine patch count must be 1..2048",
    )?;
    sources.iter().map(|s| surface(s, matrix)).collect()
}
