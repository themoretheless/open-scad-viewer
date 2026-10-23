use super::*;

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
    pub(super) poles: Vec<(Address, crate::source_collapsed_boundary::CollapsedBoundary)>,
    pub(super) faces: Vec<Vec<Wire>>,
    pub(super) edges: Vec<SharedEdge>,
    pub(super) uses: Vec<[Address; 2]>,
    pub(super) vertices: Vec<Vec<Vec<[usize; 2]>>>,
    pub(super) regions: Option<Vec<crate::source_contour_proposal::SourceRegion>>,
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
pub(super) fn error(message: &str) -> Error {
    Error::new("BREP_SOURCE_SHELL", message)
}
pub(super) fn root(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}
pub(super) fn join(parent: &mut [usize], a: usize, b: usize) {
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
