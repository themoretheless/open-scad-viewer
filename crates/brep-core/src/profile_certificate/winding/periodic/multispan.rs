//! Actual source chart winding through independently replayed rational image
//! traversals; other-curve knot preimages need not be binary64 or even rational.
use super::*;
use crate::profile_certificate::{
    MultispanAgreementCertificate, certify_multispan_agreement, verify_multispan_agreement,
};
#[derive(Clone, Debug, PartialEq)]
pub struct MultispanChartAlias {
    pub edges: [usize; 2],
    pub orientation: i8,
    pub certificate: MultispanAgreementCertificate,
}
#[derive(Clone, Debug)]
pub struct MultispanPeriodicRegions {
    original: periodic::ExactSourceChart,
    aliases: Vec<MultispanChartAlias>,
    graph: WeightedChartGraph,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: WindingRegularization,
}
fn raw(source: &periodic::ExactSourceChart) -> Result<&periodic::ChartBoundaryGraph> {
    source.materialized_graph().ok_or_else(|| {
        refused(&format!(
            "Missing actual original multispan chart: {:?}",
            source.reason()
        ))
    })
}
fn compact(
    source: &periodic::ExactSourceChart,
    aliases: &[MultispanChartAlias],
) -> Result<WeightedChartGraph> {
    source.reverify()?;
    let raw = raw(source)?;
    if aliases.len() > 64 {
        return Err(refused("Multispan chart alias allowance exceeded"));
    }
    let mut candidates = Vec::new();
    for alias in aliases {
        if alias.edges[0] == alias.edges[1]
            || alias.edges.iter().any(|&i| i >= raw.edges.len())
            || !matches!(alias.orientation, -1 | 1)
        {
            return Err(refused("Invalid original multispan alias binding"));
        }
        let a = &raw.edges[alias.edges[0]].curve;
        let b = &raw.edges[alias.edges[1]].curve;
        let mut db = b.domain();
        if alias.orientation < 0 {
            db.reverse();
        }
        verify_multispan_agreement(a, b, a.domain(), db, &alias.certificate)?;
        candidates.push(periodic::CandidateImageAlias {
            edges: alias.edges,
            orientation: alias.orientation,
        });
    }
    let graph = periodic::compact_image_alias_candidates(raw, &candidates)?
        .ok_or_else(|| refused("Original multispan weighted flow remains unproven"))?;
    if !graph.edges.is_empty() {
        let edges = graph
            .edges
            .iter()
            .map(|e| periodic::ChartEdge {
                curve: e.curve.clone(),
                source: None,
            })
            .collect::<Vec<_>>();
        if !periodic::inspect_contact_ownership(&edges, 1024)?.complete {
            return Err(refused(
                "Multispan chart endpoint port ownership remains unproven",
            ));
        }
    }
    Ok(graph)
}
impl MultispanPeriodicRegions {
    pub fn original(&self) -> &periodic::ExactSourceChart {
        &self.original
    }
    pub fn aliases(&self) -> &[MultispanChartAlias] {
        &self.aliases
    }
    pub fn graph(&self) -> &WeightedChartGraph {
        &self.graph
    }
    pub fn regions(&self) -> &WindingRegularization {
        &self.regions
    }
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    /// Search-free exact relation replay on every actual original candidate
    /// curve, complete source chart, weighted contribution and owned region.
    pub fn reverify_source(&self) -> Result<()> {
        let fresh = compact(&self.original, &self.aliases)?;
        let c = compress(&fresh)?;
        if fresh != self.graph
            || self.cycle_edges != c.edges
            || self.original_cycles != c.original_cycles
            || self.regions.source() != materialize(&fresh, &c.edges)
            || self.regions.coefficients() != c.coefficients
        {
            return Err(refused("Original multispan image region binding changed"));
        }
        self.regions.verify()
    }
}
/// Build exact original-source weighted regions using oriented complete
/// multi-span image traversals. No caller assertion grants image authority:
/// every requested pair is independently produced and verified on actual corrected
/// chart curves. max_work bounds original partition predicates; image search
/// uses its separate exact cell/depth limits and at most64 requested pairs.
/// Complete general inter-wire contact arrangement and 3D faces are separate.
pub fn regularize_multispan_periodic_winding(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    requests: &[[usize; 2]],
    tolerance: f64,
    max_work: u64,
) -> Result<MultispanPeriodicRegions> {
    if requests.len() > 64 {
        return Err(refused("Too many original multispan image requests"));
    }
    let original = periodic::assemble_exact_source_chart(
        sources, partitions, windings, chart, anchor, limits, max_work,
    )?;
    let raw = raw(&original)?;
    let mut aliases = Vec::new();
    for &edges in requests {
        if edges[0] == edges[1] || edges.iter().any(|&i| i >= raw.edges.len()) {
            return Err(refused(
                "Original multispan image pair is absent or identical",
            ));
        }
        let a = &raw.edges[edges[0]].curve;
        let b = &raw.edges[edges[1]].curve;
        let mut found = None;
        for orientation in [1, -1] {
            let mut db = b.domain();
            if orientation < 0 {
                db.reverse();
            }
            if let Ok(certificate) = certify_multispan_agreement(a, b, a.domain(), db) {
                found = Some(MultispanChartAlias {
                    edges,
                    orientation,
                    certificate,
                });
                break;
            }
        }
        aliases
            .push(found.ok_or_else(|| refused("Original multispan traversal remains unproven"))?);
    }
    let graph = compact(&original, &aliases)?;
    let c = compress(&graph)?;
    let regions =
        regularize_nonzero_winding(&materialize(&graph, &c.edges), &c.coefficients, tolerance)?;
    let owner = MultispanPeriodicRegions {
        original,
        aliases,
        graph,
        cycle_edges: c.edges,
        original_cycles: c.original_cycles,
        regions,
    };
    owner.reverify_source()?;
    Ok(owner)
}

