//! Rational triangulation proposes indices only. Exact rational boundaries,
//! positive disk maps and winding certify the resulting trimmed region.
use super::*;
type Edge = (P, P);
fn side(a: &P, b: &P, c: &P) -> Q {
    (&b[0] - &a[0]) * (&c[1] - &a[1]) - (&b[1] - &a[1]) * (&c[0] - &a[0])
}
fn winding(p: &P, ring: &[P]) -> Result<i32> {
    let mut w = 0;
    for (a, b) in ring
        .iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
    {
        let s = side(a, b, p);
        if s.is_zero()
            && (0..2).all(|k| {
                p[k] >= *std::cmp::min(&a[k], &b[k]) && p[k] <= *std::cmp::max(&a[k], &b[k])
            })
        {
            return Err(fail("trim chord boundaries touch"));
        }
        if a[1] <= p[1] && p[1] < b[1] && s > Q::zero() {
            w += 1;
        }
        if b[1] <= p[1] && p[1] < a[1] && s < Q::zero() {
            w -= 1;
        }
    }
    Ok(w)
}
fn motion(h: &[sb::H]) -> Curve {
    let n = h.len() - 1;
    let (a, b) = (sb::point(&h[0]), sb::point(h.last().unwrap()));
    let from = (0..=n + 1)
        .map(|i| {
            std::array::from_fn(|k| {
                let t = Q::new(i.into(), (n + 1).into());
                (if i <= n {
                    (Q::one() - &t) * &h[i][k]
                } else {
                    Q::zero()
                }) + (if i > 0 { &t * &h[i - 1][k] } else { Q::zero() })
            })
        })
        .collect::<Vec<sb::H>>();
    let to = (0..=n + 1)
        .map(|i| {
            std::array::from_fn(|k| {
                if k == 3 {
                    return from[i][3].clone();
                }
                let t = Q::new(i.into(), (n + 1).into());
                (if i <= n {
                    (Q::one() - &t) * &h[i][3] * &a[k]
                } else {
                    Q::zero()
                }) + (if i > 0 {
                    &t * &h[i - 1][3] * &b[k]
                } else {
                    Q::zero()
                })
            })
        })
        .collect();
    [from, to]
}
fn add(counts: &mut BTreeMap<Edge, i32>, a: P, b: P, sign: i32) {
    if a < b {
        *counts.entry((a, b)).or_default() += sign;
    } else {
        *counts.entry((b, a)).or_default() -= sign;
    }
}
// Extract each authored nonempty knot span separately. No samples or rounded
// parameters enter the boundary: both neighbours retain the exact knot value.
fn boundary_spans(c: &crate::Curve, work: &mut Work) -> Result<Vec<Vec<sb::H>>> {
    c.validate()?;
    let [a, b] = c.domain();
    let mut knots = c
        .knots
        .iter()
        .copied()
        .filter(|t| *t >= a && *t <= b)
        .map(|t| Q::from_float(t).unwrap())
        .collect::<Vec<_>>();
    knots.sort();
    knots.dedup();
    if knots.len() < 2 || knots.len() > 65 {
        return Err(fail("trim knot-span budget exceeded"));
    }
    let mut spans = Vec::new();
    for interval in knots.windows(2) {
        work.charge(c.control_points.len())?;
        let h = sb::curve(c, &interval[0], &interval[1])?;
        if spans.last().is_some_and(|previous: &Vec<sb::H>| {
            sb::point(previous.last().unwrap()) != sb::point(&h[0])
        }) {
            return Err(fail("trim knot spans are discontinuous"));
        }
        spans.push(h);
    }
    Ok(spans)
}
pub(super) fn charts(m: &Model, face: usize, work: &mut Work) -> Result<Vec<Net>> {
    let f = &m.faces[face];
    let mut curves = BTreeMap::<Edge, Vec<sb::H>>::new();
    let mut rings = Vec::<Vec<P>>::new();
    let mut motions = Vec::<(usize, usize, Curve)>::new();
    let mut vertices = BTreeSet::new();
    for (wi, l) in std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .enumerate()
    {
        let edges = &m.loops[l].coedges;
        if edges.len() < 3 || edges.len() > 64 || vertices.len() + edges.len() > 128 {
            return Err(fail("trim region boundary budget exceeded"));
        }
        let mut ring = Vec::new();
        let mut previous = None;
        let mut first = None;
        let mut spans = Vec::new();
        for e in edges {
            spans.extend(boundary_spans(&e.pcurve, work)?);
            if spans.len() > 64 || vertices.len() + spans.len() > 128 {
                return Err(fail("trim region split boundary budget exceeded"));
            }
        }
        for (ei, h) in spans.into_iter().enumerate() {
            let (a, b) = (sb::point(&h[0]), sb::point(h.last().unwrap()));
            if a == b || !vertices.insert(a.clone()) || previous.as_ref().is_some_and(|p| *p != a) {
                return Err(fail("trim region repeats a vertex or is not joined"));
            }
            if first.is_none() {
                first = Some(a.clone());
            }
            previous = Some(b.clone());
            ring.push(a.clone());
            motions.push((wi, ei, motion(&h)));
            if curves.insert((a, b), h).is_some() {
                return Err(fail("trim region repeats a boundary"));
            }
        }
        if first != previous {
            return Err(fail("trim region wire is open"));
        }
        let area: Q = ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
            .map(|(a, b)| &a[0] * &b[1] - &a[1] * &b[0])
            .sum();
        if (wi == 0 && area <= Q::zero()) || (wi != 0 && area >= Q::zero()) {
            return Err(fail("trim region wire orientation is false"));
        }
        rings.push(ring);
    }
    let n = [Q::zero(), Q::zero(), Q::one()];
    // The entire original wire family remains disjoint Jordan curves during
    // its homotopy to the chord family. No numerical containment is trusted.
    for (i, (wi, ei, a)) in motions.iter().enumerate() {
        if !monotone(a, &n, work)? {
            return Err(fail("trim region boundary monotonicity is unproved"));
        }
        for (wj, ej, b) in &motions[i + 1..] {
            let size = rings[*wi].len();
            let shared = if wi == wj && (*ei + 1) % size == *ej {
                Some(&rings[*wi][*ej])
            } else if wi == wj && (*ej + 1) % size == *ei {
                Some(&rings[*wi][*ei])
            } else {
                None
            };
            if !curve_pair(a, b, shared, &n, work)? {
                return Err(fail("trim region wire isotopy is unproved"));
            }
        }
    }
    for i in 1..rings.len() {
        if winding(&rings[i][0], &rings[0])? != 1 {
            return Err(fail("trim hole is not inside its outer wire"));
        }
        for j in 1..rings.len() {
            if i != j && winding(&rings[i][0], &rings[j])? != 0 {
                return Err(fail("trim holes are nested or overlap"));
            }
        }
    }
    let (positions, indices) = proposal(&rings, work)?;
    match certify(m, face, &curves, &positions, &indices, work) {
        Ok(charts) => Ok(charts),
        Err(error) => {
            // A smooth outer boundary can give a degenerate UV triangle at a
            // boundary-only diagonal. Try a rational interior fan instead.
            // This is a proposal: the same exact disk/winding checker decides.
            if rings.len() != 1 {
                return Err(error);
            }
            let ring = &rings[0];
            let center: P = std::array::from_fn(|k| {
                ring.iter().map(|p| p[k].clone()).sum::<Q>() / integer(ring.len() as i64)
            });
            if winding(&center, ring)? != 1 {
                return Err(error);
            }
            let mut positions = ring.clone();
            positions.push(center);
            let center = ring.len() as u32;
            let indices = (0..ring.len())
                .flat_map(|i| [center, i as u32, ((i + 1) % ring.len()) as u32])
                .collect::<Vec<_>>();
            certify(m, face, &curves, &positions, &indices, work)
        }
    }
}
fn certify(
    m: &Model,
    face: usize,
    curves: &BTreeMap<Edge, Vec<sb::H>>,
    positions: &[P],
    indices: &[u32],
    work: &mut Work,
) -> Result<Vec<Net>> {
    if indices.is_empty() || indices.len() % 3 != 0 || indices.len() > 3072 {
        return Err(fail("trim proposal triangle budget exceeded"));
    }
    let mut residual = BTreeMap::new();
    for (a, b) in curves.keys() {
        add(&mut residual, a.clone(), b.clone(), -1);
    }
    let mut incidence = BTreeMap::<Edge, usize>::new();
    let mut charts = Vec::new();
    for t in indices.chunks_exact(3) {
        let c = t
            .iter()
            .map(|i| {
                positions
                    .get(*i as usize)
                    .cloned()
                    .ok_or_else(|| fail("trim proposal index is false"))
            })
            .collect::<Result<Vec<_>>>()?;
        if side(&c[0], &c[1], &c[2]) <= Q::zero() {
            return Err(fail("trim proposal has a nonpositive simplex"));
        }
        let mut boundary = Vec::new();
        for k in 0..3 {
            let (a, b) = (c[k].clone(), c[(k + 1) % 3].clone());
            add(&mut residual, a.clone(), b.clone(), 1);
            let key = if a < b {
                (a.clone(), b.clone())
            } else {
                (b.clone(), a.clone())
            };
            let count = incidence.entry(key).or_default();
            *count += 1;
            if *count > 2 {
                return Err(fail("trim proposal edge incidence exceeds two"));
            }
            if let Some(h) = curves.get(&(a.clone(), b.clone())) {
                boundary.push(h.clone());
            } else if curves.contains_key(&(b.clone(), a.clone())) {
                return Err(fail("trim proposal reverses an authored boundary"));
            } else {
                boundary.push(vec![
                    std::array::from_fn(|k| if k == 3 { Q::one() } else { a[k].clone() }),
                    std::array::from_fn(|k| if k == 3 { Q::one() } else { b[k].clone() }),
                ]);
            }
        }
        charts.extend(disks(m, face, boundary, work)?);
    }
    if residual.values().any(|n| *n != 0) {
        return Err(fail("trim proposal boundary chain is false"));
    }
    // Each UV chart is a positive embedded disk. Its winding is 0 or1. Exact
    // boundary cancellation equates their sum to the original 0/1 region,
    // proving coverage and excluding all interior overlaps, including holes.
    Ok(charts)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn holed(curved: bool) -> Model {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let outer = m.faces[0].outer;
        let corners = [
            [0.375, 0.375],
            [0.375, 0.625],
            [0.625, 0.625],
            [0.625, 0.375],
        ];
        let mut coedges = m.loops[outer].coedges.clone();
        for i in 0..4 {
            let a = corners[i];
            let b = corners[(i + 1) % 4];
            coedges[i].pcurve = if curved {
                let edge = [b[0] - a[0], b[1] - a[1]];
                let mid = [
                    (a[0] + b[0]) / 2. - edge[1] / 8.,
                    (a[1] + b[1]) / 2. + edge[0] / 8.,
                ];
                crate::Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    weights: vec![1.; 3],
                    periodic: false,
                    control_points: vec![a.to_vec(), mid.to_vec(), b.to_vec()],
                }
            } else {
                crate::line(a.to_vec(), b.to_vec())
            };
        }
        let l = m.loops.len();
        m.loops.push(crate::Loop { coedges });
        m.faces[0].holes.push(l);
        m
    }
    #[test]
    fn internal_knots_split_exactly_and_discontinuities_refuse() {
        let mut c = crate::Curve {
            degree: 1,
            knots: vec![0., 0., 0.375, 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, 0.25], vec![1., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        };
        let spans = boundary_spans(&c, &mut Work::default()).unwrap();
        assert_eq!(spans.len(), 2);
        assert_eq!(sb::point(spans[0].last().unwrap()), sb::point(&spans[1][0]));
        assert_eq!(sb::point(&spans[1][0])[0], Q::new(1.into(), 2.into()));
        c.knots = vec![0., 0., 0.375, 0.375, 1., 1.];
        c.control_points.insert(2, vec![0.75, 0.5]);
        c.weights.push(1.);
        assert!(boundary_spans(&c, &mut Work::default()).is_err());
    }
    #[test]
    fn multispan_outer_wire_and_hole_keep_the_region_proof() {
        let mut m = holed(false);
        let loops = [m.faces[0].outer, m.faces[0].holes[0]];
        for l in loops {
            for e in &mut m.loops[l].coedges {
                let a = e.pcurve.control_points[0].clone();
                let b = e.pcurve.control_points.last().unwrap().clone();
                let mid = a.iter().zip(&b).map(|(a, b)| (a + b) / 2.).collect();
                e.pcurve = crate::Curve {
                    degree: 1,
                    knots: vec![0., 0., 0.375, 1., 1.],
                    control_points: vec![a, mid, b],
                    weights: vec![1.; 3],
                    periodic: false,
                };
            }
        }
        assert!(!charts(&m, 0, &mut Work::default()).unwrap().is_empty());
    }
    #[test]
    fn hole_is_not_filled_when_the_surface_is_split_into_knot_cells() {
        let mut m = holed(false);
        let s = &mut m.faces[0].surface;
        assert_eq!(s.degree_u, 1);
        let mid = s.control_points[0]
            .iter()
            .zip(&s.control_points[1])
            .map(|(a, b)| a.iter().zip(b).map(|(a, b)| (a + b) / 2.).collect())
            .collect();
        s.control_points.insert(1, mid);
        let mid = s.weights[0]
            .iter()
            .zip(&s.weights[1])
            .map(|(a, b)| (a + b) / 2.)
            .collect();
        s.weights.insert(1, mid);
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let half = Q::new(1.into(), 2.into());
        let hole = sb::at(s, &[half.clone(), half]).unwrap();
        let result = charts(&m, 0, &mut Work::default()).unwrap();
        assert!(result.len() > 8);
        for h in result {
            let p = bernstein::corners(&h);
            let normal = cross(&sub(&p[1], &p[0]), &sub(&p[2], &p[0]));
            assert!(!zero(&normal));
            let inside = (0..3).all(|k| {
                dot(
                    &normal,
                    &cross(&sub(&p[(k + 1) % 3], &p[k]), &sub(&hole, &p[k])),
                ) >= Q::zero()
            });
            assert!(!inside, "surface knot partition filled the original hole");
        }
    }
    #[test]
    fn curved_hole_is_retained_by_exact_disks() {
        let m = holed(true);
        let result = charts(&m, 0, &mut Work::default()).unwrap();
        assert_eq!(result.len(), 8);
        assert!(result.iter().any(|h| h.len() > 3));
        let mut bad = m.clone();
        let l = bad.faces[0].holes[0];
        for c in &mut bad.loops[l].coedges {
            for p in &mut c.pcurve.control_points {
                p[0] += 2.;
            }
        }
        assert!(charts(&bad, 0, &mut Work::default()).is_err());
    }
    #[test]
    fn filling_a_hole_is_not_a_certificate() {
        let m = holed(false);
        let f = &m.faces[0];
        let mut curves = BTreeMap::new();
        for l in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for e in &m.loops[l].coedges {
                let [a, b] = e.pcurve.domain();
                let h = sb::curve(
                    &e.pcurve,
                    &Q::from_float(a).unwrap(),
                    &Q::from_float(b).unwrap(),
                )
                .unwrap();
                curves.insert((sb::point(&h[0]), sb::point(h.last().unwrap())), h);
            }
        }
        let points = [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0]].map(|p| p.map(integer));
        assert!(certify(
            &m,
            0,
            &curves,
            &points,
            &[0, 1, 2, 0, 2, 3],
            &mut Work::default()
        )
        .is_err());
        assert!(certify(&m, 0, &curves, &points, &[0, 2, 1], &mut Work::default()).is_err());
        assert!(certify(&m, 0, &curves, &points, &[0, 1, 99], &mut Work::default()).is_err());
    }
}

