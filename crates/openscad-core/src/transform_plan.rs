//! Stable-profile transform classification. Column-major matrices retain projective rows.
#[derive(Debug)]
pub struct MatrixAnalysis {
    pub matrix2d: [f64; 9],
    pub drops_children: bool,
    pub singular3d: bool,
    pub singular2d: bool,
    pub affine3d: bool,
    pub affine2d: bool,
}
fn determinant<const D: usize>(values: &[f64]) -> f64 {
    let mut rows = [[0.; D]; D];
    for row in 0..D {
        for column in 0..D {
            rows[row][column] = values[column * D + row];
        }
    }
    let mut sign = 1.;
    let mut result = 1.;
    for column in 0..D {
        let mut pivot = column;
        while pivot < D && rows[pivot][column] == 0. {
            pivot += 1;
        }
        if pivot == D {
            return 0.;
        }
        if pivot != column {
            rows.swap(column, pivot);
            sign *= -1.;
        }
        let pivot_value = rows[column][column];
        result *= pivot_value;
        for row in column + 1..D {
            let factor = rows[row][column] / pivot_value;
            for inner in column + 1..D {
                rows[row][inner] -= factor * rows[column][inner];
            }
        }
    }
    sign * result
}
pub fn analyze(matrix: [f64; 16]) -> MatrixAnalysis {
    let matrix2d = [
        matrix[0], matrix[1], matrix[3], matrix[4], matrix[5], matrix[7], matrix[12], matrix[13],
        matrix[15],
    ];
    let finite3d = matrix.iter().all(|v| v.is_finite());
    let finite2d = matrix2d.iter().all(|v| v.is_finite());
    MatrixAnalysis {
        matrix2d,
        drops_children: !finite3d,
        singular3d: finite3d && determinant::<4>(&matrix) == 0.,
        singular2d: finite2d && determinant::<3>(&matrix2d) == 0.,
        affine3d: matrix[3] == 0. && matrix[7] == 0. && matrix[11] == 0. && matrix[15] == 1.,
        affine2d: matrix2d[2] == 0. && matrix2d[5] == 0. && matrix2d[8] == 1.,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projective_rows_and_nonfinite_axes_are_preserved() {
        let mut matrix = [
            1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
        ];
        assert!(!analyze(matrix).singular3d);
        matrix[3] = 2.;
        let a = analyze(matrix);
        assert!(!a.affine3d);
        assert!(!a.affine2d);
        assert_eq!(a.matrix2d[2], 2.);
        matrix[10] = f64::NAN;
        let a = analyze(matrix);
        assert!(a.drops_children);
        assert!(!a.singular3d);
        assert!(!a.singular2d);
        matrix[10] = 0.;
        assert!(analyze(matrix).singular3d);
    }
}

fn degree_pair(value: f64) -> (f64, f64) {
    if !value.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    let quadrant = value / 90.;
    if quadrant.fract() == 0. {
        let index = ((quadrant % 4. + 4.) % 4.) as usize;
        return ([0., 1., 0., -1.][index], [1., 0., -1., 0.][index]);
    }
    // Preserve the language's multiplication order, including overflow.
    let radians = value * std::f64::consts::PI / 180.;
    (radians.sin(), radians.cos())
}
/// Column-major stable Euler rotation. Exact quadrant values are language semantics.
pub fn euler_matrix([x, y, z]: [f64; 3]) -> [f64; 16] {
    let (sx, cx) = degree_pair(x);
    let (sy, cy) = degree_pair(y);
    let (sz, cz) = degree_pair(z);
    [
        cy * cz,
        cy * sz,
        -sy,
        0.,
        cz * sx * sy - cx * sz,
        cx * cz + sx * sy * sz,
        cy * sx,
        0.,
        cx * cz * sy + sx * sz,
        -cz * sx + cx * sy * sz,
        cx * cy,
        0.,
        0.,
        0.,
        0.,
        1.,
    ]
}
#[cfg(test)]
mod euler_tests {
    use super::*;
    #[test]
    fn exact_quadrants_and_nonfinite_angles() {
        let matrix = euler_matrix([0., 0., 90.]);
        assert_eq!(matrix[0], 0.);
        assert_eq!(matrix[1], 1.);
        assert_eq!(matrix[4], -1.);
        assert!(euler_matrix([f64::NAN, 0., 0.])[5].is_nan());
    }
}

/// Stable reflection retains the original arithmetic, including overflow/underflow.
pub fn mirror_matrix([x, y, z]: [f64; 3]) -> [f64; 16] {
    if x == 0. && y == 0. && z == 0. {
        return euler_matrix([0.; 3]);
    }
    let squared = x * x + y * y + z * z;
    [
        1. - 2. * x * x / squared,
        -2. * x * y / squared,
        -2. * x * z / squared,
        0.,
        -2. * y * x / squared,
        1. - 2. * y * y / squared,
        -2. * y * z / squared,
        0.,
        -2. * z * x / squared,
        -2. * z * y / squared,
        1. - 2. * z * z / squared,
        0.,
        0.,
        0.,
        0.,
        1.,
    ]
}
#[cfg(test)]
mod mirror_tests {
    use super::*;
    #[test]
    fn zero_normal_and_axis_reflection() {
        assert_eq!(mirror_matrix([0.; 3]), euler_matrix([0.; 3]));
        let matrix = mirror_matrix([1., 0., 0.]);
        assert_eq!(matrix[0], -1.);
        assert_eq!(matrix[5], 1.);
        assert_eq!(matrix[10], 1.);
        assert!(mirror_matrix([f64::MAX, 0., 0.])[0].is_nan());
        assert!(mirror_matrix([1e-250, 0., 0.])[0].is_nan());
    }
}

fn axis_magnitude(axis: [f64; 3]) -> f64 {
    if axis.iter().any(|v| v.is_infinite()) {
        return f64::INFINITY;
    }
    if axis.iter().any(|v| v.is_nan()) {
        return f64::NAN;
    }
    let maximum = axis.iter().fold(0_f64, |m, v| m.max(v.abs()));
    if maximum == 0. {
        return 0.;
    }
    let mut sum = 0.;
    let mut compensation = 0.;
    for value in axis {
        let scaled = value.abs() / maximum;
        let term = scaled * scaled - compensation;
        let next = sum + term;
        compensation = (next - sum) - term;
        sum = next;
    }
    sum.sqrt() * maximum
}
/// Stable Rodrigues rotation, with zero/NaN magnitude preserving the identity.
pub fn axis_angle_matrix(degrees: f64, axis: [f64; 3]) -> [f64; 16] {
    let magnitude = axis_magnitude(axis);
    if !(magnitude > 0.) {
        return euler_matrix([0.; 3]);
    }
    let [x, y, z] = axis.map(|v| v / magnitude);
    let (sine, cosine) = degree_pair(degrees);
    rodrigues_matrix([x, y, z], sine, cosine)
}
/// Viewer-subset keeps ordinary trigonometry and refuses a zero rotation axis.
pub fn viewer_axis_angle_matrix(degrees: f64, axis: [f64; 3]) -> Option<[f64; 16]> {
    let magnitude = axis_magnitude(axis);
    if magnitude == 0. {
        return None;
    }
    let radians = degrees * std::f64::consts::PI / 180.;
    Some(rodrigues_matrix(
        axis.map(|v| v / magnitude),
        radians.sin(),
        radians.cos(),
    ))
}
fn rodrigues_matrix([x, y, z]: [f64; 3], sine: f64, cosine: f64) -> [f64; 16] {
    let complement = 1. - cosine;
    [
        complement * x * x + cosine,
        complement * x * y + sine * z,
        complement * x * z - sine * y,
        0.,
        complement * x * y - sine * z,
        complement * y * y + cosine,
        complement * y * z + sine * x,
        0.,
        complement * x * z + sine * y,
        complement * y * z - sine * x,
        complement * z * z + cosine,
        0.,
        0.,
        0.,
        0.,
        1.,
    ]
}
#[cfg(test)]
mod axis_tests {
    use super::*;
    #[test]
    fn zero_axis_and_nonfinite_magnitude() {
        assert_eq!(axis_angle_matrix(f64::NAN, [0.; 3]), euler_matrix([0.; 3]));
        assert_eq!(
            axis_angle_matrix(90., [f64::NAN, 0., 0.]),
            euler_matrix([0.; 3])
        );
        assert!(axis_angle_matrix(90., [f64::INFINITY, f64::NAN, 0.])[0].is_nan());
        let matrix = axis_angle_matrix(90., [0., 0., 1e-250]);
        assert_eq!(matrix[1], 1.);
        assert_eq!(matrix[4], -1.);
    }
}

#[derive(Debug)]
pub struct VectorConversion {
    pub vector: [f64; 3],
    pub converted: bool,
}
/// Stable vec2/vec3 conversion writes numeric prefixes into initialized dimensions.
pub fn vector_with_default(
    value: Option<&[Option<f64>]>,
    initial: [f64; 3],
    default_z: f64,
) -> VectorConversion {
    let mut vector = initial;
    let Some(value) = value else {
        return VectorConversion {
            vector,
            converted: false,
        };
    };
    if value.len() == 2 {
        vector[2] = default_z;
        if let [Some(x), Some(y)] = value {
            vector[0] = *x;
            vector[1] = *y;
        }
        return VectorConversion {
            vector,
            converted: true,
        };
    }
    if value.len() != 3 {
        return VectorConversion {
            vector,
            converted: false,
        };
    }
    for (index, value) in value.iter().enumerate() {
        let Some(value) = value else {
            return VectorConversion {
                vector,
                converted: false,
            };
        };
        vector[index] = *value;
    }
    VectorConversion {
        vector,
        converted: true,
    }
}
#[cfg(test)]
mod conversion_tests {
    use super::*;
    #[test]
    fn malformed_vectors_preserve_distinct_vec2_and_vec3_rules() {
        let a = vector_with_default(Some(&[Some(2.), None]), [1., 2., 3.], 0.);
        assert_eq!(a.vector, [1., 2., 0.]);
        assert!(a.converted);
        let b = vector_with_default(Some(&[Some(5.), None, Some(6.)]), [1., 2., 3.], 0.);
        assert_eq!(b.vector, [5., 2., 3.]);
        assert!(!b.converted);
    }
}

#[derive(Debug)]
pub struct EulerArguments {
    pub angles: [f64; 3],
    pub valid: bool,
}
/// Euler fallback visits authored components in Z/Y/X order.
pub fn euler_arguments(value: &[Option<f64>], authored_length: usize) -> EulerArguments {
    let mut angles = [0.; 3];
    let mut remembered = 0.;
    let mut valid = authored_length <= 3;
    for index in (0..authored_length.min(3)).rev() {
        if let Some(number) = value.get(index).copied().flatten() {
            remembered = number;
        } else {
            valid = false;
        }
        if !remembered.is_finite() {
            valid = false;
        }
        angles[index] = remembered;
    }
    EulerArguments { angles, valid }
}
#[cfg(test)]
mod euler_argument_tests {
    use super::*;
    #[test]
    fn reversed_component_fallback_and_length() {
        let a = euler_arguments(&[Some(2.), None, Some(4.)], 3);
        assert_eq!(a.angles, [2., 4., 4.]);
        assert!(!a.valid);
        let a = euler_arguments(&[None, Some(3.)], 2);
        assert_eq!(a.angles, [3., 3., 0.]);
        assert!(!a.valid);
        assert!(euler_arguments(&[], 0).valid);
        assert!(!euler_arguments(&[Some(1.); 3], 4).valid);
    }
}

#[derive(Debug)]
pub struct AuthoredMatrix {
    pub matrix: [f64; 16],
    pub valid: bool,
}
/// Missing or nonnumeric cells retain identity defaults; homogeneous W divides every cell.
pub fn authored_matrix(rows: Option<&[Vec<Option<f64>>]>) -> AuthoredMatrix {
    let mut matrix = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    let Some(rows) = rows else {
        return AuthoredMatrix {
            matrix,
            valid: false,
        };
    };
    for (row, values) in rows.iter().take(4).enumerate() {
        for (column, value) in values.iter().take(4).enumerate() {
            if let Some(value) = value {
                matrix[column * 4 + row] = *value;
            }
        }
    }
    let w = matrix[15];
    if w != 1. {
        for value in &mut matrix {
            *value /= w;
        }
    }
    AuthoredMatrix {
        matrix,
        valid: true,
    }
}
/// Permissive legacy XY extraction: nonfinite cells retain identity defaults.
pub fn authored_affine2d(rows: Option<&[Vec<Option<f64>>]>) -> Option<[f64; 9]> {
    let rows = rows?;
    let finite_rows: Vec<_> = rows
        .iter()
        .take(4)
        .map(|row| {
            row.iter()
                .take(4)
                .map(|value| value.filter(|v| v.is_finite()))
                .collect()
        })
        .collect();
    if finite_rows
        .get(3)
        .and_then(|row: &Vec<Option<f64>>| row.get(3))
        .copied()
        .flatten()
        == Some(0.)
    {
        return None;
    }
    let matrix = authored_matrix(Some(&finite_rows)).matrix;
    Some([
        matrix[0], matrix[1], 0., matrix[4], matrix[5], 0., matrix[12], matrix[13], 1.,
    ])
}
#[cfg(test)]
mod authored_matrix_tests {
    use super::*;
    #[test]
    fn legacy_xy_defaults_nonfinite_cells_and_normalizes_w() {
        assert_eq!(authored_affine2d(None), None);
        let rows = vec![
            vec![Some(2.), Some(f64::INFINITY), None, Some(4.)],
            vec![],
            vec![],
            vec![Some(7.), None, None, Some(2.)],
        ];
        assert_eq!(
            authored_affine2d(Some(&rows)),
            Some([1., 0., 0., 0., 0.5, 0., 2., 0., 1.])
        );
        let mut zero = rows;
        zero[3][3] = Some(0.);
        assert_eq!(authored_affine2d(Some(&zero)), None);
        zero[3][3] = Some(f64::NAN);
        assert_eq!(authored_affine2d(Some(&zero)).unwrap()[0], 2.);
    }
    #[test]
    fn permissive_cells_and_projective_normalization() {
        assert!(!authored_matrix(None).valid);
        let result = authored_matrix(Some(&[
            vec![Some(2.), None],
            vec![],
            vec![],
            vec![Some(4.), None, None, Some(2.)],
        ]));
        assert_eq!(result.matrix[0], 1.);
        assert_eq!(result.matrix[5], 0.5);
        assert_eq!(result.matrix[3], 2.);
        assert_eq!(result.matrix[15], 1.);
        assert!(
            authored_matrix(Some(&[
                vec![],
                vec![],
                vec![],
                vec![None, None, None, Some(0.)]
            ]))
            .matrix[1]
                .is_nan()
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub enum VectorTransform {
    Translate,
    Scale,
    Mirror,
}
#[derive(Debug)]
pub struct VectorTransformPlan {
    pub matrix: [f64; 16],
    pub valid: bool,
    pub range_warning: bool,
}
pub fn vector_transform(
    kind: VectorTransform,
    vector: Option<&[Option<f64>]>,
    scalar: Option<f64>,
) -> VectorTransformPlan {
    let (initial, z) = match kind {
        VectorTransform::Translate => ([0.; 3], 0.),
        VectorTransform::Scale => ([1.; 3], 1.),
        VectorTransform::Mirror => ([1., 0., 0.], 0.),
    };
    let mut converted = vector_with_default(vector, initial, z);
    let mut matrix = [
        1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1.,
    ];
    let mut range_warning = false;
    match kind {
        VectorTransform::Translate => {
            converted.converted &= converted.vector.iter().all(|v| v.is_finite());
            if converted.converted {
                matrix[12..15].copy_from_slice(&converted.vector);
            }
        }
        VectorTransform::Scale => {
            if !converted.converted
                && let Some(value) = scalar
            {
                converted.vector = [value; 3];
                converted.converted = true;
            }
            for (index, value) in converted.vector.iter().enumerate() {
                matrix[index * 5] = *value;
            }
            range_warning = converted.vector.iter().any(|v| *v == 0. || !v.is_finite());
        }
        VectorTransform::Mirror => matrix = mirror_matrix(converted.vector),
    }
    VectorTransformPlan {
        matrix,
        valid: converted.converted,
        range_warning,
    }
}
#[cfg(test)]
mod vector_transform_tests {
    use super::*;
    #[test]
    fn translate_refuses_partial_while_scale_preserves_it() {
        let vector = [Some(2.), None, Some(4.)];
        let t = vector_transform(VectorTransform::Translate, Some(&vector), None);
        assert!(!t.valid);
        assert_eq!(t.matrix[12], 0.);
        let s = vector_transform(VectorTransform::Scale, Some(&vector), None);
        assert!(!s.valid);
        assert_eq!(s.matrix[0], 2.);
        assert_eq!(s.matrix[5], 1.);
        let s = vector_transform(VectorTransform::Scale, None, Some(f64::INFINITY));
        assert!(s.valid);
        assert!(s.range_warning);
    }
}

#[derive(Debug)]
pub struct ScalarRotationPlan {
    pub matrix: [f64; 16],
    pub valid: bool,
    pub angle_valid: bool,
}
pub fn scalar_rotation(
    angle: Option<f64>,
    axis: Option<&[Option<f64>]>,
    axis_provided: bool,
) -> ScalarRotationPlan {
    let angle_valid = angle.is_some_and(|v| v.is_finite());
    let degrees = if angle_valid { angle.unwrap() } else { 0. };
    let axis = if axis_provided {
        vector_with_default(axis, [0., 0., 1.], 0.)
    } else {
        VectorConversion {
            vector: [0., 0., 1.],
            converted: true,
        }
    };
    ScalarRotationPlan {
        matrix: axis_angle_matrix(degrees, axis.vector),
        valid: angle_valid && axis.converted,
        angle_valid,
    }
}
#[cfg(test)]
mod scalar_rotation_tests {
    use super::*;
    #[test]
    fn omitted_axis_and_invalid_angle_priority() {
        let plan = scalar_rotation(Some(90.), None, false);
        assert!(plan.valid);
        assert_eq!(plan.matrix[1], 1.);
        let plan = scalar_rotation(None, Some(&[Some(2.), None]), true);
        assert!(!plan.valid);
        assert!(!plan.angle_valid);
        let plan = scalar_rotation(Some(90.), Some(&[Some(1.), None, Some(2.)]), true);
        assert!(!plan.valid);
        assert!(plan.angle_valid);
    }
}