/// Actual support owned together with the original multispan traversal proof.
#[derive(Clone, Debug)]
pub struct MultispanSurfaceRegions {
    surface: crate::Surface,
    regions: MultispanPeriodicRegions,
    wrapped: [bool; 2],
    limits: periodic::SurfaceCorrespondenceLimits,
}
impl MultispanSurfaceRegions {
    pub fn surface(&self) -> &crate::Surface {
        &self.surface
    }
    pub fn regions(&self) -> &MultispanPeriodicRegions {
        &self.regions
    }
    pub fn correspondence(&self) -> Result<periodic::SurfaceCorrespondence> {
        self.regions.reverify_source()?;
        let report =
            self.regions
                .original
                .verify_surface(&self.surface, self.wrapped, self.limits)?;
        if !report.within_world_tolerance {
            return Err(refused(
                "Original multispan surface correspondence remains unproven",
            ));
        }
        let chart = self.regions.original.chart();
        for component in self.regions.regions.components() {
            for &wire in component.source_loops() {
                for &edge in &self.regions.cycle_edges[wire] {
                    let e = &self.regions.graph.edges[edge];
                    if e.contributions.iter().any(|p| p.source.is_none())
                        && (0..2).any(|a| {
                            !self.wrapped[a]
                                && (0..2).any(|side| {
                                    e.curve
                                        .control_points
                                        .iter()
                                        .all(|p| p[a] == chart[a][side])
                                })
                        })
                    {
                        return Err(refused(
                            "Retained multispan connector uses a nonwrapped axis",
                        ));
                    }
                }
            }
        }
        Ok(report)
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.correspondence().map(|_| ())
    }
}
pub fn bind_multispan_regions_to_surface(
    surface: &crate::Surface,
    regions: &MultispanPeriodicRegions,
    wrapped: [bool; 2],
    limits: periodic::SurfaceCorrespondenceLimits,
) -> Result<MultispanSurfaceRegions> {
    let owner = MultispanSurfaceRegions {
        surface: surface.clone(),
        regions: regions.clone(),
        wrapped,
        limits,
    };
    owner.reverify_source()?;
    Ok(owner)
}
#[derive(Clone)]
pub struct OwnedMultispanPeriodicFaces {
    source: MultispanSurfaceRegions,
    limits: PeriodicFaceLimits,
    built: super::face_build::FaceBuild,
}
impl OwnedMultispanPeriodicFaces {
    pub fn source(&self) -> &MultispanSurfaceRegions {
        &self.source
    }
    pub fn model(&self) -> &crate::Model {
        &self.built.model
    }
    pub fn domains(&self) -> &[[[f64; 2]; 2]] {
        &self.built.domains
    }
    pub fn injectivity(&self) -> &[nurbs_core::surface_injectivity::Report] {
        &self.built.injectivity
    }
    pub fn seam_edges(&self) -> &[usize] {
        &self.built.seam_edges
    }
    pub fn agreement(&self) -> &crate::boundary_agreement::Report {
        &self.built.agreement
    }
    pub fn boundary_error_upper(&self) -> f64 {
        self.built.boundary_error_upper
    }
    pub fn global_spatial_embedding_certified(&self) -> bool {
        false
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = assemble_multispan_periodic_surface_faces(&self.source, self.limits)?;
        self.built.verify_against(&fresh.built)
    }
    pub fn verify_model(&self, model: &crate::Model) -> Result<()> {
        let fresh = assemble_multispan_periodic_surface_faces(&self.source, self.limits)?;
        if *model != fresh.built.model {
            return Err(refused(
                "Saved multispan face differs from its original source ownership",
            ));
        }
        Ok(())
    }
}
/// Actual original support faces: no rounded inverse knot parameter, forged
/// copy-isotopy receipt or unqualified candidate edge is accepted. Uses the
/// common complete physical face obligations, retaining this source proof kind.
/// Global spatial embedding and closed volume admission remain separate.
pub fn assemble_multispan_periodic_surface_faces(
    source: &MultispanSurfaceRegions,
    limits: PeriodicFaceLimits,
) -> Result<OwnedMultispanPeriodicFaces> {
    let correspondence = source.correspondence()?;
    let r = &source.regions;
    let built = super::face_build::build(
        super::face_build::FaceContext {
            surface: &source.surface,
            wrapped: source.wrapped,
            chart: r.original.chart(),
            components: r.regions.components(),
            cycle_edges: &r.cycle_edges,
            graph: &r.graph,
            error_upper: correspondence.error_upper,
        },
        limits,
    )?;
    Ok(OwnedMultispanPeriodicFaces {
        source: source.clone(),
        limits,
        built,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use periodic::{self as p, Piece};
    fn source() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![5., 0.], vec![0., 5.], vec![0., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        }
    }
    fn mapped() -> Curve {
        Curve {
            degree: 6,
            knots: [vec![0.; 7], vec![1.; 7]].concat(),
            control_points: vec![
                vec![0., 0.],
                vec![0., 0.],
                vec![1., 0.],
                vec![3., 0.],
                vec![4., 1.],
                vec![0., 5.],
                vec![0., 0.],
            ],
            weights: vec![1.; 7],
            periodic: false,
        }
    }
    fn parts(c: &Curve, cut: f64) -> Report {
        Report {
            pieces: c
                .split(cut)
                .unwrap()
                .into_iter()
                .map(|curve| Piece {
                    source_domain: curve.domain(),
                    curve,
                    translation: [0.; 2],
                    tile: [0; 2],
                })
                .collect(),
            candidate_hulls_contained: false,
        }
    }
    fn limits() -> p::ChartAssemblyLimits {
        p::ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        }
    }

    #[test]
    fn original_multispan_alias_receipts_cannot_be_forged_or_detached() {
        let a = source();
        let b = mapped();
        let parts = vec![parts(&a, 0.25), parts(&b, 0.5)];
        let owner = regularize_multispan_periodic_winding(
            &[a, b],
            &parts,
            &[[0; 2]; 2],
            [[0., 8.]; 2],
            [1., 1.],
            limits(),
            &[[0, 2], [1, 3]],
            1e-7,
            1_000_000,
        )
        .unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.aliases[0].certificate.steps.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.aliases[0].orientation = -1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.aliases[0].edges[0] = 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.aliases[0].certificate.source.push('x');
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.graph.edges[0].coefficient += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.graph.edges[0].contributions.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.cycle_edges.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.original_cycles.clear();
        assert!(bad.reverify_source().is_err());
    }
}