fn open_segment(p: &P, a: &P, b: &P) -> bool {
    p != a
        && p != b
        && side(a, b, p).is_zero()
        && (0..2)
            .all(|k| &p[k] >= std::cmp::min(&a[k], &b[k]) && &p[k] <= std::cmp::max(&a[k], &b[k]))
}
fn crosses(a: &P, b: &P, u: &P, v: &P) -> bool {
    let (x, y, z, w) = (side(a, b, u), side(a, b, v), side(u, v, a), side(u, v, b));
    (x * y < Q::zero() && z * w < Q::zero())
        || open_segment(u, a, b)
        || open_segment(v, a, b)
        || open_segment(a, u, v)
        || open_segment(b, u, v)
}
/// Deterministic exact advancing-front proposal, without coordinate rounding,
/// square roots, tolerance tests or platform-dependent floating tie breaks.
/// Its output is still untrusted: certify() checks every disk and boundary.
fn proposal(rings: &[Vec<P>], work: &mut Work) -> Result<(Vec<P>, Vec<u32>)> {
    let positions = rings
        .iter()
        .flatten()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let ids = positions
        .iter()
        .cloned()
        .enumerate()
        .map(|(i, p)| (p, i as u32))
        .collect::<BTreeMap<_, _>>();
    let mut front = BTreeSet::<Edge>::new();
    for ring in rings {
        for (a, b) in ring
            .iter()
            .zip(ring.iter().cycle().skip(1))
            .take(ring.len())
        {
            front.insert((a.clone(), b.clone()));
        }
    }
    let mut out = Vec::new();
    while let Some((a, b)) = front.first().cloned() {
        if out.len() >= 3072 {
            return Err(fail("exact trim proposal triangle budget exceeded"));
        }
        let active = front
            .iter()
            .flat_map(|(a, b)| [a.clone(), b.clone()])
            .collect::<BTreeSet<_>>();
        let mut candidate = None;
        'vertex: for c in &active {
            work.charge(1)?;
            if side(&a, &b, c) <= Q::zero() {
                continue;
            }
            // Reject covering any unresolved boundary vertex, including a
            // T-junction on a proposed triangle edge.
            for p in &active {
                work.charge(1)?;
                if p != &a
                    && p != &b
                    && p != c
                    && side(&a, &b, p) >= Q::zero()
                    && side(&b, c, p) >= Q::zero()
                    && side(c, &a, p) >= Q::zero()
                {
                    continue 'vertex;
                }
            }
            for (x, y) in [(&a, &b), (&b, c), (c, &a)] {
                if front.contains(&(y.clone(), x.clone())) {
                    continue 'vertex;
                }
                for (u, v) in &front {
                    work.charge(1)?;
                    if (x == u && y == v) || (x == v && y == u) {
                        continue;
                    }
                    if crosses(x, y, u, v) {
                        continue 'vertex;
                    }
                }
            }
            let center = std::array::from_fn(|k| (&a[k] + &b[k] + &c[k]) / integer(3));
            let mut winding = 0;
            for (u, v) in &front {
                work.charge(1)?;
                let s = side(u, v, &center);
                if s.is_zero()
                    && (0..2).all(|k| {
                        &center[k] >= std::cmp::min(&u[k], &v[k])
                            && &center[k] <= std::cmp::max(&u[k], &v[k])
                    })
                {
                    continue 'vertex;
                }
                if u[1] <= center[1] && center[1] < v[1] && s > Q::zero() {
                    winding += 1;
                }
                if v[1] <= center[1] && center[1] < u[1] && s < Q::zero() {
                    winding -= 1;
                }
            }
            if winding == 1 {
                candidate = Some(c.clone());
                break;
            }
        }
        let c = candidate
            .ok_or_else(|| fail("exact trim proposal has no admissible front triangle"))?;
        out.extend([ids[&a], ids[&b], ids[&c]]);
        front.remove(&(a.clone(), b.clone()));
        for (u, v) in [(a, c.clone()), (c, b)] {
            if !front.remove(&(v.clone(), u.clone())) && !front.insert((u, v)) {
                return Err(fail("exact trim proposal repeats a front edge"));
            }
        }
    }
    Ok((positions, out))
}
