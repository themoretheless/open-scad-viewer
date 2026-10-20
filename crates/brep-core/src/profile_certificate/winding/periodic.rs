//! Original periodic chain admission followed by exact UV nonzero-fill regions.
mod multispan;
pub use multispan::{MultispanSurfaceRegions, OwnedMultispanPeriodicFaces, bind_multispan_regions_to_surface, assemble_multispan_periodic_surface_faces, MultispanChartAlias, MultispanPeriodicRegions, regularize_multispan_periodic_winding};
use super::*;
mod linear_contacts;
pub use linear_contacts::{LinearPeriodicContactRegions, regularize_linear_periodic_contacts};
mod clock_mapped;
mod face_build;
pub use clock_mapped::{OwnedClockMappedFaces, assemble_clock_mapped_surface_faces, ClockMappedSurfaceRegions, bind_clock_mapped_regions_to_surface, ClockMappedPeriodicRegions, regularize_clock_mapped_periodic_winding};
mod exact_contacts;
pub use exact_contacts::{ExactPeriodicContactRegions, regularize_exact_periodic_contacts};
mod approximate;
mod coherent;
pub use coherent::{OwnedPeriodicFaces, PeriodicFaceLimits, assemble_periodic_surface_faces};
pub use approximate::{
    ApproximatePeriodicWindingRegions, regularize_approximated_periodic_winding,
};
pub use coherent::{
    CoherentPeriodicWindingRegions, SurfacePeriodicRegions, bind_periodic_regions_to_surface,
    regularize_projective_coherent_periodic_winding, regularize_coherent_periodic_winding,
};
use nurbs_core::periodic_trim::{
    self as periodic, ChartAssemblyLimits, Report, WeightedChartAssembly, WeightedChartGraph,
};

