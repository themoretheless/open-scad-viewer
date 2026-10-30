//! iges interchange v2 algorithm: restore_identity.
use super::*;
pub fn restore_identity(
    entities: &BTreeMap<usize, Entity>,
    map: &[(TopoKind, usize, usize)],
    model: &mut Model,
) -> Result<crate::step_interchange::StepIdentityReport> {
    let mut metadata = BTreeMap::new();
    for e in entities
        .values()
        .filter(|e| e.directory.ty == 406 && e.directory.form == 15)
    {
        if e.values.len() != 4 {
            continue;
        }
        let Some(payload) = e.values[2].strip_prefix("OSCAD_TOPO/IGES2|") else {
            continue;
        };
        let p = payload.split('|').collect::<Vec<_>>();
        if p.len() != 4 {
            return Err(refuse("Malformed IGES identity property"));
        }
        let kind = TopoKind::parse(p[0]).ok_or_else(|| refuse("Unknown IGES identity kind"))?;
        let id = TopoId::parse(p[1]).map_err(refuse)?;
        let index = p[3]
            .parse::<usize>()
            .map_err(|_| refuse("Invalid IGES identity index"))?;
        let target = u(&e.values, 3, "identity target")?;
        let target_entity = entities
            .get(&target)
            .ok_or_else(|| refuse("Missing IGES identity target"))?;
        if id.kind() != kind || p[2] != topology_digest(target_entity) {
            return Err(refuse("Transplanted IGES identity metadata"));
        }
        if metadata.insert((kind, target, index), id).is_some() {
            return Err(refuse("Duplicate IGES identity property"));
        }
    }
    let count = map.len();
    if metadata.is_empty() {
        return Ok(crate::step_interchange::StepIdentityReport {
            preserved: false,
            source: "external-iges",
            preserved_count: 0,
            created_count: count,
            lost_count: count,
        });
    }
    if metadata.len() != count {
        return Err(refuse("Partial IGES identity metadata"));
    }
    let mut seen = BTreeSet::new();
    let mut remap = BTreeMap::new();
    let mut ordinal = BTreeMap::<(TopoKind, usize), usize>::new();
    for &(kind, index, de) in map {
        let occurrence = ordinal.entry((kind, de)).or_insert(0);
        let id = *metadata
            .get(&(kind, de, *occurrence))
            .ok_or_else(|| refuse("IGES identity map is not topology-bijective"))?;
        *occurrence += 1;
        if !seen.insert(id) {
            return Err(refuse("Duplicate IGES TopoId"));
        }
        let old = match kind {
            TopoKind::Vertex => model.1.vertices[index],
            TopoKind::Edge => model.1.edges[index],
            TopoKind::Loop => model.1.loops[index],
            TopoKind::Face => model.1.faces[index],
            TopoKind::Shell => model.1.shells[index],
            TopoKind::Body => model.1.bodies[index],
            TopoKind::ControlPoint => unreachable!(),
        };
        remap.insert(old, id);
        match kind {
            TopoKind::Vertex => model.1.vertices[index] = id,
            TopoKind::Edge => model.1.edges[index] = id,
            TopoKind::Loop => model.1.loops[index] = id,
            TopoKind::Face => model.1.faces[index] = id,
            TopoKind::Shell => model.1.shells[index] = id,
            TopoKind::Body => model.1.bodies[index] = id,
            TopoKind::ControlPoint => unreachable!(),
        }
    }
    model.1.change_set.nodes = model
        .1
        .change_set
        .nodes
        .iter()
        .map(|(id, kind)| (remap.get(id).copied().unwrap_or(*id), *kind))
        .collect();
    for change in &mut model.1.change_set.changes {
        for id in change.parents.iter_mut().chain(&mut change.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    for relation in &mut model.1.lineage {
        for id in relation.parents.iter_mut().chain(&mut relation.children) {
            if let Some(next) = remap.get(id) {
                *id = *next
            }
        }
    }
    Ok(crate::step_interchange::StepIdentityReport {
        preserved: true,
        source: "internal-metadata",
        preserved_count: count,
        created_count: 0,
        lost_count: 0,
    })
}
