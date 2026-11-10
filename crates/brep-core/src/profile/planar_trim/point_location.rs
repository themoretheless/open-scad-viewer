use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocation {
    Outside,
    Inside,
    Boundary,
}

pub(super) fn unsupported(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_PLANAR_TRIM", message)
}
pub(super) fn ambiguous(message: &str) -> Error {
    Error::new("BREP_AMBIGUOUS_PLANAR_TRIM", message)
}
pub(super) fn invalid(message: &str) -> Error {
    Error::new("BREP_INVALID_PLANAR_TRIM", message)
}
pub(super) fn limit() -> Error {
    Error::new(
        "BREP_RESOURCE_LIMIT",
        "Planar trim exceeds 256 analytic spans or 2048 fragments",
    )
}
pub(super) fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
pub(super) fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
pub(super) fn mul(a: Point, s: f64) -> Point {
    [a[0] * s, a[1] * s]
}
pub(super) fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
pub(super) fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
pub(super) fn norm(a: Point) -> f64 {
    a[0].hypot(a[1])
}
pub(super) fn distance(a: Point, b: Point) -> f64 {
    norm(sub(a, b))
}
pub(super) fn left(a: Point) -> Point {
    [-a[1], a[0]]
}
pub(super) fn point(curve: &Curve, t: f64) -> Result<Point> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1]])
}

