//! Mean-value error between a support expression and one affine UV triangle.
use nurbs_core::{Result, interval_eval::Interval as I};
/// Lower bound on correspondence error at one UV point against the original
/// affine mesh plane. This is a witness, never a whole-triangle upper bound.
pub(super) fn point_deviation_lower(
    uv: [[f64; 2]; 3],
    points: [[f64; 3]; 3],
    query: [f64; 2],
    support: [[f64; 2]; 3],
) -> Result<Option<f64>> {
    let d = |a: f64, b: f64| I::point(a).sub(I::point(b));
    let u1 = d(uv[1][0], uv[0][0])?;
    let v1 = d(uv[1][1], uv[0][1])?;
    let u2 = d(uv[2][0], uv[0][0])?;
    let v2 = d(uv[2][1], uv[0][1])?;
    let det = u1.mul(v2)?.sub(u2.mul(v1)?)?;
    if det.lo <= 0. && det.hi >= 0. {
        return Ok(None);
    }
    let offset = [d(query[0], uv[0][0])?, d(query[1], uv[0][1])?];
    let mut lower = 0_f64;
    for k in 0..3 {
        let p1 = d(points[1][k], points[0][k])?;
        let p2 = d(points[2][k], points[0][k])?;
        let slopes = [
            p1.mul(v2)?.sub(p2.mul(v1)?)?.div_signed(det)?,
            u1.mul(p2)?.sub(u2.mul(p1)?)?.div_signed(det)?,
        ];
        let plane = I::point(points[0][k])
            .add(slopes[0].mul(offset[0])?)?
            .add(slopes[1].mul(offset[1])?)?;
        let residual = I::new(support[k][0], support[k][1])?.sub(plane)?;
        // A coordinate lower bound is also a Euclidean lower bound. No
        // inward-rounded sum or square root is used, including at tiny scales.
        lower = lower.max(if residual.lo > 0. {
            residual.lo
        } else if residual.hi < 0. {
            -residual.hi
        } else {
            0.
        });
    }
    Ok(Some(lower))
}
pub(super) fn bound(
    uv: [[f64; 2]; 3],
    points: [[f64; 3]; 3],
    anchor: [[f64; 2]; 3],
    derivatives: [[[f64; 2]; 3]; 2],
) -> Result<Option<f64>> {
    bound_at(uv, points, uv[0], anchor, derivatives)
}
pub(super) fn bound_at(
    uv: [[f64; 2]; 3],
    points: [[f64; 3]; 3],
    anchor_uv: [f64; 2],
    anchor: [[f64; 2]; 3],
    derivatives: [[[f64; 2]; 3]; 2],
) -> Result<Option<f64>> {
    bound_region(uv, points, uv, anchor_uv, anchor, derivatives)
}
/// Bound a local parameter triangle against the original mesh plane. The caller
/// must enclose derivatives on all anchor-to-region segments and prove coverage
/// separately before combining regional bounds into a whole-triangle certificate.
pub(super) fn bound_region(
    uv: [[f64; 2]; 3],
    points: [[f64; 3]; 3],
    region_uv: [[f64; 2]; 3],
    anchor_uv: [f64; 2],
    anchor: [[f64; 2]; 3],
    derivatives: [[[f64; 2]; 3]; 2],
) -> Result<Option<f64>> {
    let difference = |a: f64, b: f64| I::point(a).sub(I::point(b));
    let u1 = difference(uv[1][0], uv[0][0])?;
    let v1 = difference(uv[1][1], uv[0][1])?;
    let u2 = difference(uv[2][0], uv[0][0])?;
    let v2 = difference(uv[2][1], uv[0][1])?;
    let det = u1.mul(v2)?.sub(u2.mul(v1)?)?;
    if det.lo <= 0. && det.hi >= 0. {
        return Ok(None);
    }
    // The displacement belongs to the triangle, not its enclosing rectangle.
    // For every possible gradient, extrema of its dot product occur at a vertex.
    // Keeping each vertex direction avoids combining unrelated UV extremes.
    let mut deltas = [[I::point(0.); 2], [u1, v1], [u2, v2]];
    if anchor_uv != uv[0] || region_uv != uv {
        for i in 0..3 {
            deltas[i] = [
                difference(region_uv[i][0], anchor_uv[0])?,
                difference(region_uv[i][1], anchor_uv[1])?,
            ];
        }
    }
    let offset = [
        difference(anchor_uv[0], uv[0][0])?,
        difference(anchor_uv[1], uv[0][1])?,
    ];
    let mut total = I::point(0.);
    let mut magnitudes = [0.; 3];
    let mut vertex_magnitudes = [[0.; 3]; 3];
    for k in 0..3 {
        let p1 = difference(points[1][k], points[0][k])?;
        let p2 = difference(points[2][k], points[0][k])?;
        let slopes = [
            p1.mul(v2)?.sub(p2.mul(v1)?)?.div_signed(det)?,
            u1.mul(p2)?.sub(u2.mul(p1)?)?.div_signed(det)?,
        ];
        let plane_at_anchor = if anchor_uv == uv[0] {
            I::point(points[0][k])
        } else {
            I::point(points[0][k])
                .add(slopes[0].mul(offset[0])?)?
                .add(slopes[1].mul(offset[1])?)?
        };
        let anchor_error = I::new(anchor[k][0], anchor[k][1])?.sub(plane_at_anchor)?;
        let gradients = [
            I::new(derivatives[0][k][0], derivatives[0][k][1])?.sub(slopes[0])?,
            I::new(derivatives[1][k][0], derivatives[1][k][1])?.sub(slopes[1])?,
        ];
        let mut lower = f64::INFINITY;
        let mut upper = f64::NEG_INFINITY;
        for (vertex, delta) in deltas.into_iter().enumerate() {
            let directional = anchor_error
                .add(gradients[0].mul(delta[0])?)?
                .add(gradients[1].mul(delta[1])?)?;
            vertex_magnitudes[vertex][k] = directional.lo.abs().max(directional.hi.abs());
            lower = lower.min(directional.lo);
            upper = upper.max(directional.hi);
        }
        let residual = I::new(lower, upper)?;
        magnitudes[k] = residual.lo.abs().max(residual.hi.abs());
        total = total.add(I::point(magnitudes[k]))?;
    }
    // Scaling keeps squared coordinate bounds in range for tiny and large
    // models. Each interval operation and the square root round outward.
    let scale = magnitudes.iter().copied().fold(0., f64::max);
    if scale == 0. {
        return Ok(Some(total.hi));
    }
    let mut squared = I::point(0.);
    for magnitude in magnitudes {
        let ratio = magnitude / scale;
        let unit = I::new(ratio.next_down(), ratio.next_up())?;
        squared = squared.add(unit.mul(unit)?)?;
    }
    let euclidean = I::point(scale).mul(I::point(squared.hi.sqrt().next_up()))?;
    // For each admissible residual gradient, the norm of its affine error is
    // convex on the UV triangle. Its maximum is attained at a vertex. Bound
    // each vertex independently before taking the maximum, avoiding coordinate
    // extrema from different vertices being combined into one vector.
    let mut vertex_upper = 0_f64;
    for coordinates in vertex_magnitudes {
        let scale = coordinates.into_iter().fold(0_f64, f64::max);
        if scale == 0. {
            continue;
        }
        let mut squared = I::point(0.);
        for magnitude in coordinates {
            // Direct division also works for subnormal scales, whose reciprocal
            // overflows even though every normalized coordinate is at most one.
            let ratio = magnitude / scale;
            let unit = I::new(ratio.next_down(), ratio.next_up())?;
            squared = squared.add(unit.mul(unit)?)?;
        }
        let upper = I::point(scale)
            .mul(I::point(squared.hi.sqrt().next_up()))?
            .hi;
        vertex_upper = vertex_upper.max(upper);
    }
    Ok(Some(total.hi.min(euclidean.hi).min(vertex_upper)))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn point_witness_detects_bow_preserves_plane_and_refuses_degeneracy() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        for scale in [1e-150, 1., 1e150] {
            let lower = point_deviation_lower(
                uv,
                points,
                [0.5, 0.],
                [[0.5; 2], [0.; 2], [0.25 * scale; 2]],
            )
            .unwrap()
            .unwrap();
            assert!(lower > 0.249999999999 * scale);
            assert!(lower <= 0.25 * scale);
        }
        assert_eq!(
            point_deviation_lower(
                uv,
                points,
                [0.5, 0.],
                [[0.4, 0.6], [-0.1, 0.1], [-0.1, 0.1]]
            )
            .unwrap(),
            Some(0.)
        );
        assert_eq!(
            point_deviation_lower([[0.; 2]; 3], points, [0., 0.], [[1.; 2]; 3]).unwrap(),
            None
        );
        let reversed = point_deviation_lower(
            [uv[0], uv[2], uv[1]],
            [points[0], points[2], points[1]],
            [0.5, 0.],
            [[0.5; 2], [0.; 2], [0.25; 2]],
        )
        .unwrap()
        .unwrap();
        assert!(reversed > 0.249999999999);
    }
    #[test]
    fn local_region_keeps_the_original_plane_instead_of_refitting_the_surface() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 0., 0.]];
        let anchor = [[0., 0.]; 3];
        let whole = bound(
            uv,
            points,
            anchor,
            [[[0., 2.], [0., 0.], [0., 0.]], [[0., 0.]; 3]],
        )
        .unwrap()
        .unwrap();
        let region = [[0., 0.], [0.5, 0.], [0., 0.5]];
        let local = bound_region(
            uv,
            points,
            region,
            [0., 0.],
            anchor,
            [[[0., 1.], [0., 0.], [0., 0.]], [[0., 0.]; 3]],
        )
        .unwrap()
        .unwrap();
        assert!(local < whole);
        // Refitting the subtriangle to u² would hide error against the parent
        // plane u. Keep the actual 0.25 residual at u=0.5 in the enclosure.
        assert!(local >= 0.25);
        for i in 0..=50 {
            let u = i as f64 / 100.;
            assert!((u * u - u).abs() <= local);
        }
    }
    #[test]
    fn rectangle_owned_anchor_outside_triangle_still_encloses_residual() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let error = bound_at(
            uv,
            [[0., 0., 0.], [1., 0., 0.], [0., 0., 0.]],
            [0.9, 0.9],
            [[0.81, 0.81], [0., 0.], [0., 0.]],
            [
                [[0., 2.], [0., 0.], [0., 0.]],
                [[0., 0.], [0., 0.], [0., 0.]],
            ],
        )
        .unwrap()
        .unwrap();
        for i in 0..=100 {
            let u = i as f64 / 100.;
            assert!((u * u - u).abs() <= error);
        }
    }
    #[test]
    fn vertex_norm_encloses_interval_gradient_corners_and_triangle_interior() {
        let uv = [[0., 0.], [2., 0.2], [-0.1, 1.]];
        let points = [[0.; 3]; 3];
        let anchor = [[-0.2, 0.3], [-0.4, 0.1], [-0.1, 0.2]];
        let derivatives = [
            [[-2., 3.], [-0.3, 0.7], [0.1, 0.4]],
            [[-0.4, 0.9], [-4., 1.], [-0.5, 0.2]],
        ];
        let upper = bound(uv, points, anchor, derivatives).unwrap().unwrap();
        for mask in 0..512 {
            let a: [f64; 3] = std::array::from_fn(|k| anchor[k][(mask >> k) & 1]);
            let g: [[f64; 3]; 2] = std::array::from_fn(|axis| {
                std::array::from_fn(|k| derivatives[axis][k][(mask >> (3 + 3 * axis + k)) & 1])
            });
            for i in 0..=10 {
                for j in 0..=10 - i {
                    let w = [1. - (i + j) as f64 / 10., i as f64 / 10., j as f64 / 10.];
                    let q: [f64; 2] =
                        std::array::from_fn(|axis| (0..3).map(|v| w[v] * uv[v][axis]).sum());
                    let squared: f64 = (0..3)
                        .map(|k| (a[k] + g[0][k] * q[0] + g[1][k] * q[1]).powi(2))
                        .sum();
                    assert!(squared.sqrt() <= upper);
                }
            }
        }
    }
    #[test]
    fn spatial_vertex_bound_does_not_mix_extrema_from_different_vertices() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let points = [[0.; 3]; 3];
        let derivatives = [[[4.; 2], [0.; 2], [0.; 2]], [[0.; 2], [3.; 2], [0.; 2]]];
        let error = bound(uv, points, [[0.; 2]; 3], derivatives)
            .unwrap()
            .unwrap();
        assert!(error >= 4. && error < 4.000000001);
    }
    #[test]
    fn affine_skew_triangle_has_roundoff_error_and_rounding_is_retained() {
        let uv = [[0., 0.], [2., 1.], [-1., 3.]];
        let p = uv.map(|[u, v]| [100. * u, 50. * v, 200. * u + 30. * v]);
        let d = [
            [[100.; 2], [0.; 2], [200.; 2]],
            [[0.; 2], [50.; 2], [30.; 2]],
        ];
        let reversed_uv = [uv[0], uv[2], uv[1]];
        let reversed_p = [p[0], p[2], p[1]];
        let reversed_error = bound(reversed_uv, reversed_p, [[0.; 2]; 3], d)
            .unwrap()
            .unwrap();
        assert!(reversed_error < 1e-8);
        let error = bound(uv, p, [[0.; 2]; 3], d).unwrap().unwrap();
        assert!(error < 1e-8);
        let error = bound(uv, p, [[0.01; 2]; 3], d).unwrap().unwrap();
        assert!(error >= 0.01 * 3f64.sqrt());
        assert!(error < 0.017321);
        assert!(
            bound([[0., 0.], [1., 1.], [2., 2.]], p, [[0.; 2]; 3], d)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn skew_triangle_does_not_inherit_unused_rectangle_corners() {
        // F=(u+v,0,0), with deliberately rounded zero mesh vertices.
        // The triangle reaches only [-0.1,0] in u+v, although its UV box
        // reaches [-2,1.9]. The bound must retain the rounding discrepancy.
        let error = bound(
            [[0., 0.], [1., -1.], [-1., 0.9]],
            [[0.; 3]; 3],
            [[0.; 2]; 3],
            [[[1.; 2], [0.; 2], [0.; 2]], [[1.; 2], [0.; 2], [0.; 2]]],
        )
        .unwrap()
        .unwrap();
        assert!(error >= 0.1 - 1e-15);
        assert!(error < 0.100000000001);
    }
    #[test]
    fn spatial_norm_encloses_three_four_five_residual_at_multiple_scales() {
        for scale in [1e-150, 1., 1e150] {
            let error = bound(
                [[0., 0.], [1., 0.], [0., 1.]],
                [[0.; 3]; 3],
                [[3. * scale; 2], [4. * scale; 2], [0.; 2]],
                [[[0.; 2]; 3]; 2],
            )
            .unwrap()
            .unwrap();
            assert!(error >= 5. * scale);
            assert!(error < 5.000000000001 * scale);
        }
    }
    #[test]
    fn centered_anchor_bounds_bow_and_preserves_affine_plane() {
        let uv = [[0., 0.], [1., 0.], [0., 1.]];
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let center = [1. / 3., 1. / 3.];
        let derivatives = [[[1.; 2], [0.; 2], [-1., 1.]], [[0.; 2], [1.; 2], [0.; 2]]];
        let vertex = bound(uv, points, [[0.; 2]; 3], derivatives)
            .unwrap()
            .unwrap();
        let centered = bound_at(
            uv,
            points,
            center,
            [[center[0]; 2], [center[1]; 2], [2. / 9.; 2]],
            derivatives,
        )
        .unwrap()
        .unwrap();
        assert!(centered >= 0.25);
        assert!(centered < vertex);
        let plane_derivatives = [[[1.; 2], [0.; 2], [0.; 2]], [[0.; 2], [1.; 2], [0.; 2]]];
        let flat = bound_at(
            uv,
            points,
            center,
            [[center[0]; 2], [center[1]; 2], [0.; 2]],
            plane_derivatives,
        )
        .unwrap()
        .unwrap();
        assert!(flat < 1e-12);
        let reversed = bound_at(
            [uv[0], uv[2], uv[1]],
            [points[0], points[2], points[1]],
            center,
            [[center[0]; 2], [center[1]; 2], [2. / 9.; 2]],
            derivatives,
        )
        .unwrap()
        .unwrap();
        assert!((reversed - centered).abs() < 1e-12);
    }
    #[test]
    fn interior_bow_is_bounded_without_sampling() {
        // z=u*(1-u), all three vertices lie at z=0; derivative spans [-1,1].
        let error = bound(
            [[0., 0.], [1., 0.], [0., 1.]],
            [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
            [[0.; 2]; 3],
            [[[1.; 2], [0.; 2], [-1., 1.]], [[0.; 2], [1.; 2], [0.; 2]]],
        )
        .unwrap()
        .unwrap();
        assert!(error >= 0.25);
    }
}
