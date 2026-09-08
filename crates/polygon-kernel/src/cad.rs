//! Own CAD primitives and planar set operations. No external geometry backend.
use crate::{check, cross, norm, sub, Mesh, Result};
use std::collections::{BTreeSet, HashMap};
pub type Rings = Vec<Vec<[f64; 2]>>;
pub fn empty() -> Mesh {
    Mesh {
        positions: vec![],
        indices: vec![],
        uv: None,
    }
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
pub fn area(r: &[[f64; 2]]) -> f64 {
    (0..r.len())
        .map(|i| cross2(r[i], r[(i + 1) % r.len()]))
        .sum::<f64>()
        / 2.
}
fn winding(p: [f64; 2], rings: &Rings) -> i32 {
    let mut winding = 0;
    for r in rings {
        for i in 0..r.len() {
            let a = r[i];
            let b = r[(i + 1) % r.len()];
            let c = cross2(sub2(b, a), sub2(p, a));
            if a[1] <= p[1] && b[1] > p[1] && c > 0. {
                winding += 1
            }
            if a[1] > p[1] && b[1] <= p[1] && c < 0. {
                winding -= 1
            }
        }
    }
    winding
}
pub fn contains_point(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    inside(p, &vec![ring.to_vec()])
}
fn inside(p: [f64; 2], rings: &Rings) -> bool {
    winding(p, rings) != 0
}
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
/// Split an arrangement at intersections, classify its two sides and retain only
/// the result boundary. Both crossing and collinear overlapping segments split.
pub fn planar(a: &Rings, b: &Rings, op: &str) -> Result<Rings> {
    planar_rule(a, b, op, false)
}
pub fn nonzero(r: &Rings) -> Result<Rings> {
    planar_rule(r, &vec![], "union", true)
}
fn planar_rule(a: &Rings, b: &Rings, op: &str, nonzero: bool) -> Result<Rings> {
    let edges: Vec<_> = a
        .iter()
        .chain(b)
        .flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
        .collect();
    check(edges.len() <= 4096, "Planar arrangement exceeds 4096 edges")?;
    check(
        edges
            .iter()
            .all(|(a, b)| a.iter().chain(b).all(|x| x.is_finite() && x.abs() <= 1e6)),
        "Invalid planar coordinate",
    )?;
    let scale = edges
        .iter()
        .flat_map(|(a, b)| a.iter().chain(b))
        .fold(1_f64, |s, x| s.max(x.abs()));
    let eps = scale * 1e-9;
    let choose = |p| {
        let wa = winding(p, a);
        let wb = winding(p, b);
        let x = if nonzero { wa != 0 } else { wa > 0 };
        let y = if nonzero { wb != 0 } else { wb > 0 };
        match op {
            "intersection" => x && y,
            "difference" => x && !y,
            _ => x || y,
        }
    };
    let mut points = Vec::<[f64; 2]>::new();
    let mut ids = HashMap::new();
    let mut boundary = BTreeSet::new();
    for (i, &(p, q)) in edges.iter().enumerate() {
        let d = sub2(q, p);
        let l2 = d[0] * d[0] + d[1] * d[1];
        if l2 <= eps * eps {
            continue;
        }
        let mut ts = vec![0., 1.];
        for (j, &(r, s)) in edges.iter().enumerate() {
            if i == j {
                continue;
            }
            let e = sub2(s, r);
            let rp = sub2(r, p);
            let den = cross2(d, e);
            if den.abs() > eps * eps {
                let t = cross2(rp, e) / den;
                let u = cross2(rp, d) / den;
                if t > 0. && t < 1. && (-1e-10..=1. + 1e-10).contains(&u) {
                    ts.push(t)
                }
            } else if cross2(rp, d).abs() <= eps * l2.sqrt() {
                for v in [r, s] {
                    let v = sub2(v, p);
                    let t = (v[0] * d[0] + v[1] * d[1]) / l2;
                    if t > 0. && t < 1. {
                        ts.push(t)
                    }
                }
            }
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|a, b| (*a - *b).abs() < 1e-10);
        for t in ts.windows(2) {
            let u = [p[0] + d[0] * t[0], p[1] + d[1] * t[0]];
            let v = [p[0] + d[0] * t[1], p[1] + d[1] * t[1]];
            let len = ((v[0] - u[0]).powi(2) + (v[1] - u[1]).powi(2)).sqrt();
            if len < eps {
                continue;
            }
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
                *ids.entry(k).or_insert_with(|| {
                    points.push(v);
                    points.len() - 1
                })
            };
            let x = index(u);
            let y = index(v);
            if x != y {
                boundary.insert(if left { (x, y) } else { (y, x) });
            }
        }
    }
    let mut out = Vec::new();
    while let Some(&(start, next)) = boundary.iter().next() {
        let mut ring = vec![points[start]];
        boundary.remove(&(start, next));
        let mut at = next;
        let mut previous = start;
        while at != start {
            check(
                ring.len() <= edges.len() * 4,
                "Planar boundary exceeds budget",
            )?;
            ring.push(points[at]);
            let incoming = sub2(points[at], points[previous]);
            let candidates: Vec<_> = boundary.iter().filter(|(x, _)| *x == at).copied().collect();
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
            } else if join == "Square" && den * distance > 1e-12 {
                let bis = [na[0] + nb[0], na[1] + nb[1]];
                let len = bis[0].hypot(bis[1]);
                let bis = bis.map(|x| x / len);
                for (p, d) in [(u, a), (v, b)] {
                    let t = (distance - (p[0] - q[0]) * bis[0] - (p[1] - q[1]) * bis[1])
                        / (d[0] * bis[0] + d[1] * bis[1]);
                    out.push([p[0] + t * d[0], p[1] + t * d[1]]);
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
pub fn join(meshes: &[Mesh]) -> Result<Mesh> {
    let mut out = empty();
    for m in meshes {
        m.validate()?;
        let base = out.positions.len() / 3;
        out.positions.extend(&m.positions);
        out.indices.extend(m.indices.iter().map(|i| i + base));
    }
    out.validate()?;
    Ok(out)
}
pub fn extrude(
    rings: &Rings,
    height: f64,
    slices: usize,
    twist: f64,
    scale: [f64; 2],
    center: bool,
) -> Result<Mesh> {
    check(
        height.is_finite() && height > 0. && slices <= 513,
        "Invalid extrusion",
    )?;
    check(
        rings
            .iter()
            .map(Vec::len)
            .sum::<usize>()
            .saturating_mul(if twist != 0. || scale != [1., 1.] {
                slices.max(1)
            } else {
                1
            })
            .saturating_mul(2)
            <= 20000,
        "Extrusion triangle budget exceeded",
    )?;
    let mut pieces = Vec::new();
    for outer in rings.iter().filter(|r| area(r) > 0.) {
        let holes = rings
            .iter()
            .filter(|h| area(h) < 0. && inside(h[0], &vec![outer.clone()]))
            .cloned()
            .collect();
        let profile = crate::modeling::Profile {
            outer: outer.clone(),
            holes,
        };
        let mut m = triangulate(&profile)?.thicken([0., 0., height])?.mesh;
        if twist != 0. || scale != [1., 1.] {
            // Subdivide every vertical side consistently, retaining triangulated caps.
            let base = triangulate(&profile)?;
            let n = base.positions.len() / 3;
            let steps = slices.max(1);
            m = empty();
            for row in 0..=steps {
                let f = row as f64 / steps as f64;
                let a = (-twist * f).to_radians();
                for p in base.positions.chunks_exact(3) {
                    let x = p[0] * (1. + (scale[0] - 1.) * f);
                    let y = p[1] * (1. + (scale[1] - 1.) * f);
                    m.positions.extend([
                        x * a.cos() - y * a.sin(),
                        x * a.sin() + y * a.cos(),
                        height * f,
                    ]);
                }
            }
            for t in base.indices.chunks_exact(3) {
                m.indices.extend([
                    t[2],
                    t[1],
                    t[0],
                    t[0] + steps * n,
                    t[1] + steps * n,
                    t[2] + steps * n,
                ]);
            }
            for ring in base.boundary_loops()? {
                for row in 0..steps {
                    for i in 0..ring.len() - 1 {
                        let a = ring[i] + row * n;
                        let b = ring[i + 1] + row * n;
                        m.indices.extend([a, b, b + n, a, b + n, a + n]);
                    }
                }
            }
        }
        if center {
            for p in m.positions.chunks_exact_mut(3) {
                p[2] -= height / 2.
            }
        }
        pieces.push(m);
    }
    join(&pieces)
}
pub fn cube(size: [f64; 3], center: bool) -> Result<Mesh> {
    if size.contains(&0.) {
        return Ok(empty());
    }
    let r = vec![vec![
        [0., 0.],
        [size[0], 0.],
        [size[0], size[1]],
        [0., size[1]],
    ]];
    let mut m = extrude(&r, size[2], 1, 0., [1., 1.], false)?;
    if center {
        for p in m.positions.chunks_exact_mut(3) {
            for k in 0..3 {
                p[k] -= size[k] / 2.
            }
        }
    }
    Ok(m)
}
pub fn cylinder(height: f64, r1: f64, r2: f64, n: usize, center: bool) -> Result<Mesh> {
    check(
        (3..=512).contains(&n) && height > 0. && r1 >= 0. && r2 >= 0.,
        "Invalid cylinder",
    )?;
    let mut m = empty();
    for (r, z) in [(r1, 0.), (r2, height)] {
        for i in 0..n {
            let a = std::f64::consts::TAU * i as f64 / n as f64;
            m.positions.extend([
                r * a.cos(),
                r * a.sin(),
                z - if center { height / 2. } else { 0. },
            ]);
        }
    }
    for i in 0..n {
        let j = (i + 1) % n;
        m.indices.extend([i, j, j + n, i, j + n, i + n]);
    }
    for i in 1..n - 1 {
        m.indices.extend([0, i + 1, i, n, n + i, n + i + 1]);
    }
    clean(m)
}
pub fn sphere(radius: f64, n: usize) -> Result<Mesh> {
    check(radius > 0. && (4..=512).contains(&n), "Invalid sphere")?;
    let rows = (n / 2).max(2);
    let mut m = empty();
    for j in 0..=rows {
        let a = std::f64::consts::PI * j as f64 / rows as f64;
        for i in 0..n {
            let b = std::f64::consts::TAU * i as f64 / n as f64;
            let r = if j == 0 || j == rows {
                0.
            } else {
                radius * a.sin()
            };
            m.positions
                .extend([r * b.cos(), r * b.sin(), radius * a.cos()]);
        }
    }
    for j in 0..rows {
        for i in 0..n {
            let a = j * n + i;
            let b = j * n + (i + 1) % n;
            m.indices.extend([a, a + n, b + n, a, b + n, b]);
        }
    }
    clean(m)
}
pub fn clean(mut m: Mesh) -> Result<Mesh> {
    m = crate::proximity::weld_exact(&m)?;
    let mut indices = Vec::new();
    for t in m.indices.chunks_exact(3) {
        let a = m.point(t[0])?;
        let b = m.point(t[1])?;
        let c = m.point(t[2])?;
        if norm(&cross(sub(b, a), sub(c, a))) > 1e-16 {
            indices.extend(t)
        }
    }
    m.indices = indices;
    if !m.indices.is_empty() && m.inspect()?.signed_volume_mm3 < 0. {
        m.reverse_winding()
    }
    Ok(m)
}
pub fn hull3(meshes: &[Mesh]) -> Result<Mesh> {
    let mut p: Vec<[f64; 3]> = meshes
        .iter()
        .flat_map(|m| m.positions.chunks_exact(3).map(|p| [p[0], p[1], p[2]]))
        .collect();
    p.sort_by(|a, b| {
        a[0].total_cmp(&b[0])
            .then(a[1].total_cmp(&b[1]))
            .then(a[2].total_cmp(&b[2]))
    });
    p.dedup();
    check(p.len() <= 20000, "Hull point budget exceeded")?;
    if p.len() < 4 {
        return Ok(empty());
    }
    let a = 0;
    let b = p.len() - 1;
    let c = (0..p.len())
        .max_by(|&i, &j| {
            norm(&cross(sub(p[b], p[a]), sub(p[i], p[a])))
                .total_cmp(&norm(&cross(sub(p[b], p[a]), sub(p[j], p[a]))))
        })
        .unwrap();
    let n = cross(sub(p[b], p[a]), sub(p[c], p[a]));
    let d = (0..p.len())
        .max_by(|&i, &j| {
            dot(n, sub(p[i], p[a]))
                .abs()
                .total_cmp(&dot(n, sub(p[j], p[a])).abs())
        })
        .unwrap();
    if dot(n, sub(p[d], p[a])).abs() < 1e-15 {
        return Ok(empty());
    }
    let center = std::array::from_fn(|k| (p[a][k] + p[b][k] + p[c][k] + p[d][k]) / 4.);
    let orient = |mut f: [usize; 3]| {
        if dot(
            cross(sub(p[f[1]], p[f[0]]), sub(p[f[2]], p[f[0]])),
            sub(center, p[f[0]]),
        ) > 0.
        {
            f.swap(1, 2)
        }
        f
    };
    let mut faces = vec![
        orient([a, b, c]),
        orient([a, d, b]),
        orient([a, c, d]),
        orient([b, d, c]),
    ];
    let eps = norm(&sub(p[b], p[a])) * 1e-10;
    for i in 0..p.len() {
        let mut edge = BTreeSet::new();
        faces.retain(|f| {
            let normal = cross(sub(p[f[1]], p[f[0]]), sub(p[f[2]], p[f[0]]));
            if dot(normal, sub(p[i], p[f[0]])) > eps * norm(&normal) {
                for j in 0..3 {
                    let x = f[j];
                    let y = f[(j + 1) % 3];
                    if !edge.remove(&(y, x)) {
                        edge.insert((x, y));
                    }
                }
                false
            } else {
                true
            }
        });
        for (x, y) in edge {
            faces.push(orient([x, y, i]));
        }
        check(faces.len() <= 20000, "Hull face budget exceeded")?;
    }
    clean(Mesh {
        positions: p.into_iter().flatten().collect(),
        indices: faces.into_iter().flatten().collect(),
        uv: None,
    })
}

pub fn project(mesh: &Mesh) -> Result<Rings> {
    let mut triangles = Vec::new();
    for t in mesh.indices.chunks_exact(3) {
        let mut r: Vec<_> = t
            .iter()
            .map(|&i| {
                let p = mesh.point(i).unwrap();
                [p[0], p[1]]
            })
            .collect();
        if area(&r).abs() < 1e-12 {
            continue;
        }
        if area(&r) < 0. {
            r.reverse()
        }
        triangles.push(r)
    }
    planar(&triangles, &vec![], "union")
}
pub fn slice(mesh: &Mesh, z: f64) -> Result<Rings> {
    check(z.is_finite(), "Invalid slice height")?;
    let eps = 1e-8;
    let mut points = Vec::new();
    let mut ids = HashMap::new();
    let mut edges = BTreeSet::new();
    for t in mesh.indices.chunks_exact(3) {
        let p = [mesh.point(t[0])?, mesh.point(t[1])?, mesh.point(t[2])?];
        let n = cross(sub(p[1], p[0]), sub(p[2], p[0]));
        let mut hits = Vec::new();
        for i in 0..3 {
            let a = p[i];
            let b = p[(i + 1) % 3];
            if (a[2] > z) != (b[2] > z) {
                let f = (z - a[2]) / (b[2] - a[2]);
                let h = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
                if hits
                    .iter()
                    .all(|v: &[f64; 2]| (v[0] - h[0]).hypot(v[1] - h[1]) > eps)
                {
                    hits.push(h)
                }
            }
        }
        if hits.len() != 2 {
            continue;
        }
        if (hits[1][0] - hits[0][0]) * (-n[1]) + (hits[1][1] - hits[0][1]) * n[0] < 0. {
            hits.swap(0, 1)
        }
        let mut index = |p: [f64; 2]| {
            *ids.entry(((p[0] / eps).round() as i64, (p[1] / eps).round() as i64))
                .or_insert_with(|| {
                    points.push(p);
                    points.len() - 1
                })
        };
        let a = index(hits[0]);
        let b = index(hits[1]);
        if a != b {
            edges.insert((a, b));
        }
    }
    let mut rings = vec![];
    while let Some(&(start, end)) = edges.iter().next() {
        edges.remove(&(start, end));
        let mut ring = vec![points[start]];
        let mut at = end;
        while at != start {
            ring.push(points[at]);
            check(ring.len() <= mesh.indices.len(), "Slice boundary budget")?;
            let next = edges.iter().find(|e| e.0 == at).copied();
            check(next.is_some(), "Open or ambiguous slice boundary")?;
            let edge = next.unwrap();
            edges.remove(&edge);
            at = edge.1;
        }
        if ring.len() >= 3 {
            rings.push(ring)
        }
    }
    planar(&rings, &vec![], "union")
}
pub fn minkowski(a: &Mesh, b: &Mesh) -> Result<Mesh> {
    fn vertices(m: &Mesh) -> Vec<[f64; 3]> {
        let ids: BTreeSet<_> = m.indices.iter().copied().collect();
        ids.into_iter().map(|i| m.point(i).unwrap()).collect()
    }
    let a_points = vertices(a);
    let b_points = vertices(b);
    check(
        a_points.len().saturating_mul(b_points.len()) <= 20000,
        "Minkowski vertex budget exceeded",
    )?;
    for (m, p) in [(a, &a_points), (b, &b_points)] {
        for t in m.indices.chunks_exact(3) {
            let u = m.point(t[0])?;
            let n = cross(sub(m.point(t[1])?, u), sub(m.point(t[2])?, u));
            check(
                p.iter().all(|&v| dot(n, sub(v, u)) <= norm(&n) * 1e-8),
                "Own 3D Minkowski currently requires convex operands",
            )?;
        }
    }
    let mut mesh = empty();
    for a in a_points {
        for b in &b_points {
            mesh.positions
                .extend([a[0] + b[0], a[1] + b[1], a[2] + b[2]])
        }
    }
    hull3(&[mesh])
}

/// Collapse coplanar triangulation introduced by successive CSG operations.
/// Boundary rings (including holes) remain authoritative; no shape approximation.
pub fn simplify(mesh: &Mesh) -> Result<Mesh> {
    let mut groups: Vec<([f64; 3], f64, Vec<usize>)> = vec![];
    let extent = mesh.positions.iter().fold(1_f64, |a, b| a.max(b.abs()));
    let eps = extent * 1e-9;
    for t in mesh.indices.chunks_exact(3) {
        let a = mesh.point(t[0])?;
        let normal = cross(sub(mesh.point(t[1])?, a), sub(mesh.point(t[2])?, a));
        let len = norm(&normal);
        if len < 1e-16 {
            continue;
        }
        let n = normal.map(|v| v / len);
        let d = dot(n, a);
        if let Some(g) = groups
            .iter_mut()
            .find(|(m, k, _)| dot(n, *m) > 1. - 1e-12 && (d - k).abs() < eps)
        {
            g.2.extend(t)
        } else {
            groups.push((n, d, t.to_vec()))
        }
    }
    let mut pieces = vec![];
    for (n, _, indices) in groups {
        let part = Mesh {
            positions: mesh.positions.clone(),
            indices,
            uv: None,
        };
        let loops = match part.boundary_loops() {
            Ok(loops) => loops,
            Err(_) => {
                pieces.push(part);
                continue;
            }
        };
        if loops.is_empty() {
            continue;
        }
        let origin = part.point(loops[0][0])?;
        let delta = sub(part.point(loops[0][1])?, origin);
        let len = norm(&delta);
        let u = delta.map(|v| v / len);
        let v = cross(n, u);
        let mut rings: Rings = loops
            .into_iter()
            .map(|ring| {
                ring[..ring.len() - 1]
                    .iter()
                    .map(|&i| {
                        let p = sub(part.point(i).unwrap(), origin);
                        [dot(p, u), dot(p, v)]
                    })
                    .collect()
            })
            .collect();
        for r in &mut rings {
            loop {
                if r.len() <= 3 {
                    break;
                }
                let count = r.len();
                let at = (0..count).find(|&i| {
                    let a = sub2(r[i], r[(i + count - 1) % count]);
                    let b = sub2(r[(i + 1) % count], r[i]);
                    cross2(a, b).abs() < eps * (a[0].hypot(a[1]) + b[0].hypot(b[1]))
                });
                if let Some(i) = at {
                    r.remove(i);
                } else {
                    break;
                }
            }
        }
        for outer in rings.iter().filter(|r| area(r) > 0.) {
            let holes = rings
                .iter()
                .filter(|r| area(r) < 0. && inside(r[0], &vec![outer.clone()]))
                .cloned()
                .collect();
            let mut cap = triangulate(&crate::modeling::Profile {
                outer: outer.clone(),
                holes,
            })?;
            for p in cap.positions.chunks_exact_mut(3) {
                let point =
                    std::array::from_fn::<_, 3, _>(|k| origin[k] + p[0] * u[k] + p[1] * v[k]);
                p.copy_from_slice(&point)
            }
            pieces.push(cap);
        }
    }
    let mut out = join(&pieces)?;
    let mut cells = HashMap::<[i64; 3], Vec<usize>>::new();
    let mut positions = vec![];
    let mut remap = vec![];
    for p in out.positions.chunks_exact(3) {
        let p = [p[0], p[1], p[2]];
        let cell = p.map(|x| (x / eps).floor() as i64);
        let mut found = None;
        'near: for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    if let Some(ids) = cells.get(&[cell[0] + x, cell[1] + y, cell[2] + z]) {
                        for &i in ids {
                            let q = [positions[3 * i], positions[3 * i + 1], positions[3 * i + 2]];
                            if norm(&sub(p, q)) < eps {
                                found = Some(i);
                                break 'near;
                            }
                        }
                    }
                }
            }
        }
        let i = found.unwrap_or_else(|| {
            let i = positions.len() / 3;
            positions.extend(p);
            cells.entry(cell).or_default().push(i);
            i
        });
        remap.push(i)
    }
    for i in &mut out.indices {
        *i = remap[*i]
    }
    out.positions = positions;
    if out.inspect()?.closed {
        Ok(out)
    } else {
        crate::boolean::stitch_mesh(&out, eps)
    }
}

