//! Topology-independent ambient certificates for polynomially deformed solids.
//! F(x,y,z)=(x,y,z+f(x,y)) has the explicit global inverse z-f(x,y) and
//! isotopy z+t*f(x,y). Exact original-chart identities reduce the source to
//! polyhedral material. A separate complete moving-simplex proof connects that
//! material to the actual approximation, with every vertex/edge star checked.
use super::*;
use crate::profile_certificate::topology_prisms as planar;
type Poly2 = Vec<Vec<Q>>;
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub axis: usize,
    pub polynomial: Vec<Vec<Ratio>>,
    pub source_chart_cells: usize,
    pub reference_triangles: usize,
    pub homotopy_segments: usize,
    pub checked_pairs: usize,
}
fn axes(axis: usize) -> [usize; 2] {
    [(axis + 1) % 3, (axis + 2) % 3]
}
fn trim(mut p: Poly2) -> Poly2 {
    while p.len() > 1 && p.last().unwrap().iter().all(Zero::is_zero) {
        p.pop();
    }
    while p[0].len() > 1 && p.iter().all(|r| r.last().unwrap().is_zero()) {
        for r in &mut p {
            r.pop();
        }
    }
    p
}
fn constant(q: Q) -> Poly2 {
    vec![vec![q]]
}
fn affine(c: Q, u: Q, v: Q) -> Poly2 {
    trim(vec![vec![c, v], vec![u, Q::zero()]])
}
fn add(a: &Poly2, b: &Poly2) -> Poly2 {
    let mut c = vec![vec![Q::zero(); a[0].len().max(b[0].len())]; a.len().max(b.len())];
    for p in [a, b] {
        for (i, r) in p.iter().enumerate() {
            for (j, q) in r.iter().enumerate() {
                c[i][j] += q;
            }
        }
    }
    trim(c)
}
fn scale(p: &Poly2, q: &Q) -> Poly2 {
    trim(
        p.iter()
            .map(|r| r.iter().map(|p| p * q).collect())
            .collect(),
    )
}
fn mul(a: &Poly2, b: &Poly2, work: &mut Work) -> Result<Poly2> {
    work.charge(a.len() * a[0].len() * b.len() * b[0].len())?;
    if a.len() + b.len() > 34 || a[0].len() + b[0].len() > 34 {
        return Err(fail("polynomial atlas degree budget exceeded"));
    }
    let mut c = vec![vec![Q::zero(); a[0].len() + b[0].len() - 1]; a.len() + b.len() - 1];
    for (i, r) in a.iter().enumerate() {
        for (j, p) in r.iter().enumerate() {
            if p.is_zero() {
                continue;
            }
            for (k, s) in b.iter().enumerate() {
                for (l, q) in s.iter().enumerate() {
                    if !q.is_zero() {
                        c[i + k][j + l] += p * q;
                    }
                }
            }
        }
    }
    for r in &c {
        for q in r {
            if q.numer().bits() > 8192 || q.denom().bits() > 8192 {
                return Err(fail("polynomial atlas coefficient budget exceeded"));
            }
        }
    }
    Ok(trim(c))
}
fn compose(p: &Poly2, x: &Poly2, y: &Poly2, work: &mut Work) -> Result<Poly2> {
    let powers = |a: &Poly2, n: usize, work: &mut Work| -> Result<Vec<Poly2>> {
        let mut v = vec![constant(Q::one())];
        for _ in 1..n {
            v.push(mul(v.last().unwrap(), a, work)?);
        }
        Ok(v)
    };
    let u = powers(x, p.len(), work)?;
    let v = powers(y, p[0].len(), work)?;
    let mut out = constant(Q::zero());
    for (i, r) in p.iter().enumerate() {
        for (j, q) in r.iter().enumerate() {
            if !q.is_zero() {
                out = add(&out, &scale(&mul(&u[i], &v[j], work)?, q));
            }
        }
    }
    Ok(out)
}
fn eval2(p: &Poly2, x: &Q, y: &Q) -> Q {
    p.iter().rev().fold(Q::zero(), |a, r| {
        a * x + r.iter().rev().fold(Q::zero(), |a, q| a * y + q)
    })
}
fn choose(n: usize, k: usize) -> Q {
    (0..k).fold(Q::one(), |v, i| {
        v * integer((n - i) as i64) / integer((i + 1) as i64)
    })
}
fn power(h: &sb::Net, axis: usize) -> Result<Poly2> {
    let (n, m) = (h.len() - 1, h[0].len() - 1);
    let w = &h[0][0][3];
    if h.iter().flatten().any(|p| &p[3] != w) {
        return Err(fail("polynomial atlas needs constant rational weights"));
    }
    let mut out = vec![vec![Q::zero(); m + 1]; n + 1];
    for i in 0..=n {
        for j in 0..=m {
            for (a, row) in out.iter_mut().enumerate().skip(i) {
                for (b, q) in row.iter_mut().enumerate().skip(j) {
                    let sign = if (a - i + b - j) % 2 == 0 {
                        Q::one()
                    } else {
                        -Q::one()
                    };
                    *q += &h[i][j][axis] / w
                        * choose(n, i)
                        * choose(n - i, a - i)
                        * choose(m, j)
                        * choose(m - j, b - j)
                        * sign;
                }
            }
        }
    }
    Ok(trim(out))
}
fn linear(p: &Poly2) -> bool {
    p.iter()
        .enumerate()
        .all(|(i, r)| r.iter().enumerate().all(|(j, q)| i + j <= 1 || q.is_zero()))
}
fn coefficient(p: &Poly2, i: usize, j: usize) -> Q {
    p.get(i).and_then(|r| r.get(j)).cloned().unwrap_or_default()
}
fn candidate(h: &sb::Net, axis: usize, work: &mut Work) -> Result<Poly2> {
    let [x, y] = axes(axis);
    let px = power(h, x)?;
    let py = power(h, y)?;
    let z = power(h, axis)?;
    if !linear(&px) || !linear(&py) {
        return Err(fail(
            "polynomial atlas candidate has nonlinear planar coordinates",
        ));
    }
    let (a, b, c, d) = (
        coefficient(&px, 1, 0),
        coefficient(&px, 0, 1),
        coefficient(&py, 1, 0),
        coefficient(&py, 0, 1),
    );
    let det = &a * &d - &b * &c;
    if det.is_zero() {
        return Err(fail("polynomial atlas candidate chart is vertical"));
    }
    let (x0, y0) = (px[0][0].clone(), py[0][0].clone());
    let u = affine((-&d * &x0 + &b * &y0) / &det, &d / &det, -&b / &det);
    let v = affine((&c * &x0 - &a * &y0) / &det, -&c / &det, &a / &det);
    let z0 = eval2(&z, &Q::zero(), &Q::zero());
    let zu = eval2(&z, &Q::one(), &Q::zero()) - &z0;
    let zv = eval2(&z, &Q::zero(), &Q::one()) - &z0;
    let plane = add(&constant(z0), &add(&scale(&u, &zu), &scale(&v, &zv)));
    let f = add(&compose(&z, &u, &v, work)?, &scale(&plane, &-Q::one()));
    if linear(&f) {
        return Err(fail("polynomial atlas candidate is only an affine map"));
    }
    Ok(f)
}
fn intervals(s: &nurbs_core::surface::Surface) -> [Vec<Q>; 2] {
    [
        (&s.knots_u, s.degree_u, s.control_points.len()),
        (&s.knots_v, s.degree_v, s.control_points[0].len()),
    ]
    .map(|(k, d, n)| {
        k[d..=n]
            .iter()
            .map(|x| Q::from_float(*x).unwrap())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    })
}
fn inverse(p: &P, f: &Poly2, axis: usize) -> P {
    let [x, y] = axes(axis);
    let mut p = p.clone();
    let shift = eval2(f, &p[x], &p[y]);
    p[axis] -= shift;
    p
}
fn reference_charts(
    m: &Model,
    f: &Poly2,
    axis: usize,
    work: &mut Work,
) -> Result<(Vec<[P; 3]>, usize)> {
    let mut charts = Vec::new();
    let mut cells = 0;
    for face in &m.faces {
        let s = &face.surface;
        let [a, b] = intervals(s);
        let p = inverse(&sb::at(s, &[a[0].clone(), b[0].clone()])?, f, axis);
        let q = inverse(
            &sb::at(s, &[a.last().unwrap().clone(), b[0].clone()])?,
            f,
            axis,
        );
        let r = inverse(
            &sb::at(s, &[a[0].clone(), b.last().unwrap().clone()])?,
            f,
            axis,
        );
        let u: P = std::array::from_fn(|i| (&q[i] - &p[i]) / (a.last().unwrap() - &a[0]));
        let v: P = std::array::from_fn(|i| (&r[i] - &p[i]) / (b.last().unwrap() - &b[0]));
        let c: P = std::array::from_fn(|i| &p[i] - &u[i] * &a[0] - &v[i] * &b[0]);
        if zero(&cross(&u, &v)) {
            return Err(fail("inverse atlas source chart is degenerate"));
        }
        for aa in a.windows(2) {
            for bb in b.windows(2) {
                cells += 1;
                if cells > 512 {
                    return Err(fail("polynomial source atlas chart census exceeded"));
                }
                let h = sb::cell(
                    s,
                    [aa[0].clone(), aa[1].clone()],
                    [bb[0].clone(), bb[1].clone()],
                )?;
                let expected = |k: usize| {
                    affine(
                        &c[k] + &u[k] * &aa[0] + &v[k] * &bb[0],
                        &u[k] * (&aa[1] - &aa[0]),
                        &v[k] * (&bb[1] - &bb[0]),
                    )
                };
                let [x, y] = axes(axis);
                let xx = power(&h, x)?;
                let yy = power(&h, y)?;
                if xx != expected(x)
                    || yy != expected(y)
                    || power(&h, axis)? != add(&expected(axis), &compose(f, &xx, &yy, work)?)
                {
                    return Err(fail(
                        "whole original NURBS chart does not equal the claimed polynomial deformation",
                    ));
                }
            }
        }
        charts.push([c, u, v]);
    }
    Ok((charts, cells))
}
fn reference_input(m: &Model, charts: &[[P; 3]]) -> Result<Input> {
    let mut points = BTreeMap::<P, u32>::new();
    let mut triangles = Vec::new();
    for body in &m.bodies {
        for shell in std::iter::once(body.outer_shell).chain(body.inner_shells.iter().copied()) {
            for used in &m.shells[shell].faces {
                let face = &m.faces[used.face];
                let s = &face.surface;
                let a = &charts[used.face];
                let map = |uv: &P| {
                    std::array::from_fn::<_, 3, _>(|k| {
                        &a[0][k] + &a[1][k] * &uv[0] + &a[2][k] * &uv[1]
                    })
                };
                let center = map(&[
                    (Q::from_float(s.knots_u[s.degree_u]).unwrap()
                        + Q::from_float(s.knots_u[s.control_points.len()]).unwrap())
                        / integer(2),
                    (Q::from_float(s.knots_v[s.degree_v]).unwrap()
                        + Q::from_float(s.knots_v[s.control_points[0].len()]).unwrap())
                        / integer(2),
                    Q::zero(),
                ]);
                let loops = std::iter::once(face.outer)
                    .chain(face.holes.iter().copied())
                    .map(|i| {
                        m.loops[i]
                            .coedges
                            .iter()
                            .map(|c| c.pcurve.clone())
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let (o, n) = planar::original_wires(&loops)?;
                if n.iter().enumerate().any(|(i, d)| *d != usize::from(i != 0))
                    || o.iter()
                        .enumerate()
                        .any(|(i, s)| *s != o[0] * if i == 0 { 1 } else { -1 })
                {
                    return Err(fail("polynomial reference trim nesting is unproved"));
                }
                for c in loops.iter().flatten() {
                    let d = c.domain();
                    let end = |t: f64| -> Result<P> {
                        let t = Q::from_float(t).unwrap();
                        Ok(sb::point(&sb::curve(c, &t, &t)?[0]))
                    };
                    let (p, q) = (end(d[0])?, end(d[1])?);
                    let e = sub(&q, &p);
                    if e[0].is_zero() && e[1].is_zero() {
                        return Err(fail("polynomial reference edge collapses"));
                    }
                    for k in &c.control_points {
                        let k = [Q::from_float(k[0]).unwrap(), Q::from_float(k[1]).unwrap()];
                        if &e[0] * (&k[1] - &p[1]) - &e[1] * (&k[0] - &p[0]) != Q::zero()
                            || (0..2).any(|i| {
                                k[i] < p[i].clone().min(q[i].clone())
                                    || k[i] > p[i].clone().max(q[i].clone())
                            })
                        {
                            return Err(fail(
                                "inverse reference edge is not an exact complete line image",
                            ));
                        }
                    }
                    let mut t = [center.clone(), map(&p), map(&q)];
                    if used.reversed {
                        t.swap(1, 2);
                    }
                    if zero(&cross(&sub(&t[1], &t[0]), &sub(&t[2], &t[0]))) {
                        continue;
                    }
                    for p in t {
                        let n = points.len() as u32;
                        triangles.push(*points.entry(p).or_insert(n));
                    }
                }
            }
        }
    }
    let mut p = vec![[Q::zero(), Q::zero(), Q::zero()]; points.len()];
    for (v, i) in points {
        p[i as usize] = v;
    }
    input_chain(&p, &triangles, false)
}
fn subset(a: &[Poly], b: &[Poly], work: &mut Work) -> Result<bool> {
    for a in a {
        let mut remaining = vec![a.clone()];
        for b in b {
            let planes = b
                .faces
                .iter()
                .map(|f| normalized(&plane(f), true))
                .collect::<BTreeSet<_>>();
            let mut next = Vec::new();
            for c in remaining {
                let mut inside = Some(c);
                for h in &planes {
                    let Some(c) = inside.take() else {
                        break;
                    };
                    if let Some(p) = c.clip(&neg(h), work)? {
                        next.push(p);
                    }
                    inside = c.clip(h, work)?;
                }
            }
            if next.len() > 512 {
                return Err(fail("atlas exact coverage partition exceeded"));
            }
            remaining = next;
            if remaining.is_empty() {
                break;
            }
        }
        if !remaining.is_empty() {
            return Ok(false);
        }
    }
    Ok(true)
}
fn equal(a: &[Poly], b: &[Poly], work: &mut Work) -> Result<bool> {
    if boundary_dyadic(a, work)? == boundary_dyadic(b, work)? {
        return Ok(true);
    }
    Ok(subset(a, b, work)? && subset(b, a, work)?)
}
fn attempt(m: &Model, nominal: &[Poly], axis: usize, f: &Poly2, work: &mut Work) -> Result<Report> {
    let (charts, cells) = reference_charts(m, f, axis, work)?;
    let base = decompose(&reference_input(m, &charts)?, work)?;
    let (points, triangles) = boundary_dyadic(nominal, work)?;
    if points.is_empty() {
        return Err(fail("empty polynomial atlas is unproved"));
    }
    let mut cap_vertices = BTreeSet::<u32>::new();
    for t in &triangles {
        let a = t.map(|i| points[i as usize].clone());
        if !cross(&sub(&a[1], &a[0]), &sub(&a[2], &a[0]))[axis].is_zero() {
            cap_vertices.extend(t.iter().copied());
        }
    }
    let planes = charts
        .iter()
        .map(|a| {
            plane(&[
                a[0].clone(),
                super::super::add(&a[0], &a[1]),
                super::super::add(&a[0], &a[2]),
            ])
        })
        .filter(|p| !p[axis].is_zero())
        .collect::<Vec<_>>();
    let mut reference = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let mut r = inverse(p, f, axis);
            if cap_vertices.contains(&(i as u32)) {
                let closest = planes
                    .iter()
                    .map(|h| {
                        let z = (&h[3]
                            - (0..3)
                                .filter(|k| *k != axis)
                                .map(|k| &h[k] * &p[k])
                                .sum::<Q>())
                            / &h[axis];
                        ((&z - &r[axis]).abs(), z)
                    })
                    .min()
                    .ok_or_else(|| {
                        fail("polynomial reference has no transverse supporting plane")
                    })?;
                r[axis] = closest.1;
            }
            guard(&r)?;
            Ok(r)
        })
        .collect::<Result<Vec<_>>>()?;
    // Prefer an already exact reference triangulation. Moving wall-interior
    // subdivision vertices by the nonlinear inverse is unnecessary when the
    // actual nominal material itself equals the inverse source material.
    if equal(nominal, &base, work)? {
        reference = points.clone();
    }
    let indices = triangles.iter().flatten().copied().collect::<Vec<_>>();
    let input = input_points(&reference, &indices)?;
    if !topology::manifold_links(&input) {
        return Err(fail("polynomial reference has a nonmanifold vertex star"));
    }
    let represented = decompose(&input, work)?;
    if !equal(&represented, &base, work)? {
        return Err(fail(
            "actual reference triangulation differs from original inverse source material",
        ));
    }
    let mut proved = None;
    for n in [1usize, 2, 4, 8] {
        let mut pairs = 0;
        let mut ok = true;
        for k in 0..n {
            let at = |k: usize| {
                let t = Q::new(k.into(), n.into());
                reference
                    .iter()
                    .zip(&points)
                    .map(|(a, b)| std::array::from_fn(|i| &a[i] + (&b[i] - &a[i]) * &t))
                    .collect::<Vec<P>>()
            };
            match super::super::rounding_legacy::prove_exact(&at(k), &at(k + 1), &triangles) {
                Ok(r) => pairs += r.pairs,
                Err(e) => {
                    #[cfg(test)]
                    eprintln!("Polynomial homotopy {n}:{k}: {}", e.message);
                    #[cfg(not(test))]
                    let _ = e;
                    ok = false;
                    break;
                }
            }
        }
        if ok {
            proved = Some((n, pairs));
            break;
        }
    }
    let (n, pairs) =
        proved.ok_or_else(|| fail("complete polynomial-atlas mesh homotopy is unproved"))?;
    Ok(Report {
        axis,
        polynomial: f.iter().map(|r| r.iter().map(ratio).collect()).collect(),
        source_chart_cells: cells,
        reference_triangles: triangles.len(),
        homotopy_segments: n,
        checked_pairs: pairs,
    })
}
pub(super) fn prove(s: &Source, nominal: &[Poly], work: &mut Work) -> Option<Report> {
    let Source::Brep(m) = s else {
        return None;
    };
    for axis in [2, 0, 1] {
        for face in &m.faces {
            let s = &face.surface;
            let [a, b] = intervals(s);
            if a.len() < 2 || b.len() < 2 {
                continue;
            }
            let result = (|| -> Result<Report> {
                let h = sb::cell(
                    s,
                    [a[0].clone(), a[1].clone()],
                    [b[0].clone(), b[1].clone()],
                )?;
                let f = candidate(&h, axis, work)?;
                attempt(m, nominal, axis, &f, work)
            })();
            match result {
                Ok(r) => return Some(r),
                Err(e) => {
                    #[cfg(test)]
                    eprintln!("Polynomial atlas axis {axis} unproved: {}", e.message);
                    #[cfg(not(test))]
                    let _ = e;
                }
            }
        }
    }
    None
}
impl Serialize for Report {
    fn to_value(&self) -> Value {
        json!({"kind":"polynomial-shear-ambient-isotopy","axis":self.axis,"polynomial":self.polynomial,"sourceChartCells":self.source_chart_cells,"referenceTriangles":self.reference_triangles,"homotopySegments":self.homotopy_segments,"checkedPairs":self.checked_pairs,"preservesComplement":true})
    }
}
impl<'de> Deserialize<'de> for Report {
    fn from_value(v: Value) -> value_codec::Result<Self> {
        let keys = [
            "kind",
            "axis",
            "polynomial",
            "sourceChartCells",
            "referenceTriangles",
            "homotopySegments",
            "checkedPairs",
            "preservesComplement",
        ];
        if v.as_object()
            .is_none_or(|o| o.len() != keys.len() || keys.iter().any(|k| !o.contains_key(*k)))
            || v["kind"].as_str() != Some("polynomial-shear-ambient-isotopy")
            || v["preservesComplement"].as_bool() != Some(true)
        {
            return Err(error("Polynomial ambient topology schema mismatch"));
        }
        Ok(Self {
            axis: value_codec::from_value(v["axis"].clone())?,
            polynomial: value_codec::from_value(v["polynomial"].clone())?,
            source_chart_cells: value_codec::from_value(v["sourceChartCells"].clone())?,
            reference_triangles: value_codec::from_value(v["referenceTriangles"].clone())?,
            homotopy_segments: value_codec::from_value(v["homotopySegments"].clone())?,
            checked_pairs: value_codec::from_value(v["checkedPairs"].clone())?,
        })
    }
}

#[cfg(test)]
#[path = "shear/tests/tests_integration.rs"]
mod tests;
