//! Exact rational positive-winding arrangement of represented polygon wires.
//! Crossings, coincident edges and touching vertices are split before selection.
use super::*;
use std::collections::{BTreeMap, BTreeSet};
pub(in crate::profile::profile_certificate) type Point = [Q; 2];
pub(in crate::profile::profile_certificate) type Rings = Vec<Vec<Point>>;
type Edge = (Point, Point);
fn difference(a: &Point, b: &Point) -> Point {
    [&a[0] - &b[0], &a[1] - &b[1]]
}
fn cross(a: &Point, b: &Point) -> Q {
    &a[0] * &b[1] - &a[1] * &b[0]
}
fn at(a: &Point, b: &Point, t: &Q) -> Point {
    std::array::from_fn(|k| &a[k] + (&b[k] - &a[k]) * t)
}
fn on(p: &Point, a: &Point, b: &Point) -> bool {
    cross(&difference(b, a), &difference(p, a)).is_zero()
        && (0..2).all(|k| p[k] >= *(&a[k]).min(&b[k]) && p[k] <= *(&a[k]).max(&b[k]))
}
fn parameters(a: &Point, b: &Point, c: &Point, d: &Point) -> (Vec<Q>, Vec<Q>) {
    if (0..2)
        .any(|k| (&a[k]).max(&b[k]) < (&c[k]).min(&d[k]) || (&c[k]).max(&d[k]) < (&a[k]).min(&b[k]))
    {
        return (Vec::new(), Vec::new());
    }
    let u = difference(b, a);
    let v = difference(d, c);
    let w = difference(c, a);
    let denominator = cross(&u, &v);
    if !denominator.is_zero() {
        let t = cross(&w, &v) / &denominator;
        let q = cross(&w, &u) / denominator;
        if t >= Q::zero() && t <= Q::one() && q >= Q::zero() && q <= Q::one() {
            return (vec![t], vec![q]);
        }
        return (Vec::new(), Vec::new());
    }
    if !cross(&w, &u).is_zero() {
        return (Vec::new(), Vec::new());
    }
    let mut x = Vec::with_capacity(2);
    let mut y = Vec::with_capacity(2);
    let axis = if !u[0].is_zero() { 0 } else { 1 };
    let other = if !v[0].is_zero() { 0 } else { 1 };
    for p in [c, d] {
        if on(p, a, b) {
            x.push((&p[axis] - &a[axis]) / &u[axis]);
        }
    }
    for p in [a, b] {
        if on(p, c, d) {
            y.push((&p[other] - &c[other]) / &v[other]);
        }
    }
    (x, y)
}
fn edges(rings: &Rings) -> Vec<Edge> {
    rings
        .iter()
        .flat_map(|r| {
            (0..r.len()).filter_map(|i| {
                let a = r[i].clone();
                let b = r[(i + 1) % r.len()].clone();
                (a != b).then_some((a, b))
            })
        })
        .collect()
}
pub(in crate::profile::profile_certificate) fn area(r: &[Point]) -> Q {
    (0..r.len()).fold(Q::zero(), |s, i| s + cross(&r[i], &r[(i + 1) % r.len()]))
        / Q::from_integer(2.into())
}
fn winding(p: &Point, edges: &[Edge]) -> Result<i64> {
    let mut value = 0;
    for (a, b) in edges {
        if on(p, a, b) {
            return Err(refused("Exact winding point lies on a boundary"));
        }
        let side = cross(&difference(b, a), &difference(p, a));
        if a[1] <= p[1] && b[1] > p[1] && side > Q::zero() {
            value += 1;
        }
        if a[1] > p[1] && b[1] <= p[1] && side < Q::zero() {
            value -= 1;
        }
    }
    Ok(value)
}
/// Exact limiting winding at M + epsilon*N, epsilon -> 0+. Each ray
/// predicate is linear in epsilon, so the first nonzero coefficient gives
/// its sign. Atomic edge interiors contain no other crossing vertex.
fn winding_limit(m: &Point, n: &Point, edges: &[Edge]) -> Result<i64> {
    let sign = |a: Q, b: Q| {
        if !a.is_zero() {
            a.cmp(&Q::zero())
        } else {
            b.cmp(&Q::zero())
        }
    };
    let mut value = 0;
    for (a, b) in edges {
        let e = difference(b, a);
        let c = cross(&e, &difference(m, a));
        let d = cross(&e, n);
        if c.is_zero() && d.is_zero() && on(m, a, b) {
            return Err(refused(
                "Nontransversal exact infinitesimal winding witness",
            ));
        }
        let side = sign(c, d);
        let ay = sign(&a[1] - &m[1], -&n[1]);
        let by = sign(&b[1] - &m[1], -&n[1]);
        if ay != std::cmp::Ordering::Greater
            && by == std::cmp::Ordering::Greater
            && side == std::cmp::Ordering::Greater
        {
            value += 1;
        }
        if ay == std::cmp::Ordering::Greater
            && by != std::cmp::Ordering::Greater
            && side == std::cmp::Ordering::Less
        {
            value -= 1;
        }
    }
    Ok(value)
}
fn simplify(mut ring: Vec<Point>) -> Vec<Point> {
    loop {
        let before = ring.len();
        if before < 3 {
            break;
        }
        ring = (0..before)
            .filter(|&i| {
                let a = &ring[(i + before - 1) % before];
                let b = &ring[i];
                let c = &ring[(i + 1) % before];
                !on(b, a, c)
            })
            .map(|i| ring[i].clone())
            .collect();
        if ring.len() == before {
            break;
        }
    }
    if let Some(index) = (0..ring.len()).min_by_key(|i| &ring[*i]) {
        ring.rotate_left(index);
    }
    ring
}
fn angle(a: &Point, b: &Point) -> std::cmp::Ordering {
    let half = |v: &Point| usize::from(!(v[1] > Q::zero() || v[1].is_zero() && v[0] > Q::zero()));
    half(a).cmp(&half(b)).then_with(|| {
        let c = cross(a, b);
        if c > Q::zero() {
            std::cmp::Ordering::Less
        } else if c < Q::zero() {
            std::cmp::Ordering::Greater
        } else {
            std::cmp::Ordering::Equal
        }
    })
}
/// Exact NONZERO integer-weighted fill, including negative-only islands.
/// Each source ring keeps its own winding; overlaps are selected only after
/// actual complete intersection splitting, never by unioning positive fills.
pub(in crate::profile::profile_certificate) fn nonzero(
    rings: &Rings,
    coefficients: &[i32],
) -> Result<Rings> {
    if rings.len() != coefficients.len()
        || rings.len() > 32
        || coefficients.iter().any(|&c| i64::from(c).abs() > 1_000_000)
    {
        return Err(refused("Invalid exact nonzero arrangement coefficients"));
    }
    let sets = rings.iter().map(|r| vec![r.clone()]).collect::<Vec<_>>();
    // <=1022 source edges and bounded32 coefficients make this i64 sum safe.
    arrange(
        &sets,
        &|w| {
            w.iter()
                .zip(coefficients)
                .map(|(&a, &b)| a * i64::from(b))
                .sum::<i64>()
                != 0
        },
        1022,
    )
}
pub(in crate::profile::profile_certificate) fn regularize(rings: &Rings) -> Result<Rings> {
    arrange(std::slice::from_ref(rings), &|w| w[0] > 0, 1022)
}
pub(in crate::profile::profile_certificate) fn boolean(a: &Rings, b: &Rings, op: &str) -> Result<Rings> {
    if !matches!(op, "union" | "difference" | "intersection") {
        return Err(refused("Invalid exact region operation"));
    }
    arrange(
        &[a.clone(), b.clone()],
        &|w| match op {
            "union" => w[0] > 0 || w[1] > 0,
            "intersection" => w[0] > 0 && w[1] > 0,
            _ => w[0] > 0 && w[1] <= 0,
        },
        8192,
    )
}
pub(in crate::profile::profile_certificate) fn union(rings: &Rings) -> Result<Rings> {
    arrange(std::slice::from_ref(rings), &|w| w[0] > 0, 8192)
}
const MAX_INCIDENCES: usize = 32768;
const MAX_ATOMIC_EDGES: usize = 8192;
const MAX_RING_STEPS: usize = 8192;
const MAX_OUTPUT_VERTICES: usize = 4096;
/// An input edge with the index of the ring set it came from.
type OwnedEdge = (Edge, usize);
/// Winding jump of every ring set across one atomic edge.
type AtomicEdges = BTreeMap<Edge, Vec<i64>>;
/// Parameters along every input edge where another edge meets it, always
/// including both ends.
fn segment_cuts(original: &[OwnedEdge]) -> Result<Vec<Vec<Q>>> {
    let bboxes: Vec<[(&Q, &Q); 2]> = original
        .iter()
        .map(|((a, b), _)| {
            [
                ((&a[0]).min(&b[0]), (&a[0]).max(&b[0])),
                ((&a[1]).min(&b[1]), (&a[1]).max(&b[1])),
            ]
        })
        .collect();
    let mut cuts = vec![vec![Q::zero(), Q::one()]; original.len()];
    let mut incidences = 0;
    for i in 0..original.len() {
        let box_i = &bboxes[i];
        let ((a, b), _) = &original[i];
        for j in i + 1..original.len() {
            // Exact axis boxes avoid rational division for separated segments.
            let box_j = &bboxes[j];
            if box_i[0].1 < box_j[0].0
                || box_j[0].1 < box_i[0].0
                || box_i[1].1 < box_j[1].0
                || box_j[1].1 < box_i[1].0
            {
                continue;
            }
            let ((c, d), _) = &original[j];
            let (x, y) = parameters(a, b, c, d);
            incidences += x.len() + y.len();
            if incidences > MAX_INCIDENCES {
                return Err(refused("Offset arrangement incidence budget exceeded"));
            }
            cuts[i].extend(x);
            cuts[j].extend(y);
        }
    }
    Ok(cuts)
}
/// Splits every input edge at its cuts into atomic edges with a canonical
/// direction, summing the signed uses of each ring set.
fn atomic_edges(original: &[OwnedEdge], mut cuts: Vec<Vec<Q>>, sets: usize) -> Result<AtomicEdges> {
    let mut atomic = AtomicEdges::new();
    for (((a, b), owner), cut) in original.iter().zip(&mut cuts) {
        cut.sort();
        cut.dedup();
        for p in cut.windows(2) {
            let x = at(a, b, &p[0]);
            let y = at(a, b, &p[1]);
            if x == y {
                continue;
            }
            let (key, sign) = if x < y { ((x, y), 1) } else { ((y, x), -1) };
            atomic.entry(key).or_insert_with(|| vec![0; sets])[*owner] += sign;
        }
    }
    if atomic.len() > MAX_ATOMIC_EDGES {
        return Err(refused("Offset arrangement atomic edge budget exceeded"));
    }
    Ok(atomic)
}
/// Atomic edges with material on exactly one side, directed so the
/// material lies on their left.
fn material_boundary(
    atomic: &AtomicEdges,
    originals: &[Vec<Edge>],
    material: &dyn Fn(&[i64]) -> bool,
) -> Result<BTreeSet<Edge>> {
    let mut selected = BTreeSet::<Edge>::new();
    for ((a, b), jump) in atomic {
        if jump.iter().all(|j| *j == 0) {
            continue;
        }
        let middle = at(a, b, &Q::new(1.into(), 2.into()));
        let direction = difference(b, a);
        let normal = [-&direction[1], direction[0].clone()];
        let left = originals
            .iter()
            .map(|edges| winding_limit(&middle, &normal, edges))
            .collect::<Result<Vec<_>>>()?;
        let right: Vec<_> = left.iter().zip(jump).map(|(a, b)| a - b).collect();
        if material(&left) && !material(&right) {
            selected.insert((a.clone(), b.clone()));
        } else if material(&right) && !material(&left) {
            selected.insert((b.clone(), a.clone()));
        }
    }
    Ok(selected)
}
/// Neighbours of every arrangement vertex in angular order.
fn rotations(atomic: &AtomicEdges) -> BTreeMap<Point, Vec<Point>> {
    let mut around = BTreeMap::<Point, Vec<Point>>::new();
    for (a, b) in atomic.keys() {
        around.entry(a.clone()).or_default().push(b.clone());
        around.entry(b.clone()).or_default().push(a.clone());
    }
    for (center, neighbors) in &mut around {
        neighbors.sort_by(|a, b| angle(&difference(a, center), &difference(b, center)));
    }
    around
}
/// Follows the selected boundary from `first`, turning to the next selected
/// edge clockwise at every vertex, until it closes.
fn trace_ring(
    first: &Edge,
    selected: &BTreeSet<Edge>,
    around: &BTreeMap<Point, Vec<Point>>,
    used: &mut BTreeSet<Edge>,
) -> Result<Vec<Point>> {
    let mut ring = Vec::new();
    let mut edge = first.clone();
    loop {
        if !used.insert(edge.clone()) {
            return Err(refused("Offset region boundary does not close canonically"));
        }
        ring.push(edge.0.clone());
        let neighbors = around
            .get(&edge.1)
            .ok_or_else(|| refused("Missing offset halfedge twin"))?;
        let index = neighbors
            .iter()
            .position(|p| *p == edge.0)
            .ok_or_else(|| refused("Missing offset halfedge twin"))?;
        let next = (1..=neighbors.len())
            .find_map(|n| {
                let p = &neighbors[(index + neighbors.len() - n) % neighbors.len()];
                selected
                    .contains(&(edge.1.clone(), p.clone()))
                    .then_some((edge.1.clone(), p.clone()))
            })
            .ok_or_else(|| refused("Offset selected boundary is open"))?;
        if next == *first {
            return Ok(ring);
        }
        edge = next;
        if ring.len() > MAX_RING_STEPS {
            return Err(refused("Offset boundary traversal budget exceeded"));
        }
    }
}
fn arrange(sets: &[Rings], material: &dyn Fn(&[i64]) -> bool, max_edges: usize) -> Result<Rings> {
    arrange_with_limits(sets, material, max_edges, None)
}

