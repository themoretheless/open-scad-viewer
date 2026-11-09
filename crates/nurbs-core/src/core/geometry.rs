//! Checked 3D point/vector semantics and homogeneous NURBS coordinates.
//!
//! These fixed-size types supplement, rather than replace, the dimension-flexible
//! `Vec<f64>` storage of [`Curve`](super::curve::Curve). A point is a location;
//! a vector is a displacement. Consequently `Point3 - Point3 = Vector3`,
//! `Point3 + Vector3 = Point3`, and vectors have no translation operation.
//! `#[repr(transparent)]` keeps conversion to/from `[f64; 3]` explicit and free.

use crate::{Result, check, numeric};
use core::ops::{Add, AddAssign, Div, Mul, Neg, Sub};

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point3(pub [f64; 3]);

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vector3(pub [f64; 3]);

impl Point3 {
    pub fn new(coordinates: [f64; 3]) -> Result<Self> {
        check(
            coordinates.iter().all(|x| x.is_finite()),
            "Point3 coordinates must be finite",
        )?;
        Ok(Self(coordinates))
    }

    pub fn try_from_slice(coordinates: &[f64]) -> Result<Self> {
        check(
            coordinates.len() == 3,
            "Point3 requires exactly three coordinates",
        )?;
        Self::new([coordinates[0], coordinates[1], coordinates[2]])
    }

    pub fn coordinates(self) -> [f64; 3] {
        self.0
    }

    pub fn translated(self, displacement: Vector3) -> Self {
        self + displacement
    }

    pub fn weighted(self, weight: f64) -> Result<HomogeneousPoint3> {
        HomogeneousPoint3::from_weighted(self, weight)
    }
}

impl Vector3 {
    pub fn new(coordinates: [f64; 3]) -> Result<Self> {
        check(
            coordinates.iter().all(|x| x.is_finite()),
            "Vector3 coordinates must be finite",
        )?;
        Ok(Self(coordinates))
    }

    pub fn coordinates(self) -> [f64; 3] {
        self.0
    }

    pub fn dot(self, other: Self) -> f64 {
        self.0.iter().zip(other.0).map(|(a, b)| a * b).sum()
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }
}

impl Sub<Point3> for Point3 {
    type Output = Vector3;
    fn sub(self, rhs: Point3) -> Vector3 {
        Vector3(core::array::from_fn(|i| self.0[i] - rhs.0[i]))
    }
}
impl Add<Vector3> for Point3 {
    type Output = Point3;
    fn add(self, rhs: Vector3) -> Point3 {
        Point3(core::array::from_fn(|i| self.0[i] + rhs.0[i]))
    }
}
impl Sub<Vector3> for Point3 {
    type Output = Point3;
    fn sub(self, rhs: Vector3) -> Point3 {
        Point3(core::array::from_fn(|i| self.0[i] - rhs.0[i]))
    }
}
impl Add<Vector3> for Vector3 {
    type Output = Vector3;
    fn add(self, rhs: Vector3) -> Vector3 {
        Vector3(core::array::from_fn(|i| self.0[i] + rhs.0[i]))
    }
}
impl AddAssign for Vector3 {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}
impl Sub<Vector3> for Vector3 {
    type Output = Vector3;
    fn sub(self, rhs: Vector3) -> Vector3 {
        Vector3(core::array::from_fn(|i| self.0[i] - rhs.0[i]))
    }
}
impl Mul<f64> for Vector3 {
    type Output = Vector3;
    fn mul(self, rhs: f64) -> Vector3 {
        Vector3(self.0.map(|x| x * rhs))
    }
}
impl Mul<Vector3> for f64 {
    type Output = Vector3;
    fn mul(self, rhs: Vector3) -> Vector3 {
        rhs * self
    }
}
impl Div<f64> for Vector3 {
    type Output = Vector3;
    fn div(self, rhs: f64) -> Vector3 {
        self * (1. / rhs)
    }
}
impl Neg for Vector3 {
    type Output = Vector3;
    fn neg(self) -> Vector3 {
        self * -1.
    }
}

