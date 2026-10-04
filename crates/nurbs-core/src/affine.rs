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
#[cfg(test)]
mod tests {
    use super::*;
    const M: Matrix = [
        [-2., 0.5, 0., 3.],
        [0., 1., 1., -4.],
        [1., 0., 0.25, 2.],
        [0., 0., 0., 1.],
    ];
    #[test]
    fn affine_weighted_curve_preserves_rational_data_and_commutes_with_evaluation() {
        let source = crate::paths::bezier(
            vec![vec![0., 0., 0.], vec![2., 3., 1.], vec![4., 0., 2.]],
            Some(vec![1., 2., 1.]),
        )
        .unwrap();
        let transformed = curve(&source, &M).unwrap();
        assert_eq!(transformed.weights, source.weights);
        assert_eq!(transformed.knots, source.knots);
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let p = source.evaluate(t).unwrap().point;
            let expected = [
                -2. * p[0] + 0.5 * p[1] + 3.,
                p[1] + p[2] - 4.,
                p[0] + 0.25 * p[2] + 2.,
            ];
            let q = transformed.evaluate(t).unwrap().point;
            assert!(q.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
        }
    }
    #[test]
    fn surfaces_and_patch_sets_retain_basis_and_independent_plane_equation() {
        let line = crate::primitives::line([0.; 3], [2., 0., 0.]).unwrap();
        let source = crate::surface::extrude(&line, [0., 3., 4.]).unwrap();
        let transformed = surface(&source, &M).unwrap();
        assert_eq!(transformed.weights, source.weights);
        assert_eq!(transformed.knots_u, source.knots_u);
        for i in 0..=20 {
            for j in 0..=20 {
                let u = i as f64 / 20.;
                let v = j as f64 / 20.;
                let p = transformed.evaluate(u, v).unwrap().point;
                let expected = [-4. * u + 1.5 * v + 3., 7. * v - 4., 2. * u + v + 2.];
                assert!(p.iter().zip(expected).all(|(x, y)| (x - y).abs() < 1e-11));
            }
        }
        assert_eq!(
            patches(&[source.clone(), source], &M).unwrap(),
            vec![transformed.clone(), transformed]
        );
    }
    #[test]
    fn invalid_projective_nonfinite_dimension_and_output_range_are_refused() {
        let mut source = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        let mut m = M;
        m[3][0] = 0.1;
        assert!(curve(&source, &m).is_err());
        m = M;
        m[0][0] = f64::NAN;
        assert!(curve(&source, &m).is_err());
        m = M;
        m[0][3] = 1e6 + 1.;
        assert!(curve(&source, &m).is_err());
        source.control_points.iter_mut().for_each(|p| {
            p.pop();
        });
        assert!(curve(&source, &M).is_err());
    }
}
