//! Shared admission rules for topologically ordered geometry programs.
pub(crate) fn validate<'a>(
    count: usize,
    roots: &[usize],
    inputs: impl Iterator<Item = &'a [usize]>,
    budget_error: &str,
    reference_error: &str,
) -> crate::Result<()> {
    if count > 25_000 || roots.len() > 1000 {
        return Err(crate::fail(budget_error));
    }
    let mut edges = 0usize;
    for (index, references) in inputs.enumerate() {
        edges = edges
            .checked_add(references.len())
            .ok_or_else(|| crate::fail(budget_error))?;
        if edges > 100_000 {
            return Err(crate::fail(budget_error));
        }
        if references.iter().any(|&input| input >= index) {
            return Err(crate::fail(reference_error));
        }
    }
    if roots.iter().any(|&root| root >= count) {
        return Err(crate::fail(reference_error));
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct ExecutionPlan {
    pub reachable: Vec<bool>,
    pub uses: Vec<usize>,
}
/// Validate references before indexing, then count uses only in reachable nodes.
/// Repeated roots and repeated input references each retain their own use.
pub fn execution_plan(inputs: &[&[usize]], roots: &[usize]) -> crate::Result<ExecutionPlan> {
    validate(
        inputs.len(),
        roots,
        inputs.iter().copied(),
        "Geometry program budget exceeded",
        "Geometry program references must precede consumers",
    )?;
    let mut reachable = vec![false; inputs.len()];
    let mut pending = roots.to_vec();
    while let Some(index) = pending.pop() {
        if std::mem::replace(&mut reachable[index], true) {
            continue;
        }
        pending.extend(inputs[index]);
    }
    let mut uses = vec![0usize; inputs.len()];
    for &root in roots {
        uses[root] += 1;
    }
    for (index, references) in inputs.iter().enumerate() {
        if reachable[index] {
            for &input in *references {
                uses[input] += 1;
            }
        }
    }
    Ok(ExecutionPlan { reachable, uses })
}
/// Copy reachable nodes in dependency order and remap inputs and roots once.
/// Node-specific geometry admission belongs to the returned program's owner.
pub(crate) fn compact<T: Clone>(
    arena: &[T],
    roots: &[usize],
    inputs: impl Fn(&T) -> &[usize],
    mut remap: impl FnMut(&mut T, &[usize]),
) -> crate::Result<(Vec<T>, Vec<usize>)> {
    let references = arena.iter().map(&inputs).collect::<Vec<_>>();
    let plan = execution_plan(&references, roots)?;
    let mut indices = vec![usize::MAX; arena.len()];
    let mut nodes = Vec::new();
    for (index, node) in arena.iter().enumerate() {
        if !plan.reachable[index] {
            continue;
        }
        indices[index] = nodes.len();
        let mut node = node.clone();
        remap(&mut node, &indices);
        nodes.push(node);
    }
    Ok((nodes, roots.iter().map(|&root| indices[root]).collect()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_consumers_and_unreachable_nodes_keep_correct_lifetimes() {
        let plan = execution_plan(&[&[], &[], &[0, 0], &[1]], &[2, 2]).unwrap();
        assert_eq!(plan.reachable, vec![true, false, true, false]);
        assert_eq!(plan.uses, vec![2, 0, 2, 0]);
        assert!(execution_plan(&[&[0]], &[0]).is_err());
        assert!(execution_plan(&[&[]], &[1]).is_err());
    }
}

pub(crate) enum NestedProgram<'a> {
    Solid(&'a crate::solid_program::Program),
    Profile(&'a crate::profile_program::Program),
}
/// One aggregate admission budget covers all alternating solid/profile subprograms.
pub(crate) fn validate_nested(root: NestedProgram<'_>) -> crate::Result<()> {
    use crate::{profile_program::Node as P, solid_program::Node as S};
    let mut pending = vec![(root, 0usize)];
    let (mut nodes, mut edges, mut points) = (0usize, 0usize, 0usize);
    let (mut mesh_vertices, mut mesh_triangles) = (0usize, 0usize);
    while let Some((program, depth)) = pending.pop() {
        if depth > 32 {
            return Err(crate::fail("Geometry program nesting budget exceeded"));
        }
        let mut count_rings = |rings: &Vec<Vec<[f64; 2]>>| {
            for ring in rings {
                points = points.saturating_add(ring.len());
            }
        };
        match program {
            NestedProgram::Solid(program) => {
                nodes = nodes.saturating_add(program.nodes.len());
                for node in &program.nodes {
                    edges = edges.saturating_add(node.inputs().len());
                    match node {
                        S::Mesh { mesh, .. } => {
                            mesh_vertices = mesh_vertices.saturating_add(mesh.positions.len() / 3);
                            mesh_triangles = mesh_triangles.saturating_add(mesh.indices.len() / 3);
                        }
                        S::ExtrudeRings { rings, .. } => count_rings(rings),
                        S::ExtrudeProfile { profile, .. } | S::RevolveProfile { profile, .. } => {
                            pending.push((NestedProgram::Profile(profile), depth + 1))
                        }
                        _ => {}
                    }
                }
            }
            NestedProgram::Profile(program) => {
                nodes = nodes.saturating_add(program.nodes.len());
                for node in &program.nodes {
                    edges = edges.saturating_add(node.inputs().len());
                    match node {
                        P::Rings(rings) | P::EvenOddRings(rings) => count_rings(rings),
                        P::Projection { solid, .. } => {
                            pending.push((NestedProgram::Solid(solid), depth + 1))
                        }
                        _ => {}
                    }
                }
            }
        }
        if nodes > 25_000
            || edges > 100_000
            || points > 100_000
            || mesh_vertices > 300_000
            || mesh_triangles > 100_000
        {
            return Err(crate::fail("Geometry program aggregate budget exceeded"));
        }
    }
    Ok(())
}
