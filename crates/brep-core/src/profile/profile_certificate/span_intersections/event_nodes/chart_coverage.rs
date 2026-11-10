//! Original retained trim carrier coverage in the actual source surface chart.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalChartCoverageCell {
    pub carrier: usize,
    pub source_cell: usize,
    pub critical_roots: [usize; 4],
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalTrimChartCoverage {
    topology: Arc<OwnedOriginalPeriodicTopology>,
    limits: ParameterOrderLimits,
    cells: Vec<OriginalChartCoverageCell>,
}
fn derive(
    topology: &OwnedOriginalPeriodicTopology,
    limits: ParameterOrderLimits,
) -> Result<Vec<OriginalChartCoverageCell>> {
    topology.reverify_source()?;
    if limits.max_refinements > 512 || !(1..=128).contains(&limits.max_resultant_size) {
        return Err(refused("Invalid original trim chart coverage allowance"));
    }
    let dictionary = topology.seams().dictionary();
    let chart = dictionary
        .chart()
        .chart()
        .map(|a| a.map(|v| Q::from_float(v).unwrap()));
    let owner = dictionary.chart().graph().owner();
    let mut out = Vec::new();
    for &halfedge in dictionary.regions().owner().retained() {
        let carrier = halfedge / 2;
        let c = &owner.carriers()[carrier];
        let bounds = [
            automatic_boundaries::bound(owner, c.source, &c.cuts[0])?,
            automatic_boundaries::bound(owner, c.source, &c.cuts[1])?,
        ];
        let source = cells(&owner.atlas().sources()[c.source])?
            .into_iter()
            .map(|cell| (cell.net, cell.domain))
            .collect::<Vec<_>>();
        for (source_cell, critical_roots) in arithmetic::chart_coverage::prove(
            &source,
            &bounds,
            &chart,
            limits.max_refinements,
            limits.max_resultant_size,
        )? {
            out.push(OriginalChartCoverageCell {
                carrier,
                source_cell,
                critical_roots,
            });
        }
    }
    Ok(out)
}
impl OwnedOriginalTrimChartCoverage {
    pub fn topology(&self) -> &Arc<OwnedOriginalPeriodicTopology> {
        &self.topology
    }
    pub fn cells(&self) -> &[OriginalChartCoverageCell] {
        &self.cells
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.topology, self.limits)? != self.cells {
            return Err(refused(
                "Original trim chart coverage cell/extrema census changed",
            ));
        }
        Ok(())
    }
}
/// Prove every retained original rational carrier lies in the actual closed
/// supporting-surface domain. All active cell endpoints and complete interior
/// extrema are checked at exact algebraic parameters. Control hull failure is
/// not geometry failure; no floating samples or rounded subcurves are used.
/// This admits chart coverage, independently of surface differential regularity
/// and injectivity needed for physical face publication.
pub fn own_original_trim_chart_coverage(
    topology: Arc<OwnedOriginalPeriodicTopology>,
    limits: ParameterOrderLimits,
) -> Result<OwnedOriginalTrimChartCoverage> {
    let cells = derive(&topology, limits)?;
    Ok(OwnedOriginalTrimChartCoverage {
        topology,
        limits,
        cells,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn coverage_replay_rejects_missing_cells_extrema_and_allowance() {
        let v = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
        let wire = v
            .windows(2)
            .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
            .collect();
        let surface = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            periodic_u: false,
            periodic_v: false,
        };
        let regions = Arc::new(
            assemble_original_filled_regions(
                &[wire],
                &[1],
                1e-12,
                Default::default(),
                Default::default(),
            )
            .unwrap(),
        );
        let chart = Arc::new(
            own_original_chart_vertices(
                regions.owner().cycles().graph().clone(),
                [[0., 1.], [0., 1.]],
                [false, false],
                Default::default(),
            )
            .unwrap(),
        );
        let dictionary =
            Arc::new(own_original_seam_dictionary(regions, chart, Default::default()).unwrap());
        let seams = Arc::new(bind_original_seams_to_surface(dictionary, &surface).unwrap());
        let topology = Arc::new(own_original_periodic_topology(seams).unwrap());
        let c = own_original_trim_chart_coverage(topology, Default::default()).unwrap();
        c.reverify_source().unwrap();
        let mut bad = c.clone();
        bad.cells.pop();
        assert!(bad.reverify_source().is_err());
        let mut bad = c.clone();
        bad.cells[0].critical_roots[0] += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = c;
        bad.limits.max_refinements = 513;
        assert!(bad.reverify_source().is_err());
    }
}
