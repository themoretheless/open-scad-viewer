use super::*;
pub(super) fn assemble(
    roots: &[Arc<QualifiedPairedRootSolid>],
    parents: &[Option<usize>],
    depths: &[usize],
) -> Result<(MixedBrep, Vec<crate::mixed_root_model::MixedTopologyMap>)> {
    let mut current = roots[0].model().clone();
    let mut maps = vec![current.identity_map()?];
    for source in &roots[1..] {
        let (joined, [left, right]) = current.concatenate(source.model())?.into_parts();
        maps = maps
            .iter()
            .map(|m| m.then(&left))
            .collect::<Result<Vec<_>>>()?;
        maps.push(right);
        current = joined;
    }
    let mut t = current.topology().clone();
    let mut bodies = Vec::new();
    let mut body_ids = vec![None; roots.len()];
    for i in 0..roots.len() {
        if depths[i] % 2 == 0 {
            body_ids[i] = Some(bodies.len());
            let outer_shell = maps[i]
                .shell(0)
                .ok_or_else(|| unproved("Root forest shell map missing"))?;
            let inner_shells = (0..roots.len())
                .filter(|&j| parents[j] == Some(i))
                .map(|j| {
                    maps[j]
                        .shell(0)
                        .ok_or_else(|| unproved("Root forest cavity map missing"))
                })
                .collect::<Result<Vec<_>>>()?;
            bodies.push(brep_topology::Body {
                outer_shell,
                inner_shells,
            });
        }
    }
    for i in 0..roots.len() {
        let owner = if depths[i] % 2 == 0 {
            i
        } else {
            parents[i].ok_or_else(|| unproved("Root forest cavity has no material parent"))?
        };
        let body =
            body_ids[owner].ok_or_else(|| unproved("Root forest material body owner missing"))?;
        maps[i].reparent_body(0, body)?;
        if depths[i] % 2 == 1 {
            let shell = maps[i]
                .shell(0)
                .ok_or_else(|| unproved("Root forest inward shell map missing"))?;
            for face in &mut t.shells[shell].faces {
                face.reversed = !face.reversed;
            }
            maps[i].reverse_shell(0)?;
        }
    }
    t.bodies = bodies;
    let model = MixedBrep::from_topology(t)?;
    for map in &mut maps {
        map.retarget(&model);
    }
    Ok((model, maps))
}
