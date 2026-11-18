//! Copy-coherent weighted source topology connected to exact candidate regions.
use super::*;
mod surface;
use periodic::{ApproximateChartLimits, CoherentWeightedChartAssembly};
pub use surface::{OwnedPeriodicFaces, PeriodicFaceLimits, assemble_periodic_surface_faces};
pub use surface::{SurfacePeriodicRegions, bind_periodic_regions_to_surface};
#[derive(Clone, Debug)]
enum SourceCoherence {
    NormalizedFunctions,
    Projective {
        correspondences: Vec<periodic::ProjectivePartitionHomotopy>,
        max_work: u64,
    },
}
fn source_assembly(
    source: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    proof: &SourceCoherence,
) -> Result<CoherentWeightedChartAssembly> {
    match proof {
        SourceCoherence::NormalizedFunctions => {
            periodic::assemble_coherent_weighted_chart(source, partitions, chart, anchor, limits)
        }
        SourceCoherence::Projective {
            correspondences,
            max_work,
        } => Ok(periodic::assemble_projective_coherent_weighted_chart(
            source,
            partitions,
            correspondences,
            chart,
            anchor,
            limits,
            *max_work,
        )?
        .assembly),
    }
}
#[derive(Clone, Debug)]
pub struct CoherentPeriodicWindingRegions {
    source: Vec<Curve>,
    source_coherence: SourceCoherence,
    partitions: Vec<Report>,
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    assembly: CoherentWeightedChartAssembly,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: WindingRegularization,
}
fn admitted(a: &CoherentWeightedChartAssembly) -> Result<&WeightedChartGraph> {
    if !a.candidate_winding_preserved
        || !a.transfer.coherent_source_winding_preserved
        || a.transfer.winding.is_none()
        || a.transfer.original_source.as_ref().and_then(|q| q.winding) != a.transfer.winding
        || !a.transfer.quotient.isotopy.periodic_isotopy_preserved
        || !a.transfer.quotient.isotopy.lifted_isotopy_preserved
        || a.transfer.quotient.winding.is_none()
    {
        return Err(refused(
            "Copy-coherent original topology or candidate graph is unproved",
        ));
    }
    a.graph
        .as_ref()
        .ok_or_else(|| refused("Copy-coherent candidate graph is absent"))
}
impl CoherentPeriodicWindingRegions {
    pub fn source(&self) -> &[Curve] {
        &self.source
    }
    pub fn partitions(&self) -> &[Report] {
        &self.partitions
    }
    pub fn assembly(&self) -> &CoherentWeightedChartAssembly {
        &self.assembly
    }
    pub fn candidate_regions(&self) -> &WindingRegularization {
        &self.regions
    }
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    /// Recompute the genuine quotient/coherent source proof, actual full
    /// original winding, source graph and complete candidate-region bindings.
    pub fn reverify_source(&self) -> Result<()> {
        let fresh = source_assembly(
            &self.source,
            &self.partitions,
            self.chart,
            self.anchor,
            self.limits,
            &self.source_coherence,
        )?;
        let graph = admitted(&fresh)?;
        if self.assembly.transfer.source_orientations != fresh.transfer.source_orientations
            || self.assembly.transfer.groups != fresh.transfer.groups
            || self.assembly.transfer.representatives != fresh.transfer.representatives
            || self.assembly.transfer.winding != fresh.transfer.winding
            || self.assembly.transfer.quotient.winding != fresh.transfer.quotient.winding
            || !same_graph(admitted(&self.assembly)?, graph)
        {
            return Err(refused(
                "Copy-coherent source grouping, anchor or graph changed",
            ));
        }
        let c = compress(graph)?;
        if self.cycle_edges != c.edges
            || self.original_cycles != c.original_cycles
            || self.regions.source() != materialize(graph, &c.edges)
            || self.regions.coefficients() != c.coefficients
        {
            return Err(refused("Copy-coherent source/cycle region binding changed"));
        }
        self.regions.verify()
    }
}
/// Original same-parameter copy coherence and weighted chart admission before
/// exact nonzero candidate-region normalization. Quotient embedding is kept
/// distinct from the original weighted source proof. Supporting-surface faces,
/// reversed-family aliases and general contacts remain separate obligations.
fn regularize_coherent_periodic_winding_impl(
    source: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    profile_tolerance: f64,
    source_coherence: SourceCoherence,
) -> Result<CoherentPeriodicWindingRegions> {
    if !profile_tolerance.is_finite() || profile_tolerance <= 0. {
        return Err(refused(
            "Copy-coherent regions need a finite positive profile tolerance",
        ));
    }
    let assembly = source_assembly(source, partitions, chart, anchor, limits, &source_coherence)?;
    let graph = admitted(&assembly)?;
    let c = compress(graph)?;
    let regions = regularize_nonzero_winding(
        &materialize(graph, &c.edges),
        &c.coefficients,
        profile_tolerance,
    )?;
    Ok(CoherentPeriodicWindingRegions {
        source: source.to_vec(),
        source_coherence,
        partitions: partitions.to_vec(),
        chart,
        anchor,
        limits,
        assembly,
        cycle_edges: c.edges,
        original_cycles: c.original_cycles,
        regions,
    })
}

