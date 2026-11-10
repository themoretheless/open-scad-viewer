//! Closed material cycles of an entire saved-path graph on one original face.
use super::bridge::BridgePath;
use super::*;
mod author;
mod cycles;
mod prepare;
#[cfg(test)]
mod tests;
pub struct PathGraphPartition {
    pub model: Model,
    pub faces: Vec<usize>,
    pub edges: Vec<Vec<usize>>,
    pub vertices: Vec<Vec<usize>>,
    /// Actual forward/backward material owner of each complete saved path.
    pub path_faces: Vec<[usize; 2]>,
    pub source_identity: TopoId,
    pub face_identities: Vec<TopoId>,
    pub cells: usize,
    pub pairs: usize,
}
pub fn partition_path_graph(
    source: &Model,
    face: usize,
    paths: &[BridgePath],
    uv: f64,
    limits: ClosedCutLimits,
) -> Result<PathGraphPartition> {
    partition_path_graph_with_tolerances(
        source,
        face,
        paths,
        uv,
        &vec![source.tolerance_mm; paths.len()],
        limits,
    )
}
pub(crate) fn partition_path_graph_with_tolerances(
    source: &Model,
    face: usize,
    paths: &[BridgePath],
    uv: f64,
    tols: &[f64],
    limits: ClosedCutLimits,
) -> Result<PathGraphPartition> {
    let mut work = Work::default();
    let p = prepare::prepare(source, face, paths, uv, tols, limits, &mut work)?;
    let mut offsets = Vec::new();
    let mut at = source.edges.len();
    for path in paths {
        offsets.push(at);
        at += path.segments.len();
    }
    let cycles = cycles::build(source, face, paths, &p.ports, &offsets)?;
    let curves = |ring: &[Coedge]| ring.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>();
    let mut positive = Vec::new();
    let mut negative = Vec::new();
    for (i, ring) in cycles.rings.iter().enumerate() {
        match audit(&[curves(ring)], uv, &mut work, limits)? {
            Some(true) => positive.push(i),
            Some(false) => negative.push(i),
            None => return Err(unproven("Path graph has an unresolved material cycle")),
        }
    }
    if positive.len() < 2 {
        return Err(unproven(
            "Path graph does not prove at least two material sectors",
        ));
    }
    let mut regions: Vec<_> = positive
        .iter()
        .map(|&i| vec![curves(&cycles.rings[i])])
        .collect();
    let mut owners = vec![usize::MAX; cycles.rings.len()];
    for (p, &c) in positive.iter().enumerate() {
        owners[c] = p;
    }
    let mut pending = negative;
    while !pending.is_empty() {
        let mut deferred = Vec::new();
        let mut progress = false;
        for c in pending {
            let hole = curves(&cycles.rings[c]);
            let mut candidates = Vec::new();
            for (p, region) in regions.iter().enumerate() {
                // A new edge has exactly reversed saved UV definitions on its
                // two uses. Shared ownership proves contact, so this cycle
                // cannot be a strictly separated hole of that outer cycle.
                if cycles.rings[c].iter().any(|a| {
                    a.edge >= source.edges.len()
                        && cycles.rings[positive[p]].iter().any(|b| b.edge == a.edge)
                }) {
                    continue;
                }
                let mut trial = region.clone();
                trial.push(hole.clone());
                if audit(&trial, uv, &mut work, limits)? == Some(true) {
                    candidates.push(p);
                }
            }
            if candidates.len() == 1 {
                owners[c] = candidates[0];
                regions[candidates[0]].push(hole);
                progress = true;
            } else {
                deferred.push(c);
            }
        }
        if !progress {
            return Err(unproven(
                "Path graph hole assignments have no unique proved fixed point",
            ));
        }
        pending = deferred;
    }
    for region in &regions {
        if audit(region, uv, &mut work, limits)? != Some(true) {
            return Err(unproven("Path graph final sector is unresolved"));
        }
    }
    // The successor permutation preserves each original boundary once and both
    // opposite saved directions of each new edge exactly, establishing the
    // oriented boundary cancellation used by this complete region partition.
    for ring in &cycles.rings {
        for c in ring.iter().filter(|c| c.edge >= source.edges.len()) {
            let j = offsets.iter().rposition(|&o| o <= c.edge).unwrap();
            let saved = &paths[j].segments[c.edge - offsets[j]];
            lift(
                &saved.curve,
                &c.pcurve,
                &source.faces[face].surface,
                c.reversed,
                source.tolerance_mm.min(tols[j]),
                &mut work,
                limits,
            )?;
        }
    }
    let mut result = author::author(source, face, paths, &cycles, &positive, &owners, p.shell)?;
    result.cells = work.cells;
    result.pairs = work.pairs;
    Ok(result)
}
