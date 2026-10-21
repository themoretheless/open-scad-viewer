use super::*;

pub fn area(r: &[[f64; 2]]) -> f64 {
    let Some(&origin) = r.first() else {
        return 0.;
    };
    (1..r.len().saturating_sub(1))
        .map(|i| cross2(sub2(r[i], origin), sub2(r[i + 1], origin)))
        .sum::<f64>()
        / 2.
}

/// Total Euclidean perimeter of a collection of closed polygon rings.
pub fn perimeter(rings: &Rings) -> f64 {
    let mut total = 0.0;
    for r in rings {
        if r.len() < 2 {
            continue;
        }
        let n = if r.len() > 2 && r.first() == r.last() {
            r.len() - 1
        } else {
            r.len()
        };
        if n < 2 {
            continue;
        }
        for i in 0..n {
            let a = r[i];
            let b = r[(i + 1) % n];
            total += (b[0] - a[0]).hypot(b[1] - a[1]);
        }
    }
    total
}

/// Area-weighted 2D centroid of a collection of closed polygon rings (including holes
/// with opposite winding). Falls back to vertex-average centroid if the signed area
/// vanishes, or returns `None` if `rings` has no finite vertices.
pub fn centroid(rings: &Rings) -> Option<[f64; 2]> {
    let mut area2_sum = 0.0;
    let mut cx_sum = 0.0;
    let mut cy_sum = 0.0;
    let mut vx_sum = 0.0;
    let mut vy_sum = 0.0;
    let mut v_count = 0usize;

    for r in rings {
        let n = if r.len() > 1 && r.first() == r.last() {
            r.len() - 1
        } else {
            r.len()
        };
        for &p in &r[..n] {
            if !p[0].is_finite() || !p[1].is_finite() {
                return None;
            }
            vx_sum += p[0];
            vy_sum += p[1];
            v_count += 1;
        }
        if n < 3 {
            continue;
        }
        let origin = r[0];
        for i in 1..n - 1 {
            let u = sub2(r[i], origin);
            let v = sub2(r[i + 1], origin);
            let cross = cross2(u, v);
            area2_sum += cross;
            cx_sum += cross * (origin[0] + r[i][0] + r[i + 1][0]);
            cy_sum += cross * (origin[1] + r[i][1] + r[i + 1][1]);
        }
    }
    if v_count == 0 {
        return None;
    }
    if area2_sum.abs() > 1e-15 {
        Some([cx_sum / (3.0 * area2_sum), cy_sum / (3.0 * area2_sum)])
    } else {
        Some([vx_sum / v_count as f64, vy_sum / v_count as f64])
    }
}
pub(super) fn winding(p: [f64; 2], rings: &Rings) -> i32 {
    let mut winding = 0;
    for r in rings {
        for i in 0..r.len() {
            let a = r[i];
            let b = r[(i + 1) % r.len()];
            if a[1] <= p[1] && b[1] > p[1] {
                if cross2(sub2(b, a), sub2(p, a)) > 0. {
                    winding += 1;
                }
            } else if a[1] > p[1] && b[1] <= p[1] && cross2(sub2(b, a), sub2(p, a)) < 0. {
                winding -= 1;
            }
        }
    }
    winding
}
pub fn contains_point(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    inside(p, &vec![ring.to_vec()])
}
pub fn inside(p: [f64; 2], rings: &Rings) -> bool {
    inside_with_rule(p, rings, FillRule::NonZero)
}

/// Classify a point against the combined winding of all rings. Points exactly
/// on a boundary are not assigned a separate boundary classification.
pub fn inside_with_rule(p: [f64; 2], rings: &Rings, rule: FillRule) -> bool {
    filled(winding(p, rings), Some(rule))
}

fn filled(winding: i32, rule: Option<FillRule>) -> bool {
    match rule {
        Some(FillRule::NonZero) => winding != 0,
        Some(FillRule::EvenOdd) => winding % 2 != 0,
        None => winding > 0, // Existing CAD profile contract: Positive winding.
    }
}

pub fn hull2(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let mut h = Vec::with_capacity(p.len() + 1);
    for &v in &p {
        while h.len() >= 2
            && cross2(
                sub2(h[h.len() - 1], h[h.len() - 2]),
                sub2(v, h[h.len() - 1]),
            ) <= 0.
        {
            h.pop();
        }
        h.push(v)
    }
    let lower = h.len();
    for &v in p.iter().rev().skip(1) {
        while h.len() > lower
            && cross2(
                sub2(h[h.len() - 1], h[h.len() - 2]),
                sub2(v, h[h.len() - 1]),
            ) <= 0.
        {
            h.pop();
        }
        h.push(v)
    }
    h.pop();
    h
}

