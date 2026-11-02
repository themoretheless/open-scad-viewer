//! Positive-weight clamped rational B-splines; parameters retain authored units.
use crate::{Error, Result, math::*, require};

#[derive(Clone, Debug, PartialEq)]
pub struct Curve {
    pub(crate) degree: usize,
    pub(crate) knots: Vec<f64>,
    pub(crate) controls: Vec<Point>,
    pub(crate) weights: Vec<f64>,
}
pub(crate) fn basis_valid(degree: usize, knots: &[f64], count: usize) -> Result<()> {
    require(
        (1..=16).contains(&degree) && count > degree && count <= 65536,
        "degree/count outside supported range",
    )?;
    require(
        knots.len() == count + degree + 1
            && knots.iter().all(|x| x.is_finite())
            && knots.windows(2).all(|p| p[0] <= p[1]),
        "invalid knot vector",
    )?;
    let a = knots[degree];
    let b = knots[count];
    require(
        a < b
            && (b - a).is_finite()
            && knots[..=degree].iter().all(|x| *x == a)
            && knots[count..].iter().all(|x| *x == b),
        "finite nonempty clamped domain required",
    )?;
    let mut run = 0;
    let mut previous = f64::NAN;
    for &k in knots {
        if k == previous {
            run += 1
        } else {
            run = 1;
            previous = k
        }
        require(run <= degree + 1, "excess knot multiplicity")?;
    }
    Ok(())
}
pub(crate) fn span(degree: usize, knots: &[f64], count: usize, t: f64) -> Result<usize> {
    require(
        t.is_finite() && t >= knots[degree] && t <= knots[count],
        "parameter outside domain",
    )?;
    if t == knots[count] {
        return Ok(count - 1);
    }
    Ok(knots.partition_point(|x| *x <= t) - 1)
}
pub(crate) fn homogeneous(p: Point, w: f64) -> [f64; 4] {
    [p[0] * w, p[1] * w, p[2] * w, w]
}
pub(crate) fn project(h: [f64; 4]) -> Result<Point> {
    require(
        h.iter().all(|x| x.is_finite()) && h[3] > 0.,
        "invalid homogeneous result",
    )?;
    let p = [h[0] / h[3], h[1] / h[3], h[2] / h[3]];
    require(finite(p), "nonfinite projected point")?;
    Ok(p)
}
pub(crate) fn de_boor(p: usize, knots: &[f64], points: &[[f64; 4]], t: f64) -> Result<[f64; 4]> {
    let k = span(p, knots, points.len(), t)?;
    let mut d = points[k - p..=k].to_vec();
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i = k - p + j;
            let den = knots[i + p - r + 1] - knots[i];
            let a = if den == 0. { 0. } else { (t - knots[i]) / den };
            d[j] = std::array::from_fn(|axis| (1. - a) * d[j - 1][axis] + a * d[j][axis]);
        }
    }
    Ok(d[p])
}
pub(crate) fn homogeneous_derivative(
    p: usize,
    knots: &[f64],
    points: &[[f64; 4]],
    t: f64,
    order: usize,
) -> Result<[f64; 4]> {
    if order == 0 {
        return de_boor(p, knots, points, t);
    }
    if order > p {
        return Ok([0.; 4]);
    }
    let controls = points
        .windows(2)
        .enumerate()
        .map(|(i, pair)| {
            let width = knots[i + p + 1] - knots[i + 1];
            if width == 0. {
                [0.; 4]
            } else {
                std::array::from_fn(|d| p as f64 * (pair[1][d] - pair[0][d]) / width)
            }
        })
        .collect::<Vec<_>>();
    homogeneous_derivative(p - 1, &knots[1..knots.len() - 1], &controls, t, order - 1)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Differential {
    pub point: Point,
    pub first: Point,
    pub second: Point,
}
impl Differential {
    pub fn tangent(&self) -> Result<Point> {
        unit(self.first)
    }
    pub fn curvature(&self) -> Result<f64> {
        let speed = norm(self.first);
        require(speed > 0., "curvature at stationary curve point")?;
        let result = norm(cross(self.first, self.second)) / speed.powi(3);
        require(result.is_finite(), "curvature overflow")?;
        Ok(result)
    }
}
impl Curve {
    /// Rational derivatives. Right-hand branch at internal knots, left at end.
    pub fn differential(&self, t: f64) -> Result<Differential> {
        let h = self.hpoints();
        let value = de_boor(self.degree, &self.knots, &h, t)?;
        let point = project(value)?;
        let d = homogeneous_derivative(self.degree, &self.knots, &h, t, 1)?;
        let dd = homogeneous_derivative(self.degree, &self.knots, &h, t, 2)?;
        let first = std::array::from_fn(|i| (d[i] - point[i] * d[3]) / value[3]);
        let second =
            std::array::from_fn(|i| (dd[i] - 2. * first[i] * d[3] - point[i] * dd[3]) / value[3]);
        require(finite(first) && finite(second), "curve derivative overflow")?;
        Ok(Differential {
            point,
            first,
            second,
        })
    }
    pub fn new(
        degree: usize,
        knots: Vec<f64>,
        controls: Vec<Point>,
        weights: Vec<f64>,
    ) -> Result<Self> {
        basis_valid(degree, &knots, controls.len())?;
        require(
            weights.len() == controls.len()
                && weights.iter().all(|w| w.is_finite() && *w > 0.)
                && controls.iter().copied().all(finite),
            "finite controls and positive weights required",
        )?;
        require(
            controls
                .iter()
                .zip(&weights)
                .all(|(&p, &w)| homogeneous(p, w).iter().all(|v| v.is_finite())),
            "homogeneous controls overflow",
        )?;
        Ok(Self {
            degree,
            knots,
            controls,
            weights,
        })
    }
    pub fn degree(&self) -> usize {
        self.degree
    }
    pub fn knots(&self) -> &[f64] {
        &self.knots
    }
    pub fn controls(&self) -> &[Point] {
        &self.controls
    }
    pub fn weights(&self) -> &[f64] {
        &self.weights
    }
    pub fn domain(&self) -> [f64; 2] {
        [self.knots[self.degree], self.knots[self.controls.len()]]
    }
    pub(crate) fn hpoints(&self) -> Vec<[f64; 4]> {
        self.controls
            .iter()
            .zip(&self.weights)
            .map(|(&p, &w)| homogeneous(p, w))
            .collect()
    }
    pub fn evaluate(&self, t: f64) -> Result<Point> {
        project(de_boor(self.degree, &self.knots, &self.hpoints(), t)?)
    }
    pub fn line(a: Point, b: Point) -> Result<Self> {
        require(a != b, "distinct endpoints required")?;
        Self::polyline(&[a, b])
    }
    pub fn polyline(points: &[Point]) -> Result<Self> {
        require(
            points.len() >= 2 && points.len() <= 65536 && points.windows(2).all(|s| s[0] != s[1]),
            "nondegenerate polyline required",
        )?;
        let n = points.len();
        let mut knots = vec![0., 0.];
        knots.extend((1..n - 1).map(|i| i as f64 / (n - 1) as f64));
        knots.extend([1., 1.]);
        Self::new(1, knots, points.to_vec(), vec![1.; n])
    }
    pub fn bezier(points: &[Point]) -> Result<Self> {
        let p = points
            .len()
            .checked_sub(1)
            .ok_or(Error::InvalidInput("empty Bezier"))?;
        require((1..=16).contains(&p), "Bezier degree must be 1..16")?;
        let mut k = vec![0.; p + 1];
        k.extend(vec![1.; p + 1]);
        Self::new(p, k, points.to_vec(), vec![1.; points.len()])
    }
    /// Rational quadratic conic in the XY plane; signed sweep in radians.
    pub fn ellipse_arc(center: Point, radii: [f64; 2], start: f64, sweep: f64) -> Result<Self> {
        require(
            finite(center)
                && radii.iter().all(|r| r.is_finite() && *r > 0.)
                && start.is_finite()
                && sweep.is_finite()
                && sweep != 0.
                && sweep.abs() <= std::f64::consts::TAU,
            "valid conic arguments required",
        )?;
        let n = (sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize;
        let delta = sweep / n as f64;
        let mut controls = Vec::new();
        let mut weights = Vec::new();
        let mut knots = vec![0.; 3];
        let point = |a: f64, w: f64| {
            [
                center[0] + radii[0] * a.cos() / w,
                center[1] + radii[1] * a.sin() / w,
                center[2],
            ]
        };
        for i in 0..n {
            let a = start + i as f64 * delta;
            let b = a + delta;
            let w = (delta / 2.).cos();
            if i == 0 {
                controls.push(point(a, 1.));
                weights.push(1.);
            }
            controls.extend([point((a + b) / 2., w), point(b, 1.)]);
            weights.extend([w, 1.]);
            if i + 1 < n {
                knots.extend([((i + 1) as f64) / n as f64; 2]);
            }
        }
        knots.extend([1.; 3]);
        Self::new(2, knots, controls, weights)
    }
    pub fn circle(center: Point, radius: f64) -> Result<Self> {
        Self::ellipse_arc(center, [radius; 2], 0., std::f64::consts::TAU)
    }
    /// Piecewise linear approximation. No certified error bound.
    pub fn helix(radius: f64, pitch: f64, turns: f64, segments: usize) -> Result<Self> {
        require(
            radius.is_finite()
                && radius > 0.
                && pitch.is_finite()
                && turns.is_finite()
                && turns != 0.
                && (1..65536).contains(&segments),
            "invalid helix arguments",
        )?;
        Self::polyline(
            &(0..=segments)
                .map(|i| {
                    let t = i as f64 / segments as f64;
                    let a = t * turns * std::f64::consts::TAU;
                    [radius * a.cos(), radius * a.sin(), pitch * turns * t]
                })
                .collect::<Vec<_>>(),
        )
    }
    pub fn transformed(&self, t: &Affine) -> Result<Self> {
        Self::new(
            self.degree,
            self.knots.clone(),
            self.controls.iter().map(|p| t.apply(*p)).collect(),
            self.weights.clone(),
        )
    }
    pub fn reversed(&self) -> Result<Self> {
        let [a, b] = self.domain();
        let mut points = self.controls.clone();
        points.reverse();
        let mut weights = self.weights.clone();
        weights.reverse();
        Self::new(
            self.degree,
            self.knots.iter().rev().map(|u| a + (b - u)).collect(),
            points,
            weights,
        )
    }
    /// One shape-preserving knot insertion, in homogeneous coordinates.
    pub fn insert_knot(&self, t: f64) -> Result<Self> {
        let [a, b] = self.domain();
        require(t > a && t < b, "insert an interior knot")?;
        let p = self.degree;
        let k = span(p, &self.knots, self.controls.len(), t)?;
        let s = self.knots.iter().filter(|u| **u == t).count();
        require(s < p, "interior multiplicity already reaches degree")?;
        let old = self.hpoints();
        let n = old.len();
        let mut q = vec![[0.; 4]; n + 1];
        q[..=k - p].copy_from_slice(&old[..=k - p]);
        q[k - s + 1..].copy_from_slice(&old[k - s..]);
        for i in k - p + 1..=k - s {
            let alpha = (t - self.knots[i]) / (self.knots[i + p] - self.knots[i]);
            q[i] = std::array::from_fn(|j| alpha * old[i][j] + (1. - alpha) * old[i - 1][j]);
        }
        let mut knots = self.knots.clone();
        knots.insert(k + 1, t);
        Self::new(
            p,
            knots,
            q.iter().copied().map(project).collect::<Result<_>>()?,
            q.iter().map(|h| h[3]).collect(),
        )
    }
    pub fn split(&self, t: f64) -> Result<[Self; 2]> {
        let [a, b] = self.domain();
        require(t > a && t < b, "split at interior parameter")?;
        let mut c = self.clone();
        while c.knots.iter().filter(|u| **u == t).count() < c.degree {
            c = c.insert_knot(t)?;
        }
        let k = span(c.degree, &c.knots, c.controls.len(), t)?;
        let cut = k - c.degree;
        let mut left = c.knots[..=k].to_vec();
        left.push(t);
        let mut right = vec![t];
        right.extend_from_slice(&c.knots[k - c.degree + 1..]);
        Ok([
            Self::new(
                c.degree,
                left,
                c.controls[..=cut].to_vec(),
                c.weights[..=cut].to_vec(),
            )?,
            Self::new(
                c.degree,
                right,
                c.controls[cut..].to_vec(),
                c.weights[cut..].to_vec(),
            )?,
        ])
    }
    pub fn trim(&self, a: f64, b: f64) -> Result<Self> {
        let [lo, hi] = self.domain();
        require(a >= lo && b <= hi && a < b, "invalid trim interval")?;
        let c = if b < hi {
            self.split(b)?[0].clone()
        } else {
            self.clone()
        };
        if a > lo {
            Ok(c.split(a)?[1].clone())
        } else {
            Ok(c)
        }
    }
    pub fn sample(&self, segments: usize) -> Result<Vec<Point>> {
        require(
            (1..=1_000_000).contains(&segments),
            "sampling segments must be 1..1000000",
        )?;
        let [a, b] = self.domain();
        (0..=segments)
            .map(|i| {
                self.evaluate(if i == segments {
                    b
                } else {
                    a + (b - a) * i as f64 / segments as f64
                })
            })
            .collect()
    }
    pub fn sampled_length(&self, segments: usize) -> Result<f64> {
        Ok(self
            .sample(segments)?
            .windows(2)
            .map(|s| norm(sub(s[1], s[0])))
            .sum())
    }
}
