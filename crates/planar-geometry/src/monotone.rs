//! Polygon-with-holes triangulation by y-monotone decomposition.
//!
//! The sweep follows de Berg et al., "Computational Geometry", ch. 3: a
//! top-to-bottom sweep inserts helper diagonals at split and merge vertices,
//! the resulting y-monotone faces are triangulated with the linear stack
//! algorithm, and Lawson flips then make the result constrained Delaunay so
//! sampled straight edges do not leave slivers.
//!
//! Every decision uses exact orientation (a binary64 filter with an exact
//! dyadic fallback); no coordinate is nudged and every output position is an
//! input coordinate. Degenerate input is handled deterministically:
//! consecutive duplicate vertices and zero-width spikes are dropped,
//! collinear boundary vertices are kept, rings that touch at a shared vertex
//! are separated by a symbolic perturbation of each occurrence into its own
//! interior sector, and crossings, overlapping edges, T-junctions, holes
//! outside the outer ring and nested holes are refused.
use crate::tessellation::FillMesh;
use crate::{Result, check, error};
use rustc_hash::FxHashMap;

pub(crate) const MAX_VERTICES: usize = 4096;
pub(crate) const MAX_WORK: usize = 64_000_000;

type P = [f64; 2];

fn sub(a: P, b: P) -> P {
    [a[0] - b[0], a[1] - b[1]]
}
fn cross(a: P, b: P) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn sign(v: f64) -> i8 {
    (v > 0.) as i8 - (v < 0.) as i8
}

/// Exact sign of the orientation determinant of three binary64 points.
pub(crate) fn orient_exact(a: P, b: P, c: P) -> i8 {
    let left = (a[0] - c[0]) * (b[1] - c[1]);
    let right = (a[1] - c[1]) * (b[0] - c[0]);
    let det = left - right;
    let sum = left.abs() + right.abs();
    // Shewchuk's ccwerrboundA, applied only when no intermediate can be
    // subnormal or overflowing; everything else is decided exactly.
    let bound = (3.0 + 16.0 * f64::EPSILON) * f64::EPSILON * sum;
    if sum.is_finite() && (sum == 0. || sum > 1e-280) && det.abs() > bound {
        return sign(det);
    }
    if sum == 0. && left == 0. && right == 0. {
        // Exact zero products may still hide underflow; fall through.
        let all_regular = [a, b, c]
            .iter()
            .flatten()
            .all(|v| *v == 0. || v.abs() > 1e-150);
        if all_regular {
            return 0;
        }
    }
    match crate::triangulation::exact_binary64_orientation(a, b, c) {
        cad_predicates::Sign::Negative => -1,
        cad_predicates::Sign::Zero => 0,
        cad_predicates::Sign::Positive => 1,
    }
}

/// True only when the binary64 filter certifies that `d` lies strictly inside
/// the circle through CCW `abc`. Ambiguous (including cocircular) cases
/// answer false: flips are a quality pass, and flipping only on certified
/// positives keeps Lawson's termination argument.
fn certainly_incircle(a: P, b: P, c: P, d: P) -> bool {
    let (adx, ady) = (a[0] - d[0], a[1] - d[1]);
    let (bdx, bdy) = (b[0] - d[0], b[1] - d[1]);
    let (cdx, cdy) = (c[0] - d[0], c[1] - d[1]);
    let alift = adx * adx + ady * ady;
    let blift = bdx * bdx + bdy * bdy;
    let clift = cdx * cdx + cdy * cdy;
    let det = alift * (bdx * cdy - bdy * cdx)
        + blift * (cdx * ady - cdy * adx)
        + clift * (adx * bdy - ady * bdx);
    let permanent = alift * ((bdx * cdy).abs() + (bdy * cdx).abs())
        + blift * ((cdx * ady).abs() + (cdy * adx).abs())
        + clift * ((adx * bdy).abs() + (ady * bdx).abs());
    let bound = (10.0 + 96.0 * f64::EPSILON) * f64::EPSILON * permanent;
    permanent.is_finite() && permanent > 1e-250 && det > bound
}

struct Work(usize);
impl Work {
    fn charge(&mut self, amount: usize) -> Result<()> {
        self.0 = self.0.saturating_add(amount);
        check(
            self.0 <= MAX_WORK,
            "Profile triangulation work budget exceeded",
        )
    }
}

fn key(p: P) -> [u64; 2] {
    p.map(|v| if v == 0. { 0 } else { v.to_bits() })
}

