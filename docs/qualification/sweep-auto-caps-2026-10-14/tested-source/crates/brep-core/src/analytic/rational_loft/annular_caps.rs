//! Source-owned smooth annular caps. No cached global material admission.
use super::{Curve, Model, ProgressiveBodyEvidence, Result, err};
use crate::retained_annular_cap::{self, reference_filling::Limits, source_filling};
use nurbs_core::{progressive_sweep::MultiSweep, sweeps::filled_cap_error::BoundaryCertificate};

pub struct CapEvidence {
    groups: [Vec<usize>; 2],
    filling: source_filling::Report,
    construction_work: u64,
    selection_work:u64,
}
impl CapEvidence {
    pub fn face_groups(&self) -> &[Vec<usize>; 2] {
        &self.groups
    }
    pub fn faces(&self) -> Vec<usize> {
        self.groups.iter().flatten().copied().collect()
    }
    pub fn filling(&self) -> &source_filling::Report {
        &self.filling
    }
    pub fn construction_work(&self) -> u64 {
        self.construction_work
    }
    pub fn exact_work(&self) -> u64 {
        self.selection_work + self.construction_work + self.filling.exact_work
    }
}

/// Immutable owner of the actual model and its original-source boundary union.
pub struct Boundary {
    body: ProgressiveBodyEvidence,
}
impl Boundary {
    pub(super) fn new(body: ProgressiveBodyEvidence) -> Self {
        Self { body }
    }
    pub fn model(&self) -> &Model {
        &self.body.model
    }
    pub fn evidence(&self) -> &ProgressiveBodyEvidence {
        &self.body
    }
    pub fn caps(&self) -> Option<&CapEvidence> {
        self.body.annular_caps.as_ref()
    }
    pub fn cap_faces(&self) -> Vec<usize> {
        self.caps().map_or_else(||if self.body.station_closed {Vec::new()}else{
            vec![self.body.model.faces.len()-2,self.body.model.faces.len()-1]
        }, CapEvidence::faces)
    }
    pub fn cap_selection_work(&self)->u64 {self.body.cap_selection_work}
    pub fn cap_face_groups(&self)->[Vec<usize>;2] {
        self.caps().map_or_else(||{let faces=self.cap_faces();if faces.is_empty(){[vec![],vec![]]}else{[vec![faces[0]],vec![faces[1]]]}},|caps|caps.face_groups().clone())
    }
    pub fn planar_cap_faces(&self)->Vec<usize> {if self.caps().is_some(){vec![]}else{self.cap_faces()}}
    pub fn boundary(&self) -> &BoundaryCertificate {
        &self.body.station_boundary
    }
    pub fn complete_boundary(&self) -> Option<&BoundaryCertificate> {
        let boundary = self.boundary();
        (boundary.continuous_bound && boundary.within_budget == Some(true)).then_some(boundary)
    }
    pub fn reconstruct_stations(
        &self,
        quantum: f64,
        tolerance: f64,
        max_work: u64,
    ) -> Result<crate::sweep_miter_owned::StationBoundary> {
        self.body.reconstruct_stations(quantum, tolerance, max_work)
    }
    /// Actual saved jets only. Sharp cap/wall joins remain outside this audit.
    pub fn inspect_smoothness(
        &self,
        max_work: u64,
    ) -> Result<crate::sweep_miter_owned::annular::Smoothness> {
        let walls =
            crate::sweep_smoothness::inspect_profile(self.model(), &self.cap_faces(), max_work)?;
        let mut work = walls.total_exact_work;
        let mut cap_seams = Vec::new();
        if let Some(caps) = self.caps() {
            for group in caps.face_groups() {
                if group.len() < 2 {
                    continue;
                }
                let excluded = (0..self.model().faces.len())
                    .filter(|f| !group.contains(f))
                    .collect::<Vec<_>>();
                let report = crate::sweep_smoothness::inspect_axis(
                    self.model(),
                    &excluded,
                    crate::sweep_smoothness::Axis::Profile,
                    max_work - work,
                )?;
                work += report.exact_work;
                cap_seams.push(report);
            }
        }
        Ok(crate::sweep_miter_owned::annular::Smoothness {
            walls,
            cap_seams,
            exact_work: work,
        })
    }
}

pub(super) fn validate_limits(limits: Limits) -> Result<()> {
    if limits.exact_work > 1_000_000
        || limits.products > 100_000
        || limits.trim_pairs > 100_000
        || limits.trim_cells > 100_000
        || limits.trim_domain_cells > 1_000_000
        || limits.spans > 1024
        || limits.linear_cells > 100_000
        || limits.contacts.pairs > 100_000
        || limits.contacts.cells > 100_000
        || limits.contacts.domain_cells > 1_000_000
        || limits.contacts.cells_per_pair > 100_000
        || limits.contacts.domain_cells_per_pair > 1_000_000
    {
        return Err(err("Invalid or exhausted annular progressive cap limits"));
    }
    Ok(())
}