/// Certify a strictly convex contour by matching its complete cyclic vertex
/// order to the convex hull. Same-sign turns alone would accept a pentagram.
/// Coordinates must already be validated by the caller.
pub(crate) fn convex_boundary(ring: &[[f64; 2]]) -> Option<Vec<[f64; 2]>> {
    let ring = if ring.len() > 1 && ring.first() == ring.last() {
        &ring[..ring.len() - 1]
    } else {
        ring
    };
    if ring.len() < 3 {
        return None;
    }
    let hull = hull2(ring.to_vec());
    if hull.len() != ring.len() {
        return None;
    }
    let start = hull.iter().position(|p| *p == ring[0])?;
    let n = hull.len();
    if (0..n).all(|i| ring[i] == hull[(start + i) % n])
        || (0..n).all(|i| ring[i] == hull[(start + n - i) % n])
    {
        Some(hull)
    } else {
        None
    }
}

/// Validate the numeric domain before predicates or convex shortcuts. Local
/// squared lengths, cross products and their area sums must fit binary64. This
/// accepts the complete finite f32 coordinate range used by UI path importers;
/// the limit depends on representable arithmetic, not document units.
pub(crate) fn coordinate_metrics<'a>(
    points: impl Iterator<Item = &'a [f64; 2]>,
) -> Result<(f64, f64)> {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    let mut scale = 0_f64;
    let mut count = 0_usize;
    for point in points {
        check(
            point.iter().all(|v| v.is_finite()),
            "Invalid planar coordinate",
        )?;
        count += 1;
        for k in 0..2 {
            min[k] = min[k].min(point[k]);
            max[k] = max[k].max(point[k]);
            scale = scale.max(point[k].abs());
        }
    }
    if count == 0 {
        return Ok((0., 0.));
    }
    let extent = (max[0] - min[0]).max(max[1] - min[1]);
    let squared = extent * extent;
    check(
        extent == 0. || (squared > 0. && (squared * 8. * count as f64).is_finite()),
        "Planar extent exceeds binary64 numeric range",
    )?;
    Ok((scale, extent))
}
/// Split an arrangement at intersections, classify its two sides and retain only
/// the result boundary. Both crossing and collinear overlapping segments split.
pub fn planar(a: &Rings, b: &Rings, op: &str) -> Result<Rings> {
    planar_rule(a, b, op, None, None)
}
pub fn nonzero(r: &Rings) -> Result<Rings> {
    normalize(r, FillRule::NonZero)
}

/// Resolve crossings, overlapping contours and winding into oriented region
/// boundaries. Filled material lies to the left: outer rings are CCW and holes
/// are CW. A winding cancellation is a valid empty result.
pub fn normalize(rings: &Rings, rule: FillRule) -> Result<Rings> {
    planar_with_rules(rings, &vec![], "union", rule, rule)
}

/// Boolean two compound paths with independently specified fill rules.
/// Unlike [`planar`], clockwise outer contours are valid for both rules.
pub fn planar_with_rules(
    a: &Rings,
    b: &Rings,
    op: &str,
    rule_a: FillRule,
    rule_b: FillRule,
) -> Result<Rings> {
    check(
        matches!(
            op,
            "union" | "intersection" | "difference" | "xor" | "exclude"
        ),
        "Unknown planar boolean operation",
    )?;
    planar_rule(a, b, op, Some(rule_a), Some(rule_b))
}

