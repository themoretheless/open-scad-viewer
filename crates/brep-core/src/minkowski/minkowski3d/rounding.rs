//! Whole-homotopy embedding proof for coordinate rounding. Every moving
//! triangle pair remains disjoint except for its shared abstract simplex.
//! Sufficient exact certificates refuse when an embedding cannot be proved.
use super::*;
#[derive(Clone, Debug)]
pub struct Report {
    pub precision: &'static str,
    pub pairs: usize,
    pub separated: usize,
    pub shared_edges: usize,
    pub shared_vertices: usize,
    pub identity: bool,
}
type Polynomial = Vec<Q>;
type Motion = [P; 2];
fn plus(a: &[Q], b: &[Q]) -> Polynomial {
    let mut p = vec![Q::zero(); a.len().max(b.len())];
    for (i, x) in a.iter().enumerate() {
        p[i] += x
    }
    for (i, x) in b.iter().enumerate() {
        p[i] += x
    }
    p
}
fn product(a: &[Q], b: &[Q]) -> Polynomial {
    let mut p = vec![Q::zero(); a.len() + b.len() - 1];
    for (i, x) in a.iter().enumerate() {
        for (j, y) in b.iter().enumerate() {
            p[i + j] += x * y
        }
    }
    p
}
fn diff(a: &Motion, b: &Motion) -> [Polynomial; 3] {
    std::array::from_fn(|i| {
        vec![
            &a[0][i] - &b[0][i],
            (&a[1][i] - &a[0][i]) - (&b[1][i] - &b[0][i]),
        ]
    })
}
fn minus(a: &[Q], b: &[Q]) -> Polynomial {
    let mut p = vec![Q::zero(); a.len().max(b.len())];
    for (i, x) in a.iter().enumerate() {
        p[i] += x
    }
    for (i, x) in b.iter().enumerate() {
        p[i] -= x
    }
    p
}
fn pcross(a: &[Polynomial; 3], b: &[Polynomial; 3]) -> [Polynomial; 3] {
    std::array::from_fn(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        minus(&product(&a[j], &b[k]), &product(&a[k], &b[j]))
    })
}
fn pdot(a: &[Polynomial; 3], b: &[Polynomial; 3]) -> Polynomial {
    (0..3).fold(vec![Q::zero()], |p, i| plus(&p, &product(&a[i], &b[i])))
}
fn choose(n: usize, k: usize) -> i64 {
    let mut x = 1;
    for i in 0..k {
        x = x * (n - i) as i64 / (i + 1) as i64
    }
    x
}
fn positive(p: &[Q]) -> bool {
    let n = p.len() - 1;
    let cn: Vec<i64> = (0..=n).map(|j| choose(n, j)).collect();
    let coefficients = (0..=n)
        .map(|i| {
            (0..=i)
                .map(|j| &p[j] * Q::new(choose(i, j).into(), cn[j].into()))
                .sum()
        })
        .collect::<Vec<_>>();
    positive_bernstein(&coefficients, 0)
}
fn positive_bernstein(p: &[Q], depth: u8) -> bool {
    if p[0] <= Q::zero() || p.last().unwrap() <= &Q::zero() {
        return false;
    }
    // All interior Bernstein basis functions are strictly positive; zero
    // coefficients are harmless when the endpoint coefficients are positive.
    if p.iter().all(|x| *x >= Q::zero()) {
        return true;
    }
    if depth == 12 {
        return false;
    }
    let two = integer(2);
    let mut row = p.to_vec();
    let mut left = Vec::with_capacity(p.len());
    let mut right = Vec::with_capacity(p.len());
    left.push(row[0].clone());
    right.push(row.last().unwrap().clone());
    while row.len() > 1 {
        for i in 0..row.len() - 1 {
            row[i] = (&row[i] + &row[i + 1]) / &two;
        }
        row.pop();
        left.push(row[0].clone());
        right.push(row.last().unwrap().clone());
    }
    right.reverse();
    positive_bernstein(&left, depth + 1) && positive_bernstein(&right, depth + 1)
}