#[derive(Clone, Debug)]
pub struct PeriodicWindingRegions {
    source: Vec<Curve>,
    partitions: Vec<Report>,
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    assembly: WeightedChartAssembly,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: WindingRegularization,
}
impl PeriodicWindingRegions {
    pub fn source(&self) -> &[Curve] {
        &self.source
    }
    pub fn partitions(&self) -> &[Report] {
        &self.partitions
    }
    pub fn chart(&self) -> [[f64; 2]; 2] {
        self.chart
    }
    pub fn assembly(&self) -> &WeightedChartAssembly {
        &self.assembly
    }
    pub fn regions(&self) -> &WindingRegularization {
        &self.regions
    }
    /// Dictionary edge IDs for each source wire of regions(), in cyclic order.
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    /// Every original weighted graph cycle occurs in exactly one group.
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    /// Rebuild the bounded original source/chart proof, then independently
    /// replay dictionary flow, cycle compression and output region proofs.
    /// Unlike regions().verify(), original chart re-admission performs search.
    pub fn reverify_source(&self) -> Result<()> {
        let fresh = periodic::assemble_weighted_periodic_chart(
            &self.source,
            &self.partitions,
            self.chart,
            self.anchor,
            self.limits,
        )?;
        let actual = admitted_graph(&self.assembly)?;
        let expected = admitted_graph(&fresh)?;
        if !same_graph(actual, expected) {
            return Err(refused(
                "Periodic region graph is not bound to its original source chart",
            ));
        }
        let compressed = compress(expected)?;
        let wires = materialize(expected, &compressed.edges);
        if self.cycle_edges != compressed.edges
            || self.original_cycles != compressed.original_cycles
            || self.regions.source() != wires
            || self.regions.coefficients() != compressed.coefficients
        {
            return Err(refused(
                "Periodic region cycle bindings or multiplicities changed",
            ));
        }
        self.regions.verify()
    }
}
fn admitted_graph(assembly: &WeightedChartAssembly) -> Result<&WeightedChartGraph> {
    if !assembly.winding_preserved {
        return Err(refused("Original periodic chart winding is unproved"));
    }
    assembly
        .graph
        .as_ref()
        .ok_or_else(|| refused("Original periodic chart has no admitted weighted graph"))
}
fn same_graph(a: &WeightedChartGraph, b: &WeightedChartGraph) -> bool {
    let sources = |p: &[periodic::WeightedSource]| {
        p.iter()
            .map(|p| (p.expanded_edge, p.source, p.orientation))
            .collect::<Vec<_>>()
    };
    a.original_edges == b.original_edges
        && a.expanded_edges == b.expanded_edges
        && a.loops == b.loops
        && a.connector_pairs == b.connector_pairs
        && a.edges.len() == b.edges.len()
        && a.edges.iter().zip(&b.edges).all(|(x, y)| {
            x.curve == y.curve
                && x.coefficient == y.coefficient
                && sources(&x.contributions) == sources(&y.contributions)
        })
        && a.cancellations.len() == b.cancellations.len()
        && a.cancellations
            .iter()
            .zip(&b.cancellations)
            .all(|(x, y)| sources(x) == sources(y))
}
struct Compressed {
    edges: Vec<Vec<usize>>,
    coefficients: Vec<i32>,
    original_cycles: Vec<Vec<usize>>,
}
fn compress(graph: &WeightedChartGraph) -> Result<Compressed> {
    if graph.original_edges > 512
        || graph.expanded_edges > 512
        || graph.edges.len() > 512
        || graph
            .edges
            .iter()
            .any(|e| !(1..=512).contains(&e.coefficient))
        || graph.loops.len() > 512
        || graph.cancellations.len() > 512
    {
        return Err(refused(
            "Weighted periodic region source exceeds its 512-edge owner limit",
        ));
    }
    let mut seen = vec![false; graph.original_edges];
    for (contributions, expected) in graph
        .edges
        .iter()
        .map(|e| (&e.contributions, e.coefficient as i64))
        .chain(graph.cancellations.iter().map(|g| (g, 0)))
    {
        let mut signed = 0_i64;
        for p in contributions {
            if p.expanded_edge >= seen.len()
                || seen[p.expanded_edge]
                || ![-1, 1].contains(&p.orientation)
            {
                return Err(refused(
                    "Weighted source provenance is incomplete or repeated",
                ));
            }
            seen[p.expanded_edge] = true;
            signed += i64::from(p.orientation);
        }
        if signed != expected {
            return Err(refused(
                "Weighted source provenance does not match integer flow",
            ));
        }
    }
    if seen.iter().any(|x| !x) {
        return Err(refused("Weighted source provenance omits an original edge"));
    }
    let mut counts = vec![0usize; graph.edges.len()];
    let mut total = 0usize;
    let mut out = Compressed {
        edges: Vec::new(),
        coefficients: Vec::new(),
        original_cycles: Vec::new(),
    };
    for (cycle_id, cycle) in graph.loops.iter().enumerate() {
        if cycle.is_empty() || cycle.len() > 512 || total.saturating_add(cycle.len()) > 512 {
            return Err(refused(
                "Weighted periodic cycles exceed their complete flow budget",
            ));
        }
        total += cycle.len();
        for &i in cycle {
            let n = counts
                .get_mut(i)
                .ok_or_else(|| refused("Weighted periodic cycle references an unknown edge"))?;
            *n += 1;
        }
        // Collapse a repeated closed walk to its primitive cyclic word. Every
        // occurrence remains accounted for by its integer coefficient.
        let period = (1..=cycle.len())
            .find(|&d| cycle.len() % d == 0 && (0..cycle.len()).all(|i| cycle[i] == cycle[i % d]))
            .unwrap();
        let base = &cycle[..period];
        let key = (0..period)
            .map(|shift| {
                (0..period)
                    .map(|j| base[(j + shift) % period])
                    .collect::<Vec<_>>()
            })
            .min()
            .unwrap();
        let coefficient = (cycle.len() / period) as i32;
        if let Some(i) = out.edges.iter().position(|ids| *ids == key) {
            out.coefficients[i] += coefficient;
            out.original_cycles[i].push(cycle_id);
        } else {
            out.edges.push(key);
            out.coefficients.push(coefficient);
            out.original_cycles.push(vec![cycle_id]);
        }
    }
    if total != graph.expanded_edges
        || counts
            .iter()
            .zip(&graph.edges)
            .any(|(n, e)| *n != e.coefficient || e.coefficient == 0)
    {
        return Err(refused(
            "Weighted cycle census does not cover the exact dictionary flow",
        ));
    }
    // Also independently check the compressed dictionary-flow identity.
    let mut compressed_counts = vec![0usize; graph.edges.len()];
    for (cycle, &coefficient) in out.edges.iter().zip(&out.coefficients) {
        for &i in cycle {
            compressed_counts[i] += coefficient as usize;
        }
    }
    if counts != compressed_counts {
        return Err(refused(
            "Periodic cycle compression changed integer boundary flow",
        ));
    }
    Ok(out)
}
fn materialize(graph: &WeightedChartGraph, cycles: &[Vec<usize>]) -> Vec<Vec<Curve>> {
    cycles
        .iter()
        .map(|ids| ids.iter().map(|&i| graph.edges[i].curve.clone()).collect())
        .collect()
}
/// Recompute original periodic winding admission before deriving regular UV
/// regions. Equal cyclic dictionary words merge with exact integer multiplicity;
/// primitive repeated closed walks are handled without copying geometry. Source
/// correspondence, exact reverse cancellation and seam calibration remain core
/// proof obligations, and every retained original dictionary edge is accounted
/// for. Intersecting/touching distinct cycles and owned supporting-surface faces
/// remain separate obligations. No caller-supplied graph or approval is trusted.
pub fn regularize_periodic_winding(
    source: &[Curve],
    partitions: &[Report],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    profile_tolerance: f64,
) -> Result<PeriodicWindingRegions> {
    if !profile_tolerance.is_finite() || profile_tolerance <= 0. {
        return Err(refused(
            "Periodic region needs a positive finite profile tolerance",
        ));
    }
    let assembly =
        periodic::assemble_weighted_periodic_chart(source, partitions, chart, anchor, limits)?;
    let graph = admitted_graph(&assembly)?;
    let compressed = compress(graph)?;
    let wires = materialize(graph, &compressed.edges);
    let regions = regularize_nonzero_winding(&wires, &compressed.coefficients, profile_tolerance)?;
    Ok(PeriodicWindingRegions {
        source: source.to_vec(),
        partitions: partitions.to_vec(),
        chart,
        anchor,
        limits,
        assembly,
        cycle_edges: compressed.edges,
        original_cycles: compressed.original_cycles,
        regions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn triangle_flow() -> WeightedChartGraph {
        let points = [[0., 0.], [1., 0.], [0., 1.]];
        let edges = (0..3)
            .map(|i| periodic::WeightedChartEdge {
                curve: Curve::from_polyline(vec![points[i].to_vec(), points[(i + 1) % 3].to_vec()])
                    .unwrap(),
                coefficient: 2,
                contributions: [i, i + 3]
                    .into_iter()
                    .map(|expanded_edge| periodic::WeightedSource {
                        expanded_edge,
                        source: Some([expanded_edge / 3, expanded_edge % 3]),
                        orientation: 1,
                    })
                    .collect(),
            })
            .collect();
        WeightedChartGraph {
            edges,
            loops: vec![vec![2, 0, 1, 2, 0, 1]],
            cancellations: Vec::new(),
            connector_pairs: Vec::new(),
            original_edges: 6,
            expanded_edges: 6,
        }
    }
    #[test]
    fn primitive_rotations_preserve_complete_integer_dictionary_flow() {
        let graph = triangle_flow();
        let c = compress(&graph).unwrap();
        assert_eq!(c.edges, vec![vec![0, 1, 2]]);
        assert_eq!(c.coefficients, vec![2]);
        assert_eq!(c.original_cycles, vec![vec![0]]);
        let mut split = graph.clone();
        split.loops = vec![vec![1, 2, 0], vec![2, 0, 1]];
        let c = compress(&split).unwrap();
        assert_eq!(c.edges, vec![vec![0, 1, 2]]);
        assert_eq!(c.coefficients, vec![2]);
        assert_eq!(c.original_cycles, vec![vec![0, 1]]);
        let mut stale = graph.clone();
        stale.edges[0].coefficient = 3;
        assert!(compress(&stale).is_err());
        let mut stale = graph.clone();
        stale.loops[0].truncate(3);
        assert!(compress(&stale).is_err());
        let mut stale = graph.clone();
        stale.edges[1].contributions[0].expanded_edge = 0;
        assert!(compress(&stale).is_err());
        let mut stale = graph;
        stale.loops[0][0] = 3;
        assert!(compress(&stale).is_err());
    }
}

#[cfg(test)]
mod source_binding_tests {
    use super::*;
    #[test]
    fn replay_rejects_stale_periodic_source_graph_and_cycle_bindings() {
        let c = Curve::from_polyline(vec![
            vec![0.25, 0.25],
            vec![0.75, 0.25],
            vec![0.75, 0.75],
            vec![0.25, 0.75],
            vec![0.25, 0.25],
        ])
        .unwrap();
        let p = periodic::split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
        let limits = ChartAssemblyLimits {
            max_segments: 128,
            max_links: 64,
            max_edges: 128,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let result =
            regularize_periodic_winding(&[c], &[p], [[0., 1.]; 2], [0.5, 0.5], limits, 1e-7)
                .unwrap();
        result.reverify_source().unwrap();
        let mut stale = result.clone();
        stale.original_cycles[0].clear();
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.cycle_edges[0].push(0);
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.assembly.graph.as_mut().unwrap().edges[0].coefficient = 2;
        assert!(stale.reverify_source().is_err());
        let mut stale = result.clone();
        stale.source[0].control_points[0][0] += 0.125;
        assert!(stale.reverify_source().is_err());
        let mut stale = result;
        stale.partitions[0].pieces[0].translation[0] += 0.125;
        assert!(stale.reverify_source().is_err());
    }
}
