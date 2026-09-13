//! Bounded, geometry-only intersection queries on retained rational definitions.
//!
//! This is the numerical foundation for sectioning and future curved Booleans,
//! not a Boolean topology oracle. Bernstein sign bounds and parameter boxes are
//! retained, but knot insertion/evaluation roundoff and branch adjacency do not
//! yet have independent coverage certificates. Even `NumericallyResolved` MUST
//! NOT be consumed as a certificate authorizing a topology change. No triangles,
//! fitted curves, snapping, or tessellation participate in these queries.
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

#[derive(Clone, Copy, Debug)]
pub struct Plane {
    /// The equation is normal.dot(point) = offset; need not be normalized.
    pub normal: [f64; 3],
    pub offset: f64,
}

impl Plane {
    fn normalized(self) -> Result<Self> {
        if !self.normal.iter().all(|v| v.is_finite()) || !self.offset.is_finite() {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        let scale = self.normal.iter().map(|v| v.abs()).fold(0., f64::max);
        if scale == 0. {
            return Err(invalid(
                "Plane needs a finite, nonzero normal and finite offset",
            ));
        }
        // Never form the original norm: it may overflow, or round a
        // subnormal vector's length down to a single component. The scaled
        // norm is in [1,sqrt(3)] even at binary64's extreme exponents.
        let scaled = self.normal.map(|x| x / scale);
        let length = scaled[0].hypot(scaled[1]).hypot(scaled[2]);
        // Divide the larger offset by length first when offset/scale could
        // overflow. For a smaller offset, divide by scale first so a tiny
        // offset is not lost before a subsequent division by a tiny scale.
        let scaled_offset = self.offset / scale;
        let offset = if !scaled_offset.is_finite() {
            (self.offset / length) / scale
        } else {
            scaled_offset / length
        };
        let plane = Self {
            normal: scaled.map(|x| x / length),
            offset,
        };
        if !plane.offset.is_finite() {
            return Err(invalid("Normalized plane offset is not finite"));
        }
        Ok(plane)
    }
    fn distance(self, point: [f64; 3]) -> f64 {
        dot(self.normal, point) - self.offset
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Options {
    /// Absolute model-space residual target. Does not grow during a query.
    pub distance_tolerance: f64,
    /// Absolute parameter interval target, in the input knot coordinates.
    pub parameter_tolerance: f64,
    pub max_depth: usize,
    pub max_boxes: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            distance_tolerance: 1e-9,
            parameter_tolerance: 1e-10,
            max_depth: 48,
            max_boxes: 8192,
        }
    }
}
impl Options {
    fn validate(self) -> Result<Self> {
        if !self.distance_tolerance.is_finite()
            || self.distance_tolerance <= 0.
            || !self.parameter_tolerance.is_finite()
            || self.parameter_tolerance <= 0.
            || !(1..=64).contains(&self.max_depth)
            || !(1..=65536).contains(&self.max_boxes)
        {
            return Err(invalid(
                "Intersection tolerances must be positive and finite; depth 1..64, boxes 1..65536",
            ));
        }
        Ok(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Coverage {
    /// All parameter regions were handled numerically. NOT certified complete.
    NumericallyResolved,
    Incomplete,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnresolvedReason {
    BudgetExceeded,
    TangencyOrMultipleRoot,
    NearCoincidence,
    BoundaryCrossing,
    UnsupportedSurface,
    CoincidentTrim,
}
#[derive(Clone, Debug)]
pub struct Unresolved {
    /// Curve: [t0,t1]. Surface: [u0,u1,v0,v1]. Surface pairs concatenate both boxes.
    pub parameter_box: Vec<f64>,
    pub reason: UnresolvedReason,
}
#[derive(Clone, Debug)]
pub struct Report<T> {
    pub components: Vec<T>,
    pub unresolved: Vec<Unresolved>,
    pub boxes_visited: usize,
    pub bernstein_excluded: usize,
    pub coverage: Coverage,
}
impl<T> Default for Report<T> {
    fn default() -> Self {
        Self {
            components: Vec::new(),
            unresolved: Vec::new(),
            boxes_visited: 0,
            bernstein_excluded: 0,
            coverage: Coverage::NumericallyResolved,
        }
    }
}
impl<T> Report<T> {
    /// Always false until independent coverage and correspondence certificates
    /// exist. Keeping this query beside coverage prevents accidental promotion.
    pub fn permits_topology_change(&self) -> bool {
        false
    }
    fn unresolved(&mut self, domain: impl Into<Vec<f64>>, reason: UnresolvedReason) {
        self.coverage = Coverage::Incomplete;
        self.unresolved.push(Unresolved {
            parameter_box: domain.into(),
            reason,
        });
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Contact {
    Transverse,
    Boundary,
}
#[derive(Clone, Debug)]
pub struct CurvePoint {
    pub parameter: f64,
    pub parameter_interval: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
    pub contact: Contact,
}
#[derive(Clone, Debug)]
pub enum CurvePlaneComponent {
    Point(CurvePoint),
    /// All retained Bezier control points lie numerically in the plane.
    Overlap {
        parameter_interval: [f64; 2],
        control_residual: f64,
    },
}

/// Outward binary64 interval arithmetic is used only for sign exclusion on the
/// current homogeneous Bernstein data. It does not cover preceding knot edits.
#[derive(Clone, Copy, Debug)]
struct Interval {
    lo: f64,
    hi: f64,
}
impl Interval {
    fn whole() -> Self {
        Self {
            lo: f64::NEG_INFINITY,
            hi: f64::INFINITY,
        }
    }
    fn exact(x: f64) -> Self {
        Self { lo: x, hi: x }
    }
    fn add(self, b: Self) -> Self {
        if self.lo == 0. && self.hi == 0. {
            return b;
        }
        if b.lo == 0. && b.hi == 0. {
            return self;
        }
        if self.lo == self.hi && b.lo == b.hi && self.lo.is_finite() && self.lo == -b.lo {
            return Self::exact(0.);
        }
        if (self.lo + b.lo).is_nan() || (self.hi + b.hi).is_nan() {
            return Self::whole();
        }
        Self {
            lo: (self.lo + b.lo).next_down(),
            hi: (self.hi + b.hi).next_up(),
        }
    }
    fn mul(self, b: Self) -> Self {
        if self.lo == 1. && self.hi == 1. {
            return b;
        }
        if b.lo == 1. && b.hi == 1. {
            return self;
        }
        if self.lo == -1. && self.hi == -1. {
            return Self {
                lo: -b.hi,
                hi: -b.lo,
            };
        }
        if b.lo == -1. && b.hi == -1. {
            return Self {
                lo: -self.hi,
                hi: -self.lo,
            };
        }
        if (self.lo == 0. && self.hi == 0. && b.lo.is_finite() && b.hi.is_finite())
            || (b.lo == 0. && b.hi == 0. && self.lo.is_finite() && self.hi.is_finite())
        {
            return Self::exact(0.);
        }
        let values = [
            self.lo * b.lo,
            self.lo * b.hi,
            self.hi * b.lo,
            self.hi * b.hi,
        ];
        if values.iter().any(|v| v.is_nan()) {
            return Self::whole();
        }
        Self {
            lo: values.into_iter().fold(f64::INFINITY, f64::min).next_down(),
            hi: values
                .into_iter()
                .fold(f64::NEG_INFINITY, f64::max)
                .next_up(),
        }
    }
    fn sign(self) -> i8 {
        if self.lo > 0. {
            1
        } else if self.hi < 0. {
            -1
        } else {
            0
        }
    }
}

#[derive(Clone, Copy)]
struct Coefficient {
    value: f64,
    bound: Interval,
}
fn coefficients(curve: &Curve, plane: Plane) -> Vec<Coefficient> {
    curve
        .control_points
        .iter()
        .zip(&curve.weights)
        .map(|(point, weight)| {
            let mut bound = Interval::exact(-plane.offset);
            for (a, b) in plane.normal.into_iter().zip(point) {
                bound = bound.add(Interval::exact(a).mul(Interval::exact(*b)));
            }
            Coefficient {
                value: plane.distance(point3(point)) * weight,
                bound: bound.mul(Interval::exact(*weight)),
            }
        })
        .collect()
}
fn split(coefficients: &[Coefficient]) -> (Vec<Coefficient>, Vec<Coefficient>) {
    let mut row = coefficients.to_vec();
    let mut left = vec![row[0]];
    let mut right = vec![*row.last().unwrap()];
    while row.len() > 1 {
        row = row
            .windows(2)
            .map(|p| Coefficient {
                value: (p[0].value + p[1].value) * 0.5,
                bound: p[0].bound.add(p[1].bound).mul(Interval::exact(0.5)),
            })
            .collect();
        left.push(row[0]);
        right.push(*row.last().unwrap());
    }
    right.reverse();
    (left, right)
}
fn excluded(coefficients: &[Coefficient]) -> bool {
    coefficients.iter().all(|c| c.bound.sign() == 1)
        || coefficients.iter().all(|c| c.bound.sign() == -1)
}
fn sign_variations(coefficients: &[Coefficient]) -> usize {
    let signs: Vec<_> = coefficients
        .iter()
        .filter_map(|c| {
            if c.value == 0. {
                None
            } else {
                Some(c.value > 0.)
            }
        })
        .collect();
    signs.windows(2).filter(|s| s[0] != s[1]).count()
}
fn spans(knots: &[f64], degree: usize, count: usize) -> Vec<[f64; 2]> {
    knots[degree..=count]
        .windows(2)
        .filter_map(|s| (s[0] < s[1]).then_some([s[0], s[1]]))
        .collect()
}

/// Isolate intersections of any validated positive-weight 3D NURBS curve with
/// a plane, over the curve's entire active knot domain. Multiple/tangent roots,
/// precision bands and work exhaustion stay explicit instead of becoming empty.
pub fn curve_plane(
    curve: &Curve,
    plane: Plane,
    options: Options,
) -> Result<Report<CurvePlaneComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/plane requires a 3D curve"));
    }
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let mut pending: std::collections::VecDeque<([f64; 2], Option<Vec<Coefficient>>, usize)> =
        spans(&curve.knots, curve.degree, curve.control_points.len())
            .into_iter()
            .map(|domain| (domain, None, 0))
            .collect();
    while let Some((interval, values, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(interval, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let initial_span = values.is_none();
        let values = match values {
            Some(values) => values,
            None => coefficients(&curve.trim(interval[0], interval[1])?, plane),
        };
        if initial_span && values.iter().all(|c| c.value == 0.) {
            report.components.push(CurvePlaneComponent::Overlap {
                parameter_interval: interval,
                control_residual: 0.,
            });
            continue;
        }
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        // Endpoints belong to both neighboring spans; only exact numerical
        // zeros are merged, never distinct roots merely closer than a tol.
        for (parameter, coefficient) in [
            (interval[0], values[0]),
            (interval[1], *values.last().unwrap()),
        ] {
            if coefficient.value == 0. {
                let jet = curve.evaluate(parameter)?;
                let point = point3(&jet.point);
                if plane.distance(point).abs() <= options.distance_tolerance {
                    let derivative = jet.d1.map(|v| dot(plane.normal, point3(&v)));
                    if parameter != curve.domain()[0]
                        && parameter != curve.domain()[1]
                        && derivative.is_none_or(|d| d.abs() <= options.distance_tolerance)
                    {
                        report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
                    } else if !report.components.iter().any(
                        |c| matches!(c, CurvePlaneComponent::Point(p) if p.parameter == parameter),
                    ) {
                        report
                            .components
                            .push(CurvePlaneComponent::Point(CurvePoint {
                                parameter,
                                parameter_interval: [parameter, parameter],
                                point,
                                plane_residual: plane.distance(point).abs(),
                                contact: Contact::Boundary,
                            }));
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
        }
        let variations = sign_variations(&values);
        if variations == 0 {
            // Numerically one-sided Bernstein coefficients. This is not an
            // interval exclusion if zero-containing coefficient intervals
            // remain; coverage is deliberately only numerical throughout.
            if values.iter().any(|c| c.value != 0. && c.bound.sign() == 0) {
                report.unresolved(interval, UnresolvedReason::NearCoincidence);
            }
            continue;
        }
        let width = interval[1] - interval[0];
        let midpoint = interval[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == interval[0]
            || midpoint == interval[1]
        {
            let point = point3(&curve.evaluate(midpoint)?.point);
            let residual = plane.distance(point).abs();
            if variations == 1 && residual <= options.distance_tolerance {
                report
                    .components
                    .push(CurvePlaneComponent::Point(CurvePoint {
                        parameter: midpoint,
                        parameter_interval: interval,
                        point,
                        plane_residual: residual,
                        contact: Contact::Transverse,
                    }));
            } else {
                report.unresolved(interval, UnresolvedReason::TangencyOrMultipleRoot);
            }
            continue;
        }
        let (left, right) = split(&values);
        pending.push_back(([interval[0], midpoint], Some(left), depth + 1));
        pending.push_back(([midpoint, interval[1]], Some(right), depth + 1));
    }
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurvePlaneComponent| match c {
            CurvePlaneComponent::Point(p) => p.parameter,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => parameter_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct SurfacePoint {
    pub uv: [f64; 2],
    pub point: [f64; 3],
    pub plane_residual: f64,
}

#[derive(Clone, Debug)]
pub enum SurfaceTrace {
    /// A ruled trace linear in U, parameterized by original V.
    RuledU {
        surface: Surface,
        plane: Plane,
        v_interval: [f64; 2],
    },
    /// Affine line in the surface's original parameter coordinates.
    Line {
        surface: Surface,
        plane: Plane,
        start: [f64; 2],
        end: [f64; 2],
    },
    /// A retained ruled rational surface and plane define v(u) exactly as a
    /// procedural trace. This avoids replacing a conic with fitted samples.
    /// This variant is linear in V; RuledU retains the transposed case.
    Ruled {
        surface: Surface,
        plane: Plane,
        u_interval: [f64; 2],
    },
}
impl SurfaceTrace {
    /// Ordered rational pieces on the original [0,1] trace-fraction domain.
    /// Knot-crossing endpoints remain independent numerical values: this API
    /// does not reconcile them or grant permission to sew topology.
    pub fn to_curve_segments(&self) -> Result<Vec<Curve>> {
        self.evaluate(0.)?;
        self.evaluate(1.)?;
        match self {
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => return ruled_segments(surface, *plane, *u_interval),
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => return ruled_segments(&transpose_surface(surface), *plane, *v_interval),
            _ => (),
        }
        if let Self::Line {
            surface,
            plane,
            start,
            end,
        } = self
        {
            if start[0] != end[0] && start[1] != end[1] {
                if surface.degree_u + surface.degree_v > 25 {
                    return Err(invalid(
                        "UV diagonal conversion requires result degree at most 25",
                    ));
                }
                return diagonal_segments(surface, *plane, *start, *end);
            }
        }
        let mut curve = self.to_curve()?;
        let [lo, hi] = curve.domain();
        for knot in &mut curve.knots {
            *knot = (*knot - lo) / (hi - lo);
        }
        curve.validate()?;
        Ok(vec![curve])
    }
    /// Algebraic conversion of supported retained traces, never sample fitting.
    /// Floating-point coefficient arithmetic is numerical, not certified exact.
    /// Fraction maps linearly to the returned curve's active knot domain.
    pub fn to_curve(&self) -> Result<Curve> {
        let start_point = self.evaluate(0.)?.point;
        let end_point = self.evaluate(1.)?.point;
        match self {
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .to_curve(),
            Self::Line {
                surface,
                plane,
                start,
                end,
            } => {
                let (axis, constant, range) = if start[1] == end[1] && start[0] != end[0] {
                    (nurbs_core::surface::Axis::V, start[1], [start[0], end[0]])
                } else if start[0] == end[0] && start[1] != end[1] {
                    (nurbs_core::surface::Axis::U, start[0], [start[1], end[1]])
                } else {
                    if affine_frame(surface).is_some() || start == end {
                        return Curve::from_polyline(vec![
                            start_point.to_vec(),
                            end_point.to_vec(),
                        ]);
                    }
                    let degree = surface.degree_u + surface.degree_v;
                    if degree > 25 {
                        return Err(invalid(
                            "UV diagonal conversion requires result degree at most 25",
                        ));
                    }
                    let piece = surface.trim([
                        start[0].min(end[0]),
                        start[0].max(end[0]),
                        start[1].min(end[1]),
                        start[1].max(end[1]),
                    ])?;
                    let p = piece.degree_u;
                    let q = piece.degree_v;
                    if piece.control_points.len() != p + 1 || piece.control_points[0].len() != q + 1
                    {
                        return diagonal_multispan(surface, *plane, *start, *end);
                    }
                    // Restrict the homogeneous tensor product to u(t), v(t).
                    // B_i^p(t) B_j^q(t) = C(p,i) C(q,j) / C(p+q,i+j) B_(i+j)^(p+q)(t).
                    let binomial = |n: usize, k: usize| {
                        (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                    };
                    let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                    let mut controls = vec![[0.; 4]; degree + 1];
                    for i in 0..=p {
                        for j in 0..=q {
                            let u = if start[0] > end[0] { p - i } else { i };
                            let v = if start[1] > end[1] { q - j } else { j };
                            let factor = binomial(p, i) * binomial(q, j) / binomial(degree, i + j);
                            let weight = factor * (piece.weights[u][v] / scale);
                            for axis in 0..3 {
                                controls[i + j][axis] += weight * piece.control_points[u][v][axis];
                            }
                            controls[i + j][3] += weight;
                        }
                    }
                    let curve = Curve {
                        degree,
                        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
                        control_points: controls
                            .iter()
                            .enumerate()
                            .map(|(index, h)| {
                                // At either end the tensor basis has exactly one
                                // nonzero term. Copy its Cartesian coefficient;
                                // a redundant multiply/divide introduces drift.
                                if index == 0 || index == degree {
                                    let last = index == degree;
                                    let u = if (start[0] > end[0]) != last { p } else { 0 };
                                    let v = if (start[1] > end[1]) != last { q } else { 0 };
                                    piece.control_points[u][v].clone()
                                } else {
                                    (0..3).map(|axis| h[axis] / h[3]).collect()
                                }
                            })
                            .collect(),
                        weights: controls.iter().map(|h| h[3]).collect(),
                        periodic: false,
                    };
                    curve.validate()?;
                    return Ok(curve);
                };
                let curve = surface
                    .iso(axis, constant)?
                    .trim(range[0].min(range[1]), range[0].max(range[1]))?;
                if range[0] > range[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => {
                let degree = surface.degree_u;
                if degree > 12 || u_interval[0] == u_interval[1] {
                    return Err(invalid(
                        "Trace conversion requires nonzero interval and result degree at most 25",
                    ));
                }
                let domain = surface_domain(surface);
                let lo = u_interval[0].min(u_interval[1]);
                let hi = u_interval[0].max(u_interval[1]);
                let piece = surface.trim([lo, hi, domain[2], domain[3]])?;
                if piece.control_points.len() != degree + 1 {
                    let mut breaks: Vec<f64> = piece
                        .knots_u
                        .iter()
                        .copied()
                        .filter(|u| *u >= lo && *u <= hi)
                        .collect();
                    breaks.dedup();
                    let result_degree = 2 * degree;
                    if (breaks.len() - 1) * result_degree + 1 > 256 {
                        return Err(invalid("Converted trace exceeds 256 control points"));
                    }
                    let mut result: Option<Curve> = None;
                    for span in breaks.windows(2) {
                        let segment = Self::Ruled {
                            surface: piece.trim([span[0], span[1], domain[2], domain[3]])?,
                            plane: *plane,
                            u_interval: [span[0], span[1]],
                        }
                        .to_curve()?;
                        if let Some(joined) = &mut result {
                            // Do not weld geometrically nearby endpoints: only identical
                            // Cartesian endpoint values can share a control point here.
                            if joined.control_points.last() != segment.control_points.first() {
                                return Err(invalid(
                                    "Converted span endpoints do not coincide numerically",
                                ));
                            }
                            let factor = joined.weights.last().unwrap() / segment.weights[0];
                            joined.knots.pop();
                            joined
                                .knots
                                .extend_from_slice(&segment.knots[result_degree + 1..]);
                            joined
                                .control_points
                                .extend_from_slice(&segment.control_points[1..]);
                            joined
                                .weights
                                .extend(segment.weights[1..].iter().map(|w| w * factor));
                        } else {
                            result = Some(segment);
                        }
                    }
                    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
                    let scale = curve.weights.iter().copied().fold(0., f64::max);
                    for weight in &mut curve.weights {
                        *weight /= scale;
                    }
                    curve.validate()?;
                    return if u_interval[0] > u_interval[1] {
                        curve.reverse()
                    } else {
                        Ok(curve)
                    };
                }
                let plane = plane.normalized()?;
                let scale = piece.weights.iter().flatten().copied().fold(0., f64::max);
                let homogeneous = (0..=degree)
                    .map(|i| {
                        std::array::from_fn::<_, 2, _>(|j| {
                            let w = piece.weights[i][j] / scale;
                            [
                                piece.control_points[i][j][0] * w,
                                piece.control_points[i][j][1] * w,
                                piece.control_points[i][j][2] * w,
                                w,
                            ]
                        })
                    })
                    .collect::<Vec<_>>();
                let values = homogeneous
                    .iter()
                    .map(|row| {
                        row.map(|h| dot(plane.normal, [h[0], h[1], h[2]]) - plane.offset * h[3])
                    })
                    .collect::<Vec<_>>();
                if values.iter().flatten().any(|v| !v.is_finite()) {
                    return Err(invalid("Nonfinite ruled boundary residual"));
                }
                admit_ruled_parameter_bounds(ruled_boundary_bounds(&piece, plane))?;
                let binomial = |n: usize, k: usize| {
                    (0..k).fold(1_f64, |value, i| value * (n - i) as f64 / (i + 1) as f64)
                };
                let mut controls = vec![[0.; 4]; 2 * degree + 1];
                for i in 0..=degree {
                    for j in 0..=degree {
                        let factor =
                            binomial(degree, i) * binomial(degree, j) / binomial(2 * degree, i + j);
                        for axis in 0..4 {
                            controls[i + j][axis] += factor
                                * (homogeneous[i][1][axis] * values[j][0]
                                    - homogeneous[i][0][axis] * values[j][1]);
                        }
                    }
                }
                let sign = if controls.iter().all(|h| h[3] > 0.) {
                    1.
                } else if controls.iter().all(|h| h[3] < 0.) {
                    -1.
                } else {
                    return Err(invalid(
                        "Converted trace has no positive-weight representation in this basis",
                    ));
                };
                let scale = controls.iter().map(|h| h[3].abs()).fold(0., f64::max);
                let curve = Curve {
                    degree: 2 * degree,
                    knots: [vec![lo; 2 * degree + 1], vec![hi; 2 * degree + 1]].concat(),
                    control_points: controls
                        .iter()
                        .map(|h| (0..3).map(|i| h[i] / h[3]).collect())
                        .collect(),
                    weights: controls.iter().map(|h| sign * h[3] / scale).collect(),
                    periodic: false,
                };
                curve.validate()?;
                if u_interval[0] > u_interval[1] {
                    curve.reverse()
                } else {
                    Ok(curve)
                }
            }
        }
    }
    /// `fraction` is in [0,1]. The source surface remains authoritative.
    pub fn evaluate(&self, fraction: f64) -> Result<SurfacePoint> {
        if !fraction.is_finite() || !(0. ..=1.).contains(&fraction) {
            return Err(invalid("Trace fraction must be in [0,1]"));
        }
        if let Self::RuledU {
            surface,
            plane,
            v_interval,
        } = self
        {
            surface.validate()?;
            let mut point = Self::Ruled {
                surface: transpose_surface(surface),
                plane: *plane,
                u_interval: *v_interval,
            }
            .evaluate(fraction)?;
            point.uv.swap(0, 1);
            return Ok(point);
        }
        let (surface, plane) = match self {
            Self::Line { surface, plane, .. }
            | Self::Ruled { surface, plane, .. }
            | Self::RuledU { surface, plane, .. } => (surface, *plane),
        };
        surface.validate()?;
        let plane = plane.normalized()?;
        let uv = match self {
            Self::RuledU { .. } => unreachable!("Handled above"),
            Self::Line { start, end, .. } => {
                let domain = surface_domain(surface);
                if ![start, end].iter().all(|uv| {
                    (0..2).all(|axis| {
                        uv[axis].is_finite()
                            && uv[axis] >= domain[axis * 2]
                            && uv[axis] <= domain[axis * 2 + 1]
                    })
                }) {
                    return Err(invalid(
                        "Line trace endpoints must lie in the source domain",
                    ));
                }
                std::array::from_fn(|i| start[i] + fraction * (end[i] - start[i]))
            }
            Self::Ruled {
                surface: source,
                u_interval,
                ..
            } => {
                if source.degree_v != 1 || source.control_points[0].len() != 2 {
                    return Err(invalid("Ruled trace requires a surface linear in V"));
                }
                let domain = surface_domain(source);
                if !u_interval
                    .iter()
                    .all(|u| u.is_finite() && *u >= domain[0] && *u <= domain[1])
                {
                    return Err(invalid(
                        "Ruled trace interval must lie in the source domain",
                    ));
                }
                let u = u_interval[0] + fraction * (u_interval[1] - u_interval[0]);
                let bottom = source.evaluate(u, domain[2])?.point;
                let top = source.evaluate(u, domain[3])?.point;
                // Linear interpolation occurs in homogeneous coordinates.
                // Both boundary weights must use the same normalization.
                let basis = nurbs_core::curve::basis(
                    source.degree_u,
                    &source.knots_u,
                    source.control_points.len(),
                    u,
                    source.periodic_u,
                )?;
                let scale = source.weights.iter().flatten().copied().fold(0., f64::max);
                let weights: [f64; 2] = std::array::from_fn(|column| {
                    basis
                        .basis
                        .iter()
                        .zip(&source.weights)
                        .map(|(b, w)| b * (w[column] / scale))
                        .sum()
                });
                let a = weights[0] * plane.distance(bottom);
                let b = weights[1] * plane.distance(top);
                let denominator = a - b;
                if !denominator.is_finite() || denominator == 0. {
                    return Err(invalid("Singular ruled intersection trace"));
                }
                let v = domain[2] + a / denominator * (domain[3] - domain[2]);
                if v < domain[2] || v > domain[3] {
                    return Err(invalid("Ruled trace left the source domain"));
                }
                [u, v]
            }
        };
        let point = surface.evaluate(uv[0], uv[1])?.point;
        Ok(SurfacePoint {
            uv,
            point,
            plane_residual: plane.distance(point).abs(),
        })
    }
}
#[derive(Clone, Debug)]
pub enum SurfacePlaneComponent {
    Curve {
        trace: SurfaceTrace,
        /// Parameter region supporting the branch, including root uncertainty.
        parameter_box: [f64; 4],
        samples: Vec<SurfacePoint>,
        max_sample_residual: f64,
    },
    Point(SurfacePoint),
    Overlap {
        parameter_box: [f64; 4],
        control_residual: f64,
    },
}

/// Enclose residual construction from retained trimmed Cartesian coefficients
/// and the normalized plane. Prior trim and plane normalization are not covered.
fn ruled_boundary_bounds(surface: &Surface, plane: Plane) -> Vec<[Interval; 2]> {
    let scale = surface.weights.iter().flatten().copied().fold(0., f64::max);
    surface
        .control_points
        .iter()
        .zip(&surface.weights)
        .map(|(points, weights)| {
            std::array::from_fn(|side| {
                let mut residual = Interval::exact(-plane.offset);
                for axis in 0..3 {
                    residual = residual.add(
                        Interval::exact(plane.normal[axis])
                            .mul(Interval::exact(points[side][axis])),
                    );
                }
                let weight = if weights[side] == scale {
                    Interval::exact(1.)
                } else {
                    let ratio = weights[side] / scale;
                    Interval {
                        lo: ratio.next_down().max(0.),
                        hi: ratio.next_up(),
                    }
                };
                residual.mul(weight)
            })
        })
        .collect()
}
#[cfg(test)]
fn admit_ruled_parameter_range(values: &[[f64; 2]]) -> Result<()> {
    if values.iter().flatten().any(|v| !v.is_finite()) {
        return Err(invalid("Nonfinite ruled boundary residual"));
    }
    admit_ruled_parameter_bounds(values.iter().map(|r| r.map(Interval::exact)).collect())
}
/// Bernstein bounds cover residual construction and subdivision rounding, but
/// not earlier knot edits or plane normalization. This is not a certificate.
fn admit_ruled_parameter_bounds(bounds: Vec<[Interval; 2]>) -> Result<()> {
    let mut pending = vec![(bounds, 0usize)];
    let mut visited = 0;
    while let Some((row, depth)) = pending.pop() {
        visited += 1;
        if visited > 4096 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range admission exceeded its work budget",
            ));
        }
        let opposite = row.iter().all(|r| r[0].hi <= 0. && r[1].lo >= 0.)
            || row.iter().all(|r| r[0].lo >= 0. && r[1].hi <= 0.);
        if opposite {
            continue;
        }
        // Endpoint Bernstein coefficients enclose actual endpoint residuals.
        // Equal strict signs mean that the plane lies outside that ruling.
        for r in [row.first().unwrap(), row.last().unwrap()] {
            if r[0].sign() != 0 && r[0].sign() == r[1].sign() {
                return Err(invalid("Ruled trace leaves the source parameter domain"));
            }
        }
        if depth >= 32 {
            return Err(Error::new(
                "BREP_INTERSECTION_UNRESOLVED",
                "Ruled parameter range remains unresolved at the subdivision limit",
            ));
        }
        let mut work = row;
        let mut left = vec![work[0]];
        let mut right = vec![*work.last().unwrap()];
        while work.len() > 1 {
            work = work
                .windows(2)
                .map(|r| {
                    std::array::from_fn(|axis| r[0][axis].add(r[1][axis]).mul(Interval::exact(0.5)))
                })
                .collect();
            left.push(work[0]);
            right.push(*work.last().unwrap());
        }
        right.reverse();
        pending.push((right, depth + 1));
        pending.push((left, depth + 1));
    }
    Ok(())
}

/// Keep each ruled section span independently, avoiding any endpoint welding.
/// The source U interval is mapped to the same global fraction as evaluate().
fn ruled_segments(surface: &Surface, plane: Plane, interval: [f64; 2]) -> Result<Vec<Curve>> {
    let degree = surface.degree_u;
    if degree > 12 || interval[0] == interval[1] {
        return Err(invalid(
            "Trace conversion requires nonzero interval and result degree at most 25",
        ));
    }
    let lo = interval[0].min(interval[1]);
    let hi = interval[0].max(interval[1]);
    let domain = surface_domain(surface);
    let mut breaks = vec![lo, hi];
    breaks.extend(
        surface
            .knots_u
            .iter()
            .copied()
            .filter(|u| *u > lo && *u < hi),
    );
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    if (breaks.len() - 1) * (2 * degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    if interval[0] > interval[1] {
        breaks.reverse();
    }
    let mut result = Vec::new();
    for span in breaks.windows(2) {
        let a = span[0].min(span[1]);
        let b = span[0].max(span[1]);
        let piece = surface.trim([a, b, domain[2], domain[3]])?;
        if piece.control_points.len() != degree + 1 {
            return Err(invalid(
                "Ruled knot partition does not isolate one Bezier span",
            ));
        }
        let mut curve = SurfaceTrace::Ruled {
            surface: piece,
            plane,
            u_interval: [span[0], span[1]],
        }
        .to_curve()?;
        let t0 = if span[0] == interval[0] {
            0.
        } else {
            (span[0] - interval[0]) / (interval[1] - interval[0])
        };
        let t1 = if span[1] == interval[1] {
            1.
        } else {
            (span[1] - interval[0]) / (interval[1] - interval[0])
        };
        if !t0.is_finite() || !t1.is_finite() || t0 >= t1 {
            return Err(invalid("Ruled knot crossing interval is not representable"));
        }
        // A Bezier piece has only its two clamped endpoint knots. Reverse()
        // reverses coefficients while retaining this ascending source domain.
        for knot in &mut curve.knots {
            *knot = if *knot == a { t0 } else { t1 };
        }
        curve.validate()?;
        result.push(curve);
    }
    Ok(result)
}

/// Partition an affine UV trace at source knot crossings. Every piece remains
/// a homogeneous tensor restriction, and joins require identical endpoint
/// values; tolerance welding is deliberately not part of this conversion.
fn diagonal_segments(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Vec<Curve>> {
    let degree = surface.degree_u + surface.degree_v;
    let mut crossings = vec![(0., None), (1., None)];
    for (axis, knots) in [(0, &surface.knots_u), (1, &surface.knots_v)] {
        for &knot in knots {
            if knot > start[axis].min(end[axis]) && knot < start[axis].max(end[axis]) {
                let t = (knot - start[axis]) / (end[axis] - start[axis]);
                if !t.is_finite() || t <= 0. || t >= 1. {
                    return Err(invalid("UV knot crossing is not representable"));
                }
                crossings.push((t, Some((axis, knot))));
            }
        }
    }
    crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut breaks: Vec<(f64, [f64; 2], [bool; 2])> = Vec::new();
    for (t, crossing) in crossings {
        if breaks.last().is_none_or(|b| b.0 != t) {
            let uv = if t == 0. {
                start
            } else if t == 1. {
                end
            } else {
                std::array::from_fn(|axis| start[axis] + t * (end[axis] - start[axis]))
            };
            breaks.push((t, uv, [false; 2]));
        }
        if let Some((axis, knot)) = crossing {
            let b = breaks.last_mut().unwrap();
            if b.2[axis] && b.1[axis] != knot {
                return Err(invalid("Distinct UV knots collapse to one trace parameter"));
            }
            b.1[axis] = knot;
            b.2[axis] = true;
        }
    }
    if (breaks.len() - 1) * (degree + 1) > 256 {
        return Err(invalid("Converted trace exceeds 256 control points"));
    }
    let mut result = Vec::new();
    for pair in breaks.windows(2) {
        let (t0, a, _) = pair[0];
        let (t1, b, _) = pair[1];
        if a[0] == b[0] || a[1] == b[1] {
            return Err(invalid("UV knot crossing interval is not representable"));
        }
        let piece = surface.trim([
            a[0].min(b[0]),
            a[0].max(b[0]),
            a[1].min(b[1]),
            a[1].max(b[1]),
        ])?;
        if piece.control_points.len() != piece.degree_u + 1
            || piece.control_points[0].len() != piece.degree_v + 1
        {
            return Err(invalid(
                "UV knot partition does not isolate one tensor Bezier span",
            ));
        }
        let mut segment = SurfaceTrace::Line {
            surface: piece,
            plane,
            start: a,
            end: b,
        }
        .to_curve()?
        .elevate(degree)?;
        // Single-span conversion is parameterized on [0,1]. Preserve the
        // original trace fraction globally, including nonuniform crossings.
        for knot in &mut segment.knots {
            *knot = if *knot == 0. { t0 } else { t1 };
        }
        segment.validate()?;
        result.push(segment);
    }
    Ok(result)
}
fn diagonal_multispan(
    surface: &Surface,
    plane: Plane,
    start: [f64; 2],
    end: [f64; 2],
) -> Result<Curve> {
    let degree = surface.degree_u + surface.degree_v;
    let mut result: Option<Curve> = None;
    for segment in diagonal_segments(surface, plane, start, end)? {
        if let Some(joined) = &mut result {
            if joined.control_points.last() != segment.control_points.first() {
                return Err(invalid(
                    "Converted span endpoints do not coincide numerically",
                ));
            }
            let factor = joined.weights.last().unwrap() / segment.weights[0];
            joined.knots.pop();
            joined.knots.extend_from_slice(&segment.knots[degree + 1..]);
            joined
                .control_points
                .extend_from_slice(&segment.control_points[1..]);
            joined
                .weights
                .extend(segment.weights[1..].iter().map(|w| w * factor));
        } else {
            result = Some(segment);
        }
    }
    let mut curve = result.ok_or_else(|| invalid("Trace has no nonzero spans"))?;
    let scale = curve.weights.iter().copied().fold(0., f64::max);
    for weight in &mut curve.weights {
        *weight /= scale;
    }
    curve.validate()?;
    Ok(curve)
}

fn transpose_surface(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|v| s.control_points.iter().map(|row| row[v].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|v| s.weights.iter().map(|row| row[v]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
fn swap_trace(trace: SurfaceTrace) -> SurfaceTrace {
    match trace {
        SurfaceTrace::Line {
            surface,
            plane,
            start,
            end,
        } => SurfaceTrace::Line {
            surface: transpose_surface(&surface),
            plane,
            start: [start[1], start[0]],
            end: [end[1], end[0]],
        },
        SurfaceTrace::Ruled {
            surface,
            plane,
            u_interval,
        } => SurfaceTrace::RuledU {
            surface: transpose_surface(&surface),
            plane,
            v_interval: u_interval,
        },
        SurfaceTrace::RuledU {
            surface,
            plane,
            v_interval,
        } => SurfaceTrace::Ruled {
            surface: transpose_surface(&surface),
            plane,
            u_interval: v_interval,
        },
    }
}

fn surface_domain(surface: &Surface) -> [f64; 4] {
    [
        surface.knots_u[surface.degree_u],
        surface.knots_u[surface.control_points.len()],
        surface.knots_v[surface.degree_v],
        surface.knots_v[surface.control_points[0].len()],
    ]
}

#[derive(Clone, Copy)]
struct AffineFrame {
    origin: [f64; 3],
    u: [f64; 3],
    v: [f64; 3],
    domain: [f64; 4],
}
fn affine_frame(surface: &Surface) -> Option<AffineFrame> {
    if surface.degree_u != 1
        || surface.degree_v != 1
        || surface.control_points.len() != 2
        || surface.control_points[0].len() != 2
        || surface.periodic_u
        || surface.periodic_v
        || !surface
            .weights
            .iter()
            .flatten()
            .all(|w| *w == surface.weights[0][0])
    {
        return None;
    }
    let p = &surface.control_points;
    let origin = point3(&p[0][0]);
    let u = sub(point3(&p[1][0]), origin);
    let v = sub(point3(&p[0][1]), origin);
    if (0..3).any(|i| p[1][1][i] != origin[i] + u[i] + v[i]) {
        return None;
    }
    let n = cross(u, v);
    if dot(n, n) <= f64::MIN_POSITIVE {
        return None;
    }
    Some(AffineFrame {
        origin,
        u,
        v,
        domain: surface_domain(surface),
    })
}

fn add_trace(
    report: &mut Report<SurfacePlaneComponent>,
    parameter_box: [f64; 4],
    trace: SurfaceTrace,
    tolerance: f64,
) -> Result<()> {
    let samples = (0..=8)
        .map(|i| trace.evaluate(i as f64 / 8.))
        .collect::<Result<Vec<_>>>()?;
    let max_sample_residual = samples.iter().map(|p| p.plane_residual).fold(0., f64::max);
    if max_sample_residual > tolerance {
        report.unresolved(parameter_box, UnresolvedReason::NearCoincidence);
    } else {
        report.components.push(SurfacePlaneComponent::Curve {
            trace,
            parameter_box,
            samples,
            max_sample_residual,
        });
    }
    Ok(())
}

/// Plane sections of untrimmed NURBS patches. Affine planes and rational ruled
/// patches linear in V retain procedural parameter traces. General patches may
/// be excluded by Bernstein control bounds; unresolved regions are explicit.
/// Face trims, branch sewing, tangency certification and arbitrary SS are later
/// stages: callers must not infer a trimmed B-rep section from this report alone.
pub fn surface_plane(
    surface: &Surface,
    plane: Plane,
    options: Options,
) -> Result<Report<SurfacePlaneComponent>> {
    surface.validate()?;
    let plane = plane.normalized()?;
    let options = options.validate()?;
    let linear_u = surface.degree_u == 1;
    let linear_v = surface.degree_v == 1;
    if linear_u && !linear_v {
        let mut report = surface_plane(&transpose_surface(surface), plane, options)?;
        let swap_box = |b: &mut [f64]| {
            b.swap(0, 2);
            b.swap(1, 3);
        };
        for pending in &mut report.unresolved {
            swap_box(&mut pending.parameter_box);
        }
        for component in &mut report.components {
            match component {
                SurfacePlaneComponent::Curve {
                    trace,
                    parameter_box,
                    samples,
                    ..
                } => {
                    *trace = swap_trace(trace.clone());
                    swap_box(parameter_box);
                    for sample in samples {
                        sample.uv.swap(0, 1);
                    }
                }
                SurfacePlaneComponent::Point(point) => point.uv.swap(0, 1),
                SurfacePlaneComponent::Overlap { parameter_box, .. } => swap_box(parameter_box),
            }
        }
        return Ok(report);
    }
    let mut report = Report::default();
    if let Some(frame) = affine_frame(surface) {
        report.boxes_visited = 1;
        let domain = frame.domain;
        let uv = [
            [domain[0], domain[2]],
            [domain[1], domain[2]],
            [domain[1], domain[3]],
            [domain[0], domain[3]],
        ];
        let points = uv
            .map(|p| surface.evaluate(p[0], p[1]).map(|j| j.point))
            .into_iter()
            .collect::<Result<Vec<_>>>()?;
        let distances: Vec<_> = points.iter().map(|p| plane.distance(*p)).collect();
        if distances.iter().all(|d| *d == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            return Ok(report);
        }
        let mut hits = Vec::new();
        for i in 0..4 {
            let next = (i + 1) % 4;
            if distances[i] == 0. {
                hits.push(uv[i]);
            }
            if distances[i] * distances[next] < 0. {
                let t = distances[i] / (distances[i] - distances[next]);
                hits.push(std::array::from_fn(|d| {
                    uv[i][d] + t * (uv[next][d] - uv[i][d])
                }));
            }
        }
        hits.dedup();
        match hits.len() {
            0 => {
                report.bernstein_excluded = 1;
            }
            1 => {
                let point = surface.evaluate(hits[0][0], hits[0][1])?.point;
                report
                    .components
                    .push(SurfacePlaneComponent::Point(SurfacePoint {
                        uv: hits[0],
                        point,
                        plane_residual: plane.distance(point).abs(),
                    }));
            }
            2 => add_trace(
                &mut report,
                domain,
                SurfaceTrace::Line {
                    surface: surface.clone(),
                    plane,
                    start: hits[0],
                    end: hits[1],
                },
                options.distance_tolerance,
            )?,
            _ => report.unresolved(domain, UnresolvedReason::NearCoincidence),
        }
        return Ok(report);
    }
    let mut pending = std::collections::VecDeque::new();
    for u in spans(
        &surface.knots_u,
        surface.degree_u,
        surface.control_points.len(),
    ) {
        for v in spans(
            &surface.knots_v,
            surface.degree_v,
            surface.control_points[0].len(),
        ) {
            pending.push_back(([u[0], u[1], v[0], v[1]], 0usize));
        }
    }
    while let Some((domain, depth)) = pending.pop_front() {
        if report.boxes_visited >= options.max_boxes || report.components.len() >= 1024 {
            report.unresolved(domain, UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let piece = surface.trim(domain)?;
        let values: Vec<_> = piece
            .control_points
            .iter()
            .zip(&piece.weights)
            .flat_map(|(p, w)| {
                let c = Curve {
                    degree: 1,
                    knots: vec![],
                    control_points: p.clone(),
                    weights: w.clone(),
                    periodic: false,
                };
                coefficients(&c, plane)
            })
            .collect();
        if excluded(&values) {
            report.bernstein_excluded += 1;
            continue;
        }
        if values.iter().all(|c| c.value == 0.) {
            report.components.push(SurfacePlaneComponent::Overlap {
                parameter_box: domain,
                control_residual: 0.,
            });
            continue;
        }
        let ruled = piece.degree_v == 1 && piece.control_points[0].len() == 2;
        if !ruled {
            report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
            continue;
        }
        let lower: Vec<_> = values.iter().step_by(2).copied().collect();
        let upper: Vec<_> = values.iter().skip(1).step_by(2).copied().collect();
        let boundary_scale = piece.weights[0][1] / piece.weights[0][0];
        let proportional_boundaries = boundary_scale.is_finite()
            && boundary_scale > 0.
            && piece.weights.iter().all(|w| w[1] / w[0] == boundary_scale)
            && piece
                .control_points
                .iter()
                .all(|row| plane.distance(point3(&row[0])) == plane.distance(point3(&row[1])));
        if lower.iter().zip(&upper).all(|(a, b)| a.value == b.value) || proportional_boundaries {
            // A plane parallel to every ruling reduces to a boundary-curve
            // root query. Each root carries its parameter interval through to
            // a whole generator line; no sampled 3D curve fitting is needed.
            if report.boxes_visited == options.max_boxes {
                report.unresolved(domain, UnresolvedReason::BudgetExceeded);
                continue;
            }
            let boundary = piece.iso(nurbs_core::surface::Axis::V, domain[2])?;
            let roots = curve_plane(
                &boundary,
                plane,
                Options {
                    max_boxes: options.max_boxes - report.boxes_visited,
                    ..options
                },
            )?;
            report.boxes_visited += roots.boxes_visited;
            report.bernstein_excluded += roots.bernstein_excluded;
            for pending in roots.unresolved {
                report.unresolved(
                    [
                        pending.parameter_box[0],
                        pending.parameter_box[1],
                        domain[2],
                        domain[3],
                    ],
                    pending.reason,
                );
            }
            for component in roots.components {
                match component {
                    CurvePlaneComponent::Point(root) => {
                        add_trace(
                            &mut report,
                            [
                                root.parameter_interval[0],
                                root.parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            SurfaceTrace::Line {
                                surface: piece.clone(),
                                plane,
                                start: [root.parameter, domain[2]],
                                end: [root.parameter, domain[3]],
                            },
                            options.distance_tolerance,
                        )?;
                    }
                    CurvePlaneComponent::Overlap {
                        parameter_interval,
                        control_residual,
                    } => {
                        report.components.push(SurfacePlaneComponent::Overlap {
                            parameter_box: [
                                parameter_interval[0],
                                parameter_interval[1],
                                domain[2],
                                domain[3],
                            ],
                            control_residual,
                        });
                    }
                }
            }
            continue;
        }
        let side = |v: &[Coefficient]| {
            if v.iter().all(|c| c.bound.sign() == 1) {
                1
            } else if v.iter().all(|c| c.bound.sign() == -1) {
                -1
            } else {
                0
            }
        };
        let lower_zero = lower.iter().all(|c| c.value == 0.);
        let upper_zero = upper.iter().all(|c| c.value == 0.);
        if (lower_zero && side(&upper) != 0) || (upper_zero && side(&lower) != 0) {
            let v = if lower_zero { domain[2] } else { domain[3] };
            // A continuous shared knot is owned by the preceding V span.
            // Full-multiplicity discontinuities retain both independent sides.
            let shared_lower = lower_zero
                && v > surface_domain(surface)[2]
                && surface.knots_v.iter().filter(|k| **k == v).count() <= surface.degree_v;
            if !shared_lower {
                add_trace(
                    &mut report,
                    [domain[0], domain[1], v, v],
                    SurfaceTrace::Line {
                        surface: piece,
                        plane,
                        start: [domain[0], v],
                        end: [domain[1], v],
                    },
                    options.distance_tolerance,
                )?;
            }
            continue;
        }
        if side(&lower) * side(&upper) == -1 {
            // Full cross-ruling branch: the denominator cannot vanish because
            // the two boundary curves are uniformly on opposite plane sides.
            let trace = SurfaceTrace::Ruled {
                surface: piece,
                plane,
                u_interval: [domain[0], domain[1]],
            };
            add_trace(&mut report, domain, trace, options.distance_tolerance)?;
            continue;
        }
        let width = domain[1] - domain[0];
        let midpoint = domain[0] + width * 0.5;
        if width <= options.parameter_tolerance
            || depth == options.max_depth
            || midpoint == domain[0]
            || midpoint == domain[1]
        {
            report.unresolved(domain, UnresolvedReason::BoundaryCrossing);
            continue;
        }
        pending.push_back(([midpoint, domain[1], domain[2], domain[3]], depth + 1));
        pending.push_back(([domain[0], midpoint, domain[2], domain[3]], depth + 1));
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub enum CurveSurfaceComponent {
    Point {
        curve: CurvePoint,
        uv: [f64; 2],
        surface_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Curve/surface entry point currently admits affine rectangular planar patches.
/// Curved support surfaces return an explicit unsupported parameter region.
/// Coplanar curves are admitted only if their entire control hull lies inside
/// the rectangle. Partial coincident clipping is explicitly unresolved.
pub fn curve_surface(
    curve: &Curve,
    surface: &Surface,
    options: Options,
) -> Result<Report<CurveSurfaceComponent>> {
    curve.validate()?;
    if curve.control_points[0].len() != 3 {
        return Err(invalid("Curve/surface requires a 3D curve"));
    }
    surface.validate()?;
    let options = options.validate()?;
    let Some(frame) = affine_frame(surface) else {
        let mut report = Report::default();
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    let normal = cross(frame.u, frame.v);
    let plane = Plane {
        normal,
        offset: dot(normal, frame.origin),
    };
    let source = curve_plane(curve, plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let a = dot(frame.u, frame.u);
    let b = dot(frame.u, frame.v);
    let c = dot(frame.v, frame.v);
    let determinant = a * c - b * b;
    if determinant <= 1e-24 * a * c {
        report.unresolved(curve.domain(), UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |point: [f64; 3]| {
        let d = sub(point, frame.origin);
        let x = dot(d, frame.u);
        let y = dot(d, frame.v);
        let u = (x * c - y * b) / determinant;
        let v = (y * a - x * b) / determinant;
        [
            frame.domain[0] + u * (frame.domain[1] - frame.domain[0]),
            frame.domain[2] + v * (frame.domain[3] - frame.domain[2]),
        ]
    };
    let inside = |uv: [f64; 2]| {
        uv[0] >= frame.domain[0]
            && uv[0] <= frame.domain[1]
            && uv[1] >= frame.domain[2]
            && uv[1] <= frame.domain[3]
    };
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => {
                let uv = project(point.point);
                if point.parameter_interval[0] < point.parameter_interval[1] {
                    let interval_curve =
                        curve.trim(point.parameter_interval[0], point.parameter_interval[1])?;
                    let hull: Vec<_> = interval_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect();
                    let outside = (0..2).any(|axis| {
                        hull.iter().all(|p| p[axis] < frame.domain[2 * axis])
                            || hull.iter().all(|p| p[axis] > frame.domain[2 * axis + 1])
                    });
                    if outside {
                        continue;
                    }
                    if !hull.iter().all(|p| inside(*p)) {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::BoundaryCrossing,
                        );
                        continue;
                    }
                }
                if inside(uv) {
                    let on_surface = surface.evaluate(uv[0], uv[1])?.point;
                    let delta = sub(on_surface, point.point);
                    let residual = dot(delta, delta).sqrt();
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSurfaceComponent::Point {
                            curve: point,
                            uv,
                            surface_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
            }
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                if piece
                    .control_points
                    .iter()
                    .all(|p| inside(project(point3(p))))
                {
                    report.components.push(CurveSurfaceComponent::Overlap {
                        curve_interval: parameter_interval,
                    });
                } else {
                    let dual_u =
                        std::array::from_fn(|i| (c * frame.u[i] - b * frame.v[i]) / determinant);
                    let dual_v =
                        std::array::from_fn(|i| (a * frame.v[i] - b * frame.u[i]) / determinant);
                    let mut cuts = vec![parameter_interval[0], parameter_interval[1]];
                    let mut bands = Vec::new();
                    let mut endpoints = Vec::new();
                    for normal in [dual_u, dual_v] {
                        for boundary in [0., 1.] {
                            if report.boxes_visited >= options.max_boxes {
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::BudgetExceeded,
                                );
                                continue;
                            }
                            let roots = curve_plane(
                                &piece,
                                Plane {
                                    normal,
                                    offset: dot(normal, frame.origin) + boundary,
                                },
                                Options {
                                    max_boxes: options.max_boxes - report.boxes_visited,
                                    ..options
                                },
                            )?;
                            report.boxes_visited += roots.boxes_visited;
                            report.bernstein_excluded += roots.bernstein_excluded;
                            for unresolved in roots.unresolved {
                                let band =
                                    [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                                cuts.extend(band);
                                bands.push(band);
                                report.unresolved(band, unresolved.reason);
                            }
                            for root in roots.components {
                                match root {
                                    CurvePlaneComponent::Point(point) => {
                                        cuts.extend(point.parameter_interval);
                                        if point.parameter_interval[0]
                                            == point.parameter_interval[1]
                                        {
                                            endpoints.push(point);
                                        } else {
                                            bands.push(point.parameter_interval);
                                            report.unresolved(
                                                point.parameter_interval,
                                                UnresolvedReason::BoundaryCrossing,
                                            );
                                        }
                                    }
                                    // A curve along a rectangle edge is admissible;
                                    // the other three boundary constraints still apply.
                                    CurvePlaneComponent::Overlap {
                                        parameter_interval, ..
                                    } => cuts.extend(parameter_interval),
                                }
                            }
                        }
                    }
                    cuts.sort_by(f64::total_cmp);
                    cuts.dedup();
                    for cell in cuts.windows(2) {
                        if cell[0] == cell[1]
                            || bands.iter().any(|b| cell[0] >= b[0] && cell[1] <= b[1])
                        {
                            continue;
                        }
                        if report.boxes_visited >= options.max_boxes {
                            report.unresolved(cell.to_vec(), UnresolvedReason::BudgetExceeded);
                            continue;
                        }
                        report.boxes_visited += 1;
                        let cell_curve = curve.trim(cell[0], cell[1])?;
                        let mut hull = cell_curve
                            .control_points
                            .iter()
                            .map(|p| project(point3(p)))
                            .collect::<Vec<_>>();
                        hull[0] = project(point3(&curve.evaluate(cell[0])?.point));
                        let last = hull.len() - 1;
                        hull[last] = project(point3(&curve.evaluate(cell[1])?.point));
                        if hull.iter().all(|p| inside(*p)) {
                            report.components.push(CurveSurfaceComponent::Overlap {
                                curve_interval: [cell[0], cell[1]],
                            });
                        } else if !(0..2).any(|axis| {
                            (hull.iter().all(|p| p[axis] <= frame.domain[2 * axis])
                                && hull.iter().any(|p| p[axis] < frame.domain[2 * axis]))
                                || (hull.iter().all(|p| p[axis] >= frame.domain[2 * axis + 1])
                                    && hull.iter().any(|p| p[axis] > frame.domain[2 * axis + 1]))
                        }) {
                            report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                        }
                    }
                    for point in endpoints {
                        if report.components.iter().any(|c| match c {
                            CurveSurfaceComponent::Point { curve, .. } => {
                                curve.parameter == point.parameter
                            }
                            CurveSurfaceComponent::Overlap { curve_interval } => {
                                point.parameter >= curve_interval[0]
                                    && point.parameter <= curve_interval[1]
                            }
                        }) {
                            continue;
                        }
                        let uv = project(point.point);
                        if !inside(uv) {
                            continue;
                        }
                        let actual = surface.evaluate(uv[0], uv[1])?.point;
                        let d = sub(actual, point.point);
                        let residual = d[0].hypot(d[1]).hypot(d[2]);
                        if residual <= options.distance_tolerance {
                            report.components.push(CurveSurfaceComponent::Point {
                                curve: point,
                                uv,
                                surface_residual: residual,
                            });
                        } else {
                            report.unresolved(
                                point.parameter_interval,
                                UnresolvedReason::NearCoincidence,
                            );
                        }
                    }
                }
            }
        }
    }
    Ok(report)
}

#[derive(Clone, Debug)]
pub enum SurfaceSurfaceComponent {
    /// Implicitly closed convex boundary, ordered counterclockwise in first UV.
    Overlap {
        first_boundary: Vec<[f64; 2]>,
        second_boundary: Vec<[f64; 2]>,
        points: Vec<[f64; 3]>,
        max_sample_residual: f64,
    },
    Point {
        first: SurfacePoint,
        second: SurfacePoint,
        residual: f64,
    },
    /// Both retained traces use the same fraction in [0,1].
    Curve {
        first: SurfaceTrace,
        second: SurfaceTrace,
        max_sample_residual: f64,
    },
}

/// Finite affine-patch intersection with retained UV correspondence. General
/// curved pairs remain explicitly unresolved; coplanar affine overlap retains paired UV polygons.
pub fn surface_surface(
    first: &Surface,
    second: &Surface,
    options: Options,
) -> Result<Report<SurfaceSurfaceComponent>> {
    first.validate()?;
    second.validate()?;
    let options = options.validate()?;
    let mut report = Report::default();
    let domain = [surface_domain(first), surface_domain(second)].concat();
    let (Some(a), Some(b)) = (affine_frame(first), affine_frame(second)) else {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    };
    let normal = cross(b.u, b.v);
    let plane_b = Plane {
        normal,
        offset: dot(normal, b.origin),
    }
    .normalized()?;
    let normal = cross(a.u, a.v);
    let plane_a = Plane {
        normal,
        offset: dot(normal, a.origin),
    }
    .normalized()?;
    let source = surface_plane(first, plane_b, options)?;
    report.boxes_visited = source.boxes_visited;
    report.bernstein_excluded = source.bernstein_excluded;
    for pending in source.unresolved {
        report.unresolved(
            [pending.parameter_box, surface_domain(second).to_vec()].concat(),
            pending.reason,
        );
    }
    let uu = dot(b.u, b.u);
    let uv = dot(b.u, b.v);
    let vv = dot(b.v, b.v);
    let determinant = uu * vv - uv * uv;
    if !determinant.is_finite() || determinant <= 1e-24 * uu * vv {
        report.unresolved(domain, UnresolvedReason::UnsupportedSurface);
        return Ok(report);
    }
    let project = |p| {
        let d = sub(p, b.origin);
        let x = dot(d, b.u);
        let y = dot(d, b.v);
        [
            b.domain[0] + (x * vv - y * uv) / determinant * (b.domain[1] - b.domain[0]),
            b.domain[2] + (y * uu - x * uv) / determinant * (b.domain[3] - b.domain[2]),
        ]
    };
    for component in source.components {
        if report.boxes_visited >= options.max_boxes {
            report.unresolved(domain.clone(), UnresolvedReason::BudgetExceeded);
            continue;
        }
        report.boxes_visited += 1;
        let (start, end) = match component {
            SurfacePlaneComponent::Curve {
                trace: SurfaceTrace::Line { start, end, .. },
                ..
            } => (start, end),
            SurfacePlaneComponent::Point(point) => (point.uv, point.uv),
            SurfacePlaneComponent::Overlap { .. } => {
                let corners = [
                    [a.domain[0], a.domain[2]],
                    [a.domain[1], a.domain[2]],
                    [a.domain[1], a.domain[3]],
                    [a.domain[0], a.domain[3]],
                ];
                let mut polygon = corners
                    .into_iter()
                    .map(|uv| first.evaluate(uv[0], uv[1]).map(|p| (uv, project(p.point))))
                    .collect::<Result<Vec<_>>>()?;
                let mut failed = false;
                for (axis, boundary, greater) in [
                    (0, b.domain[0], true),
                    (0, b.domain[1], false),
                    (1, b.domain[2], true),
                    (1, b.domain[3], false),
                ] {
                    if polygon.is_empty() {
                        break;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        failed = true;
                        break;
                    }
                    report.boxes_visited += 1;
                    let inside = |p: ([f64; 2], [f64; 2])| {
                        if greater {
                            p.1[axis] >= boundary
                        } else {
                            p.1[axis] <= boundary
                        }
                    };
                    let mut clipped = Vec::new();
                    for i in 0..polygon.len() {
                        let previous = polygon[(i + polygon.len() - 1) % polygon.len()];
                        let current = polygon[i];
                        let before = inside(previous);
                        let after = inside(current);
                        if before != after {
                            let t = (boundary - previous.1[axis])
                                / (current.1[axis] - previous.1[axis]);
                            if !t.is_finite() || !(0. ..=1.).contains(&t) {
                                failed = true;
                                break;
                            }
                            let first_uv = std::array::from_fn(|j| {
                                previous.0[j] + t * (current.0[j] - previous.0[j])
                            });
                            let mut second_uv = std::array::from_fn(|j| {
                                previous.1[j] + t * (current.1[j] - previous.1[j])
                            });
                            second_uv[axis] = boundary;
                            clipped.push((first_uv, second_uv));
                        }
                        if after {
                            clipped.push(current);
                        }
                    }
                    if failed {
                        break;
                    }
                    clipped.dedup_by(|a, b| a.0 == b.0);
                    if clipped.len() > 1 && clipped.first().unwrap().0 == clipped.last().unwrap().0
                    {
                        clipped.pop();
                    }
                    polygon = clipped;
                }
                if failed {
                    report.unresolved(
                        domain.clone(),
                        if report.boxes_visited >= options.max_boxes {
                            UnresolvedReason::BudgetExceeded
                        } else {
                            UnresolvedReason::NearCoincidence
                        },
                    );
                    continue;
                }
                if polygon.is_empty() {
                    continue;
                }
                if polygon.len() >= 3 {
                    let origin = polygon[0].0;
                    let area = polygon
                        .windows(2)
                        .skip(1)
                        .map(|p| {
                            let x = [p[0].0[0] - origin[0], p[0].0[1] - origin[1]];
                            let y = [p[1].0[0] - origin[0], p[1].0[1] - origin[1]];
                            x[0] * y[1] - x[1] * y[0]
                        })
                        .sum::<f64>();
                    if !area.is_finite() || area <= 0. {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    let mut points = Vec::new();
                    let mut residual = 0_f64;
                    for (u, v) in &polygon {
                        let valid = |p: [f64; 2], d: [f64; 4]| {
                            (0..2).all(|i| {
                                p[i].is_finite() && p[i] >= d[2 * i] && p[i] <= d[2 * i + 1]
                            })
                        };
                        if !valid(*u, a.domain) || !valid(*v, b.domain) {
                            residual = f64::INFINITY;
                            break;
                        }
                        let p = first.evaluate(u[0], u[1])?.point;
                        let q = second.evaluate(v[0], v[1])?.point;
                        let d = sub(p, q);
                        residual = residual.max(d[0].hypot(d[1]).hypot(d[2]));
                        points.push(p);
                    }
                    if residual > options.distance_tolerance {
                        report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    report.components.push(SurfaceSurfaceComponent::Overlap {
                        first_boundary: polygon.iter().map(|p| p.0).collect(),
                        second_boundary: polygon.iter().map(|p| p.1).collect(),
                        points,
                        max_sample_residual: residual,
                    });
                    continue;
                }
                (polygon[0].0, polygon.last().unwrap().0)
            }
            _ => {
                report.unresolved(domain.clone(), UnresolvedReason::UnsupportedSurface);
                continue;
            }
        };
        let p = first.evaluate(start[0], start[1])?.point;
        let q = first.evaluate(end[0], end[1])?.point;
        let u = project(p);
        let v = project(q);
        let mut low = 0_f64;
        let mut high = 1_f64;
        let mut invalid_numeric = false;
        for axis in 0..2 {
            let delta = v[axis] - u[axis];
            let min = b.domain[2 * axis];
            let max = b.domain[2 * axis + 1];
            if !delta.is_finite() || !u[axis].is_finite() {
                invalid_numeric = true;
                break;
            }
            if delta == 0. {
                if u[axis] < min || u[axis] > max {
                    high = -1.;
                    break;
                }
            } else {
                let x = (min - u[axis]) / delta;
                let y = (max - u[axis]) / delta;
                if !x.is_finite() || !y.is_finite() {
                    invalid_numeric = true;
                    break;
                }
                low = low.max(x.min(y));
                high = high.min(x.max(y));
            }
        }
        if invalid_numeric {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low > high {
            continue;
        }
        let interpolate =
            |x: [f64; 2], y: [f64; 2], t: f64| std::array::from_fn(|i| x[i] + t * (y[i] - x[i]));
        let first_trace = SurfaceTrace::Line {
            surface: first.clone(),
            plane: plane_b,
            start: interpolate(start, end, low),
            end: interpolate(start, end, high),
        };
        let second_trace = SurfaceTrace::Line {
            surface: second.clone(),
            plane: plane_a,
            start: interpolate(u, v, low),
            end: interpolate(u, v, high),
        };
        let mut max_residual = 0_f64;
        let mut sample = None;
        for i in 0..=8 {
            let (Ok(a), Ok(b)) = (
                first_trace.evaluate(i as f64 / 8.),
                second_trace.evaluate(i as f64 / 8.),
            ) else {
                max_residual = f64::INFINITY;
                break;
            };
            let d = sub(a.point, b.point);
            let residual = d[0].hypot(d[1]).hypot(d[2]);
            max_residual = max_residual
                .max(residual)
                .max(a.plane_residual)
                .max(b.plane_residual);
            sample = Some((a, b, residual));
        }
        if max_residual > options.distance_tolerance {
            report.unresolved(domain.clone(), UnresolvedReason::NearCoincidence);
            continue;
        }
        if low == high || start == end {
            let (first, second, residual) = sample.unwrap();
            report.components.push(SurfaceSurfaceComponent::Point {
                first,
                second,
                residual,
            });
        } else {
            report.components.push(SurfaceSurfaceComponent::Curve {
                first: first_trace,
                second: second_trace,
                max_sample_residual: max_residual,
            });
        }
    }
    Ok(report)
}

fn invalid(message: &str) -> Error {
    Error::new("BREP_INTERSECTION_INVALID_INPUT", message)
}
/// Numerical curve/finite-segment correspondence. Overlaps retain the original
/// curve interval; degree-one coincidences are clipped in source parameters.
/// Higher-degree clipping isolates segment-boundary roots and retains unresolved bands.
#[derive(Clone, Debug)]
pub enum CurveSegmentComponent {
    Point {
        curve: CurvePoint,
        segment_parameter: f64,
        line_residual: f64,
    },
    Overlap {
        curve_interval: [f64; 2],
    },
}

/// Intersect a retained NURBS curve with a finite 3D segment using two supporting
/// planes. Both plane queries share max_boxes; results remain uncertified and
/// cannot authorize topology changes. No sampled polyline defines the curve.
pub fn curve_segment(
    curve: &Curve,
    start: [f64; 3],
    end: [f64; 3],
    options: Options,
) -> Result<Report<CurveSegmentComponent>> {
    curve.validate()?;
    let options = options.validate()?;
    if curve.control_points[0].len() != 3 || !start.iter().chain(&end).all(|x| x.is_finite()) {
        return Err(invalid("Curve/segment requires finite 3D inputs"));
    }
    let delta = sub(end, start);
    let length = delta[0].hypot(delta[1]).hypot(delta[2]);
    if !length.is_finite() || length == 0. {
        return Err(invalid("Segment must have finite nonzero length"));
    }
    let direction = delta.map(|x| x / length);
    let axis = (0..3)
        .min_by(|a, b| direction[*a].abs().total_cmp(&direction[*b].abs()))
        .unwrap();
    let mut basis = [0.; 3];
    basis[axis] = 1.;
    let n = cross(direction, basis);
    let first_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let n = cross(direction, first_plane.normal);
    let second_plane = Plane {
        normal: n,
        offset: dot(n, start),
    }
    .normalized()?;
    let source = curve_plane(curve, first_plane, options)?;
    let mut report = Report {
        components: vec![],
        unresolved: source.unresolved,
        boxes_visited: source.boxes_visited,
        bernstein_excluded: source.bernstein_excluded,
        coverage: source.coverage,
    };
    let mut candidates = Vec::new();
    for component in source.components {
        match component {
            CurvePlaneComponent::Point(point) => candidates.push(CurvePlaneComponent::Point(point)),
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => {
                if report.boxes_visited >= options.max_boxes {
                    report.unresolved(parameter_interval, UnresolvedReason::BudgetExceeded);
                    continue;
                }
                let piece = curve.trim(parameter_interval[0], parameter_interval[1])?;
                let other = curve_plane(
                    &piece,
                    second_plane,
                    Options {
                        max_boxes: options.max_boxes - report.boxes_visited,
                        ..options
                    },
                )?;
                report.boxes_visited += other.boxes_visited;
                report.bernstein_excluded += other.bernstein_excluded;
                if other.coverage == Coverage::Incomplete {
                    report.coverage = Coverage::Incomplete;
                }
                report.unresolved.extend(other.unresolved);
                candidates.extend(other.components);
            }
        }
    }
    let project = |p| dot(sub(p, start), direction) / length;
    for candidate in candidates {
        let interval = match &candidate {
            CurvePlaneComponent::Point(p) => p.parameter_interval,
            CurvePlaneComponent::Overlap {
                parameter_interval, ..
            } => *parameter_interval,
        };
        let hull = if interval[0] < interval[1] {
            curve
                .trim(interval[0], interval[1])?
                .control_points
                .iter()
                .map(|p| point3(p))
                .collect::<Vec<_>>()
        } else {
            vec![point3(&curve.evaluate(interval[0])?.point)]
        };
        let parameters = hull.iter().map(|p| project(*p)).collect::<Vec<_>>();
        if parameters.iter().all(|t| *t < 0.) || parameters.iter().all(|t| *t > 1.) {
            continue;
        }
        let distances = hull
            .iter()
            .map(|p| second_plane.distance(*p))
            .collect::<Vec<_>>();
        if distances.iter().all(|d| *d > options.distance_tolerance)
            || distances.iter().all(|d| *d < -options.distance_tolerance)
        {
            continue;
        }
        if !parameters
            .iter()
            .all(|t| t.is_finite() && (0. ..=1.).contains(t))
        {
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. })
                && curve.degree == 1
                && parameters.len() == 2
                && parameters.iter().all(|t| t.is_finite())
            {
                let piece = curve.trim(interval[0], interval[1])?;
                let [a, b] = [parameters[0], parameters[1]];
                let low = a.min(b).max(0.);
                let high = a.max(b).min(1.);
                let scale = piece.weights[0].max(piece.weights[1]);
                let [wa, wb] = [piece.weights[0] / scale, piece.weights[1] / scale];
                let parameter = |q: f64| {
                    let r = if q == a {
                        0.
                    } else if q == b {
                        1.
                    } else {
                        let numerator = wa * (q - a);
                        numerator / (wb * (b - q) + numerator)
                    };
                    interval[0] + r * (interval[1] - interval[0])
                };
                let [x, y] = [parameter(low), parameter(high)];
                if !x.is_finite()
                    || !y.is_finite()
                    || x < interval[0]
                    || x > interval[1]
                    || y < interval[0]
                    || y > interval[1]
                {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                let clipped = [x.min(y), x.max(y)];
                if clipped[0] == clipped[1] && low != high {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                    continue;
                }
                if clipped[0] == clipped[1] {
                    let point = point3(&curve.evaluate(clipped[0])?.point);
                    let t = low;
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual > options.distance_tolerance {
                        report.unresolved(interval, UnresolvedReason::NearCoincidence);
                        continue;
                    }
                    if report.components.iter().any(|c| {
                        matches!(c,CurveSegmentComponent::Point {curve,..}
                        if curve.parameter==clipped[0])
                    }) {
                        continue;
                    }
                    report.components.push(CurveSegmentComponent::Point {
                        curve: CurvePoint {
                            parameter: clipped[0],
                            parameter_interval: clipped,
                            point,
                            plane_residual: first_plane.distance(point).abs(),
                            contact: Contact::Boundary,
                        },
                        segment_parameter: t,
                        line_residual: residual,
                    });
                } else {
                    report.components.push(CurveSegmentComponent::Overlap {
                        curve_interval: clipped,
                    });
                }
                continue;
            }
            if matches!(&candidate, CurvePlaneComponent::Overlap { .. }) {
                let piece = curve.trim(interval[0], interval[1])?;
                let mut cuts = vec![interval[0], interval[1]];
                let mut bands = Vec::new();
                let mut endpoints = Vec::new();
                for (position, segment_parameter) in [(start, 0.), (end, 1.)] {
                    if report.boxes_visited >= options.max_boxes {
                        bands.push(interval);
                        report.unresolved(interval, UnresolvedReason::BudgetExceeded);
                        break;
                    }
                    let boundary = curve_plane(
                        &piece,
                        Plane {
                            normal: direction,
                            offset: dot(direction, position),
                        },
                        Options {
                            max_boxes: options.max_boxes - report.boxes_visited,
                            ..options
                        },
                    )?;
                    report.boxes_visited += boundary.boxes_visited;
                    report.bernstein_excluded += boundary.bernstein_excluded;
                    for unresolved in boundary.unresolved {
                        let band = [unresolved.parameter_box[0], unresolved.parameter_box[1]];
                        cuts.extend(band);
                        bands.push(band);
                        report.unresolved(band, unresolved.reason);
                    }
                    for component in boundary.components {
                        match component {
                            CurvePlaneComponent::Point(point) => {
                                cuts.extend(point.parameter_interval);
                                if point.parameter_interval[0] == point.parameter_interval[1] {
                                    endpoints.push((point, segment_parameter));
                                } else {
                                    bands.push(point.parameter_interval);
                                    report.unresolved(
                                        point.parameter_interval,
                                        UnresolvedReason::BoundaryCrossing,
                                    );
                                }
                            }
                            CurvePlaneComponent::Overlap {
                                parameter_interval, ..
                            } => {
                                cuts.extend(parameter_interval);
                                bands.push(parameter_interval);
                                report.unresolved(
                                    parameter_interval,
                                    UnresolvedReason::CoincidentTrim,
                                );
                            }
                        }
                    }
                }
                cuts.sort_by(f64::total_cmp);
                cuts.dedup();
                for cell in cuts.windows(2) {
                    if cell[0] == cell[1]
                        || bands.iter().any(|b| cell[0] >= b[0] && cell[1] <= b[1])
                    {
                        continue;
                    }
                    if report.boxes_visited >= options.max_boxes {
                        report.unresolved(cell.to_vec(), UnresolvedReason::BudgetExceeded);
                        continue;
                    }
                    report.boxes_visited += 1;
                    let cell_curve = curve.trim(cell[0], cell[1])?;
                    let mut projection = cell_curve
                        .control_points
                        .iter()
                        .map(|p| project(point3(p)))
                        .collect::<Vec<_>>();
                    // Knot insertion can round the trim's endpoint controls.
                    // Query the authoritative source at the retained cut parameters.
                    projection[0] = project(point3(&curve.evaluate(cell[0])?.point));
                    let last = projection.len() - 1;
                    projection[last] = project(point3(&curve.evaluate(cell[1])?.point));

                    if projection.iter().all(|t| *t <= 0.) || projection.iter().all(|t| *t >= 1.) {
                        continue;
                    }
                    if projection
                        .iter()
                        .all(|t| t.is_finite() && (0. ..=1.).contains(t))
                    {
                        report.components.push(CurveSegmentComponent::Overlap {
                            curve_interval: [cell[0], cell[1]],
                        });
                    } else {
                        report.unresolved(cell.to_vec(), UnresolvedReason::CoincidentTrim);
                    }
                }
                for (point, t) in endpoints {
                    let covered = report.components.iter().any(|c| match c {
                        CurveSegmentComponent::Point { curve, .. } => {
                            curve.parameter == point.parameter
                        }
                        CurveSegmentComponent::Overlap { curve_interval } => {
                            point.parameter >= curve_interval[0]
                                && point.parameter <= curve_interval[1]
                        }
                    });
                    if covered {
                        continue;
                    }
                    let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                    let d = sub(point.point, on_line);
                    let residual = d[0].hypot(d[1]).hypot(d[2]);
                    if residual <= options.distance_tolerance {
                        report.components.push(CurveSegmentComponent::Point {
                            curve: point,
                            segment_parameter: t,
                            line_residual: residual,
                        });
                    } else {
                        report.unresolved(
                            point.parameter_interval,
                            UnresolvedReason::NearCoincidence,
                        );
                    }
                }
                continue;
            }
            report.unresolved(
                interval,
                match candidate {
                    CurvePlaneComponent::Overlap { .. } => UnresolvedReason::CoincidentTrim,
                    _ => UnresolvedReason::BoundaryCrossing,
                },
            );
            continue;
        }
        match candidate {
            CurvePlaneComponent::Point(point) => {
                let t = project(point.point);
                let on_line = std::array::from_fn(|i| start[i] + t * delta[i]);
                let d = sub(point.point, on_line);
                let residual = d[0].hypot(d[1]).hypot(d[2]);
                if residual <= options.distance_tolerance {
                    if !report.components.iter().any(|c|matches!(c,CurveSegmentComponent::Point {curve,..} if curve.parameter==point.parameter)) {
                        report.components.push(CurveSegmentComponent::Point {curve:point,segment_parameter:t,line_residual:residual});
                    }
                } else {
                    report.unresolved(interval, UnresolvedReason::NearCoincidence);
                }
            }
            CurvePlaneComponent::Overlap { .. } => {
                report.components.push(CurveSegmentComponent::Overlap {
                    curve_interval: interval,
                })
            }
        }
    }
    report.components.sort_by(|a, b| {
        let parameter = |c: &CurveSegmentComponent| match c {
            CurveSegmentComponent::Point { curve, .. } => curve.parameter,
            CurveSegmentComponent::Overlap { curve_interval } => curve_interval[0],
        };
        parameter(a).total_cmp(&parameter(b))
    });
    Ok(report)
}

fn point3(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|i| a[i] - b[i])
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

impl value_codec::Serialize for Plane {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"normal":self.normal,"offset":self.offset})
    }
}
impl<'de> value_codec::Deserialize<'de> for Plane {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        Ok(Self {
            normal: value_codec::from_value(value["normal"].clone())?,
            offset: value_codec::from_value(value["offset"].clone())?,
        })
    }
}
impl value_codec::Serialize for Options {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"distanceTolerance":self.distance_tolerance,"parameterTolerance":self.parameter_tolerance,
            "maxDepth":self.max_depth,"maxBoxes":self.max_boxes})
    }
}
impl<'de> value_codec::Deserialize<'de> for Options {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        if !value.is_object() {
            return Err(value_codec::error("Intersection options must be an object"));
        }
        let mut options = Self::default();
        if let Some(v) = value.get("distanceTolerance") {
            options.distance_tolerance = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("parameterTolerance") {
            options.parameter_tolerance = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("maxDepth") {
            options.max_depth = value_codec::from_value(v.clone())?;
        }
        if let Some(v) = value.get("maxBoxes") {
            options.max_boxes = value_codec::from_value(v.clone())?;
        }
        Ok(options)
    }
}
impl value_codec::Serialize for Unresolved {
    fn to_value(&self) -> value_codec::Value {
        let reason = match self.reason {
            UnresolvedReason::BudgetExceeded => "budget_exceeded",
            UnresolvedReason::TangencyOrMultipleRoot => "tangency_or_multiple_root",
            UnresolvedReason::NearCoincidence => "near_coincidence",
            UnresolvedReason::BoundaryCrossing => "boundary_crossing",
            UnresolvedReason::UnsupportedSurface => "unsupported_surface",
            UnresolvedReason::CoincidentTrim => "coincident_trim",
        };
        value_codec::json!({"parameterBox":self.parameter_box,"reason":reason})
    }
}
impl<T: value_codec::Serialize> value_codec::Serialize for Report<T> {
    fn to_value(&self) -> value_codec::Value {
        let coverage = match self.coverage {
            Coverage::NumericallyResolved => "numerically_resolved",
            Coverage::Incomplete => "incomplete",
        };
        value_codec::json!({"components":self.components,"unresolved":self.unresolved,"boxesVisited":self.boxes_visited,
            "bernsteinExcluded":self.bernstein_excluded,"coverage":coverage,"permitsTopologyChange":false,
            "evidence":"numerical_uncertified"})
    }
}
impl value_codec::Serialize for CurvePoint {
    fn to_value(&self) -> value_codec::Value {
        let contact = match self.contact {
            Contact::Transverse => "transverse",
            Contact::Boundary => "boundary",
        };
        value_codec::json!({"parameter":self.parameter,"parameterInterval":self.parameter_interval,"point":self.point,
            "planeResidual":self.plane_residual,"contact":contact})
    }
}
impl value_codec::Serialize for CurvePlaneComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point(point) => value_codec::json!({"kind":"point","curve":point}),
            Self::Overlap {
                parameter_interval,
                control_residual,
            } => {
                value_codec::json!({"kind":"overlap","parameterInterval":parameter_interval,"controlResidual":control_residual})
            }
        }
    }
}
impl value_codec::Serialize for SurfacePoint {
    fn to_value(&self) -> value_codec::Value {
        value_codec::json!({"uv":self.uv,"point":self.point,"planeResidual":self.plane_residual})
    }
}
impl value_codec::Serialize for SurfaceTrace {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::RuledU {
                surface,
                plane,
                v_interval,
            } => {
                value_codec::json!({"kind":"ruled_u","surface":surface,"plane":plane,"vInterval":v_interval})
            }

            Self::Line {
                surface,
                plane,
                start,
                end,
            } => {
                value_codec::json!({"kind":"line","surface":surface,"plane":plane,"start":start,"end":end})
            }
            Self::Ruled {
                surface,
                plane,
                u_interval,
            } => {
                value_codec::json!({"kind":"ruled","surface":surface,"plane":plane,"uInterval":u_interval})
            }
        }
    }
}
impl<'de> value_codec::Deserialize<'de> for SurfaceTrace {
    fn from_value(value: value_codec::Value) -> value_codec::Result<Self> {
        let surface = value_codec::from_value(value["surface"].clone())?;
        let plane = value_codec::from_value(value["plane"].clone())?;
        match value["kind"].as_str() {
            Some("line") => Ok(Self::Line {
                surface,
                plane,
                start: value_codec::from_value(value["start"].clone())?,
                end: value_codec::from_value(value["end"].clone())?,
            }),
            Some("ruled_u") => Ok(Self::RuledU {
                surface,
                plane,
                v_interval: value_codec::from_value(value["vInterval"].clone())?,
            }),
            Some("ruled") => Ok(Self::Ruled {
                surface,
                plane,
                u_interval: value_codec::from_value(value["uInterval"].clone())?,
            }),
            _ => Err(value_codec::error(
                "Unknown surface intersection trace kind",
            )),
        }
    }
}
impl value_codec::Serialize for SurfacePlaneComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Curve {
                trace,
                parameter_box,
                samples,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"curve","trace":trace,"parameterBox":parameter_box,"samples":samples,"maxSampleResidual":max_sample_residual})
            }
            Self::Point(point) => value_codec::json!({"kind":"point","surface":point}),
            Self::Overlap {
                parameter_box,
                control_residual,
            } => {
                value_codec::json!({"kind":"overlap","parameterBox":parameter_box,"controlResidual":control_residual})
            }
        }
    }
}
impl value_codec::Serialize for SurfaceSurfaceComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Overlap {
                first_boundary,
                second_boundary,
                points,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"overlap","firstBoundary":first_boundary,"secondBoundary":second_boundary,"points":points,"maxSampleResidual":max_sample_residual})
            }
            Self::Point {
                first,
                second,
                residual,
            } => {
                value_codec::json!({"kind":"point","first":first,"second":second,"residual":residual})
            }
            Self::Curve {
                first,
                second,
                max_sample_residual,
            } => {
                value_codec::json!({"kind":"curve","first":first,"second":second,"maxSampleResidual":max_sample_residual})
            }
        }
    }
}
impl value_codec::Serialize for CurveSegmentComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                curve,
                segment_parameter,
                line_residual,
            } => {
                value_codec::json!({"kind":"point","curve":curve,"segmentParameter":segment_parameter,"lineResidual":line_residual})
            }
            Self::Overlap { curve_interval } => {
                value_codec::json!({"kind":"overlap","curveInterval":curve_interval})
            }
        }
    }
}
impl value_codec::Serialize for CurveSurfaceComponent {
    fn to_value(&self) -> value_codec::Value {
        match self {
            Self::Point {
                curve,
                uv,
                surface_residual,
            } => {
                value_codec::json!({"kind":"point","curve":curve,"uv":uv,"surfaceResidual":surface_residual})
            }
            Self::Overlap { curve_interval } => {
                value_codec::json!({"kind":"overlap","curveInterval":curve_interval})
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn indeterminate_interval_arithmetic_never_proves_a_sign() {
        let a = Interval {
            lo: 0.,
            hi: f64::INFINITY,
        }
        .mul(Interval::exact(0.));
        let b = Interval::exact(f64::INFINITY).add(Interval::exact(f64::NEG_INFINITY));
        for bound in [a, b] {
            assert_eq!(bound.lo, f64::NEG_INFINITY);
            assert_eq!(bound.hi, f64::INFINITY);
            assert_eq!(bound.sign(), 0);
        }
    }
    #[test]
    fn ruled_range_subdivision_admits_interior_and_boundary_but_refuses_uncertainty() {
        // Mixed control signs need subdivision, although the polynomial stays negative.
        assert!(admit_ruled_parameter_range(&[[-1., 1.], [0.25, 2.25], [-1., 1.]]).is_ok());
        assert!(admit_ruled_parameter_range(&[[0., 2.]; 3]).is_ok());
        assert!(admit_ruled_parameter_range(&[[2., 0.]; 3]).is_ok());
        // Exact dyadic cancellation resolves this boundary tangency without
        // manufacturing an interval error for 0+x, 1*x or x+(-x).
        assert!(admit_ruled_parameter_range(&[[-1., 1.], [1., 3.], [-1., 1.]]).is_ok());
        let uncertain = admit_ruled_parameter_bounds(vec![
            [
                Interval {
                    lo: -1e-16,
                    hi: 1e-16
                },
                Interval::exact(2.)
            ];
            3
        ])
        .unwrap_err();
        assert_eq!(uncertain.code, "BREP_INTERSECTION_UNRESOLVED");
        assert!(uncertain.to_string().contains("subdivision limit"));
        assert!(admit_ruled_parameter_range(&[[f64::INFINITY, 0.]; 3]).is_err());
    }
    #[test]
    fn residual_construction_cancellation_cannot_authorize_a_ruled_trace() {
        let source = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..2)
                .map(|i| {
                    vec![
                        vec![268435456., 1e-8, i as f64],
                        vec![268435456., 4., i as f64],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        // The exact input equation x+y=268435456 has no point on this surface.
        // Rounded point residuals nevertheless vanish on the lower boundary.
        for (axis, surface) in [source.clone(), transpose_surface(&source)]
            .into_iter()
            .enumerate()
        {
            let plane = Plane {
                normal: [1., 1., 0.],
                offset: 268435456.,
            };
            let trace = if axis == 0 {
                SurfaceTrace::Ruled {
                    surface,
                    plane,
                    u_interval: [0., 1.],
                }
            } else {
                SurfaceTrace::RuledU {
                    surface,
                    plane,
                    v_interval: [0., 1.],
                }
            };
            assert_eq!(trace.evaluate(0.5).unwrap().plane_residual, 0.);
            assert_eq!(
                trace.to_curve().unwrap_err().code,
                "BREP_INTERSECTION_UNRESOLVED"
            );
            assert_eq!(
                trace.to_curve_segments().unwrap_err().code,
                "BREP_INTERSECTION_UNRESOLVED"
            );
        }
    }
    #[test]
    fn plane_normalization_preserves_extreme_equivalent_equations() {
        for scale in [f64::from_bits(1), f64::MIN_POSITIVE, 1., f64::MAX] {
            let p = Plane {
                normal: [scale, scale, 0.],
                offset: scale,
            }
            .normalized()
            .unwrap();
            assert!((p.normal[0].hypot(p.normal[1]) - 1.).abs() < 3e-16);
            assert!((p.offset - std::f64::consts::FRAC_1_SQRT_2).abs() < 2e-16);
            assert_eq!(p.distance([1., 0., 0.]), 0.);
        }
        let p = Plane {
            normal: [f64::from_bits(1); 3],
            offset: f64::from_bits(2),
        }
        .normalized()
        .unwrap();
        assert!((p.offset - 2. / 3_f64.sqrt()).abs() < 3e-16);
        // offset/scale overflows here, while the normalized offset is finite.
        let p = Plane {
            normal: [0.75; 3],
            offset: f64::MAX,
        }
        .normalized()
        .unwrap();
        assert!(p.offset.is_finite());
        assert!((p.offset / f64::MAX - 1. / (0.75 * 3_f64.sqrt())).abs() < 3e-16);
        assert!(
            Plane {
                normal: [f64::from_bits(1), 0., 0.],
                offset: 1.
            }
            .normalized()
            .is_err()
        );
        assert!(
            Plane {
                normal: [f64::NAN, 1., 0.],
                offset: 0.
            }
            .normalized()
            .is_err()
        );
    }
    #[test]
    fn curve_plane_roots_are_invariant_under_extreme_equation_scaling() {
        let source = Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 0.]]).unwrap();
        for magnitude in [f64::from_bits(1), f64::MIN_POSITIVE, 1., f64::MAX] {
            for sign in [-1., 1.] {
                let scale = sign * magnitude;
                let report = curve_plane(
                    &source,
                    Plane {
                        normal: [scale, scale, 0.],
                        offset: scale,
                    },
                    Options::default(),
                )
                .unwrap();
                assert_eq!(report.coverage, Coverage::NumericallyResolved);
                let roots = points(&report);
                assert_eq!(roots.len(), 1);
                assert_eq!(roots[0].parameter, 0.5);
                assert_eq!(roots[0].point, [1., 0., 0.]);
                assert_eq!(roots[0].plane_residual, 0.);
                assert!(!report.permits_topology_change());
            }
        }
    }
    fn plane(z: f64) -> Plane {
        Plane {
            normal: [0., 0., 1.],
            offset: z,
        }
    }
    fn bezier(z: &[f64]) -> Curve {
        let degree = z.len() - 1;
        Curve {
            degree,
            knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
            control_points: z
                .iter()
                .enumerate()
                .map(|(i, z)| vec![i as f64 / degree as f64, 0., *z])
                .collect(),
            weights: vec![1.; z.len()],
            periodic: false,
        }
    }
    fn points(report: &Report<CurvePlaneComponent>) -> Vec<&CurvePoint> {
        report
            .components
            .iter()
            .filter_map(|c| {
                if let CurvePlaneComponent::Point(p) = c {
                    Some(p)
                } else {
                    None
                }
            })
            .collect()
    }
    #[test]
    fn roots_are_isolated_without_seed_sampling_and_residuals_are_checked() {
        // z(t) = (t-.2)(t-.5)(t-.8): three intersections, including
        // an exact subdivision boundary root that must not be duplicated.
        let curve = bezier(&[-0.08, 0.14, -0.14, 0.08]);
        let report = curve_plane(&curve, plane(0.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let roots = points(&report);
        assert_eq!(roots.len(), 3, "{report:?}");
        for (root, expected) in roots.iter().zip([0.2, 0.5, 0.8]) {
            assert!((root.parameter - expected).abs() < 1e-8);
            assert!(root.plane_residual < 1e-9);
            assert!(
                root.parameter_interval[0] <= expected + 1e-14
                    && root.parameter_interval[1] >= expected - 1e-14
            );
        }
        assert!(!report.permits_topology_change());
    }
    #[test]
    fn rational_arc_plane_query_retains_curve_parameters() {
        let body = crate::cylinder(2., 3.).unwrap();
        let arc = &body
            .edges
            .iter()
            .find(|e| e.curve.degree == 2)
            .unwrap()
            .curve;
        let report = curve_plane(
            arc,
            Plane {
                normal: [1., 0., 0.],
                offset: 1.,
            },
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let roots = points(&report);
        assert_eq!(roots.len(), 1);
        let p = roots[0].point;
        assert!((p[0] - 1.).abs() < 1e-9);
        assert!((p[0] * p[0] + p[1] * p[1] - 4.).abs() < 1e-10);
    }
    #[test]
    fn tangencies_and_work_exhaustion_are_not_reported_as_empty() {
        let tangent =
            curve_plane(&bezier(&[0.25, -0.25, 0.25]), plane(0.), Options::default()).unwrap();
        assert_eq!(tangent.coverage, Coverage::Incomplete);
        assert!(
            tangent
                .unresolved
                .iter()
                .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot)
        );
        let limited = curve_plane(
            &bezier(&[-0.08, 0.14, -0.14, 0.08]),
            plane(0.),
            Options {
                max_boxes: 1,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(limited.coverage, Coverage::Incomplete);
        assert_eq!(limited.boxes_visited, 1);
        assert!(
            limited
                .unresolved
                .iter()
                .all(|u| u.reason == UnresolvedReason::BudgetExceeded)
        );
        assert!(
            curve_plane(&bezier(&[1., 2.]), plane(0.), Options::default())
                .unwrap()
                .components
                .is_empty()
        );
        assert!(matches!(
            curve_plane(&bezier(&[0., 0.]), plane(0.), Options::default())
                .unwrap()
                .components[0],
            CurvePlaneComponent::Overlap { .. }
        ));
    }
    #[test]
    fn ruled_cylinder_sections_preserve_exact_procedural_traces() {
        let body = crate::cylinder(2., 4.).unwrap();
        let surface = &body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface;
        for plane in [
            plane(1.5),
            Plane {
                normal: [0.2, -0.1, 1.],
                offset: 2.,
            },
        ] {
            let report = surface_plane(surface, plane, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            assert_eq!(report.components.len(), 1);
            let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
                panic!("Expected a section curve")
            };
            for i in 0..101 {
                let p = trace.evaluate(i as f64 / 100.).unwrap();
                assert!(p.plane_residual < 1e-12);
                assert!((p.point[0] * p.point[0] + p.point[1] * p.point[1] - 4.).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn ruled_u_sections_preserve_source_uv_and_serialized_traces() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut original = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        for knot in &mut original.knots_u {
            *knot = 3. + 2. * *knot;
        }
        for knot in &mut original.knots_v {
            *knot = -4. + 7. * *knot;
        }
        let source = transpose_surface(&original);
        for plane in [
            plane(1.5),
            Plane {
                normal: [0.2, -0.1, 1.],
                offset: 2.,
            },
            Plane {
                normal: [1., 0., 0.],
                offset: 1.,
            },
        ] {
            let a = surface_plane(&original, plane, Options::default()).unwrap();
            let b = surface_plane(&source, plane, Options::default()).unwrap();
            assert_eq!(a.coverage, b.coverage);
            assert_eq!(a.components.len(), b.components.len());
            assert!(!b.components.is_empty());
            for (a, b) in a.components.iter().zip(&b.components) {
                let (
                    SurfacePlaneComponent::Curve {
                        trace: ta,
                        parameter_box: ba,
                        ..
                    },
                    SurfacePlaneComponent::Curve {
                        trace: tb,
                        parameter_box: bb,
                        ..
                    },
                ) = (a, b)
                else {
                    panic!("Expected traces");
                };
                assert_eq!(*bb, [ba[2], ba[3], ba[0], ba[1]]);
                let wire = value_codec::to_string(tb).unwrap();
                let loaded: SurfaceTrace = value_codec::from_str_strict(&wire).unwrap();
                for i in 0..=20 {
                    let fraction = i as f64 / 20.;
                    let a = ta.evaluate(fraction).unwrap();
                    let b = loaded.evaluate(fraction).unwrap();
                    assert_eq!(b.uv, [a.uv[1], a.uv[0]]);
                    assert!(
                        a.point
                            .iter()
                            .zip(b.point)
                            .all(|(a, b)| (a - b).abs() < 1e-12)
                    );
                    let source_point = source.evaluate(b.uv[0], b.uv[1]).unwrap().point;
                    assert!(
                        source_point
                            .iter()
                            .zip(b.point)
                            .all(|(a, b)| (a - b).abs() < 1e-11)
                    );
                    assert!(b.plane_residual < 1e-9);
                }
            }
        }
        let plane = Plane {
            normal: [0.2, -0.1, 1.],
            offset: 0.,
        };
        let options = Options {
            max_boxes: 1,
            ..Options::default()
        };
        let a = surface_plane(&original, plane, options).unwrap();
        let b = surface_plane(&source, plane, options).unwrap();
        assert_eq!(a.coverage, Coverage::Incomplete);
        assert_eq!(a.unresolved.len(), b.unresolved.len());
        for (a, b) in a.unresolved.iter().zip(&b.unresolved) {
            assert_eq!(a.reason, b.reason);
            assert_eq!(
                b.parameter_box,
                vec![
                    a.parameter_box[2],
                    a.parameter_box[3],
                    a.parameter_box[0],
                    a.parameter_box[1]
                ]
            );
        }
    }
    #[test]
    fn trace_evaluation_admits_the_whole_parameter_definition() {
        let body = crate::cylinder(2., 4.).unwrap();
        let surface = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        let domain = surface_domain(&surface);
        for bad in [f64::NAN, f64::INFINITY, domain[1] + 1., domain[0] - 1.] {
            let trace = SurfaceTrace::Ruled {
                surface: surface.clone(),
                plane: plane(2.),
                u_interval: [domain[0], bad],
            };
            // Even fraction zero must admit the unused endpoint.
            assert!(trace.evaluate(0.).is_err());
            assert!(swap_trace(trace).evaluate(0.).is_err());
            let line = SurfaceTrace::Line {
                surface: surface.clone(),
                plane: plane(2.),
                start: [domain[0], domain[2]],
                end: [bad, domain[3]],
            };
            assert!(line.evaluate(0.).is_err());
        }
        // Reversed intervals remain valid oriented traces.
        let forward = SurfaceTrace::Ruled {
            surface: surface.clone(),
            plane: plane(2.),
            u_interval: [domain[0], domain[1]],
        };
        let reverse = SurfaceTrace::Ruled {
            surface,
            plane: plane(2.),
            u_interval: [domain[1], domain[0]],
        };
        assert_eq!(
            forward.evaluate(0.25).unwrap().uv,
            reverse.evaluate(0.75).unwrap().uv
        );
    }
    #[test]
    fn unequal_ruling_weights_preserve_rational_section_parameters() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut surface = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        for row in &mut surface.weights {
            row[1] *= 3.;
        }
        for source in [surface.clone(), transpose_surface(&surface)] {
            let report = surface_plane(&source, plane(2.), Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
            assert_eq!(report.components.len(), 1);
            let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
                panic!("Expected curve")
            };
            let loaded: SurfaceTrace =
                value_codec::from_str_strict(&value_codec::to_string(trace).unwrap()).unwrap();
            for i in 0..=20 {
                let point = loaded.evaluate(i as f64 / 20.).unwrap();
                let ruling_axis = if source.degree_v == 1 { 1 } else { 0 };
                // z=4*(3*t)/(1-t+3*t), hence z=2 at t=1/4.
                assert!((point.uv[ruling_axis] - 0.25).abs() < 1e-12);
                assert!((point.point[2] - 2.).abs() < 1e-12);
                assert!((point.point[0].powi(2) + point.point[1].powi(2) - 4.).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn knot_refinement_preserves_linear_u_section_support() {
        let body = crate::cylinder(2., 4.).unwrap();
        let original = &body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface;
        let mut source = transpose_surface(original);
        // Insert a half-domain knot along the equal-weight linear direction.
        let middle = source.control_points[0]
            .iter()
            .zip(&source.control_points[1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) * 0.5).collect())
            .collect();
        source.control_points.insert(1, middle);
        source.weights.insert(1, source.weights[0].clone());
        source.knots_u.insert(2, 0.5);
        source.validate().unwrap();
        for height in [1., 2., 3.] {
            let report = surface_plane(&source, plane(height), Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            assert_eq!(report.components.len(), 1);
            let SurfacePlaneComponent::Curve {
                trace,
                parameter_box,
                ..
            } = &report.components[0]
            else {
                panic!("Expected curve")
            };
            assert_eq!(
                [parameter_box[0], parameter_box[1]],
                if height < 2. {
                    [0., 0.5]
                } else if height == 2. {
                    [0.5, 0.5]
                } else {
                    [0.5, 1.]
                }
            );
            for i in 0..=20 {
                let point = trace.evaluate(i as f64 / 20.).unwrap();
                assert!((point.uv[0] - height / 4.).abs() < 1e-12);
                assert!((point.point[2] - height).abs() < 1e-12);
                assert!((point.point[0].powi(2) + point.point[1].powi(2) - 4.).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn disconnected_ruling_knot_is_rejected_before_sectioning() {
        let body = crate::cylinder(2., 4.).unwrap();
        let original = &body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface;
        let mut source = transpose_surface(original);
        let mut first_top = source.control_points[0].clone();
        for p in &mut first_top {
            p[2] = 2.;
        }
        let mut second_bottom = first_top.clone();
        let mut second_top = source.control_points[1].clone();
        for row in [&mut second_bottom, &mut second_top] {
            for p in row {
                p[0] *= 1.5;
                p[1] *= 1.5;
            }
        }
        source.control_points = vec![
            source.control_points[0].clone(),
            first_top,
            second_bottom,
            second_top,
        ];
        source.weights = vec![source.weights[0].clone(); 4];
        source.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        assert_eq!(
            surface_plane(&source, plane(2.), Options::default())
                .unwrap_err()
                .code,
            "NURBS_INVALID_INPUT"
        );
    }
    #[test]
    fn proportional_ruling_weights_retain_generator_sections() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut source = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        for row in &mut source.weights {
            row[1] *= 3.;
        }
        let plane = Plane {
            normal: [1., 0., 0.],
            offset: 1.,
        };
        for surface in [source.clone(), transpose_surface(&source)] {
            let report = surface_plane(&surface, plane, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            assert_eq!(report.components.len(), 1);
            let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
                panic!("Expected generator")
            };
            for i in 0..=20 {
                let t = i as f64 / 20.;
                let p = trace.evaluate(t).unwrap();
                assert!((p.point[0] - 1.).abs() < 1e-9);
                assert!((p.point[1] - 3_f64.sqrt()).abs() < 1e-9);
                assert!((p.point[2] - 12. * t / (1. + 2. * t)).abs() < 1e-11);
            }
        }
    }
    #[test]
    fn nonproportional_boundary_weights_do_not_become_generator_lines() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut surface = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        for (row, scale) in surface.weights.iter_mut().zip([2., 3., 5.]) {
            row[1] *= scale;
        }
        let plane = Plane {
            normal: [1., 0., 0.],
            offset: 1.,
        };
        let report = surface_plane(
            &surface,
            plane,
            Options {
                max_boxes: 128,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(!report.components.is_empty(), "{report:?}");
        for component in &report.components {
            let SurfacePlaneComponent::Curve { trace, .. } = component else {
                panic!("Expected curve")
            };
            assert!(matches!(trace, SurfaceTrace::Ruled { .. }));
            for i in 0..=20 {
                let p = trace.evaluate(i as f64 / 20.).unwrap();
                assert!(p.plane_residual < 1e-9);
                assert!((p.point[0] - 1.).abs() < 1e-9);
                let [u, v] = p.uv;
                let basis = [(1. - u).powi(2), 2. * u * (1. - u), u * u];
                let base = [1., std::f64::consts::FRAC_1_SQRT_2, 1.];
                let upper: f64 = (0..3).map(|i| basis[i] * base[i] * [2., 3., 5.][i]).sum();
                let lower: f64 = (0..3).map(|i| basis[i] * base[i]).sum();
                let weight = (1. - v) * lower + v * upper;
                assert!((p.point[2] - 4. * v * upper / weight).abs() < 1e-11);
                let x = (0..2)
                    .map(|i| 2. * basis[i] * base[i] * ((1. - v) + v * [2., 3.][i]))
                    .sum::<f64>()
                    / weight;
                assert!((x - 1.).abs() < 1e-9);
            }
        }
        // Endpoint-root bands are retained explicitly under a finite budget.
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert!(!report.unresolved.is_empty());
    }
    #[test]
    fn surface_budget_visits_other_spans_before_refining_difficult_branches() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [-2., -2., -2., 5., -2.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| {
                    vec![
                        vec![i as f64, 0., z],
                        vec![i as f64, 1., if i == 3 { 5. } else { 2. }],
                    ]
                })
                .collect(),
            weights: vec![vec![1., 1.]; 5],
            periodic_u: false,
            periodic_v: false,
        };
        let report = surface_plane(
            &surface,
            plane(0.),
            Options {
                max_boxes: 8,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert!(report.boxes_visited <= 8);
        assert!(
            report.components.iter().any(|component| matches!(component,
            SurfacePlaneComponent::Curve {parameter_box,..} if *parameter_box==[0.,0.5,0.,1.])),
            "The simple first span was starved: {report:?}"
        );
    }
    #[test]
    fn curve_budget_admits_later_span_endpoint_before_deep_refinement() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            control_points: [1., -2., 1., 1., 0.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| vec![i as f64, 0., z])
                .collect(),
            weights: vec![1.; 5],
            periodic: false,
        };
        let report = curve_plane(
            &curve,
            plane(0.),
            Options {
                max_boxes: 4,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert!(report.boxes_visited <= 4);
        assert!(
            points(&report).iter().any(|p| p.parameter == 1.),
            "Later endpoint was starved"
        );
    }
    #[test]
    fn finite_curve_segment_queries_keep_parameters_and_refusals() {
        let curve = bezier(&[-1., 1.]);
        let report = curve_segment(&curve, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(!report.permits_topology_change());
        assert_eq!(report.components.len(), 1);
        let CurveSegmentComponent::Point {
            curve: p,
            segment_parameter,
            ..
        } = &report.components[0]
        else {
            panic!("Expected point")
        };
        assert!((p.parameter - 0.5).abs() < 1e-10);
        assert!((segment_parameter - 0.5).abs() < 1e-10);
        let skew = curve_segment(&curve, [0., 1., 0.], [1., 1., 0.], Options::default()).unwrap();
        assert!(skew.components.is_empty());
        assert_eq!(skew.coverage, Coverage::NumericallyResolved);
        let outside =
            curve_segment(&curve, [2., 0., 0.], [3., 0., 0.], Options::default()).unwrap();
        assert!(outside.components.is_empty());
        let line = bezier(&[0., 0.]);
        assert!(matches!(
            curve_segment(&line, [0., 0., 0.], [1., 0., 0.], Options::default())
                .unwrap()
                .components[0],
            CurveSegmentComponent::Overlap {
                curve_interval: [0., 1.]
            }
        ));
        let partial =
            curve_segment(&line, [0.25, 0., 0.], [0.75, 0., 0.], Options::default()).unwrap();
        assert_eq!(partial.coverage, Coverage::NumericallyResolved);
        assert!(matches!(
            partial.components[0],
            CurveSegmentComponent::Overlap {
                curve_interval: [0.25, 0.75]
            }
        ));
        let limited = curve_segment(
            &line,
            [0., 0., 0.],
            [1., 0., 0.],
            Options {
                max_boxes: 1,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(limited.boxes_visited, 1);
        assert_eq!(limited.coverage, Coverage::Incomplete);
        assert!(curve_segment(&line, [0.; 3], [0.; 3], Options::default()).is_err());
    }
    #[test]
    fn rational_arc_segment_query_preserves_reversed_segment_correspondence() {
        let body = crate::cylinder(2., 4.).unwrap();
        let arc = &body
            .edges
            .iter()
            .find(|e| e.curve.degree == 2)
            .unwrap()
            .curve;
        for (start, end, expected) in [
            ([0., 1., 0.], [3., 1., 0.], 3_f64.sqrt() / 3.),
            ([3., 1., 0.], [0., 1., 0.], 1. - 3_f64.sqrt() / 3.),
        ] {
            let report = curve_segment(arc, start, end, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            assert_eq!(report.components.len(), 1);
            let CurveSegmentComponent::Point {
                curve,
                segment_parameter,
                line_residual,
            } = &report.components[0]
            else {
                panic!("Expected arc hit")
            };
            assert!((curve.point[0] - 3_f64.sqrt()).abs() < 1e-9);
            assert!((curve.point[1] - 1.).abs() < 1e-9);
            assert!((segment_parameter - expected).abs() < 1e-9);
            assert!(*line_residual < 1e-9);
            assert!(report.boxes_visited <= Options::default().max_boxes);
        }
    }
    #[test]
    fn rational_linear_overlap_clipping_inverts_weights_and_keeps_endpoint_contacts() {
        let mut line = bezier(&[0., 0.]);
        line.weights = vec![1., 3.];
        for (start, end) in [
            ([0.25, 0., 0.], [0.75, 0., 0.]),
            ([0.75, 0., 0.], [0.25, 0., 0.]),
        ] {
            let report = curve_segment(&line, start, end, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved);
            let CurveSegmentComponent::Overlap { curve_interval } = report.components[0] else {
                panic!("Expected overlap")
            };
            assert!((curve_interval[0] - 0.1).abs() < 1e-12);
            assert!((curve_interval[1] - 0.5).abs() < 1e-12);
        }
        let report = curve_segment(&line, [1., 0., 0.], [2., 0., 0.], Options::default()).unwrap();
        assert!(
            matches!(&report.components[0],CurveSegmentComponent::Point {curve,segment_parameter,..}
            if curve.parameter==1. && *segment_parameter==0.)
        );
        let quadratic = bezier(&[0., 0., 0.]);
        let unresolved = curve_segment(
            &quadratic,
            [0.25, 0., 0.],
            [0.75, 0., 0.],
            Options::default(),
        )
        .unwrap();
        assert_eq!(unresolved.coverage, Coverage::NumericallyResolved);
    }
    #[test]
    fn shared_knot_segment_contact_has_one_parameter_event() {
        let curve = Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![-1., 0., 0.], vec![0., 0., 0.], vec![-1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let report = curve_segment(&curve, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(
            report.components.len(),
            1,
            "Duplicate shared-knot contact: {report:?}"
        );
        assert!(
            matches!(&report.components[0],CurveSegmentComponent::Point {curve,segment_parameter,..}
            if curve.parameter==0.5 && *segment_parameter==0.)
        );
        let repeated = Curve {
            degree: 1,
            knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
            control_points: vec![
                vec![-1., 0., 0.],
                vec![0., 0., 0.],
                vec![-1., 0., 0.],
                vec![0., 0., 0.],
                vec![-1., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let report =
            curve_segment(&repeated, [0., 0., 0.], [1., 0., 0.], Options::default()).unwrap();
        let parameters = report
            .components
            .iter()
            .map(|c| match c {
                CurveSegmentComponent::Point { curve, .. } => curve.parameter,
                _ => panic!("Expected point"),
            })
            .collect::<Vec<_>>();
        assert_eq!(parameters, vec![0.25, 0.75]);
    }
    #[test]
    fn quadratic_coincident_clipping_uses_original_nonlinear_parameters() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0., 0., 0.], vec![1., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let report =
            curve_segment(&curve, [0.25, 0., 0.], [0.5625, 0., 0.], Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        assert!(matches!(
            report.components[0],
            CurveSegmentComponent::Overlap {
                curve_interval: [0.5, 0.75]
            }
        ));
        let uncertain =
            curve_segment(&curve, [0.3, 0., 0.], [0.6, 0., 0.], Options::default()).unwrap();
        assert_eq!(uncertain.coverage, Coverage::Incomplete);
        assert!(!uncertain.unresolved.is_empty());
        assert!(uncertain.boxes_visited <= Options::default().max_boxes);
    }
    #[test]
    fn coincident_backtracking_curve_keeps_disconnected_parameter_intervals() {
        // x(t)=4t(1-t): the same spatial segment is visited twice.
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![2., 0., 0.], vec![0., 0., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let report =
            curve_segment(&curve, [0.4375, 0., 0.], [0.75, 0., 0.], Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let intervals = report
            .components
            .iter()
            .map(|c| match c {
                CurveSegmentComponent::Overlap { curve_interval } => *curve_interval,
                _ => panic!("Unexpected isolated point"),
            })
            .collect::<Vec<_>>();
        assert_eq!(intervals, vec![[0.125, 0.25], [0.75, 0.875]]);
        let uncertain =
            curve_segment(&curve, [0.3, 0., 0.], [0.7, 0., 0.], Options::default()).unwrap();
        assert_eq!(uncertain.coverage, Coverage::Incomplete);
        for q in [0.3_f64, 0.7] {
            for root in [(1. - (1. - q).sqrt()) / 2., (1. + (1. - q).sqrt()) / 2.] {
                assert!(
                    uncertain
                        .unresolved
                        .iter()
                        .any(|r| root >= r.parameter_box[0] && root <= r.parameter_box[1]),
                    "Missing boundary-root band for {root}: {uncertain:?}"
                );
            }
        }
        for c in uncertain.components {
            if let CurveSegmentComponent::Overlap {
                curve_interval: [a, b],
            } = c
            {
                for i in 0..=20 {
                    let t = a + (b - a) * i as f64 / 20.;
                    let x = 4. * t * (1. - t);
                    assert!((0.3..=0.7).contains(&x));
                }
            }
        }
    }
    #[test]
    fn coplanar_curve_is_clipped_to_affine_surface_domain() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![2., 2., 4., 4.],
            knots_v: vec![-1., -1., 3., 3.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.75, 0., 0.], vec![0.75, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let report = curve_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        assert!(matches!(
            report.components[0],
            CurveSurfaceComponent::Overlap {
                curve_interval: [0.25, 0.75]
            }
        ));
        let mut edge = curve.clone();
        for p in &mut edge.control_points {
            p[1] = 0.;
        }
        let report = curve_surface(&edge, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert!(matches!(
            report.components[0],
            CurveSurfaceComponent::Overlap {
                curve_interval: [0.25, 0.75]
            }
        ));
        let limited = curve_surface(
            &curve,
            &surface,
            Options {
                max_boxes: 2,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(limited.coverage, Coverage::Incomplete);
        assert!(limited.boxes_visited <= 2);
    }
    #[test]
    fn affine_clipping_preserves_shear_and_single_corner_contacts() {
        let mut curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![1., 1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let mut surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0.25, 0., 0.], vec![0.25, 1., 0.]],
                vec![vec![0.75, 0., 0.], vec![0.75, 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        for point in &mut curve.control_points {
            point[0] += point[1];
        }
        for point in surface.control_points.iter_mut().flatten() {
            point[0] += point[1];
        }
        let report = curve_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert!(matches!(
            report.components[0],
            CurveSurfaceComponent::Overlap {
                curve_interval: [0.25, 0.75]
            }
        ));
        let surface = Surface {
            control_points: vec![
                vec![vec![0., 0., 0.], vec![1., 1., 0.]],
                vec![vec![1., 0., 0.], vec![2., 1., 0.]],
            ],
            ..surface
        };
        let diagonal = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![0., 1., 0.], vec![0., -1., 0.]],
            weights: vec![1.; 2],
            periodic: false,
        };
        let report = curve_surface(&diagonal, &surface, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let CurveSurfaceComponent::Point { curve, uv, .. } = &report.components[0] else {
            panic!("Expected corner")
        };
        assert_eq!(curve.parameter, 0.5);
        assert_eq!(*uv, [0., 0.]);
    }
    #[test]
    fn finite_affine_surface_pair_retains_both_parameterizations() {
        let a = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
                vec![vec![1., -1., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let b = Surface {
            control_points: vec![
                vec![vec![-0.5, 0., -1.], vec![-0.5, 0., 1.]],
                vec![vec![0.5, 0., -1.], vec![0.5, 0., 1.]],
            ],
            ..a.clone()
        };
        for (first, second) in [(&a, &b), (&b, &a)] {
            let report = surface_surface(first, second, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            assert_eq!(report.components.len(), 1);
            let SurfaceSurfaceComponent::Curve { first, second, .. } = &report.components[0] else {
                panic!("Expected intersection segment")
            };
            for i in 0..=20 {
                let p = first.evaluate(i as f64 / 20.).unwrap();
                let q = second.evaluate(i as f64 / 20.).unwrap();
                assert!(
                    p.point
                        .iter()
                        .zip(q.point)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
                assert_eq!(p.point[1], 0.);
                assert_eq!(p.point[2], 0.);
                assert!(p.point[0] >= -0.5 && p.point[0] <= 0.5);
            }
        }
        assert_eq!(
            surface_surface(&a, &a, Options::default())
                .unwrap()
                .coverage,
            Coverage::NumericallyResolved
        );
        let limited = surface_surface(
            &a,
            &b,
            Options {
                max_boxes: 1,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(limited.coverage, Coverage::Incomplete);
        assert_eq!(limited.boxes_visited, 1);
    }
    #[test]
    fn coplanar_affine_pairs_keep_area_edge_and_point_dimensions() {
        let square = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
                vec![vec![1., -1., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let diamond = Surface {
            control_points: vec![
                vec![vec![0., -1.5, 0.], vec![-1.5, 0., 0.]],
                vec![vec![1.5, 0., 0.], vec![0., 1.5, 0.]],
            ],
            ..square.clone()
        };
        let report = surface_surface(&square, &diamond, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let SurfaceSurfaceComponent::Overlap {
            points,
            first_boundary,
            second_boundary,
            ..
        } = &report.components[0]
        else {
            panic!("Expected polygon")
        };
        assert_eq!(points.len(), 8);
        assert_eq!(first_boundary.len(), 8);
        assert_eq!(second_boundary.len(), 8);
        let area = (0..points.len())
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % points.len()];
                a[0] * b[1] - a[1] * b[0]
            })
            .sum::<f64>()
            / 2.;
        assert!((area - 3.5).abs() < 1e-12);
        for (dx, dy, kind) in [(2., 0., 1), (2., 2., 0), (3., 0., -1)] {
            let mut target = square.clone();
            for p in target.control_points.iter_mut().flatten() {
                p[0] += dx;
                p[1] += dy;
            }
            let report = surface_surface(&square, &target, Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            match kind {
                1 => assert!(matches!(
                    report.components[0],
                    SurfaceSurfaceComponent::Curve { .. }
                )),
                0 => assert!(matches!(
                    report.components[0],
                    SurfaceSurfaceComponent::Point { .. }
                )),
                _ => assert!(report.components.is_empty()),
            }
        }
        assert_eq!(
            surface_surface(
                &square,
                &diamond,
                Options {
                    max_boxes: 3,
                    ..Options::default()
                }
            )
            .unwrap()
            .coverage,
            Coverage::Incomplete
        );
    }
    #[test]
    fn coplanar_pair_correspondence_survives_swap_uv_reversal_and_knot_scaling() {
        let square = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![-1., -1., 0.], vec![-1., 1., 0.]],
                vec![vec![1., -1., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let diamond = Surface {
            control_points: vec![
                vec![vec![0., -1.5, 0.], vec![-1.5, 0., 0.]],
                vec![vec![1.5, 0., 0.], vec![0., 1.5, 0.]],
            ],
            ..square.clone()
        };
        for swap in [false, true] {
            for reverse in [false, true] {
                for transpose in [false, true] {
                    let (mut a, mut b) = if swap {
                        (diamond.clone(), square.clone())
                    } else {
                        (square.clone(), diamond.clone())
                    };
                    if reverse {
                        a.control_points.reverse();
                        a.weights.reverse();
                    }
                    if transpose {
                        b = transpose_surface(&b);
                    }
                    a.knots_u = a.knots_u.iter().map(|t| -3. + 8. * t).collect();
                    b.knots_v = b.knots_v.iter().map(|t| 10. + 4. * t).collect();
                    for row in &mut b.weights {
                        for w in row {
                            *w *= 32.;
                        }
                    }
                    let report = surface_surface(&a, &b, Options::default()).unwrap();
                    assert_eq!(
                        report.coverage,
                        Coverage::NumericallyResolved,
                        "swap={swap} reverse={reverse} transpose={transpose}: {report:?}"
                    );
                    let SurfaceSurfaceComponent::Overlap {
                        first_boundary,
                        second_boundary,
                        points,
                        ..
                    } = &report.components[0]
                    else {
                        panic!("Expected area")
                    };
                    assert_eq!(points.len(), 8);
                    let area = (0..8)
                        .map(|i| {
                            let p = points[i];
                            let q = points[(i + 1) % 8];
                            p[0] * q[1] - p[1] * q[0]
                        })
                        .sum::<f64>()
                        .abs()
                        / 2.;
                    assert!((area - 3.5).abs() < 1e-11);
                    for ((u, v), point) in first_boundary.iter().zip(second_boundary).zip(points) {
                        let p = a.evaluate(u[0], u[1]).unwrap().point;
                        let q = b.evaluate(v[0], v[1]).unwrap().point;
                        for axis in 0..3 {
                            assert!((p[axis] - q[axis]).abs() < 1e-11);
                            assert!((p[axis] - point[axis]).abs() < 1e-11);
                        }
                        assert!(point[0].abs() <= 1. + 1e-12 && point[1].abs() <= 1. + 1e-12);
                        assert!(point[0].abs() + point[1].abs() <= 1.5 + 1e-12);
                    }
                }
            }
        }
    }
    #[test]
    fn ruled_sections_convert_algebraically_to_rational_curves() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut surface = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        for row in &mut surface.weights {
            row[1] *= 3.;
        }
        for surface in [surface.clone(), transpose_surface(&surface)] {
            for plane in [
                plane(2.),
                Plane {
                    normal: [0.2, -0.1, 1.],
                    offset: 2.,
                },
                Plane {
                    normal: [1., 0., 0.],
                    offset: 1.,
                },
            ] {
                let report = surface_plane(&surface, plane, Options::default()).unwrap();
                for component in report.components {
                    let SurfacePlaneComponent::Curve { trace, .. } = component else {
                        panic!("Expected trace")
                    };
                    let curve = trace.to_curve().unwrap();
                    let [a, b] = curve.domain();
                    for i in 0..=40 {
                        let t = i as f64 / 40.;
                        let expected = trace.evaluate(t).unwrap().point;
                        let actual = curve.evaluate(a + t * (b - a)).unwrap().point;
                        assert!(
                            expected
                                .iter()
                                .zip(actual)
                                .all(|(a, b)| (a - b).abs() < 1e-10)
                        );
                    }
                }
            }
        }
    }
    #[test]
    fn trace_curve_conversion_preserves_reversed_fraction_on_shifted_domains() {
        let body = crate::cylinder(2., 4.).unwrap();
        let mut source = body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface
            .clone();
        source.knots_u.iter_mut().for_each(|t| *t = -3. + 8. * *t);
        source.knots_v.iter_mut().for_each(|t| *t = 10. + 4. * *t);
        for surface in [source.clone(), transpose_surface(&source)] {
            for plane in [
                plane(2.),
                Plane {
                    normal: [1., 0., 0.],
                    offset: 1.,
                },
            ] {
                for component in surface_plane(&surface, plane, Options::default())
                    .unwrap()
                    .components
                {
                    let SurfacePlaneComponent::Curve { trace, .. } = component else {
                        panic!("Expected trace")
                    };
                    let mut reverse = trace.clone();
                    match &mut reverse {
                        SurfaceTrace::Line { start, end, .. } => std::mem::swap(start, end),
                        SurfaceTrace::Ruled { u_interval, .. } => u_interval.swap(0, 1),
                        SurfaceTrace::RuledU { v_interval, .. } => v_interval.swap(0, 1),
                    }
                    let curve = reverse.to_curve().unwrap();
                    let [a, b] = curve.domain();
                    for i in 0..=20 {
                        let t = i as f64 / 20.;
                        let p = trace.evaluate(1. - t).unwrap().point;
                        let q = curve.evaluate(a + t * (b - a)).unwrap().point;
                        assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                    }
                }
            }
        }
    }

    #[test]
    fn multispan_ruled_conversion_preserves_seams_and_parameterization() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 4.]],
                vec![vec![1., 2., 0.], vec![1., 2., 4.]],
                vec![vec![2., 0., 0.], vec![2., 0., 4.]],
            ],
            weights: vec![vec![1., 2.]; 3],
            periodic_u: false,
            periodic_v: false,
        }
        .edit_axis(nurbs_core::surface::Axis::U, |c| {
            c.insert(0.25, 1)?.insert(0.75, 1)
        })
        .unwrap();
        let dense = surface
            .trim([0., 0.25, 0., 1.])
            .unwrap()
            .edit_axis(nurbs_core::surface::Axis::U, |c| {
                let mut curve = c.elevate(12)?;
                for i in 1..20 {
                    curve = curve.insert(i as f64 / 80., 1)?;
                }
                Ok(curve)
            })
            .unwrap();
        let oversized = SurfaceTrace::Ruled {
            surface: dense,
            plane: plane(2.),
            u_interval: [0., 0.25],
        };
        assert!(
            oversized
                .to_curve()
                .unwrap_err()
                .to_string()
                .contains("256 control points")
        );
        for reverse in [false, true] {
            let trace = SurfaceTrace::Ruled {
                surface: surface.clone(),
                plane: plane(2.),
                u_interval: if reverse { [1., 0.] } else { [0., 1.] },
            };
            let curve = trace.to_curve().unwrap();
            assert_eq!(curve.degree, 4);
            assert_eq!(curve.control_points.len(), 13);
            for i in 0..=100 {
                let t = i as f64 / 100.;
                let expected = trace.evaluate(t).unwrap().point;
                let actual = curve.evaluate(t).unwrap().point;
                assert!(
                    expected
                        .iter()
                        .zip(actual)
                        .all(|(a, b)| (a - b).abs() < 1e-10)
                );
            }
        }
    }
    #[test]
    fn multispan_conversion_handles_varying_boundary_weights_and_oblique_cuts() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..5)
                .map(|i| {
                    let x = i as f64 / 2.;
                    vec![vec![x, 0., 0.], vec![x, 0., 4.]]
                })
                .collect(),
            weights: vec![
                vec![1., 2.],
                vec![2., 3.],
                vec![3., 4.],
                vec![2., 5.],
                vec![1., 3.],
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let mut shifted = surface.clone();
        shifted.knots_u = shifted
            .knots_u
            .iter()
            .map(|&u| if u == 0.5 { -1. } else { -3. + 8. * u })
            .collect();
        shifted.knots_v = shifted.knots_v.iter().map(|&v| 10. + 4. * v).collect();
        for source in [shifted.clone(), transpose_surface(&shifted)] {
            for reverse in [false, true] {
                let interval = if reverse { [4., -2.] } else { [-2., 4.] };
                let plane = Plane {
                    normal: [0.25, 0., 1.],
                    offset: 2.,
                };
                let trace = if source.degree_v == 1 {
                    SurfaceTrace::Ruled {
                        surface: source.clone(),
                        plane,
                        u_interval: interval,
                    }
                } else {
                    SurfaceTrace::RuledU {
                        surface: source.clone(),
                        plane,
                        v_interval: interval,
                    }
                };
                let pieces = trace.to_curve_segments().unwrap();
                let middle = if reverse { 5. / 6. } else { 1. / 6. };
                assert_eq!(pieces.len(), 2);
                assert_eq!(pieces[0].domain(), [0., middle]);
                assert_eq!(pieces[1].domain(), [middle, 1.]);
                for curve in pieces {
                    let [lo, hi] = curve.domain();
                    for i in 0..=100 {
                        let t = if i == 100 {
                            hi
                        } else {
                            lo + (hi - lo) * (i as f64 / 100.)
                        };
                        let p = curve.evaluate(t).unwrap().point;
                        let q = trace.evaluate(t).unwrap().point;
                        assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                        assert!((0.25 * p[0] + p[2] - 2.).abs() < 1e-10);
                    }
                }
            }
        }
        for surface in [surface.clone(), transpose_surface(&surface)] {
            for reverse in [false, true] {
                let interval = if reverse { [1., 0.] } else { [0., 1.] };
                let plane = Plane {
                    normal: [0.25, 0., 1.],
                    offset: 2.,
                };
                let trace = if surface.degree_v == 1 {
                    SurfaceTrace::Ruled {
                        surface: surface.clone(),
                        plane,
                        u_interval: interval,
                    }
                } else {
                    SurfaceTrace::RuledU {
                        surface: surface.clone(),
                        plane,
                        v_interval: interval,
                    }
                };
                let curve = trace.to_curve().unwrap();
                for i in 0..=100 {
                    let t = i as f64 / 100.;
                    let p = curve.evaluate(t).unwrap().point;
                    let q = trace.evaluate(t).unwrap().point;
                    assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                    assert!((0.25 * p[0] + p[2] - 2.).abs() < 1e-10);
                }
            }
        }
    }
    #[test]
    fn tensor_uv_diagonal_conversion_preserves_spherical_geometry() {
        let mut surface = crate::sphere(2.).unwrap().faces[0].surface.clone();
        for knot in &mut surface.knots_u {
            *knot = -3. + 8. * *knot;
        }
        for knot in &mut surface.knots_v {
            *knot = 10. + 4. * *knot;
        }
        for flip_u in [false, true] {
            for flip_v in [false, true] {
                let u = if flip_u { [4.2, -2.2] } else { [-2.2, 4.2] };
                let v = if flip_v { [13.2, 10.8] } else { [10.8, 13.2] };
                let trace = SurfaceTrace::Line {
                    surface: surface.clone(),
                    plane: plane(0.),
                    start: [u[0], v[0]],
                    end: [u[1], v[1]],
                };
                let curve = trace.to_curve().unwrap();
                assert_eq!(curve.degree, surface.degree_u + surface.degree_v);
                for i in 0..=100 {
                    let t = i as f64 / 100.;
                    let p = curve.evaluate(t).unwrap().point;
                    let q = trace.evaluate(t).unwrap().point;
                    assert!(p.iter().zip(q).all(|(a, b)| (a - b).abs() < 1e-10));
                    assert!((p.iter().map(|x| x * x).sum::<f64>() - 4.).abs() < 1e-10);
                }
            }
        }
        let high_degree = surface
            .edit_axis(nurbs_core::surface::Axis::U, |c| c.elevate(13))
            .unwrap()
            .edit_axis(nurbs_core::surface::Axis::V, |c| c.elevate(13))
            .unwrap();
        let oversized = SurfaceTrace::Line {
            surface: high_degree,
            plane: plane(0.),
            start: [-2.2, 10.8],
            end: [4.2, 13.2],
        };
        assert!(
            oversized
                .to_curve()
                .unwrap_err()
                .to_string()
                .contains("degree at most 25")
        );
        let point_trace = SurfaceTrace::Line {
            surface: surface.clone(),
            plane: plane(0.),
            start: [1., 12.],
            end: [1., 12.],
        };
        let point_curve = point_trace.to_curve().unwrap();
        assert_eq!(
            point_curve.evaluate(0.3).unwrap().point,
            point_trace.evaluate(0.3).unwrap().point
        );
        let refined = surface
            .edit_axis(nurbs_core::surface::Axis::U, |c| c.insert(1., 1))
            .unwrap();
        let unsupported = SurfaceTrace::Line {
            surface: refined,
            plane: plane(0.),
            start: [-2.2, 10.8],
            end: [4.2, 13.2],
        };
        assert!(
            unsupported
                .to_curve()
                .unwrap_err()
                .to_string()
                .contains("endpoints do not coincide")
        );
    }
    #[test]
    fn multispan_uv_diagonal_preserves_fraction_and_refined_surface() {
        let original = crate::sphere(2.).unwrap().faces[0].surface.clone();
        let refined = original
            .edit_axis(nurbs_core::surface::Axis::U, |c| {
                c.insert(0.25, 1)?.insert(0.875, 1)
            })
            .unwrap()
            .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
            .unwrap();
        for reverse in [false, true] {
            let (start, end) = if reverse {
                ([1., 1.], [0., 0.])
            } else {
                ([0., 0.], [1., 1.])
            };
            let trace = SurfaceTrace::Line {
                surface: refined.clone(),
                plane: plane(0.),
                start,
                end,
            };
            let pieces = trace.to_curve_segments().unwrap();
            assert_eq!(pieces.len(), 4);
            let boundaries = if reverse {
                [0., 0.125, 0.5, 0.75, 1.]
            } else {
                [0., 0.25, 0.5, 0.875, 1.]
            };
            for (index, curve) in pieces.iter().enumerate() {
                assert_eq!(curve.domain(), [boundaries[index], boundaries[index + 1]]);
                assert_eq!(curve.degree, 4);
                for i in 0..=100 {
                    let t = boundaries[index]
                        + (boundaries[index + 1] - boundaries[index]) * i as f64 / 100.;
                    let actual = curve.evaluate(t).unwrap().point;
                    let expected = trace.evaluate(t).unwrap().point;
                    assert!(
                        actual
                            .iter()
                            .zip(expected)
                            .all(|(a, b)| (a - b).abs() < 1e-10)
                    );
                    assert!((actual.iter().map(|v| v * v).sum::<f64>() - 4.).abs() < 1e-10);
                }
            }
        }
        let dense = original
            .edit_axis(nurbs_core::surface::Axis::U, |c| {
                let mut result = c.clone();
                for i in 1..28 {
                    result = result.insert(i as f64 / 64., 1)?;
                }
                Ok(result)
            })
            .unwrap()
            .edit_axis(nurbs_core::surface::Axis::V, |c| {
                let mut result = c.clone();
                for i in 33..60 {
                    result = result.insert(i as f64 / 64., 1)?;
                }
                Ok(result)
            })
            .unwrap();
        assert!(
            SurfaceTrace::Line {
                surface: dense,
                plane: plane(0.),
                start: [0., 0.],
                end: [1., 1.]
            }
            .to_curve_segments()
            .unwrap_err()
            .to_string()
            .contains("256 control points")
        );
    }
    #[test]
    fn multispan_uv_diagonal_joins_identical_polynomial_endpoints() {
        let mut source = flat();
        source.control_points[1][1][2] = 1.;
        let refined = source
            .edit_axis(nurbs_core::surface::Axis::U, |c| {
                c.insert(0.25, 1)?.insert(0.75, 1)
            })
            .unwrap()
            .edit_axis(nurbs_core::surface::Axis::V, |c| c.insert(0.5, 1))
            .unwrap();
        let trace = SurfaceTrace::Line {
            surface: refined,
            plane: plane(0.),
            start: [0., 0.],
            end: [1., 1.],
        };
        let curve = trace.to_curve().unwrap();
        assert_eq!(curve.control_points.len(), 9);
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let point = curve.evaluate(t).unwrap().point;
            for (a, b) in point.iter().zip([t, t, t * t]) {
                assert!((a - b).abs() < 1e-12);
            }
        }
    }
    #[test]
    fn ruled_conversion_rejects_an_interior_excursion_from_the_source_domain() {
        let source = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [-1., 3., -1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| vec![vec![i as f64 / 2., 0., z], vec![i as f64 / 2., 1., z + 2.]])
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        for surface in [source.clone(), transpose_surface(&source)] {
            for reverse in [false, true] {
                let interval = if reverse { [1., 0.] } else { [0., 1.] };
                let trace = if surface.degree_v == 1 {
                    SurfaceTrace::Ruled {
                        surface: surface.clone(),
                        plane: plane(0.),
                        u_interval: interval,
                    }
                } else {
                    SurfaceTrace::RuledU {
                        surface: surface.clone(),
                        plane: plane(0.),
                        v_interval: interval,
                    }
                };
                assert!(trace.evaluate(0.).is_ok());
                assert!(trace.evaluate(1.).is_ok());
                assert!(trace.evaluate(0.5).is_err());
                assert!(
                    trace
                        .to_curve()
                        .unwrap_err()
                        .to_string()
                        .contains("leaves the source parameter domain")
                );
                assert!(
                    trace
                        .to_curve_segments()
                        .unwrap_err()
                        .to_string()
                        .contains("leaves the source parameter domain")
                );
            }
        }
    }
    #[test]
    fn trace_conversion_refuses_an_ambiguous_ruling_family() {
        let source = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [-1., 2., -1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., -z]])
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let trace = SurfaceTrace::Ruled {
            surface: source,
            plane: plane(0.),
            u_interval: [0., 1.],
        };
        assert!(trace.evaluate(0.).is_ok());
        assert!(trace.evaluate(1.).is_ok());
        assert!(trace.to_curve().is_err());
        assert!(trace.to_curve_segments().is_err());
        let late_ambiguous = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: [-1., -1., -1., 2., -1.]
                .into_iter()
                .enumerate()
                .map(|(i, z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., -z]])
                .collect(),
            weights: vec![vec![1.; 2]; 5],
            periodic_u: false,
            periodic_v: false,
        };
        let first = SurfaceTrace::Ruled {
            surface: late_ambiguous.clone(),
            plane: plane(0.),
            u_interval: [0., 0.5],
        };
        assert_eq!(first.to_curve_segments().unwrap().len(), 1);
        let whole = SurfaceTrace::Ruled {
            surface: late_ambiguous,
            plane: plane(0.),
            u_interval: [0., 1.],
        };
        assert!(whole.to_curve_segments().is_err()); // no partial success after the valid first span
        let mut dense = flat();
        for row in &mut dense.control_points {
            row[1][2] = 4.;
        }
        let dense = dense
            .edit_axis(nurbs_core::surface::Axis::U, |c| {
                let mut curve = c.elevate(12)?;
                for i in 1..13 {
                    curve = curve.insert(i as f64 / 16., 1)?;
                }
                Ok(curve)
            })
            .unwrap();
        let oversized = SurfaceTrace::Ruled {
            surface: dense,
            plane: plane(2.),
            u_interval: [0., 1.],
        };
        assert!(
            oversized
                .to_curve_segments()
                .unwrap_err()
                .to_string()
                .contains("256 control points")
        );
    }
    #[test]
    fn frustum_sections_and_incomplete_boundary_bands_are_distinguished() {
        let body = crate::frustum(2., 1., 4.).unwrap();
        let surface = &body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface;
        let report = surface_plane(surface, plane(2.), Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        let SurfacePlaneComponent::Curve { trace, .. } = &report.components[0] else {
            panic!()
        };
        for i in 0..51 {
            let p = trace.evaluate(i as f64 / 50.).unwrap();
            assert!((p.point[0] * p.point[0] + p.point[1] * p.point[1] - 2.25).abs() < 1e-10);
        }
        let crossing = surface_plane(
            surface,
            Plane {
                normal: [1., -1., 0.],
                offset: 0.,
            },
            Options {
                max_boxes: 128,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(crossing.coverage, Coverage::Incomplete);
        assert!(!crossing.unresolved.is_empty());
    }
    fn flat() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn affine_patch_sections_and_curve_surface_clipping() {
        let surface = flat();
        let report = surface_plane(
            &surface,
            Plane {
                normal: [1., 1., 0.],
                offset: 1.,
            },
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved);
        assert_eq!(report.components.len(), 1);
        let curve = Curve::from_polyline(vec![vec![0.5, 0.5, -1.], vec![0.5, 0.5, 1.]]).unwrap();
        let report = curve_surface(&curve, &surface, Options::default()).unwrap();
        assert_eq!(report.components.len(), 1);
        let CurveSurfaceComponent::Point {
            uv,
            surface_residual,
            ..
        } = report.components[0]
        else {
            panic!()
        };
        assert_eq!(uv, [0.5, 0.5]);
        assert!(surface_residual < 1e-12);
        let outside = Curve::from_polyline(vec![vec![2., 0.5, -1.], vec![2., 0.5, 1.]]).unwrap();
        assert!(
            curve_surface(&outside, &surface, Options::default())
                .unwrap()
                .components
                .is_empty()
        );
        let crossing = Curve::from_polyline(vec![vec![0.5, 0.5, 0.], vec![2., 0.5, 0.]]).unwrap();
        assert_eq!(
            curve_surface(&crossing, &surface, Options::default())
                .unwrap()
                .coverage,
            Coverage::Incomplete
        );
    }

    #[test]
    fn vertical_cylinder_cut_is_a_retained_generator_with_root_interval() {
        let body = crate::cylinder(2., 4.).unwrap();
        let surface = &body
            .faces
            .iter()
            .find(|f| f.surface.degree_u == 2)
            .unwrap()
            .surface;
        let plane = Plane {
            normal: [1., 0., 0.],
            offset: 1.,
        };
        let report = surface_plane(surface, plane, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
        assert_eq!(report.components.len(), 1);
        let SurfacePlaneComponent::Curve {
            trace,
            parameter_box,
            ..
        } = &report.components[0]
        else {
            panic!()
        };
        assert!(parameter_box[1] - parameter_box[0] <= Options::default().parameter_tolerance);
        for i in 0..101 {
            let sample = trace.evaluate(i as f64 / 100.).unwrap();
            assert!(sample.plane_residual < 1e-9);
            assert!((sample.point[2] - 4. * i as f64 / 100.).abs() < 1e-12);
        }
    }

    #[test]
    fn nearby_roots_are_not_merged_and_curve_edits_preserve_intersections() {
        let separation = 1e-5_f64;
        let c = 0.25 - separation * separation;
        let curve = bezier(&[c, c - 0.5, c]);
        for source in [
            curve.clone(),
            curve.reverse().unwrap(),
            curve.insert(0.4, 1).unwrap(),
            curve.elevate(4).unwrap(),
        ] {
            let report = curve_plane(&source, plane(0.), Options::default()).unwrap();
            assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
            let roots = points(&report);
            assert_eq!(roots.len(), 2, "{report:?}");
            assert!((roots[0].parameter - (0.5 - separation)).abs() < 1e-9);
            assert!((roots[1].parameter - (0.5 + separation)).abs() < 1e-9);
        }
    }

    #[test]
    fn invalid_inputs_and_unsupported_surfaces_remain_explicit() {
        assert!(
            curve_plane(
                &bezier(&[-1., 1.]),
                Plane {
                    normal: [0.; 3],
                    offset: 0.
                },
                Options::default()
            )
            .is_err()
        );
        assert!(
            curve_plane(
                &bezier(&[-1., 1.]),
                plane(0.),
                Options {
                    max_boxes: 0,
                    ..Options::default()
                }
            )
            .is_err()
        );
        let mut curved = flat();
        curved.control_points[1][1][2] = 1.;
        let curve = bezier(&[-1., 1.]);
        let report = curve_surface(&curve, &curved, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        assert_eq!(
            report.unresolved[0].reason,
            UnresolvedReason::UnsupportedSurface
        );
        assert!(!report.permits_topology_change());
    }
}
