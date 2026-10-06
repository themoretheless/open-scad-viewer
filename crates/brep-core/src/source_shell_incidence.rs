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
pub struct Shell {
    poles: Vec<(Address, crate::source_collapsed_boundary::CollapsedBoundary)>,
    faces: Vec<Vec<Wire>>,
    edges: Vec<SharedEdge>,
    uses: Vec<[Address; 2]>,
    vertices: Vec<Vec<Vec<[usize; 2]>>>,
    regions: Option<Vec<crate::source_contour_proposal::SourceRegion>>,
}
impl Shell {
    pub fn definition(&self) -> Result<value_codec::Value> {
        crate::source_shell_restore::definition(self)
    }
    pub fn poles(&self) -> &[(Address, crate::source_collapsed_boundary::CollapsedBoundary)] {
        &self.poles
    }
    pub fn regions(&self) -> Option<&[crate::source_contour_proposal::SourceRegion]> {
        self.regions.as_deref()
    }
    pub fn faces(&self) -> &[Vec<Wire>] {
        &self.faces
    }
    pub fn edges(&self) -> &[SharedEdge] {
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
impl Shell {
    /// Prove whole original charts injective, hence any retained source subset.
    /// Does not classify contacts between different faces or certify a volume.
    pub fn inspect_face_charts(
        &self,
        max_spans: usize,
        max_linear_cells: usize,
    ) -> Result<ChartReport> {
        if !(1..=100000).contains(&max_spans) || max_linear_cells > 100000 {
            return Err(error("Choose bounded source chart work"));
        }
        let mut out = ChartReport {
            all_injective: true,
            spans: 0,
            linear_cells: 0,
            faces: Vec::new(),
        };
        for (face, wires) in self.faces.iter().enumerate() {
            let surface = wires[0].edges()[0].surface();
            let contraction = if out.spans < max_spans {
                Some(nurbs_core::surface_injectivity::certify_contraction(
                    surface,
                    max_spans - out.spans,
                )?)
            } else {
                None
            };
            out.spans += contraction.as_ref().map_or(0, |r| r.spans);
            let linear = if !contraction.as_ref().is_some_and(|r| r.proven)
                && out.linear_cells < max_linear_cells
            {
                Some(nurbs_core::surface_linear_monotonicity::inspect_candidate(
                    surface,
                    max_linear_cells - out.linear_cells,
                )?)
            } else {
                None
            };
            out.linear_cells += linear.as_ref().map_or(0, |r| r.cells);
            let mut ruled_quotient = None;
            let mut polar_quotient = None;
            let quotient = if !contraction.as_ref().is_some_and(|r| r.proven)
                && !linear.as_ref().is_some_and(|r| r.certified)
                && max_spans - out.spans >= 16
            {
                let u = [
                    surface.knots_u[surface.degree_u],
                    surface.knots_u[surface.control_points.len()],
                ];
                let v = [
                    surface.knots_v[surface.degree_v],
                    surface.knots_v[surface.control_points[0].len()],
                ];
                let mut ends = Vec::new();
                for (address, pole) in &self.poles {
                    if address.face != face {
                        continue;
                    }
                    let source = pole.source();
                    let curve = source.curve();
                    let bounds = source.parameter_bounds();
                    let domain = curve.domain();
                    let full = bounds.iter().any(|b| *b == [domain[0]; 2])
                        && bounds.iter().any(|b| *b == [domain[1]; 2]);
                    if !full || curve.degree != 1 || curve.control_points.len() != 2 {
                        continue;
                    }
                    let a = &curve.control_points[0];
                    let b = &curve.control_points[1];
                    if a[0] == b[0]
                        && ((a[1] == v[0] && b[1] == v[1]) || (a[1] == v[1] && b[1] == v[0]))
                    {
                        if let Some(end) = u.iter().position(|x| *x == a[0]) {
                            ends.push(end);
                        }
                    }
                }
                if ends.len() == 1 {
                    let proof = nurbs_core::surface_quotient_injectivity::certify_source_frame(
                        surface,
                        ends[0],
                        16,
                        max_spans - out.spans,
                    )?;
                    out.spans += proof.cells;
                    if !proof.proven && surface.degree_u == 1 && max_spans - out.spans >= 16 {
                        let ruled =
                            nurbs_core::surface_quotient_injectivity::certify_ruled_source_frame(
                                surface,
                                ends[0],
                                16,
                                max_spans - out.spans,
                            )?;
                        out.spans += ruled.cells;
                        ruled_quotient = Some(ruled);
                    }
                    if !proof.proven && surface.degree_u >= 2 && max_spans - out.spans >= 256 {
                        let polar =
                            nurbs_core::surface_quotient_injectivity::certify_polar_source_frame(
                                surface,
                                ends[0],
                                16,
                                max_spans - out.spans,
                            )?;
                        out.spans += polar.cells;
                        polar_quotient = Some(polar);
                    }
                    Some(proof)
                } else {
                    None
                }
            } else {
                None
            };
            let result = ChartFace {
                face,
                contraction,
                linear,
                quotient,
                ruled_quotient,
                polar_quotient,
            };
            out.all_injective &= result.injectivity_proven();
            out.faces.push(result);
        }
        Ok(out)
    }
}
pub struct Report {
    pub shell: Option<Shell>,
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
pub fn assemble_with_pole_inputs(
    faces: &[Vec<Wire>],
    pairs: &[Pair],
    planes: &[RootPlanes],
    maps: &[AffineMaps],
    candidates: &[RootCandidates],
    poles: &[Pole],
    max_work: u64,
    max_driver_cells: usize,
) -> Result<Report> {
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
        let report = if mapped.contains_key(&i) || witnessed.contains_key(&i) {
            let full = p.world_reversed.map(|r| {
                if r {
                    [[1., 1.], [0., 1.]]
                } else {
                    [[0., 1.], [1., 1.]]
                }
            });
            let ranges = mapped.get(&i).copied().unwrap_or(full);
            crate::source_mapped_edge::qualify_with_candidates(
                &p.world,
                sources,
                ranges,
                supplied.get(&i).copied().unwrap_or([None, None]),
                witnessed.get(&i).copied().unwrap_or([[None; 2]; 2]),
                max_work - out.work_used,
                max_driver_cells - out.driver_cells,
            )?
        } else {
            source_shared_edge::qualify_with_cutters_and_planes(
                &p.world,
                sources,
                p.world_reversed,
                [p.cutters[0].as_ref(), p.cutters[1].as_ref()],
                supplied.get(&i).copied().unwrap_or([None, None]),
                max_work - out.work_used,
                max_driver_cells - out.driver_cells,
            )?
        };
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
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint, Fragment};
    use nurbs_core::surface::Surface;
    #[test]
    fn full_turn_caps_contract_poles_and_have_one_vertex_link_cycle() {
        for (radii, sweep) in [[0.5, 1.25], [1.25, 0.5], [0., 1.], [1., 0.]]
            .into_iter()
            .flat_map(|r| [std::f64::consts::TAU, -std::f64::consts::TAU].map(|s| (r, s)))
        {
            let spans = crate::linear_canal::construct(
                [[10., -7., 5.], [13., -3., 17.]],
                radii,
                [1., 0., 0.],
                sweep,
            )
            .unwrap();
            let model = crate::linear_canal::to_capped_region(&spans, 1e-7).unwrap();
            let mut faces = Vec::new();
            let mut regions = Vec::new();
            let mut poles = Vec::new();
            let mut pending = BTreeMap::new();
            let mut pairs = Vec::new();
            for (face, f) in model.faces.iter().enumerate() {
                let mut fragments = Vec::new();
                for (edge, use_) in model.loops[f.outer].coedges.iter().enumerate() {
                    let a = Address {
                        face,
                        wire: 0,
                        edge,
                    };
                    fragments.push(
                        Fragment::new(
                            &f.surface,
                            &use_.pcurve,
                            Endpoint::Parameter(0.),
                            Endpoint::Parameter(1.),
                        )
                        .unwrap(),
                    );
                    let world = &model.edges[use_.edge];
                    if world.degenerate {
                        poles.push(Pole {
                            use_: a,
                            point: model.vertices[world.vertices[0]].point,
                        });
                    } else if let Some((other, reversed)) = pending.remove(&use_.edge) {
                        pairs.push(Pair {
                            uses: [other, a],
                            world: world.curve.clone(),
                            world_reversed: [reversed, use_.reversed],
                            cutters: [None, None],
                        });
                    } else {
                        pending.insert(use_.edge, (a, use_.reversed));
                    }
                }
                let boundaries = model.loops[f.outer]
                    .coedges
                    .iter()
                    .map(|use_| crate::trimmed_face_recipe::Boundary {
                        curve: model.edges[use_.edge].curve.clone(),
                        pcurve: use_.pcurve.clone(),
                        reversed: use_.reversed,
                    })
                    .collect::<Vec<_>>();
                let qualified = crate::source_contour_proposal::qualify_original_region(
                    &cad_predicates::ToleranceContext::default_valid(),
                    &f.surface,
                    &[boundaries],
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 10000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                )
                .unwrap();
                regions.push(qualified.region.expect(qualified.reason));
                faces.push(vec![Wire::new(&fragments).unwrap()]);
            }
            assert!(pending.is_empty());
            let report =
                assemble_regions_with_poles(&regions, &pairs, &poles, 100_000_000).unwrap();
            let shell = report.shell.expect(report.reason);
            crate::source_shell_restore::assert_replay(&shell);
            assert_eq!(shell.poles().len(), poles.len());
            assert!(shell.regions().is_some());
            let authored = crate::linear_canal::to_capped_source_shell(
                &spans,
                1e-7,
                1e-8,
                crate::trimmed_face_recipe::Limits {
                    pairs: 10000,
                    region_cells: 10000,
                    domain_cells: 10000,
                    agreement_cells: 10000,
                },
                100_000_000,
            )
            .unwrap()
            .shell
            .unwrap();
            assert_eq!(authored.vertices(), shell.vertices());
            assert_eq!(authored.uses(), shell.uses());

            let charts = shell.inspect_face_charts(100000, 100000).unwrap();
            for (address, _) in shell.poles() {
                let face = &charts.faces[address.face];
                assert!(face.quotient.is_some(), "missing pole diagnostic");
                if shell.faces()[address.face][0].edges()[0].surface().degree_u == 1 {
                    assert!(
                        face.ruled_quotient.as_ref().is_some_and(|q| q.proven),
                        "{:?}",
                        face.ruled_quotient
                    );
                } else {
                    assert!(
                        face.polar_quotient.as_ref().is_some_and(|q| q.proven),
                        "sphere cap: {:?}",
                        face.polar_quotient
                    );
                }
            }
            assert!(
                charts.all_injective,
                "unproven faces: {:?}",
                charts
                    .faces
                    .iter()
                    .filter(|f| !f.injectivity_proven())
                    .map(|f| (
                        f.face,
                        f.contraction.as_ref().map(|r| r.reason),
                        f.linear.as_ref().map(|r| r.certified)
                    ))
                    .collect::<Vec<_>>()
            );
            let limited = shell.inspect_face_charts(1, 0).unwrap();
            assert!(!limited.all_injective && limited.spans <= 1);

            let links = crate::source_vertex_links::inspect(&shell, 100000).unwrap();
            assert_eq!(links.euler_characteristic, Some(2));
            assert_eq!(links.genus, Some(0));
            assert!(
                links.all_manifold,
                "{} {:?}",
                links.reason, links.uncertain_vertex
            );
            assert_eq!(
                links.links.len() as i64 - shell.edges().len() as i64 + faces.len() as i64,
                2
            );
            assert!(
                assemble_with_poles(&faces, &pairs, &poles, 1)
                    .unwrap()
                    .shell
                    .is_none()
            );
            assert!(assemble_with_poles(&faces, &pairs, &poles[1..], 100_000_000).is_err());
            let mut duplicate = poles
                .iter()
                .map(|p| Pole {
                    use_: p.use_,
                    point: p.point,
                })
                .collect::<Vec<_>>();
            duplicate[1].use_ = duplicate[0].use_;
            assert!(assemble_with_poles(&faces, &pairs, &duplicate, 100_000_000).is_err());
            let mut bad = poles;
            bad[0].point[0] += 1e-12;
            assert!(
                assemble_with_poles(&faces, &pairs, &bad, 100_000_000)
                    .unwrap()
                    .shell
                    .is_none()
            );
        }
    }
    #[test]
    fn qualified_source_pole_enables_the_existing_weighted_quotient_proof() {
        let surface = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 0., 0.]],
                vec![vec![0.5, 0., 0.], vec![0.5, 0., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let corners = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let curves = [
            surface.iso(nurbs_core::surface::Axis::V, 0.).unwrap(),
            surface.iso(nurbs_core::surface::Axis::U, 1.).unwrap(),
            surface
                .iso(nurbs_core::surface::Axis::V, 1.)
                .unwrap()
                .reverse()
                .unwrap(),
            surface
                .iso(nurbs_core::surface::Axis::U, 0.)
                .unwrap()
                .reverse()
                .unwrap(),
        ];
        let uv = (0..4)
            .map(|i| {
                Curve::from_polyline(vec![corners[i].to_vec(), corners[(i + 1) % 4].to_vec()])
                    .unwrap()
            })
            .collect::<Vec<_>>();
        let fragment = |c: &Curve| {
            Fragment::new(
                &surface,
                c,
                Endpoint::Parameter(0.),
                Endpoint::Parameter(1.),
            )
            .unwrap()
        };
        let a = uv.iter().map(fragment).collect::<Vec<_>>();
        let b = uv
            .iter()
            .rev()
            .map(|c| fragment(&c.reverse().unwrap()))
            .collect::<Vec<_>>();
        let faces = vec![vec![Wire::new(&a).unwrap()], vec![Wire::new(&b).unwrap()]];
        let pairs = (0..3)
            .map(|edge| Pair {
                uses: [
                    Address {
                        face: 0,
                        wire: 0,
                        edge,
                    },
                    Address {
                        face: 1,
                        wire: 0,
                        edge: 3 - edge,
                    },
                ],
                world: curves[edge].clone(),
                world_reversed: [false, true],
                cutters: [None, None],
            })
            .collect::<Vec<_>>();
        let poles = [
            Pole {
                use_: Address {
                    face: 0,
                    wire: 0,
                    edge: 3,
                },
                point: [0.; 3],
            },
            Pole {
                use_: Address {
                    face: 1,
                    wire: 0,
                    edge: 0,
                },
                point: [0.; 3],
            },
        ];
        let shell = assemble_with_poles(&faces, &pairs, &poles, 1000000)
            .unwrap()
            .shell
            .unwrap();
        let charts = shell.inspect_face_charts(1000, 0).unwrap();
        assert!(charts.all_injective);
        assert!(
            charts
                .faces
                .iter()
                .all(|f| f.quotient.as_ref().is_some_and(|q| q.proven))
        );
        assert!(charts.spans <= 1000 && charts.spans >= 512);
        assert!(!shell.inspect_face_charts(255, 0).unwrap().all_injective);
        // These two charts overlap; per-face injectivity is not embedding.
        assert!(shell.regions().is_none());
    }
    fn tetrahedron() -> (Vec<Vec<Wire>>, Vec<Pair>) {
        let points = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.], [0., 0., 1.]];
        let triangles = [[0, 2, 1], [0, 1, 3], [1, 2, 3], [2, 0, 3]];
        let mut pending = BTreeMap::new();
        let mut pairs = Vec::new();
        let mut faces = Vec::new();
        for (face, t) in triangles.iter().enumerate() {
            let [a, b, c] = t.map(|i| points[i]);
            let fourth: Vec<_> = (0..3).map(|i| b[i] + c[i] - a[i]).collect();
            let s = Surface {
                degree_u: 1,
                degree_v: 1,
                knots_u: vec![0., 0., 1., 1.],
                knots_v: vec![0., 0., 1., 1.],
                control_points: vec![vec![a.to_vec(), c.to_vec()], vec![b.to_vec(), fourth]],
                weights: vec![vec![1.; 2]; 2],
                periodic_u: false,
                periodic_v: false,
            };
            let uv = [[0., 0.], [1., 0.], [0., 1.]];
            let mut fragments = Vec::new();
            for edge in 0..3 {
                let next = (edge + 1) % 3;
                let pc = Curve::from_polyline(vec![uv[edge].to_vec(), uv[next].to_vec()]).unwrap();
                fragments.push(
                    Fragment::new(&s, &pc, Endpoint::Parameter(0.), Endpoint::Parameter(1.))
                        .unwrap(),
                );
                let address = Address {
                    face,
                    wire: 0,
                    edge,
                };
                let key = (t[edge].min(t[next]), t[edge].max(t[next]));
                if let Some((old, world)) = pending.remove(&key) {
                    pairs.push(Pair {
                        uses: [old, address],
                        world,
                        world_reversed: [false, true],
                        cutters: [None, None],
                    });
                } else {
                    pending.insert(
                        key,
                        (
                            address,
                            Curve::from_polyline(vec![
                                points[t[edge]].to_vec(),
                                points[t[next]].to_vec(),
                            ])
                            .unwrap(),
                        ),
                    );
                }
            }
            faces.push(vec![Wire::new(&fragments).unwrap()]);
        }
        assert!(pending.is_empty());
        (faces, pairs)
    }
    #[test]
    fn tetrahedron_pairs_produce_shared_vertex_owners_and_refuse_missing_or_duplicate_uses() {
        let (faces, mut pairs) = tetrahedron();
        let report = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(
            report.shell.is_some(),
            "{} pair {:?}",
            report.reason,
            report.uncertain_pair
        );
        let shell = report.shell.unwrap();
        assert_eq!(shell.edges().len(), 6);
        let topology = crate::source_vertex_links::inspect(&shell, 12).unwrap();
        assert!(topology.all_manifold);
        assert_eq!(topology.links.len(), 4);
        assert!(topology.links.iter().all(|v| v.cycle.len() == 3));
        assert!(topology.euler_characteristic.is_none());
        assert!(
            !crate::source_vertex_links::inspect(&shell, 11)
                .unwrap()
                .all_manifold
        );
        assert!(crate::source_allowed_contact::certify(&shell, [0, 1], 100_000_000, 2).is_err());
        assert!(
            crate::source_fiber_contact::certify(&shell, [0, 1], 100_000_000, 2, 10000).is_err()
        );
        let owners: std::collections::BTreeSet<_> = shell
            .vertices()
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        assert_eq!(owners.len(), 4);
        for pair in shell.uses() {
            let vertex = |a: Address| shell.vertices()[a.face][a.wire][a.edge];
            assert_eq!(vertex(pair[0]), [vertex(pair[1])[1], vertex(pair[1])[0]]);
        }
        assert!(assemble(&faces, &pairs[..5], 100_000_000).is_err());
        pairs[1].uses[0] = pairs[0].uses[0];
        assert!(assemble(&faces, &pairs, 100_000_000).is_err());
        assert!(
            crate::source_shell_geometry::qualify(
                shell,
                crate::source_shell_geometry::Limits {
                    tolerance_uv: 1e-8,
                    corners: 12,
                    spans: 100,
                    linear_cells: 100,
                    pairs: crate::face_contacts::Limits {
                        pairs: 6,
                        cells: 1000,
                        domain_cells: 1000,
                        cells_per_pair: 100,
                        domain_cells_per_pair: 100
                    },
                    exact_work: 100_000_000,
                    driver_cells: 100,
                }
            )
            .is_err()
        );
    }
    #[test]
    fn root_partition_of_shared_edge_preserves_closed_shell_and_original_definitions() {
        root_partition_control(false, false);
    }
    #[test]
    fn irrational_root_partition_preserves_closed_body_and_exact_source_topology() {
        root_partition_control(true, false);
    }
    #[test]
    fn curved_shell_preserves_roots_but_refuses_unproven_face_contacts() {
        root_partition_control(true, true);
    }
    fn root_partition_control(irrational: bool, curved: bool) {
        use crate::source_boundary_fragment::Role;
        let (mut faces, mut pairs) = tetrahedron();
        if curved {
            // Exact polynomial shear F(x,y,z)=(x,y,z+x*x/4), determinant one.
            // Elevate the affine charts to biquadratic Bezier coefficients.
            for face in &mut faces {
                let original=face[0].edges()[0].surface();
                let a=&original.control_points[0][0];
                let b:Vec<_>=(0..3).map(|k|original.control_points[1][0][k]-a[k]).collect();
                let c:Vec<_>=(0..3).map(|k|original.control_points[0][1][k]-a[k]).collect();
                let controls=(0..3).map(|i| (0..3).map(|j| {
                    let mut q:Vec<_>=(0..3).map(|k|a[k]+b[k]*(i as f64/2.)+c[k]*(j as f64/2.)).collect();
                    let square=a[0]*a[0]+a[0]*b[0]*i as f64+a[0]*c[0]*j as f64
                        +if i==2 {b[0]*b[0]} else {0.}
                        +if j==2 {c[0]*c[0]} else {0.}
                        +0.5*b[0]*c[0]*(i*j) as f64;
                    q[2]+=square/4.;q
                }).collect()).collect();
                let surface=Surface {degree_u:2,degree_v:2,periodic_u:false,periodic_v:false,
                    knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
                    control_points:controls,weights:vec![vec![1.;3];3]};
                let mut leaves=surface.control_points.iter().flatten().flatten().map(|v|
                    cad_predicates::AuthoredScalar::Binary64Bits(v.to_bits())).collect::<Vec<_>>();
                leaves.push(cad_predicates::AuthoredScalar::Binary64Bits(0.25f64.to_bits()));
                let arena=cad_predicates::SourceArena::authored("curved-body-shear",1,leaves).unwrap();
                let tolerance=cad_predicates::ToleranceContext::default_valid();
                let mut predicate=cad_predicates::PredicateContext::new(&arena,&tolerance,
                    cad_predicates::Limits::default(),None);
                let refs=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|
                    arena.leaf(9*i+3*j+k).unwrap())));
                let proof=cad_predicates::quadratic_shear_chart_identity(&mut predicate,refs,
                    arena.leaf(27).unwrap(),0,2).unwrap();
                assert_eq!(proof.outcome,cad_predicates::ParameterIdentity::Equal);
                let inverse=crate::source_inverse_shear::qualify(&surface,[0,2],0.25,100000).unwrap();
                let certificate=inverse.certificate.expect(inverse.reason);
                assert_eq!(certificate.source(),&surface);
                assert_eq!(certificate.inverse().degree_u,1);
                assert_eq!(certificate.inverse().degree_v,1);
                for wire in face {
                    let edges=wire.edges().iter().map(|e|Fragment::new(&surface,e.curve(),
                        Endpoint::Parameter(0.),Endpoint::Parameter(1.)).unwrap()).collect::<Vec<_>>();
                    *wire=Wire::new(&edges).unwrap();
                }
            }
            for pair in &mut pairs {
                let a=&pair.world.control_points[0];let b=&pair.world.control_points[1];
                let mut first=a.clone();first[2]+=a[0]*a[0]/4.;
                let mut last=b.clone();last[2]+=b[0]*b[0]/4.;
                let mut middle:Vec<_>=(0..3).map(|k|(a[k]+b[k])/2.).collect();
                middle[2]+=a[0]*b[0]/4.;
                pair.world=Curve {degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
                    control_points:vec![first,middle,last],weights:vec![1.;3]};
            }
        }
        let context = cad_predicates::ToleranceContext::default_valid();
        let mut regions = faces
            .iter()
            .map(|face| {
                let surface = face[0].edges()[0].surface();
                let boundaries = face
                    .iter()
                    .map(|wire| {
                        wire.edges()
                            .iter()
                            .map(|e| {
                                let c = e.curve();
                                let a = &c.control_points[0];
                                let b = &c.control_points[1];
                                let first=surface.evaluate(a[0],a[1]).unwrap().point.to_vec();
                                let last=surface.evaluate(b[0],b[1]).unwrap().point.to_vec();
                                let curve=if curved {
                                    let mut middle:Vec<_>=(0..3).map(|k|(first[k]+last[k])/2.).collect();
                                    middle[2]-=(last[0]-first[0])*(last[0]-first[0])/8.;
                                    Curve {degree:2,periodic:false,knots:vec![0.,0.,0.,1.,1.,1.],
                                        control_points:vec![first,middle,last],weights:vec![1.;3]}
                                } else {Curve::from_polyline(vec![first,last]).unwrap()};
                                crate::trimmed_face_recipe::Boundary {
                                    curve,
                                    pcurve: c.clone(),
                                    reversed: false,
                                }
                            })
                            .collect()
                    })
                    .collect::<Vec<Vec<_>>>();
                let report = crate::source_contour_proposal::qualify_original_region(
                    &context,
                    surface,
                    &boundaries,
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 1000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                )
                .unwrap();
                let mut damaged = boundaries.clone();
                damaged[0][0].curve.control_points[0][0] += 1e-3;
                let refused = crate::source_contour_proposal::qualify_original_region(
                    &context,
                    surface,
                    &damaged,
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 1000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                )
                .unwrap();
                assert!(refused.region.is_none());
                assert!(
                    report.region.is_some(),
                    "{} / {}",
                    report.reason,
                    report.audit.reason
                );
                report.region.unwrap()
            })
            .collect::<Vec<_>>();
        let old = pairs.remove(0);
        for (i, address) in old.uses.iter().enumerate() {
            let edge = &faces[address.face][address.wire].edges()[address.edge];
            let c = edge.curve();
            let t = if i == 0 { 0.25 } else { 0.75 };
            let a = &c.control_points[0];
            let b = &c.control_points[1];
            let point = [a[0] + t * (b[0] - a[0]), a[1] + t * (b[1] - a[1])];
            let normal = [-(b[1] - a[1]), b[0] - a[0]];
            let cutter = if irrational {
                // q(t)=a+d*t+n*(t²-1/2), reflected along d on the other use.
                // The boundary root is sqrt(1/2), or 1-sqrt(1/2), not a
                // floating point authored parameter. All controls are dyadic.
                let controls = [(if i == 0 { 0. } else { 1. }, -0.5),
                    (0.5, -0.5), (if i == 0 { 1. } else { 0. }, 0.5)]
                    .map(|(along,across)| (0..2).map(|axis|
                        a[axis]+along*(b[axis]-a[axis])+across*normal[axis]
                    ).collect());
                Curve { periodic:false,degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
                    control_points:controls.to_vec(),weights:vec![1.;3] }
            } else {
                Curve::from_polyline(vec![
                    vec![point[0] - normal[0], point[1] - normal[1]],
                    vec![point[0] + normal[0], point[1] + normal[1]],
                ]).unwrap()
            };
            if irrational {
                let broad = crate::source_contact_point::qualify(edge.surface(), c, &cutter, [[0.,1.];2], 10000).unwrap();
                assert!(broad.point.is_none());
            }
            let selector = if irrational {
                [if i == 0 { [0.7,0.71] } else { [0.29,0.3] }, [0.7,0.71]]
            } else { [[0.,1.];2] };
            let point_report = crate::source_contact_point::qualify(edge.surface(), c, &cutter, selector, 10000).unwrap();
            let p = point_report.point.expect(point_report.reason);
            assert!(
                regions[address.face]
                    .split_boundary(address.wire, address.edge, &p, Role::Contact)
                    .is_err()
            );
            regions[address.face] = regions[address.face]
                .split_boundary(address.wire, address.edge, &p, Role::Boundary)
                .unwrap();
            let parts = edge.split_at(&p, Role::Boundary).unwrap();
            assert_eq!(parts[0].curve(), c);
            assert_eq!(parts[1].curve(), c);
            let mut edges = faces[address.face][address.wire].edges().to_vec();
            edges.splice(address.edge..address.edge + 1, parts);
            faces[address.face][address.wire] = Wire::new(&edges).unwrap();
        }
        for pair in &mut pairs {
            for address in &mut pair.uses {
                for split in old.uses {
                    if address.face == split.face
                        && address.wire == split.wire
                        && address.edge > split.edge
                    {
                        address.edge += 1;
                    }
                }
            }
        }
        let [a, b] = old.uses;
        pairs.push(Pair {
            uses: [
                a,
                Address {
                    edge: b.edge + 1,
                    ..b
                },
            ],
            world: old.world.clone(),
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        pairs.push(Pair {
            uses: [
                Address {
                    edge: a.edge + 1,
                    ..a
                },
                b,
            ],
            world: old.world,
            world_reversed: old.world_reversed,
            cutters: [None, None],
        });
        let r = assemble_regions(&regions, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let shell = r.shell.unwrap();
        crate::source_shell_restore::assert_replay(&shell);
        assert_eq!(shell.regions().unwrap().len(), 4);
        let network = crate::source_boundary_network::inspect(&shell, 14, 14).unwrap();
        assert_eq!(network.vertices.len(), 5);
        assert_eq!(network.endpoints, 14);
        assert_eq!(network.spans, 14);
        let root_vertex = network.vertices.iter().find(|v| v.ends.len() == 2).unwrap();
        if curved {
            assert!(shell.faces().iter().all(|face| face[0].edges()[0].surface().degree_u==2));
            assert!(shell.edges().iter().any(|edge|edge.world().degree==2));
            let display=crate::source_boundary_display::prepare(&shell,8,14,14).unwrap();
            assert_eq!(display.edges.len(),7);
            for [edge,end] in &root_vertex.ends {
                let bounds=shell.edges()[*edge].uses()[0].parameter_bounds();
                assert!(bounds.iter().any(|b|b[0]<b[1]));
                let _=end;
            }
            let admitted=crate::source_shell_geometry::qualify(shell,
                crate::source_shell_geometry::Limits {
                    tolerance_uv:1e-8,corners:14,spans:1000,linear_cells:1000,
                    pairs:crate::face_contacts::Limits {pairs:6,cells:10000,domain_cells:10000,
                        cells_per_pair:1000,domain_cells_per_pair:1000},
                    exact_work:100_000_000,driver_cells:10000,
                }).unwrap();
            // Closed incidence and injective charts cannot authorize unproven contacts.
            // This fixture remains a Body admission target; no volume claim is made.
            assert!(admitted.geometry.is_none());
            assert_eq!(admitted.reason,"source-shell-different-face-contacts-unproven");
            eprintln!("curved contact pending pair: {:?}; work {}",admitted.next_pair,admitted.exact_work);
            return;
        }
        assert!(root_vertex.poles.is_empty());
        for count in [1, 2, 8] {
            let preview = crate::source_boundary_display::prepare(&shell, count, 14, 14).unwrap();
            let mut common = None;
            for [edge, end] in &root_vertex.ends {
                let displayed = &preview.edges[*edge];
                assert_eq!(displayed.vertices[*end], root_vertex.id);
                let segment = &displayed.segments[if *end == 0 { 0 } else { count - 1 }];
                let point = segment.1[*end];
                if let Some(previous) = common { assert_eq!(point, previous); }
                common = Some(point);
                for axis in 0..3 {
                    assert!(point[axis] >= root_vertex.bounds[axis][0]);
                    assert!(point[axis] <= root_vertex.bounds[axis][1]);
                    assert!(segment.2[axis][0] <= root_vertex.bounds[axis][0]);
                    assert!(segment.2[axis][1] >= root_vertex.bounds[axis][1]);
                }
            }
        }
        assert!(crate::source_boundary_display::prepare(&shell, 0, 14, 14).is_err());
        assert!(crate::source_boundary_display::prepare(&shell, 4097, 14, 14).is_err());
        // Both halves keep their original carrier and independent root recipes.
        // The shared vertex enclosure must be inside each canonical end box.
        for [edge, end] in &root_vertex.ends {
            let restriction = crate::source_edge_restriction::Restriction::from_edge(
                &shell.edges()[*edge],
            );
            let parameter = restriction.parameter_bounds().unwrap()[*end];
            let domain = restriction.edge().world().domain();
            assert!(parameter[0] > domain[0] && parameter[1] < domain[1]);
            if irrational {
                assert!(parameter[0] < parameter[1]);
                assert!(restriction.edge().uses().iter().any(|fragment|
                    fragment.endpoints().iter().any(|endpoint|
                        matches!(endpoint, crate::source_boundary_fragment::Endpoint::Crossing { .. }))));
            }
            let bounds = restriction.endpoint_boxes(2).unwrap()[*end];
            for axis in 0..3 {
                assert!(root_vertex.bounds[axis][0] >= bounds[axis][0]);
                assert!(root_vertex.bounds[axis][1] <= bounds[axis][1]);
            }
        }
        assert!(crate::source_boundary_network::inspect(&shell, 13, 14).is_err());
        assert!(crate::source_boundary_network::inspect(&shell, 14, 13).is_err());
        let topology = crate::source_vertex_links::inspect(&shell, 14).unwrap();
        assert!(topology.all_manifold);
        assert_eq!(topology.links.len(), 5);
        assert_eq!(topology.euler_characteristic, Some(2));
        assert_eq!(topology.genus, Some(0));
        assert_eq!(
            topology.links.iter().filter(|v| v.cycle.len() == 2).count(),
            1
        );
        for a in 0..4 {
            for b in a + 1..4 {
                let contact =
                    crate::source_allowed_contact::certify(&shell, [a, b], 100_000_000, 2).unwrap();
                assert!(
                    contact.certificate.is_some(),
                    "{} faces {:?}",
                    contact.reason,
                    [a, b]
                );
            }
        }
        let mut fiber_pairs = 0;
        for a in 0..4 {
            for b in 0..4 {
                if a == b {
                    continue;
                }
                let r = crate::source_fiber_contact::certify(&shell, [a, b], 100_000_000, 2, 10000)
                    .unwrap();
                if let Some(c) = r.certificate {
                    fiber_pairs += 1;
                    assert_eq!(c.faces(), [a, b]);
                    assert!(!c.edges().is_empty());
                    assert_eq!(c.regions()[1].loops().len(), regions[b].loops().len());
                    assert_eq!(c.fiber().surface(), regions[b].loops()[0][0].surface());
                }
            }
        }
        assert!(fiber_pairs > 0);
        let fiber_audit = crate::source_face_contacts::inspect_shell_with_boundary_fibers(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 6,
                cells: 1,
                domain_cells: 1,
                cells_per_pair: 1,
                domain_cells_per_pair: 1,
            },
            100_000_000,
            128,
            10000,
        )
        .unwrap();
        assert!(fiber_audit.all_pairs_qualified);
        assert_eq!(fiber_audit.pairs.len(), 6);
        assert!(fiber_audit.pairs.iter().any(|p| p.fiber.is_some()));
        assert_eq!(fiber_audit.cells, 0);
        let exhausted = crate::source_face_contacts::inspect_shell_with_boundary_fibers(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 6,
                cells: 1,
                domain_cells: 1,
                cells_per_pair: 1,
                domain_cells_per_pair: 1,
            },
            1,
            128,
            10000,
        )
        .unwrap();
        assert!(!exhausted.all_pairs_qualified);
        assert!(exhausted.pairs.iter().all(|p| p.fiber.is_none()));
        assert!(exhausted.pairs[0].result.is_some());

        assert!(
            crate::source_fiber_contact::certify(&shell, [0, 1], 1, 2, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            crate::source_fiber_contact::certify(&shell, [0, 1], 100_000_000, 1, 100)
                .unwrap()
                .certificate
                .is_none()
        );
        let allowed = crate::source_face_contacts::inspect_shell_with_allowed(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 6,
                cells: 1,
                domain_cells: 1,
                cells_per_pair: 1,
                domain_cells_per_pair: 1,
            },
            100_000_000,
            12,
        )
        .unwrap();
        assert!(allowed.all_pairs_qualified);
        assert!(!allowed.all_pairs_absence_proven);
        assert_eq!(allowed.pairs.len(), 6);
        assert_eq!(allowed.cells, 0);
        assert!(
            allowed
                .pairs
                .iter()
                .all(|p| p.allowed.is_some() && p.result.is_none())
        );
        let limited = crate::source_face_contacts::inspect_shell_with_allowed(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 1,
                cells: 1,
                domain_cells: 1,
                cells_per_pair: 1,
                domain_cells_per_pair: 1,
            },
            1,
            1,
        )
        .unwrap();
        assert!(!limited.all_pairs_qualified);
        assert!(limited.pairs[0].allowed.is_none());
        assert!(
            limited.pairs[0]
                .result
                .as_ref()
                .is_some_and(|r| !r.absence_proven)
        );
        let stopped = crate::source_allowed_contact::certify(&shell, [0, 1], 1, 2).unwrap();
        assert!(stopped.certificate.is_none() && stopped.exact_work <= 1);
        let stopped =
            crate::source_allowed_contact::certify(&shell, [0, 1], 100_000_000, 1).unwrap();
        assert!(stopped.certificate.is_none());
        assert!(shell.inspect_face_charts(4, 0).unwrap().all_injective);
        let pairs = crate::source_face_contacts::inspect_shell(
            &shell,
            1e-8,
            crate::face_contacts::Limits {
                pairs: 1,
                cells: 2,
                domain_cells: 2,
                cells_per_pair: 1,
                domain_cells_per_pair: 1,
            },
        )
        .unwrap();
        assert_eq!(pairs.total_pairs, 6);
        assert_eq!(pairs.pairs.len(), 1);
        assert_eq!(pairs.pairs[0].faces, [0, 1]);
        assert_eq!(pairs.next_pair, Some([0, 2]));
        assert!(!pairs.all_pairs_absence_proven);
        assert!(
            !pairs.pairs[0]
                .result
                .as_ref()
                .unwrap()
                .unresolved
                .is_empty()
        );
        for (face, region) in shell.regions().unwrap().iter().enumerate() {
            assert_eq!(region.source_loop_indices(), &[0]);
            for (i, edge) in region.loops()[0].iter().enumerate() {
                assert_eq!(
                    edge.definition(),
                    shell.faces()[face][0].edges()[i].definition()
                );
            }
        }
        assert_eq!(shell.edges().len(), 7);
        let owners: std::collections::BTreeSet<_> = shell
            .vertices()
            .iter()
            .flatten()
            .flatten()
            .flatten()
            .copied()
            .collect();
        assert_eq!(owners.len(), 5);
        for pair in shell.uses() {
            let vertex = |a: Address| shell.vertices()[a.face][a.wire][a.edge];
            assert_eq!(vertex(pair[0]), [vertex(pair[1])[1], vertex(pair[1])[0]]);
        }
        let admitted = crate::source_shell_geometry::qualify(
            shell,
            crate::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 14,
                spans: 1000,
                linear_cells: 1000,
                pairs: crate::face_contacts::Limits {
                    pairs: 6,
                    cells: 10000,
                    domain_cells: 10000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 1000,
                },
                exact_work: 100_000_000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert!(admitted.geometry.is_some(), "{}", admitted.reason);
        let geometry = admitted.geometry.unwrap();
        assert_eq!(geometry.topology().genus, Some(0));
        let volume = crate::source_volume::qualify(
            geometry,
            crate::source_volume::Limits {
                axis: 2,
                origin: 0.,
                absolute_error: 0.02,
                tolerance_uv: 1e-8,
                cells: 10000,
                spans: 100000,
                domain_cells: 1000000,
            },
        )
        .unwrap();
        assert!(
            volume.body.is_some(),
            "{} {:?}",
            volume.reason,
            volume.signed_bounds
        );
        let body = volume.body.unwrap();
        assert!(crate::source_body_model::convert(&body, 1e-7).is_err());
        let source_model = crate::source_body_topology::convert(&body, 1e-7, 14, 14).unwrap();
        assert_eq!(source_model.definition(), &body.definition().unwrap());
        let indexed = source_model.topology();
        assert_eq!(indexed.vertices.len(), 5);
        assert_eq!(indexed.edges.len(), 7);
        for (i,edge) in indexed.edges.iter().enumerate() {
            let crate::source_body_topology::Carrier::Edge(restriction) = &edge.curve else { panic!("Unexpected pole"); };
            assert_eq!(restriction.definition(), body.edge_restriction(i).unwrap().definition());
        }
        for (face,wires) in body.geometry().shell().faces().iter().enumerate() {
            for (wire,source) in wires.iter().enumerate() {
                let loop_index = if wire == 0 { indexed.faces[face].outer } else { indexed.faces[face].holes[wire-1] };
                for (edge,fragment) in source.edges().iter().enumerate() {
                    assert_eq!(indexed.loops[loop_index].coedges[edge].pcurve.definition(), fragment.definition());
                }
            }
        }
        assert!(crate::source_body_topology::convert(&body, 0., 14, 14).is_err());
        assert!(crate::source_body_topology::convert(&body, 1e-7, 13, 14).is_err());
        let seam_limits = || crate::source_seam_tangency::Limits {
            max_sine_squared: 1e-6,
            cells: 100,
            curve_spans: 200,
            normal_spans: 200,
        };
        // A valid tetrahedron is intentionally sharp; volume admission must
        // not silently label its selected shared edge tangent.
        assert!(
            body.qualify_edge_tangency(0, seam_limits())
                .unwrap()
                .seam
                .is_none()
        );
        assert!(
            body.qualify_edge_tangency(body.geometry().shell().edges().len(), seam_limits())
                .is_err()
        );
        let center = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 0.]]).unwrap();
        let radius = Curve::from_polyline(vec![vec![1., 0.], vec![1., 0.]]).unwrap();
        assert!(
            body.qualify_face_radius(0, &center, &radius, 1e-6, 100, 100_000_000)
                .unwrap()
                .certificate
                .is_none()
        );
        assert!(
            body.qualify_face_radius(
                body.geometry().shell().faces().len(),
                &center,
                &radius,
                1e-6,
                100,
                100_000_000
            )
            .is_err()
        );
        let bounds = body.volume();
        assert!(bounds[0] <= 1. / 6. && 1. / 6. <= bounds[1]);
        assert!(!body.reverse_orientation());
        if irrational {
            if let Some(path) = std::env::var_os("CAD_IRRATIONAL_BODY_FIXTURE_OUTPUT") {
                let template = std::env::var_os("CAD_IRRATIONAL_BODY_FIXTURE_TEMPLATE").unwrap();
                let mut request: value_codec::Value = value_codec::from_str(&std::fs::read_to_string(template).unwrap()).unwrap();
                request["definition"] = body.definition().unwrap();
                std::fs::write(path, value_codec::to_string(&request).unwrap()).unwrap();
            }
        }
    }
    #[test]
    fn chart_audit_catches_folded_geometry_even_when_exact_boundary_incidence_closes() {
        let (mut faces, pairs) = tetrahedron();
        let shell = assemble(&faces, &pairs, 100_000_000)
            .unwrap()
            .shell
            .unwrap();
        let charts = shell.inspect_face_charts(4, 0).unwrap();
        assert!(charts.all_injective);
        assert_eq!(charts.spans, 4);
        let stopped = shell.inspect_face_charts(1, 0).unwrap();
        assert!(!stopped.all_injective);
        assert_eq!(stopped.spans, 1);
        assert_eq!(stopped.faces.len(), 4);
        assert!(stopped.faces[1].contraction.is_none());
        let s = faces[0][0].edges()[0].surface();
        let a = &s.control_points[0][0];
        let b = &s.control_points[1][0];
        let c = &s.control_points[0][1];
        // Affine face plus exact polynomial bubble 16*u*v*(1-u-v)
        // in its u direction. All three original triangle boundaries stay
        // unchanged while an interior directional derivative changes sign.
        let controls = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let u = i as f64 / 2.;
                        let v = j as f64 / 2.;
                        let delta = 16.
                            * (u * v - if i == 2 { v } else { 0. } - if j == 2 { u } else { 0. });
                        (0..3)
                            .map(|k| a[k] + (u + delta) * (b[k] - a[k]) + v * (c[k] - a[k]))
                            .collect()
                    })
                    .collect()
            })
            .collect();
        let folded = Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: controls,
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        assert_eq!(
            folded.evaluate(0.375, 0.25).unwrap().point,
            folded.evaluate(0.625, 0.25).unwrap().point
        );
        let edges: Vec<_> = faces[0][0]
            .edges()
            .iter()
            .map(|e| {
                Fragment::new(
                    &folded,
                    e.curve(),
                    e.endpoints()[0].clone(),
                    e.endpoints()[1].clone(),
                )
                .unwrap()
            })
            .collect();
        faces[0][0] = Wire::new(&edges).unwrap();
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_some(), "{} {:?}", r.reason, r.uncertain_pair);
        let charts = r.shell.unwrap().inspect_face_charts(4, 0).unwrap();
        assert!(!charts.all_injective);
        assert!(!charts.faces[0].injectivity_proven());
        assert!(charts.faces[1..].iter().all(|f| f.injectivity_proven()));
        let context = cad_predicates::ToleranceContext::default_valid();
        let regions = faces
            .iter()
            .enumerate()
            .map(|(face, wires)| {
                let boundaries = wires
                    .iter()
                    .enumerate()
                    .map(|(wire, w)| {
                        w.edges()
                            .iter()
                            .enumerate()
                            .map(|(edge, e)| {
                                let address = Address { face, wire, edge };
                                let p = pairs.iter().find(|p| p.uses.contains(&address)).unwrap();
                                let index = p.uses.iter().position(|a| *a == address).unwrap();
                                let mut world = p.world.clone();
                                if p.world_reversed[index] {
                                    world.control_points.reverse();
                                    world.weights.reverse();
                                }
                                crate::trimmed_face_recipe::Boundary {
                                    curve: world,
                                    pcurve: e.curve().clone(),
                                    reversed: false,
                                }
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let r = crate::source_contour_proposal::qualify_original_region(
                    &context,
                    wires[0].edges()[0].surface(),
                    &boundaries,
                    1e-8,
                    crate::trimmed_face_recipe::Limits {
                        pairs: 10000,
                        region_cells: 10000,
                        domain_cells: 10000,
                        agreement_cells: 10000,
                    },
                )
                .unwrap();
                assert!(r.region.is_some(), "face {} {}", face, r.reason);
                r.region.unwrap()
            })
            .collect::<Vec<_>>();
        let shell = assemble_regions(&regions, &pairs, 100_000_000)
            .unwrap()
            .shell
            .unwrap();
        let admission = crate::source_shell_geometry::qualify(
            shell,
            crate::source_shell_geometry::Limits {
                tolerance_uv: 1e-8,
                corners: 12,
                spans: 1000,
                linear_cells: 0,
                pairs: crate::face_contacts::Limits {
                    pairs: 6,
                    cells: 10000,
                    domain_cells: 10000,
                    cells_per_pair: 1000,
                    domain_cells_per_pair: 1000,
                },
                exact_work: 100_000_000,
                driver_cells: 10000,
            },
        )
        .unwrap();
        assert!(admission.geometry.is_none());
        assert_eq!(admission.reason, "source-shell-chart-injectivity-unproven");
        assert_eq!(admission.pairs, 0);
        assert_eq!(admission.uncertain_face, Some(0));
    }
    #[test]
    fn disconnected_closed_components_and_exhausted_work_are_not_a_shell() {
        let (mut faces, mut pairs) = tetrahedron();
        let stopped = assemble(&faces, &pairs, 1).unwrap();
        assert!(stopped.shell.is_none() && stopped.uncertain_pair.is_some());
        assert!(stopped.work_used <= 1);
        let (other, mut more) = tetrahedron();
        for pair in &mut more {
            for address in &mut pair.uses {
                address.face += faces.len();
            }
        }
        faces.extend(other);
        pairs.extend(more);
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_none());
        assert_eq!(r.reason, "source-shell-disconnected");
    }
    #[test]
    fn exact_geometry_mismatch_never_produces_partial_shell() {
        let (faces, mut pairs) = tetrahedron();
        pairs[0].world.control_points[0][0] += 1e-12;
        let r = assemble(&faces, &pairs, 100_000_000).unwrap();
        assert!(r.shell.is_none());
        assert_eq!(r.uncertain_pair, Some(0));
    }
}