fn planar_rule(
    a: &Rings,
    b: &Rings,
    op: &str,
    rule_a: Option<FillRule>,
    rule_b: Option<FillRule>,
) -> Result<Rings> {
    use crate::limits::ARRANGEMENT_ATOMS as MAX_ATOMS;
    use crate::limits::ARRANGEMENT_SPLIT_POINTS as MAX_SPLIT_POINTS;
    let edges: Vec<_> = a
        .iter()
        .chain(b)
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    check(
        edges.len() <= crate::limits::ARRANGEMENT_EDGES,
        "Planar arrangement exceeds 65536 edges",
    )?;
    let (coordinate_scale, extent) = coordinate_metrics(a.iter().chain(b).flatten())?;
    if b.is_empty()
        && a.len() == 1
        && op == "union"
        && let Some(hull) = convex_boundary(&a[0])
    {
        return Ok(if rule_a.is_none() && area(&a[0]) < 0. {
            vec![]
        } else {
            vec![hull]
        });
    }
    // Geometric tolerances follow local extent; translating a tiny contour far
    // from the origin must not erase it. Retain an ULP floor for coordinates.
    let eps = (extent * 1e-12)
        .max(coordinate_scale * f64::EPSILON * 16.)
        .max(f64::MIN_POSITIVE);
    let winding_index = WindingIndex::new(a, b);
    let choose = |p| {
        let mut count = [0; 2];
        winding_index.winding(p, &mut count);
        let [wa, wb] = count;
        let x = filled(wa, rule_a);
        let y = filled(wb, rule_b);
        match op {
            "intersection" => x && y,
            "difference" => x && !y,
            "xor" | "exclude" => x != y,
            _ => x || y,
        }
    };
    let mut points = Vec::<[f64; 2]>::new();
    let mut ids: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    let mut boundary = BTreeSet::new();
    let mut visited_atoms = BTreeSet::new();
    // Each edge pair is tested once. Bounding-box rejection avoids expensive
    // intersections for the many short, separated edges of flattened curves.
    let mut splits = vec![vec![0., 1.]; edges.len()];
    let mut split_count = 2 * edges.len();
    let ordered = |axis: usize| {
        let mut order: Vec<_> = (0..edges.len()).collect();
        order.sort_by(|&i, &j| {
            edges[i].0[axis]
                .min(edges[i].1[axis])
                .total_cmp(&edges[j].0[axis].min(edges[j].1[axis]))
        });
        order
    };
    let candidates = |order: &[usize], axis: usize| -> usize {
        order
            .iter()
            .enumerate()
            .map(|(at, &i)| {
                let end = edges[i].0[axis].max(edges[i].1[axis]) + eps;
                order[at + 1..].partition_point(|&j| edges[j].0[axis].min(edges[j].1[axis]) <= end)
            })
            .sum()
    };
    let mut axis = 0;
    let mut edge_order = ordered(axis);
    // Long horizontal strips (wide strokes and retracing) make an x-only
    // sweep quadratic even when their y intervals are well separated.
    // Choose the cheaper exact 1D candidate count before visiting any pairs.
    if edges.len() >= 128 {
        let by_y = ordered(1);
        if candidates(&by_y, 1) < candidates(&edge_order, 0) {
            axis = 1;
            edge_order = by_y;
        }
    }
    check(
        candidates(&edge_order, axis) <= crate::limits::ARRANGEMENT_PAIR_CANDIDATES,
        "Planar candidate pair budget exceeded",
    )?;
    for (order, &i) in edge_order.iter().enumerate() {
        let (p, q) = edges[i];
        let d = sub2(q, p);
        let l2 = d[0] * d[0] + d[1] * d[1];
        if l2 <= eps * eps {
            continue;
        }
        for &j in &edge_order[order + 1..] {
            let (r, s) = edges[j];
            if r[axis].min(s[axis]) > p[axis].max(q[axis]) + eps {
                break;
            }
            if (0..2).any(|k| {
                p[k].min(q[k]) > r[k].max(s[k]) + eps || r[k].min(s[k]) > p[k].max(q[k]) + eps
            }) {
                continue;
            }
            let e = sub2(s, r);
            let e2 = e[0] * e[0] + e[1] * e[1];
            if e2 <= eps * eps {
                continue;
            }
            let rp = sub2(r, p);
            let den = cross2(d, e);
            let mut add = |edge: usize, t: f64| {
                if t > 0. && t < 1. {
                    splits[edge].push(t);
                    split_count += 1;
                }
            };
            // Subtraction of nearly equal cross-product terms cannot support a
            // reliable line intersection. Treat coincident supports as overlaps
            // rather than manufacturing distant split points from roundoff.
            let denominator_error =
                (d[0] * e[1]).abs().max((d[1] * e[0]).abs()) * f64::EPSILON * 32.;
            let collinear = cross2(rp, d).abs() <= eps * l2.sqrt()
                && cross2(sub2(s, p), d).abs() <= eps * l2.sqrt();
            if !collinear && den.abs() > denominator_error {
                let t = cross2(rp, e) / den;
                let u = cross2(rp, d) / den;
                let te = eps / l2.sqrt();
                let ue = eps / e2.sqrt();
                if (-te..=1. + te).contains(&t) && (-ue..=1. + ue).contains(&u) {
                    add(i, t);
                    add(j, u);
                }
            } else if collinear {
                for v in [r, s] {
                    let v = sub2(v, p);
                    add(i, (v[0] * d[0] + v[1] * d[1]) / l2);
                }
                for v in [p, q] {
                    let v = sub2(v, r);
                    add(j, (v[0] * e[0] + v[1] * e[1]) / e2);
                }
            }
            check(
                split_count <= MAX_SPLIT_POINTS,
                "Planar intersection budget exceeded",
            )?;
        }
    }
    for (i, &(p, q)) in edges.iter().enumerate() {
        let d = sub2(q, p);
        let l2 = d[0] * d[0] + d[1] * d[1];
        if l2 <= eps * eps {
            continue;
        }
        let ts = &mut splits[i];
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() * l2.sqrt() <= eps);
        for [t0, t1] in ts.array_windows() {
            let u = [p[0] + d[0] * t0, p[1] + d[1] * t0];
            let v = [p[0] + d[0] * t1, p[1] + d[1] * t1];
            let len = ((v[0] - u[0]).powi(2) + (v[1] - u[1]).powi(2)).sqrt();
            if len < eps {
                continue;
            }
            // Interior atoms never participate in boundary tracing. Classify
            // before interning endpoints: wide/crossing strokes generate many
            // more internal intersections than visible boundary vertices.
            let mid = [(u[0] + v[0]) / 2., (u[1] + v[1]) / 2.];
            let delta = (eps * 8.).min(len * 1e-4);
            let n = [-d[1] / l2.sqrt() * delta, d[0] / l2.sqrt() * delta];
            let left = choose([mid[0] + n[0], mid[1] + n[1]]);
            let right = choose([mid[0] - n[0], mid[1] - n[1]]);
            if left == right {
                continue;
            }
            let mut index = |v: [f64; 2]| {
                let k = ((v[0] / eps).round() as i64, (v[1] / eps).round() as i64);
                // The same intersection can round to adjacent hash cells when
                // evaluated on two different source segments.
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        if let Some(candidates) = ids.get(&(k.0 + dx, k.1 + dy)) {
                            for &id in candidates {
                                if (points[id][0] - v[0]).hypot(points[id][1] - v[1]) <= eps {
                                    return id;
                                }
                            }
                        }
                    }
                }
                let id = points.len();
                points.push(v);
                ids.entry(k).or_default().push(id);
                id
            };
            let x = index(u);
            let y = index(v);
            if x == y || !visited_atoms.insert((x.min(y), x.max(y))) {
                continue;
            }
            check(
                visited_atoms.len() <= MAX_ATOMS,
                "Planar segment budget exceeded",
            )?;
            boundary.insert(if left { (x, y) } else { (y, x) });
        }
    }
    if !balanced_boundary(&boundary) {
        // Near-tangent intersections can generate distinct samples along the
        // same short support. Node those *boundary* overlaps once more, sharing
        // existing vertex identities rather than dropping open fragments.
        boundary = node_boundary_overlaps(&points, &boundary, eps)?;
    }
    check(
        balanced_boundary(&boundary),
        "Open planar arrangement boundary",
    )?;
    let mut out = Vec::new();
    while let Some(&(start, next)) = boundary.iter().next() {
        let mut ring = vec![points[start]];
        boundary.remove(&(start, next));
        let mut at = next;
        let mut previous = start;
        while at != start {
            check(ring.len() <= MAX_ATOMS, "Planar boundary exceeds budget")?;
            ring.push(points[at]);
            let incoming = sub2(points[at], points[previous]);
            let candidates: Vec<_> = boundary
                .range((at, 0)..=(at, usize::MAX))
                .copied()
                .collect();
            check(!candidates.is_empty(), "Open planar arrangement boundary")?;
            let edge = *candidates
                .iter()
                .min_by(|a, b| {
                    let turn = |end| {
                        let d = sub2(points[end], points[at]);
                        cross2(incoming, d).atan2(incoming[0] * d[0] + incoming[1] * d[1])
                    };
                    turn(b.1).total_cmp(&turn(a.1))
                })
                .unwrap();
            boundary.remove(&edge);
            previous = at;
            at = edge.1;
        }
        // Remove redundant collinear samples left by arrangement splitting.
        loop {
            let n = ring.len();
            if n <= 3 {
                break;
            }
            let remove = (0..n).find(|&i| {
                cross2(
                    sub2(ring[i], ring[(i + n - 1) % n]),
                    sub2(ring[(i + 1) % n], ring[i]),
                )
                .abs()
                    < eps
                        * ((ring[i][0] - ring[(i + n - 1) % n][0])
                            .hypot(ring[i][1] - ring[(i + n - 1) % n][1])
                            + (ring[(i + 1) % n][0] - ring[i][0])
                                .hypot(ring[(i + 1) % n][1] - ring[i][1]))
            });
            if let Some(i) = remove {
                ring.remove(i);
            } else {
                break;
            }
        }
        if ring.len() >= 3 && area(&ring).abs() > eps * eps {
            out.push(ring)
        }
    }
    out.sort_by(|a, b| area(b).abs().total_cmp(&area(a).abs()));
    Ok(out)
}
fn balanced_boundary(boundary: &BTreeSet<(usize, usize)>) -> bool {
    let mut degree = HashMap::<usize, i32>::new();
    for &(a, b) in boundary {
        *degree.entry(a).or_default() += 1;
        *degree.entry(b).or_default() -= 1;
    }
    degree.values().all(|&count| count == 0)
}

