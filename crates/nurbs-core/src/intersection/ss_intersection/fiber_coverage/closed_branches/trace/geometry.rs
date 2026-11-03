use super::*;
pub(super) fn midpoint(d: [f64; 2]) -> f64 {
    (d[0] * 0.5 + d[1] * 0.5).max(d[0]).min(d[1])
}
pub(super) fn world_bounds(s: &Surface, box4: Box4, w: &mut Work) -> Result<[[f64; 2]; 3]> {
    let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    let mut cells = 0;
    for u in s.degree_u..s.control_points.len() {
        if s.knots_u[u] == s.knots_u[u + 1] {
            continue;
        }
        for v in s.degree_v..s.control_points[0].len() {
            if s.knots_v[v] == s.knots_v[v + 1] {
                continue;
            }
            let d = [
                [
                    box4[0][0].max(s.knots_u[u]),
                    box4[0][1].min(s.knots_u[u + 1]),
                ],
                [
                    box4[1][0].max(s.knots_v[v]),
                    box4[1][1].min(s.knots_v[v + 1]),
                ],
            ];
            if d.iter().any(|a| a[0] > a[1]) {
                continue;
            }
            w.remaining()?;
            w.used += 1;
            cells += 1;
            let b = crate::surface_distance::enclosure(s, [u, v], d)?;
            for k in 0..3 {
                bounds[k] = [bounds[k][0].min(b[k].lo), bounds[k][1].max(b[k].hi)];
            }
        }
    }
    check(
        cells > 0
            && bounds
                .iter()
                .all(|d| d.iter().all(|x| x.is_finite()) && d[0] <= d[1]),
        "No original world root enclosure",
    )?;
    Ok(bounds)
}
pub(super) fn tube_errors(
    fibers: &[QualifiedFiber; 2],
    ports: &[&Port; 2],
    w: &mut Work,
) -> Result<(f64, [f64; 2])> {
    let mut world = 0_f64;
    let mut uv = [0_f64; 2];
    for (f, port) in fibers.iter().zip(ports) {
        let roots = super::super::cycle::point_bounds(f)?;
        let b = world_bounds(f.source().first(), roots, w)?;
        let mut width = I::point(0.);
        for k in 0..3 {
            width = width.add(
                I::point(b[k][1].max(port.world[k])).sub(I::point(b[k][0].min(port.world[k])))?,
            )?;
        }
        world = world.max(width.hi);
        for support in 0..2 {
            let mut width = I::point(0.);
            for k in 2 * support..2 * support + 2 {
                width = width.add(
                    I::point(roots[k][1].max(port.parameters[k]))
                        .sub(I::point(roots[k][0].min(port.parameters[k])))?,
                )?;
            }
            uv[support] = uv[support].max(width.hi);
        }
    }
    Ok((world, uv))
}
pub(super) fn parameter_errors(proof: &QualifiedFiberWorldTrace) -> Result<[f64; 2]> {
    let f = proof.source();
    let op = f.report().operator.as_ref().unwrap();
    let mut errors = [I::point(0.); 2];
    let mut unknown = 0;
    for k in 0..4 {
        if k != f.free_axis().index() {
            errors[k / 2] = errors[k / 2].add(
                I::point(op.scales()[unknown]).mul(I::point(proof.scaled_parameter_error()))?,
            )?;
            unknown += 1;
        }
    }
    Ok(errors.map(|i| i.hi))
}
