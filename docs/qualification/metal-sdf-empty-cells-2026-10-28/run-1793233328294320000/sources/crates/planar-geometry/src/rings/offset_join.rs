use super::*;

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
            } else if (join == "Bevel" || join == "Square") && den * distance > 1e-12 {
                // Bevel: keep both offset endpoints (flat cut). Square extends
                // further along the bisector like SVG stroke-linejoin:square.
                if join == "Square" {
                    let bis = [na[0] + nb[0], na[1] + nb[1]];
                    let len = bis[0].hypot(bis[1]);
                    let bis = bis.map(|x| x / len);
                    for (p, d) in [(u, a), (v, b)] {
                        let t = (distance - (p[0] - q[0]) * bis[0] - (p[1] - q[1]) * bis[1])
                            / (d[0] * bis[0] + d[1] * bis[1]);
                        out.push([p[0] + t * d[0], p[1] + t * d[1]]);
                    }
                } else {
                    out.push(u);
                    out.push(v);
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

pub fn from_outer_holes(outer: Vec<[f64; 2]>, holes: Vec<Vec<[f64; 2]>>) -> Rings {
    std::iter::once(outer).chain(holes).collect()
}

pub fn work(rings: &Rings) -> usize {
    rings.iter().map(Vec::len).sum()
}

fn area2(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn even_odd(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut inside = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}

/// Profile contract used by SDF extrude/revolve: first ring is outer, rest are holes.
/// Validate bounded, simple, mutually disjoint boundaries without imposing one outer ring.
pub fn validate_contours(rings: &Rings) -> Result<()> {
    if rings.is_empty()
        || rings.len() > 17
        || work(rings) > 512
        || rings
            .iter()
            .any(|l| l.len() < 3 || l.iter().flatten().any(|v| !v.is_finite() || v.abs() > 1e6))
    {
        return Err(crate::error("Invalid planar region"));
    }
    let mut edges = Vec::new();
    for (li, l) in rings.iter().enumerate() {
        let loop_area = (1..l.len() - 1)
            .map(|i| area2(l[0], l[i], l[i + 1]))
            .sum::<f64>();
        if loop_area.abs() < 1e-14 {
            return Err(crate::error("Zero-area profile loop"));
        }
        for i in 0..l.len() {
            let a = l[i];
            let b = l[(i + 1) % l.len()];
            if a == b {
                return Err(crate::error("Repeated profile vertex"));
            }
            edges.push((li, i, a, b));
        }
    }
    let on = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        area2(a, b, p).abs() <= 1e-12
            && p[0] >= a[0].min(b[0]) - 1e-12
            && p[0] <= a[0].max(b[0]) + 1e-12
            && p[1] >= a[1].min(b[1]) - 1e-12
            && p[1] <= a[1].max(b[1]) + 1e-12
    };
    for i in 0..edges.len() {
        for j in i + 1..edges.len() {
            let (li, ai, a, b) = edges[i];
            let (lj, bi, c, d) = edges[j];
            if li == lj && ((ai + 1) % rings[li].len() == bi || (bi + 1) % rings[li].len() == ai) {
                continue;
            }
            if on(a, c, d)
                || on(b, c, d)
                || on(c, a, b)
                || on(d, a, b)
                || (area2(a, b, c) > 0.) != (area2(a, b, d) > 0.)
                    && (area2(c, d, a) > 0.) != (area2(c, d, b) > 0.)
            {
                return Err(crate::error("Profile loops intersect or touch"));
            }
        }
    }
    Ok(())
}

pub fn validate_profile(rings: &Rings) -> Result<()> {
    validate_contours(rings)?;
    let outer = &rings[0];
    for (i, h) in rings[1..].iter().enumerate() {
        if !even_odd(h[0], outer)
            || rings[1..]
                .iter()
                .enumerate()
                .any(|(j, other)| i != j && even_odd(h[0], other))
        {
            return Err(crate::error("Invalid hole containment"));
        }
    }
    Ok(())
}

pub fn signed_distance(rings: &Rings, p: [f64; 2]) -> f64 {
    let mut distance = f64::INFINITY;
    for l in rings {
        for i in 0..l.len() {
            let a = l[i];
            let b = l[(i + 1) % l.len()];
            let v = [b[0] - a[0], b[1] - a[1]];
            let t = (((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / (v[0] * v[0] + v[1] * v[1]))
                .clamp(0., 1.);
            distance = distance.min((p[0] - a[0] - t * v[0]).hypot(p[1] - a[1] - t * v[1]));
        }
    }
    if !rings.is_empty() && even_odd(p, &rings[0]) && !rings[1..].iter().any(|h| even_odd(p, h)) {
        -distance
    } else {
        distance
    }
}