/// Homogeneous NURBS point `(w·x, w·y, w·z, w)`.
///
/// This is not a Euclidean direction. Linear B-spline operations are performed
/// on all four coordinates and [`project`](Self::project) divides only at the
/// end. The positive-weight contract matches the public NURBS curve storage.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HomogeneousPoint3 {
    pub weighted: Point3,
    pub weight: f64,
}

impl HomogeneousPoint3 {
    pub fn from_weighted(point: Point3, weight: f64) -> Result<Self> {
        check(
            weight.is_finite() && weight > 0.,
            "Homogeneous NURBS weight must be positive and finite",
        )?;
        let result = Self {
            weighted: Point3(point.0.map(|x| x * weight)),
            weight,
        };
        numeric(
            result.weighted.0.iter().all(|x| x.is_finite()),
            "Homogeneous coordinate multiplication overflowed",
        )?;
        Ok(result)
    }

    pub fn coordinates(self) -> [f64; 4] {
        [
            self.weighted.0[0],
            self.weighted.0[1],
            self.weighted.0[2],
            self.weight,
        ]
    }

    pub fn project(self) -> Result<Point3> {
        numeric(
            self.weight.is_finite() && self.weight > 0.,
            "Homogeneous projection requires a positive finite weight",
        )?;
        let point = Point3(self.weighted.0.map(|x| x / self.weight));
        numeric(
            point.0.iter().all(|x| x.is_finite()),
            "Homogeneous projection exhausted finite precision",
        )?;
        Ok(point)
    }

    pub(crate) fn lerp(self, other: Self, t: f64) -> Self {
        Self {
            weighted: Point3(core::array::from_fn(|i| {
                (1. - t) * self.weighted.0[i] + t * other.weighted.0[i]
            })),
            weight: (1. - t) * self.weight + t * other.weight,
        }
    }
}

impl TryFrom<&[f64]> for Point3 {
    type Error = crate::Error;
    fn try_from(value: &[f64]) -> Result<Self> {
        Self::try_from_slice(value)
    }
}

impl TryFrom<HomogeneousPoint3> for Point3 {
    type Error = crate::Error;
    fn try_from(value: HomogeneousPoint3) -> Result<Self> {
        value.project()
    }
}

/// Evaluate a polynomial Bézier curve by the iterative de Casteljau triangle.
/// The control polygon must contain 2..=26 finite points and `t` must be in
/// `[0, 1]`. Each level uses convex interpolation, so no powers or Bernstein
/// coefficient table are formed.
pub fn de_casteljau(control_points: &[Point3], t: f64) -> Result<Point3> {
    check(
        (2..=26).contains(&control_points.len()),
        "Bezier de Casteljau requires 2..26 control points",
    )?;
    check(
        t.is_finite() && (0. ..=1.).contains(&t),
        "Bezier parameter must be finite and inside [0, 1]",
    )?;
    let mut work = control_points.to_vec();
    check(
        work.iter().flat_map(|p| p.0).all(|x| x.is_finite()),
        "Bezier control points must be finite",
    )?;
    for remaining in (2..=work.len()).rev() {
        for i in 0..remaining - 1 {
            work[i] = work[i] + (work[i + 1] - work[i]) * t;
        }
    }
    numeric(
        work[0].0.iter().all(|x| x.is_finite()),
        "Bezier evaluation exhausted finite precision",
    )?;
    Ok(work[0])
}

