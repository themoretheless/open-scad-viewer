//! Checked rational homotopy tubes. No nonzero tangent or simple-profile premise.
use super::*;
type Net = Vec<[Q; 3]>;
fn point(p: &[Q; 3]) -> Point {
    [&p[0] / &p[2], &p[1] / &p[2]]
}
fn straight(n: &Net) -> bool {
    let a = point(&n[0]);
    let b = point(n.last().unwrap());
    let e = [&b[0] - &a[0], &b[1] - &a[1]];
    if e.iter().all(Zero::is_zero) {
        // A closed retracing line contributes zero winding, even when its
        // controls leave the endpoint. Removing it preserves the filled set.
        let Some(b) = n.iter().map(point).find(|p| *p != a) else {
            return true;
        };
        let e = [&b[0] - &a[0], &b[1] - &a[1]];
        return n.iter().all(|p| {
            polygon::cross(&e, &[&p[0] - &a[0] * &p[2], &p[1] - &a[1] * &p[2]]).is_zero()
        });
    }
    n.iter()
        .all(|p| polygon::cross(&e, &[&p[0] - &a[0] * &p[2], &p[1] - &a[1] * &p[2]]).is_zero())
}
fn flat(n: &Net, eps: &Q) -> bool {
    if straight(n) {
        return true;
    }
    let p = n.len() - 1;
    let a = point(&n[0]);
    let b = point(n.last().unwrap());
    // Elevate homogeneous N,W to degree p+1 and multiply W by the chord.
    for i in 0..=p + 1 {
        let left = Q::new(i.into(), (p + 1).into());
        let right = Q::one() - &left;
        let h: [Q; 3] = std::array::from_fn(|k| {
            let x = if i > 0 {
                &n[i - 1][k] * &left
            } else {
                Q::zero()
            };
            let y = if i <= p { &n[i][k] * &right } else { Q::zero() };
            x + y
        });
        let error = (0..2).fold(Q::zero(), |sum, k| {
            let chord = (if i <= p {
                &n[i][2] * &a[k] * &right
            } else {
                Q::zero()
            }) + (if i > 0 {
                &n[i - 1][2] * &b[k] * &left
            } else {
                Q::zero()
            });
            sum + (&h[k] - chord).pow(2)
        });
        if error > eps.pow(2) * h[2].pow(2) {
            return false;
        }
    }
    true
}
fn descend(n: &Net, path: &str) -> Net {
    let mut n = n.clone();
    for c in path.bytes() {
        let (a, b) = split(&n);
        n = if c == b'0' { a } else { b };
    }
    n
}
fn full_cover(paths: &[String]) -> Result<()> {
    fn visit(paths: &[String], index: &mut usize, prefix: &str) -> Result<()> {
        if prefix.len() > 24 || *index >= paths.len() {
            return Err(refused("Minkowski subdivision coverage is incomplete"));
        }
        if paths[*index] == prefix {
            *index += 1;
            return Ok(());
        }
        if !paths[*index].starts_with(prefix) {
            return Err(refused("Minkowski subdivision coverage is reordered"));
        }
        visit(paths, index, &format!("{prefix}0"))?;
        visit(paths, index, &format!("{prefix}1"))
    }
    if paths.len() > 512
        || paths
            .iter()
            .any(|p| p.len() > 24 || p.bytes().any(|b| b != b'0' && b != b'1'))
    {
        return Err(refused("Minkowski subdivision witness size exceeded"));
    }
    let mut i = 0;
    visit(paths, &mut i, "")?;
    if i != paths.len() {
        return Err(refused(
            "Minkowski subdivision has duplicate or extra leaves",
        ));
    }
    Ok(())
}
pub(super) fn produce(loops: &[Vec<Curve>], eps: &Q) -> Result<Vec<Vec<String>>> {
    source_bits(loops, 1e-7)?;
    let nets = extraction::nets(loops)?;
    let mut paths = Vec::new();
    let mut count = 0;
    for (_, net) in nets {
        let mut leaves = Vec::new();
        let mut stack = vec![(net, String::new())];
        while let Some((n, p)) = stack.pop() {
            if flat(&n, eps) {
                leaves.push(p);
                count += 1;
                if count > 512 {
                    return Err(refused(
                        "Minkowski curve approximation leaf budget exceeded",
                    ));
                }
            } else {
                if p.len() >= 24 || eps.is_zero() {
                    return Err(refused(
                        "Minkowski curve approximation cannot meet its bound",
                    ));
                }
                let (a, b) = split(&n);
                stack.push((b, format!("{p}1")));
                stack.push((a, format!("{p}0")));
            }
        }
        paths.push(leaves);
    }
    Ok(paths)
}
#[derive(Clone)]
pub(super) struct Bound {
    pub nominal: Rings,
    pub inner: Rings,
    pub outer: Rings,
    pub exact: bool,
}
pub(super) fn verify(loops: &[Vec<Curve>], eps: &Q, paths: &[Vec<String>]) -> Result<Bound> {
    source_bits(loops, 1e-7)?;
    let nets = extraction::nets(loops)?;
    if paths.len() != nets.len() || paths.iter().map(Vec::len).sum::<usize>() > 512 {
        return Err(refused("Minkowski source span census mismatch"));
    }
    let mut rings = vec![Vec::new(); loops.len()];
    let mut first = vec![None; loops.len()];
    let mut last = vec![None; loops.len()];
    let mut tubes = Vec::new();
    for ((wire, net), leaves) in nets.iter().zip(paths) {
        let a = point(&net[0]);
        let b = point(net.last().unwrap());
        if last[*wire].as_ref().is_some_and(|p| p != &a) {
            return Err(refused(
                "Minkowski source has an open or discontinuous seam",
            ));
        }
        if first[*wire].is_none() {
            first[*wire] = Some(a);
        }
        last[*wire] = Some(b);
        full_cover(leaves)?;
        for path in leaves {
            let n = descend(net, path);
            if !flat(&n, eps) {
                return Err(refused("Minkowski source-to-chord bound is false"));
            }
            let a = point(&n[0]);
            let b = point(n.last().unwrap());
            if rings[*wire].last() != Some(&a) {
                rings[*wire].push(a.clone());
            }
            if !straight(&n) {
                let vertices = [a, b]
                    .iter()
                    .flat_map(|p| {
                        [-1i64, 1].into_iter().flat_map(move |x| {
                            [-1i64, 1].into_iter().map(move |y| {
                                [
                                    &p[0] + eps * Q::from_integer(x.into()),
                                    &p[1] + eps * Q::from_integer(y.into()),
                                ]
                            })
                        })
                    })
                    .collect();
                tubes.push(polygon::hull(vertices));
            }
        }
    }
    if first.iter().zip(&last).any(|(a, b)| a != b || a.is_none()) {
        return Err(refused(
            "Minkowski filled-region source is not exactly closed",
        ));
    }
    for r in &mut rings {
        if r.first() == r.last() {
            r.pop();
        }
    }
    rings.retain(|r| r.len() >= 3);
    let nominal = arrangement::union(&rings)?;
    let exact = tubes.is_empty();
    if exact {
        return Ok(Bound {
            inner: nominal.clone(),
            outer: nominal.clone(),
            nominal,
            exact,
        });
    }
    let tube = arrangement::union(&tubes)?;
    let inner = arrangement::boolean(&nominal, &tube, "difference")?;
    let outer = arrangement::boolean(&nominal, &tube, "union")?;
    Ok(Bound {
        nominal,
        inner,
        outer,
        exact,
    })
}
pub(super) fn disk(radius: f64, eps: &Q) -> Result<(Rings, Rings)> {
    let r = Q::from_float(radius).unwrap();
    if r <= Q::zero() || eps <= &Q::zero() {
        return Err(refused("A positive disk needs a positive enclosure bound"));
    }
    for count in [8usize, 16, 32, 64, 128, 256] {
        let slack = (eps.to_f64().unwrap() * 0.125).min(radius * 0.01);
        let inner_radius = (radius - slack).max(radius * 0.5);
        let outer_radius = (radius + slack) / ((std::f64::consts::PI / count as f64).cos());
        let ring = |rr: f64| {
            // Preserve exact quarter-turn/reflection symmetries in stored bits;
            // near-zero cardinal coordinates must not create microscopic cells.
            let octant: Vec<_> = (0..=count / 8)
                .map(|i| {
                    let t = std::f64::consts::TAU * i as f64 / count as f64;
                    let x = rr * t.cos();
                    let y = if i == 0 {
                        0.
                    } else if i == count / 8 {
                        x
                    } else {
                        rr * t.sin()
                    };
                    [x, y]
                })
                .collect();
            polygon::hull(
                (0..count)
                    .map(|i| {
                        let q = i / (count / 4);
                        let j = i % (count / 4);
                        let p = if j <= count / 8 {
                            octant[j]
                        } else {
                            let p = octant[count / 4 - j];
                            [p[1], p[0]]
                        };
                        let p = match q {
                            0 => p,
                            1 => [-p[1], p[0]],
                            2 => [-p[0], -p[1]],
                            _ => [p[1], -p[0]],
                        };
                        p.map(|v| Q::from_float(v).unwrap())
                    })
                    .collect(),
            )
        };
        let a = ring(inner_radius);
        let b = ring(outer_radius);
        if disk_check(&a, &r, eps, false) && disk_check(&b, &r, eps, true) {
            return Ok((vec![a], vec![b]));
        }
    }
    Err(refused(
        "Minkowski disk enclosure exceeds its polygon budget",
    ))
}
fn disk_check(p: &[Point], r: &Q, eps: &Q, outer: bool) -> bool {
    if p.len() < 3 {
        return false;
    }
    let min = if outer {
        r.clone()
    } else {
        (r - eps).max(Q::zero())
    };
    let max = if outer { r + eps } else { r.clone() };
    if p.iter().any(|v| v[0].pow(2) + v[1].pow(2) > max.pow(2)) {
        return false;
    }
    for i in 0..p.len() {
        let a = &p[i];
        let b = &p[(i + 1) % p.len()];
        let edge = [&b[0] - &a[0], &b[1] - &a[1]];
        let c = polygon::cross(a, b);
        if c <= Q::zero() || c.pow(2) < min.pow(2) * (edge[0].pow(2) + edge[1].pow(2)) {
            return false;
        }
    }
    true
}

pub(super) fn verify_disk(inner: &Rings, outer: &Rings, radius: &Q, eps: &Q) -> Result<()> {
    if inner.len() != 1
        || outer.len() != 1
        || polygon::hull(inner[0].clone()) != inner[0]
        || polygon::hull(outer[0].clone()) != outer[0]
        || !disk_check(&inner[0], radius, eps, false)
        || !disk_check(&outer[0], radius, eps, true)
    {
        return Err(refused("Minkowski disk inner/outer enclosure is false"));
    }
    Ok(())
}
