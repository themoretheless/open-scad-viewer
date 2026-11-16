use super::*;
use crate::closed_solid::{
    BooleanFace, BooleanFaceDisposition, BooleanFaceSeedLimits, BooleanMaterialOp,
    BooleanSourceSide, QualifiedClosedSolid,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenSSBooleanEdgeUse {
    pub model: usize,
    pub face: usize,
    pub edge: usize,
    /// Boundary direction after applying the Boolean face orientation.
    pub reversed: bool,
}

struct SelectedUse {
    edge: OpenSSBooleanEdgeUse,
    reverse_material: bool,
}

impl QualifiedOpenSSNetwork<'_, '_, '_, '_, '_> {
    /// Match selected boundary uses on the two copies of each open SS edge.
    /// For XOR each edge contributes separate seams for A−B and B−A, in order.
    /// Unrelated rims still require the other face pairs of the Boolean.
    pub fn qualify_boolean_seams(
        &self,
        solids: [&QualifiedClosedSolid; 2],
        operation: BooleanMaterialOp,
        directions: &[[f64; 3]],
        limits: BooleanFaceSeedLimits,
    ) -> Result<Vec<[OpenSSBooleanEdgeUse; 2]>> {
        let input = match &self.source {
            NetworkSource::Paths(paths) => paths[0].input,
            NetworkSource::Boundary { input, .. } => input,
        };
        let mut selected = [Vec::new(), Vec::new()];
        for (side, source_side) in [BooleanSourceSide::First, BooleanSourceSide::Second]
            .into_iter()
            .enumerate()
        {
            let face = input.source.roots.faces[side].1;
            let orientation = solids[side]
                .model()
                .shells
                .iter()
                .flat_map(|s| &s.faces)
                .find(|use_| use_.face == face)
                .ok_or_else(|| unproven("SS Boolean source lacks its material face use"))?
                .reversed;
            let query = BooleanFace {
                solids,
                operation,
                side: source_side,
                face,
            };
            selected[side] = self
                .discover_boolean_descendants(query, directions, limits)?
                .into_iter()
                .filter_map(|(face, disposition)| match disposition {
                    BooleanFaceDisposition::Discard => None,
                    BooleanFaceDisposition::Keep => Some((face, orientation, false)),
                    BooleanFaceDisposition::Reverse => Some((face, !orientation, true)),
                })
                .collect();
        }
        let mut seams = Vec::new();
        let mut used = std::collections::BTreeSet::new();
        for cut in &self.cuts {
            let [a, b] = &cut.sides;
            if a.edges.len() != b.edges.len() {
                return Err(unproven("Paired SS cut lost an edge on one operand"));
            }
            for (&first, &second) in a.edges.iter().zip(&b.edges) {
                let lhs = edge_uses(&self.models[a.model], a.model, first, &selected[0]);
                let rhs = edge_uses(&self.models[b.model], b.model, second, &selected[1]);
                for [left, right] in material_pairs(operation, &lhs, &rhs)? {
                    let first = &self.models[left.model].edges[left.edge];
                    let second = &self.models[right.model].edges[right.edge];
                    let same_ports = first.vertices.iter().zip(second.vertices).all(|(&u, v)| {
                        self.models[left.model].vertices[u].point
                            == self.models[right.model].vertices[v].point
                    });
                    if left.reversed == right.reversed
                        || first.curve != second.curve
                        || !same_ports
                        || !used.insert((left.model, left.face, left.edge))
                        || !used.insert((right.model, right.face, right.edge))
                    {
                        return Err(unproven(
                            "Selected SS seam lacks exact opposite oriented edge uses",
                        ));
                    }
                    seams.push([left, right]);
                }
            }
        }
        Ok(seams)
    }
}

fn material_pairs(
    operation: BooleanMaterialOp,
    left: &[SelectedUse],
    right: &[SelectedUse],
) -> Result<Vec<[OpenSSBooleanEdgeUse; 2]>> {
    match (operation, left, right) {
        (_, [], []) => Ok(Vec::new()),
        (BooleanMaterialOp::Xor, [a, b], [c, d])
            if a.reverse_material != b.reverse_material
                && c.reverse_material != d.reverse_material =>
        {
            let [keep_a, reverse_a] = if a.reverse_material { [b, a] } else { [a, b] };
            let [keep_b, reverse_b] = if c.reverse_material { [d, c] } else { [c, d] };
            Ok(vec![
                [keep_a.edge, reverse_b.edge],
                [reverse_a.edge, keep_b.edge],
            ])
        }
        (BooleanMaterialOp::Xor, _, _) => Err(unproven(
            "XOR SS seam needs both material sectors on both operands",
        )),
        (_, [a], [b]) => Ok(vec![[a.edge, b.edge]]),
        _ => Err(unproven(
            "Selected SS edge needs exactly one face use on each operand",
        )),
    }
}

fn edge_uses(
    model: &Model,
    slot: usize,
    edge: usize,
    selected: &[(usize, bool, bool)],
) -> Vec<SelectedUse> {
    let mut uses = Vec::new();
    for &(face, reversed, reverse_material) in selected {
        let region = &model.faces[face];
        for wire in std::iter::once(region.outer).chain(region.holes.iter().copied()) {
            for coedge in &model.loops[wire].coedges {
                if coedge.edge == edge {
                    uses.push(SelectedUse {
                        edge: OpenSSBooleanEdgeUse {
                            model: slot,
                            face,
                            edge,
                            reversed: reversed ^ coedge.reversed,
                        },
                        reverse_material,
                    });
                }
            }
        }
    }
    uses
}
