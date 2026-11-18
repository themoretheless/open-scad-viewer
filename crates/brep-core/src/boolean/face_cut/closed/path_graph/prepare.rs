use super::*;
pub(super) struct Prepared {
    pub ports: Vec<[(usize, usize); 2]>,
    pub shell: (usize, bool),
}
pub(super) fn prepare(
    source: &Model,
    face: usize,
    paths: &[BridgePath],
    uv: f64,
    tols: &[f64],
    limits: ClosedCutLimits,
    w: &mut Work,
) -> Result<Prepared> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !(1..=256).contains(&paths.len())
        || paths.iter().any(|p| p.segments.is_empty())
        || paths.iter().map(|p| p.segments.len()).sum::<usize>() > 256
        || !uv.is_finite()
        || uv <= 0.
        || tols.len() != paths.len()
        || tols.iter().any(|t| !t.is_finite() || *t <= 0.)
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
    {
        return Err(invalid(
            "Need bounded saved path graph and finite tolerances",
        ));
    }
    let f = source
        .faces
        .get(face)
        .ok_or_else(|| invalid("Missing path graph face"))?;
    if f.surface.periodic_u || f.surface.periodic_v || f.holes.len() > 15 {
        return Err(unproven("Path graph needs a nonperiodic original chart"));
    }
    let uses: Vec<_> = source
        .shells
        .iter()
        .enumerate()
        .flat_map(|(si, s)| {
            s.faces
                .iter()
                .filter(move |u| u.face == face)
                .map(move |u| (si, u.reversed))
        })
        .collect();
    if uses.len() != 1 {
        return Err(invalid("Path graph face needs one oriented shell use"));
    }
    let loops: Vec<_> = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .collect();
    let original: Vec<_> = loops
        .iter()
        .map(|&l| {
            source.loops[l]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    if audit(&original, uv, w, limits)? != Some(true) {
        return Err(unproven("Original path graph region is not valid material"));
    }
    for &l in &loops {
        for c in &source.loops[l].coedges {
            let e = &source.edges[c.edge];
            if c.trim().is_some_and(|t| {
                t.curve_parameter != e.curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(unproven("Path graph needs complete source coedge windows"));
            }
            let tol = source.tolerance_mm.min(
                *source
                    .1
                    .local_tolerances_mm
                    .get(&source.1.edges[c.edge])
                    .unwrap_or(&source.tolerance_mm),
            );
            lift(&e.curve, &c.pcurve, &f.surface, c.reversed, tol, w, limits)?;
        }
    }
    let mut endpoints = std::collections::BTreeSet::new();
    let mut ports = Vec::new();
    for p in paths {
        for s in &p.segments {
            s.curve.validate()?;
            s.pcurve.validate()?;
        }
        for (i, s) in p.segments.iter().enumerate() {
            if !clamped(&s.curve)
                || !clamped(&s.pcurve)
                || s.curve.control_points[0].len() != 3
                || s.pcurve.control_points[0].len() != 2
                || (i > 0
                    && (p.segments[i - 1].curve.control_points.last()
                        != Some(&s.curve.control_points[0])
                        || p.segments[i - 1].pcurve.control_points.last()
                            != Some(&s.pcurve.control_points[0])))
            {
                return Err(invalid("Path graph needs literal world and UV joins"));
            }
        }
        let mut owned = [(0, 0); 2];
        for side in 0..2 {
            let v = p.endpoints[side];
            if v >= source.vertices.len() || !endpoints.insert(v) {
                return Err(unproven(
                    "Path graph requires distinct existing boundary ports",
                ));
            }
            let mut hits = Vec::new();
            for &l in &loops {
                for (i, c) in source.loops[l].coedges.iter().enumerate() {
                    if source.edges[c.edge].vertices[usize::from(c.reversed)] == v {
                        hits.push((l, i));
                    }
                }
            }
            if hits.len() != 1 {
                return Err(unproven(
                    "Path graph endpoint lacks unique original ownership",
                ));
            }
            owned[side] = hits[0];
            let s = if side == 0 {
                &p.segments[0]
            } else {
                p.segments.last().unwrap()
            };
            let world = if side == 0 {
                &s.curve.control_points[0]
            } else {
                s.curve.control_points.last().unwrap()
            };
            let pc = if side == 0 {
                &s.pcurve.control_points[0]
            } else {
                s.pcurve.control_points.last().unwrap()
            };
            if world.as_slice() != source.vertices[v].point
                || *pc
                    != source.loops[owned[side].0].coedges[owned[side].1]
                        .pcurve
                        .control_points[0]
            {
                return Err(unproven(
                    "Path graph endpoint does not literally match its native port",
                ));
            }
        }
        ports.push(owned);
    }
    Ok(Prepared {
        ports,
        shell: uses[0],
    })
}