fn constant(p: &P) -> [Polynomial; 3] {
    std::array::from_fn(|i| vec![p[i].clone()])
}
fn valid_triangle(t: &[Motion; 3]) -> bool {
    let n = pcross(&diff(&t[1], &t[0]), &diff(&t[2], &t[0]));
    let reference = cross(&sub(&t[1][0], &t[0][0]), &sub(&t[2][0], &t[0][0]));
    positive(&pdot(&n, &constant(&reference))) || positive(&pdot(&n, &n))
}
fn ranges(p: &[P], n: &P) -> (Q, Q) {
    let first = dot(n, &p[0]);
    let mut lo = first.clone();
    let mut hi = first;
    for pt in &p[1..] {
        let v = dot(n, pt);
        if v < lo {
            lo = v;
        } else if v > hi {
            hi = v;
        }
    }
    (lo, hi)
}
fn axis_separates(a: &[P], b: &[P], n: &P) -> bool {
    if zero(n) {
        return false;
    }
    let (x, y) = ranges(a, n);
    let (u, v) = ranges(b, n);
    y < u || v < x
}
pub(super) fn separated(a: &[P], b: &[P], work: &mut Work) -> Result<bool> {
    for i in 0..3 {
        if a.iter().map(|p| &p[i]).max().unwrap() < b.iter().map(|p| &p[i]).min().unwrap()
            || b.iter().map(|p| &p[i]).max().unwrap() < a.iter().map(|p| &p[i]).min().unwrap()
        {
            return Ok(true);
        }
    }
    // The usual 3D face/edge axes are incomplete for two coplanar
    // lower-dimensional hulls: every cross product can be normal to their
    // plane. Include in-plane edge normals, with an exact coplanarity test.
    let all = a.iter().chain(b).collect::<Vec<_>>();
    let mut normal = None;
    'plane: for p in all.iter().skip(1) {
        for q in all.iter().skip(1) {
            let n = cross(&sub(p, all[0]), &sub(q, all[0]));
            if !zero(&n) {
                normal = Some(n);
                break 'plane;
            }
        }
    }
    if let Some(n) = normal {
        if all.iter().all(|p| dot(&n, &sub(p, all[0])).is_zero()) {
            for p in [a, b] {
                for i in 0..p.len() {
                    for j in i + 1..p.len() {
                        work.charge(1)?;
                        if axis_separates(a, b, &cross(&n, &sub(&p[j], &p[i]))) {
                            return Ok(true);
                        }
                    }
                }
            }
            return Ok(false);
        }
    } else {
        // Collinear hulls (including points) need their line direction.
        return Ok(all
            .iter()
            .skip(1)
            .any(|p| axis_separates(a, b, &sub(p, all[0]))));
    }
    for p in [a, b] {
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                for k in j + 1..p.len() {
                    work.charge(1)?;
                    if axis_separates(a, b, &cross(&sub(&p[j], &p[i]), &sub(&p[k], &p[i]))) {
                        return Ok(true);
                    }
                }
            }
        }
    }
    for i in 0..a.len() {
        for j in i + 1..a.len() {
            for k in 0..b.len() {
                for l in k + 1..b.len() {
                    work.charge(1)?;
                    if axis_separates(a, b, &cross(&sub(&a[j], &a[i]), &sub(&b[l], &b[k]))) {
                        return Ok(true);
                    }
                }
            }
        }
    }
    Ok(false)
}
/// Search exact supporting planes using closest-point candidates on all faces
/// of dimension <=2 of the small vector hull. A candidate is admitted only if
/// all input vectors are strictly on its positive side.
pub(super) fn pointed(v: &[P], work: &mut Work) -> Result<bool> {
    let valid = |n: &P| !zero(n) && v.iter().all(|p| dot(n, p) > Q::zero());
    for p in v {
        if valid(p) {
            return Ok(true);
        }
    }
    for i in 0..v.len() {
        for j in i + 1..v.len() {
            work.charge(1)?;
            let d = sub(&v[j], &v[i]);
            let den = dot(&d, &d);
            if den.is_zero() {
                continue;
            }
            let t = -dot(&v[i], &d) / den;
            let n = std::array::from_fn(|k| &v[i][k] + &t * &d[k]);
            if valid(&n) {
                return Ok(true);
            }
        }
    }
    for i in 0..v.len() {
        for j in i + 1..v.len() {
            for k in j + 1..v.len() {
                work.charge(1)?;
                let a = sub(&v[j], &v[i]);
                let b = sub(&v[k], &v[i]);
                let n = cross(&a, &b);
                let den = dot(&n, &n);
                if den.is_zero() {
                    continue;
                }
                let scale = dot(&n, &v[i]) / den;
                let candidate = n.map(|x| x * &scale);
                if valid(&candidate) {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}
fn edge_safe(a: &Motion, b: &Motion, p: &Motion, q: &Motion) -> bool {
    let e = diff(b, a);
    let u = diff(p, a);
    let v = diff(q, a);
    let det = pdot(&e, &pcross(&u, &v));
    positive(&det)
        || positive(&det.iter().map(|x| -x).collect::<Vec<_>>())
        || positive(
            &pdot(&pcross(&e, &u), &pcross(&e, &v))
                .iter()
                .map(|x| -x)
                .collect::<Vec<_>>(),
        )
}
pub(super) fn prove(
    exact: &[P],
    positions: &[f64],
    triangles: &[[u32; 3]],
    precision: &'static str,
) -> Result<Report> {
    if positions.len() != exact.len() * 3 {
        return Err(fail("rounded coordinate census mismatch"));
    }
    let motions = exact
        .iter()
        .zip(positions.chunks_exact(3))
        .map(|(p, x)| {
            if x.iter().any(|x| !x.is_finite()) {
                return Err(fail("nonfinite rounded coordinate"));
            }
            Ok([
                p.clone(),
                std::array::from_fn(|i| Q::from_float(x[i]).unwrap()),
            ])
        })
        .collect::<Result<Vec<Motion>>>()?;
    prove_motions(motions, triangles, precision)
}
/// The same complete moving-simplex checker for exact rational endpoint
/// meshes. Used by source-atlas certificates; no intermediate binary64 mesh
/// or coordinate-error estimate stands in for the homotopy.
#[cfg(test)]
pub(super) fn prove_exact(from: &[P], to: &[P], triangles: &[[u32; 3]]) -> Result<Report> {
    if from.len() != to.len() {
        return Err(fail("exact homotopy vertex census differs"));
    }
    for p in from.iter().chain(to) {
        guard(p)?;
    }
    prove_motions(
        from.iter()
            .zip(to)
            .map(|(a, b)| [a.clone(), b.clone()])
            .collect(),
        triangles,
        "exact",
    )
}
/// Common moving translations do not affect contact. Subtract one vertex
/// before enclosing a time slab, so large rigid motion does not inflate bounds.
fn pair_slab(
    a: &[Motion; 3],
    b: &[Motion; 3],
    ai: &[u32; 3],
    bi: &[u32; 3],
    shared: &[u32],
    work: &mut Work,
) -> Result<bool> {
    match shared.len() {
        0 => {
            let absolute = |t: &[Motion; 3]| t.iter().flatten().cloned().collect::<Vec<_>>();
            if separated(&absolute(a), &absolute(b), work)? {
                return Ok(true);
            }
            let relative = |t: &[Motion; 3]| {
                t.iter()
                    .flat_map(|m| (0..2).map(|k| sub(&m[k], &a[0][k])))
                    .collect::<Vec<_>>()
            };
            separated(&relative(a), &relative(b), work)
        }
        1 => {
            let origin = &a[ai.iter().position(|i| *i == shared[0]).unwrap()];
            let mut vectors = Vec::with_capacity(8);
            for (t, ids, sign) in [(a, ai, 1), (b, bi, -1)] {
                for (m, id) in t.iter().zip(ids) {
                    if *id != shared[0] {
                        let scale = integer(sign);
                        for k in 0..2 {
                            vectors.push(sub(&m[k], &origin[k]).map(|q| q * &scale));
                        }
                    }
                }
            }
            pointed(&vectors, work)
        }
        2 => {
            let vertex = |id: u32| -> &Motion {
                if let Some(i) = ai.iter().position(|i| *i == id) {
                    &a[i]
                } else {
                    &b[bi.iter().position(|i| *i == id).unwrap()]
                }
            };
            Ok(edge_safe(
                vertex(shared[0]),
                vertex(shared[1]),
                vertex(*ai.iter().find(|i| !shared.contains(i)).unwrap()),
                vertex(*bi.iter().find(|i| !shared.contains(i)).unwrap()),
            ))
        }
        _ => Ok(false),
    }
}
fn bisect(t: &[Motion; 3]) -> [[Motion; 3]; 2] {
    let two = integer(2);
    let mid: [P; 3] =
        std::array::from_fn(|i| std::array::from_fn(|k| (&t[i][0][k] + &t[i][1][k]) / &two));
    [
        std::array::from_fn(|i| [t[i][0].clone(), mid[i].clone()]),
        std::array::from_fn(|i| [mid[i].clone(), t[i][1].clone()]),
    ]
}
/// Every accepted slab covers a closed time interval; subdivision never
/// replaces a continuous check with samples. Exhaustion remains unproved.
fn pair_over_time(
    a: &[Motion; 3],
    b: &[Motion; 3],
    ai: &[u32; 3],
    bi: &[u32; 3],
    shared: &[u32],
    depth: u8,
    work: &mut Work,
) -> Result<bool> {
    work.charge(1)?;
    if pair_slab(a, b, ai, bi, shared, work)? {
        return Ok(true);
    }
    if depth == 12 || shared.len() == 3 {
        return Ok(false);
    }
    for k in 0..2 {
        let endpoint =
            |t: &[Motion; 3]| std::array::from_fn(|i| [t[i][k].clone(), t[i][k].clone()]);
        if !pair_slab(&endpoint(a), &endpoint(b), ai, bi, shared, work)? {
            return Ok(false);
        }
    }
    let [a0, a1] = bisect(a);
    let [b0, b1] = bisect(b);
    Ok(pair_over_time(&a0, &b0, ai, bi, shared, depth + 1, work)?
        && pair_over_time(&a1, &b1, ai, bi, shared, depth + 1, work)?)
}
fn prove_motions(
    motions: Vec<Motion>,
    triangles: &[[u32; 3]],
    precision: &'static str,
) -> Result<Report> {
    let mut r = Report {
        precision,
        pairs: 0,
        separated: 0,
        shared_edges: 0,
        shared_vertices: 0,
        identity: motions.iter().all(|m| m[0] == m[1]),
    };
    if r.identity {
        return Ok(r);
    }
    let mut work = Work::default();
    let tri_motions: Vec<[Motion; 3]> = triangles
        .iter()
        .map(|t| t.map(|i| motions[i as usize].clone()))
        .collect();
    for m in &tri_motions {
        if !valid_triangle(m) {
            return Err(fail("rounding homotopy can collapse or reverse a triangle"));
        }
    }
    for (i, (a, x)) in triangles.iter().zip(&tri_motions).enumerate() {
        for (b, y) in triangles[i + 1..].iter().zip(&tri_motions[i + 1..]) {
            r.pairs += 1;
            let shared = a
                .iter()
                .copied()
                .filter(|i| b.contains(i))
                .collect::<Vec<_>>();
            if !pair_over_time(x, y, a, b, &shared, 0, &mut work)? {
                #[cfg(test)]
                if std::env::var_os("OSV_ROUNDING_DIAGNOSTICS").is_some() {
                    eprintln!("unproved rounded pair {a:?} {b:?}, shared {shared:?}");
                    for k in 0..2 {
                        eprintln!(
                            "endpoint {k}: {:?} {:?}",
                            x.each_ref()
                                .map(|m| m[k].each_ref().map(|q| q.to_f64().unwrap())),
                            y.each_ref()
                                .map(|m| m[k].each_ref().map(|q| q.to_f64().unwrap()))
                        );
                        eprintln!(
                            "endpoint slab admitted: {:?}",
                            pair_slab(
                                &x.each_ref().map(|m| [m[k].clone(), m[k].clone()]),
                                &y.each_ref().map(|m| [m[k].clone(), m[k].clone()]),
                                a,
                                b,
                                &shared,
                                &mut work
                            )
                        );
                    }
                }
                return Err(fail(match shared.len() {
                    0 => "rounding separation of disjoint triangles is unproved",
                    1 => "rounding vertex-star separation is unproved",
                    2 => "rounding edge-star separation is unproved",
                    _ => "duplicate rounded abstract triangle",
                }));
            }
            match shared.len() {
                0 => r.separated += 1,
                1 => r.shared_vertices += 1,
                2 => r.shared_edges += 1,
                _ => unreachable!(),
            }
        }
    }
    Ok(r)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn moving_origin_must_not_erase_an_absolute_thin_gap_witness() {
        let gap = Q::new(1.into(), num_bigint::BigInt::one() << 80);
        let from = vec![
            [integer(0), integer(0), integer(0)],
            [integer(10), integer(10), integer(0)],
            [integer(10), integer(20), integer(0)],
            [integer(10) + &gap, integer(0), integer(0)],
            [integer(11), integer(10), integer(0)],
            [integer(10) + gap, integer(20), integer(0)],
        ];
        let mut to = from.clone();
        to[0][0] = integer(9);
        assert!(
            super::super::rounding_legacy::prove_exact(&from, &to, &[[0, 1, 2], [3, 4, 5]]).is_ok()
        );
        assert!(prove_exact(&from, &to, &[[0, 1, 2], [3, 4, 5]]).is_ok());
    }
    #[test]
    fn a_triangle_may_rotate_beyond_its_initial_normal_without_collapsing() {
        let from = vec![[0, 0, 0], [1, 0, 0], [0, 1, 0]]
            .into_iter()
            .map(|p| p.map(integer))
            .collect::<Vec<_>>();
        let to = vec![[0, 0, 0], [1, 0, 0], [0, -1, 1]]
            .into_iter()
            .map(|p| p.map(integer))
            .collect::<Vec<_>>();
        assert!(prove_exact(&from, &to, &[[0, 1, 2]]).is_ok());
        let mut collapsed = to;
        collapsed[2][2] = Q::zero();
        assert!(prove_exact(&from, &collapsed, &[[0, 1, 2]]).is_err());
    }
    #[test]
    fn coplanar_disjoint_hulls_need_in_plane_separating_axes() {
        let a = vec![[-8, 0, 0], [-8, 2, 0], [-7, 1, 0]]
            .into_iter()
            .map(|p| p.map(integer))
            .collect::<Vec<_>>();
        let b = vec![[-7, -2, 0], [-7, -1, 0], [-6, 0, 0]]
            .into_iter()
            .map(|p| p.map(integer))
            .collect::<Vec<_>>();
        assert!(separated(&a, &b, &mut Work::default()).unwrap());
        assert!(!separated(&a, &a, &mut Work::default()).unwrap());
    }
    #[test]
    fn synchronous_rotation_proves_disjointness_after_time_subdivision() {
        let from = vec![
            [0, 0, 0],
            [20, 0, 0],
            [0, 20, 0],
            [15, 15, 0],
            [16, 15, 0],
            [15, 16, 0],
        ]
        .into_iter()
        .map(|p| p.map(integer))
        .collect::<Vec<_>>();
        let to = from
            .iter()
            .map(|p| [-p[1].clone(), p[0].clone(), p[2].clone()])
            .collect::<Vec<_>>();
        let a = std::array::from_fn(|i| [from[i].clone(), to[i].clone()]);
        let b = std::array::from_fn(|i| [from[i + 3].clone(), to[i + 3].clone()]);
        assert!(!pair_slab(&a, &b, &[0, 1, 2], &[3, 4, 5], &[], &mut Work::default()).unwrap());
        assert!(prove_exact(&from, &to, &[[0, 1, 2], [3, 4, 5]]).is_ok());
    }
    #[test]
    fn common_large_translation_does_not_inflate_separation_bounds() {
        let from = vec![
            [0, 0, 0],
            [2, 0, 0],
            [0, 2, 0],
            [0, 0, 1],
            [2, 0, 1],
            [0, 2, 1],
        ]
        .into_iter()
        .map(|p| p.map(integer))
        .collect::<Vec<_>>();
        let to = from
            .iter()
            .map(|p| [p[0].clone(), p[1].clone(), &p[2] + integer(8)])
            .collect::<Vec<_>>();
        assert!(
            super::super::rounding_legacy::prove_exact(&from, &to, &[[0, 1, 2], [3, 4, 5]])
                .is_err()
        );
        assert!(prove_exact(&from, &to, &[[0, 1, 2], [3, 4, 5]]).is_ok());
    }
    #[test]
    fn exact_endpoint_checker_rejects_crossing_and_checks_nonzero_motion() {
        let from = vec![
            [0, 0, 0],
            [2, 0, 0],
            [0, 2, 0],
            [0, 0, 1],
            [2, 0, 1],
            [0, 2, 1],
        ]
        .into_iter()
        .map(|p| p.map(integer))
        .collect::<Vec<_>>();
        let mut crossing = from.clone();
        for p in &mut crossing[3..] {
            p[2] = -Q::one();
        }
        assert!(prove_exact(&from, &crossing, &[[0, 1, 2], [3, 4, 5]]).is_err());
        let moved = from
            .iter()
            .map(|p| std::array::from_fn(|i| &p[i] + Q::new(1.into(), 3.into())))
            .collect::<Vec<_>>();
        let r = prove_exact(&from, &moved, &[[0, 1, 2], [3, 4, 5]]).unwrap();
        assert!(!r.identity);
        assert_eq!(r.pairs, 1);
    }
    #[test]
    fn detects_a_thin_feature_collapse() {
        let p = vec![
            [Q::one(), Q::zero(), Q::zero()],
            [
                Q::one() + Q::new(1.into(), (1u64 << 30).into()),
                Q::zero(),
                Q::zero(),
            ],
            [Q::one(), Q::one(), Q::zero()],
        ];
        let f = p
            .iter()
            .flatten()
            .map(|q| q.to_f64().unwrap() as f32 as f64)
            .collect::<Vec<_>>();
        assert!(prove(&p, &f, &[[0, 1, 2]], "f32").is_err())
    }
    #[test]
    fn rejects_crossing_disconnected_triangles() {
        let p = vec![
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [0, 1, 1],
        ]
        .into_iter()
        .map(|p| p.map(integer))
        .collect::<Vec<_>>();
        let f = vec![
            0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., -1., 1., 0., -1., 0., 1., -1.,
        ];
        assert!(prove(&p, &f, &[[0, 1, 2], [3, 4, 5]], "test").is_err())
    }
    #[test]
    fn proves_a_nonzero_uniform_translation_with_shared_stars() {
        let p = vec![[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 1, 0], [-1, 0, 0]]
            .into_iter()
            .map(|p| p.map(integer))
            .collect::<Vec<_>>();
        let f = p
            .iter()
            .flat_map(|p| {
                [
                    p[0].to_f64().unwrap() + 0.125,
                    p[1].to_f64().unwrap(),
                    p[2].to_f64().unwrap(),
                ]
            })
            .collect::<Vec<_>>();
        let r = prove(&p, &f, &[[0, 1, 2], [0, 2, 3], [0, 3, 4]], "test").unwrap();
        assert_eq!(r.pairs, 3);
        assert!(r.shared_edges > 0 && r.shared_vertices > 0)
    }
}