/// Removes consecutive duplicates and zero-width spikes; keeps collinear
/// straight-through vertices.
fn clean_ring(ring: &[P]) -> Vec<P> {
    let mut out: Vec<P> = Vec::with_capacity(ring.len());
    for &p in ring {
        out.push(p);
        loop {
            let n = out.len();
            if n >= 2 && out[n - 1] == out[n - 2] {
                out.pop();
                continue;
            }
            if n >= 3 && is_spike(out[n - 3], out[n - 2], out[n - 1]) {
                out.remove(n - 2);
                continue;
            }
            break;
        }
    }
    // Cyclic closure.
    loop {
        let n = out.len();
        if n < 3 {
            break;
        }
        if out[0] == out[n - 1] || is_spike(out[n - 2], out[n - 1], out[0]) {
            out.pop();
        } else if is_spike(out[n - 1], out[0], out[1]) {
            out.remove(0);
        } else {
            break;
        }
    }
    out
}

fn is_spike(a: P, b: P, c: P) -> bool {
    a == c
        || (orient_exact(a, b, c) == 0 && {
            let (u, v) = (sub(a, b), sub(c, b));
            u[0] * v[0] + u[1] * v[1] > 0.
        })
}

fn shoelace(ring: &[P]) -> f64 {
    let o = ring[0];
    (1..ring.len() - 1)
        .map(|i| cross(sub(ring[i], o), sub(ring[i + 1], o)))
        .sum::<f64>()
        / 2.
}

struct Poly {
    pos: Vec<P>,
    /// Symbolic perturbation direction; zero for vertices at unique positions.
    dir: Vec<P>,
    next: Vec<u32>,
    prev: Vec<u32>,
}

impl Poly {
    fn p(&self, i: u32) -> P {
        self.pos[i as usize]
    }

    /// Orientation of three vertex occurrences, with coincident occurrences
    /// moved infinitesimally into their own interior sectors.
    fn orient(&self, i: u32, j: u32, k: u32) -> i8 {
        let (a, b, c) = (self.p(i), self.p(j), self.p(k));
        let s = orient_exact(a, b, c);
        if s != 0 || !(a == b || b == c || a == c) {
            return s;
        }
        let (da, db, dc) = (
            self.dir[i as usize],
            self.dir[j as usize],
            self.dir[k as usize],
        );
        let first = cross(sub(b, a), sub(dc, da)) + cross(sub(db, da), sub(c, a));
        if first != 0. {
            return sign(first);
        }
        sign(cross(sub(db, da), sub(dc, da)))
    }

    /// Sweep order: higher y first, then smaller x, then perturbation, then id.
    fn above(&self, i: u32, j: u32) -> bool {
        let (a, b) = (self.p(i), self.p(j));
        if a[1] != b[1] {
            return a[1] > b[1];
        }
        if a[0] != b[0] {
            return a[0] < b[0];
        }
        let (da, db) = (self.dir[i as usize], self.dir[j as usize]);
        if da[1] != db[1] {
            return da[1] > db[1];
        }
        if da[0] != db[0] {
            return da[0] < db[0];
        }
        i < j
    }
}

