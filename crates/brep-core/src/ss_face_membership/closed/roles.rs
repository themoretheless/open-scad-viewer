//! Whole authored sectors relative to the opposite ORIGINAL graph support.
//! Opposite trim membership, body containment and Solid sewing are separate.
use super::*;
#[derive(Clone, Copy, Debug)]
pub struct ClosedGraphSideLimits {
    pub operations: usize,
}
impl Default for ClosedGraphSideLimits {
    fn default() -> Self {
        Self {
            operations: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphFaceSide {
    pub side: usize,
    pub model: usize,
    pub face: usize,
    /// Above the opposite complete graph support, away from the shared contour.
    pub above: bool,
    pub inner: bool,
}
pub struct QualifiedClosedGraphSides<'s, 'p, 'a> {
    source: &'s QualifiedClosedBranches<'p, 'a>,
    limits: ClosedGraphSideLimits,
    faces: Vec<GraphFaceSide>,
    operations: usize,
}
impl QualifiedClosedGraphSides<'_, '_, '_> {
    pub fn source(&self) -> &QualifiedClosedBranches<'_, '_> {
        self.source
    }
    pub fn faces(&self) -> &[GraphFaceSide] {
        &self.faces
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let r = qualify_closed_graph_sides(self.source, self.limits)?;
        if self.faces != r.faces || self.operations != r.operations {
            return Err(unproven("Original closed graph sector sides changed"));
        }
        Ok(())
    }
}
fn remaining(used: usize, max: usize) -> Result<usize> {
    if used == max {
        return Err(unproven("No exact closed graph role work remains"));
    }
    Ok((max - used).min(4096))
}
/// Replays the complete actual partition before assigning either height side.
/// Every new actual pcurve coefficient must satisfy the original zero equation.
/// The audited simple closed cycle is therefore the unique complete ellipse,
/// and the entire audited outer/inner regions inherit its two strict signs.
/// No sample point, guessed winding, tolerance band or height proxy assigns a
/// role. Different original holes do not change a component's sign. This is
/// support-relative selection evidence, NOT other-trim or body material proof.
pub fn qualify_closed_graph_sides<'s, 'p, 'a>(
    source: &'s QualifiedClosedBranches<'p, 'a>,
    limits: ClosedGraphSideLimits,
) -> Result<QualifiedClosedGraphSides<'s, 'p, 'a>> {
    if !(1..=1000000).contains(&limits.operations) {
        return Err(invalid("Need bounded closed graph role work"));
    }
    let mut used = 0;
    charge(&mut used, limits.operations, source.operations)?;
    source.reverify()?;
    let original = source.membership.curves.original();
    let (signs, work) = original.prove_closed_height_sides(remaining(used, limits.operations)?)?;
    charge(&mut used, limits.operations, work)?;
    if source.regions.len() != 2
        || !(0..2).all(|side| source.regions.iter().filter(|r| r.side == side).count() == 1)
    {
        return Err(unproven(
            "Closed graph role needs the retained original ellipse on both sides",
        ));
    }
    let mut faces = Vec::new();
    for r in &source.regions {
        let model = &source.models[r.model];
        let inner = &model.faces[r.faces[1]];
        let outer = &model.faces[r.faces[0]];
        charge(
            &mut used,
            limits.operations,
            r.edges.len() + outer.holes.len() + 2,
        )?;
        let expected: std::collections::BTreeSet<_> = r.edges.iter().map(|e| e.edge).collect();
        let original_edges: std::collections::BTreeSet<_> =
            r.edges.iter().map(|e| e.original_edge).collect();
        if r.original_branch != 0
            || original_edges != original.branches()[0].edges.iter().copied().collect()
            || expected.len() != r.edges.len()
        {
            return Err(unproven(
                "Actual zero contour lost original complete branch ownership",
            ));
        }
        let is_contour = |wire: usize| {
            let c = &model.loops[wire].coedges;
            c.len() == expected.len()
                && c.iter()
                    .map(|c| c.edge)
                    .collect::<std::collections::BTreeSet<_>>()
                    == expected
        };
        let holes: Vec<_> = outer
            .holes
            .iter()
            .copied()
            .filter(|&wire| is_contour(wire))
            .collect();
        if !is_contour(inner.outer) || holes.len() != 1 {
            return Err(unproven(
                "Actual inner and outer cycles lost their complete shared zero contour",
            ));
        }
        for wire in [inner.outer, holes[0]] {
            for c in &model.loops[wire].coedges {
                charge(&mut used, limits.operations, 1)?;
                if c.trim().is_some_and(|t| {
                    t.curve_parameter != model.edges[c.edge].curve.domain()
                        || t.pcurve_parameter != c.pcurve.domain()
                        || t.periodic_lift != [[0; 2]; 2]
                }) {
                    return Err(unproven(
                        "Exact closed graph role needs complete actual coedge clocks",
                    ));
                }
                let work = original.prove_zero_trace(
                    &c.pcurve,
                    r.side,
                    remaining(used, limits.operations)?,
                )?;
                charge(&mut used, limits.operations, work)?;
            }
        }
        for (component, &face) in r.faces.iter().enumerate() {
            faces.push(GraphFaceSide {
                side: r.side,
                model: r.model,
                face,
                above: signs[component] ^ (r.side == 1),
                inner: component == 1,
            });
        }
    }
    Ok(QualifiedClosedGraphSides {
        source,
        limits,
        faces,
        operations: used,
    })
}
#[cfg(test)]
mod tests;