fn node_boundary_overlaps(
    points: &[[f64; 2]],
    boundary: &BTreeSet<(usize, usize)>,
    eps: f64,
) -> Result<BTreeSet<(usize, usize)>> {
    let edges: Vec<_> = boundary.iter().copied().collect();
    let mut order: Vec<_> = (0..edges.len()).collect();
    order.sort_by(|&i, &j| {
        points[edges[i].0][0]
            .min(points[edges[i].1][0])
            .total_cmp(&points[edges[j].0][0].min(points[edges[j].1][0]))
    });
    let mut splits: Vec<_> = edges.iter().map(|&(a, b)| vec![(0., a), (1., b)]).collect();
    let mut count = splits.len() * 2;
    for (position, &i) in order.iter().enumerate() {
        let (a, b) = edges[i];
        let (p, q) = (points[a], points[b]);
        let d = sub2(q, p);
        let len = d[0].hypot(d[1]);
        for &j in &order[position + 1..] {
            let (c, e) = edges[j];
            let (r, s) = (points[c], points[e]);
            if r[0].min(s[0]) > p[0].max(q[0]) + eps {
                break;
            }
            if r[1].min(s[1]) > p[1].max(q[1]) + eps || p[1].min(q[1]) > r[1].max(s[1]) + eps {
                continue;
            }
            if cross2(d, sub2(r, p)).abs() > eps * len || cross2(d, sub2(s, p)).abs() > eps * len {
                continue;
            }
            for (edge, from, to, nodes) in [(i, a, b, [c, e]), (j, c, e, [a, b])] {
                let start = points[from];
                let delta = sub2(points[to], start);
                let l2 = delta[0] * delta[0] + delta[1] * delta[1];
                for node in nodes {
                    if node == from || node == to {
                        continue;
                    }
                    let offset = sub2(points[node], start);
                    let t = (offset[0] * delta[0] + offset[1] * delta[1]) / l2;
                    if t > 0. && t < 1. {
                        splits[edge].push((t, node));
                        count += 1;
                    }
                }
            }
            check(
                count <= crate::limits::ARRANGEMENT_SPLIT_POINTS,
                "Planar boundary noding budget exceeded",
            )?;
        }
    }
    let mut out = BTreeSet::new();
    for mut split in splits {
        split.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        split.dedup_by_key(|entry| entry.1);
        for pair in split.windows(2) {
            if pair[0].1 != pair[1].1 {
                out.insert((pair[0].1, pair[1].1));
            }
        }
    }
    let opposite: Vec<_> = out
        .iter()
        .filter(|&&(a, b)| out.contains(&(b, a)))
        .copied()
        .collect();
    for edge in opposite {
        out.remove(&edge);
    }
    Ok(out)
}

pub fn offset(rings: &Rings, distance: f64, round: bool, segments: usize) -> Result<Rings> {
    offset_join(
        rings,
        distance,
        if round { "Round" } else { "Miter" },
        segments,
    )
}
