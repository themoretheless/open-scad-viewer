use crate::{Error, Result, require};
pub type Point = [f64; 3];
pub fn add(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] + b[i])
}
pub fn sub(a: Point, b: Point) -> Point {
    std::array::from_fn(|i| a[i] - b[i])
}
pub fn scale(a: Point, s: f64) -> Point {
    a.map(|v| v * s)
}
pub fn dot(a: Point, b: Point) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
pub fn cross(a: Point, b: Point) -> Point {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
pub fn norm(a: Point) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
pub fn unit(a: Point) -> Result<Point> {
    let n = norm(a);
    if n.is_finite() && n > 0. {
        Ok(scale(a, 1. / n))
    } else {
        Err(Error::Degenerate("zero or nonfinite direction"))
    }
}
pub fn finite(a: Point) -> bool {
    a.iter().all(|v| v.is_finite())
}
pub fn lerp(a: Point, b: Point, t: f64) -> Point {
    add(scale(a, 1. - t), scale(b, t))
}
pub fn rotate_vector(p: Point, axis: Point, angle: f64) -> Result<Point> {
    require(finite(p) && angle.is_finite(), "finite rotation required")?;
    let a = unit(axis)?;
    let (s, c) = angle.sin_cos();
    Ok(add(
        add(scale(p, c), scale(cross(a, p), s)),
        scale(a, dot(a, p) * (1. - c)),
    ))
}
pub fn frame(normal: Point) -> Result<(Point, Point)> {
    let n = unit(normal)?;
    let seed = if n[0].abs() <= n[1].abs() && n[0].abs() <= n[2].abs() {
        [1., 0., 0.]
    } else if n[1].abs() <= n[2].abs() {
        [0., 1., 0.]
    } else {
        [0., 0., 1.]
    };
    let u = unit(cross(seed, n))?;
    Ok((u, cross(n, u)))
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    linear: [[f64; 3]; 3],
    translation: Point,
}
impl Affine {
    pub fn new(linear: [[f64; 3]; 3], translation: Point) -> Result<Self> {
        require(
            linear.iter().flatten().all(|x| x.is_finite()) && finite(translation),
            "finite affine transform required",
        )?;
        let value = Self {
            linear,
            translation,
        };
        require(
            value.determinant().is_finite() && value.determinant() != 0.,
            "nonsingular affine transform required",
        )?;
        Ok(value)
    }
    pub fn identity() -> Self {
        Self {
            linear: [[1., 0., 0.], [0., 1., 0.], [0., 0., 1.]],
            translation: [0.; 3],
        }
    }
    pub fn translation(offset: Point) -> Result<Self> {
        Self::new(Self::identity().linear, offset)
    }
    pub fn scaling(factors: Point) -> Result<Self> {
        Self::new(
            [
                [factors[0], 0., 0.],
                [0., factors[1], 0.],
                [0., 0., factors[2]],
            ],
            [0.; 3],
        )
    }
    pub fn rotation(axis: Point, angle: f64) -> Result<Self> {
        let cols = [
            rotate_vector([1., 0., 0.], axis, angle)?,
            rotate_vector([0., 1., 0.], axis, angle)?,
            rotate_vector([0., 0., 1.], axis, angle)?,
        ];
        Self::new(
            std::array::from_fn(|i| std::array::from_fn(|j| cols[j][i])),
            [0.; 3],
        )
    }
    pub fn determinant(&self) -> f64 {
        dot(self.linear[0], cross(self.linear[1], self.linear[2]))
    }
    pub fn apply(&self, p: Point) -> Point {
        std::array::from_fn(|i| dot(self.linear[i], p) + self.translation[i])
    }
    /// Apply self, then next.
    pub fn then(&self, next: &Self) -> Result<Self> {
        let linear = std::array::from_fn(|i| {
            std::array::from_fn(|j| (0..3).map(|k| next.linear[i][k] * self.linear[k][j]).sum())
        });
        Self::new(linear, next.apply(self.translation))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}
impl Bounds {
    pub fn new(min: Point, max: Point) -> Result<Self> {
        require(
            finite(min) && finite(max) && (0..3).all(|i| min[i] < max[i]),
            "bounds require finite strictly increasing axes",
        )?;
        Ok(Self { min, max })
    }
    pub fn contains(&self, p: Point) -> bool {
        finite(p) && (0..3).all(|i| p[i] >= self.min[i] && p[i] <= self.max[i])
    }
    pub fn volume(&self) -> f64 {
        (0..3).map(|i| self.max[i] - self.min[i]).product()
    }
}
