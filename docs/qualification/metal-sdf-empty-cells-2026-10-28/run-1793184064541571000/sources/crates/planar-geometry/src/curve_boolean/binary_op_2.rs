use super::*;

/// Boolean operation with a separate source winding rule for each operand.
pub fn binary_op(
    a: &BezPath,
    b: &BezPath,
    rule_a: FillRule,
    rule_b: FillRule,
    op: BinaryOp,
    tolerance: f64,
) -> Result<Contours> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Invalid curve Boolean tolerance",
    )?;
    // Parsing validates command order and finite coordinates; open contours are
    // rejected instead of silently closing editable open paths.
    for p in a
        .to_bezier_paths()?
        .iter()
        .chain(b.to_bezier_paths()?.iter())
    {
        check(p.closed, "Curve Boolean requires closed contours")?;
    }
    let a_count = a.segments().count();
    let curves: Vec<_> = a.segments().chain(b.segments()).collect();
    check(
        curves.len() <= 65_536,
        "Curve Boolean segment budget exceeded",
    )?;
    if curves.is_empty() {
        return Ok(Contours::default());
    }
    let mut minimum = [f64::INFINITY; 2];
    let mut maximum = [f64::NEG_INFINITY; 2];
    let mut magnitude = 0_f64;
    for &c in &curves {
        let (points, count) = controls(c);
        for p in &points[..count] {
            check(
                p.x.is_finite() && p.y.is_finite(),
                "Invalid curve Boolean coordinate",
            )?;
            minimum[0] = minimum[0].min(p.x);
            minimum[1] = minimum[1].min(p.y);
            maximum[0] = maximum[0].max(p.x);
            maximum[1] = maximum[1].max(p.y);
            magnitude = magnitude.max(p.x.abs()).max(p.y.abs());
        }
    }
    let extent = (maximum[0] - minimum[0]).max(maximum[1] - minimum[1]);
    check(
        extent.is_finite(),
        "Curve Boolean coordinate span overflows",
    )?;
    let numeric = (extent * 1e-10)
        .max(magnitude * f64::EPSILON * 64.)
        .max(1e-13);
    // Broad-phase exact curve bounds before constructing approximation chords.
    // Most existing compound holes are disjoint, so re-flattening every one on
    // each subtraction fold is avoidable. Monotone curves cannot self-cross;
    // shared endpoint-only contacts already have both required parameters.
    let bounds: Vec<_> = curves.iter().map(|&c| c.bounding_box()).collect();
    let mut active: Vec<_> = curves.iter().map(|&c| !has_monotone_axis(c)).collect();
    let mut possible_pairs = BTreeSet::new();
    let mut curve_order: Vec<_> = (0..curves.len()).collect();
    curve_order.sort_by(|&a, &b| bounds[a].x0.total_cmp(&bounds[b].x0));
    let mut broad_comparisons = 0usize;
    for (position, &i) in curve_order.iter().enumerate() {
        for &j in &curve_order[position + 1..] {
            broad_comparisons += 1;
            check(
                broad_comparisons <= 8_000_000,
                "Curve Boolean broad-phase work budget exceeded",
            )?;
            let aa = bounds[i];
            let bb = bounds[j];
            if bb.x0 > aa.x1 + numeric * 4. {
                break;
            }
            if bb.y0 > aa.y1 + numeric * 4. || aa.y0 > bb.y1 + numeric * 4. {
                continue;
            }
            if no_interior_intersection(curves[i], curves[j], aa, bb) {
                continue;
            }
            active[i] = true;
            active[j] = true;
            possible_pairs.insert((i.min(j), i.max(j)));
        }
    }
    let chord_tol = (tolerance * 0.125).min(extent * 0.001).max(numeric * 4.);
    let mut chords = Vec::new();
    let mut chord_ranges = Vec::new();
    for (i, &c) in curves.iter().enumerate() {
        let begin = chords.len();
        if active[i] {
            flatten(c, i, 0., 1., chord_tol, 0, &mut chords)?;
        }
        chord_ranges.push(begin..chords.len());
    }
    let mut parameters = vec![vec![0., 1.]; curves.len()];
    let mut checked_pairs = BTreeSet::new();
    let mut coincident_pairs = HashMap::new();
    let mut order: Vec<_> = (0..chords.len()).collect();
    order.sort_by(|&i, &j| {
        chords[i]
            .a
            .x
            .min(chords[i].b.x)
            .total_cmp(&chords[j].a.x.min(chords[j].b.x))
    });
    let mut comparisons = 0usize;
    for (position, &i) in order.iter().enumerate() {
        let x = chords[i];
        for &j in &order[position + 1..] {
            let y = chords[j];
            if y.a.x.min(y.b.x) > x.a.x.max(x.b.x) + 2. * chord_tol {
                break;
            }
            if y.a.y.min(y.b.y) > x.a.y.max(x.b.y) + 2. * chord_tol
                || x.a.y.min(x.b.y) > y.a.y.max(y.b.y) + 2. * chord_tol
            {
                continue;
            }
            if x.source == y.source && ((x.t1 - y.t0).abs() < 1e-12 || (y.t1 - x.t0).abs() < 1e-12)
            {
                continue;
            }
            comparisons += 1;
            check(
                comparisons <= 8_000_000,
                "Curve Boolean intersection budget exceeded",
            )?;
            let pair = (x.source.min(y.source), x.source.max(y.source));
            if x.source != y.source && !possible_pairs.contains(&pair) {
                continue;
            }
            if x.source != y.source
                && checked_pairs.insert(pair)
                && let Some(((t0, u0), (t1, u1))) = coincident_interval(
                    curves[pair.0],
                    curves[pair.1],
                    &chords[chord_ranges[pair.0].clone()],
                    &chords[chord_ranges[pair.1].clone()],
                    numeric,
                    chord_tol,
                )
            {
                parameters[pair.0].extend([t0, t1]);
                parameters[pair.1].extend([u0, u1]);
                coincident_pairs.insert(pair, ((t0, u0), (t1, u1)));
            }
            // Coincident arcs have infinitely many intersections. Splitting at
            // every approximate chord crossing creates mismatched micro-spans.
            // Their exact overlap endpoints partition both original curves.
            if let Some(&((t0, u0), (t1, u1))) = coincident_pairs.get(&pair) {
                let (first, second) = if x.source == pair.0 { (x, y) } else { (y, x) };
                let map = |u: f64| t0 + (u - u0) * (t1 - t0) / (u1 - u0);
                let a = map(second.t0);
                let b = map(second.t1);
                // Skip only matching parameter spans. A looping cubic may
                // cross a distinct part of itself beyond its shared span.
                if first.t0 <= a.max(b) + 1e-9 && first.t1 >= a.min(b) - 1e-9 {
                    continue;
                }
            }
            let d = sub(x.b, x.a);
            let e = sub(y.b, y.a);
            let delta = sub(y.a, x.a);
            let det = cross(d, e);
            if det.abs() > numeric * len(d).max(len(e)) {
                let t = cross(delta, e) / det;
                let u = cross(delta, d) / det;
                let margin_x = 2. * chord_tol / len(d).max(numeric);
                let margin_y = 2. * chord_tol / len(e).max(numeric);
                if t >= -margin_x
                    && t <= 1. + margin_x
                    && u >= -margin_y
                    && u <= 1. + margin_y
                    && let Some((t, u)) = refine(
                        curves[x.source],
                        curves[y.source],
                        x.t0 + (x.t1 - x.t0) * t.clamp(0., 1.),
                        y.t0 + (y.t1 - y.t0) * u.clamp(0., 1.),
                        numeric,
                    )
                    && t >= x.t0 - 1e-6
                    && t <= x.t1 + 1e-6
                    && u >= y.t0 - 1e-6
                    && u <= y.t1 + 1e-6
                {
                    parameters[x.source].push(t);
                    parameters[y.source].push(u);
                }
            } else if len(d) > numeric
                && len(e) > numeric
                && cross(delta, d).abs() <= 2. * chord_tol * len(d)
            {
                // Coincident lines/curves and tangencies: project endpoint
                // candidates onto the actual other curve, never onto output chords.
                for (s, t, other, lo, hi) in [
                    (x.source, x.t0, y.source, y.t0, y.t1),
                    (x.source, x.t1, y.source, y.t0, y.t1),
                    (y.source, y.t0, x.source, x.t0, x.t1),
                    (y.source, y.t1, x.source, x.t0, x.t1),
                ] {
                    let point = curves[s].eval(t);
                    let q = nearest_parameter(curves[other], point, lo, hi);
                    if curves[other].eval(q).distance(point) <= numeric * 4. {
                        parameters[s].push(t);
                        parameters[other].push(q);
                    }
                }
            }
        }
    }
    let prepared_a = PreparedWinding::new(&curves[..a_count], &bounds[..a_count]);
    let prepared_b = PreparedWinding::new(&curves[a_count..], &bounds[a_count..]);
    let inside = |p: Point| {
        let classify = |w: i32, rule| match rule {
            FillRule::NonZero => w != 0,
            FillRule::EvenOdd => w % 2 != 0,
        };
        let x = classify(prepared_a.winding(p), rule_a);
        let y = classify(prepared_b.winding(p), rule_b);
        match op {
            BinaryOp::Union => x || y,
            BinaryOp::Intersection => x && y,
            BinaryOp::Difference => x && !y,
            BinaryOp::Xor => x != y,
        }
    };
    let mut vertices = Vec::<Point>::new();
    let mut grid = HashMap::<(i64, i64), Vec<usize>>::new();
    let mut boundary = Vec::new();
    let mut seen = BTreeSet::new();
    for (i, mut ts) in parameters.into_iter().enumerate() {
        for t in &mut ts {
            if *t < 1e-9 {
                *t = 0.;
            } else if *t > 1. - 1e-9 {
                *t = 1.;
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);
        for pair in ts.windows(2) {
            if pair[1] - pair[0] <= 1e-10 {
                continue;
            }
            let mut c = curves[i].subsegment(pair[0], pair[1]);
            // A cubic may have a stationary point at its exact midpoint.
            // The whole span remains a valid boundary; select another interior
            // point instead of mistaking the stationary tangent for zero area.
            let Some((middle, direction, length)) = [0.5, 0.3819660112501051, 0.6180339887498949]
                .into_iter()
                .find_map(|t| {
                    let tangent = c.derivative(t);
                    let direction = [tangent.x, tangent.y];
                    let length = len(direction);
                    (length > numeric).then(|| (c.eval(t), direction, length))
                })
            else {
                continue;
            };
            // Classification has a tighter precision budget than vertex welds:
            // a weld-sized probe can jump across a thin crescent between nearly
            // coincident curves and remove a real boundary.
            let probe = (magnitude * f64::EPSILON * 8.)
                .max(extent * 1e-14)
                .max(1e-15);
            let distance = (probe * 16.).min(length * 1e-3);
            let normal = [
                -direction[1] / length * distance,
                direction[0] / length * distance,
            ];
            let left = inside(Point::new(middle.x + normal[0], middle.y + normal[1]));
            let right = inside(Point::new(middle.x - normal[0], middle.y - normal[1]));
            if left == right {
                continue;
            }
            if !left {
                c = c.reverse();
            }
            let from = vertex_id(c.start(), numeric * 8., &mut vertices, &mut grid);
            let to = vertex_id(c.end(), numeric * 8., &mut vertices, &mut grid);
            if from == to && c.eval(0.5).distance(c.start()) < numeric {
                continue;
            }
            // Distinct arcs can share endpoints; include their midpoint in the key.
            // A straight cubic may traverse its line with nonuniform speed.
            // Its parametric midpoint is then different from the line's even
            // though their entire geometry is coincident. Canonicalize only
            // the key, retaining the actual original curve for output.
            let mid = if flat(c, numeric * 4.) {
                at(c.start(), c.end(), 0.5)
            } else {
                c.eval(0.5)
            };
            let key = (
                from,
                to,
                (mid.x / (numeric * 16.)).round() as i64,
                (mid.y / (numeric * 16.)).round() as i64,
            );
            if seen.insert(key) {
                boundary.push(Boundary { curve: c, from, to });
            }
        }
    }
    let mut outgoing = vec![Vec::new(); vertices.len()];
    for (i, edge) in boundary.iter().enumerate() {
        outgoing[edge.from].push(i);
    }
    let mut used = vec![false; boundary.len()];
    let mut result = Vec::new();
    for first in 0..boundary.len() {
        if used[first] {
            continue;
        }
        let start = boundary[first].from;
        let mut current = first;
        let mut elements = vec![PathEl::MoveTo(vertices[start])];
        let mut count = 0;
        loop {
            check(!used[current], "Curve Boolean reused boundary")?;
            used[current] = true;
            let edge = boundary[current];
            let c = snap_endpoints(edge.curve, vertices[edge.from], vertices[edge.to]);
            elements.push(c.to_path_el());
            count += 1;
            check(
                count <= boundary.len(),
                "Curve Boolean boundary budget exceeded",
            )?;
            if edge.to == start {
                break;
            }
            let t = c.derivative(1.);
            let incoming = [t.x, t.y];
            current=outgoing[edge.to].iter().filter(|&&i|!used[i]).copied().max_by(|&i,&j| {
                let turn=|k:usize|{let t=boundary[k].curve.derivative(0.);cross(incoming,[t.x,t.y]).atan2(incoming[0]*t.x+incoming[1]*t.y)};
                turn(i).total_cmp(&turn(j))
            }).ok_or_else(||crate::error(format!("Open curve Boolean arrangement boundary at {:?}, incoming {:?}, {} total edges, outgoing {:?}",vertices[edge.to],edge.curve,boundary.len(),outgoing[edge.to].iter().map(|&i|(used[i],boundary[i].curve)).collect::<Vec<_>>())))?;
        }
        elements.push(PathEl::ClosePath);
        let path = BezPath::from_vec(elements.clone());
        if path.area().abs() > numeric * numeric {
            result.push(Contour { path: elements });
        }
    }
    result.sort_by(|a, b| {
        BezPath::from_vec(b.path.clone())
            .area()
            .abs()
            .total_cmp(&BezPath::from_vec(a.path.clone()).area().abs())
    });
    Ok(Contours { contours: result })
}

