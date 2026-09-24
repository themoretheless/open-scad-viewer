//! Closed rings: area, winding, planar boolean, offset.
use crate::tessellation::FillRule;
use crate::{Result, check};
use std::collections::{BTreeSet, HashMap};

pub type Rings = Vec<Vec<math_core::V2>>;
pub use math_core::{cross2, sub2};
pub fn area(r: &[[f64; 2]]) -> f64 {
    let Some(&origin) = r.first() else {
        return 0.;
    };
    (1..r.len().saturating_sub(1))
        .map(|i| cross2(sub2(r[i], origin), sub2(r[i + 1], origin)))
        .sum::<f64>()
        / 2.
}
fn winding(p: [f64; 2], rings: &Rings) -> i32 {
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

mod winding;
use winding::WindingIndex;

pub fn hull2(mut p: Vec<[f64; 2]>) -> Vec<[f64; 2]> {
    p.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let mut h = Vec::new();
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
pub fn offset_join(rings: &Rings, distance: f64, join: &str, segments: usize) -> Result<Rings> {
    check(
        distance.is_finite() && distance.abs() <= 1e6,
        "Invalid offset",
    )?;
    if distance == 0. {
        return Ok(rings.clone());
    }
    let mut result = Vec::new();
    for r in rings {
        let mut out = Vec::new();
        for i in 0..r.len() {
            let p = r[(i + r.len() - 1) % r.len()];
            let q = r[i];
            let s = r[(i + 1) % r.len()];
            let a = sub2(q, p);
            let b = sub2(s, q);
            let la = a[0].hypot(a[1]);
            let lb = b[0].hypot(b[1]);
            if la < 1e-12 || lb < 1e-12 {
                continue;
            }
            let na = [a[1] / la, -a[0] / la];
            let nb = [b[1] / lb, -b[0] / lb];
            let u = [q[0] + na[0] * distance, q[1] + na[1] * distance];
            let v = [q[0] + nb[0] * distance, q[1] + nb[1] * distance];
            let den = cross2(a, b);
            if join == "Round" && den * distance > 1e-12 {
                let first = (na[1] * distance).atan2(na[0] * distance);
                let mut turn = cross2(na, nb).atan2(na[0] * nb[0] + na[1] * nb[1]);
                if distance < 0. && turn > 0. {
                    turn -= std::f64::consts::TAU
                }
                let steps =
                    ((turn.abs() / std::f64::consts::TAU * segments as f64).ceil() as usize).max(1);
                for k in 0..=steps {
                    let t = first + turn * k as f64 / steps as f64;
                    out.push([
                        q[0] + distance.abs() * t.cos(),
                        q[1] + distance.abs() * t.sin(),
                    ])
                }
            } else if (join == "Bevel" || join == "Square") && den * distance > 1e-12 {
                // Bevel: keep both offset endpoints (flat cut). Square extends
                // further along the bisector like SVG stroke-linejoin:square.
                if join == "Square" {
                    let bis = [na[0] + nb[0], na[1] + nb[1]];
                    let len = bis[0].hypot(bis[1]);
                    let bis = bis.map(|x| x / len);
                    for (p, d) in [(u, a), (v, b)] {
                        let t = (distance - (p[0] - q[0]) * bis[0] - (p[1] - q[1]) * bis[1])
                            / (d[0] * bis[0] + d[1] * bis[1]);
                        out.push([p[0] + t * d[0], p[1] + t * d[1]]);
                    }
                } else {
                    out.push(u);
                    out.push(v);
                }
            } else if den.abs() > 1e-12 {
                let t = cross2(sub2(v, u), b) / den;
                out.push([u[0] + a[0] * t, u[1] + a[1] * t])
            } else {
                out.push(u)
            }
        }
        if out.len() >= 3 && area(&out) * area(r) > 0. {
            result.push(out)
        }
    }
    planar(&result, &vec![], "union")
}

pub fn from_outer_holes(outer: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>>) -> Rings {
    std::iter::once(outer).chain(holes).collect()
}

pub fn work(rings: &Rings) -> usize {
    rings.iter().map(Vec::len).sum()
}

fn area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn even_odd(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// Profile contract used by SDF extrude/revolve: first ring is outer, rest are holes.
/// Validate bounded, simple, mutually disjoint boundaries without imposing one outer ring.
pub fn validate_contours(rings: &Rings) -> Result<()> {
    if rings.is_empty()
        || rings.len() > 17
        || work(rings) > 512
        || rings
            .iter()
            .any(|l| l.len() < 3 || l.iter().flatten().any(|v| !v.is_finite() || v.abs() > 1e6))
    {
        return Err(crate::error("Invalid planar region"));
    }
    let mut edges = Vec::new();
    for (li, l) in rings.iter().enumerate() {
        let loop_area = (1..l.len() - 1)
            .map(|i| area2(l[0], l[i], l[i + 1]))
            .sum::<f64>();
        if loop_area.abs() < 1e-14 {
            return Err(crate::error("Zero-area profile loop"));
        }
        for i in 0..l.len() {
            let a = l[i];
            let b = l[(i + 1) % l.len()];
            if a == b {
                return Err(crate::error("Repeated profile vertex"));
            }
            edges.push((li, i, a, b));
        }
    }
    let on = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        area2(a, b, p).abs() <= 1e-12
            && p[0] >= a[0].min(b[0]) - 1e-12
            && p[0] <= a[0].max(b[0]) + 1e-12
            && p[1] >= a[1].min(b[1]) - 1e-12
            && p[1] <= a[1].max(b[1]) + 1e-12
    };
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (li, ai, a, b) = edges[i];
            let (lj, bi, c, d) = edges[j];
            if li == lj && ((ai + 1) % rings[li].len() == bi || (bi + 1) % rings[li].len() == ai) {
                continue;
            }
            if on(a, c, d)
                || on(b, c, d)
                || on(c, a, b)
                || on(d, a, b)
                || (area2(a, b, c) > 0.) != (area2(a, b, d) > 0.)
                    && (area2(c, d, a) > 0.) != (area2(c, d, b) > 0.)
            {
                return Err(crate::error("Profile loops intersect or touch"));
            }
        }
    }
    Ok(())
}