/// Triangulates `rings[0]` minus `rings[1..]`. Positions are the cleaned ring
/// vertices in input order; indices form CCW triangles.
pub(crate) fn triangulate(outer: &[P], holes: &[Vec<P>]) -> Result<FillMesh> {
    check(
        outer.len() >= 3 && holes.iter().all(|h| h.len() >= 3),
        "Profile rings need at least three vertices",
    )?;
    let vertex_count = outer.len() + holes.iter().map(Vec::len).sum::<usize>();
    check(
        vertex_count <= MAX_VERTICES,
        "Profile triangulation budget exceeded",
    )?;
    check(
        outer
            .iter()
            .chain(holes.iter().flatten())
            .flatten()
            .all(|v| v.is_finite()),
        "Non-finite profile coordinate",
    )?;
    let mut work = Work(0);
    let mut pos = Vec::with_capacity(vertex_count);
    let mut next = Vec::with_capacity(vertex_count);
    let mut prev = Vec::with_capacity(vertex_count);
    let mut ring_start = Vec::with_capacity(holes.len() + 1);
    let mut expected_area = 0.;
    for (index, ring) in std::iter::once(outer)
        .chain(holes.iter().map(Vec::as_slice))
        .enumerate()
    {
        let mut ring = clean_ring(ring);
        check(ring.len() >= 3, "Degenerate profile ring")?;
        let area = shoelace(&ring);
        check(area != 0. && area.is_finite(), "Degenerate profile ring")?;
        if (area > 0.) != (index == 0) {
            ring.reverse();
        }
        expected_area += if index == 0 { area.abs() } else { -area.abs() };
        let base = pos.len() as u32;
        let n = ring.len() as u32;
        ring_start.push(base);
        for (k, p) in ring.into_iter().enumerate() {
            let k = k as u32;
            pos.push(p);
            next.push(base + (k + 1) % n);
            prev.push(base + (k + n - 1) % n);
        }
    }
    ring_start.push(pos.len() as u32);
    check(expected_area > 0., "Holes cover the profile")?;
    let n = pos.len();

    // Coincident occurrences (rings touching at a vertex).
    let mut groups: FxHashMap<[u64; 2], Vec<u32>> = FxHashMap::default();
    for (i, &p) in pos.iter().enumerate() {
        groups.entry(key(p)).or_default().push(i as u32);
    }
    let mut clusters: Vec<Vec<u32>> = groups.into_values().filter(|m| m.len() > 1).collect();
    clusters.sort_unstable();
    let mut clustered = vec![false; n];
    for &v in clusters.iter().flatten() {
        clustered[v as usize] = true;
    }
    let mut poly = Poly {
        pos,
        dir: vec![[0.; 2]; n],
        next,
        prev,
    };
    validate_edges(&poly, &mut work)?;
    validate_containment(&poly, &ring_start, &clustered, &mut work)?;
    for members in &clusters {
        relink_pinch(&mut poly, members)?;
    }
    for &v in clusters.iter().flatten() {
        let p = poly.p(v);
        let unit = |d: P| {
            let l = d[0].hypot(d[1]);
            [d[0] / l, d[1] / l]
        };
        let a = unit(sub(p, poly.p(poly.prev[v as usize])));
        let b = unit(sub(poly.p(poly.next[v as usize]), p));
        // Sum of left normals points into the material wedge.
        poly.dir[v as usize] = [-a[1] - b[1], a[0] + b[0]];
    }

    let diagonals = monotone_diagonals(&poly, &mut work)?;
    let mut triangles = triangulate_faces(&poly, &diagonals, &mut work)?;
    let mut adjacency = Adjacency::new(&triangles);
    repair_degenerate(&poly, &mut triangles, &mut adjacency, &mut work)?;
    delaunay_flips(&poly, &mut triangles, &mut adjacency, &mut work);

    // A triangle joining two occurrences of one touching point only has
    // perturbed area; dropping it leaves the pinch conforming by position.
    triangles.retain(|t| {
        let [a, b, c] = t.map(|i| poly.p(i));
        a != b && b != c && a != c
    });
    let mut area = 0.;
    for t in &triangles {
        let [a, b, c] = t.map(|i| poly.p(i));
        check(
            poly.orient(t[0], t[1], t[2]) > 0 && orient_exact(a, b, c) > 0,
            "Profile triangulation produced a degenerate triangle",
        )?;
        area += cross(sub(b, a), sub(c, a)) / 2.;
    }
    check_conforming(&poly, &triangles)?;
    check(
        (area - expected_area).abs() <= expected_area.abs() * 1e-9,
        "Profile triangulation does not conserve area",
    )?;
    Ok(FillMesh {
        positions: poly.pos,
        indices: triangles.into_iter().flatten().collect(),
    })
}

/// Re-pairs the edges meeting at a touching point so that every occurrence
/// owns exactly one material wedge: rays sorted CCW must alternate outgoing,
/// incoming; each outgoing ray is joined to the next incoming ray. A
/// non-alternating fan means the rings overlap at that point.
fn relink_pinch(poly: &mut Poly, members: &[u32]) -> Result<()> {
    let q = poly.p(members[0]);
    // (target or source vertex, outgoing)
    let mut rays: Vec<(u32, bool)> = members
        .iter()
        .flat_map(|&v| {
            [
                (poly.next[v as usize], true),
                (poly.prev[v as usize], false),
            ]
        })
        .collect();
    let half = |w: P| {
        let d = sub(w, q);
        if d[1] > 0. || (d[1] == 0. && d[0] > 0.) {
            0
        } else {
            1
        }
    };
    let pos = &poly.pos;
    rays.sort_by(|x, y| {
        let (a, b) = (pos[x.0 as usize], pos[y.0 as usize]);
        half(a)
            .cmp(&half(b))
            .then_with(|| match orient_exact(q, a, b) {
                1 => std::cmp::Ordering::Less,
                -1 => std::cmp::Ordering::Greater,
                _ => x.cmp(y),
            })
    });
    let first_out = rays
        .iter()
        .position(|r| r.1)
        .ok_or_else(|| error("Profile rings overlap"))?;
    rays.rotate_left(first_out);
    for (k, pair) in rays.chunks(2).enumerate() {
        check(
            pair[0].1 && !pair[1].1,
            "Profile rings overlap at a shared vertex",
        )?;
        let v = members[k];
        let (target, source) = (pair[0].0, pair[1].0);
        poly.next[v as usize] = target;
        poly.prev[target as usize] = v;
        poly.prev[v as usize] = source;
        poly.next[source as usize] = v;
    }
    Ok(())
}

