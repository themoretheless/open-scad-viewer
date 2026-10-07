//! Incremental measurements of affine-transformed triangle geometry.
use crate::{Result, check};

/// Compensated area sum retained across cooperatively scheduled batches.
#[derive(Clone, Copy, Debug, Default)]
pub struct SurfaceArea {
    pub sum: f64,
    pub correction: f64,
}
impl SurfaceArea {
    /// Translation cancels before arithmetic; even huge finite translations
    /// therefore preserve the precision of local triangle edges.
    pub fn add_triangles(&mut self, points: &[f64], matrix: [[f64; 4]; 4]) -> Result<()> {
        check(points.len().is_multiple_of(9), "invalid-compute-input")?;
        check(
            matrix.iter().flatten().all(|x| x.is_finite()) && matrix[3] == [0., 0., 0., 1.],
            "invalid-compute-input",
        )?;
        check(
            self.sum.is_finite() && self.correction.is_finite(),
            "invalid-compute-input",
        )?;
        for p in points.chunks_exact(9) {
            let u: [f64; 3] = std::array::from_fn(|i| p[3 + i] - p[i]);
            let v: [f64; 3] = std::array::from_fn(|i| p[6 + i] - p[i]);
            let transform = |v: [f64; 3]| -> [f64; 3] {
                std::array::from_fn(|i| {
                    matrix[i][0] * v[0] + matrix[i][1] * v[1] + matrix[i][2] * v[2]
                })
            };
            let [x, y, z] = transform(u);
            let [a, b, c] = transform(v);
            let area = (y * c - z * b).hypot(z * a - x * c).hypot(x * b - y * a) / 2.;
            check(area.is_finite(), "invalid-compute-input")?;
            let corrected = area - self.correction;
            let next = self.sum + corrected;
            self.correction = (next - self.sum) - corrected;
            self.sum = next;
        }
        check(self.sum.is_finite(), "invalid-compute-input")
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_area_is_independent_of_translation_and_batch_boundaries() {
        let p = [0., 0., 0., 2., 0., 0., 0., 3., 0.];
        let m = [
            [-2., 0., 0., 1e200],
            [0., 3., 0., -1e200],
            [0., 0., 4., 1e200],
            [0., 0., 0., 1.],
        ];
        let mut batched = SurfaceArea::default();
        batched.add_triangles(&p, m).unwrap();
        batched.add_triangles(&p, m).unwrap();
        let mut full = SurfaceArea::default();
        full.add_triangles(&p.repeat(2), m).unwrap();
        assert_eq!(full.sum, 36.);
        assert_eq!(batched.sum, full.sum);
        assert_eq!(batched.correction, full.correction);
    }
}
