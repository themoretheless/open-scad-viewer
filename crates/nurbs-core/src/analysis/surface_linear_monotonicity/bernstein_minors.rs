//! Correlated principal minors on an original rational ruled quadratic span.
//! Strict positive Bernstein coefficients enclose the complete cell, including
//! its boundary. This is only a local P-matrix test; the caller owns the C1
//! rectangle prerequisite, shared cell count and global injectivity conclusion.
use super::*;
use crate::surface_injectivity::{
    PolarPoly, polar_poly_add, polar_poly_derivative, polar_poly_mul,
};

pub(super) fn certify(
    s: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    projection: [[i8; 3]; 2],
) -> Result<bool> {
    // Keep tensor-product arithmetic bounded. Other degrees retain the
    // existing interval-jet proof and subdivision policy.
    if !matches!((s.degree_u, s.degree_v), (2, 1) | (1, 2)) {
        return Ok(false);
    }
    let net = crate::curve_surface_composition::surface_net_on(s, span, domain)?;
    if net.iter().flatten().any(|h| h[3].lo <= 0.) {
        return Ok(false);
    }
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    let mut coordinates: [PolarPoly; 3] = std::array::from_fn(|_| Vec::new());
    for row in net {
        let mut projected: [Vec<I>; 3] = std::array::from_fn(|_| Vec::new());
        for h in row {
            for r in 0..2 {
                let mut value = I::point(0.);
                for k in 0..3 {
                    if projection[r][k] != 0 {
                        let relative = h[k].sub(I::point(origin[k]).mul(h[3])?)?;
                        value = value.add(relative.mul(I::point(projection[r][k] as f64))?)?;
                    }
                }
                projected[r].push(value);
            }
            projected[2].push(h[3]);
        }
        for (target, row) in coordinates.iter_mut().zip(projected) {
            target.push(row);
        }
    }
    let w = &coordinates[2];
    let numerator = |row: usize, axis: usize| -> Result<PolarPoly> {
        let x = &coordinates[row];
        polar_poly_add(
            &polar_poly_mul(&polar_poly_derivative(x, axis, domain)?, w)?,
            &polar_poly_mul(x, &polar_poly_derivative(w, axis, domain)?)?,
            -1.,
        )
    };
    let positive = |p: &PolarPoly| p.iter().flatten().all(|c| c.lo > 0.);
    let uu = numerator(0, 0)?;
    let vv = numerator(1, 1)?;
    if !positive(&uu) || !positive(&vv) {
        return Ok(false);
    }
    // J_ij = A_ij / W^2. W is positive, so the signs of the diagonal
    // numerators and det(A) are exactly the P-matrix signs of J. Multiplying
    // coefficient nets before taking a hull retains the shared U/V dependence.
    let uv = numerator(0, 1)?;
    let vu = numerator(1, 0)?;
    let determinant = polar_poly_add(&polar_poly_mul(&uu, &vv)?, &polar_poly_mul(&uv, &vu)?, -1.)?;
    Ok(positive(&determinant))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ruled_quarter() -> Surface {
        let h = 1. / 1024.;
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![1., 0., 0.], vec![1., 0., h]],
                vec![vec![1., 1., 0.], vec![1., 1., h]],
                vec![vec![0., 1., 0.], vec![0., 1., h]],
            ],
            weights: vec![
                vec![1.; 2],
                vec![std::f64::consts::FRAC_1_SQRT_2; 2],
                vec![1.; 2],
            ],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn full_rational_ruled_chart_uses_one_shared_cell_and_preserves_source() {
        let s = ruled_quarter();
        let before = s.clone();
        let map = [[-1, 1, 0], [0, 0, 1]];
        let proof = inspect(&s, map, 1).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert_eq!(proof.cells, 1);
        assert!(!inspect(&s, map, 0).unwrap().certified);
        assert_eq!(s, before);
        // Independently known positive quarter-circle traversal and positive
        // Z extrusion admit the fixed map on the full closed rectangle.
        for u in [0., 0.125, 0.5, 0.875, 1.] {
            let (du, dv) = s.evaluate(u, 0.5).unwrap().first_derivatives().unwrap();
            assert!(-du[0] + du[1] > 0.);
            assert!((dv[2] - 1. / 1024.).abs() <= 1e-15);
        }
    }

    #[test]
    fn uniform_valid_weight_scale_preserves_proof_and_oversized_weights_refuse() {
        let mut s = ruled_quarter();
        // The exact power-of-two scale preserves the image and stays inside
        // the existing [1e-12, 1e12] source-weight admission range.
        for weight in s.weights.iter_mut().flatten() {
            *weight *= 2_f64.powi(39);
        }
        let map = [[-1, 1, 0], [0, 0, 1]];
        let proof = inspect(&s, map, 1).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert_eq!(proof.cells, 1);
        for weight in s.weights.iter_mut().flatten() {
            *weight *= 4.;
        }
        let error = inspect(&s, map, 512).unwrap_err();
        assert_eq!(error.code, "NURBS_INVALID_INPUT");
        assert!(error.message.contains("Weights must be"));
    }

    #[test]
    #[cfg(feature = "codec")]
    fn retained_three_cavity_chart_certifies_without_subdivision() {
        let s: Surface = value_codec::from_str(include_str!("three_cavity_face.json")).unwrap();
        let proof = inspect_candidate(&s, 1).unwrap();
        assert!(proof.certified, "{proof:?}");
        assert_eq!(proof.cells, 1);
    }

    #[test]
    fn collapsed_ruled_interior_is_not_certified_by_correlated_minors() {
        let mut s = ruled_quarter();
        for row in &mut s.control_points {
            row[1][0] = -row[0][0];
            row[1][1] = -row[0][1];
        }
        // Distinct source parameters have exactly the same image, independent
        // of the predicate implementation and its proposed projection.
        assert_eq!(
            s.evaluate(0., 0.5).unwrap().point,
            s.evaluate(1., 0.5).unwrap().point
        );
        assert!(!inspect_candidate(&s, 64).unwrap().certified);
        assert!(!certify(&s, [2, 1], [[0., 1.]; 2], [[-1, 1, 0], [0, 0, 1]]).unwrap());
    }
}
