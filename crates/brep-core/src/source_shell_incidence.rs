//! Closed oriented incidence of original surface-composed wires.
//! This is not a certificate of embedded faces, material orientation or volume.
use crate::{
    source_shared_edge::{self, SharedEdge},
    source_world_wire::Wire,
};
use nurbs_core::{Error, Result, curve::Curve};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    pub face: usize,
    pub wire: usize,
    pub edge: usize,
}
pub struct Pair {
    pub uses: [Address; 2],
    pub world: Curve,
    pub world_reversed: [bool; 2],
    pub cutters: [Option<Curve>; 2],
}
/// Raw plane inputs, indexed by canonical pair and first-use endpoint.
/// Assembly recomputes all plane/root identities; these are not certificates.
pub struct RootPlanes {
    pub pair: usize,
    pub planes: [Option<[[f64; 3]; 3]>; 2],
}
/// Original-use affine ranges on a canonical carrier; rechecked during assembly.
pub struct AffineMaps {
    pub pair: usize,
    pub ranges: [[[f64; 2]; 2]; 2],
}
/// Raw UV parameter witnesses indexed by pair, use and directed endpoint.
pub struct RootCandidates {
    pub pair: usize,
    pub candidates: [[Option<[f64; 2]>; 2]; 2],
}
pub struct Pole {
    pub use_: Address,
    pub point: [f64; 3],
}
pub struct Shell<E = SharedEdge> {
    poles: Vec<(Address, crate::source_collapsed_boundary::CollapsedBoundary)>,
    faces: Vec<Vec<Wire>>,
    edges: Vec<E>,
    uses: Vec<[Address; 2]>,
    vertices: Vec<Vec<Vec<[usize; 2]>>>,
    regions: Option<Vec<crate::source_contour_proposal::SourceRegion>>,
}
impl Shell {
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> Result<value_codec::Value> {
        crate::source_shell_restore::definition(self)
    }
}
impl<E> Shell<E> {
    pub fn poles(&self) -> &[(Address, crate::source_collapsed_boundary::CollapsedBoundary)] {
        &self.poles
    }
    pub fn regions(&self) -> Option<&[crate::source_contour_proposal::SourceRegion]> {
        self.regions.as_deref()
    }
    pub fn faces(&self) -> &[Vec<Wire>] {
        &self.faces
    }
    pub fn edges(&self) -> &[E] {
        &self.edges
    }
    pub fn uses(&self) -> &[[Address; 2]] {
        &self.uses
    }
    pub fn vertices(&self) -> &[Vec<Vec<[usize; 2]>>] {
        &self.vertices
    }
}
pub struct ChartFace {
    pub face: usize,
    pub contraction: Option<nurbs_core::surface_injectivity::Report>,
    pub linear: Option<nurbs_core::surface_linear_monotonicity::Report>,
    pub quotient: Option<nurbs_core::surface_quotient_injectivity::Report>,
    pub ruled_quotient: Option<nurbs_core::surface_quotient_injectivity::RuledReport>,
    pub polar_quotient: Option<nurbs_core::surface_quotient_injectivity::PolarReport>,
}
impl ChartFace {
    pub fn injectivity_proven(&self) -> bool {
        self.contraction.as_ref().is_some_and(|r| r.proven)
            || self.linear.as_ref().is_some_and(|r| r.certified)
            || self.quotient.as_ref().is_some_and(|r| r.proven)
            || self.ruled_quotient.as_ref().is_some_and(|r| r.proven)
            || self.polar_quotient.as_ref().is_some_and(|r| r.proven)
    }
}
pub struct ChartReport {
    pub all_injective: bool,
    pub spans: usize,
    pub linear_cells: usize,
    pub faces: Vec<ChartFace>,
}