#[derive(Clone, Debug)]
pub(super) enum Carrier {
    Line,
    Circle {
        center: Point,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}
#[derive(Clone, Debug)]
pub(super) struct Span {
    pub(super) curve: Curve,
    // This translated definition is only for numerical carrier/query work.
    // Output always trims/reverses `curve`, never translates this copy back.
    pub(super) local_curve: Curve,
    pub(super) p0: Point,
    pub(super) p1: Point,
    pub(super) carrier: Carrier,
    pub(super) loop_id: usize,
}
impl Span {
    pub(super) fn from_curve(source: Curve, loop_id: usize, epsilon: f64, origin: Point) -> Result<Self> {
        let mut curve = source.clone();
        for p in &mut curve.control_points {
            p[0] -= origin[0];
            p[1] -= origin[1];
        }
        let domain = curve.domain();
        let p0 = point(&curve, domain[0])?;
        let p1 = point(&curve, domain[1])?;
        if distance(p0, p1) <= epsilon * 4. {
            return Err(ambiguous(
                "A trim span has coincident or unresolved endpoints",
            ));
        }
        let carrier = if curve.degree == 1 && curve.control_points.len() == 2 {
            Carrier::Line
        } else if curve.degree == 2 && curve.control_points.len() == 3 {
            let middle = [curve.control_points[1][0], curve.control_points[1][1]];
            let a = sub(middle, p0);
            let b = sub(p1, middle);
            let determinant = cross(a, b);
            if determinant.abs() <= 1e-12 * norm(a) * norm(b) {
                return Err(unsupported(
                    "Quadratic trim is not a conditioned circular arc",
                ));
            }
            let center = add(p0, mul(left(a), dot(sub(p1, p0), b) / determinant));
            let radius = distance(center, p0);
            if !radius.is_finite() || radius <= epsilon {
                return Err(unsupported("Invalid circular trim radius"));
            }
            // The homogeneous circle equation is a degree-four Bernstein
            // polynomial. All five coefficients must vanish to roundoff; a
            // vaguely circle-like sampled quadratic is not accepted.
            let maximum = curve.weights.iter().copied().fold(0., f64::max);
            let mut coefficients = [0.; 5];
            let mut magnitudes = [0.; 5];
            let choose2 = [1., 2., 1.];
            let choose4 = [1., 4., 6., 4., 1.];
            for i in 0..3 {
                for j in 0..3 {
                    let pi = mul(
                        sub(
                            [curve.control_points[i][0], curve.control_points[i][1]],
                            center,
                        ),
                        1. / radius,
                    );
                    let pj = mul(
                        sub(
                            [curve.control_points[j][0], curve.control_points[j][1]],
                            center,
                        ),
                        1. / radius,
                    );
                    let weight = curve.weights[i] / maximum * curve.weights[j] / maximum
                        * choose2[i]
                        * choose2[j]
                        / choose4[i + j];
                    coefficients[i + j] += weight * (dot(pi, pj) - 1.);
                    magnitudes[i + j] +=
                        weight * (pi[0].abs() * pj[0].abs() + pi[1].abs() * pj[1].abs() + 1.);
                }
            }
            if coefficients
                .iter()
                .zip(magnitudes)
                .any(|(v, m)| v.abs() > 2048. * f64::EPSILON * m.max(1.))
            {
                return Err(unsupported(
                    "Quadratic trim does not satisfy the rational circle identity",
                ));
            }
            let start = (p0[1] - center[1]).atan2(p0[0] - center[0]);
            let end = (p1[1] - center[1]).atan2(p1[0] - center[0]);
            let sweep = if determinant > 0. {
                (end - start).rem_euclid(TAU)
            } else {
                -((start - end).rem_euclid(TAU))
            };
            if sweep.abs() >= PI - 1e-10 || sweep.abs() * radius <= epsilon * 4. {
                return Err(unsupported(
                    "Circular spans must sweep strictly less than 180 degrees",
                ));
            }
            Carrier::Circle {
                center,
                radius,
                start,
                sweep,
            }
        } else {
            return Err(unsupported(
                "Planar trims admit degree-one lines and rational quadratic circular arcs",
            ));
        };
        Ok(Self {
            curve: source,
            local_curve: curve,
            p0,
            p1,
            carrier,
            loop_id,
        })
    }
    pub(super) fn position(&self, fraction: f64) -> Result<Point> {
        let d = self.curve.domain();
        point(&self.local_curve, d[0] + fraction * (d[1] - d[0]))
    }
    pub(super) fn tangent(&self, fraction: f64) -> Result<Point> {
        match self.carrier {
            Carrier::Line => Ok(mul(sub(self.p1, self.p0), 1. / distance(self.p1, self.p0))),
            Carrier::Circle {
                center,
                radius,
                sweep,
                ..
            } => Ok(mul(
                left(sub(self.position(fraction)?, center)),
                sweep.signum() / radius,
            )),
        }
    }
    pub(super) fn arc_fraction(&self, p: Point, epsilon: f64) -> Option<f64> {
        let Carrier::Circle {
            center,
            radius,
            start,
            sweep,
        } = self.carrier
        else {
            return None;
        };
        let angle = (p[1] - center[1]).atan2(p[0] - center[0]);
        let progress = if sweep > 0. {
            (angle - start).rem_euclid(TAU)
        } else {
            (start - angle).rem_euclid(TAU)
        };
        if distance(p, self.p0) <= epsilon {
            Some(0.)
        } else if distance(p, self.p1) <= epsilon {
            Some(1.)
        } else if progress <= sweep.abs() + epsilon / radius {
            Some((progress / sweep.abs()).clamp(0., 1.))
        } else {
            None
        }
    }
    pub(super) fn contains(&self, p: Point, epsilon: f64) -> bool {
        match self.carrier {
            Carrier::Line => {
                let d = sub(self.p1, self.p0);
                let length = norm(d);
                let s = dot(sub(p, self.p0), d) / length;
                cross(sub(p, self.p0), d).abs() / length <= epsilon
                    && s >= -epsilon
                    && s <= length + epsilon
            }
            Carrier::Circle { center, radius, .. } => {
                (distance(p, center) - radius).abs() <= epsilon
                    && self.arc_fraction(p, epsilon).is_some()
            }
        }
    }
    /// Recover the parameter on the retained source. A monotone angular
    /// parameter bracket is used only after the analytic carrier intersection.
    pub(super) fn parameter(&self, p: Point, epsilon: f64) -> Result<f64> {
        let domain = self.curve.domain();
        if distance(p, self.p0) <= epsilon {
            return Ok(domain[0]);
        }
        if distance(p, self.p1) <= epsilon {
            return Ok(domain[1]);
        }
        let fraction = match self.carrier {
            Carrier::Line => {
                let delta = sub(self.p1, self.p0);
                let lambda = dot(sub(p, self.p0), delta) / dot(delta, delta);
                let a = self.curve.weights[0];
                let b = self.curve.weights[1];
                lambda * a / (b * (1. - lambda) + lambda * a)
            }
            Carrier::Circle { .. } => {
                let target = self
                    .arc_fraction(p, epsilon)
                    .ok_or_else(|| ambiguous("Circle event left its source arc"))?;
                let (mut lo, mut hi) = (0., 1.);
                for _ in 0..56 {
                    let mid = (lo + hi) * 0.5;
                    let progress = self
                        .arc_fraction(self.position(mid)?, epsilon * 0.01)
                        .ok_or_else(|| ambiguous("Unresolved circular parameter inversion"))?;
                    if progress < target {
                        lo = mid
                    } else {
                        hi = mid
                    }
                }
                (lo + hi) * 0.5
            }
        };
        if !fraction.is_finite() || fraction < 0. || fraction > 1. {
            return Err(ambiguous(
                "Intersection parameter is outside its source span",
            ));
        }
        let parameter = domain[0] + fraction * (domain[1] - domain[0]);
        if distance(point(&self.local_curve, parameter)?, p) > epsilon * 8. {
            return Err(ambiguous(
                "Intersection parameter residual exceeds the numerical bound",
            ));
        }
        Ok(parameter)
    }
    pub(super) fn area(&self, origin: Point) -> f64 {
        match self.carrier {
            Carrier::Line => cross(sub(self.p0, origin), sub(self.p1, origin)) * 0.5,
            Carrier::Circle {
                center,
                radius,
                sweep,
                ..
            } => {
                0.5 * (cross(sub(center, origin), sub(self.p1, self.p0)) + radius * radius * sweep)
            }
        }
    }
}

pub(super) struct Region {
    pub(super) spans: Vec<Span>,
    pub(super) loops: Vec<Vec<usize>>,
    pub(super) epsilon: f64,
    pub(super) tolerance: f64,
    pub(super) origin: Point,
    /// Construction/representation error at a declared loop junction. This is
    /// never used to identify coincident carriers or classify material sides.
    pub(super) endpoint_error: f64,
}
pub(super) fn region_origin(loops: &[Vec<Curve>]) -> Point {
    loops
        .iter()
        .flatten()
        .find_map(|c| {
            c.control_points
                .first()
                .filter(|p| p.len() == 2)
                .map(|p| [p[0], p[1]])
        })
        .unwrap_or([0., 0.])
}
