//! Actual shared-edge subdivision with original-parameter geometry evidence.
//! One common world partition, independently partitioned face-local pcurves.
use super::error;
use super::region_pattern::qualification;
pub use super::region_pattern::{Limits, Verification};
use crate::{Model, Result, TopoId, TopoKind};
use nurbs_core::{curve_partition_agreement as partition, curve_surface_agreement as lift};
mod checks;
#[cfg(feature = "codec")]
pub mod history;
mod naming;
pub mod reclock;
pub mod seam;
mod trim;
pub use trim::{AuthoredRelation, RelationFragment};
#[derive(Clone, Copy, Default)]
struct Work {
    cells: usize,
    pairs: usize,
    domain_cells: usize,
}
impl Work {
    fn remaining(self, l: Limits) -> Result<Limits> {
        let r = Limits {
            max_cells: l.max_cells.saturating_sub(self.cells),
            max_pairs: l.max_pairs.saturating_sub(self.pairs),
            max_domain_cells: l.max_domain_cells.saturating_sub(self.domain_cells),
            ..l
        };
        if r.max_cells == 0 || r.max_pairs == 0 || r.max_domain_cells == 0 {
            return Err(error("Shared edge partition work exhausted"));
        }
        Ok(r)
    }
    fn include(self, v: &mut Verification) {
        v.cells += self.cells;
        v.pairs += self.pairs;
        v.domain_cells += self.domain_cells;
    }
}
pub struct Split {
    source: Model,
    model: Model,
    edge: usize,
    cuts: Vec<f64>,
    key: String,
    limits: Limits,
    uv_tolerance: f64,
    edges: Vec<usize>,
    vertices: Vec<usize>,
    evidence: Verification,
    relations: Vec<AuthoredRelation>,
}
impl Split {
    /// Fresh exact identity for the actual stored 3D children on their original
    /// root clock. This is stronger than the tolerance evidence used for
    /// numerical partition proposals; None does not grant an exact identity.
    /// The functional certificate is separate from document/solid admission.
    pub fn exact_world_root_partition(&self,max_work:u64)
        -> Result<Option<nurbs_core::curve_partition_agreement::ExactBsplineRootPartition>> {
        let source=&self.source.edges[self.edge].curve;
        let pieces=self.edges.iter().map(|&i|self.model.edges[i].curve.clone()).collect::<Vec<_>>();
        nurbs_core::curve_partition_agreement::verify_exact_bspline_root_partition(
            source,&pieces,source.domain(),max_work)
    }
    pub fn relations(&self) -> &[AuthoredRelation] {
        &self.relations
    }
    pub fn source(&self) -> &Model {
        &self.source
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    /// Segments in increasing ORIGINAL 3D-curve parameter order.
    pub fn edges(&self) -> &[usize] {
        &self.edges
    }
    pub fn vertices(&self) -> &[usize] {
        &self.vertices
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    pub fn reverify(&self) -> Result<Verification> {
        let actual = split_with_uv_tolerance(
            &self.source,
            self.edge,
            &self.cuts,
            &self.key,
            self.uv_tolerance,
            self.limits,
        )?;
        if actual.model != self.model
            || actual.edges != self.edges
            || actual.vertices != self.vertices
            || actual.relations != self.relations
        {
            return Err(error(
                "Saved shared-edge partition differs from original reconstruction",
            ));
        }
        Ok(actual.evidence)
    }
}
/// Cuts are literal interior parameters of the original 3D edge, never inferred
/// from a face pcurve. This refines a closed model without removing material.
/// Authored trim relations retain exact rational fragment coverage in `relations`;
/// child scalar proposals require fresh original-world and stored-lift proofs.
pub fn split(
    source: &Model,
    edge: usize,
    cuts: &[f64],
    key: &str,
    limits: Limits,
) -> Result<Split> {
    split_with_uv_tolerance(source, edge, cuts, key, 1e-8, limits)
}
/// As `split`, with an explicit tolerance in the original face UV coordinates.
/// Actual world-curve preservation and all lift checks retain the model/local
/// THREE-DIMENSIONAL tolerance independently of this UV representation budget.
pub fn split_with_uv_tolerance(
    source: &Model,
    edge: usize,
    cuts: &[f64],
    key: &str,
    uv_tolerance: f64,
    limits: Limits,
) -> Result<Split> {
    if !uv_tolerance.is_finite() || uv_tolerance <= 0. {
        return Err(error("UV partition tolerance must be finite and positive"));
    }
    source.validate()?;
    if edge >= source.edges.len()
        || source.edges[edge].degenerate
        || cuts.is_empty()
        || cuts.len() > 16
        || key.is_empty()
        || key.len() > 128
    {
        return Err(error(
            "Choose a regular edge, 1..16 cuts and a bounded split key",
        ));
    }
    if !(1..=100000).contains(&limits.max_cells)
        || !(1..=100000).contains(&limits.max_pairs)
        || !(1..=1000000).contains(&limits.max_domain_cells)
        || !(100..=2000000).contains(&limits.mass_evaluations)
    {
        return Err(error("Choose bounded original edge refinement work"));
    }
    let original = &source.edges[edge].curve;
    let domain = original.domain();
    if cuts
        .iter()
        .any(|u| !u.is_finite() || *u <= domain[0] || *u >= domain[1])
        || cuts.windows(2).any(|w| w[0] >= w[1])
    {
        return Err(error(
            "Cuts need strictly increasing original interior parameters",
        ));
    }
    let uses = source
        .faces
        .iter()
        .enumerate()
        .flat_map(|(f, face)| {
            std::iter::once(face.outer)
                .chain(face.holes.iter().copied())
                .flat_map(move |w| {
                    source.loops[w]
                        .coedges
                        .iter()
                        .enumerate()
                        .filter_map(move |(slot, c)| {
                            if c.edge == edge {
                                Some((f, w, slot))
                            } else {
                                None
                            }
                        })
                })
        })
        .collect::<Vec<_>>();
    if uses.len() != 2 {
        return Err(error(
            "One closed-model edge with exactly two original coedge uses is required",
        ));
    }
    let mut work = Work::default();
    let source_check = qualification::verify_edge_refinement(source, work.remaining(limits)?)
        .map_err(|e| error(format!("Original edge source qualification: {e:?}")))?;
    work.cells += source_check.cells;
    work.pairs += source_check.pairs;
    work.domain_cells += source_check.domain_cells;
    let tol = source
        .1
        .local_tolerances_mm
        .get(&source.1.edges[edge])
        .copied()
        .unwrap_or(source.tolerance_mm)
        .min(source.tolerance_mm);
    for (face, wire, slot) in &uses {
        let c = &source.loops[*wire].coedges[*slot];
        let full = lift::verify(
            original,
            &c.pcurve,
            &source.faces[*face].surface,
            c.reversed,
            tol,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += full.cells;
        if full.status != lift::Status::WithinTolerance {
            return Err(error(
                "Original selected edge lift exceeds its local tolerance",
            ));
        }
    }
    let world = partition::partition(original, cuts, tol, work.remaining(limits)?.max_cells)
        .map_err(|e| error(format!("Original world partition: {e:?}")))?;
    work.cells += world.visited;
    if !world.geometry_preserved
        || !world.endpoint_joins_exact
        || world.periodic_closure_exact == Some(false)
    {
        return Err(error("Original 3D edge partition remains unproven"));
    }
    let n = world.pieces.len();
    let mut model = source.clone();
    let mut vertices = Vec::new();
    for piece in &world.pieces[..n - 1] {
        let p = piece.curve.control_points.last().unwrap();
        vertices.push(model.vertices.len());
        model.vertices.push(crate::Vertex {
            point: [p[0], p[1], p[2]],
        });
    }
    let mut chain = vec![source.edges[edge].vertices[0]];
    chain.extend(&vertices);
    chain.push(source.edges[edge].vertices[1]);
    let mut edges = Vec::new();
    for (i, piece) in world.pieces.iter().enumerate() {
        let e = crate::Edge {
            curve: piece.curve.clone(),
            vertices: [chain[i], chain[i + 1]],
            degenerate: false,
        };
        if i == 0 {
            model.edges[edge] = e;
            edges.push(edge);
        } else {
            edges.push(model.edges.len());
            model.edges.push(e);
        }
    }
    let mut relations = Vec::new();
    let mut changed = std::collections::BTreeSet::new();
    // Work backwards within a wire if an original seam has two uses there.
    let mut uses = uses;
    uses.sort_by_key(|(_, w, slot)| (*w, std::cmp::Reverse(*slot)));
    for (face, wire, slot) in uses {
        let old = &source.loops[wire].coedges[slot];
        let pc_domain = old.pcurve.domain();
        let mut transfer = trim::Transfer::new(source, face, wire, slot)?;
        let mut pc_cuts = cuts
            .iter()
            .map(|u| {
                let mut t = (*u - domain[0]) / (domain[1] - domain[0]);
                if old.reversed {
                    t = 1. - t;
                }
                pc_domain[0] + (pc_domain[1] - pc_domain[0]) * t
            })
            .collect::<Vec<_>>();
        if old.reversed {
            pc_cuts.reverse();
        }
        let uv = partition::partition(
            &old.pcurve,
            &pc_cuts,
            uv_tolerance,
            work.remaining(limits)?.max_cells,
        )?;
        work.cells += uv.visited;
        if !uv.geometry_preserved
            || !uv.endpoint_joins_exact
            || uv.periodic_closure_exact == Some(false)
        {
            return Err(error(format!(
                "Original face-local pcurve partition remains unproven: face={face}, cells={}, geometry={}, exact_joins={}",
                uv.visited, uv.geometry_preserved, uv.endpoint_joins_exact
            )));
        }
        let mut replacement = Vec::new();
        for (i, piece) in uv.pieces.iter().enumerate() {
            let j = if old.reversed { n - 1 - i } else { i };
            let checked = lift::verify(
                &world.pieces[j].curve,
                &piece.curve,
                &source.faces[face].surface,
                old.reversed,
                tol,
                work.remaining(limits)?.max_cells,
            )?;
            work.cells += checked.cells;
            if checked.status != lift::Status::WithinTolerance {
                return Err(error(
                    "Shared world segment and actual face pcurve disagree",
                ));
            }
            let mut coedge = crate::Coedge::new(edges[j], old.reversed, piece.curve.clone());
            if let Some(t) = &mut transfer {
                if let Some(metadata) = t.piece(
                    edges[j],
                    &world.pieces[j],
                    &piece.curve,
                    tol,
                    limits,
                    &mut work,
                )? {
                    coedge = coedge.with_trim(metadata)?;
                }
            }
            replacement.push(coedge);
        }
        if let Some(t) = transfer {
            relations.push(t.finish()?);
        }
        model.loops[wire]
            .coedges
            .splice(slot..slot + 1, replacement);
        changed.insert(wire);
    }
    naming::assign(source, &mut model, edge, &edges, &vertices, &changed, key)
        .map_err(|e| error(format!("Stored edge topology: {e:?}")))?;
    checks::endpoints(&model, &edges, limits, &mut work)?;
    let mut evidence = qualification::verify_edge_refinement(&model, work.remaining(limits)?)
        .map_err(|e| error(format!("Stored edge qualification: {e:?}")))?;
    work.include(&mut evidence);
    Ok(Split {
        source: source.clone(),
        model,
        edge,
        cuts: cuts.to_vec(),
        key: key.into(),
        limits,
        uv_tolerance,
        edges,
        vertices,
        evidence,
        relations,
    })
}
