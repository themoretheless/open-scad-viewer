use crate::{Result, check, numeric, numeric_err, resource};

#[derive(Clone, Debug)]
pub struct Curve {
    pub degree: usize,
    pub knots: Vec<f64>,
    pub control_points: Vec<Vec<f64>>,
    pub weights: Vec<f64>,
    pub periodic: bool,
}
impl value_codec::Serialize for Curve {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "degree".into(),
            value_codec::Serialize::to_value(&self.degree),
        );
        object.insert(
            "knots".into(),
            value_codec::Serialize::to_value(&self.knots),
        );
        object.insert(
            "controlPoints".into(),
            value_codec::Serialize::to_value(&self.control_points),
        );
        object.insert(
            "weights".into(),
            value_codec::Serialize::to_value(&self.weights),
        );
        object.insert(
            "periodic".into(),
            value_codec::Serialize::to_value(&self.periodic),
        );
        value_codec::Value::Object(object)
    }
}
impl<'de> value_codec::Deserialize<'de> for Curve {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let mut object = value
            .as_object()
            .ok_or_else(|| value_codec::error("Expected object"))?
            .clone();
        let degree: usize = value_codec::Deserialize::from_value(
            object
                .remove("degree")
                .ok_or_else(|| value_codec::error("Missing field degree"))?,
        )?;
        let knots: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("knots")
                .ok_or_else(|| value_codec::error("Missing field knots"))?,
        )?;
        let control_points: Vec<Vec<f64>> = value_codec::Deserialize::from_value(
            object
                .remove("controlPoints")
                .ok_or_else(|| value_codec::error("Missing field controlPoints"))?,
        )?;
        let weights: Vec<f64> = value_codec::Deserialize::from_value(
            object
                .remove("weights")
                .ok_or_else(|| value_codec::error("Missing field weights"))?,
        )?;
        let periodic: bool = if let Some(v) = object.remove("periodic") {
            value_codec::Deserialize::from_value(v)?
        } else {
            Default::default()
        };
        Ok(Self {
            degree,
            knots,
            control_points,
            weights,
            periodic,
        })
    }
}
#[derive(Debug)]
pub struct Basis {
    pub basis: Vec<f64>,
    pub d1: Vec<f64>,
    pub d2: Vec<f64>,
    pub domain: [f64; 2],
    pub continuity: Option<i32>,
    pub derivative_status: &'static str,
    pub derivative_side: &'static str,
}
impl value_codec::Serialize for Basis {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "basis".into(),
            value_codec::Serialize::to_value(&self.basis),
        );
        object.insert("d1".into(), value_codec::Serialize::to_value(&self.d1));
        object.insert("d2".into(), value_codec::Serialize::to_value(&self.d2));
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        object.insert(
            "continuity".into(),
            value_codec::Serialize::to_value(&self.continuity),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side".into(),
            value_codec::Serialize::to_value(&self.derivative_side),
        );
        value_codec::Value::Object(object)
    }
}
#[derive(Debug)]
pub struct Evaluation {
    pub point: Vec<f64>,
    pub d1: Option<Vec<f64>>,
    pub d2: Option<Vec<f64>>,
    pub domain: [f64; 2],
    pub continuity: Option<i32>,
    pub derivative_status: &'static str,
    pub derivative_side: &'static str,
}
impl value_codec::Serialize for Evaluation {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "point".into(),
            value_codec::Serialize::to_value(&self.point),
        );
        object.insert("d1".into(), value_codec::Serialize::to_value(&self.d1));
        object.insert("d2".into(), value_codec::Serialize::to_value(&self.d2));
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        object.insert(
            "continuity".into(),
            value_codec::Serialize::to_value(&self.continuity),
        );
        object.insert(
            "derivative_status".into(),
            value_codec::Serialize::to_value(&self.derivative_status),
        );
        object.insert(
            "derivative_side".into(),
            value_codec::Serialize::to_value(&self.derivative_side),
        );
        value_codec::Value::Object(object)
    }
}
fn budget(count: usize) -> Result<()> {
    if count > 256 {
        return Err(resource("The result exceeds 256 control points"));
    }
    Ok(())
}
pub fn validate_basis(p: usize, k: &[f64], n: usize) -> Result<[f64; 2]> {
    check(
        (1..=25).contains(&p),
        "Degree must be an integer in [1, 25]",
    )?;
    check(
        n > p && n <= 256,
        "Control point count must be between degree+1 and 256",
    )?;
    check(
        k.len() == n + p + 1,
        "Expanded knot count must equal control point count + degree + 1",
    )?;
    let mut multiplicity = 0;
    for i in 0..k.len() {
        check(
            k[i].is_finite() && k[i].abs() <= 1e9,
            "Knots must be finite and bounded by 1e9",
        )?;
        check(i == 0 || k[i] >= k[i - 1], "Knots must be nondecreasing")?;
        multiplicity = if i > 0 && k[i] == k[i - 1] {
            multiplicity + 1
        } else {
            1
        };
        check(
            multiplicity <= p + 1,
            "Knot multiplicity must not exceed degree+1",
        )?;
    }
    let domain = [k[p], k[n]];
    check(
        domain[0] < domain[1],
        "The active knot domain must have positive length",
    )?;
    let mut i = 0;
    while i < k.len() {
        let mut end = i + 1;
        while end < k.len() && k[end] == k[i] {
            end += 1;
        }
        check(
            k[i] <= domain[0] || k[i] >= domain[1] || end - i <= p,
            "Interior multiplicity must not exceed degree; disconnected curves need separate nodes",
        )?;
        i = end;
    }
    Ok(domain)
}
impl Curve {
    /// Exact degree-one representation of a 2D or 3D polyline. Repeating the
    /// first point closes it; this does not create periodic spline storage.
    pub fn from_polyline(points: Vec<Vec<f64>>) -> Result<Self> {
        check(points.len() >= 2, "A polyline needs at least two points")?;
        budget(points.len())?;
        let count = points.len();
        let mut knots = vec![0.];
        knots.extend((0..count).map(|i| i as f64));
        knots.push((count - 1) as f64);
        let curve = Self {
            degree: 1,
            knots,
            control_points: points,
            weights: vec![1.; count],
            periodic: false,
        };
        curve.validate()?;
        Ok(curve)
    }
    pub fn domain(&self) -> [f64; 2] {
        [
            self.knots[self.degree],
            self.knots[self.control_points.len()],
        ]
    }
    pub fn validate(&self) -> Result<()> {
        let n = self.control_points.len();
        let domain = validate_basis(self.degree, &self.knots, n)?;
        let dim = self.control_points[0].len();
        check(
            dim == 2 || dim == 3,
            "Control points must have two or three coordinates",
        )?;
        check(
            self.control_points
                .iter()
                .all(|p| p.len() == dim && p.iter().all(|x| x.is_finite() && x.abs() <= 1e9)),
            "Control points must have consistent dimensions and finite coordinates bounded by 1e9",
        )?;
        check(
            self.weights.len() == n,
            "Weights must match the control point count",
        )?;
        check(
            self.weights
                .iter()
                .all(|w| w.is_finite() && *w >= 1e-12 && *w <= 1e12),
            "Weights must be positive, finite, and in [1e-12, 1e12]",
        )?;
        let max = self.weights.iter().copied().fold(0., f64::max);
        let min = self.weights.iter().copied().fold(f64::INFINITY, f64::min);
        check(
            max / min <= 1e12,
            "Weight conditioning must not exceed 1e12",
        )?;
        if self.periodic {
            let period_controls = n - self.degree;
            check(
                period_controls > self.degree,
                "A periodic curve needs at least degree+1 unwrapped control points",
            )?;
            for i in 0..self.degree {
                check(
                    self.weights[i] == self.weights[period_controls + i]
                        && self.control_points[i] == self.control_points[period_controls + i],
                    "Periodic curves must explicitly repeat the first degree control points and weights at the end",
                )?;
            }
            let period = domain[1] - domain[0];
            let scale = self.knots.iter().map(|k| k.abs()).fold(0., f64::max);
            for i in 0..self.knots.len() - period_controls {
                let difference = self.knots[i + period_controls] - self.knots[i];
                check(
                    (difference - period).abs()
                        <= 32.
                            * f64::EPSILON
                            * scale
                                .max(difference.abs())
                                .max(period.abs())
                                .max(f64::from_bits(1)),
                    "Periodic exterior knots must repeat with the active period",
                )?;
            }
        }
        Ok(())
    }
    pub fn evaluate(&self, u: f64) -> Result<Evaluation> {
        self.validate()?;
        let b = basis(
            self.degree,
            &self.knots,
            self.control_points.len(),
            u,
            self.periodic,
        )?;
        let dim = self.control_points[0].len();
        let mut point = vec![0.; dim];
        let mut d1 = point.clone();
        let mut d2 = point.clone();
        let scale = self.weights.iter().copied().fold(0., f64::max);
        let index = b
            .basis
            .iter()
            .position(|v| *v > 0.)
            .ok_or_else(|| numeric_err("Empty basis support"))?;
        let origin = &self.control_points[index];
        let (mut weight, mut w1, mut w2) = (0., 0., 0.);
        for i in 0..self.control_points.len() {
            let w = self.weights[i] / scale;
            weight += b.basis[i] * w;
            w1 += b.d1[i] * w;
            w2 += b.d2[i] * w;
            for axis in 0..dim {
                let coordinate = self.control_points[i][axis] - origin[axis];
                point[axis] += b.basis[i] * w * coordinate;
                d1[axis] += b.d1[i] * w * coordinate;
                d2[axis] += b.d2[i] * w * coordinate;
            }
        }
        numeric(
            weight > 0. && weight.is_finite(),
            "Rational denominator lost its positive finite value",
        )?;
        for axis in 0..dim {
            point[axis] /= weight;
            d1[axis] = (d1[axis] - w1 * point[axis]) / weight;
            d2[axis] = (d2[axis] - 2. * w1 * d1[axis] - w2 * point[axis]) / weight;
            point[axis] += origin[axis];
        }
        numeric(
            point.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
            "Rational evaluation exhausted finite precision",
        )?;
        Ok(Evaluation {
            point,
            d1: (b.continuity.unwrap_or(2) >= 1).then_some(d1),
            d2: (b.continuity.unwrap_or(2) >= 2).then_some(d2),
            domain: b.domain,
            continuity: b.continuity,
            derivative_status: b.derivative_status,
            derivative_side: b.derivative_side,
        })
    }
    pub fn insert(&self, u: f64, count: usize) -> Result<Self> {
        self.validate()?;
        check(
            count >= 1 && count <= self.degree + 1,
            "Insertion count must be in [1, degree+1]",
        )?;
        let [a, b] = self.domain();
        check(
            u.is_finite() && u >= a && u <= b,
            "Inserted knot must be inside the active domain",
        )?;
        let base = if self.periodic {
            self.clamp(a, b)?
        } else {
            self.clone()
        };
        let max = if u == a || u == b {
            self.degree + 1
        } else {
            self.degree
        };
        check(
            multiplicity(&base.knots, u) + count <= max,
            "Insertion would exceed the permitted knot multiplicity",
        )?;
        budget(base.control_points.len() + count)?;
        let mut work = Homogeneous::from(&base);
        for _ in 0..count {
            work = work.insert(u)?;
        }
        work.to_curve()
    }
    fn clamp(&self, a: f64, b: f64) -> Result<Self> {
        let mut work = Homogeneous::from(self);
        while multiplicity(&work.knots, a) < self.degree + 1 {
            work = work.insert(a)?;
        }
        while multiplicity(&work.knots, b) < self.degree + 1 {
            work = work.insert(b)?;
        }
        let start = work.knots.iter().position(|v| *v == a).unwrap();
        let end = work.knots.iter().rposition(|v| *v == b).unwrap();
        let knots = work.knots[start..=end].to_vec();
        let count = knots.len() - self.degree - 1;
        Homogeneous {
            degree: self.degree,
            knots,
            controls: work.controls[start..start + count].to_vec(),
        }
        .to_curve()
    }
    pub fn trim(&self, a: f64, b: f64) -> Result<Self> {
        self.validate()?;
        let domain = self.domain();
        check(
            a.is_finite() && b.is_finite() && domain[0] <= a && a < b && b <= domain[1],
            "Trim must be a nonempty ordered subdomain of the active knot domain",
        )?;
        self.clamp(a, b)
    }
    pub fn split(&self, u: f64) -> Result<[Self; 2]> {
        self.validate()?;
        let [a, b] = self.domain();
        check(
            u.is_finite() && a < u && u < b,
            "Split parameter must lie strictly inside the active domain",
        )?;
        Ok([self.trim(a, u)?, self.trim(u, b)?])
    }
    pub fn reverse(&self) -> Result<Self> {
        self.validate()?;
        let [a, b] = self.domain();
        let mut result = self.clone();
        result.knots = self.knots.iter().rev().map(|k| a + b - k).collect();
        result.control_points.reverse();
        result.weights.reverse();
        result.validate()?;
        Ok(result)
    }
    pub fn decompose(&self) -> Result<Vec<Segment>> {
        self.validate()?;
        let [a, b] = self.domain();
        let mut breaks: Vec<f64> = self
            .knots
            .iter()
            .copied()
            .filter(|k| *k >= a && *k <= b)
            .collect();
        breaks.dedup();
        breaks
            .array_windows()
            .map(|[a, b]| {
                Ok(Segment {
                    curve: self.trim(*a, *b)?,
                    domain: [*a, *b],
                })
            })
            .collect()
    }
    pub fn elevate(&self, degree: usize) -> Result<Self> {
        self.validate()?;
        check(
            degree >= self.degree && degree <= 25,
            "Elevation degree must be between the current degree and 25",
        )?;
        if degree == self.degree {
            return Ok(self.clone());
        }
        let [a, b] = self.domain();
        let mut breaks: Vec<f64> = self
            .knots
            .iter()
            .copied()
            .filter(|k| *k >= a && *k <= b)
            .collect();
        breaks.dedup();
        budget((breaks.len() - 1) * degree + 1)?;
        let segments = self.decompose()?;
        let mut controls = Vec::new();
        let mut knots = vec![a; degree + 1];
        for (index, segment) in segments.iter().enumerate() {
            let mut points = Homogeneous::from(&segment.curve).controls;
            for order in self.degree..degree {
                let mut next = vec![points[0].clone()];
                for i in 1..=order {
                    let alpha = i as f64 / (order + 1) as f64;
                    next.push(
                        points[i]
                            .iter()
                            .enumerate()
                            .map(|(axis, x)| alpha * points[i - 1][axis] + (1. - alpha) * x)
                            .collect(),
                    );
                }
                next.push(points.last().unwrap().clone());
                points = next;
            }
            controls.extend_from_slice(&points[usize::from(index != 0)..]);
            knots.extend(vec![
                segment.domain[1];
                if index == segments.len() - 1 {
                    degree + 1
                } else {
                    degree
                }
            ]);
        }
        Homogeneous {
            degree,
            knots,
            controls,
        }
        .to_curve()
    }
    pub fn bounds(&self) -> Result<value_codec::Value> {
        self.validate()?;
        Ok(bounds(&self.control_points))
    }
}
pub fn bounds(points: &[Vec<f64>]) -> value_codec::Value {
    let dim = points[0].len();
    let min: Vec<f64> = (0..dim)
        .map(|a| points.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min))
        .collect();
    let max: Vec<f64> = (0..dim)
        .map(|a| {
            points
                .iter()
                .map(|p| p[a])
                .fold(f64::NEG_INFINITY, f64::max)
        })
        .collect();
    value_codec::json!({"min":min,"max":max})
}