pub struct Report<E = SharedEdge> {
    pub shell: Option<Shell<E>>,
    pub work_used: u64,
    pub root_checks: usize,
    pub driver_cells: usize,
    pub uncertain_pair: Option<usize>,
    pub reason: &'static str,
}
fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_SHELL", message)
}
fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
fn join(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[b] = a;
}
/// Assemble only immutable qualified original/replacement UV regions.
/// Their source loop payloads become the exact wire payload being paired.
pub fn assemble_regions(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    max_work: u64,
) -> Result<Report> {
    assemble_regions_with_root_planes(regions, pairs, &[], max_work, 0)
}
pub fn assemble_regions_with_root_planes(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_regions_with_maps(regions, pairs, planes, &[], max_work, max_driver_cells)
}
pub fn assemble_regions_with_maps(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_regions_with_endpoint_inputs(
        regions,
        pairs,
        planes,
        maps,
        &[],
        max_work,
        max_driver_cells,
    )
}
pub fn assemble_regions_with_endpoint_inputs(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    if regions.len() < 2 || regions.len() > 4096 || max_work == 0 || max_work > 100_000_000 {
        return Err(error(
            "Choose bounded qualified regions and exact shell work",
        ));
    }
    let faces = regions
        .iter()
        .map(|r| r.world_wires())
        .collect::<Result<Vec<_>>>()?;
    let mut report = assemble_with_endpoint_inputs(
        &faces,
        pairs,
        planes,
        maps,
        candidates,
        max_work,
        max_driver_cells,
    )?;
    if let Some(shell) = report.shell.as_mut() {
        shell.regions = Some(regions.to_vec());
    }
    Ok(report)
}
/// Qualified original regions retain their material ownership through pole contraction.
pub fn assemble_regions_with_poles(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    poles: &[Pole],
    max_work: u64,
) -> Result<Report> {
    if regions.len() < 2 || regions.len() > 4096 || max_work == 0 || max_work > 100_000_000 {
        return Err(error(
            "Choose bounded qualified regions and exact shell work",
        ));
    }
    let faces = regions
        .iter()
        .map(|r| r.world_wires())
        .collect::<Result<Vec<_>>>()?;
    let mut report = assemble_with_poles(&faces, pairs, poles, max_work)?;
    if let Some(shell) = report.shell.as_mut() {
        shell.regions = Some(regions.to_vec());
    }
    Ok(report)
}
/// Recompute qualified material incidence with all root inputs and pole uses.
pub fn assemble_regions_with_pole_inputs(
    regions:&[crate::source_contour_proposal::SourceRegion],pairs:&[Pair],
    planes:&[RootPlanes],maps:&[AffineMaps],candidates:&[RootCandidates],poles:&[Pole],
    max_work:u64,max_driver_cells:usize,
)->Result<Report> {
    if regions.len()<2 || regions.len()>4096 {return Err(error("Choose bounded qualified regions"));}
    let faces=regions.iter().map(|r|r.world_wires()).collect::<Result<Vec<_>>>()?;
    let mut report=assemble_with_pole_inputs(&faces,pairs,planes,maps,candidates,poles,max_work,max_driver_cells)?;
    if let Some(shell)=report.shell.as_mut() {shell.regions=Some(regions.to_vec());}
    Ok(report)
}
/// Recheck every canonical pair. Each directed use must appear exactly once.
/// Source joins own local vertices; exact opposite pairs own cross-face vertices.
pub fn assemble(faces: &[Vec<Wire>], pairs: &[Pair], max_work: u64) -> Result<Report> {
    assemble_with_root_planes(faces, pairs, &[], max_work, 0)
}
pub fn assemble_with_root_planes(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_with_maps(faces, pairs, planes, &[], max_work, max_driver_cells)
}
pub fn assemble_with_maps(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_with_endpoint_inputs(faces, pairs, planes, maps, &[], max_work, max_driver_cells)
}
pub fn assemble_with_endpoint_inputs(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
    assemble_with_pole_inputs(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        max_work,
        max_driver_cells,
    )
}
/// Contract only independently qualified original collapsed boundaries.
/// Ordinary edges still require exactly one opposite partner.
pub fn assemble_with_poles(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    poles: &[Pole],
    max_work: u64,
) -> Result<Report> {
    assemble_with_pole_inputs(faces, pairs, &[], &[], &[], poles, max_work, 0)
}
type Ranges=[[[f64;2];2];2];
type Planes=[Option<[[f64;3];3]>;2];
type Candidates=[[Option<[f64;2]>;2];2];
struct PairInput<'a> {
    index:usize,pair:&'a Pair,sources:[&'a crate::source_boundary_fragment::Fragment;2],
    ranges:Option<Ranges>,planes:Planes,candidates:Option<Candidates>,
}
struct EdgeReport<E> {edge:Option<E>,work_used:u64,root_checks:usize,driver_cells:usize,reason:&'static str}

/// Original rational-linear map proposals, rechecked for every paired use.
pub struct ProjectiveMaps {
    pub pair: usize,
    pub ranges: Ranges,
    pub weights: [[f64; 2]; 2],
}
/// Original boundary/contact parameter fractions for addressed retained roots.
pub struct RootFractionCandidates {
    pub pair: usize,
    pub candidates: crate::source_projective_edge::FractionCandidates,
}

/// Closed oriented incidence using freshly qualified projective shared edges.
/// This establishes edge/vertex ownership, not embedded material or volume.
pub fn assemble_projective(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    assemble_projective_with_poles(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        max_work,
        max_driver_cells,
    )
}
pub fn assemble_projective_with_poles(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    assemble_projective_with_fraction_candidates(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        poles,
        max_work,
        max_driver_cells,
    )
}
/// Candidate proposals have no authority until each original root is rechecked.
pub fn assemble_projective_with_fraction_candidates(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    fraction_candidates: &[RootFractionCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    assemble_projective_fraction_impl(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        fraction_candidates,
        poles,
        None,
        max_work,
        max_driver_cells,
    )
}
/// Discover rational original-root parameters under the same shell work limit.
pub fn assemble_projective_with_discovered_fraction_candidates(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_denominator: usize,
    max_candidates_per_axis: usize,
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    if !(1..=4096).contains(&max_denominator) || !(1..=64).contains(&max_candidates_per_axis) {
        return Err(error("Bounded rational discovery required"));
    }
    assemble_projective_fraction_impl(
        faces,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        poles,
        Some([max_denominator, max_candidates_per_axis]),
        max_work,
        max_driver_cells,
    )
}

/// Retain freshly qualified material-region ownership through projective pairing.
pub fn assemble_regions_projective(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    assemble_regions_projective_with_poles(
        regions,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        max_work,
        max_driver_cells,
    )
}
pub fn assemble_regions_projective_with_poles(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    assemble_regions_projective_with_fraction_candidates(
        regions,
        pairs,
        planes,
        maps,
        candidates,
        &[],
        poles,
        max_work,
        max_driver_cells,
    )
}
/// Preserve material regions while transporting exact rational root proposals.
pub fn assemble_regions_projective_with_fraction_candidates(
    regions: &[crate::source_contour_proposal::SourceRegion],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[ProjectiveMaps],
    candidates: &[RootCandidates],
    fraction_candidates: &[RootFractionCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report<crate::source_projective_edge::Edge>> {
    if !(2..=4096).contains(&regions.len()) {
        return Err(error("Choose bounded qualified source regions"));
    }
    let faces = regions
        .iter()
        .map(|r| r.world_wires())
        .collect::<Result<Vec<_>>>()?;
    let mut report = assemble_projective_with_fraction_candidates(
        &faces,
        pairs,
        planes,
        maps,
        candidates,
        fraction_candidates,
        poles,
        max_work,
        max_driver_cells,
    )?;
    if let Some(shell) = report.shell.as_mut() {
        shell.regions = Some(regions.to_vec());
    }
    Ok(report)
}

fn assemble_core<E>(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
    mut qualify: impl FnMut(PairInput<'_>, u64, usize) -> Result<EdgeReport<E>>,
) -> Result<Report<E>> {
    if faces.len() < 2
        || faces.len() > 4096
        || max_work == 0
        || max_work > 100_000_000
        || max_driver_cells > 100000
    {
        return Err(error("Choose bounded faces and exact shell work"));
    }
    let mut offsets = Vec::new();
    let mut total = 0;
    for face in faces {
        if face.is_empty() || face.len() > 16 {
            return Err(error("Face needs bounded original wires"));
        }
        let surface = face[0].edges()[0].surface();
        let mut starts = Vec::new();
        for wire in face {
            if wire.edges().iter().any(|e| e.surface() != surface) {
                return Err(error("Face wires must share the original surface"));
            }
            starts.push(total);
            total += wire.edges().len();
            if total > 65536 {
                return Err(error("Source shell incidence work limit"));
            }
        }
        offsets.push(starts);
    }
    if pairs
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(poles.len()))
        != Some(total)
    {
        return Err(error("Every directed source edge requires one partner"));
    }
    let mut supplied = BTreeMap::new();
    for spec in planes {
        if spec.pair >= pairs.len()
            || supplied.insert(spec.pair, spec.planes).is_some()
            || spec
                .planes
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(error(
                "Plane inputs need unique valid pair addresses and finite anchors",
            ));
        }
    }
    let mut mapped = BTreeMap::new();
    for spec in maps {
        if spec.pair >= pairs.len()
            || mapped.insert(spec.pair, spec.ranges).is_some()
            || spec
                .ranges
                .iter()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || pairs[spec.pair].cutters.iter().any(Option::is_some)
        {
            return Err(error(
                "Affine maps need unique valid pair addresses, finite fractions and no cutter hints",
            ));
        }
    }
    let mut witnessed = BTreeMap::new();
    for spec in candidates {
        if spec.pair >= pairs.len()
            || witnessed.insert(spec.pair, spec.candidates).is_some()
            || spec
                .candidates
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || pairs[spec.pair].cutters.iter().any(Option::is_some)
        {
            return Err(error(
                "Root witnesses need unique valid addresses, finite original parameters and no cutter hints",
            ));
        }
    }
    let lookup = |a: Address| -> Result<usize> {
        let wire = faces
            .get(a.face)
            .and_then(|f| f.get(a.wire))
            .ok_or_else(|| error("Unknown source wire address"))?;
        if a.edge >= wire.edges().len() {
            return Err(error("Unknown source edge address"));
        }
        Ok(offsets[a.face][a.wire] + a.edge)
    };
    let mut seen = vec![false; total];
    for p in pairs {
        if p.uses[0].face == p.uses[1].face {
            return Err(error("Pair must join distinct faces"));
        }
        for &a in &p.uses {
            let i = lookup(a)?;
            if seen[i] {
                return Err(error("Source edge use paired more than once"));
            }
            seen[i] = true;
        }
    }
    for pole in poles {
        let i = lookup(pole.use_)?;
        if seen[i] {
            return Err(error("Source pole use claimed more than once"));
        }
        seen[i] = true;
    }
    let mut out = Report {
        shell: None,
        work_used: 0,
        root_checks: 0,
        driver_cells: 0,
        uncertain_pair: None,
        reason: "source-shell-pair-unqualified",
    };
    let mut parent: Vec<_> = (0..total).collect();
    let mut face_parent: Vec<_> = (0..faces.len()).collect();
    let mut edges = Vec::new();
    let mut qualified_poles = Vec::new();
    for pole in poles {
        if out.work_used == max_work {
            out.reason = "source-shell-pole-work-limit";
            return Ok(out);
        }
        let a = pole.use_;
        let source = &faces[a.face][a.wire].edges()[a.edge];
        let report = crate::source_collapsed_boundary::qualify(
            source,
            pole.point,
            (max_work - out.work_used).min(cad_predicates::MAX_WORK),
        )?;
        out.work_used += report.work_used;
        let Some(proof) = report.boundary else {
            out.reason = report.reason;
            return Ok(out);
        };
        let vertices = faces[a.face][a.wire].vertices()[a.edge];
        join(
            &mut parent,
            offsets[a.face][a.wire] + vertices[0],
            offsets[a.face][a.wire] + vertices[1],
        );
        qualified_poles.push((a, proof));
    }
    for (i, p) in pairs.iter().enumerate() {
        if out.work_used == max_work {
            out.uncertain_pair = Some(i);
            out.reason = "source-shell-work-limit";
            return Ok(out);
        }
        let sources = p.uses.map(|a| &faces[a.face][a.wire].edges()[a.edge]);
        let report=qualify(PairInput{index:i,pair:p,sources,ranges:mapped.get(&i).copied(),
            planes:supplied.get(&i).copied().unwrap_or([None;2]),candidates:witnessed.get(&i).copied()},
            max_work-out.work_used,max_driver_cells-out.driver_cells)?;
        out.work_used += report.work_used;
        out.root_checks += report.root_checks;
        out.driver_cells += report.driver_cells;
        let Some(edge) = report.edge else {
            out.uncertain_pair = Some(i);
            out.reason = report.reason;
            return Ok(out);
        };
        let endpoint = |a: Address, end: usize| {
            offsets[a.face][a.wire] + faces[a.face][a.wire].vertices()[a.edge][end]
        };
        join(&mut parent, endpoint(p.uses[0], 0), endpoint(p.uses[1], 1));
        join(&mut parent, endpoint(p.uses[0], 1), endpoint(p.uses[1], 0));
        join(&mut face_parent, p.uses[0].face, p.uses[1].face);
        edges.push(edge);
    }
    let first = root(&mut face_parent, 0);
    if (1..faces.len()).any(|i| root(&mut face_parent, i) != first) {
        out.reason = "source-shell-disconnected";
        return Ok(out);
    }
    let mut ids = BTreeMap::new();
    let vertices = faces
        .iter()
        .enumerate()
        .map(|(f, face)| {
            face.iter()
                .enumerate()
                .map(|(w, wire)| {
                    wire.vertices()
                        .iter()
                        .map(|v| {
                            v.map(|i| {
                                let owner = root(&mut parent, offsets[f][w] + i);
                                let next = ids.len();
                                *ids.entry(owner).or_insert(next)
                            })
                        })
                        .collect()
                })
                .collect()
        })
        .collect();
    out.shell = Some(Shell {
        poles: qualified_poles,
        faces: faces.to_vec(),
        regions: None,
        edges,
        uses: pairs.iter().map(|p| p.uses).collect(),
        vertices,
    });
    out.reason = "source-closed-oriented-incidence-qualified";
    Ok(out)
}
#[cfg(test)]
#[path = "source_shell_incidence/tests/mod.rs"]
mod tests;

#[path = "source_shell_incidence/replay.rs"]
mod replay;

#[path="source_shell_incidence/methods_inspect_face_charts.rs"]
mod methods_inspect_face_charts;
#[path="source_shell_incidence/algorithm_assemble_with_pole_inputs.rs"]
mod algorithm_assemble_with_pole_inputs;
pub use algorithm_assemble_with_pole_inputs::assemble_with_pole_inputs;
#[path="source_shell_incidence/algorithm_assemble_projective_fraction_impl.rs"]
mod algorithm_assemble_projective_fraction_impl;
use algorithm_assemble_projective_fraction_impl::assemble_projective_fraction_impl;
#[path="source_shell_incidence/algorithm_assemble_regions_projective_with_discovered_fraction_candidates.rs"]
mod algorithm_assemble_regions_projective_with_discovered_fraction_candidates;
pub use algorithm_assemble_regions_projective_with_discovered_fraction_candidates::assemble_regions_projective_with_discovered_fraction_candidates;