struct Pair {
    model: Model,
    groups: [Vec<usize>; 2],
    work: u64,
}
fn replace_pair(model: &Model, max_work: u64) -> Result<Pair> {
    model.validate()?;
    if model.faces.len() < 2 {
        return Err(err("Missing original progressive end caps"));
    }
    let caps = [model.faces.len() - 2, model.faces.len() - 1];
    let first = retained_annular_cap::replace(model, caps[0], max_work)?;
    let last = retained_annular_cap::replace(
        &first.model,
        first.source_face_map[caps[1]].unwrap(),
        max_work - first.exact_work,
    )?;
    let groups = [
        first
            .cap_faces
            .iter()
            .map(|&f| last.source_face_map[f].unwrap())
            .collect(),
        last.cap_faces,
    ];
    Ok(Pair {
        model: last.model,
        groups,
        work: first.exact_work + last.exact_work,
    })
}

pub(super) struct Assembled {
    pub model: Model,
    pub evidence: CapEvidence,
}
pub(super) fn assemble(
    source: &MultiSweep<'_>,
    model: &Model,
    sizes: &[usize],
    tolerance: f64,
    tolerance_uv: f64,
    limits: Limits,
    selection_work:u64,
) -> Result<Assembled> {
    validate_limits(limits)?;
    let available=limits.exact_work.checked_sub(selection_work).ok_or_else(||err("Cap selection work exhausted"))?;
    let pair = replace_pair(model, available)?;
    let filling = source_filling::inspect_smooth(
        source,
        &pair.model,
        &pair.groups,
        sizes,
        tolerance,
        tolerance_uv,
        Limits {
            exact_work: available - pair.work,
            ..limits
        },
    )?;
    Ok(Assembled {
        model: pair.model,
        evidence: CapEvidence {
            groups: pair.groups,
            filling,
            construction_work: pair.work,
            selection_work,
        },
    })
}

fn preserve_groups(source: &Model, candidate: &Model, groups: &[Vec<usize>; 2]) -> Result<()> {
    source.validate()?;
    candidate.validate()?;
    if source.faces.len() != candidate.faces.len() {
        return Err(err("Station smoothing changed annular cap ownership"));
    }
    for &face in groups.iter().flatten() {
        if source.faces[face] != candidate.faces[face] {
            return Err(err("Station smoothing changed an annular cap"));
        }
        let cap = &source.faces[face];
        for &wire in std::iter::once(&cap.outer).chain(&cap.holes) {
            if source.loops[wire] != candidate.loops[wire] {
                return Err(err("Station smoothing changed annular trims"));
            }
            for coedge in &source.loops[wire].coedges {
                let edge = &source.edges[coedge.edge];
                if *edge != candidate.edges[coedge.edge] {
                    return Err(err("Station smoothing changed an annular boundary edge"));
                }
                for &vertex in &edge.vertices {
                    if source.vertices[vertex] != candidate.vertices[vertex] {
                        return Err(err("Station smoothing changed an annular boundary vertex"));
                    }
                }
            }
        }
    }
    Ok(())
}

/// Replay private source sections, then preserve every physical cap face.
/// Replay, wall reconstruction and both cap assemblies consume one allowance.
pub(super) fn reconstruct(
    source: &Model,
    sections: &[Vec<Vec<Curve>>],
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<crate::sweep_station_reconstruction::Reconstruction> {
    if max_work > 1_000_000 {
        return Err(err("Annular station work exceeds existing limit"));
    }
    let raw = super::rational_section_loft(sections)?;
    let replay = replace_pair(&raw, max_work)?;
    if replay.model != *source {
        return Err(err("Station sections do not reproduce owned annular body"));
    }
    let mut rebuilt = crate::sweep_station_reconstruction::reconstruct(
        &raw,
        sections,
        &[],
        false,
        quantum,
        tolerance,
        max_work - replay.work,
    )?;
    rebuilt.candidate.work += replay.work;
    if let Some(candidate) = rebuilt.model.take() {
        let pair = replace_pair(&candidate, max_work - rebuilt.candidate.work)?;
        rebuilt.candidate.work += pair.work;
        if pair.groups != replay.groups {
            return Err(err("Station smoothing changed logical cap groups"));
        }
        preserve_groups(source, &pair.model, &replay.groups)?;
        rebuilt.model = Some(pair.model);
    }
    Ok(rebuilt)
}

#[cfg(test)]
mod tests;
