//! Author full internal closed SS contours from complete source membership.
use super::*;
use crate::face_cut::{self, Segment};
mod paths;
use paths::*;
#[cfg(test)]
mod tests;
#[derive(Clone, Copy, Debug)]
pub struct ClosedBranchLimits {
    pub cells: usize,
    pub pairs: usize,
    pub operations: usize,
}
impl Default for ClosedBranchLimits {
    fn default() -> Self {
        Self {
            cells: 1000000,
            pairs: 100000,
            operations: 1000000,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClosedSourceEdge {
    pub original_edge: usize,
    pub edge: usize,
    pub reversed: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClosedBranchFaces {
    pub original_branch: usize,
    pub side: usize,
    pub model: usize,
    pub source_face: usize,
    /// Outer-with-hole then inner-with-original-holes, both retained.
    pub faces: [usize; 2],
    pub edges: Vec<ClosedSourceEdge>,
    pub vertices: Vec<usize>,
}
/// Immutable source ownership and fresh contour audits, not Boolean material
/// selection. Both sectors remain in the original shell on each selected side.
pub struct QualifiedClosedBranches<'p, 'a> {
    membership: &'p QualifiedMembership<'a>,
    limits: ClosedBranchLimits,
    models: Vec<Model>,
    side_model: [usize; 2],
    regions: Vec<ClosedBranchFaces>,
    cells: usize,
    pairs: usize,
    operations: usize,
}
impl QualifiedClosedBranches<'_, '_> {
    pub fn models(&self) -> &[Model] {
        &self.models
    }
    pub fn model_for_side(&self, side: usize) -> Option<&Model> {
        self.side_model.get(side).map(|&i| &self.models[i])
    }
    pub fn side_model(&self) -> [usize; 2] {
        self.side_model
    }
    pub fn regions(&self) -> &[ClosedBranchFaces] {
        &self.regions
    }
    pub fn cells(&self) -> usize {
        self.cells
    }
    pub fn pairs(&self) -> usize {
        self.pairs
    }
    pub fn operations(&self) -> usize {
        self.operations
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = partition_closed_branches(self.membership, self.limits)?;
        if self.models != fresh.models
            || self.side_model != fresh.side_model
            || self.regions != fresh.regions
            || self.cells != fresh.cells
            || self.pairs != fresh.pairs
            || self.operations != fresh.operations
        {
            return Err(unproven("Original closed SS branch construction changed"));
        }
        Ok(())
    }
}
fn charge(used: &mut usize, max: usize, n: usize) -> Result<()> {
    if n > max - *used {
        return Err(unproven("Shared closed SS branch budget exhausted"));
    }
    *used += n;
    Ok(())
}
/// Entire original branches and saved proxies must lie in both retained faces,
/// or be completely excluded from their intersection. Trim-crossing branches
/// need the root-bound open-contour constructor, never a partial closed owner.
pub fn partition_closed_branches<'p, 'a>(
    membership: &'p QualifiedMembership<'a>,
    limits: ClosedBranchLimits,
) -> Result<QualifiedClosedBranches<'p, 'a>> {
    if !(1..=1000000).contains(&limits.cells)
        || !(1..=100000).contains(&limits.pairs)
        || !(1..=1000000).contains(&limits.operations)
    {
        return Err(invalid("Need bounded closed SS contour work"));
    }
    let mut cells = 0;
    let mut pairs = 0;
    let mut operations = 0;
    charge(&mut cells, limits.cells, membership.work.cells)?;
    charge(&mut pairs, limits.pairs, membership.work.pairs)?;
    charge(
        &mut operations,
        limits.operations,
        membership.curves.original().operations(),
    )?;
    charge(
        &mut operations,
        limits.operations,
        membership.curves.operations(),
    )?;
    charge(
        &mut operations,
        limits.operations,
        membership.work.range_operations,
    )?;
    membership.reverify()?;
    let same = std::ptr::eq(membership.faces[0].0, membership.faces[1].0);
    let side_model = if same { [0, 0] } else { [0, 1] };
    let mut models = if same {
        vec![membership.faces[0].0.clone()]
    } else {
        vec![membership.faces[0].0.clone(), membership.faces[1].0.clone()]
    };
    let mut regions = Vec::new();
    let original = membership.curves.original();
    for (bi, branch) in original.branches().iter().enumerate() {
        if branch.edges.is_empty() {
            continue;
        }
        charge(
            &mut operations,
            limits.operations,
            branch.edges.len() + membership.edge_cells.len() + membership.vertices.len(),
        )?;
        let relevant: Vec<_> = membership
            .edge_cells
            .iter()
            .filter(|c| branch.edges.contains(&c.original_edge))
            .collect();
        let vertices: std::collections::BTreeSet<_> = branch
            .edges
            .iter()
            .flat_map(|&e| original.edges()[e].vertices)
            .collect();
        let points: Vec<_> = membership
            .vertices
            .iter()
            .filter(|v| vertices.contains(&v.original_vertex))
            .collect();
        if points.len() != vertices.len()
            || branch
                .edges
                .iter()
                .any(|&e| !relevant.iter().any(|c| c.original_edge == e))
        {
            return Err(unproven(
                "Closed SS branch lacks complete original membership",
            ));
        }
        let inside = |l: [Location; 2]| l == [Location::Inside; 2];
        let excluded = |l: [Location; 2]| l.contains(&Location::Outside);
        let all_inside = relevant.iter().all(|c| inside(c.locations))
            && points.iter().all(|p| inside(p.locations));
        let all_outside = relevant.iter().all(|c| excluded(c.locations))
            && points.iter().all(|p| excluded(p.locations));
        if all_outside {
            continue;
        }
        if !all_inside || !branch.closed {
            return Err(unproven(
                "Retained SS branch requires root-bound open-contour partitioning",
            ));
        }
        let ordered = cycle(
            membership.curves,
            &branch.edges,
            &mut operations,
            limits.operations,
        )?;
        for side in 0..2 {
            let slot = side_model[side];
            let face = membership.faces[side].1;
            for &(edge, _) in &ordered {
                charge(&mut operations, limits.operations, 1)?;
                let saved = &membership.curves.edges()[edge];
                if saved.world_error_upper > models[slot].tolerance_mm
                    || saved.uv_error_upper[side] > membership.tolerance_uv
                {
                    return Err(unproven(
                        "Closed SS source proxy needs accuracy within actual model/UV tolerance",
                    ));
                }
            }
            let path = segments(membership.curves, &ordered, side)?;
            if cells == limits.cells || pairs == limits.pairs {
                return Err(unproven("Closed SS face construction work exhausted"));
            }
            let result = face_cut::partition_closed(
                &models[slot],
                face,
                &path,
                membership.tolerance_uv,
                face_cut::ClosedCutLimits {
                    cells: limits.cells - cells,
                    pairs: limits.pairs - pairs,
                },
            )?;
            if !result.source_world_reference_exact || !result.source_uv_reference_exact {
                return Err(unproven(
                    "Closed SS construction lost exact qualified proxy clocks",
                ));
            }
            charge(&mut cells, limits.cells, result.cells)?;
            charge(&mut pairs, limits.pairs, result.pairs)?;
            let mut order = ordered.clone();
            if result.path_reversed {
                order.reverse();
                for (_, r) in &mut order {
                    *r = !*r;
                }
            }
            let mut edges = Vec::new();
            for ((original_edge, reversed), &edge) in order.into_iter().zip(&result.edges) {
                let proxy = &membership.curves.edges()[original_edge];
                let expected = if reversed {
                    proxy.world.reverse()?
                } else {
                    proxy.world.clone()
                };
                if result.model.edges[edge].curve != expected {
                    return Err(unproven(
                        "Authored closed SS edge differs from original qualified proxy",
                    ));
                }
                edges.push(ClosedSourceEdge {
                    original_edge,
                    edge,
                    reversed,
                });
            }
            regions.push(ClosedBranchFaces {
                original_branch: bi,
                side,
                model: slot,
                source_face: face,
                faces: result.faces,
                edges,
                vertices: result.vertices,
            });
            models[slot] = result.model;
        }
    }
    Ok(QualifiedClosedBranches {
        membership,
        limits,
        models,
        side_model,
        regions,
        cells,
        pairs,
        operations,
    })
}