pub struct Segment {
    curve: Curve,
    domain: [f64; 2],
}
impl Segment {
    pub fn definition(&self) -> &Curve {
        &self.curve
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
}
impl value_codec::Serialize for Segment {
    fn to_value(&self) -> value_codec::Value {
        let mut object = value_codec::Map::new();
        object.insert(
            "curve".into(),
            value_codec::Serialize::to_value(&self.curve),
        );
        object.insert(
            "domain".into(),
            value_codec::Serialize::to_value(&self.domain),
        );
        value_codec::Value::Object(object)
    }
}
fn multiplicity(knots: &[f64], u: f64) -> usize {
    knots.iter().filter(|k| **k == u).count()
}
struct Homogeneous {
    degree: usize,
    knots: Vec<f64>,
    controls: Vec<Vec<f64>>,
}
impl From<&Curve> for Homogeneous {
    fn from(c: &Curve) -> Self {
        Self {
            degree: c.degree,
            knots: c.knots.clone(),
            controls: c
                .control_points
                .iter()
                .zip(&c.weights)
                .map(|(p, w)| p.iter().map(|x| x * w).chain(std::iter::once(*w)).collect())
                .collect(),
        }
    }
}
impl Homogeneous {
    fn to_curve(&self) -> Result<Curve> {
        budget(self.controls.len())?;
        let weights: Vec<f64> = self.controls.iter().map(|p| *p.last().unwrap()).collect();
        let c = Curve {
            degree: self.degree,
            knots: self.knots.clone(),
            control_points: self
                .controls
                .iter()
                .zip(&weights)
                .map(|(p, w)| p[..p.len() - 1].iter().map(|x| x / w).collect())
                .collect(),
            weights,
            periodic: false,
        };
        c.validate()?;
        Ok(c)
    }
    fn insert(&self, u: f64) -> Result<Self> {
        let p = self.degree;
        let n = self.controls.len() - 1;
        let s = multiplicity(&self.knots, u);
        check(s <= p, "Requested knot already has degree+1 multiplicity")?;
        let mut k = p;
        while k + 1 < self.knots.len() && self.knots[k + 1] <= u {
            k += 1;
        }
        check(
            k >= p && k - s <= n,
            "Knot insertion has an empty local span",
        )?;
        let mut controls = vec![Vec::new(); n + 2];
        controls[..=k - p].clone_from_slice(&self.controls[..=k - p]);
        controls[k - s + 1..n + 2].clone_from_slice(&self.controls[k - s..n + 1]);
        for (i, control) in controls
            .iter_mut()
            .enumerate()
            .take(k - s + 1)
            .skip(k - p + 1)
        {
            let denominator = self.knots[i + p] - self.knots[i];
            check(denominator > 0., "Knot insertion has an empty local span")?;
            let alpha = (u - self.knots[i]) / denominator;
            *control = self.controls[i]
                .iter()
                .enumerate()
                .map(|(axis, x)| alpha * x + (1. - alpha) * self.controls[i - 1][axis])
                .collect();
        }
        let mut knots = self.knots.clone();
        knots.insert(k + 1, u);
        Ok(Self {
            degree: p,
            knots,
            controls,
        })
    }
}
pub fn basis(p: usize, k: &[f64], n: usize, u: f64, periodic: bool) -> Result<Basis> {
    let domain = validate_basis(p, k, n)?;
    check(
        u.is_finite() && u >= domain[0] && u <= domain[1],
        "Parameter is outside the active knot domain",
    )?;
    let mut span = p;
    if u == domain[1] {
        span = n - 1;
        while span > 0 && k[span] == u {
            span -= 1;
        }
    } else {
        while span + 1 < k.len() && k[span + 1] <= u {
            span += 1;
        }
    }
    let mut b = vec![0.; k.len() - 1];
    b[span] = 1.;
    let mut d1 = vec![0.; b.len()];
    let mut d2 = d1.clone();
    for order in 1..=p {
        let mut next = vec![0.; k.len() - order - 1];
        let mut first = next.clone();
        let mut second = next.clone();
        for i in 0..next.len() {
            let left = k[i + order] - k[i];
            let right = k[i + order + 1] - k[i + 1];
            if left != 0. {
                next[i] += (u - k[i]) / left * b[i];
                first[i] += order as f64 / left * b[i];
                second[i] += order as f64 / left * d1[i];
            }
            if right != 0. {
                next[i] += (k[i + order + 1] - u) / right * b[i + 1];
                first[i] -= order as f64 / right * b[i + 1];
                second[i] -= order as f64 / right * d1[i + 1];
            }
        }
        b = next;
        d1 = first;
        d2 = second;
    }
    let endpoint = u == domain[0] || u == domain[1];
    let m = if endpoint && !periodic {
        0
    } else {
        multiplicity(k, u)
    };
    let continuity = if m == 0 {
        None
    } else {
        Some(p as i32 - m as i32)
    };
    let status = if continuity.unwrap_or(2) >= 2 {
        "available"
    } else {
        "insufficient_continuity"
    };
    let side = if u == domain[0] {
        "right"
    } else if u == domain[1] {
        "left"
    } else if status == "insufficient_continuity" {
        "right"
    } else {
        "two_sided"
    };
    numeric(
        b.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
        "Knot scale exhausted finite derivative precision",
    )?;
    Ok(Basis {
        basis: b,
        d1,
        d2,
        domain,
        continuity,
        derivative_status: status,
        derivative_side: side,
    })
}