/// Original exact normalized-function source admission.
pub fn regularize_coherent_periodic_winding(
    source: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    profile_tolerance: f64,
) -> Result<CoherentPeriodicWindingRegions> {
    regularize_coherent_periodic_winding_impl(
        source,
        partitions,
        chart,
        anchor,
        limits,
        profile_tolerance,
        SourceCoherence::NormalizedFunctions,
    )
}
/// Own exact nonlinear source/candidate homotopy receipts alongside original
/// curves and seam partitions. Fresh source replay retains the projective proof
/// kind and shared work allowance through region, actual-support and face binding.
pub fn regularize_projective_coherent_periodic_winding(
    source: &[Curve],
    partitions: &[Report],
    correspondences: &[periodic::ProjectivePartitionHomotopy],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    profile_tolerance: f64,
    max_correspondence_work: u64,
) -> Result<CoherentPeriodicWindingRegions> {
    regularize_coherent_periodic_winding_impl(
        source,
        partitions,
        chart,
        anchor,
        limits,
        profile_tolerance,
        SourceCoherence::Projective {
            correspondences: correspondences.to_vec(),
            max_work: max_correspondence_work,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_replay_retains_nonlinear_proof_kind_and_shared_allowance() {
        let a = Curve {
            degree: 4,
            knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
            control_points: vec![
                vec![0.5, 0.5],
                vec![0.75, 0.5],
                vec![0.5, 0.75],
                vec![0.25, 0.5],
                vec![0.5, 0.5],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        let mut b = a.clone();
        b.weights = vec![1., 2., 4., 8., 16.];
        let part = |c: &Curve| Report {
            pieces: vec![periodic::Piece {
                source_domain: c.domain(),
                translation: [0.; 2],
                tile: [0; 2],
                curve: c.clone(),
            }],
            candidate_hulls_contained: false,
        };
        let pa = part(&a);
        let pb = part(&b);
        let proof = periodic::verify_projective_partition_homotopy(
            &a,
            &b,
            &pa,
            &pb,
            [2., 1.],
            false,
            1_000_000,
        )
        .unwrap()
        .unwrap();
        let limits = ApproximateChartLimits {
            isotopy: periodic::IsotopyLimits {
                tolerance_uv: 0.01,
                max_deviation_cells: 40000,
                max_jet_cells: 256,
                max_pairs: 20000,
            },
            max_edges: 128,
            max_images: 256,
            max_cells: 30000,
            max_checks: 10000,
            tolerance_uv: 1e-8,
        };
        let owned = regularize_projective_coherent_periodic_winding(
            &[a, b],
            &[pa, pb],
            &[proof],
            [[0., 2.]; 2],
            [0.5, 0.55],
            limits,
            1e-7,
            1_000_000,
        )
        .unwrap();
        owned.reverify_source().unwrap();
        let mut bad = owned.clone();
        bad.source_coherence = SourceCoherence::NormalizedFunctions;
        assert!(bad.reverify_source().is_err());
        let mut bad = owned.clone();
        if let SourceCoherence::Projective { max_work, .. } = &mut bad.source_coherence {
            *max_work = 0;
        }
        assert!(bad.reverify_source().is_err());
        let mut bad = owned.clone();
        if let SourceCoherence::Projective {
            correspondences, ..
        } = &mut bad.source_coherence
        {
            correspondences.clear();
        }
        assert!(bad.reverify_source().is_err());
    }
    #[test]
    fn replay_rejects_changed_copy_groups_original_integer_and_region_multiplicity() {
        let c = Curve::from_polyline(vec![
            vec![0.25, 0.25],
            vec![0.75, 0.25],
            vec![0.75, 0.75],
            vec![0.25, 0.75],
            vec![0.25, 0.25],
        ])
        .unwrap();
        let p = periodic::split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
        let limits = ApproximateChartLimits {
            isotopy: periodic::IsotopyLimits {
                tolerance_uv: 0.01,
                max_deviation_cells: 40000,
                max_jet_cells: 256,
                max_pairs: 20000,
            },
            max_edges: 128,
            max_images: 256,
            max_cells: 30000,
            max_checks: 10000,
            tolerance_uv: 1e-8,
        };
        let result = regularize_coherent_periodic_winding(
            &[c.clone(), c],
            &[p.clone(), p],
            [[0., 1.]; 2],
            [0.5, 0.5],
            limits,
            1e-7,
        )
        .unwrap();
        let mut stale = result.clone();
        stale.assembly.transfer.source_orientations[0] = -1;
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.assembly.transfer.groups[0].pop();
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.assembly.transfer.winding = Some(1);
        stale
            .assembly
            .transfer
            .original_source
            .as_mut()
            .unwrap()
            .winding = Some(1);
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.regions.coefficients[0] = 1;
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.original_cycles[0].clear();
        assert!(stale.reverify_source().is_err());
        result.reverify_source().unwrap();
    }
}
