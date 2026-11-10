//! Actual complete supporting-surface regularity after original trim admission.
use super::*;
use std::sync::Arc;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalSurfaceRegularityRecord {
    pub cells: usize,
    pub proven_cells: usize,
    /// Sufficient basis continuity only, not geometric or periodic seam G1.
    pub interior_basis_c1: bool,
}
#[derive(Clone, Debug)]
pub struct OwnedOriginalRegularTrimSurface {
    coverage: Arc<OwnedOriginalTrimChartCoverage>,
    max_cells: usize,
    record: OriginalSurfaceRegularityRecord,
}
fn derive(
    coverage: &OwnedOriginalTrimChartCoverage,
    max_cells: usize,
) -> Result<OriginalSurfaceRegularityRecord> {
    coverage.reverify_source()?;
    let surface = coverage.topology().seams().surface();
    let report = nurbs_core::surface_regularity::inspect(surface, max_cells)?;
    if !report.spanwise_regular
        || !report.unresolved.is_empty()
        || report.cells == 0
        || report.proven_cells == 0
    {
        return Err(refused(
            "Actual supporting-surface nonzero normals remain unresolved",
        ));
    }
    Ok(OriginalSurfaceRegularityRecord {
        cells: report.cells,
        proven_cells: report.proven_cells,
        interior_basis_c1: report.interior_basis_c1,
    })
}
impl OwnedOriginalRegularTrimSurface {
    pub fn coverage(&self) -> &Arc<OwnedOriginalTrimChartCoverage> {
        &self.coverage
    }
    pub fn record(&self) -> &OriginalSurfaceRegularityRecord {
        &self.record
    }
    pub fn reverify_source(&self) -> Result<()> {
        if derive(&self.coverage, self.max_cells)? != self.record {
            return Err(refused(
                "Original supporting-surface regularity source or cell receipt changed",
            ));
        }
        Ok(())
    }
}
/// Independently prove a nonzero one-sided normal over every original surface
/// knot rectangle closure, with outward interval subdivision. Unresolved
/// rectangles never become regular by sampling. This stronger full-source gate
/// may refuse singularities outside the retained trim; trim-local proof remains
/// a separate future admission path. No global injectivity, periodic G1 or
/// physical face publication is implied by regularity alone.
pub fn own_original_regular_trim_surface(
    coverage: Arc<OwnedOriginalTrimChartCoverage>,
    max_cells: usize,
) -> Result<OwnedOriginalRegularTrimSurface> {
    let record = derive(&coverage, max_cells)?;
    Ok(OwnedOriginalRegularTrimSurface {
        coverage,
        max_cells,
        record,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn regularity_replay_rejects_counts_continuity_and_allowance_changes() {
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
        let coverage =
            Arc::new(own_original_trim_chart_coverage(topology, Default::default()).unwrap());
        let r = own_original_regular_trim_surface(coverage, 1).unwrap();
        r.reverify_source().unwrap();
        let mut bad = r.clone();
        bad.record.proven_cells += 1;
        assert!(bad.reverify_source().is_err());
        let mut bad = r.clone();
        bad.record.interior_basis_c1 = false;
        assert!(bad.reverify_source().is_err());
        let mut bad = r;
        bad.max_cells = 0;
        assert!(bad.reverify_source().is_err());
    }
}
