//! A genus-independent sufficient ambient-isotopy certificate for arbitrary
//! positive-weight rational tensor charts. There is no global radial centre or
//! prescribed deformation. Local projected vertex disks and all disjoint chart
//! pairs are checked throughout S -> its actual corner triangulation.
use super::*;
mod admission;
pub(super) mod algebraic;
mod bernstein;
mod conform;
mod conical;
pub(super) mod difference;
pub(super) mod domains;
mod graph_pole;
pub(in crate::minkowski::minkowski3d::curved) mod linear;
mod nonlinear;
mod nonlinear_cancel;
mod planar_pole;
mod projection;
pub(crate) mod point_material;
pub(super) mod pullback;
mod transport;
mod trim;
use bernstein::{Net, Scalar};
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub chart_triangles: usize,
    pub vertex_stars: usize,
    pub disjoint_pairs: usize,
    pub boundary_pairs: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Certificate {
    pub version: u32,
    pub source: String,
    pub depth: u8,
    pub positions: Vec<ExactPoint>,
    pub triangles: Vec<[u32; 3]>,
    pub report: Report,
}
#[derive(Clone)]
struct Patch {
    ids: [u32; 3],
    h: Net,
    motion: [Net; 2],
    derivatives: [Vec<Scalar>; 2],
}
fn points(h: &Net) -> Vec<P> {
    // Try the actual corner-triangle directions before unrelated control-net
    // triples. Duplicates do not enlarge a convex hull.
    let mut seen = BTreeSet::new();
    bernstein::corners(h)
        .into_iter()
        .chain(
            h.iter()
                .flatten()
                .filter(|p| p[3] > Q::zero())
                .map(sb::point),
        )
        .filter(|p| seen.insert(p.clone()))
        .collect()
}
fn bounds(p: &[P], n: &P) -> (Q, Q) {
    let v = p.iter().map(|p| dot(p, n)).collect::<Vec<_>>();
    (
        v.iter().min().unwrap().clone(),
        v.iter().max().unwrap().clone(),
    )
}
fn separates(a: &[P], b: &[P], n: &P) -> bool {
    if zero(n) {
        return false;
    }
    let (a0, a1) = bounds(a, n);
    let (b0, b1) = bounds(b, n);
    a1 < b0 || b1 < a0
}
/// A deliberately small set of exact separating axes. Failure only requests
/// subdivision; no axis or floating-point proximity is treated as evidence.
fn separated(a: &[P], b: &[P], work: &mut Work) -> Result<bool> {
    for k in 0..3 {
        if separates(
            a,
            b,
            &std::array::from_fn(|i| if i == k { Q::one() } else { Q::zero() }),
        ) {
            return Ok(true);
        }
    }
    let center = |p: &[P]| {
        std::array::from_fn(|i| p.iter().map(|p| p[i].clone()).sum::<Q>() / integer(p.len() as i64))
    };
    if separates(a, b, &sub(&center(a), &center(b))) {
        return Ok(true);
    }
    // Corner triangle face normals and crossed edge directions are cheap
    // exact witnesses for the usual near-linear chart hulls.
    if a.len() >= 3 && b.len() >= 3 {
        for p in [a, b] {
            let n = cross(&sub(&p[1], &p[0]), &sub(&p[2], &p[0]));
            if separates(a, b, &n) {
                return Ok(true);
            }
            for k in 0..3 {
                work.charge(1)?;
                if separates(a, b, &cross(&n, &sub(&p[(k + 1) % 3], &p[k]))) {
                    return Ok(true);
                }
            }
        }
        for i in 0..3 {
            for j in 0..3 {
                work.charge(1)?;
                if separates(
                    a,
                    b,
                    &cross(&sub(&a[(i + 1) % 3], &a[i]), &sub(&b[(j + 1) % 3], &b[j])),
                ) {
                    return Ok(true);
                }
            }
        }
    }
    // Convex hull face normals are useful even for spatially rotated charts.
    for p in [a, b] {
        for i in 0..p.len().min(6) {
            for j in i + 1..p.len().min(6) {
                for k in j + 1..p.len().min(6) {
                    work.charge(1)?;
                    if separates(a, b, &cross(&sub(&p[j], &p[i]), &sub(&p[k], &p[i]))) {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
fn disjoint(a: &[Net; 2], b: &[Net; 2], depth: u8, work: &mut Work) -> Result<bool> {
    work.charge(1)?;
    let pa = a.iter().flat_map(points).collect::<Vec<_>>();
    let pb = b.iter().flat_map(points).collect::<Vec<_>>();
    if separated(&pa, &pb, work)? {
        return Ok(true);
    }
    if depth == 4 {
        return Ok(false);
    }
    // Split the larger swept box; every child covers its full time interval.
    let diameter = |p: &[P]| {
        (0..3)
            .map(|k| {
                p.iter().map(|p| p[k].clone()).max().unwrap()
                    - p.iter().map(|p| p[k].clone()).min().unwrap()
            })
            .max()
            .unwrap()
    };
    let (first, other, swap) = if diameter(&pa) >= diameter(&pb) {
        (a, b, false)
    } else {
        (b, a, true)
    };
    let x = bernstein::subdivide(&first[0], work)?;
    let y = bernstein::subdivide(&first[1], work)?;
    for (x, y) in x.into_iter().zip(y) {
        let child = [x, y];
        if !(if swap {
            disjoint(other, &child, depth + 1, work)?
        } else {
            disjoint(&child, other, depth + 1, work)?
        }) {
            return Ok(false);
        }
    }
    Ok(true)
}
fn scalar_dot(p: &[Scalar], n: &P) -> Scalar {
    p[0].iter()
        .enumerate()
        .map(|(i, r)| {
            r.iter()
                .enumerate()
                .map(|(j, _)| (0..3).map(|k| &p[k][i][j] * &n[k]).sum())
                .collect()
        })
        .collect()
}
fn positive_sum(a: &Scalar, b: &Scalar, sign: i64) -> bool {
    a.iter().zip(b).all(|(r, s)| {
        r.iter()
            .zip(s)
            .all(|(x, y)| x + y * integer(sign) > Q::zero())
    })
}
fn regular(p: &Patch, n: &P, work: &mut Work) -> Result<bool> {
    regular_parameter_disk(p, n, work, false)
}
fn regular_parameter_disk(
    p: &Patch,
    n: &P,
    work: &mut Work,
    stationary_boundary: bool,
) -> Result<bool> {
    let c = bernstein::corners(&p.h);
    let (e, f) = (sub(&c[1], &c[0]), sub(&c[2], &c[0]));
    let det = dot(n, &cross(&e, &f));
    if det <= Q::zero() {
        return Ok(false);
    }
    // Both duals share the positive factor det. Keeping them unnormalized
    // preserves every sign/interval criterion and avoids a large denominator.
    let a = cross(&f, n);
    let b = cross(n, &e);
    let au = scalar_dot(&p.derivatives[0], &a);
    let av = scalar_dot(&p.derivatives[1], &a);
    let bu = scalar_dot(&p.derivatives[0], &b);
    let bv = scalar_dot(&p.derivatives[1], &b);
    // For J_t=(1-t)J+tI, det J_t=(1-t)^2 det J+t(1-t)tr J+t².
    // Either exact positive diagonal dominance or positive determinant and
    // trace therefore prove the complete time homotopy, including skew charts.
    if positive_sum(&au, &av, 1)
        && positive_sum(&au, &av, -1)
        && positive_sum(&bv, &bu, 1)
        && positive_sum(&bv, &bu, -1)
    {
        return Ok(true);
    }
    if !positive_sum(&au, &bv, 1)
        && !stationary_boundary
        && !p.h.iter().flatten().any(|p| p[3].is_zero())
    {
        return Ok(false);
    }
    let range = |p: &Scalar| {
        [
            p.iter().flatten().min().unwrap().clone(),
            p.iter().flatten().max().unwrap().clone(),
        ]
    };
    let times = |a: [Q; 2], b: [Q; 2]| {
        let products = [&a[0] * &b[0], &a[0] * &b[1], &a[1] * &b[0], &a[1] * &b[1]];
        [
            products.iter().min().unwrap().clone(),
            products.iter().max().unwrap().clone(),
        ]
    };
    if times(range(&au), range(&bv))[0] > times(range(&av), range(&bu))[1] {
        return Ok(true);
    }
    if 2 * (au.len() - 1) > 33 {
        return Ok(false);
    }
    let det = bernstein::difference(
        &bernstein::product(&au, &bv, work)?,
        &bernstein::product(&av, &bu, work)?,
    );
    if stationary_boundary || p.h.iter().flatten().any(|p| p[3].is_zero()) {
        // At a certified removable homogeneous vertex, strict coefficient
        // positivity is impossible. Positivity in the open triangle plus the
        // separately proved Jordan boundary still gives projected degree one.
        let trace = au
            .iter()
            .zip(&bv)
            .flat_map(|(a, b)| a.iter().zip(b).map(|(a, b)| a + b))
            .collect::<Vec<_>>();
        if !(trace.iter().all(|q| *q >= Q::zero())
            && det.iter().flatten().all(|q| *q >= Q::zero())
            && det.iter().flatten().any(|q| *q > Q::zero()))
        {
            return Ok(false);
        }
        // Prove the boundary of each singular chart, including internal star
        // edges. Interior positivity alone must not authorize an edge collapse.
        let edges = [
            bernstein::edges(&p.motion[0]),
            bernstein::edges(&p.motion[1]),
        ];
        let curves = (0..3)
            .map(|i| [edges[0][i].clone(), edges[1][i].clone()])
            .collect::<Vec<_>>();
        for curve in &curves {
            if !monotone_parameter(curve, n, work, stationary_boundary)? {
                return Ok(false);
            }
        }
        for i in 0..3 {
            for j in i + 1..3 {
                let shared = if (i + 1) % 3 == j { &c[j] } else { &c[i] };
                if !curve_pair(&curves[i], &curves[j], Some(shared), n, work)? {
                    return Ok(false);
                }
            }
        }
        return Ok(true);
    }
    Ok(det.iter().flatten().all(|q| *q > Q::zero()))
}
type Curve = [Vec<sb::H>; 2];

fn star(
    vertex: u32,
    incident: &[usize],
    patches: &[Patch],
    positions: &[P],
    work: &mut Work,
) -> Result<usize> {
    let mut links = BTreeMap::<u32, (u32, Curve)>::new();
    let mut n: P = std::array::from_fn(|_| Q::zero());
    for &i in incident {
        let p = &patches[i];
        let k = p.ids.iter().position(|v| *v == vertex).unwrap();
        let c = bernstein::corners(&p.h);
        n = add(&n, &cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0])));
        let e = (k + 1) % 3;
        let curve = [
            bernstein::edges(&p.motion[0])[e].clone(),
            bernstein::edges(&p.motion[1])[e].clone(),
        ];
        if links
            .insert(p.ids[e], (p.ids[(e + 1) % 3], curve))
            .is_some()
        {
            return Err(fail("atlas vertex link branches"));
        }
    }
    match star_projected(incident, patches, positions, &links, &n, work) {
        Ok(count) => Ok(count),
        Err(first) => {
            // Area sums can lie exactly in a cap plane at a conical rim.
            // Normalize/deduplicate directions, then reprove the entire disk.
            let mut normals = BTreeSet::new();
            for &i in incident {
                let c = bernstein::corners(&patches[i].h);
                let v = cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0]));
                let norm: Q = v
                    .iter()
                    .map(|q| {
                        if *q < Q::zero() {
                            -q.clone()
                        } else {
                            q.clone()
                        }
                    })
                    .sum();
                if !norm.is_zero() {
                    normals.insert(v.map(|q| q / &norm));
                }
            }
            let n: P = std::array::from_fn(|k| normals.iter().map(|p| p[k].clone()).sum());
            if let Ok(count) = star_projected(incident, patches, positions, &links, &n, work) {
                return Ok(count);
            }
            for n in projection::directions(&normals, work)? {
                if let Ok(count) = star_projected(incident, patches, positions, &links, &n, work) {
                    return Ok(count);
                }
            }
            Err(first)
        }
    }
}
fn star_projected(
    incident: &[usize],
    patches: &[Patch],
    positions: &[P],
    links: &BTreeMap<u32, (u32, Curve)>,
    n: &P,
    work: &mut Work,
) -> Result<usize> {
    if zero(n) {
        return Err(fail("rational atlas projection is degenerate"));
    }
    for i in incident {
        if !regular(&patches[*i], n, work)? {
            return Err(fail(
                "rational atlas projected vertex disk regularity is unproved",
            ));
        }
    }
    let start = *links.keys().next().unwrap();
    let mut next = start;
    let mut boundary = Vec::new();
    for _ in 0..links.len() {
        let (end, c) = links
            .get(&next)
            .ok_or_else(|| fail("atlas vertex link is open"))?;
        boundary.push((next, *end, c));
        next = *end;
        if next == start && boundary.len() != links.len() {
            return Err(fail("atlas vertex has disconnected links"));
        }
    }
    if next != start {
        return Err(fail("atlas vertex link is not a circle"));
    }
    for (_, _, c) in &boundary {
        if !monotone(c, &n, work)? {
            return Err(fail(
                "rational atlas projected link edge regularity is unproved",
            ));
        }
    }
    let mut count = 0;
    for (i, (a0, a1, a)) in boundary.iter().enumerate() {
        for (b0, b1, b) in &boundary[i + 1..] {
            let shared = if a0 == b1 {
                Some(&positions[*a0 as usize])
            } else if a1 == b0 {
                Some(&positions[*a1 as usize])
            } else {
                None
            };
            if !curve_pair(a, b, shared, &n, work)? {
                return Err(fail(
                    "rational atlas projected link Jordan homotopy is unproved",
                ));
            }
            count += 1;
        }
    }
    Ok(count)
}
fn append_patch(
    h: Net,
    reversed: bool,
    vertices: &mut BTreeMap<P, u32>,
    patches: &mut Vec<Patch>,
    work: &mut Work,
) -> Result<()> {
    if patches.len() >= 1024 {
        return Err(fail("rational atlas triangle budget exceeded"));
    }
    transport::check(&h)?;
    let h = if reversed { transport::reverse(&h) } else { h };
    let c = bernstein::corners(&h);
    if zero(&cross(&sub(&c[1], &c[0]), &sub(&c[2], &c[0]))) {
        return Err(fail("rational atlas has a collapsed corner simplex"));
    }
    let ids = c.map(|p| {
        let i = vertices.len() as u32;
        *vertices.entry(p).or_insert(i)
    });
    if vertices.len() > 1024 {
        return Err(fail("rational atlas vertex budget exceeded"));
    }
    let derivatives = bernstein::derivatives(&h, work)?;
    let motion = bernstein::homotopy(&h, work)?;
    patches.push(Patch {
        ids,
        h,
        motion,
        derivatives,
    });
    Ok(())
}
fn build(m: &Model, depth: u8, work: &mut Work) -> Result<(Vec<P>, Vec<Patch>)> {
    m.validate()?;
    if depth > 3 || m.faces.len() > 64 || m.bodies.is_empty() {
        return Err(fail("rational atlas source budget exceeded"));
    }
    if m.faces
        .iter()
        .map(|f| f.surface.control_points.iter().map(Vec::len).sum::<usize>())
        .sum::<usize>()
        > 8192
        || m.faces
            .iter()
            .flat_map(|f| f.surface.control_points.iter().flatten())
            .any(|p| p.len() != 3 || p.iter().any(|x| !x.is_finite() || x.abs() > 1e6))
    {
        return Err(fail("rational atlas coordinate/control budget exceeded"));
    }
    let mut uses = Vec::new();
    for b in &m.bodies {
        for shell in std::iter::once(b.outer_shell).chain(b.inner_shells.iter().copied()) {
            if !m.shells[shell].closed {
                return Err(fail("rational atlas needs closed shells"));
            }
            uses.extend(m.shells[shell].faces.iter().cloned());
        }
    }
    let mut stations = BTreeSet::from([Q::zero(), Q::one()]);
    for u in &uses {
        let s = &m.faces[u.face].surface;
        if !(1..=8).contains(&s.degree_u) || !(1..=8).contains(&s.degree_v) {
            return Err(fail("rational atlas degree must be 1..8"));
        }
        sb::continuous(s, work)?;
        for (k, d, count) in [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ] {
            let a = Q::from_float(k[d]).unwrap();
            let b = Q::from_float(k[count]).unwrap();
            for x in &k[d..=count] {
                let x = (Q::from_float(*x).unwrap() - &a) / (&b - &a);
                stations.insert(x.clone());
                stations.insert(Q::one() - x);
            }
        }
    }
    trimmed::stations(m, &uses, &mut stations)?;
    for i in 0..=1usize << depth {
        stations.insert(Q::new(i.into(), (1usize << depth).into()));
    }
    let stations = stations.into_iter().collect::<Vec<_>>();
    if stations.len() > 33 || uses.len() * (stations.len() - 1).pow(2) > 512 {
        return Err(fail("rational atlas chart cell budget exceeded"));
    }
    let mut vertices = BTreeMap::<P, u32>::new();
    let mut patches = Vec::new();
    for u in uses {
        let s = &m.faces[u.face].surface;
        if rectangle(m, u.face).is_err() {
            let mut charts = trim::charts(m, u.face, work)?;
            for _ in 0..depth {
                charts = charts
                    .iter()
                    .map(|h| bernstein::subdivide(h, work))
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();
            }
            for h in charts {
                append_patch(h, u.reversed, &mut vertices, &mut patches, work)?;
            }
            continue;
        }
        let range = |knots: &[f64], degree: usize, count: usize| {
            [
                Q::from_float(knots[degree]).unwrap(),
                Q::from_float(knots[count]).unwrap(),
            ]
        };
        let (a, b) = (
            range(&s.knots_u, s.degree_u, s.control_points.len()),
            range(&s.knots_v, s.degree_v, s.control_points[0].len()),
        );
        for x in stations.windows(2) {
            for y in stations.windows(2) {
                let h = sb::cell(
                    s,
                    [&x[0], &x[1]].map(|t| &a[0] + t * (&a[1] - &a[0])),
                    [&y[0], &y[1]].map(|t| &b[0] + t * (&b[1] - &b[0])),
                )?;
                let mut charts = Vec::new();
                let mut rotated = h.clone();
                for _ in 0..4 {
                    if let Some(pole) = transport::pole(&rotated, work)? {
                        // The blowdown's (x,y) chart has the opposite UV orientation.
                        charts.push(transport::reverse(&pole));
                        break;
                    }
                    rotated = transport::rotate(&rotated);
                }
                if charts.is_empty() {
                    let c = sb::corners(&h);
                    let uv = [[0, 0], [1, 0], [1, 1], [0, 1]];
                    let tris = if dot(&sub(&c[0], &c[2]), &sub(&c[0], &c[2]))
                        >= dot(&sub(&c[1], &c[3]), &sub(&c[1], &c[3]))
                    {
                        [[0, 1, 2], [0, 2, 3]]
                    } else {
                        [[0, 1, 3], [1, 2, 3]]
                    };
                    for t in tris {
                        charts.push(bernstein::triangle(&h, t.map(|i| uv[i]), work)?);
                    }
                }
                for h in charts {
                    append_patch(h, u.reversed, &mut vertices, &mut patches, work)?;
                }
            }
        }
    }
    let mut positions = vec![std::array::from_fn(|_| Q::zero()); vertices.len()];
    for (p, i) in vertices {
        positions[i as usize] = p;
    }
    Ok((positions, patches))
}
pub(super) fn derive(m: &Model, depth: u8) -> Result<Certificate> {
    checked_charts(m, depth).map(|(certificate, _)| certificate)
}
fn checked_charts(m: &Model, depth: u8) -> Result<(Certificate, Vec<Patch>)> {
    admission::checked(m, depth)
}
fn checked_charts_recipe(
    m: &Model,
    depth: u8,
    normalize_images: bool,
) -> Result<(Certificate, Vec<Patch>)> {
    let mut work = Work::default();
    let (mut positions, mut patches) = build(m, depth, &mut work)?;
    if normalize_images && !conform::normalize_images(&positions, &mut patches, &mut work)? {
        return Err(fail(
            "no proved source image normalization extends this atlas",
        ));
    }
    conform::synchronize(&positions, &mut patches, &mut work)?;
    let mut original = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
    for p in &patches {
        for (k, mut e) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            if a > b {
                e.reverse();
            }
            original.entry((a.min(b), a.max(b))).or_default().push(e);
        }
    }
    if original
        .values()
        .any(|e| e.len() == 2 && !sb::same(&e[0], &e[1]))
    {
        // Preserve the old successful recipe and its deterministic receipts.
        // A failed attempt never leaves partially changed charts behind.
        let mut candidate = patches.clone();
        let mut trial = Work(work.0);
        if transport::synchronize(&mut candidate, &mut trial).is_ok() {
            patches = candidate;
            work = trial;
        } else {
            let mut candidate = patches.clone();
            let mut trial = Work(work.0);
            if transport::synchronize_images(&mut candidate, &mut trial).is_ok() {
                patches = candidate;
                work = trial;
            } else {
                nonlinear::synchronize(&mut patches, &mut work)?;
            }
        }
    }
    if patches
        .iter()
        .any(|p| p.h.iter().flatten().any(|h| h[3].is_zero()))
    {
        // A whole conical sector can have no common projection with its cap
        // at a rim vertex. Subdivide the exact source charts, including their
        // neighbors, to obtain smaller conforming stars before proving them.
        // Image-only straight seam gluing is insufficient for midpoint splits.
        let mut seams = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
        for p in &patches {
            for (k, mut h) in bernstein::edges(&p.h).into_iter().enumerate() {
                let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
                if a > b {
                    h.reverse();
                }
                seams.entry((a.min(b), a.max(b))).or_default().push(h);
            }
        }
        if seams
            .values()
            .any(|e| e.len() != 2 || !sb::same(&e[0], &e[1]))
        {
            return Err(fail(
                "singular atlas refinement needs pointwise synchronized seams",
            ));
        }
        let mut vertices = BTreeMap::new();
        let mut refined = Vec::new();
        for p in &patches {
            for h in bernstein::subdivide(&p.h, &mut work)? {
                append_patch(h, false, &mut vertices, &mut refined, &mut work)?;
            }
        }
        positions = vec![std::array::from_fn(|_| Q::zero()); vertices.len()];
        for (p, i) in vertices {
            positions[i as usize] = p;
        }
        patches = refined;
    }
    let mut edges = BTreeMap::<(u32, u32), Vec<Vec<sb::H>>>::new();
    let mut stars = vec![Vec::new(); positions.len()];
    for (i, p) in patches.iter().enumerate() {
        for v in p.ids {
            stars[v as usize].push(i);
        }
        for (k, mut e) in bernstein::edges(&p.h).into_iter().enumerate() {
            let (a, b) = (p.ids[k], p.ids[(k + 1) % 3]);
            if a > b {
                e.reverse();
            }
            edges.entry((a.min(b), a.max(b))).or_default().push(e);
        }
    }
    for ((a, b), e) in &edges {
        if e.len() != 2 || !(sb::same(&e[0], &e[1]) || segment_seam(&e[0], &e[1], &mut work)?) {
            return Err(fail("rational atlas has an unmatched pointwise seam"));
        }
        let balance = patches
            .iter()
            .map(|p| {
                (0..3)
                    .map(|k| {
                        if p.ids[k] == *a && p.ids[(k + 1) % 3] == *b {
                            1
                        } else if p.ids[k] == *b && p.ids[(k + 1) % 3] == *a {
                            -1
                        } else {
                            0
                        }
                    })
                    .sum::<i32>()
            })
            .sum::<i32>();
        if balance != 0 {
            return Err(fail("rational atlas edge orientations disagree"));
        }
    }
    let mut boundary_pairs = 0;
    for (v, incident) in stars.iter().enumerate() {
        boundary_pairs += star(v as u32, incident, &patches, &positions, &mut work)?;
    }
    let mut pairs = 0;
    for (i, a) in patches.iter().enumerate() {
        for b in &patches[i + 1..] {
            if a.ids.iter().any(|id| b.ids.contains(id)) {
                continue;
            }
            if !disjoint(&a.motion, &b.motion, 0, &mut work)? {
                return Err(fail("rational atlas disjoint chart homotopy is unproved"));
            }
            pairs += 1;
        }
    }
    let certificate = Certificate {
        version: 1,
        source: source_id(&Source::Brep(m.clone()))?,
        depth,
        positions: positions.iter().map(|p| p.each_ref().map(ratio)).collect(),
        triangles: patches.iter().map(|p| p.ids).collect(),
        report: Report {
            chart_triangles: patches.len(),
            vertex_stars: stars.len(),
            disjoint_pairs: pairs,
            boundary_pairs,
        },
    };
    Ok((certificate, patches))
}
/// Bind the proved PL endpoint to the exact chain actually decomposed by the
/// Minkowski code. Equality includes orientations and multiplicities.
pub(super) fn nominal_report(m: &Model, depth: u8, input: &Input) -> Result<Report> {
    let c = derive(m, depth)?;
    let points = c
        .positions
        .iter()
        .map(|p| {
            p.each_ref()
                .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()))
        })
        .collect::<Vec<_>>();
    let canonical = |mut f: Vec<P>| {
        let i = f.iter().enumerate().min_by_key(|(_, p)| *p).unwrap().0;
        f.rotate_left(i);
        f
    };
    let mut expected = c
        .triangles
        .iter()
        .map(|t| canonical(t.iter().map(|i| points[*i as usize].clone()).collect()))
        .collect::<Vec<_>>();
    let mut actual = input
        .faces
        .iter()
        .cloned()
        .map(canonical)
        .collect::<Vec<_>>();
    expected.sort();
    actual.sort();
    if expected != actual {
        return Err(fail(
            "rational atlas endpoint is not the actual nominal source chain",
        ));
    }
    Ok(c.report)
}
#[cfg(feature = "codec")]
#[path = "atlas/codec.rs"]
mod codec;
#[cfg(test)]
#[path = "atlas/tests/tests_integration.rs"]
mod tests;
pub(super) mod region_brep;
#[path = "atlas/curve_projection.rs"]
mod curve_projection;
#[cfg(test)] use curve_projection::curve_points;
use curve_projection::{monotone, monotone_parameter, curve_pair, segment_seam};
pub(super) mod boolean;