/// Every boundary edge is used once in its direction, every other edge is
/// used once in each direction (compared by coordinates across pinches).
fn check_conforming(poly: &Poly, triangles: &[[u32; 3]]) -> Result<()> {
    let mut uses: FxHashMap<([u64; 2], [u64; 2]), u8> = FxHashMap::default();
    for t in triangles {
        for k in 0..3 {
            let e = (key(poly.p(t[k])), key(poly.p(t[(k + 1) % 3])));
            let count = uses.entry(e).or_default();
            *count += 1;
            check(*count == 1, "Profile triangulation overlaps itself")?;
        }
    }
    for v in 0..poly.pos.len() as u32 {
        let e = (key(poly.p(v)), key(poly.p(poly.next[v as usize])));
        check(
            uses.remove(&e).is_some(),
            "Profile triangulation lost a boundary edge",
        )?;
    }
    for &(a, b) in uses.keys() {
        check(
            uses.contains_key(&(b, a)),
            "Profile triangulation lost a face",
        )?;
    }
    Ok(())
}

/// Refuses crossings, overlapping edges and vertices inside other edges.
fn validate_edges(poly: &Poly, work: &mut Work) -> Result<()> {
    let n = poly.pos.len() as u32;
    let mut edges: Vec<(f64, f64, u32)> = (0..n)
        .map(|i| {
            let (a, b) = (poly.p(i), poly.p(poly.next[i as usize]));
            (a[0].min(b[0]), a[0].max(b[0]), i)
        })
        .collect();
    edges.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.2.cmp(&y.2)));
    let mut active: Vec<(f64, u32)> = Vec::new();
    let strictly_inside = |a: P, b: P, c: P| {
        c != a
            && c != b
            && orient_exact(a, b, c) == 0
            && (c[0] - a[0]) * (c[0] - b[0]) <= 0.
            && (c[1] - a[1]) * (c[1] - b[1]) <= 0.
    };
    for &(min_x, max_x, i) in &edges {
        active.retain(|&(end, _)| end >= min_x);
        let (a, b) = (poly.p(i), poly.p(poly.next[i as usize]));
        let (ylo, yhi) = (a[1].min(b[1]), a[1].max(b[1]));
        work.charge(active.len() + 1)?;
        for &(_, j) in &active {
            let (c, d) = (poly.p(j), poly.p(poly.next[j as usize]));
            if c[1].max(d[1]) < ylo || c[1].min(d[1]) > yhi {
                continue;
            }
            check(
                !((a == c && b == d) || (a == d && b == c)),
                "Profile has overlapping edges",
            )?;
            if strictly_inside(a, b, c)
                || strictly_inside(a, b, d)
                || strictly_inside(c, d, a)
                || strictly_inside(c, d, b)
            {
                return Err(error("Profile has touching or overlapping edges"));
            }
            let (o1, o2) = (orient_exact(a, b, c), orient_exact(a, b, d));
            let (o3, o4) = (orient_exact(c, d, a), orient_exact(c, d, b));
            check(
                !(o1 * o2 < 0 && o3 * o4 < 0),
                "Profile boundary self-intersects",
            )?;
        }
        active.push((max_x, i));
    }
    Ok(())
}

fn winding(poly: &Poly, start: u32, end: u32, q: P) -> i32 {
    let mut w = 0;
    for i in start..end {
        let (a, b) = (poly.p(i), poly.p(poly.next[i as usize]));
        if a[1] <= q[1] {
            if b[1] > q[1] && orient_exact(a, b, q) > 0 {
                w += 1;
            }
        } else if b[1] <= q[1] && orient_exact(a, b, q) < 0 {
            w -= 1;
        }
    }
    w
}

