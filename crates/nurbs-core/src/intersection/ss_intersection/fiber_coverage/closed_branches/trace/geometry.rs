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
/// Both original surfaces contain the same root image. Intersect their
/// independent enclosures to avoid inheriting a loose bound from one chart.
pub(super) fn shared_world_bounds(
    source: &FiberSources,
    root: Box4,
    w: &mut Work,
) -> Result<[[f64; 2]; 3]> {
    let first = world_bounds(source.first(), root, w)?;
    let second = world_bounds(source.second(), [root[2], root[3], root[0], root[1]], w)?;
    let shared = std::array::from_fn(|axis| {
        [
            first[axis][0].max(second[axis][0]),
            first[axis][1].min(second[axis][1]),
        ]
    });
    check(
        shared.iter().all(|[lo, hi]| lo <= hi),
        "Original root world enclosures disagree",
    )?;
    Ok(shared)
}

pub(super) fn tube_errors(
    fibers: &[QualifiedFiber; 2],
    ports: &[&Port; 2],
    original_ports: [Box4; 2],
    w: &mut Work,
) -> Result<(f64, [f64; 2])> {
    let mut world = 0_f64;
    let mut uv = [0_f64; 2];
    for ((f, port), original) in fibers.iter().zip(ports).zip(original_ports) {
        let mut roots = super::super::cycle::point_bounds(f)?;
        let domain = f.domain();
        let [lo, hi] = domain[f.free_axis().index()];
        if contained(original, domain) {
            let transport = if lo == hi {
                Some(original)
            } else {
                let (transport, cells) = f.transported_root_bounds(original, w.remaining()?)?;
                w.used += cells;
                transport
            };
            if let Some(transport) = transport {
                for axis in 0..4 {
                    roots[axis][0] = roots[axis][0].max(transport[axis][0]);
                    roots[axis][1] = roots[axis][1].min(transport[axis][1]);
                }
                check(
                    roots.iter().all(|[lo, hi]| lo <= hi),
                    "Endpoint root bounds disagree",
                )?;
            }
        }
        let b = shared_world_bounds(f.source(), roots, w)?;
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
