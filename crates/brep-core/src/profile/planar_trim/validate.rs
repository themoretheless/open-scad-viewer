use super::*;

impl Region {
    pub(super) fn parse(loops: &[Vec<Curve>], tolerance: f64) -> Result<Self> {
        Self::parse_at(loops, tolerance, region_origin(loops))
    }
    pub(super) fn parse_at(loops: &[Vec<Curve>], tolerance: f64, origin: Point) -> Result<Self> {
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
    pub(super) fn location(&self, p: Point) -> Result<PointLocation> {
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
    pub(super) fn validate(&self) -> Result<()> {
        self.nesting_depths(true).map(|_| ())
    }
    pub(super) fn nesting_depths(&self, material_left: bool) -> Result<Vec<usize>> {
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

pub(super) fn same_carrier(a: &Span, b: &Span, epsilon: f64) -> bool {
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

pub(super) fn intersections(a: &Span, b: &Span, epsilon: f64, tolerance: f64) -> Result<Vec<Point>> {
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
