//! Exact convex containment of a complete embedded tool, not a centroid test.
use super::*;
pub(super) fn shell(m: &Model, c: &Certificate) -> Result<Q> {
    if m.bodies.len() != 1
        || m.shells.len() != 1
        || !m.bodies[0].inner_shells.is_empty()
        || m.bodies[0].outer_shell != 0
        || !m.shells[0].closed
    {
        return Err(fail(
            "enclosed subtraction requires one original closed outer shell",
        ));
    }
    if c.positions.is_empty() {
        return Err(fail("enclosed subtraction source is empty"));
    }
    let points = c
        .positions
        .iter()
        .map(|p| {
            p.each_ref()
                .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()))
        })
        .collect::<Vec<P>>();
    let mut adjacency = vec![Vec::new(); points.len()];
    let mut volume = Q::zero();
    for face in &c.triangles {
        for k in 0..3 {
            adjacency[face[k] as usize].push(face[(k + 1) % 3] as usize);
            adjacency[face[(k + 1) % 3] as usize].push(face[k] as usize);
        }
        volume += dot(
            &points[face[0] as usize],
            &cross(&points[face[1] as usize], &points[face[2] as usize]),
        );
    }
    let mut seen = vec![false; points.len()];
    let mut stack = vec![0];
    while let Some(v) = stack.pop() {
        if seen[v] {
            continue;
        }
        seen[v] = true;
        stack.extend(adjacency[v].iter().copied());
    }
    if seen.iter().any(|v| !*v) || volume <= Q::zero() {
        return Err(fail(
            "enclosed subtraction needs a connected outward source boundary",
        ));
    }
    Ok(volume / integer(6))
}
fn evaluate(plane: &[Q; 4], h: &sb::H) -> Q {
    (0..3).map(|k| &plane[k] * &h[k]).sum::<Q>() - &plane[3] * &h[3]
}
pub(super) fn containment(
    container: &[Patch],
    tool: &[Patch],
    tool_certificate: &Certificate,
    work: &mut Work,
) -> Result<Vec<[Q; 4]>> {
    let mut planes = BTreeSet::new();
    for p in container {
        let corners = bernstein::corners(&p.h);
        let n = cross(
            &sub(&corners[1], &corners[0]),
            &sub(&corners[2], &corners[0]),
        );
        let raw = [
            n[0].clone(),
            n[1].clone(),
            n[2].clone(),
            dot(&n, &corners[0]),
        ];
        let plane = normalized(&raw, true);
        for h in p.h.iter().flatten() {
            work.charge(1)?;
            if !evaluate(&plane, h).is_zero() {
                return Err(fail(
                    "enclosed subtraction container is not proved planar-convex",
                ));
            }
        }
        planes.insert(plane);
    }
    if planes.len() < 4 {
        return Err(fail(
            "enclosed subtraction container has insufficient supports",
        ));
    }
    for plane in &planes {
        for p in container {
            for h in p.h.iter().flatten() {
                work.charge(1)?;
                if evaluate(plane, h) > Q::zero() {
                    return Err(fail(
                        "container does not lie inside its outward convex supports",
                    ));
                }
            }
        }
        for p in tool {
            for h in p.h.iter().flatten() {
                work.charge(1)?;
                if h[3] > Q::zero() && evaluate(plane, h) >= Q::zero() {
                    return Err(fail(
                        "tool touches or crosses an outer support; closing caps are not proved",
                    ));
                }
            }
        }
        // Uniform removable source limits are checked too, not divided as 0/0.
        for point in &tool_certificate.positions {
            let p = point
                .each_ref()
                .map(|q| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap()));
            let h = [p[0].clone(), p[1].clone(), p[2].clone(), Q::one()];
            if evaluate(plane, &h) >= Q::zero() {
                return Err(fail(
                    "tool source limit is not strictly inside the container",
                ));
            }
        }
    }
    Ok(planes.into_iter().collect())
}
