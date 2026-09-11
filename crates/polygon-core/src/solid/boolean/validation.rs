use super::*;
fn inside_triangle(p: Point, t: [Point; 3], eps: f64) -> bool {
    let normal = cross(sub(t[1], t[0]), sub(t[2], t[0]));
    let n = scale(normal, 1. / norm(&normal));
    (0..3).all(|i| {
        let edge = sub(t[(i + 1) % 3], t[i]);
        dot(cross(edge, sub(p, t[i])), n) >= -eps * norm(&edge)
    })
}
fn on_segment(p: Point, a: Point, b: Point, eps: f64) -> bool {
    let d = sub(b, a);
    let length = norm(&d);
    if length <= eps {
        return norm(&sub(p, a)) <= eps;
    }
    let u = dot(sub(p, a), d) / (length * length);
    u >= -eps / length && u <= 1. + eps / length && norm(&sub(sub(p, a), scale(d, u))) <= eps * 2.
}
fn intersections(a: [Point; 3], b: [Point; 3], eps: f64) -> Vec<Point> {
    let na = cross(sub(a[1], a[0]), sub(a[2], a[0]));
    let na = scale(na, 1. / norm(&na));
    let nb = cross(sub(b[1], b[0]), sub(b[2], b[0]));
    let nb = scale(nb, 1. / norm(&nb));
    let mut points = Vec::new();
    let coplanar =
        norm(&cross(na, nb)) <= eps && a.iter().all(|p| dot(nb, sub(*p, b[0])).abs() <= eps);
    if coplanar {
        for p in a {
            if inside_triangle(p, b, eps) {
                points.push(p);
            }
        }
        for p in b {
            if inside_triangle(p, a, eps) {
                points.push(p);
            }
        }
        let axis = (0..3)
            .max_by(|i, j| na[*i].abs().total_cmp(&na[*j].abs()))
            .unwrap();
        let i = (axis + 1) % 3;
        let j = (axis + 2) % 3;
        let cross2 = |x: Point, y: Point| x[i] * y[j] - x[j] * y[i];
        for ai in 0..3 {
            for bi in 0..3 {
                let p = a[ai];
                let q = b[bi];
                let r = sub(a[(ai + 1) % 3], p);
                let s = sub(b[(bi + 1) % 3], q);
                let det = cross2(r, s);
                if det.abs() > eps * norm(&r) * norm(&s) {
                    let t = cross2(sub(q, p), s) / det;
                    let u = cross2(sub(q, p), r) / det;
                    if (0. ..=1.).contains(&t) && (0. ..=1.).contains(&u) {
                        points.push(std::array::from_fn(|k| p[k] + t * r[k]));
                    }
                }
            }
        }
    } else {
        for (from, to, normal) in [(a, b, nb), (b, a, na)] {
            for i in 0..3 {
                let p = from[i];
                let q = from[(i + 1) % 3];
                let dp = dot(normal, sub(p, to[0]));
                let dq = dot(normal, sub(q, to[0]));
                if dp.abs() <= eps && inside_triangle(p, to, eps) {
                    points.push(p);
                }
                if (dp > eps && dq < -eps) || (dp < -eps && dq > eps) {
                    let t = dp / (dp - dq);
                    let point = std::array::from_fn(|k| p[k] + t * (q[k] - p[k]));
                    if inside_triangle(point, to, eps) {
                        points.push(point);
                    }
                }
            }
        }
    }
    points
}
/// Bounded sweep checks crossings and coplanar overlaps, allowing only contact
/// represented by shared vertex/edge indices. All predicates use the CSG tolerance.
pub(super) fn geometry(mesh: &Mesh, eps: f64, budget: &mut Budget) -> Result<()> {
    let triangles: Vec<[Point; 3]> = mesh
        .indices
        .chunks_exact(3)
        .map(|t| {
            [
                mesh.point(t[0]).unwrap(),
                mesh.point(t[1]).unwrap(),
                mesh.point(t[2]).unwrap(),
            ]
        })
        .collect();
    let bounds: Vec<(Point, Point)> = triangles
        .iter()
        .map(|t| {
            (
                std::array::from_fn(|a| t.iter().map(|p| p[a]).fold(f64::INFINITY, f64::min)),
                std::array::from_fn(|a| t.iter().map(|p| p[a]).fold(f64::NEG_INFINITY, f64::max)),
            )
        })
        .collect();
    // AABB hierarchy avoids quadratic scans for thin fragments whose x
    // intervals overlap but which are separated on another axis.
    struct Node {
        lo: Point,
        hi: Point,
        children: Option<(usize, usize)>,
        triangles: Vec<usize>,
    }
    fn build(ids: &mut [usize], bounds: &[(Point, Point)], nodes: &mut Vec<Node>) -> usize {
        let lo = std::array::from_fn(|k| {
            ids.iter()
                .map(|&i| bounds[i].0[k])
                .fold(f64::INFINITY, f64::min)
        });
        let hi = std::array::from_fn(|k| {
            ids.iter()
                .map(|&i| bounds[i].1[k])
                .fold(f64::NEG_INFINITY, f64::max)
        });
        let at = nodes.len();
        nodes.push(Node {
            lo,
            hi,
            children: None,
            triangles: vec![],
        });
        if ids.len() <= 8 {
            nodes[at].triangles = ids.to_vec()
        } else {
            let axis = (0..3)
                .max_by(|&a, &b| (hi[a] - lo[a]).total_cmp(&(hi[b] - lo[b])))
                .unwrap();
            ids.sort_by(|&a, &b| {
                (bounds[a].0[axis] + bounds[a].1[axis])
                    .total_cmp(&(bounds[b].0[axis] + bounds[b].1[axis]))
            });
            let (left, right) = ids.split_at_mut(ids.len() / 2);
            let l = build(left, bounds, nodes);
            let r = build(right, bounds, nodes);
            nodes[at].children = Some((l, r));
        }
        at
    }
    let mut ids: Vec<_> = (0..triangles.len()).collect();
    let mut nodes = vec![];
    if ids.is_empty() {
        return Ok(());
    }
    let root = build(&mut ids, &bounds, &mut nodes);
    for a in 0..triangles.len() {
        let mut stack = vec![root];
        let mut candidates = vec![];
        while let Some(i) = stack.pop() {
            budget.tick(1)?;
            let node = &nodes[i];
            if (0..3)
                .any(|k| bounds[a].0[k] > node.hi[k] + eps || node.lo[k] > bounds[a].1[k] + eps)
            {
                continue;
            }
            if let Some((left, right)) = node.children {
                stack.extend([left, right])
            } else {
                candidates.extend(node.triangles.iter().copied().filter(|&b| b > a))
            }
        }
        candidates.sort_unstable();
        for b in candidates {
            budget.tick(1)?;
            if (0..3).any(|i| {
                bounds[a].0[i] > bounds[b].1[i] + eps || bounds[b].0[i] > bounds[a].1[i] + eps
            }) {
                continue;
            }
            let ta = &mesh.indices[3 * a..3 * a + 3];
            let tb = &mesh.indices[3 * b..3 * b + 3];
            let shared: Vec<Point> = ta
                .iter()
                .filter(|i| tb.contains(i))
                .map(|i| mesh.point(*i).unwrap())
                .collect();
            for p in intersections(triangles[a], triangles[b], eps) {
                let allowed = match shared.as_slice() {
                    [a] => norm(&sub(p, *a)) <= eps * 2.,
                    [a, b] => on_segment(p, *a, *b, eps),
                    _ => false,
                };
                if !allowed {
                    return Err(invalid(&format!(
                        "Solid contains intersecting, overlapping or unstitched triangles at the Boolean tolerance: triangles {a}/{b}, shared={}, point={p:?}",
                        shared.len()
                    )));
                }
            }
        }
    }
    Ok(())
}
fn winding(mesh: &Mesh, p: Point, budget: &mut Budget) -> Result<f64> {
    let mut angle = 0.;
    for t in mesh.indices.chunks_exact(3) {
        budget.tick(1)?;
        let a = sub(mesh.point(t[0])?, p);
        let b = sub(mesh.point(t[1])?, p);
        let c = sub(mesh.point(t[2])?, p);
        let aa = norm(&a);
        let bb = norm(&b);
        let cc = norm(&c);
        angle += 2.
            * dot(a, cross(b, c))
                .atan2(aa * bb * cc + dot(a, b) * cc + dot(b, c) * aa + dot(c, a) * bb);
    }
    Ok(angle / (4. * std::f64::consts::PI))
}
/// A component can be an outer shell or an inward-oriented cavity. Sample both
/// sides of a large face in each component against the full oriented solid.
pub(super) fn orientation(mesh: &Mesh, eps: f64, budget: &mut Budget) -> Result<()> {
    if mesh.indices.is_empty() {
        return Ok(());
    }
    let mut by_vertex = BTreeMap::<usize, Vec<usize>>::new();
    for (i, t) in mesh.indices.chunks_exact(3).enumerate() {
        for v in t {
            by_vertex.entry(*v).or_default().push(i);
        }
    }
    let mut seen = vec![false; mesh.indices.len() / 3];
    for start in 0..seen.len() {
        if seen[start] {
            continue;
        }
        let mut stack = vec![start];
        let mut best = start;
        let mut area = 0.;
        while let Some(i) = stack.pop() {
            if seen[i] {
                continue;
            }
            seen[i] = true;
            let t = &mesh.indices[3 * i..3 * i + 3];
            let n = norm(&cross(
                sub(mesh.point(t[1])?, mesh.point(t[0])?),
                sub(mesh.point(t[2])?, mesh.point(t[0])?),
            ));
            if n > area {
                area = n;
                best = i;
            }
            for v in t {
                stack.extend(&by_vertex[v]);
            }
        }
        let t = &mesh.indices[3 * best..3 * best + 3];
        let a = mesh.point(t[0])?;
        let b = mesh.point(t[1])?;
        let c = mesh.point(t[2])?;
        let center = std::array::from_fn(|i| (a[i] + b[i] + c[i]) / 3.);
        let direction = scale(cross(sub(b, a), sub(c, a)), eps * 8. / area);
        let inside = winding(mesh, sub(center, direction), budget)?;
        let outside = winding(
            mesh,
            std::array::from_fn(|i| center[i] + direction[i]),
            budget,
        )?;
        if (inside - 1.).abs() > 1e-4 || outside.abs() > 1e-4 {
            return Err(invalid(
                "Shell orientation/nesting is inconsistent, or features are too close for the Boolean tolerance",
            ));
        }
    }
    Ok(())
}