/// Every hole must lie inside the outer ring and outside every other hole.
fn validate_containment(
    poly: &Poly,
    ring_start: &[u32],
    clustered: &[bool],
    work: &mut Work,
) -> Result<()> {
    let rings = ring_start.len() - 1;
    // Bounding boxes reject most pairs before a winding test.
    let boxes: Vec<[f64; 4]> = (0..rings)
        .map(|r| {
            (ring_start[r]..ring_start[r + 1]).fold(
                [f64::INFINITY, f64::INFINITY, -f64::INFINITY, -f64::INFINITY],
                |b, i| {
                    let p = poly.p(i);
                    [
                        b[0].min(p[0]),
                        b[1].min(p[1]),
                        b[2].max(p[0]),
                        b[3].max(p[1]),
                    ]
                },
            )
        })
        .collect();
    for hole in 1..rings {
        // Any vertex not shared with another ring decides containment, because
        // boundaries neither cross nor overlap.
        let Some(sample) =
            (ring_start[hole]..ring_start[hole + 1]).find(|&i| !clustered[i as usize])
        else {
            continue;
        };
        let q = poly.p(sample);
        for other in 0..rings {
            if other == hole {
                continue;
            }
            let b = boxes[other];
            let in_box = q[0] >= b[0] && q[0] <= b[2] && q[1] >= b[1] && q[1] <= b[3];
            if !in_box {
                check(other != 0, "Profile hole lies outside the outer ring")?;
                continue;
            }
            work.charge((ring_start[other + 1] - ring_start[other]) as usize)?;
            let w = winding(poly, ring_start[other], ring_start[other + 1], q);
            if other == 0 {
                check(w != 0, "Profile hole lies outside the outer ring")?;
            } else {
                check(w == 0, "Profile holes are nested")?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Start,
    End,
    Split,
    Merge,
    Regular,
}

/// Sweep that returns diagonals splitting the polygon into y-monotone faces.
fn monotone_diagonals(poly: &Poly, work: &mut Work) -> Result<Vec<[u32; 2]>> {
    let n = poly.pos.len() as u32;
    let mut order: Vec<u32> = (0..n).collect();
    order.sort_by(|&a, &b| {
        if a == b {
            std::cmp::Ordering::Equal
        } else if poly.above(a, b) {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });
    let mut kind = vec![Kind::Regular; n as usize];
    for v in 0..n {
        let (p, q) = (poly.prev[v as usize], poly.next[v as usize]);
        let (pb, qb) = (poly.above(v, p), poly.above(v, q));
        let turn = poly.orient(p, v, q);
        kind[v as usize] = match (pb, qb) {
            (true, true) => match turn {
                1 => Kind::Start,
                -1 => Kind::Split,
                _ => return Err(error("Degenerate profile vertex")),
            },
            (false, false) => match turn {
                1 => Kind::End,
                -1 => Kind::Merge,
                _ => return Err(error("Degenerate profile vertex")),
            },
            _ => Kind::Regular,
        };
    }
    // Status: edges (by start vertex, directed downward) with interior to
    // their right, ordered left to right along the sweep line.
    let mut status: Vec<u32> = Vec::new();
    let mut helper = vec![u32::MAX; n as usize];
    let mut diagonals = Vec::new();
    // Edge e = (e, next[e]) goes downward; v is to its right when orient > 0.
    let right_of = |e: u32, v: u32| -> Result<bool> {
        match poly.orient(e, poly.next[e as usize], v) {
            0 => Err(error("Profile has touching or overlapping edges")),
            s => Ok(s > 0),
        }
    };
    let locate = |status: &Vec<u32>, v: u32, work: &mut Work| -> Result<usize> {
        work.charge(2 + status.len().max(1).ilog2() as usize)?;
        let (mut lo, mut hi) = (0, status.len());
        while lo < hi {
            let mid = (lo + hi) / 2;
            if right_of(status[mid], v)? {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        Ok(lo)
    };
    let left_edge = |status: &Vec<u32>, v: u32, work: &mut Work| -> Result<u32> {
        let at = locate(status, v, work)?;
        check(at > 0, "Profile sweep lost its left boundary")?;
        Ok(status[at - 1])
    };
    let remove = |status: &mut Vec<u32>, e: u32, work: &mut Work| -> Result<()> {
        work.charge(status.len() / 8 + 1)?;
        let at = status
            .iter()
            .position(|&x| x == e)
            .ok_or_else(|| error("Profile sweep lost an edge"))?;
        status.remove(at);
        Ok(())
    };
    let insert = |status: &mut Vec<u32>, e: u32, v: u32, work: &mut Work| -> Result<()> {
        let at = locate(status, v, work)?;
        work.charge(status.len() / 8 + 1)?;
        status.insert(at, e);
        Ok(())
    };
    let is_merge = |h: u32, kind: &Vec<Kind>| h != u32::MAX && kind[h as usize] == Kind::Merge;
    for &v in &order {
        let e_prev = poly.prev[v as usize];
        match kind[v as usize] {
            Kind::Start => {
                insert(&mut status, v, v, work)?;
                helper[v as usize] = v;
            }
            Kind::End => {
                let h = helper[e_prev as usize];
                if is_merge(h, &kind) {
                    diagonals.push([v, h]);
                }
                remove(&mut status, e_prev, work)?;
            }
            Kind::Split => {
                let ej = left_edge(&status, v, work)?;
                diagonals.push([v, helper[ej as usize]]);
                helper[ej as usize] = v;
                insert(&mut status, v, v, work)?;
                helper[v as usize] = v;
            }
            Kind::Merge => {
                let h = helper[e_prev as usize];
                if is_merge(h, &kind) {
                    diagonals.push([v, h]);
                }
                remove(&mut status, e_prev, work)?;
                let ej = left_edge(&status, v, work)?;
                if is_merge(helper[ej as usize], &kind) {
                    diagonals.push([v, helper[ej as usize]]);
                }
                helper[ej as usize] = v;
            }
            Kind::Regular => {
                if poly.above(e_prev, v) {
                    // Interior lies to the right of v.
                    let h = helper[e_prev as usize];
                    if is_merge(h, &kind) {
                        diagonals.push([v, h]);
                    }
                    remove(&mut status, e_prev, work)?;
                    insert(&mut status, v, v, work)?;
                    helper[v as usize] = v;
                } else {
                    let ej = left_edge(&status, v, work)?;
                    if is_merge(helper[ej as usize], &kind) {
                        diagonals.push([v, helper[ej as usize]]);
                    }
                    helper[ej as usize] = v;
                }
            }
        }
    }
    check(status.is_empty(), "Profile sweep did not close")?;
    Ok(diagonals)
}

/// Extracts the monotone faces and triangulates each one.
fn triangulate_faces(
    poly: &Poly,
    diagonals: &[[u32; 2]],
    work: &mut Work,
) -> Result<Vec<[u32; 3]>> {
    let n = poly.pos.len();
    // Outgoing interior neighbours of each vertex in CCW order, starting with
    // `next` and ending with `prev`.
    let mut extra: Vec<Vec<u32>> = vec![Vec::new(); n];
    for &[a, b] in diagonals {
        check(a != b, "Profile sweep produced an empty diagonal")?;
        extra[a as usize].push(b);
        extra[b as usize].push(a);
    }
    let mut fans: Vec<Vec<u32>> = Vec::with_capacity(n);
    for v in 0..n as u32 {
        let (first, last) = (poly.next[v as usize], poly.prev[v as usize]);
        let mut list = std::mem::take(&mut extra[v as usize]);
        if !list.is_empty() {
            work.charge(list.len() * 4)?;
            let half = |w: u32| if poly.orient(v, first, w) > 0 { 0 } else { 1 };
            list.sort_by(|&a, &b| {
                half(a).cmp(&half(b)).then_with(|| {
                    if a == b {
                        std::cmp::Ordering::Equal
                    } else if poly.orient(v, a, b) > 0 {
                        std::cmp::Ordering::Less
                    } else {
                        std::cmp::Ordering::Greater
                    }
                })
            });
            list.dedup();
        }
        let mut fan = Vec::with_capacity(list.len() + 2);
        fan.push(first);
        fan.extend(list);
        fan.push(last);
        fans.push(fan);
    }
    // Half-edge (u -> v) is followed by (v -> w), w preceding u in v's fan.
    let mut visited: FxHashMap<(u32, u32), ()> = FxHashMap::default();
    let mut triangles = Vec::with_capacity(n + 2);
    let mut face = Vec::new();
    for start in 0..n as u32 {
        for &first in &fans[start as usize][..fans[start as usize].len() - 1] {
            if visited.contains_key(&(start, first)) {
                continue;
            }
            face.clear();
            let (mut u, mut v) = (start, first);
            loop {
                work.charge(1)?;
                check(
                    visited.insert((u, v), ()).is_none(),
                    "Profile face walk revisited an edge",
                )?;
                face.push(u);
                let fan = &fans[v as usize];
                let k = fan
                    .iter()
                    .position(|&x| x == u)
                    .ok_or_else(|| error("Profile face walk lost an edge"))?;
                check(k > 0, "Profile face walk left the interior")?;
                let w = fan[k - 1];
                (u, v) = (v, w);
                if (u, v) == (start, first) {
                    break;
                }
                check(face.len() <= n, "Profile face walk did not close")?;
            }
            triangulate_monotone(poly, &face, &mut triangles, work)?;
        }
    }
    Ok(triangles)
}

/// Linear-time triangulation of one y-monotone CCW face.
fn triangulate_monotone(
    poly: &Poly,
    face: &[u32],
    out: &mut Vec<[u32; 3]>,
    work: &mut Work,
) -> Result<()> {
    let m = face.len();
    check(m >= 3, "Profile face has fewer than three vertices")?;
    work.charge(m * 4)?;
    if m == 3 {
        out.push([face[0], face[1], face[2]]);
        return Ok(());
    }
    let top = (0..m)
        .min_by(|&a, &b| {
            if poly.above(face[a], face[b]) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        })
        .unwrap();
    let bottom = (0..m)
        .min_by(|&a, &b| {
            if poly.above(face[b], face[a]) {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Greater
            }
        })
        .unwrap();
    // Walking CCW from the top descends the left chain.
    let mut left = FxHashMap::default();
    let mut sorted = Vec::with_capacity(m);
    sorted.push(face[top]);
    let (mut i, mut j) = ((top + 1) % m, (top + m - 1) % m);
    // Merge the two descending chains.
    while sorted.len() < m {
        let take_left = if i == bottom && j != bottom {
            false
        } else if j == bottom && i != bottom {
            true
        } else if i == bottom && j == bottom {
            sorted.push(face[bottom]);
            break;
        } else {
            poly.above(face[i], face[j])
        };
        if take_left {
            left.insert(face[i], true);
            sorted.push(face[i]);
            i = (i + 1) % m;
        } else {
            left.insert(face[j], false);
            sorted.push(face[j]);
            j = (j + m - 1) % m;
        }
    }
    check(sorted.len() == m, "Profile face is not monotone")?;
    let is_left = |v: u32| left.get(&v).copied();
    let emit = |out: &mut Vec<[u32; 3]>, a: u32, b: u32, c: u32, fallback_ccw: bool| {
        let s = poly.orient(a, b, c);
        if s > 0 || (s == 0 && fallback_ccw) {
            out.push([a, b, c]);
        } else {
            out.push([a, c, b]);
        }
    };
    let mut stack = vec![sorted[0], sorted[1]];
    for &u in &sorted[2..m - 1] {
        let top_left = is_left(*stack.last().unwrap());
        let u_left = is_left(u);
        if u_left != top_left {
            // Opposite chains: fan to every stack vertex.
            let pending = std::mem::take(&mut stack);
            let stack_left = top_left.unwrap_or(!u_left.unwrap_or(true));
            for w in pending.windows(2) {
                let (hi, lo) = (w[0], w[1]);
                // Left chain u sees the right chain as (u, lo, hi).
                if stack_left {
                    emit(out, u, hi, lo, true);
                } else {
                    emit(out, u, lo, hi, true);
                }
            }
            stack.push(*pending.last().unwrap());
            stack.push(u);
        } else {
            let mut last = stack.pop().unwrap();
            let u_is_left = u_left.unwrap_or(false);
            while let Some(&t) = stack.last() {
                let ok = if u_is_left {
                    poly.orient(u, t, last) > 0
                } else {
                    poly.orient(u, last, t) > 0
                };
                if !ok {
                    break;
                }
                stack.pop();
                if u_is_left {
                    out.push([u, t, last]);
                } else {
                    out.push([u, last, t]);
                }
                last = t;
            }
            stack.push(last);
            stack.push(u);
        }
    }
    let u = sorted[m - 1];
    let stack_left = is_left(*stack.last().unwrap()).unwrap_or(true);
    for w in stack.windows(2) {
        let (hi, lo) = (w[0], w[1]);
        if stack_left {
            emit(out, u, hi, lo, true);
        } else {
            emit(out, u, lo, hi, true);
        }
    }
    Ok(())
}

struct Adjacency(FxHashMap<(u32, u32), usize>);

impl Adjacency {
    fn new(triangles: &[[u32; 3]]) -> Self {
        let mut map = FxHashMap::default();
        for (t, tri) in triangles.iter().enumerate() {
            for k in 0..3 {
                map.insert((tri[k], tri[(k + 1) % 3]), t);
            }
        }
        Self(map)
    }
    fn set(&mut self, t: usize, tri: [u32; 3]) {
        for k in 0..3 {
            self.0.insert((tri[k], tri[(k + 1) % 3]), t);
        }
    }
    fn unset(&mut self, tri: [u32; 3]) {
        for k in 0..3 {
            self.0.remove(&(tri[k], tri[(k + 1) % 3]));
        }
    }
}

/// Replaces (a, b, c) and (b, a, d) by (c, a, d) and (c, d, b).
#[allow(clippy::too_many_arguments)]
fn flip(
    triangles: &mut [[u32; 3]],
    adjacency: &mut Adjacency,
    t1: usize,
    t2: usize,
    a: u32,
    b: u32,
    c: u32,
    d: u32,
) {
    adjacency.unset(triangles[t1]);
    adjacency.unset(triangles[t2]);
    triangles[t1] = [c, a, d];
    triangles[t2] = [c, d, b];
    adjacency.set(t1, triangles[t1]);
    adjacency.set(t2, triangles[t2]);
}

fn rotate_to(tri: [u32; 3], a: u32) -> [u32; 3] {
    match tri.iter().position(|&x| x == a) {
        Some(0) | None => tri,
        Some(1) => [tri[1], tri[2], tri[0]],
        _ => [tri[2], tri[0], tri[1]],
    }
}

/// Removes zero-area triangles produced by collinear chains: the middle
/// vertex lies on the opposite edge, which is flipped.
fn repair_degenerate(
    poly: &Poly,
    triangles: &mut [[u32; 3]],
    adjacency: &mut Adjacency,
    work: &mut Work,
) -> Result<()> {
    let mut pending: Vec<usize> = (0..triangles.len())
        .filter(|&t| {
            let [a, b, c] = triangles[t];
            poly.orient(a, b, c) == 0
        })
        .collect();
    while let Some(t) = pending.pop() {
        work.charge(8)?;
        let [a, b, c] = triangles[t];
        if poly.orient(a, b, c) != 0 {
            continue;
        }
        // The vertex strictly between the other two along their line.
        let between = |m: u32, x: u32, y: u32| {
            let (pm, px, py) = (poly.p(m), poly.p(x), poly.p(y));
            (pm[0] - px[0]) * (pm[0] - py[0]) + (pm[1] - px[1]) * (pm[1] - py[1]) < 0.
        };
        let (m, x, y) = if between(a, b, c) {
            (a, b, c)
        } else if between(b, c, a) {
            (b, c, a)
        } else {
            (c, a, b)
        };
        // Triangle is (x, y, m) cyclically; flip edge (x, y).
        let Some(&t2) = adjacency.0.get(&(y, x)) else {
            return Err(error(
                "Profile triangulation produced a degenerate triangle",
            ));
        };
        let z = rotate_to(triangles[t2], y)[2];
        // flip of (x, y, m) and (y, x, z) gives (m, x, z) and (m, z, y).
        flip(triangles, adjacency, t, t2, x, y, m, z);
        for tt in [t, t2] {
            let [p, q, r] = triangles[tt];
            if poly.orient(p, q, r) == 0 {
                pending.push(tt);
            }
        }
    }
    Ok(())
}

/// Lawson flips toward the constrained Delaunay triangulation. Quality only:
/// stops silently when the work budget is spent.
fn delaunay_flips(
    poly: &Poly,
    triangles: &mut [[u32; 3]],
    adjacency: &mut Adjacency,
    work: &mut Work,
) {
    let mut stack: Vec<(u32, u32)> = adjacency
        .0
        .keys()
        .filter(|&&(a, b)| a < b && adjacency.0.contains_key(&(b, a)))
        .copied()
        .collect();
    stack.sort_unstable();
    let budget = 64 * triangles.len() + 1024;
    let mut flips = 0;
    while let Some((a, b)) = stack.pop() {
        if flips > budget || work.charge(4).is_err() {
            return;
        }
        let (Some(&t1), Some(&t2)) = (adjacency.0.get(&(a, b)), adjacency.0.get(&(b, a))) else {
            continue;
        };
        let c = rotate_to(triangles[t1], a)[2];
        let d = rotate_to(triangles[t2], b)[2];
        let (pa, pb, pc, pd) = (poly.p(a), poly.p(b), poly.p(c), poly.p(d));
        if c == d || !certainly_incircle(pa, pb, pc, pd) {
            continue;
        }
        // The flipped triangles must both be strictly positive.
        if poly.orient(c, a, d) <= 0 || poly.orient(c, d, b) <= 0 {
            continue;
        }
        if orient_exact(pc, pa, pd) <= 0 || orient_exact(pc, pd, pb) <= 0 {
            continue;
        }
        flip(triangles, adjacency, t1, t2, a, b, c, d);
        flips += 1;
        for (x, y) in [(a, d), (d, b), (b, c), (c, a)] {
            stack.push((x.min(y), x.max(y)));
        }
    }
}
