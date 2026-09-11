//! Closed rings: area, winding, planar boolean, offset.
use crate::{check, Result};
use std::collections::{BTreeSet, HashMap};

pub type Rings = Vec<Vec<[f64; 2]>>;

pub(crate) fn cross2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[1] - a[1] * b[0]
}
pub(crate) fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
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
pub(crate) fn inside(p: [f64; 2], rings: &Rings) -> bool {
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
            "xor" | "exclude" => x != y,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planar_overlap_and_hole() {
        let a = vec![vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]]];
        let b = vec![vec![[1., 1.], [3., 1.], [3., 3.], [1., 3.]]];
        let r = planar(&a, &b, "difference").unwrap();
        assert!((r.iter().map(|r| area(r)).sum::<f64>() - 12.).abs() < 1e-8);
        let m = crate::solid::modeling::extrude_rings(&r, 2., 1, 0., [1., 1.], false).unwrap();
        assert!((m.inspect().unwrap().signed_volume_mm3 - 24.).abs() < 1e-8);
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
