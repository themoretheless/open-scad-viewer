//! Finite analytic planar arrangements over retained rational NURBS spans.
//!
//! Lines and circular quadratic arcs are admitted. Intersections, winding and
//! areas use their analytic carriers; output boundaries are trims/reversals of
//! the input curves, never chords or fitted replacements. Ill-conditioned or
//! tolerance-ambiguous events fail explicitly. This is numerical binary64
//! analytic geometry, not an independent exact-arithmetic certificate.
use nurbs_core::{Error, Result, curve::Curve};
use std::f64::consts::{PI, TAU};

const MAX_SPANS: usize = 256;
const MAX_FRAGMENTS: usize = 2048;
type Point = [f64; 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocation {
    Outside,
    Inside,
    Boundary,
}

fn unsupported(message: &str) -> Error {
    Error::new("BREP_UNSUPPORTED_PLANAR_TRIM", message)
}
fn ambiguous(message: &str) -> Error {
    Error::new("BREP_AMBIGUOUS_PLANAR_TRIM", message)
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_INVALID_PLANAR_TRIM", message)
}
fn limit() -> Error {
    Error::new(
        "BREP_RESOURCE_LIMIT",
        "Planar trim exceeds 256 analytic spans or 2048 fragments",
    )
}
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: Point, s: f64) -> Point {
    [a[0] * s, a[1] * s]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn cross(a: Point, b: Point) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn norm(a: Point) -> f64 {
    a[0].hypot(a[1])
}
fn distance(a: Point, b: Point) -> f64 {
    norm(sub(a, b))
}
fn left(a: Point) -> Point {
    [-a[1], a[0]]
}
fn point(curve: &Curve, t: f64) -> Result<Point> {
    let p = curve.evaluate(t)?.point;
    Ok([p[0], p[1]])
}

#[derive(Clone, Debug)]
enum Carrier {
    Line,
    Circle {
        center: Point,
        radius: f64,
        start: f64,
        sweep: f64,
    },
}
#[derive(Clone, Debug)]
struct Span {
    curve: Curve,
    // This translated definition is only for numerical carrier/query work.
    // Output always trims/reverses `curve`, never translates this copy back.
    local_curve: Curve,
    p0: Point,
    p1: Point,
    carrier: Carrier,
    loop_id: usize,
}
impl Span {
    fn from_curve(source: Curve, loop_id: usize, epsilon: f64, origin: Point) -> Result<Self> {
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
    fn position(&self, fraction: f64) -> Result<Point> {
        let d = self.curve.domain();
        point(&self.local_curve, d[0] + fraction * (d[1] - d[0]))
    }
    fn tangent(&self, fraction: f64) -> Result<Point> {
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
    fn arc_fraction(&self, p: Point, epsilon: f64) -> Option<f64> {
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
    fn contains(&self, p: Point, epsilon: f64) -> bool {
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
    fn parameter(&self, p: Point, epsilon: f64) -> Result<f64> {
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
    fn area(&self, origin: Point) -> f64 {
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

struct Region {
    spans: Vec<Span>,
    loops: Vec<Vec<usize>>,
    epsilon: f64,
    tolerance: f64,
    origin: Point,
    /// Construction/representation error at a declared loop junction. This is
    /// never used to identify coincident carriers or classify material sides.
    endpoint_error: f64,
}
fn region_origin(loops: &[Vec<Curve>]) -> Point {
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
impl Region {
    fn parse(loops: &[Vec<Curve>], tolerance: f64) -> Result<Self> {
        Self::parse_at(loops, tolerance, region_origin(loops))
    }
    fn parse_at(loops: &[Vec<Curve>], tolerance: f64, origin: Point) -> Result<Self> {
        if !tolerance.is_finite() || !(1e-12..=1e-2).contains(&tolerance) {
            return Err(invalid("Planar trim tolerance must be 1e-12..1e-2"));
        }
        if loops.len() > MAX_SPANS {
            return Err(limit());
        }
        let mut scale = 1_f64;
        let mut endpoint_error = 0_f64;
        for curve in loops.iter().flatten() {
            curve.validate()?;
            if curve.control_points[0].len() != 2 {
                return Err(invalid("Planar trims must be 2D curves"));
            }
            if curve.degree > 2 {
                return Err(unsupported(
                    "Planar trims support lines and circular quadratic arcs",
                ));
            }
            // Unrelated world translation must never widen coincidence.
            let anchor = &curve.control_points[0];
            let mut extent = 0_f64;
            let mut coordinate_ulp = 0_f64;
            for p in &curve.control_points {
                extent = extent.max(norm([p[0] - anchor[0], p[1] - anchor[1]]));
                coordinate_ulp =
                    coordinate_ulp.max((p[0].next_up() - p[0]).hypot(p[1].next_up() - p[1]));
            }
            scale = scale.max(extent);
            let domain = curve.domain();
            let parameter_ulp =
                (domain[0].next_up() - domain[0]).max(domain[1].next_up() - domain[1]);
            let ratio = curve.weights.iter().copied().fold(0., f64::max)
                / curve.weights.iter().copied().fold(f64::INFINITY, f64::min);
            // Positive-weight rational derivative bound on the current span:
            // |C'| <= degree * extent * (2*r + r²) / domain_width.
            let minimum_span = curve
                .knots
                .windows(2)
                .filter(|w| w[0] >= domain[0] && w[1] <= domain[1] && w[0] < w[1])
                .map(|w| w[1] - w[0])
                .fold(f64::INFINITY, f64::min);
            let speed = curve.degree as f64 * extent * (2. * ratio + ratio * ratio) / minimum_span;
            endpoint_error = endpoint_error.max(4. * coordinate_ulp + 2. * parameter_ulp * speed);
        }
        let epsilon = scale * 256. * f64::EPSILON;
        if epsilon * 8. > tolerance {
            return Err(ambiguous(
                "Requested tolerance is below the coordinate roundoff envelope",
            ));
        }
        endpoint_error = endpoint_error.max(epsilon * 8.);
        if endpoint_error > tolerance {
            return Err(ambiguous(
                "Source parameter or coordinate precision cannot meet the requested trim tolerance",
            ));
        }
        let mut region = Self {
            spans: vec![],
            loops: vec![],
            epsilon,
            tolerance,
            origin,
            endpoint_error,
        };
        for (loop_id, curves) in loops.iter().enumerate() {
            if curves.is_empty() {
                return Err(invalid("A planar trim loop must not be empty"));
            }
            let mut ids = Vec::new();
            for curve in curves {
                let d = curve.domain();
                let mut knots: Vec<_> = curve
                    .knots
                    .iter()
                    .copied()
                    .filter(|v| *v >= d[0] && *v <= d[1])
                    .collect();
                knots.dedup();
                for interval in knots.windows(2) {
                    if region.spans.len() >= MAX_SPANS {
                        return Err(limit());
                    }
                    let piece = curve.trim(interval[0], interval[1])?;
                    ids.push(region.spans.len());
                    region
                        .spans
                        .push(Span::from_curve(piece, loop_id, epsilon, origin)?);
                }
            }
            for i in 0..ids.len() {
                if distance(
                    region.spans[ids[i]].p1,
                    region.spans[ids[(i + 1) % ids.len()]].p0,
                ) > endpoint_error
                {
                    return Err(ambiguous(&format!(
                        "Trim loop endpoints have unresolved gap {} (numerical bound {})",
                        distance(
                            region.spans[ids[i]].p1,
                            region.spans[ids[(i + 1) % ids.len()]].p0
                        ),
                        endpoint_error
                    )));
                }
            }
            region.loops.push(ids);
        }
        Ok(region)
    }
    fn location(&self, p: Point) -> Result<PointLocation> {
        if self.spans.iter().any(|s| s.contains(p, self.tolerance)) {
            return Ok(PointLocation::Boundary);
        }
        // Select a ray which does not pass through a trim vertex or circle
        // extremum. This avoids half-open endpoint guesses at analytic events.
        for attempt in 0..32 {
            let direction = if attempt == 0 {
                [1., 0.]
            } else {
                let y = (attempt as f64) * 0.371;
                mul([1., y], 1. / 1_f64.hypot(y))
            };
            let normal = left(direction);
            let py = dot(p, normal);
            if self.spans.iter().any(|s| {
                if (dot(s.p0, normal) - py).abs() <= self.epsilon * 8.
                    || (dot(s.p1, normal) - py).abs() <= self.epsilon * 8.
                {
                    return true;
                }
                if let Carrier::Circle { center, radius, .. } = s.carrier {
                    (dot(center, normal) + radius - py).abs() <= self.epsilon * 8.
                        || (dot(center, normal) - radius - py).abs() <= self.epsilon * 8.
                } else {
                    false
                }
            }) {
                continue;
            }
            let px = dot(p, direction);
            let mut winding = 0_i32;
            for span in &self.spans {
                match span.carrier {
                    Carrier::Line => {
                        let y0 = dot(span.p0, normal);
                        let y1 = dot(span.p1, normal);
                        if (y0 < py && y1 > py) || (y1 < py && y0 > py) {
                            let t = (py - y0) / (y1 - y0);
                            let hit = add(span.p0, mul(sub(span.p1, span.p0), t));
                            if dot(hit, direction) > px {
                                winding += if y1 > y0 { 1 } else { -1 };
                            }
                        }
                    }
                    Carrier::Circle {
                        center,
                        radius,
                        sweep,
                        ..
                    } => {
                        let h = py - dot(center, normal);
                        let square = radius * radius - h * h;
                        if square <= 0. {
                            continue;
                        }
                        let delta = square.sqrt();
                        for x in [-delta, delta] {
                            let hit = add(center, add(mul(normal, h), mul(direction, x)));
                            if dot(hit, direction) > px
                                && span.arc_fraction(hit, self.epsilon).is_some()
                            {
                                let derivative =
                                    dot(mul(left(sub(hit, center)), sweep.signum()), normal);
                                winding += if derivative > 0. { 1 } else { -1 };
                            }
                        }
                    }
                }
            }
            return Ok(if winding == 0 {
                PointLocation::Outside
            } else {
                PointLocation::Inside
            });
        }
        Err(ambiguous(
            "Could not construct an unambiguous bounded winding ray",
        ))
    }
    fn validate(&self) -> Result<()> {
        self.nesting_depths(true).map(|_| ())
    }
    fn nesting_depths(&self, material_left: bool) -> Result<Vec<usize>> {
        let areas: Vec<f64> = self
            .loops
            .iter()
            .map(|ids| {
                let origin = self.spans[ids[0]].p0;
                ids.iter().map(|i| self.spans[*i].area(origin)).sum()
            })
            .collect();
        if areas
            .iter()
            .any(|area| area.abs() <= self.tolerance * self.tolerance)
        {
            return Err(ambiguous("Trim loop has unresolved signed area"));
        }
        let mut cuts: Vec<Vec<f64>> = self
            .spans
            .iter()
            .map(|s| s.curve.domain().to_vec())
            .collect();
        for (i, a) in self.spans.iter().enumerate() {
            for (j, b) in self.spans.iter().enumerate().skip(i + 1) {
                let events = intersections(a, b, self.epsilon, self.tolerance)?;
                if events.len() > 1 && same_carrier(a, b, self.epsilon) {
                    return Err(invalid(
                        "Input trim boundaries overlap over positive length",
                    ));
                }
                for p in events {
                    let a_end = distance(p, a.p0) <= self.endpoint_error
                        || distance(p, a.p1) <= self.endpoint_error;
                    let b_end = distance(p, b.p0) <= self.endpoint_error
                        || distance(p, b.p1) <= self.endpoint_error;
                    if a.loop_id == b.loop_id {
                        let ids = &self.loops[a.loop_id];
                        let ia = ids.iter().position(|id| *id == i).unwrap();
                        let ib = ids.iter().position(|id| *id == j).unwrap();
                        let adjacent = (ia + 1) % ids.len() == ib || (ib + 1) % ids.len() == ia;
                        if !adjacent || !a_end || !b_end {
                            return Err(invalid("A trim loop self-intersects or overlaps"));
                        }
                    } else {
                        if material_left && (areas[a.loop_id] < 0. || areas[b.loop_id] < 0.) {
                            return Err(invalid(
                                "A hole boundary touches or intersects another trim boundary",
                            ));
                        }
                        cuts[i].push(a.parameter(p, self.epsilon * 8.)?);
                        cuts[j].push(b.parameter(p, self.epsilon * 8.)?);
                    }
                }
            }
        }
        for cut in &mut cuts {
            cut.sort_by(f64::total_cmp);
            cut.dedup_by(|a, b| *a == *b);
        }
        let mut work = 0usize;
        let mut depths = Vec::with_capacity(self.loops.len());
        for (loop_id, ids) in self.loops.iter().enumerate() {
            let mut depth = 0;
            for (other, other_ids) in self.loops.iter().enumerate() {
                if loop_id == other {
                    continue;
                }
                let target = Self {
                    spans: other_ids.iter().map(|i| self.spans[*i].clone()).collect(),
                    loops: vec![],
                    epsilon: self.epsilon,
                    tolerance: self.tolerance,
                    origin: self.origin,
                    endpoint_error: self.endpoint_error,
                };
                let mut relation = None;
                // Every open event fragment must agree on the relation to each
                // other loop. A single witness misses crossings deliberately
                // encoded as curve endpoints, especially for segmented lines.
                for id in ids {
                    for interval in cuts[*id].windows(2) {
                        work += 1;
                        if work > MAX_FRAGMENTS * MAX_SPANS {
                            return Err(limit());
                        }
                        let midpoint = interval[0] + (interval[1] - interval[0]) * 0.5;
                        if midpoint == interval[0] || midpoint == interval[1] {
                            if distance(
                                point(&self.spans[*id].local_curve, interval[0])?,
                                point(&self.spans[*id].local_curve, interval[1])?,
                            ) <= self.epsilon * 8.
                            {
                                continue;
                            }
                            return Err(ambiguous(
                                "Loop arrangement exceeds source parameter precision",
                            ));
                        }
                        let inside = match target
                            .location(point(&self.spans[*id].local_curve, midpoint)?)?
                        {
                            PointLocation::Inside => true,
                            PointLocation::Outside => false,
                            PointLocation::Boundary => {
                                return Err(ambiguous(
                                    "Loop relation lies in a coincident boundary tolerance band",
                                ));
                            }
                        };
                        if relation.is_some_and(|r| r != inside) {
                            return Err(invalid("Input loops cross or have inconsistent nesting"));
                        }
                        relation = Some(inside);
                    }
                }
                if relation.ok_or_else(|| ambiguous("Unresolved loop nesting"))? {
                    depth += 1;
                }
            }
            if material_left && (areas[loop_id] > 0.) != (depth % 2 == 0) {
                return Err(invalid(
                    "Trim orientation must keep material left: CCW outer loops, CW holes",
                ));
            }
            depths.push(depth);
        }
        Ok(depths)
    }
}

fn same_carrier(a: &Span, b: &Span, epsilon: f64) -> bool {
    match (&a.carrier, &b.carrier) {
        (Carrier::Line, Carrier::Line) => {
            cross(sub(a.p1, a.p0), sub(b.p1, b.p0)).abs()
                <= epsilon * distance(a.p0, a.p1).max(distance(b.p0, b.p1))
                && cross(sub(b.p0, a.p0), sub(a.p1, a.p0)).abs() <= epsilon * distance(a.p0, a.p1)
        }
        (
            Carrier::Circle {
                center: a,
                radius: ra,
                ..
            },
            Carrier::Circle {
                center: b,
                radius: rb,
                ..
            },
        ) => distance(*a, *b) <= epsilon * 8. && (ra - rb).abs() <= epsilon * 8.,
        _ => false,
    }
}

fn intersections(a: &Span, b: &Span, epsilon: f64, tolerance: f64) -> Result<Vec<Point>> {
    // Positive rational weights place each span inside its control hull. These
    // conservative bounds discard irrelevant carrier contacts before solving.
    for axis in 0..2 {
        let interval = |span: &Span| {
            span.local_curve
                .control_points
                .iter()
                .map(|p| p[axis])
                .fold([f64::INFINITY, f64::NEG_INFINITY], |bounds, x| {
                    [bounds[0].min(x), bounds[1].max(x)]
                })
        };
        let ai = interval(a);
        let bi = interval(b);
        if ai[1] + tolerance < bi[0] || bi[1] + tolerance < ai[0] {
            return Ok(vec![]);
        }
    }
    let mut candidates = Vec::new();
    // Roundoff-indeterminate tangencies are admitted only when a retained
    // endpoint independently anchors the contact on both analytic carriers.
    // Otherwise they remain ambiguous rather than snapping a near miss.
    let tangent_anchor = |foot: Point| {
        [a.p0, a.p1, b.p0, b.p1].into_iter().find(|p| {
            distance(*p, foot) <= epsilon * 32.
                && a.contains(*p, epsilon * 8.)
                && b.contains(*p, epsilon * 8.)
        })
    };
    if same_carrier(a, b, epsilon) {
        for p in [a.p0, a.p1, b.p0, b.p1] {
            if a.contains(p, epsilon * 8.) && b.contains(p, epsilon * 8.) {
                candidates.push(p);
            }
        }
    } else {
        match (&a.carrier, &b.carrier) {
            (Carrier::Line, Carrier::Line) => {
                let da = sub(a.p1, a.p0);
                let db = sub(b.p1, b.p0);
                let determinant = cross(da, db);
                if determinant.abs() <= epsilon * norm(da).max(norm(db)) {
                    if cross(sub(b.p0, a.p0), da).abs() / norm(da) < tolerance {
                        return Err(ambiguous("Unresolved nearly coincident line carriers"));
                    }
                } else {
                    let t = cross(sub(b.p0, a.p0), db) / determinant;
                    candidates.push(add(a.p0, mul(da, t)));
                }
            }
            (Carrier::Line, Carrier::Circle { center, radius, .. })
            | (Carrier::Circle { center, radius, .. }, Carrier::Line) => {
                let line = if matches!(a.carrier, Carrier::Line) {
                    a
                } else {
                    b
                };
                let direction = mul(sub(line.p1, line.p0), 1. / distance(line.p0, line.p1));
                let foot = add(
                    line.p0,
                    mul(direction, dot(sub(*center, line.p0), direction)),
                );
                let square = radius * radius - dot(sub(foot, *center), sub(foot, *center));
                let bound = epsilon * (radius.abs() + distance(foot, *center)) * 8.;
                if square < -bound {
                } else if square.abs() <= bound {
                    if a.contains(foot, tolerance) && b.contains(foot, tolerance) {
                        candidates.push(tangent_anchor(foot).ok_or_else(|| {
                            ambiguous("Unresolved line/circle tangency without a retained endpoint witness")
                        })?);
                    }
                } else if square == 0. {
                    candidates.push(foot);
                } else {
                    let delta = square.sqrt();
                    if delta < epsilon * 8. {
                        return Err(ambiguous(
                            "Unresolved pair of near-tangent line/circle events",
                        ));
                    }
                    candidates.push(add(foot, mul(direction, delta)));
                    candidates.push(add(foot, mul(direction, -delta)));
                }
            }
            (
                Carrier::Circle {
                    center: ca,
                    radius: ra,
                    ..
                },
                Carrier::Circle {
                    center: cb,
                    radius: rb,
                    ..
                },
            ) => {
                let d = distance(*ca, *cb);
                if d <= epsilon * 8. {
                    if (ra - rb).abs() < tolerance {
                        return Err(ambiguous("Unresolved concentric circle carriers"));
                    }
                } else if d > ra + rb + tolerance || d < (ra - rb).abs() - tolerance {
                } else {
                    let x = (ra * ra - rb * rb + d * d) / (2. * d);
                    let square = ra * ra - x * x;
                    let bound = epsilon * (ra.abs() + x.abs()) * 8.;
                    if square < -bound {
                    } else {
                        let direction = mul(sub(*cb, *ca), 1. / d);
                        let foot = add(*ca, mul(direction, x));
                        if square.abs() <= bound {
                            if a.contains(foot, tolerance) && b.contains(foot, tolerance) {
                                candidates.push(tangent_anchor(foot).ok_or_else(||ambiguous("Unresolved circle/circle tangency without a retained endpoint witness"))?);
                            }
                        } else {
                            let delta = square.sqrt();
                            if delta < epsilon * 8. {
                                return Err(ambiguous(
                                    "Unresolved pair of near-tangent circle events",
                                ));
                            }
                            candidates.push(add(foot, mul(left(direction), delta)));
                            candidates.push(add(foot, mul(left(direction), -delta)));
                        }
                    }
                }
            }
        }
    }
    let mut result = Vec::new();
    for p in candidates {
        if a.contains(p, epsilon * 16.)
            && b.contains(p, epsilon * 16.)
            && !result.iter().any(|q| distance(*q, p) <= epsilon * 8.)
        {
            result.push(p);
        }
    }
    Ok(result)
}

/// Validates bounded connected simple analytic loops and material-left nesting.
pub fn validate(loops: &[Vec<Curve>], tolerance: f64) -> Result<()> {
    Region::parse(loops, tolerance)?.validate()
}
/// Exact-carrier Green area, with floating-point arithmetic and retained arcs.
pub fn signed_area(curves: &[Curve], tolerance: f64) -> Result<f64> {
    if curves.is_empty() {
        return Err(invalid("Signed area requires a nonempty closed loop"));
    }
    let region = Region::parse(&[curves.to_vec()], tolerance)?;
    let origin = region.spans[0].p0;
    Ok(region.spans.iter().map(|s| s.area(origin)).sum())
}
/// Orient simple, noncrossing even-odd rings with material on the left.
/// The complete analytic event-fragment nesting proof is shared with validate;
/// input curve definitions are only reversed, never approximated or fitted.
pub fn orient_even_odd(loops: &[Vec<Curve>], tolerance: f64) -> Result<Vec<Vec<Curve>>> {
    let region = Region::parse(loops, tolerance)?;
    let depths = region.nesting_depths(false)?;
    let mut output = Vec::with_capacity(loops.len());
    for ((curves, ids), depth) in loops.iter().zip(&region.loops).zip(depths) {
        let origin = region.spans[ids[0]].p0;
        let area = ids
            .iter()
            .map(|i| region.spans[*i].area(origin))
            .sum::<f64>();
        output.push(if (area > 0.) == (depth % 2 == 0) {
            curves.clone()
        } else {
            curves
                .iter()
                .rev()
                .map(Curve::reverse)
                .collect::<Result<Vec<_>>>()?
        });
    }
    // Once parity determines the holes, enforce the existing strict rule that
    // a hole must not touch any other boundary, even at an isolated endpoint.
    validate(&output, tolerance)?;
    Ok(output)
}
/// Group admitted material-left loops into outer components and their holes.
/// Returned indices address the original input; curve definitions are retained.
pub fn components(profile: &[Vec<Curve>], tolerance: f64) -> Result<Vec<(usize, Vec<usize>)>> {
    validate(profile, tolerance)?;
    let areas = profile
        .iter()
        .map(|wire| signed_area(wire, tolerance))
        .collect::<Result<Vec<_>>>()?;
    let mut parents = vec![None; profile.len()];
    for i in 0..profile.len() {
        for j in 0..profile.len() {
            if i == j {
                continue;
            }
            // Distinct outer components can meet at an endpoint (e.g. XOR
            // crescents). Use a point on the open span for nesting; the planar
            // arrangement validates intersections before this containment pass.
            let mut location = PointLocation::Boundary;
            'witness: for curve in &profile[i] {
                for t in [0.37, 0.63, 0.5] {
                    let [a, b] = curve.domain();
                    let p = curve.evaluate(a + (b - a) * t)?.point;
                    location =
                        locate_point(std::slice::from_ref(&profile[j]), [p[0], p[1]], tolerance)?;
                    if location != PointLocation::Boundary {
                        break 'witness;
                    }
                }
            }
            match location {
                PointLocation::Boundary => {
                    return Err(invalid(
                        "Profile loop nesting is unresolved at coincident boundaries",
                    ));
                }
                PointLocation::Inside => {
                    if parents[i].is_none_or(|k: usize| areas[j].abs() < areas[k].abs()) {
                        parents[i] = Some(j);
                    }
                }
                PointLocation::Outside => {}
            }
        }
        if areas[i].abs() <= tolerance * tolerance {
            return Err(invalid("Profile loop has zero or unresolved area"));
        }
    }
    let mut components = Vec::new();
    for i in 0..profile.len() {
        let expected_positive = parents[i].is_none_or(|parent| areas[parent] < 0.);
        if areas[i].is_sign_positive() != expected_positive {
            return Err(invalid(
                "Profile loops must alternate CCW outer boundaries and CW holes",
            ));
        }
        if areas[i] > 0. {
            components.push((
                i,
                (0..profile.len())
                    .filter(|&j| parents[j] == Some(i))
                    .collect::<Vec<_>>(),
            ));
        }
    }
    Ok(components)
}
/// Nonzero analytic winding. An isolated clockwise loop still has an inside.
pub fn locate_point(loops: &[Vec<Curve>], point: Point, tolerance: f64) -> Result<PointLocation> {
    if point.iter().any(|x| !x.is_finite()) {
        return Err(invalid("Planar query point must be finite"));
    }
    let region = Region::parse(loops, tolerance)?;
    region.location(sub(point, region.origin))
}

/// Give adjacent analytic cells the same retained edge parameterization. All
/// endpoints come from the common arrangement; no control point is fitted or
/// moved. Coincident spans select one source trim (possibly reversed).
pub(crate) fn conform_profiles(
    profiles: &[Vec<Vec<Curve>>],
    tolerance: f64,
) -> Result<Vec<Vec<Vec<Curve>>>> {
    let origin = profiles
        .iter()
        .find(|p| !p.is_empty())
        .map_or([0., 0.], |p| region_origin(p));
    let regions = profiles
        .iter()
        .map(|p| Region::parse_at(p, tolerance, origin))
        .collect::<Result<Vec<_>>>()?;
    for region in &regions {
        region.validate()?;
    }
    let spans: Vec<_> = regions.iter().flat_map(|r| &r.spans).collect();
    if spans.len() > MAX_SPANS {
        return Err(limit());
    }
    let epsilon = regions.iter().map(|r| r.epsilon).fold(0., f64::max);
    let mut cuts: Vec<_> = spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    for i in 0..spans.len() {
        for j in i + 1..spans.len() {
            for p in intersections(spans[i], spans[j], epsilon, tolerance)? {
                cuts[i].push(spans[i].parameter(p, epsilon * 8.)?);
                cuts[j].push(spans[j].parameter(p, epsilon * 8.)?);
            }
        }
    }
    let mut canonical = Vec::<Span>::new();
    let mut split = Vec::new();
    let mut work = 0;
    for (span, mut cut) in spans.into_iter().zip(cuts) {
        cut.sort_by(f64::total_cmp);
        cut.dedup();
        let mut pieces = Vec::new();
        for pair in cut.windows(2) {
            work += 1;
            if work > MAX_FRAGMENTS {
                return Err(limit());
            }
            if pair[1] - pair[0] <= 64. * f64::EPSILON * (1. + pair[0].abs()) {
                if distance(
                    point(&span.local_curve, pair[0])?,
                    point(&span.local_curve, pair[1])?,
                ) > epsilon * 8.
                {
                    return Err(ambiguous(
                        "Common cell events exceed source parameter precision",
                    ));
                }
                continue;
            }
            let mut curve = span.curve.trim(pair[0], pair[1])?;
            let [low, high] = curve.domain();
            for k in &mut curve.knots {
                *k = (*k - low) / (high - low);
            }
            let piece = Span::from_curve(curve, 0, epsilon, origin)?;
            let shared = canonical.iter().find_map(|c| {
                if !same_carrier(c, &piece, epsilon) {
                    return None;
                }
                if distance(c.p0, piece.p0) <= epsilon * 16.
                    && distance(c.p1, piece.p1) <= epsilon * 16.
                {
                    Some((c, false))
                } else if distance(c.p0, piece.p1) <= epsilon * 16.
                    && distance(c.p1, piece.p0) <= epsilon * 16.
                {
                    Some((c, true))
                } else {
                    None
                }
            });
            pieces.push(if let Some((c, reverse)) = shared {
                if reverse {
                    c.curve.reverse()?
                } else {
                    c.curve.clone()
                }
            } else {
                let curve = piece.curve.clone();
                canonical.push(piece);
                curve
            });
        }
        split.push(pieces);
    }
    let mut offset = 0;
    let mut output = Vec::new();
    for region in regions {
        let profile: Vec<Vec<Curve>> = region
            .loops
            .iter()
            .map(|wire| {
                wire.iter()
                    .flat_map(|i| split[offset + i].clone())
                    .collect()
            })
            .collect();
        offset += region.spans.len();
        validate(&profile, tolerance)?;
        output.push(profile);
    }
    Ok(output)
}

#[derive(Clone)]
struct Fragment {
    span: Span,
    source_loop: usize,
}
fn material(operation: &str, a: bool, b: bool) -> bool {
    match operation {
        "union" => a || b,
        "intersection" => a && b,
        "difference" => a && !b,
        "xor" => a != b,
        _ => false,
    }
}

/// Regularized Boolean of material-left planar loops. Empty output is valid.
/// A point contact contributes no area; separate tangent components remain
/// separate loops. Near-coincidence/ambiguous sewing fails without fallback.
pub fn boolean(
    a: &[Vec<Curve>],
    b: &[Vec<Curve>],
    operation: &str,
    tolerance: f64,
) -> Result<Vec<Vec<Curve>>> {
    if !matches!(operation, "union" | "intersection" | "difference" | "xor") {
        return Err(invalid(
            "Planar Boolean must be union, intersection, difference or xor",
        ));
    }
    let origin = if a.iter().any(|wire| !wire.is_empty()) {
        region_origin(a)
    } else {
        region_origin(b)
    };
    let a = Region::parse_at(a, tolerance, origin)?;
    let b = Region::parse_at(b, tolerance, origin)?;
    a.validate()?;
    b.validate()?;
    if a.spans.len() + b.spans.len() > MAX_SPANS {
        return Err(limit());
    }
    let epsilon = a.epsilon.max(b.epsilon);
    let mut acuts: Vec<Vec<f64>> = a.spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    let mut bcuts: Vec<Vec<f64>> = b.spans.iter().map(|s| s.curve.domain().to_vec()).collect();
    for (i, sa) in a.spans.iter().enumerate() {
        for (j, sb) in b.spans.iter().enumerate() {
            for p in intersections(sa, sb, epsilon, tolerance)? {
                acuts[i].push(sa.parameter(p, epsilon * 8.)?);
                bcuts[j].push(sb.parameter(p, epsilon * 8.)?);
            }
        }
    }
    let mut fragments: Vec<Fragment> = Vec::new();
    let mut fragment_work = 0usize;
    for (from_a, source, other, cuts) in [(true, &a, &b, &mut acuts), (false, &b, &a, &mut bcuts)] {
        for (span, cut) in source.spans.iter().zip(cuts) {
            cut.sort_by(f64::total_cmp);
            cut.dedup_by(|a, b| *a == *b);
            for parameters in cut.windows(2) {
                fragment_work += 1;
                if fragment_work > MAX_FRAGMENTS {
                    return Err(limit());
                }
                if parameters[1] - parameters[0] <= 64. * f64::EPSILON * (1. + parameters[0].abs())
                {
                    if distance(
                        point(&span.local_curve, parameters[0])?,
                        point(&span.local_curve, parameters[1])?,
                    ) > epsilon * 8.
                    {
                        return Err(ambiguous(
                            "Distinct geometric events cannot be isolated in the source parameter precision",
                        ));
                    }
                    continue;
                }
                let curve = span.curve.trim(parameters[0], parameters[1])?;
                let piece = Span::from_curve(curve, span.loop_id, epsilon, source.origin)?;
                let midpoint = piece.position(0.5)?;
                let (other_left, other_right) = match other.location(midpoint)? {
                    PointLocation::Inside => (true, true),
                    PointLocation::Outside => (false, false),
                    PointLocation::Boundary => {
                        let coincident: Vec<_> = other
                            .spans
                            .iter()
                            .filter(|s| {
                                same_carrier(&piece, s, epsilon)
                                    && s.contains(midpoint, epsilon * 16.)
                            })
                            .collect();
                        if coincident.is_empty() {
                            return Err(ambiguous(
                                "Fragment classification falls in a noncoincident boundary tolerance band",
                            ));
                        }
                        let tangent = piece.tangent(0.5)?;
                        let mut direction = None;
                        for neighbor in coincident {
                            let domain = neighbor.curve.domain();
                            let t = (neighbor.parameter(midpoint, epsilon * 8.)? - domain[0])
                                / (domain[1] - domain[0]);
                            let same = dot(tangent, neighbor.tangent(t)?) > 0.;
                            if direction.is_some_and(|d| d != same) {
                                return Err(ambiguous(
                                    "Conflicting coincident boundary material sides",
                                ));
                            }
                            direction = Some(same);
                        }
                        let same = direction.unwrap();
                        (same, !same)
                    }
                };
                let left = if from_a {
                    material(operation, true, other_left)
                } else {
                    material(operation, other_left, true)
                };
                let right = if from_a {
                    material(operation, false, other_right)
                } else {
                    material(operation, other_right, false)
                };
                if left == right {
                    continue;
                }
                let selected = if left {
                    piece
                } else {
                    Span::from_curve(
                        piece.curve.reverse()?,
                        piece.loop_id,
                        epsilon,
                        source.origin,
                    )?
                };
                let duplicate = fragments.iter().any(|f| {
                    same_carrier(&selected, &f.span, epsilon)
                        && distance(selected.p0, f.span.p0) <= epsilon * 16.
                        && distance(selected.p1, f.span.p1) <= epsilon * 16.
                });
                if !duplicate {
                    fragments.push(Fragment {
                        span: selected,
                        source_loop: span.loop_id + if from_a { 0 } else { a.loops.len() },
                    });
                }
            }
        }
    }
    let mut used = vec![false; fragments.len()];
    let mut loops = Vec::new();
    while let Some(first) = used.iter().position(|v| !*v) {
        let mut current = first;
        let mut chain = Vec::new();
        loop {
            if used[current] {
                return Err(ambiguous("Planar sewing reused an oriented fragment"));
            }
            used[current] = true;
            chain.push(fragments[current].span.curve.clone());
            let end = fragments[current].span.p1;
            if distance(end, fragments[first].span.p0) <= epsilon * 16. {
                break;
            }
            let mut candidates: Vec<_> = (0..fragments.len())
                .filter(|i| !used[*i] && distance(end, fragments[*i].span.p0) <= epsilon * 16.)
                .collect();
            if candidates.is_empty() {
                return Err(ambiguous(
                    "Planar arrangement has an unmatched boundary endpoint",
                ));
            }
            if candidates.len() > 1 {
                let same: Vec<_> = candidates
                    .iter()
                    .copied()
                    .filter(|i| fragments[*i].source_loop == fragments[current].source_loop)
                    .collect();
                if same.len() == 1 {
                    candidates = same;
                } else {
                    let incoming = fragments[current].span.tangent(1.)?;
                    let mut angles = candidates
                        .iter()
                        .map(|i| {
                            let outgoing = fragments[*i].span.tangent(0.)?;
                            Ok((
                                *i,
                                cross(incoming, outgoing)
                                    .atan2(dot(incoming, outgoing))
                                    .rem_euclid(TAU),
                            ))
                        })
                        .collect::<Result<Vec<_>>>()?;
                    angles.sort_by(|a, b| a.1.total_cmp(&b.1));
                    if angles.len() > 1 && (angles[0].1 - angles[1].1).abs() < 1e-10 {
                        return Err(ambiguous("Unresolved tangent junction in planar sewing"));
                    }
                    candidates = vec![angles[0].0];
                }
            }
            current = candidates[0];
        }
        if signed_area(&chain, tolerance)?.abs() <= tolerance * tolerance {
            return Err(ambiguous("Selected boundary has unresolved area"));
        }
        loops.push(chain);
    }
    validate(&loops, tolerance)?;
    Ok(loops)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn component_grouping_retains_nested_islands_and_parameter_domains() {
        let mut loops = orient_even_odd(
            &[
                rectangle(1., 1., 9., 9.),
                rectangle(2., 2., 3., 3.),
                rectangle(0., 0., 10., 10.),
                rectangle(12., 0., 13., 1.),
            ],
            1e-7,
        )
        .unwrap();
        for wire in &mut loops {
            for curve in wire {
                for knot in &mut curve.knots {
                    *knot = 2. + 3. * *knot;
                }
            }
        }
        assert_eq!(
            components(&loops, 1e-7).unwrap(),
            vec![(1, vec![]), (2, vec![0]), (3, vec![])]
        );
        let invalid = vec![rectangle(0., 0., 2., 2.), rectangle(1., 1., 3., 3.)];
        assert!(components(&invalid, 1e-7).is_err());
    }
    fn line(a: Point, b: Point) -> Curve {
        Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
    }
    fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<Curve> {
        let p = [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
        (0..4).map(|i| line(p[i], p[(i + 1) % 4])).collect()
    }
    fn circle(center: Point, r: f64, start: f64) -> Vec<Curve> {
        (0..4)
            .map(|i| {
                let a = start + i as f64 * PI / 2.;
                let b = a + PI / 2.;
                let m = (a + b) * 0.5;
                Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: vec![
                        add(center, [r * a.cos(), r * a.sin()]).to_vec(),
                        add(
                            center,
                            [r * 2_f64.sqrt() * m.cos(), r * 2_f64.sqrt() * m.sin()],
                        )
                        .to_vec(),
                        add(center, [r * b.cos(), r * b.sin()]).to_vec(),
                    ],
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                }
            })
            .collect()
    }
    fn area(loops: &[Vec<Curve>]) -> f64 {
        loops.iter().map(|l| signed_area(l, 1e-7).unwrap()).sum()
    }
    fn reverse(loop_: &[Curve]) -> Vec<Curve> {
        loop_.iter().rev().map(|c| c.reverse().unwrap()).collect()
    }
    #[test]
    fn analytic_area_winding_and_material_left_holes() {
        let outer = circle([0., 0.], 2., 0.);
        let hole = reverse(&circle([0., 0.], 1., 0.));
        assert!((signed_area(&outer, 1e-7).unwrap() - 4. * PI).abs() < 1e-12);
        assert!((signed_area(&hole, 1e-7).unwrap() + PI).abs() < 1e-12);
        let region = vec![hole.clone(), outer];
        validate(&region, 1e-7).unwrap();
        assert_eq!(
            locate_point(&region, [0., 0.], 1e-7).unwrap(),
            PointLocation::Outside
        );
        assert_eq!(
            locate_point(&region, [1.5, 0.], 1e-7).unwrap(),
            PointLocation::Inside
        );
        assert_eq!(
            locate_point(&[hole], [0., 0.], 1e-7).unwrap(),
            PointLocation::Inside
        );
    }
    #[test]
    fn circle_line_cut_retains_rational_arcs_and_semicircle_area() {
        let disk = vec![circle([0., 0.], 1., 0.)];
        let half = vec![rectangle(0., -2., 2., 2.)];
        for operation in ["intersection", "difference"] {
            let out = boolean(&disk, &half, operation, 1e-7).unwrap();
            assert!((area(&out) - PI / 2.).abs() < 1e-11, "{operation}");
            assert!(out.iter().flatten().any(|c| c.degree == 2));
            assert!(out.iter().flatten().any(|c| c.degree == 1));
            for curve in out.iter().flatten().filter(|c| c.degree == 2) {
                for i in 0..19 {
                    let d = curve.domain();
                    let p = point(curve, d[0] + i as f64 / 18. * (d[1] - d[0])).unwrap();
                    assert!((dot(p, p) - 1.).abs() < 1e-12);
                }
            }
        }
    }
    #[test]
    fn transverse_circle_boolean_matches_independent_lens_area() {
        let a = vec![circle([0., 0.], 1., 0.)];
        let b = vec![circle([1., 0.], 1., 0.)];
        let lens = 2. * (0.5_f64).acos() - 0.5 * 3_f64.sqrt();
        for (op, expected) in [
            ("intersection", lens),
            ("union", 2. * PI - lens),
            ("difference", PI - lens),
            ("xor", 2. * PI - 2. * lens),
        ] {
            let out = boolean(&a, &b, op, 1e-7).unwrap();
            assert!(
                (area(&out) - expected).abs() < 1e-10,
                "{op}: {} vs {expected}",
                area(&out)
            );
            if op != "difference" {
                assert!((area(&boolean(&b, &a, op, 1e-7).unwrap()) - expected).abs() < 1e-10);
            }
        }
    }
    #[test]
    fn coincidence_tangency_and_disjoint_components_are_regularized() {
        let a = vec![circle([0., 0.], 1., 0.)];
        let same = vec![circle([0., 0.], 1., PI / 4.)];
        for op in ["union", "intersection"] {
            let result = boolean(&a, &same, op, 1e-7).unwrap();
            assert!((area(&result) - PI).abs() < 1e-10);
        }
        assert!(boolean(&a, &same, "difference", 1e-7).unwrap().is_empty());
        for shift in [2., 3.] {
            let b = vec![circle([shift, 0.], 1., 0.)];
            let union = boolean(&a, &b, "union", 1e-7).unwrap();
            assert_eq!(union.len(), 2);
            assert!((area(&union) - 2. * PI).abs() < 1e-10);
            assert!(boolean(&a, &b, "intersection", 1e-7).unwrap().is_empty());
        }
    }
    #[test]
    fn holes_multiple_outputs_and_shared_line_subsegments() {
        let a = vec![rectangle(-3., -3., 3., 3.)];
        let b = vec![circle([0., 0.], 1., 0.)];
        let hole = boolean(&a, &b, "difference", 1e-7).unwrap();
        assert_eq!(hole.len(), 2);
        assert!((area(&hole) - (36. - PI)).abs() < 1e-10);
        let cutter = vec![rectangle(-0.25, -4., 0.25, 4.)];
        let split = boolean(&hole, &cutter, "difference", 1e-7).unwrap();
        assert_eq!(split.len(), 2);
        validate(&split, 1e-7).unwrap();
        let union = boolean(
            &[rectangle(0., 0., 2., 1.)],
            &[rectangle(1., 0., 3., 1.)],
            "union",
            1e-7,
        )
        .unwrap();
        assert_eq!(union.len(), 1);
        assert!((area(&union) - 3.).abs() < 1e-12);
    }
    #[test]
    fn rejects_noncircular_quadratics_and_wrong_hole_orientation() {
        let mut fake = circle([0., 0.], 1., 0.);
        fake[0].weights[1] = 1.;
        assert!(validate(&[fake], 1e-7).is_err());
        assert!(validate(&[circle([0., 0.], 2., 0.), circle([0., 0.], 1., 0.)], 1e-7).is_err());
    }

    #[test]
    fn rotated_translated_and_reparameterized_arrangements_keep_area_and_carriers() {
        let a = vec![circle([0., 0.], 2., 0.)];
        let b = vec![rectangle(0.3, -3., 4., 3.)];
        let cap = 4. * (0.15_f64).acos() - 0.3 * (4. - 0.09_f64).sqrt();
        let transform = |region: &[Vec<Curve>], reflect: bool| -> Vec<Vec<Curve>> {
            region
                .iter()
                .map(|loop_| {
                    let mut result: Vec<_> = loop_
                        .iter()
                        .map(|curve| {
                            let mut curve = curve.clone();
                            for p in &mut curve.control_points {
                                let x = if reflect { -p[0] } else { p[0] };
                                let y = p[1];
                                let (s, c) = 0.371_f64.sin_cos();
                                p[0] = 123. + 2.7 * (c * x - s * y);
                                p[1] = -55. + 2.7 * (s * x + c * y);
                            }
                            for knot in &mut curve.knots {
                                *knot = 1000. + 0.125 * (*knot);
                            }
                            curve
                        })
                        .collect();
                    result.rotate_left(1);
                    if reflect { reverse(&result) } else { result }
                })
                .collect()
        };
        for reflect in [false, true] {
            let a = transform(&a, reflect);
            let b = transform(&b, reflect);
            for (operation, expected) in [
                ("intersection", cap),
                ("difference", 4. * PI - cap),
                ("union", 4. * PI + 22.2 - cap),
                ("xor", 4. * PI + 22.2 - 2. * cap),
            ] {
                let output = boolean(&a, &b, operation, 1e-7).unwrap();
                assert!(
                    (area(&output) - expected * 2.7 * 2.7).abs() < 1e-8,
                    "{operation} reflect={reflect}"
                );
                assert!(output.iter().flatten().any(|c| c.degree == 2));
                for curve in output.iter().flatten() {
                    let domain = curve.domain();
                    assert!(domain[0] >= 1000. && domain[1] <= 1000.125);
                    let p = point(curve, (domain[0] + domain[1]) * 0.5).unwrap();
                    assert!(
                        p[0] > 110. && p[0] < 140. && p[1] > -75. && p[1] < -35.,
                        "Output curve left its original coordinate frame"
                    );
                    if curve.degree == 2 {
                        assert!((distance(p, [123., -55.]) - 5.4).abs() < 1e-9);
                    }
                }
                if operation != "difference" {
                    assert!(
                        (area(&boolean(&b, &a, operation, 1e-7).unwrap()) - area(&output)).abs()
                            < 1e-8
                    );
                }
            }
        }
        let tangent = vec![circle([4., 0.], 2., 0.)];
        let out = boolean(
            &transform(&a, false),
            &transform(&tangent, false),
            "union",
            1e-7,
        )
        .unwrap();
        assert_eq!(out.len(), 2);
        assert!((area(&out) - 8. * PI * 2.7 * 2.7).abs() < 1e-8);
    }

    #[test]
    fn bounded_refusal_preserves_sources_and_does_not_snap_near_coincidence() {
        let a = vec![rectangle(0., 0., 1., 1.)];
        let b = vec![rectangle(0., 1e-9, 1., 1. + 1e-9)];
        let before = value_codec::to_value(&a).unwrap();
        assert!(boolean(&a, &b, "union", 1e-7).is_err());
        assert_eq!(value_codec::to_value(&a).unwrap(), before);
        let huge = vec![vec![line([0., 0.], [1., 0.]); MAX_SPANS + 1]];
        assert_eq!(
            validate(&huge, 1e-7).unwrap_err().code,
            "BREP_RESOURCE_LIMIT"
        );
        assert!(boolean(&a, &[], "intersection", 1e-7).unwrap().is_empty());
        assert!((area(&boolean(&a, &[], "difference", 1e-7).unwrap()) - 1.).abs() < 1e-12);
    }

    #[test]
    fn validation_rejects_endpoint_encoded_crossings_and_hole_contacts() {
        let polygon = |points: &[Point]| -> Vec<Curve> {
            (0..points.len())
                .map(|i| line(points[i], points[(i + 1) % points.len()]))
                .collect()
        };
        let a = polygon(&[[0., 0.], [1., 0.], [2., 0.], [2., 1.], [2., 2.], [0., 2.]]);
        let b = polygon(&[[1., -1.], [3., -1.], [3., 1.], [2., 1.], [1., 1.], [1., 0.]]);
        assert!(validate(&[a, b], 1e-7).is_err());
        let outer = circle([0., 0.], 2., 0.);
        let tangent_hole = reverse(&circle([1., 0.], 1., 0.));
        assert!(validate(&[outer, tangent_hole], 1e-7).is_err());
        assert!(
            validate(
                &[rectangle(0., 0., 1., 1.), rectangle(1., 0., 2., 1.)],
                1e-7
            )
            .is_err()
        );
    }

    #[test]
    fn near_coincidence_refusal_is_independent_of_translation() {
        for [x, y] in [[0., 0.], [10_000., 0.], [10_000., -20_000.]] {
            let a = vec![rectangle(x, y, x + 1., y + 1.)];
            let b = vec![rectangle(x, y + 1e-10, x + 1., y + 1. + 1e-10)];
            assert!(
                boolean(&a, &b, "union", 1e-7).is_err(),
                "Near-coincident boundary silently merged at {x},{y}"
            );
        }
    }

    #[test]
    fn local_queries_preserve_world_outputs_and_refuse_insufficient_parameter_precision() {
        let a = vec![rectangle(10_000., -20_000., 10_001., -19_999.)];
        assert_eq!(
            locate_point(&a, [10_000.5, -19_999.5], 1e-7).unwrap(),
            PointLocation::Inside
        );
        let before = value_codec::to_value(&a).unwrap();
        let b = vec![rectangle(10_000.25, -20_001., 10_002., -19_998.)];
        let output = boolean(&a, &b, "intersection", 1e-7).unwrap();
        assert!((area(&output) - 0.75).abs() < 1e-12);
        for curve in output.iter().flatten() {
            for p in &curve.control_points {
                assert!(
                    p[0] >= 10_000.25 && p[0] <= 10_001. && p[1] >= -20_000. && p[1] <= -19_999.
                );
            }
        }
        assert_eq!(value_codec::to_value(&a).unwrap(), before);
        let mut ill_conditioned = a.clone();
        for curve in ill_conditioned.iter_mut().flatten() {
            for knot in &mut curve.knots {
                *knot = 1e8 + 1e-6 * (*knot);
            }
            curve.validate().unwrap();
        }
        assert_eq!(
            validate(&ill_conditioned, 1e-7).unwrap_err().code,
            "BREP_AMBIGUOUS_PLANAR_TRIM"
        );
    }
}
