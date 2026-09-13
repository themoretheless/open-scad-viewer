//! Authoritative affine placement of retained B-rep geometry.
use crate::{Model, Result, invalid};

pub fn affine(model: &Model, matrix: [[f64; 4]; 4]) -> Result<Model> {
    if matrix.iter().flatten().any(|v| !v.is_finite()) || matrix[3] != [0., 0., 0., 1.] {
        return Err(invalid(
            "B-rep transform requires a finite affine 4x4 matrix",
        ));
    }
    let scale = matrix[..3]
        .iter()
        .flat_map(|row| &row[..3])
        .map(|v| v.abs())
        .fold(0., f64::max);
    if scale == 0. {
        return Err(invalid("B-rep transform must be nonsingular"));
    }
    let [a, b, c] = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| matrix[i][j] / scale)
    });
    let determinant = a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0])
        + a[2] * (b[0] * c[1] - b[1] * c[0]);
    if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
        return Err(invalid(
            "B-rep transform is singular or numerically unresolved",
        ));
    }
    model.validate()?;
    let point = |p: &[f64]| -> [f64; 3] {
        std::array::from_fn(|i| matrix[i][3] + (0..3).map(|j| matrix[i][j] * p[j]).sum::<f64>())
    };
    let mut result = model.clone();
    for vertex in &mut result.vertices {
        vertex.point = point(&vertex.point);
    }
    for edge in &mut result.edges {
        for p in &mut edge.curve.control_points {
            *p = point(p).to_vec();
        }
    }
    for face in &mut result.faces {
        for row in &mut face.surface.control_points {
            for p in row {
                *p = point(p).to_vec();
            }
        }
    }
    if determinant < 0. {
        for shell in &mut result.shells {
            for face in &mut shell.faces {
                face.reversed = !face.reversed;
            }
        }
    }
    result.validate()?;
    Ok(result)
}

