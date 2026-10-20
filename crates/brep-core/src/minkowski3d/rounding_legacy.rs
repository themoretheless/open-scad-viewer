//! Frozen v3 polynomial-atlas homotopy recipe. Keep its predicates and work
//! accounting stable: they choose the serialized segment count and pair census.
//! Display rounding and new source atlases use the enhanced checker instead.
//! Whole-homotopy embedding proof for coordinate rounding. Every moving
//! triangle pair remains disjoint except for its shared abstract simplex.
//! Sufficient exact certificates refuse when an embedding cannot be proved.
use super::rounding::Report;
use super::*;
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
fn pcross(a: &[Polynomial; 3], b: &[Polynomial; 3]) -> [Polynomial; 3] {
    std::array::from_fn(|i| {
        let (j, k) = ((i + 1) % 3, (i + 2) % 3);
        plus(
            &product(&a[j], &b[k]),
            &product(&a[k], &b[j]).iter().map(|x| -x).collect::<Vec<_>>(),
        )
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
    (0..=n).all(|i| {
        (0..=i)
            .map(|j| &p[j] * Q::new(choose(i, j).into(), choose(n, j).into()))
            .sum::<Q>()
            > Q::zero()
    })
}
fn constant(p: &P) -> [Polynomial; 3] {
    std::array::from_fn(|i| vec![p[i].clone()])
}
fn valid_triangle(t: &[Motion; 3]) -> bool {
    let n = pcross(&diff(&t[1], &t[0]), &diff(&t[2], &t[0]));
    let reference = cross(&sub(&t[1][0], &t[0][0]), &sub(&t[2][0], &t[0][0]));
    positive(&pdot(&n, &constant(&reference)))
}
fn ranges(p: &[P], n: &P) -> (Q, Q) {
    let values = p.iter().map(|p| dot(n, p)).collect::<Vec<_>>();
    (
        values.iter().min().unwrap().clone(),
        values.iter().max().unwrap().clone(),
    )
}
fn axis_separates(a: &[P], b: &[P], n: &P) -> bool {
    if zero(n) {
        return false;
    }
    let (x, y) = ranges(a, n);
    let (u, v) = ranges(b, n);
    y < u || v < x
}
fn separated(a: &[P], b: &[P], work: &mut Work) -> Result<bool> {
    for i in 0..3 {
        if a.iter().map(|p| &p[i]).max().unwrap() < b.iter().map(|p| &p[i]).min().unwrap()
            || b.iter().map(|p| &p[i]).max().unwrap() < a.iter().map(|p| &p[i]).min().unwrap()
        {
            return Ok(true);
        }
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
fn pointed(v: &[P], work: &mut Work) -> Result<bool> {
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
/// The same complete moving-simplex checker for exact rational endpoint
/// meshes. Used by source-atlas certificates; no intermediate binary64 mesh
/// or coordinate-error estimate stands in for the homotopy.
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
    for t in triangles {
        let m = t.map(|i| motions[i as usize].clone());
        if !valid_triangle(&m) {
            return Err(fail("rounding homotopy can collapse or reverse a triangle"));
        }
    }
    for (i, a) in triangles.iter().enumerate() {
        for b in &triangles[i + 1..] {
            work.charge(1)?;
            r.pairs += 1;
            let shared = a
                .iter()
                .copied()
                .filter(|i| b.contains(i))
                .collect::<Vec<_>>();
            match shared.len() {
                0 => {
                    let x = a
                        .iter()
                        .flat_map(|i| motions[*i as usize].clone())
                        .collect::<Vec<_>>();
                    let y = b
                        .iter()
                        .flat_map(|i| motions[*i as usize].clone())
                        .collect::<Vec<_>>();
                    if !separated(&x, &y, &mut work)? {
                        return Err(fail(
                            "rounding separation of disjoint triangles is unproved",
                        ));
                    }
                    r.separated += 1
                }
                1 => {
                    let origin = &motions[shared[0] as usize];
                    let mut v = Vec::new();
                    for (t, sign) in [(a, 1), (b, -1)] {
                        for i in t {
                            if *i == shared[0] {
                                continue;
                            }
                            for endpoint in 0..2 {
                                let d = sub(&motions[*i as usize][endpoint], &origin[endpoint]);
                                v.push(d.map(|q| q * integer(sign)))
                            }
                        }
                    }
                    if !pointed(&v, &mut work)? {
                        return Err(fail("rounding vertex-star separation is unproved"));
                    }
                    r.shared_vertices += 1
                }
                2 => {
                    let p = *a.iter().find(|i| !shared.contains(i)).unwrap();
                    let q = *b.iter().find(|i| !shared.contains(i)).unwrap();
                    if !edge_safe(
                        &motions[shared[0] as usize],
                        &motions[shared[1] as usize],
                        &motions[p as usize],
                        &motions[q as usize],
                    ) {
                        return Err(fail("rounding edge-star separation is unproved"));
                    }
                    r.shared_edges += 1
                }
                _ => return Err(fail("duplicate rounded abstract triangle")),
            }
        }
    }
    Ok(r)
}