/// Constrained ear clipping with visible bridges for holes. It reuses all input
/// boundary coordinates, so neighboring solid faces acquire no extra T junctions.
pub fn triangulate(profile: &crate::modeling::Profile) -> Result<Mesh> {
    let mut m = empty();
    let mut ring: Vec<usize> = vec![];
    for p in &profile.outer {
        ring.push(m.positions.len() / 3);
        m.positions.extend([p[0], p[1], 0.]);
    }
    if area(&profile.outer) < 0. {
        ring.reverse()
    }
    let point = |m: &Mesh, i: usize| [m.positions[3 * i], m.positions[3 * i + 1]];
    let mut holes = profile.holes.clone();
    holes.sort_by(|a, b| {
        a.iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min)
            .total_cmp(&b.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min))
    });
    for mut h in holes {
        if area(&h) > 0. {
            h.reverse()
        }
        let start = (0..h.len())
            .min_by(|&a, &b| {
                h[a][0]
                    .total_cmp(&h[b][0])
                    .then(h[a][1].total_cmp(&h[b][1]))
            })
            .unwrap();
        h.rotate_left(start);
        let hp = h[0];
        let mut options: Vec<_> = (0..ring.len()).collect();
        options.sort_by(|&i, &j| {
            let a = point(&m, ring[i]);
            let b = point(&m, ring[j]);
            (a[0] - hp[0])
                .hypot(a[1] - hp[1])
                .total_cmp(&(b[0] - hp[0]).hypot(b[1] - hp[1]))
        });
        let index = options
            .into_iter()
            .find(|&i| {
                let p = point(&m, ring[i]);
                let d = sub2(hp, p);
                let midpoint = [(p[0] + hp[0]) / 2., (p[1] + hp[1]) / 2.];
                if !inside(midpoint, &vec![profile.outer.clone()])
                    || profile
                        .holes
                        .iter()
                        .any(|hole| inside(midpoint, &vec![hole.clone()]))
                {
                    return false;
                }
                for boundary in std::iter::once(&profile.outer).chain(profile.holes.iter()) {
                    for k in 0..boundary.len() {
                        let a = boundary[k];
                        let b = boundary[(k + 1) % boundary.len()];
                        let e = sub2(b, a);
                        let den = cross2(d, e);
                        if den.abs() > 1e-14 {
                            let t = cross2(sub2(a, p), e) / den;
                            let u = cross2(sub2(a, p), d) / den;
                            if t > 1e-10 && t < 1. - 1e-10 && u > 1e-10 && u < 1. - 1e-10 {
                                return false;
                            }
                        }
                    }
                }
                true
            })
            .ok_or_else(|| crate::Error::new("No visible bridge for profile hole"))?;
        let mut ids = Vec::new();
        for p in h {
            ids.push(m.positions.len() / 3);
            m.positions.extend([p[0], p[1], 0.])
        }
        ids.push(ids[0]);
        ids.push(ring[index]);
        ring.splice(index + 1..index + 1, ids);
    }
    check(ring.len() <= 2048, "Profile triangulation budget exceeded")?;
    let mut remaining = ring;
    let eps = 1e-12;
    while remaining.len() > 3 {
        let n = remaining.len();
        let mut ear = None;
        for i in 0..n {
            let a = remaining[(i + n - 1) % n];
            let b = remaining[i];
            let c = remaining[(i + 1) % n];
            let pa = point(&m, a);
            let pb = point(&m, b);
            let pc = point(&m, c);
            if cross2(sub2(pb, pa), sub2(pc, pb)) <= eps {
                continue;
            }
            let blocked = remaining.iter().any(|&v| {
                let p = point(&m, v);
                if p == pa || p == pb || p == pc {
                    return false;
                }
                cross2(sub2(pb, pa), sub2(p, pa)) >= -eps
                    && cross2(sub2(pc, pb), sub2(p, pb)) >= -eps
                    && cross2(sub2(pa, pc), sub2(p, pc)) >= -eps
            });
            if !blocked {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        if let Some((i, t)) = ear {
            m.indices.extend(t);
            remaining.remove(i);
        } else {
            return Err(crate::Error::new(
                "Profile cannot be triangulated without crossing its boundary",
            ));
        }
    }
    if remaining.len() == 3 {
        m.indices.extend(remaining)
    }
    m.validate()?;
    Ok(m)
}

pub fn halfspace(mesh: &Mesh, normal: [f64; 3], offset: f64) -> Result<Mesh> {
    let length = norm(&normal);
    check(length > 0. && offset.is_finite(), "Invalid cutting plane")?;
    let n = normal.map(|x| x / length);
    let a = if n[0].abs() < 0.8 {
        [1., 0., 0.]
    } else {
        [0., 1., 0.]
    };
    let u = cross(n, a);
    let ul = norm(&u);
    let u = u.map(|x| x / ul);
    let v = cross(n, u);
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for p in mesh.positions.chunks_exact(3) {
        for k in 0..3 {
            min[k] = min[k].min(p[k]);
            max[k] = max[k].max(p[k])
        }
    }
    let c = std::array::from_fn(|k| (min[k] + max[k]) / 2.);
    let distance = offset - dot(c, n);
    let origin = std::array::from_fn::<_, 3, _>(|k| c[k] + distance * n[k]);
    let extent = norm(&sub(max, min)) + distance.abs() + 1.;
    let mut box_mesh = cube([2. * extent, 2. * extent, extent], false)?;
    for p in box_mesh.positions.chunks_exact_mut(3) {
        let q = std::array::from_fn::<_, 3, _>(|k| {
            origin[k] + (p[0] - extent) * u[k] + (p[1] - extent) * v[k] + p[2] * n[k]
        });
        p.copy_from_slice(&q)
    }
    Ok(box_mesh)
}

/// Exact specialization for vertical prisms with coincident end planes. Reduces
/// repeated drilled-hole CSG to a planar arrangement and one triangulation.
pub fn prism_boolean(a: &Mesh, b: &Mesh, operation: &str) -> Result<Option<Mesh>> {
    fn bounds(m: &Mesh) -> Option<(f64, f64)> {
        if m.indices.is_empty() {
            return None;
        }
        let lo = m
            .positions
            .chunks_exact(3)
            .map(|p| p[2])
            .fold(f64::INFINITY, f64::min);
        let hi = m
            .positions
            .chunks_exact(3)
            .map(|p| p[2])
            .fold(f64::NEG_INFINITY, f64::max);
        let eps = (hi - lo).abs().max(1.) * 1e-9;
        if hi - lo <= eps {
            return None;
        }
        for t in m.indices.chunks_exact(3) {
            let a = m.point(t[0]).ok()?;
            let b = m.point(t[1]).ok()?;
            let c = m.point(t[2]).ok()?;
            let n = cross(sub(b, a), sub(c, a));
            if n[2].abs() > norm(&n) * 1e-9
                && (![a, b, c].iter().all(|p| (p[2] - lo).abs() < eps)
                    && ![a, b, c].iter().all(|p| (p[2] - hi).abs() < eps))
            {
                return None;
            }
        }
        Some((lo, hi))
    }
    let (Some((lo, hi)), Some((other_lo, other_hi))) = (bounds(a), bounds(b)) else {
        return Ok(None);
    };
    if (lo - other_lo).abs() > 1e-9 || (hi - other_hi).abs() > 1e-9 {
        return Ok(None);
    }
    let pa = slice(a, (lo + hi) / 2.)?;
    let pb = slice(b, (lo + hi) / 2.)?;
    let p = planar(&pa, &pb, operation)?;
    let mut result = extrude(&p, hi - lo, 1, 0., [1., 1.], false)?;
    for v in result.positions.chunks_exact_mut(3) {
        v[2] += lo
    }
    Ok(Some(result))
}

/// Join solids stacked on a shared horizontal cap. Boundary equivalence is
/// checked explicitly; overlapping bounds alone never select this path.
pub fn join_touching(a: &Mesh, b: &Mesh) -> Result<Option<Mesh>> {
    if a.indices.is_empty() || b.indices.is_empty() {
        return Ok(None);
    }
    let range = |m: &Mesh| {
        (
            m.positions
                .chunks_exact(3)
                .map(|p| p[2])
                .fold(f64::INFINITY, f64::min),
            m.positions
                .chunks_exact(3)
                .map(|p| p[2])
                .fold(f64::NEG_INFINITY, f64::max),
        )
    };
    let (al, ah) = range(a);
    let (bl, bh) = range(b);
    let eps = 1e-8;
    let z = if (ah - bl).abs() < eps {
        ah
    } else if (bh - al).abs() < eps {
        al
    } else {
        return Ok(None);
    };
    type CapBoundary = BTreeSet<([i64; 2], [i64; 2])>;
    let cap = |m: &Mesh| -> Result<Option<(Mesh, CapBoundary)>> {
        let mut rest = m.clone();
        let mut top = empty();
        top.positions = m.positions.clone();
        rest.indices.clear();
        for t in m.indices.chunks_exact(3) {
            if t.iter().all(|&i| (m.positions[i * 3 + 2] - z).abs() < eps) {
                top.indices.extend(t)
            } else {
                rest.indices.extend(t)
            }
        }
        if top.indices.is_empty() {
            return Ok(None);
        }
        let mut edges = BTreeSet::new();
        for ring in top.boundary_loops()? {
            for edge in ring.windows(2) {
                let key = |i: usize| {
                    [
                        (m.positions[i * 3] / eps).round() as i64,
                        (m.positions[i * 3 + 1] / eps).round() as i64,
                    ]
                };
                let x = key(edge[0]);
                let y = key(edge[1]);
                edges.insert(if x < y { (x, y) } else { (y, x) });
            }
        }
        Ok(Some((rest, edges)))
    };
    let (Some((left, le)), Some((right, re))) = (cap(a)?, cap(b)?) else {
        return Ok(None);
    };
    if le != re {
        return Ok(None);
    }
    if !a.inspect()?.closed || !b.inspect()?.closed {
        return Ok(None);
    }
    let mut out = join(&[left, right])?;
    let mut shared = HashMap::new();
    let mut remap: Vec<_> = (0..out.positions.len() / 3).collect();
    for (i, p) in out.positions.chunks_exact(3).enumerate() {
        if (p[2] - z).abs() < eps {
            let key = [(p[0] / eps).round() as i64, (p[1] / eps).round() as i64];
            remap[i] = *shared.entry(key).or_insert(i);
        }
    }
    for i in &mut out.indices {
        *i = remap[*i]
    }
    if out.inspect()?.closed {
        Ok(Some(out))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn primitives_closed() {
        for m in [
            cube([2., 3., 4.], false).unwrap(),
            sphere(3., 24).unwrap(),
            cylinder(4., 2., 0., 24, false).unwrap(),
        ] {
            let r = m.inspect().unwrap();
            assert!(r.closed, "{r:?}");
            assert_eq!(r.degenerate_triangles, 0);
            assert!(r.signed_volume_mm3 > 0.)
        }
    }
    #[test]
    fn planar_overlap_and_hole() {
        let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        let b = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
        let r = planar(&a, &b, "difference").unwrap();
        assert!((r.iter().map(|r| area(r)).sum::<f64>() - 12.).abs() < 1e-8);
        let m = extrude(&r, 2., 1, 0., [1., 1.], false).unwrap();
        assert!((m.inspect().unwrap().signed_volume_mm3 - 24.).abs() < 1e-8);
    }
    #[test]
    fn hull_box() {
        let m = cube([2., 3., 4.], false).unwrap();
        let h = hull3(&[m]).unwrap();
        let r = h.inspect().unwrap();
        assert!(r.closed);
        assert!((r.signed_volume_mm3 - 24.).abs() < 1e-8)
    }
}