/// Exact linear intersection in a shared UV chart, retaining collinear
/// arrangement vertices as possible shared BRep edge endpoints.
pub(crate) fn intersection_rings(
    first: Vec<Vec<[Q; 2]>>,
    second: Vec<Vec<[Q; 2]>>,
    max_work: usize,
) -> Result<Vec<Vec<Vec<[Q; 2]>>>> {
    let rings = arrange_with_limits(
        &[first, second],
        &|w| w[0] > 0 && w[1] > 0,
        8192,
        Some(max_work),
    )?;
    Ok(groups(&rings)?
        .into_iter()
        .map(|group| group.into_iter().map(|i| rings[i].clone()).collect())
        .collect())
}

fn arrange_with_limits(
    sets: &[Rings],
    material: &dyn Fn(&[i64]) -> bool,
    max_edges: usize,
    max_work: Option<usize>,
) -> Result<Rings> {
    let originals: Vec<_> = sets.iter().map(edges).collect();
    let original: Vec<OwnedEdge> = originals
        .iter()
        .enumerate()
        .flat_map(|(owner, es)| es.iter().cloned().map(move |e| (e, owner)))
        .collect();
    if original.len() > max_edges {
        return Err(refused(
            "Exact region arrangement input edge budget exceeded",
        ));
    }
    let count = original.len();
    let pair_tests = count.saturating_mul(count.saturating_sub(1)) / 2;
    let budget_error = || {
        crate::Error::new("BREP_RESOURCE_LIMIT", "Exact UV arrangement exceeded its work budget")
    };
    if max_work.is_some_and(|work| pair_tests > work) {
        return Err(budget_error());
    }
    let cuts = segment_cuts(&original)?;
    let atomic = atomic_edges(&original, cuts, sets.len())?;
    if max_work
        .is_some_and(|work| pair_tests.saturating_add(count.saturating_mul(atomic.len())) > work)
    {
        return Err(budget_error());
    }
    let selected = material_boundary(&atomic, &originals, material)?;
    let around = rotations(&atomic);
    let mut used = BTreeSet::new();
    let mut result = Vec::new();
    for first in &selected {
        if used.contains(first) {
            continue;
        }
        let ring = trace_ring(first, &selected, &around, &mut used)?;
        let ring = if max_work.is_some() { ring } else { simplify(ring) };
        if ring.len() < 3 || area(&ring).is_zero() {
            return Err(refused("Offset positive region has a degenerate boundary"));
        }
        result.push(ring);
    }
    result.sort();
    if result.iter().map(Vec::len).sum::<usize>() > MAX_OUTPUT_VERTICES {
        return Err(refused("Offset region output vertex budget exceeded"));
    }
    Ok(result)
}
pub(in crate::profile::profile_certificate) fn groups(rings: &Rings) -> Result<Vec<Vec<usize>>> {
    let outers: Vec<_> = (0..rings.len())
        .filter(|i| area(&rings[*i]) > Q::zero())
        .collect();
    let mut groups: Vec<_> = outers.iter().map(|i| vec![*i]).collect();
    for i in 0..rings.len() {
        if area(&rings[i]) > Q::zero() {
            continue;
        }
        let mut owners = Vec::new();
        for (group, &outer) in outers.iter().enumerate() {
            if winding(&rings[i][0], &edges(&vec![rings[outer].clone()]))? != 0 {
                owners.push((area(&rings[outer]), group));
            }
        }
        owners.sort();
        let owner = owners
            .first()
            .ok_or_else(|| refused("Offset hole has no material owner"))?
            .1;
        groups[owner].push(i);
    }
    Ok(groups)
}
pub(in crate::profile::profile_certificate) fn check_rounding(exact: &Rings, rounded: &Rings) -> Result<()> {
    let a = edges(exact);
    let b = edges(rounded);
    if a.len() != b.len() {
        return Err(refused("Offset rounding collapses an edge"));
    }
    for i in 0..b.len() {
        for j in i + 1..b.len() {
            let (x, _) = parameters(&b[i].0, &b[i].1, &b[j].0, &b[j].1);
            for t in x {
                let point = at(&b[i].0, &b[i].1, &t);
                let left = if point == b[i].0 {
                    Some(&a[i].0)
                } else if point == b[i].1 {
                    Some(&a[i].1)
                } else {
                    None
                };
                let right = if point == b[j].0 {
                    Some(&a[j].0)
                } else if point == b[j].1 {
                    Some(&a[j].1)
                } else {
                    None
                };
                if left.is_none() || left != right {
                    return Err(refused(
                        "Offset output rounding changes an intersection or contact",
                    ));
                }
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn ring(points: &[[i64; 2]]) -> Vec<Point> {
        points
            .iter()
            .map(|p| p.map(|x| Q::from_integer(x.into())))
            .collect()
    }
    #[test]
    fn crossings_overlaps_negative_islands_and_holes() {
        let a = ring(&[[0, 0], [4, 0], [4, 4], [0, 4]]);
        let b = ring(&[[2, 0], [6, 0], [6, 4], [2, 4]]);
        let r = regularize(&vec![a.clone(), b]).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(area(&r[0]), Q::from_integer(24.into()));
        let mut negative = ring(&[[10, 0], [12, 0], [12, 2], [10, 2]]);
        negative.reverse();
        let r = regularize(&vec![a.clone(), negative]).unwrap();
        assert_eq!(r.len(), 1);
        let mut hole = ring(&[[1, 1], [3, 1], [3, 3], [1, 3]]);
        hole.reverse();
        let r = regularize(&vec![a, hole]).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(groups(&r).unwrap()[0].len(), 2);
        let bow = ring(&[[0, 0], [4, 4], [0, 4], [4, 0]]);
        let r = regularize(&vec![bow]).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(area(&r[0]), Q::from_integer(4.into()));
    }
}