fn refine(
    a: CurveSegment,
    b: CurveSegment,
    mut t: f64,
    mut u: f64,
    eps: f64,
) -> Option<(f64, f64)> {
    for _ in 0..32 {
        let p = a.eval(t);
        let q = b.eval(u);
        let f = sub(p, q);
        if len(f) <= eps {
            return Some((t, u));
        }
        let da = a.derivative(t);
        let db = b.derivative(u);
        let da = [da.x, da.y];
        let db = [db.x, db.y];
        let den = cross(da, db);
        if den.abs() <= 1e-18 * len(da) * len(db) {
            return None;
        }
        let dt = -cross(f, db) / den;
        let du = -cross(f, da) / den;
        let next_t = (t + dt).clamp(0., 1.);
        let next_u = (u + du).clamp(0., 1.);
        if (next_t - t).abs() + (next_u - u).abs() < 1e-15 {
            break;
        }
        t = next_t;
        u = next_u;
    }
    (a.eval(t).distance(b.eval(u)) <= eps * 4.).then_some((t, u))
}
fn coincident_interval(
    a: CurveSegment,
    b: CurveSegment,
    a_chords: &[Chord],
    b_chords: &[Chord],
    eps: f64,
    chord_tolerance: f64,
) -> Option<((f64, f64), (f64, f64))> {
    let mut matches = Vec::new();
    for t in [0., 1.] {
        if let Some(u) = parameter_at_point(b, a.eval(t), b_chords, eps, chord_tolerance) {
            matches.push((t, u));
        }
    }
    for u in [0., 1.] {
        if let Some(t) = parameter_at_point(a, b.eval(u), a_chords, eps, chord_tolerance) {
            matches.push((t, u));
        }
    }
    matches.sort_by(|a, b| a.0.total_cmp(&b.0));
    let &(t0, u0) = matches.first()?;
    let &(t1, u1) = matches.last()?;
    if t1 - t0 < 1e-9 || (u1 - u0).abs() < 1e-9 {
        return None;
    }
    let a_span = a.subsegment(t0, t1);
    let b_span = b.subsegment(u0, u1);
    // Four samples characterize a cubic polynomial under an affine parameter
    // restriction. Additional samples improve roundoff discrimination.
    for t in [0., 0.2, 0.4, 0.6, 0.8, 1.] {
        if a_span.eval(t).distance(b_span.eval(t)) > eps * 4. {
            return None;
        }
    }
    Some(((t0, u0), (t1, u1)))
}
fn parameter_at_point(
    curve: CurveSegment,
    p: Point,
    chords: &[Chord],
    eps: f64,
    chord_tolerance: f64,
) -> Option<f64> {
    for t in [0., 1.] {
        if curve.eval(t).distance(p) <= eps {
            return Some(t);
        }
    }
    for chord in chords {
        let pad = chord_tolerance + eps;
        if p.x < chord.a.x.min(chord.b.x) - pad
            || p.x > chord.a.x.max(chord.b.x) + pad
            || p.y < chord.a.y.min(chord.b.y) - pad
            || p.y > chord.a.y.max(chord.b.y) + pad
        {
            continue;
        }
        let t = nearest_parameter(curve, p, chord.t0, chord.t1);
        if curve.eval(t).distance(p) <= eps {
            return Some(t);
        }
    }
    None
}
fn nearest_parameter(curve: CurveSegment, p: Point, mut lo: f64, mut hi: f64) -> f64 {
    // Convex local chord intervals; golden-section minimization remains stable
    // when a Newton intersection Jacobian is singular at a tangency.
    for _ in 0..50 {
        let t = lo + (hi - lo) / 3.;
        let u = hi - (hi - lo) / 3.;
        if curve.eval(t).distance(p) < curve.eval(u).distance(p) {
            hi = u;
        } else {
            lo = t;
        }
    }
    (lo + hi) * 0.5
}
fn vertex_id(
    p: Point,
    eps: f64,
    points: &mut Vec<Point>,
    grid: &mut HashMap<(i64, i64), Vec<usize>>,
) -> usize {
    let key = ((p.x / eps).round() as i64, (p.y / eps).round() as i64);
    for dx in -1..=1 {
        for dy in -1..=1 {
            if let Some(ids) = grid.get(&(key.0 + dx, key.1 + dy)) {
                for &i in ids {
                    if points[i].distance(p) <= eps {
                        return i;
                    }
                }
            }
        }
    }
    let id = points.len();
    points.push(p);
    grid.entry(key).or_default().push(id);
    id
}
fn snap_endpoints(c: CurveSegment, start: Point, end: Point) -> CurveSegment {
    match c {
        CurveSegment::Line(..) => CurveSegment::Line(start, end),
        CurveSegment::Quad(_, p, _) => CurveSegment::Quad(start, p, end),
        CurveSegment::Cubic(_, p, q, _) => CurveSegment::Cubic(start, p, q, end),
    }
}
