//! Tolerance-qualified original-source topology and exact candidate UV regions.
use super::*;
use periodic::{ApproximateChartLimits, ApproximateWeightedChartAssembly};
#[derive(Clone, Debug)]
pub struct ApproximatePeriodicWindingRegions {
    source: Vec<Curve>,
    partitions: Vec<Report>,
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    assembly: ApproximateWeightedChartAssembly,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: WindingRegularization,
}
fn admitted(assembly: &ApproximateWeightedChartAssembly) -> Result<&WeightedChartGraph> {
    if !assembly.candidate_winding_preserved
        || assembly.transfer.winding.is_none()
        || !assembly.transfer.isotopy.periodic_isotopy_preserved
        || !assembly.transfer.isotopy.lifted_isotopy_preserved
        || assembly.transfer.source.as_ref().and_then(|q| q.winding) != assembly.transfer.winding
    {
        return Err(refused(
            "Approximate original topology or candidate winding remains unproved",
        ));
    }
    assembly
        .graph
        .as_ref()
        .ok_or_else(|| refused("Approximate periodic chart has no admitted weighted graph"))
}
impl ApproximatePeriodicWindingRegions {
    pub fn source(&self) -> &[Curve] {
        &self.source
    }
    pub fn partitions(&self) -> &[Report] {
        &self.partitions
    }
    pub fn assembly(&self) -> &ApproximateWeightedChartAssembly {
        &self.assembly
    }
    /// Exact filled regions of the admitted moved candidate. Source topology
    /// follows the separate continuous isotopy, not pointwise boundary equality.
    pub fn candidate_regions(&self) -> &WindingRegularization {
        &self.regions
    }
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    /// Fresh bounded source/candidate isotopy, image separation and anchor
    /// admission, followed by source-bound integer flow and region replay.
    pub fn reverify_source(&self) -> Result<()> {
        let fresh = periodic::assemble_approximated_weighted_chart(
            &self.source,
            &self.partitions,
            self.chart,
            self.anchor,
            self.limits,
        )?;
        let graph = admitted(&fresh)?;
        if self.assembly.transfer.winding != fresh.transfer.winding
            || !same_graph(admitted(&self.assembly)?, graph)
        {
            return Err(refused(
                "Approximate periodic graph changed from its original source admission",
            ));
        }
        let c = compress(graph)?;
        if self.cycle_edges != c.edges
            || self.original_cycles != c.original_cycles
            || self.regions.source() != materialize(graph, &c.edges)
            || self.regions.coefficients() != c.coefficients
        {
            return Err(refused(
                "Approximate periodic region source/cycle binding changed",
            ));
        }
        self.regions.verify()
    }
}
/// Recompute continuous original-to-candidate periodic topology and anchor
/// winding before constructing exact nonzero-filled candidate UV regions.
/// All isotopy/image-separation/work gates remain mandatory. Coincident source
/// loops need an admissible coherent isotopy; source/candidate boundaries need
/// not coincide pointwise. Supporting-surface and 3D face admission are separate.
pub fn regularize_approximated_periodic_winding(
    source: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ApproximateChartLimits,
    profile_tolerance: f64,
) -> Result<ApproximatePeriodicWindingRegions> {
    if !profile_tolerance.is_finite() || profile_tolerance <= 0. {
        return Err(refused(
            "Approximate periodic regions need a finite positive profile tolerance",
        ));
    }
    let assembly =
        periodic::assemble_approximated_weighted_chart(source, partitions, chart, anchor, limits)?;
    let graph = admitted(&assembly)?;
    let c = compress(graph)?;
    let regions = regularize_nonzero_winding(
        &materialize(graph, &c.edges),
        &c.coefficients,
        profile_tolerance,
    )?;
    Ok(ApproximatePeriodicWindingRegions {
        source: source.to_vec(),
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_rejects_wrong_original_anchor_integer_and_unproved_isotopy() {
        let c = Curve::from_polyline(vec![
            vec![0.75, 0.25],
            vec![1.125, 0.25],
            vec![1.125, 0.75],
            vec![0.75, 0.75],
            vec![0.75, 0.25],
        ])
        .unwrap();
        let p = periodic::split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.01, 16, 32, 10000, 30000)
            .unwrap()
            .partition
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
        let result = regularize_approximated_periodic_winding(
            &[c],
            &[p],
            [[0., 1.]; 2],
            [0.875, 0.5],
            limits,
            1e-7,
        )
        .unwrap();
        let mut stale = result.clone();
        stale.assembly.transfer.winding = Some(2);
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.assembly.transfer.winding = Some(2);
        stale.assembly.transfer.source.as_mut().unwrap().winding = Some(2);
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.assembly.transfer.isotopy.periodic_isotopy_preserved = false;
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.partitions[0].pieces[0].translation[0] += 0.125;
        assert!(stale.reverify_source().is_err());
        let mut stale = result;
        stale.cycle_edges[0].push(0);
        assert!(stale.reverify_source().is_err());
    }
}
