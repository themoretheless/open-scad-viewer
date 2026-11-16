//! Exact homogeneous elimination for rational graphs with affine XY projection.
use super::*;

pub(super) fn weight_net(s: &Surface) -> Result<Net> {
    s.weights.iter().map(|row| row.iter().map(|&w| q(w)).collect()).collect()
}

pub(super) fn coordinate_net(s: &Surface, coordinate: usize, weights: Option<&Net>) -> Result<Net> {
    s.control_points.iter().enumerate().map(|(i, row)| {
        row.iter().enumerate().map(|(j, point)| {
            let value = q(point[coordinate])?;
            Ok(if let Some(w) = weights { value * &w[i][j] } else { value })
        }).collect()
    }).collect()
}

// Input degrees are bounded by the affine graph admission (at most eight).
// Compute exact tensor Bernstein product factors once per axis.
pub(super) fn product(a: &Net, b: &Net) -> Net {
    let factors = |p: usize, q: usize| {
        let binomials = |n: usize| {
            let mut value = 1u32;
            (0..=n).map(|i| {
                let current = value;
                value = value * (n - i) as u32 / (i + 1) as u32;
                current
            }).collect::<Vec<_>>()
        };
        let (a, b, c) = (binomials(p), binomials(q), binomials(p + q));
        a.iter().enumerate().map(|(i, x)| {
            b.iter().enumerate().map(|(j, y)| Q::new((x * y).into(), c[i + j].into()))
                .collect::<Vec<_>>()
        }).collect::<Vec<_>>()
    };
    let u = factors(a.len() - 1, b.len() - 1);
    let v = factors(a[0].len() - 1, b[0].len() - 1);
    let mut out = vec![vec![Q::zero(); a[0].len() + b[0].len() - 1]; a.len() + b.len() - 1];
    for (i, row) in a.iter().enumerate() {
        for (j, x) in row.iter().enumerate().filter(|(_, x)| !x.is_zero()) {
            for (k, row) in b.iter().enumerate() {
                let x = x * &u[i][k];
                for (l, y) in row.iter().enumerate().filter(|(_, y)| !y.is_zero()) {
                    out[i + k][j + l] += &x * y * &v[j][l];
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_projection_with_nonconstant_weights_preserves_exact_contact() {
        let surface = |z: fn(f64) -> f64| Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            periodic_u: false,
            periodic_v: false,
            control_points: [0., 0.25, 1.].map(|x| {
                vec![vec![x, 0., z(x)], vec![x, 1., z(x)]]
            }).to_vec(),
            weights: vec![vec![1.; 2], vec![2.; 2], vec![3.; 2]],
        };
        let a = surface(|x| x);
        let b = surface(|_| 0.5);
        let source = FiberSources::new(&a, &b).unwrap();
        let proof = AffineGraphExclusion::new(&source).unwrap();
        assert!(!proof.excludes([[0., 1.]; 4]).unwrap());
        assert!(proof.excludes([[0., 0.25], [0., 1.], [0., 1.], [0., 1.]]).unwrap());
        assert!(!proof.excludes([[0., 0.5], [0., 1.], [0., 1.], [0., 1.]]).unwrap());

        let mut multispan = b.clone();
        multispan.knots_u = vec![2., 2., 2., 3., 4., 4., 4.];
        multispan.control_points = [0., 0.125, 0.6875, 1.]
            .map(|x| vec![vec![x, 0., 0.5], vec![x, 1., 0.5]])
            .to_vec();
        multispan.weights = [0.5, 1., 2., 2.5].map(|w| vec![w; 2]).to_vec();
        let source = FiberSources::new(&a, &multispan).unwrap();
        let proof = AffineGraphExclusion::new(&source).unwrap();
        assert_eq!(proof.matching_parameter(0, 0.5).unwrap(), [3., 3.]);
        let mut domain = [[0., 1.], [0., 1.], [2., 2.5], [0., 1.]];
        assert!(proof.excludes(domain).unwrap());
        domain[2][1] = 3.;
        assert!(!proof.excludes(domain).unwrap());

        multispan.control_points[1][0][0] += 0.125;
        let source = FiberSources::new(&a, &multispan).unwrap();
        assert!(AffineGraphExclusion::new(&source).is_err());

        let rotate = |mut surface: Surface| {
            for point in surface.control_points.iter_mut().flatten() {
                point.swap(0, 2);
                point[0] = -point[0];
            }
            surface
        };
        let (rotated_a, rotated_b) = (rotate(a.clone()), rotate(b.clone()));
        let rotated = FiberSources::new(&rotated_a, &rotated_b).unwrap();
        let proof = AffineGraphExclusion::new(&rotated).unwrap();
        assert!(!proof.excludes([[0., 1.]; 4]).unwrap());
        assert!(proof.excludes([[0., 0.25], [0., 1.], [0., 1.], [0., 1.]]).unwrap());

        let mut damaged = a;
        damaged.control_points[1][0][0] += 0.125;
        let source = FiberSources::new(&damaged, &b).unwrap();
        assert!(AffineGraphExclusion::new(&source).is_err());
    }

    #[test]
    fn exact_subdivision_separates_mixed_sign_controls_and_keeps_tangency() {
        let plane = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            periodic_u: false,
            periodic_v: false,
            control_points: [0., 0.5, 1.]
                .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
                .to_vec(),
            weights: vec![vec![1.; 2]; 3],
        };
        let mut patch = plane.clone();
        for (row, z) in patch.control_points.iter_mut().zip([0.3125, -0.1875, 0.3125]) {
            for point in row {
                point[2] = z;
            }
        }
        let domain = [[0., 1.]; 4];
        let source = FiberSources::new(&plane, &patch).unwrap();
        let proof = AffineGraphExclusion::new(&source).unwrap();
        assert_eq!(proof.excludes_subdivided(domain, 1).unwrap(), (false, 1));
        assert_eq!(proof.excludes_subdivided(domain, 3).unwrap(), (true, 3));
        for point in patch.control_points.iter_mut().flatten() {
            point[2] -= 0.0625;
        }
        let source = FiberSources::new(&plane, &patch).unwrap();
        let proof = AffineGraphExclusion::new(&source).unwrap();
        assert!(!proof.excludes_subdivided(domain, 128).unwrap().0);
    }
}
