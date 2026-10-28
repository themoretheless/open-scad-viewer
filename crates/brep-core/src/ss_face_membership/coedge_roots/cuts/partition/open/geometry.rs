use super::*;
pub(super) fn trim(c: &Curve, clock: [f64; 2], reverse: bool) -> Result<Curve> {
    if c.domain() != [0., 1.] || c.knots.iter().any(|&k| k != 0. && k != 1.) {
        return Err(unproven(
            "Original SS path needs exact single-span source clocks",
        ));
    }
    let c = if clock == [0., 1.] {
        c.clone()
    } else {
        c.trim(clock[0], clock[1])?
    };
    if reverse { c.reverse() } else { Ok(c) }
}
fn available(tolerance: f64, source_error: f64) -> Result<f64> {
    let remaining = I::point(tolerance).sub(I::point(source_error))?.lo;
    if remaining <= 0. {
        return Err(unproven(
            "Original SS proxy leaves no certified authoring error allowance",
        ));
    }
    Ok(remaining)
}
pub(super) fn prove(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    source: &ss::GraphEdgeCurves,
    world: &Curve,
    uv: &[Curve; 2],
    p: &Part,
    world_tol: f64,
    uv_tol: f64,
    cells: &mut usize,
    max: usize,
) -> Result<()> {
    let clock = if p.step.reversed {
        [p.clock[1], p.clock[0]]
    } else {
        p.clock
    };
    let map = crate::line(vec![clock[0], 0.], vec![clock[1], 0.]);
    agreement(
        world,
        &map,
        &carrier(&source.world),
        false,
        available(world_tol, source.world_error_upper)?,
        cells,
        max,
    )?;
    for side in 0..2 {
        agreement(
            &embed_uv(&uv[side]),
            &map,
            &carrier(&embed_uv(&source.pcurves[side])),
            false,
            available(uv_tol, source.uv_error_upper[side])?,
            cells,
            max,
        )?;
        let (m, f) = input.source.roots.faces[side];
        agreement(
            world,
            &uv[side],
            &m.faces[f].surface,
            false,
            world_tol.min(m.tolerance_mm),
            cells,
            max,
        )?;
    }
    Ok(())
}
fn range(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    edge: usize,
    clock: [f64; 2],
    used: &mut usize,
    max: usize,
) -> Result<ss::OriginalGraphSSRange> {
    if *used == max {
        return Err(unproven("Original SS path enclosure work exhausted"));
    }
    let r = input
        .source
        .roots
        .original
        .edge_enclosure(edge, clock, (max - *used).min(4096))?;
    spend(used, max, r.operations)?;
    Ok(r)
}
pub(super) fn classify_cores(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    parts: &[Part],
    uv: f64,
    used: &mut usize,
    cells: &mut usize,
    limits: OpenSSPathLimits,
) -> Result<()> {
    let faces = input.source.roots.faces;
    let domains = [
        FaceDomain::new(faces[0].0, faces[0].1, uv)?,
        FaceDomain::new(faces[1].0, faces[1].1, uv)?,
    ];
    for p in parts {
        let t = p.core[0] + (p.core[1] - p.core[0]) * 0.5;
        if !(p.core[0] < t && t < p.core[1]) {
            return Err(unproven(
                "No representable point inside the guard-free SS core",
            ));
        }
        let r = range(input, p.step.original_edge, [t, t], used, limits.operations)?;
        for (side, box_uv) in [r.first_uv, r.second_uv].into_iter().enumerate() {
            spend(used, limits.operations, 1)?;
            let location = if domains[side].contains_closed_rectangle(box_uv) {
                Location::Inside
            } else {
                if *cells == limits.cells {
                    return Err(unproven(
                        "SS path retained-core classification budget exhausted",
                    ));
                }
                let r = domains[side].classify(box_uv, (limits.cells - *cells).min(100000))?;
                spend(cells, limits.cells, r.cells)?;
                r.location
            };
            if location != Location::Inside {
                return Err(unproven(
                    "Original SS core is outside a retained face or unresolved",
                ));
            }
        }
    }
    Ok(())
}
pub(super) fn guard(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    end: &End,
    p: &Part,
    segment: &OpenSSSegment,
    endpoint: usize,
    world: f64,
    uv: f64,
    used: &mut usize,
    max: usize,
) -> Result<OpenSSRootGuard> {
    let bounds = range(input, p.step.original_edge, end.band, used, max)?;
    spend(used, max, 24)?;
    let world_error_upper = distance(&end.position, &bounds.world)?;
    let mut uv_error_upper = [0.; 2];
    for (side, b) in [bounds.first_uv, bounds.second_uv].into_iter().enumerate() {
        let c = &segment.pcurves[side];
        let p = if endpoint == 0 {
            &c.control_points[0]
        } else {
            c.control_points.last().unwrap()
        };
        uv_error_upper[side] = distance(p, &b)?;
    }
    if world_error_upper > world || uv_error_upper.iter().any(|&v| v > uv) {
        return Err(unproven(
            "SS endpoint uncertainty cap exceeds its original source tolerance",
        ));
    }
    Ok(OpenSSRootGuard {
        node: end.node,
        original_edge: p.step.original_edge,
        clock: end.band,
        world_error_upper,
        uv_error_upper,
    })
}
