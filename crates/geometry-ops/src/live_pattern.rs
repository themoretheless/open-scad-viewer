//! Affine placements for source-linked patterns. Source placement is index zero.
use crate::{Result, fail};
pub type Matrix = [[f64; 4]; 4];
#[derive(Clone, Copy, Debug)]
pub enum Pattern {
    Grid { rows: usize, columns: usize, spacing: [f64; 2] },
    Radial { count: usize, axis: usize, center: [f64; 3] },
    Mirror { axis: usize, center: [f64; 3] },
}
fn identity() -> Matrix {
    std::array::from_fn(|i| std::array::from_fn(|j| if i == j { 1. } else { 0. }))
}
pub fn placements(pattern: Pattern) -> Result<Vec<Matrix>> {
    let finite = |p: &[f64]| p.iter().all(|v| v.is_finite() && v.abs() <= 1e6);
    let result = match pattern {
        Pattern::Grid { rows, columns, spacing } => {
            let count = rows.checked_mul(columns).ok_or_else(|| fail("Pattern count overflow"))?;
            if rows == 0 || columns == 0 || !(2..=256).contains(&count) || !finite(&spacing) {
                return Err(fail("Grid needs 2–256 placements and finite spacing"));
            }
            (1..count).map(|i| {
                let mut m = identity();
                m[0][3] = (i % columns) as f64 * spacing[0];
                m[1][3] = (i / columns) as f64 * spacing[1];
                m
            }).collect()
        }
        Pattern::Radial { count, axis, center } => {
            if !(2..=256).contains(&count) || axis > 2 || !finite(&center) {
                return Err(fail("Radial pattern needs 2–256 placements and a finite pivot"));
            }
            let a = (axis + 1) % 3;
            let b = (axis + 2) % 3;
            (1..count).map(|i| {
                let (sin, cos) = (std::f64::consts::TAU * i as f64 / count as f64).sin_cos();
                let mut m = identity();
                m[a][a] = cos; m[b][b] = cos;
                m[a][b] = -sin; m[b][a] = sin;
                for k in 0..3 {
                    m[k][3] = center[k] - (0..3).map(|j| m[k][j] * center[j]).sum::<f64>();
                }
                m
            }).collect()
        }
        Pattern::Mirror { axis, center } => {
            if axis > 2 || !finite(&center) { return Err(fail("Invalid mirror plane")); }
            let mut m = identity(); m[axis][axis] = -1.; m[axis][3] = 2. * center[axis];
            vec![m]
        }
    };
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn grid_and_mirror_have_stable_source_excluding_order() {
        let p = placements(Pattern::Grid { rows: 2, columns: 3, spacing: [10., 20.] }).unwrap();
        assert_eq!(p.len(), 5); assert_eq!([p[0][0][3], p[0][1][3]], [10., 0.]);
        assert_eq!([p[2][0][3], p[2][1][3]], [0., 20.]);
        let m = placements(Pattern::Mirror { axis: 0, center: [3., 0., 0.] }).unwrap()[0];
        assert_eq!(m[0], [-1., 0., 0., 6.]);
        assert!(placements(Pattern::Grid { rows: usize::MAX, columns: 2, spacing: [1., 1.] }).is_err());
        assert!(placements(Pattern::Radial { count: 2, axis: 3, center: [0.; 3] }).is_err());
    }
    #[test]
    fn radial_keeps_pivot_fixed_and_excludes_duplicate_full_turn() {
        let center = [10., 5., 2.];
        let p = placements(Pattern::Radial { count: 4, axis: 2, center }).unwrap();
        assert_eq!(p.len(), 3);
        for m in p { for k in 0..3 {
            let q = m[k][3] + (0..3).map(|j| m[k][j] * center[j]).sum::<f64>();
            assert!((q - center[k]).abs() < 1e-12);
        }}
    }
}
