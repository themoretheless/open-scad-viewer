//! Exact unchanged original functions followed by complete candidate-region
//! certification. Endpoint ownership alone is never a complete contact census.
use super::*;
use periodic::{ExactSourceChart, WeightedChartGraph};
#[derive(Clone, Debug)]
pub struct ExactPeriodicContactRegions {
    chart: ExactSourceChart,
    cycle_edges: Vec<Vec<usize>>,
    original_cycles: Vec<Vec<usize>>,
    regions: Vec<WindingRegularization>,
}
fn graph(chart: &ExactSourceChart) -> Result<&WeightedChartGraph> {
    let graph = chart
        .graph()
        .ok_or_else(|| refused("Exact original source chart remains unproven"))?;
    if !graph.edges.is_empty() && !chart.contact_ownership().is_some_and(|c| c.complete) {
        return Err(refused(
            "Exact source endpoint contact ownership remains unproven",
        ));
    }
    Ok(graph)
}
impl ExactPeriodicContactRegions {
    pub fn chart(&self) -> &ExactSourceChart {
        &self.chart
    }
    pub fn regions(&self) -> &[WindingRegularization] {
        &self.regions
    }
    pub fn cycle_edges(&self) -> &[Vec<usize>] {
        &self.cycle_edges
    }
    pub fn original_cycles(&self) -> &[Vec<usize>] {
        &self.original_cycles
    }
    pub fn reverify_source(&self) -> Result<()> {
        self.chart.reverify()?;
        let g = graph(&self.chart)?;
        let c = compress(g)?;
        if self.cycle_edges != c.edges
            || self.original_cycles != c.original_cycles
            || self.regions.len() != c.edges.len()
        {
            return Err(refused("Exact source contact region/graph binding changed"));
        }
        let loops = materialize(g, &c.edges);
        separated_interiors(&loops)?;
        for ((region, wire), coefficient) in self.regions.iter().zip(loops).zip(c.coefficients) {
            if region.source() != [wire] || region.coefficients() != [coefficient] {
                return Err(refused("Exact isolated region source binding changed"));
            }
            region.verify()?;
        }
        Ok(())
    }
}
/// Retain exact original-source function identities through weighted flow,
/// endpoint angular ownership and independent complete candidate nonzero-fill
/// certification of each component plus exact separating hulls between components.
/// This admits endpoint contacts between components in separate half planes;
/// nesting, crossings and flat shared boundaries remain refused. Unsupported interior contacts remain refused by that final
/// certification. No strict disjoint-source isotopy flag is fabricated.
pub fn regularize_exact_periodic_contacts(
    sources: &[Curve],
    partitions: &[Report],
    windings: &[[i32; 2]],
    chart: [[f64; 2]; 2],
    anchor: [f64; 2],
    limits: ChartAssemblyLimits,
    profile_tolerance: f64,
    max_source_work: u64,
) -> Result<ExactPeriodicContactRegions> {
    if !profile_tolerance.is_finite() || profile_tolerance <= 0. {
        return Err(refused(
            "Exact contact regions require positive finite profile tolerance",
        ));
    }
    let chart = periodic::assemble_exact_source_chart(
        sources,
        partitions,
        windings,
        chart,
        anchor,
        limits,
        max_source_work,
    )?;
    let c = compress(graph(&chart)?)?;
    let loops = materialize(graph(&chart)?, &c.edges);
    separated_interiors(&loops)?;
    let regions = loops
        .iter()
        .zip(&c.coefficients)
        .map(|(wire, &coefficient)| {
            regularize_nonzero_winding(&[wire.clone()], &[coefficient], profile_tolerance)
        })
        .collect::<Result<Vec<_>>>()?;
    let out = ExactPeriodicContactRegions {
        chart,
        cycle_edges: c.edges,
        original_cycles: c.original_cycles,
        regions,
    };
    out.reverify_source()?;
    Ok(out)
}

// Every complete positive-weight span stays in its closed half plane and its
// open parameter interval stays strictly off the common plane. Thus only span
// endpoints can contact across wires, and their filled interiors are disjoint.
// Search directions are bounded proposals; all comparisons use exact rationals.
fn separated_interiors(loops: &[Vec<Curve>]) -> Result<()> {
    if loops.len() > 32 {
        return Err(refused("Exact contact region wire budget exceeded"));
    }
    let nets = extraction::nets(loops)?;
    for i in 0..loops.len() {
        for j in i + 1..loops.len() {
            let a = nets
                .iter()
                .filter(|(w, _)| *w == i)
                .map(|(_, n)| n)
                .collect::<Vec<_>>();
            let b = nets
                .iter()
                .filter(|(w, _)| *w == j)
                .map(|(_, n)| n)
                .collect::<Vec<_>>();
            let separated = [
                (1, 0),
                (0, 1),
                (1, 1),
                (1, -1),
                (2, 1),
                (2, -1),
                (1, 2),
                (1, -2),
            ]
            .iter()
            .any(|&(nx, ny)| {
                let project = |net: &&Vec<[Q; 3]>| {
                    net.iter()
                        .map(|p| {
                            (&p[0] * Q::from_integer(nx.into())
                                + &p[1] * Q::from_integer(ny.into()))
                                / &p[2]
                        })
                        .collect::<Vec<_>>()
                };
                let x = a.iter().map(project).collect::<Vec<_>>();
                let y = b.iter().map(project).collect::<Vec<_>>();
                let ordered = |x: &Vec<Vec<Q>>, y: &Vec<Vec<Q>>| {
                    let Some(hi) = x.iter().flatten().max() else {
                        return false;
                    };
                    let Some(lo) = y.iter().flatten().min() else {
                        return false;
                    };
                    hi < lo
                        || (hi == lo
                            && x.iter().all(|n| n.iter().any(|q| q < hi))
                            && y.iter().all(|n| n.iter().any(|q| q > lo)))
                };
                ordered(&x, &y) || ordered(&y, &x)
            });
            if !separated {
                return Err(refused(
                    "Complete inter-wire half-plane/contact census remains unproven",
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owned_contact_regions_reject_cached_binding_and_coefficient_damage() {
        let source = [-1., 1.]
            .into_iter()
            .map(|s| Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: vec![
                    vec![0.5, 0.5],
                    vec![0.5 + s * 0.25, 0.5],
                    vec![0.5, 0.5 + s * 0.25],
                    vec![0.5, 0.5],
                ],
                weights: vec![1.; 4],
                periodic: false,
            })
            .collect::<Vec<_>>();
        let parts = source
            .iter()
            .map(|c| Report {
                pieces: vec![periodic::Piece {
                    source_domain: c.domain(),
                    curve: c.clone(),
                    translation: [0.; 2],
                    tile: [0; 2],
                }],
                candidate_hulls_contained: false,
            })
            .collect::<Vec<_>>();
        let limits = ChartAssemblyLimits {
            max_segments: 64,
            max_links: 32,
            max_edges: 32,
            max_images: 256,
            max_cells: 30000,
            tolerance_uv: 1e-8,
        };
        let owner = regularize_exact_periodic_contacts(
            &source,
            &parts,
            &[[0, 0]; 2],
            [[0., 1.]; 2],
            [0.53, 0.53],
            limits,
            1e-7,
            1_000_000,
        )
        .unwrap();
        owner.reverify_source().unwrap();
        let mut bad = owner.clone();
        bad.regions.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.cycle_edges[0].clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner.clone();
        bad.original_cycles.clear();
        assert!(bad.reverify_source().is_err());
        let mut bad = owner;
        bad.regions[0].coefficients[0] = 0;
        assert!(bad.reverify_source().is_err());
    }
}
