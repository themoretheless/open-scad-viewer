use super::*;
pub(super) struct Prepared {
    pub loops: [usize; 2],
    pub indices: [[usize; 2]; 2],
    pub ab_forward: [bool; 2],
    pub shell: (usize, bool),
}
pub(super) fn prepare(
    source: &Model,
    face: usize,
    paths: &[BridgePath; 2],
    uv: f64,
    limits: ClosedCutLimits,
    w: &mut Work,
) -> Result<Prepared> {
    source.validate()?;
    if !source.persistent_naming_complete()
        || !uv.is_finite()
        || uv <= 0.
        || !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || paths.iter().any(|p| p.segments.is_empty())
        || paths.iter().map(|p| p.segments.len()).sum::<usize>() > 256
    {
        return Err(invalid(
            "Need two bounded saved bridge paths and proof work",
        ));
    }
    let f = source
        .faces
        .get(face)
        .ok_or_else(|| invalid("Missing bridge support face"))?;
    if f.surface.periodic_u || f.surface.periodic_v || f.holes.is_empty() || f.holes.len() > 15 {
        return Err(unproven(
            "Bridge partition needs a nonperiodic face with original holes",
        ));
    }
    let mut vertices = std::collections::BTreeSet::new();
    for p in paths {
        for &v in &p.endpoints {
            if v >= source.vertices.len() || !vertices.insert(v) {
                return Err(invalid(
                    "Bridge ends must be four distinct owned boundary vertices",
                ));
            }
        }
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
        return Err(invalid("Bridge support needs one oriented shell use"));
    }
    let mut hits = [[(0, 0); 2]; 2];
    for (i, p) in paths.iter().enumerate() {
        for s in &p.segments {
            s.curve.validate()?;
            s.pcurve.validate()?;
        }
        for (j, s) in p.segments.iter().enumerate() {
            if !clamped(&s.curve)
                || !clamped(&s.pcurve)
                || s.curve.control_points[0].len() != 3
                || s.pcurve.control_points[0].len() != 2
                || (j > 0
                    && (p.segments[j - 1].curve.control_points.last()
                        != Some(&s.curve.control_points[0])
                        || p.segments[j - 1].pcurve.control_points.last()
                            != Some(&s.pcurve.control_points[0])))
            {
                return Err(invalid(
                    "Bridge path needs literal saved world and UV joins",
                ));
            }
        }
        for side in 0..2 {
            let mut owned = Vec::new();
            for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                for (j, c) in source.loops[wire].coedges.iter().enumerate() {
                    if source.edges[c.edge].vertices[usize::from(c.reversed)] == p.endpoints[side] {
                        owned.push((wire, j));
                    }
                }
            }
            if owned.len() != 1 {
                return Err(invalid(
                    "Bridge end has no unique original boundary occurrence",
                ));
            }
            hits[i][side] = owned[0];
            let c = &source.loops[owned[0].0].coedges[owned[0].1];
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
            let uv = if side == 0 {
                &s.pcurve.control_points[0]
            } else {
                s.pcurve.control_points.last().unwrap()
            };
            if world.as_slice() != source.vertices[p.endpoints[side]].point
                || *uv != c.pcurve.control_points[0]
            {
                return Err(invalid(
                    "Bridge end must literally match its native world and UV port",
                ));
            }
        }
    }
    let mut loops = [hits[0][0].0, hits[0][1].0];
    if loops[0] == loops[1] {
        return Err(invalid("Each bridge must connect different original loops"));
    }
    if loops[1] == f.outer || (loops[0] != f.outer && loops[1] < loops[0]) {
        loops.swap(0, 1);
    }
    let mut indices = [[0; 2]; 2];
    let mut ab_forward = [false; 2];
    for i in 0..2 {
        ab_forward[i] = hits[i][0].0 == loops[0];
        let order = if ab_forward[i] { [0, 1] } else { [1, 0] };
        for j in 0..2 {
            if hits[i][order[j]].0 != loops[j] {
                return Err(invalid(
                    "Both bridges must connect the same two original loops",
                ));
            }
            indices[i][j] = hits[i][order[j]].1;
        }
    }
    let original: Vec<_> = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .map(|wire| {
            source.loops[wire]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    if audit(&original, uv, w, limits)? != Some(true) {
        return Err(unproven(
            "Original bridge region is not proven simple positive material",
        ));
    }
    for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
        for c in &source.loops[wire].coedges {
            let e = &source.edges[c.edge];
            if c.trim().is_some_and(|t| {
                t.curve_parameter != e.curve.domain()
                    || t.pcurve_parameter != c.pcurve.domain()
                    || t.periodic_lift != [[0; 2]; 2]
            }) {
                return Err(unproven(
                    "Bridge partition needs complete original coedge windows",
                ));
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
    Ok(Prepared {
        loops,
        indices,
        ab_forward,
        shell: uses[0],
    })
}