pub fn validate_profile(rings: &Rings) -> Result<()> {
    validate_contours(rings)?;
    let outer = &rings[0];
    for (i, h) in rings[1..].iter().enumerate() {
        if !even_odd(h[0], outer)
            || rings[1..]
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && even_odd(h[0], other))
        {
            return Err(crate::error("Invalid hole containment"));
        }
    }
    Ok(())
}

pub fn signed_distance(rings: &Rings, p: [f64; 2]) -> f64 {
    let mut distance = f64::INFINITY;
    for l in rings {
        for i in 0..l.len() {
            let a = l[i];
            let b = l[(i + 1) % l.len()];
            let v = [b[0] - a[0], b[1] - a[1]];
            let t = (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / (v[0] * v[0] + v[1] * v[1]))
                .clamp(0., 1.);
            distance = distance.min((p[0] - a[0] - t * v[0]).hypot(p[1] - a[1] - t * v[1]));
        }
    }
    if !rings.is_empty() && even_odd(p, &rings[0]) && !rings[1..].iter().any(|h| even_odd(p, h)) {
        -distance
    } else {
        distance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rectangle(min: [f64; 2], max: [f64; 2]) -> Vec<[f64; 2]> {
        vec![min, [max[0], min[1]], max, [min[0], max[1]]]
    }

    fn region_area(rings: &Rings) -> f64 {
        rings.iter().map(|r| area(r)).sum()
    }

    #[test]
    fn curvex_open_gradient_stroke_polyline_remains_closed_region() {
        let p0 = [10_f32, 75.];
        let c1 = [15_f32, -20.];
        let c2 = [85_f32, 120.];
        let p3 = [90_f32, 25.];
        let mut points = Vec::new();
        for i in 0..=28 {
            let t = i as f32 / 28.;
            let mt = 1. - t;
            let (a, b, c, d) = (mt * mt * mt, 3. * mt * mt * t, 3. * mt * t * t, t * t * t);
            points.push([
                (a * p0[0] + b * c1[0] + c * c2[0] + d * p3[0]) as f64,
                (a * p0[1] + b * c1[1] + c * c2[1] + d * p3[1]) as f64,
            ]);
        }
        let path = crate::path::BezierPath::from_polyline(&points, false).unwrap();
        let contours = crate::stroke::outline_stroke_tol(
            &path,
            &crate::stroke::StrokeOptions {
                width: 7.,
                ..Default::default()
            },
            0.05,
        )
        .unwrap();
        let rings = contours.iter().map(|p| p.to_ring(0.05).unwrap()).collect();
        let mesh = crate::tessellation::tessellate_rings(&rings, FillRule::NonZero).unwrap();
        assert!(mesh.triangle_count() >= 28);
        let expected = points
            .windows(2)
            .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
            .sum::<f64>()
            * 7.;
        assert!((region_area(&rings) - expected).abs() < 1e-6);
    }

    #[test]
    fn convex_fast_route_requires_hull_order_not_only_turn_signs() {
        let points: Vec<_> = (0..5)
            .map(|i| {
                let a = i as f64 * std::f64::consts::TAU / 5.;
                [a.cos(), a.sin()]
            })
            .collect();
        assert!(convex_boundary(&points).is_some());
        assert!(convex_boundary(&points.iter().rev().copied().collect::<Vec<_>>()).is_some());
        let pentagram: Vec<_> = (0..5).map(|i| points[(i * 2) % 5]).collect();
        assert!(convex_boundary(&pentagram).is_none());
        let normalized = normalize(&vec![pentagram], FillRule::EvenOdd).unwrap();
        assert_eq!(normalized.len(), 5);
    }

    #[test]
    fn fill_rules_combine_winding_across_nested_rings() {
        let outer = rectangle([0., 0.], [10., 10.]);
        let inner = rectangle([2., 2.], [8., 8.]);
        let same = vec![outer.clone(), inner.clone()];
        assert_eq!(
            region_area(&normalize(&same, FillRule::NonZero).unwrap()),
            100.
        );
        assert_eq!(
            region_area(&normalize(&same, FillRule::EvenOdd).unwrap()),
            64.
        );
        let opposite = vec![outer, inner.into_iter().rev().collect()];
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            let result = normalize(&opposite, rule).unwrap();
            assert_eq!(region_area(&result), 64.);
            assert!(!inside([5., 5.], &result));
            assert!(inside([1., 1.], &result));
        }
    }

    #[test]
    fn normalization_resolves_overlap_and_zero_signed_area_crossing() {
        let overlap = vec![rectangle([0., 0.], [4., 4.]), rectangle([2., 0.], [6., 4.])];
        assert_eq!(
            region_area(&normalize(&overlap, FillRule::NonZero).unwrap()),
            24.
        );
        assert_eq!(
            region_area(&normalize(&overlap, FillRule::EvenOdd).unwrap()),
            16.
        );
        let bowtie = vec![vec![[0., 0.], [4., 4.], [0., 4.], [4., 0.]]];
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            let result = normalize(&bowtie, rule).unwrap();
            assert_eq!(result.len(), 2);
            assert_eq!(region_area(&result), 8.);
        }
    }

    #[test]
    fn clockwise_and_cancelled_inputs_are_valid_fills() {
        let ring = rectangle([0., 0.], [4., 4.]);
        let reversed: Vec<_> = ring.iter().rev().copied().collect();
        for rule in [FillRule::NonZero, FillRule::EvenOdd] {
            assert_eq!(
                region_area(&normalize(&vec![reversed.clone()], rule).unwrap()),
                16.
            );
            assert!(
                normalize(&vec![ring.clone(), reversed.clone()], rule)
                    .unwrap()
                    .is_empty()
            );
        }
        assert!(
            normalize(&vec![ring.clone(), ring], FillRule::EvenOdd)
                .unwrap()
                .is_empty()
        );
        assert!(normalize(&vec![], FillRule::NonZero).unwrap().is_empty());
        // Preserve the existing Positive-only CAD API.
        assert!(
            planar(&vec![reversed], &vec![], "union")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn booleans_allow_a_different_fill_rule_for_each_operand() {
        let a = vec![
            rectangle([0., 0.], [10., 10.]),
            rectangle([2., 2.], [8., 8.]),
        ];
        let b = vec![rectangle([4., 0.], [6., 10.]).into_iter().rev().collect()];
        let cut = planar_with_rules(&a, &b, "intersection", FillRule::EvenOdd, FillRule::NonZero)
            .unwrap();
        assert_eq!(region_area(&cut), 8.);
        let solid = planar_with_rules(&a, &b, "intersection", FillRule::NonZero, FillRule::EvenOdd)
            .unwrap();
        assert_eq!(region_area(&solid), 20.);
        assert!(
            planar_with_rules(&a, &b, "unknown", FillRule::NonZero, FillRule::NonZero).is_err()
        );
    }

    #[test]
    fn normalization_preserves_small_translated_regions_and_rejects_invalid_coordinates() {
        let ring = rectangle([900_000., 900_000.], [900_000.001, 900_000.001]);
        let expected = area(&ring);
        assert!(expected > 9e-7);
        let result = normalize(&vec![ring], FillRule::NonZero).unwrap();
        assert!((region_area(&result) - expected).abs() < 1e-12);
        assert!(
            normalize(
                &vec![rectangle([0., 0.], [f64::NAN, 1.])],
                FillRule::NonZero
            )
            .is_err()
        );
    }

    #[test]
    fn compound_normalization_is_independent_of_document_units() {
        for (origin, unit) in [(1e7, 10.), (0., 1e30), (0., 1e-30)] {
            let p = |x, y| [origin + x * unit, origin + y * unit];
            let rings = vec![
                rectangle(p(0., 0.), p(10., 10.)),
                rectangle(p(2., 2.), p(8., 8.)),
            ];
            let expected = area(&rings[0]) - area(&rings[1]);
            let result = normalize(&rings, FillRule::EvenOdd).unwrap();
            assert_eq!(result.len(), 2, "origin {origin}, unit {unit}");
            assert!((region_area(&result) / expected - 1.).abs() < 1e-12);
            assert!(inside(p(1., 1.), &result));
            assert!(!inside(p(5., 5.), &result));
        }
        // Non-finite intermediate products must fail before the convex shortcut.
        assert!(
            normalize(
                &vec![rectangle([0., 0.], [1e200, 1e200])],
                FillRule::NonZero
            )
            .is_err()
        );
    }

    #[test]
    fn planar_overlap_and_hole() {
        let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        let b = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
        let r = planar(&a, &b, "difference").unwrap();
        assert!((r.iter().map(|r| area(r)).sum::<f64>() - 12.).abs() < 1e-8);
    }

    #[test]
    fn planar_xor() {
        let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        let b = vec![vec![[2., 2.], [6., 2.], [6., 6.], [2., 6.]]];
        let r = planar(&a, &b, "xor").unwrap();
        let s: f64 = r.iter().map(|ring| area(ring).abs()).sum();
        assert!((s - 24.).abs() < 1e-6, "xor area={s}");
    }
}