/// Rational counterpart of [`de_casteljau`]. All triangle levels remain in
/// homogeneous space; exactly one projective division is performed at the end.
pub fn de_casteljau_homogeneous(control_points: &[HomogeneousPoint3], t: f64) -> Result<Point3> {
    check(
        (2..=26).contains(&control_points.len()),
        "Rational Bezier de Casteljau requires 2..26 control points",
    )?;
    check(
        t.is_finite() && (0. ..=1.).contains(&t),
        "Bezier parameter must be finite and inside [0, 1]",
    )?;
    check(
        control_points
            .iter()
            .flat_map(|p| p.coordinates())
            .all(|x| x.is_finite()),
        "Homogeneous Bezier controls must be finite",
    )?;
    let mut work = control_points.to_vec();
    for remaining in (2..=work.len()).rev() {
        for i in 0..remaining - 1 {
            work[i] = work[i].lerp(work[i + 1], t);
        }
    }
    work[0].project()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_vector_and_homogeneous_contracts_are_explicit() {
        let a = Point3::new([1., 2., 3.]).unwrap();
        let b = Point3::new([4., 6., 3.]).unwrap();
        assert_eq!((b - a).0, [3., 4., 0.]);
        assert_eq!((b - a).norm(), 5.);
        assert_eq!((a + Vector3([2., -1., 4.])).0, [3., 1., 7.]);
        assert_eq!((a - Vector3([2., -1., 4.])).0, [-1., 3., -1.]);
        let h = a.weighted(2.5).unwrap();
        assert_eq!(h.coordinates(), [2.5, 5., 7.5, 2.5]);
        assert_eq!(h.project().unwrap(), a);
        assert!(Point3::new([f64::NAN, 0., 0.]).is_err());
        assert!(a.weighted(f64::INFINITY).is_err());
        assert!(a.weighted(0.).is_err());
    }

    #[test]
    fn de_casteljau_matches_polynomial_and_rational_reference() {
        let controls = [
            Point3([0., 0., 0.]),
            Point3([1., 2., 0.]),
            Point3([3., 2., 1.]),
            Point3([4., 0., 1.]),
        ];
        let t = 0.37;
        let q = de_casteljau(&controls, t).unwrap();
        let s = 1. - t;
        let expected = controls[0].0.map(|x| x * s * s * s);
        let expected: [f64; 3] = core::array::from_fn(|i| {
            expected[i]
                + 3. * t * s * s * controls[1].0[i]
                + 3. * t * t * s * controls[2].0[i]
                + t * t * t * controls[3].0[i]
        });
        assert!(q.0.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-14));

        let weights = [1., 2., 0.5, 1.5];
        let homogeneous: Vec<_> = controls
            .iter()
            .zip(weights)
            .map(|(&p, w)| p.weighted(w).unwrap())
            .collect();
        let q = de_casteljau_homogeneous(&homogeneous, t).unwrap();
        let basis = [s * s * s, 3. * t * s * s, 3. * t * t * s, t * t * t];
        let denominator: f64 = basis.iter().zip(weights).map(|(b, w)| b * w).sum();
        let expected: [f64; 3] = core::array::from_fn(|i| {
            basis
                .iter()
                .zip(weights)
                .zip(controls)
                .map(|((b, w), p)| b * w * p.0[i])
                .sum::<f64>()
                / denominator
        });
        assert!(q.0.iter().zip(expected).all(|(a, b)| (a - b).abs() < 1e-14));
        assert!(de_casteljau(&controls, f64::NAN).is_err());
        assert!(de_casteljau(&controls, -0.1).is_err());
    }

    #[test]
    fn curve_method_agrees_with_basis_and_de_boor_on_rational_bezier() {
        let curve = crate::paths::bezier(
            vec![
                vec![0., 0., 0.],
                vec![1., 2., 0.],
                vec![3., 2., 1.],
                vec![4., 0., 1.],
            ],
            Some(vec![1., 2., 0.5, 1.5]),
        )
        .unwrap();
        for i in 0..=20 {
            let u = i as f64 / 20.;
            let direct = curve.evaluate_bezier_de_casteljau(u).unwrap().0;
            let basis = curve.evaluate(u).unwrap().point;
            let de_boor = curve.evaluate_de_boor(u).unwrap();
            for axis in 0..3 {
                assert!((direct[axis] - basis[axis]).abs() < 1e-12);
                assert!((direct[axis] - de_boor[axis]).abs() < 1e-12);
            }
        }
        let line = crate::primitives::line([0.; 3], [1., 0., 0.]).unwrap();
        assert!(line.evaluate_bezier_de_casteljau(0.5).is_ok());
        let multispan =
            crate::primitives::polyline(&[[0., 0., 0.], [1., 0., 0.], [2., 0., 0.]], false)
                .unwrap();
        assert!(multispan.evaluate_bezier_de_casteljau(0.5).is_err());
    }
}