/// Maps local XY and its cross-product normal into a sketch workplane.
pub fn workplane(
    model: &Model,
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    offset: [f64; 3],
) -> Result<Model> {
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let mut matrix = [[0.; 4]; 4];
    for i in 0..3 {
        matrix[i] = [
            u[i],
            v[i],
            n[i],
            origin[i] + offset[0] * u[i] + offset[1] * v[i] + offset[2] * n[i],
        ];
    }
    matrix[3][3] = 1.;
    affine(model, matrix)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn affine_reflection_preserves_identity_and_reverses_shell_uses() {
        let source = crate::cuboid([0., 0., 0.], [1., 2., 3.]).unwrap();
        let before = value_codec::to_string(&source).unwrap();
        let result = affine(
            &source,
            [
                [-2., 0., 0., 10.],
                [0., 3., 0., -5.],
                [0., 0., 4., 6.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        for (a, b) in source.vertices.iter().zip(&result.vertices) {
            assert_eq!(
                b.point,
                [
                    -2. * a.point[0] + 10.,
                    3. * a.point[1] - 5.,
                    4. * a.point[2] + 6.
                ]
            );
        }
        assert_eq!(source.1.faces, result.1.faces);
        for (a, b) in source.shells.iter().zip(&result.shells) {
            for (a, b) in a.faces.iter().zip(&b.faces) {
                assert_ne!(a.reversed, b.reversed);
            }
        }
        assert_eq!(before, value_codec::to_string(&source).unwrap());
        assert!(affine(&source, [[0.; 4]; 4]).is_err());
    }
    #[test]
    fn workplane_placement_matches_authored_axes() {
        let source = crate::cylinder(2., 3.).unwrap();
        let result = workplane(
            &source,
            [10., 20., 30.],
            [0., 1., 0.],
            [0., 0., 1.],
            [2., 3., -4.],
        )
        .unwrap();
        for (a, b) in source.vertices.iter().zip(&result.vertices) {
            assert_eq!(
                b.point,
                [6. + a.point[2], 22. + a.point[0], 33. + a.point[1]]
            );
        }
    }
}

/// SPC1 column-major affine transform of material-left planar profile loops.
pub fn profile(
    loops: &[Vec<nurbs_core::curve::Curve>],
    matrix: [f64; 16],
    tolerance: f64,
) -> Result<Vec<Vec<nurbs_core::curve::Curve>>> {
    let unsupported =
        |message| nurbs_core::Error::new("BREP_UNSUPPORTED_PROFILE_TRANSFORM", message);
    if matrix.iter().any(|x| !x.is_finite())
        || [matrix[3], matrix[7], matrix[11], matrix[15]] != [0., 0., 0., 1.]
    {
        return Err(unsupported(
            "A profile transform requires a finite affine matrix",
        ));
    }
    if matrix[2] != 0. || matrix[6] != 0. || matrix[14] != 0. {
        return Err(unsupported(
            "A 2D B-rep transform must preserve the XY plane",
        ));
    }
    let [a, b, c, d] = [matrix[0], matrix[4], matrix[1], matrix[5]];
    let scale = [a, b, c, d].into_iter().map(f64::abs).fold(0., f64::max);
    let [na, nb, nc, nd] = [a, b, c, d].map(|x| x / scale);
    let determinant = na * nd - nb * nc;
    if !determinant.is_finite() || determinant.abs() <= f64::EPSILON {
        return Err(unsupported(
            "A 2D B-rep transform must be numerically nonsingular",
        ));
    }
    if loops.iter().flatten().any(|curve| curve.degree > 1)
        && ((na * na + nc * nc - nb * nb - nd * nd).abs() > 16. * f64::EPSILON
            || (na * nb + nc * nd).abs() > 16. * f64::EPSILON)
    {
        return Err(unsupported(
            "Nonuniform scale/shear of circular profiles requires unsupported elliptical trims",
        ));
    }
    crate::planar_trim::validate(loops, tolerance)?;
    let mut result = loops.to_vec();
    for wire in &mut result {
        for curve in wire.iter_mut() {
            for point in &mut curve.control_points {
                let (x, y) = (point[0], point[1]);
                *point = vec![a * x + b * y + matrix[12], c * x + d * y + matrix[13]];
            }
        }
        if determinant < 0. {
            wire.reverse();
            for curve in wire {
                *curve = curve.reverse()?;
            }
        }
    }
    crate::planar_trim::validate(&result, tolerance)?;
    Ok(result)
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[test]
    fn reflected_circle_keeps_material_orientation_and_scaled_area() {
        let loops = vec![crate::sketch::circle_wire(2.).unwrap()];
        let original = value_codec::to_string(&loops).unwrap();
        let matrix = [
            -3., 0., 0., 0., 0., 3., 0., 0., 0., 0., 1., 0., 10., 20., 0., 1.,
        ];
        let moved = profile(&loops, matrix, 1e-7).unwrap();
        let area = crate::planar_trim::signed_area(&moved[0], 1e-7).unwrap();
        assert!((area - 36. * std::f64::consts::PI).abs() < 1e-8);
        assert_eq!(original, value_codec::to_string(&loops).unwrap());
        for curve in &moved[0] {
            for i in 0..=16 {
                let p = curve.evaluate(i as f64 / 16.).unwrap().point;
                assert!(((p[0] - 10.).powi(2) + (p[1] - 20.).powi(2) - 36.).abs() < 1e-10);
            }
        }
        let mut shear = matrix;
        shear[4] = 1.;
        assert_eq!(
            profile(&loops, shear, 1e-7).unwrap_err().code,
            "BREP_UNSUPPORTED_PROFILE_TRANSFORM"
        );
        let mut lifted = matrix;
        lifted[14] = 1.;
        assert!(profile(&loops, lifted, 1e-7).is_err());
    }
    #[test]
    fn polygon_profile_admits_nonsingular_shear() {
        let loops = vec![
            crate::sketch::polygon_wire(vec![[0., 0.], [2., 0.], [2., 1.], [0., 1.]]).unwrap(),
        ];
        let moved = profile(
            &loops,
            [
                2., 0., 0., 0., 1., 3., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
            ],
            1e-7,
        )
        .unwrap();
        assert!((crate::planar_trim::signed_area(&moved[0], 1e-7).unwrap() - 12.).abs() < 1e-10);
    }
}
