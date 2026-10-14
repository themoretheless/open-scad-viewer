//! Bicubic interpolation of corner positions and parametric jets.
use crate::{Result, check, surface::Surface};
type Corners = [[[f64; 3]; 2]; 2];

/// All conditions are indexed [u][v] on the normalized [0,1]^2 domain.
/// Does not certify regularity or injectivity; zero tangents are permitted.
pub fn patch(
    points: Corners,
    tangent_u: Corners,
    tangent_v: Corners,
    twist: Corners,
) -> Result<Surface> {
    check(
        [&points, &tangent_u, &tangent_v, &twist]
            .iter()
            .flat_map(|a| a.iter().flatten().flatten())
            .all(|x| x.is_finite()),
        "Hermite patch corner conditions must be finite",
    )?;
    let mut controls = vec![vec![vec![0.; 3]; 4]; 4];
    for i in 0..2 {
        for j in 0..2 {
            let a = 3 * i;
            let b = 3 * j;
            let inner_a = if i == 0 { 1 } else { 2 };
            let inner_b = if j == 0 { 1 } else { 2 };
            let su = if i == 0 { 1. } else { -1. };
            let sv = if j == 0 { 1. } else { -1. };
            for k in 0..3 {
                let p = points[i][j][k];
                let du = su * tangent_u[i][j][k] / 3.;
                let dv = sv * tangent_v[i][j][k] / 3.;
                controls[a][b][k] = p;
                controls[inner_a][b][k] = p + du;
                controls[a][inner_b][k] = p + dv;
                let base = p + du + dv;
                controls[inner_a][inner_b][k] = base + su * sv * twist[i][j][k] / 9.;
                check(
                    tangent_u[i][j][k] == 0. || controls[inner_a][b][k] != p,
                    "Hermite patch u tangent collapses at supplied coordinate precision",
                )?;
                check(
                    tangent_v[i][j][k] == 0. || controls[a][inner_b][k] != p,
                    "Hermite patch v tangent collapses at supplied coordinate precision",
                )?;
                check(
                    twist[i][j][k] == 0. || controls[inner_a][inner_b][k] != base,
                    "Hermite patch twist collapses at supplied coordinate precision",
                )?;
            }
        }
    }
    crate::patches::bezier(controls, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn jet(u: f64, v: f64) -> [[f64; 3]; 4] {
        [
            [
                u + 2. * v + u * v,
                u * u + v * v * v + u * u * u * v * v,
                u * u * u * v * v * v,
            ],
            [1. + v, 2. * u + 3. * u * u * v * v, 3. * u * u * v * v * v],
            [
                2. + u,
                3. * v * v + 2. * u * u * u * v,
                3. * u * u * u * v * v,
            ],
            [1., 6. * u * u * v, 9. * u * u * v * v],
        ]
    }
    #[test]
    fn matches_independent_bicubic_positions_and_first_and_mixed_derivatives() {
        let data: [Corners; 4] = std::array::from_fn(|k| {
            std::array::from_fn(|i| std::array::from_fn(|j| jet(i as f64, j as f64)[k]))
        });
        let s = patch(data[0], data[1], data[2], data[3]).unwrap();
        for u in [0., 0.13, 0.5, 0.87, 1.] {
            for v in [0., 0.17, 0.5, 0.83, 1.] {
                let q = s.evaluate(u, v).unwrap();
                let expected = jet(u, v);
                let (du, dv) = q.first_derivatives().unwrap();
                let (_, duv, _) = q.second_derivatives().unwrap();
                for k in 0..3 {
                    assert!((q.point[k] - expected[0][k]).abs() < 1e-11);
                    assert!((du[k] - expected[1][k]).abs() < 1e-10);
                    assert!((dv[k] - expected[2][k]).abs() < 1e-10);
                    assert!((duv[k] - expected[3][k]).abs() < 1e-9);
                }
            }
        }
        assert_eq!(s.degree_u, 3);
        assert_eq!(s.degree_v, 3);
    }
    #[test]
    fn refuses_nonfinite_and_lost_authored_conditions_but_allows_degenerate_patches() {
        let zero = [[[0.; 3]; 2]; 2];
        assert!(patch(zero, zero, zero, zero).is_ok());
        assert!(patch([[[f64::NAN; 3]; 2]; 2], zero, zero, zero).is_err());
        let p = [[[1e9; 3]; 2]; 2];
        let tiny = [[[1e-9; 3]; 2]; 2];
        assert!(patch(p, tiny, zero, zero).is_err());
        assert!(patch(p, zero, tiny, zero).is_err());
        assert!(patch(p, zero, zero, tiny).is_err());
        assert!(patch(zero, [[[1e300; 3]; 2]; 2], zero, zero).is_err());
    }
}
