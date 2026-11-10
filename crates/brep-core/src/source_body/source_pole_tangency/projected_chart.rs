//! Per-chart globally injective projections on the complete owned pole atlas.
//! This is not a proof that different chart images form a one-sheeted fan.
use super::*;
use nurbs_core::surface_quotient_injectivity::{
    ProjectedPolarCollar, certify_projected_polar_collar,
};
pub struct ProjectedChartLimits {
    pub transverse: TransverseLimits,
    pub subdivisions: usize,
    pub cells: usize,
}
pub struct ProjectedChartAtlas<'a, E: Carrier> {
    transverse: TransverseBodyPoleAtlas<'a, E>,
    charts: Vec<(Address, ProjectedPolarCollar<'a>)>,
    outer_radii: Vec<(usize, f64)>,
}
impl<'a, E: Carrier> ProjectedChartAtlas<'a, E> {
    pub fn transverse(&self) -> &TransverseBodyPoleAtlas<'a, E> {
        &self.transverse
    }
    pub fn charts(&self) -> &[(Address, ProjectedPolarCollar<'a>)] {
        &self.charts
    }
    /// Separation of the full charts' outer radial boundaries from each
    /// projected pole. Does not certify a full disk image or retained trims.
    pub fn outer_radius_bounds(&self) -> &[(usize, f64)] {
        &self.outer_radii
    }
}
pub struct ProjectedChartReport<'a, E: Carrier> {
    pub certificate: Option<ProjectedChartAtlas<'a, E>>,
    pub exact_work: u64,
    pub source_cells: usize,
    pub projection_columns: usize,
    pub projected_cells: usize,
    pub completed_poles: usize,
    pub uncertain_pole: Option<Address>,
    pub reason: &'static str,
}
/// Fresh body/atlas admission and a complete original projected-polar proof
/// for every owned chart. All projected dominance cells share ONE additional
/// allowance. Original 3D injectivity is never substituted for this proof.
pub fn qualify_body_projected_chart_atlas<E: Carrier>(
    body: &Body<E>,
    limits: ProjectedChartLimits,
) -> Result<ProjectedChartReport<'_, E>> {
    if !(1..=64).contains(&limits.subdivisions) || !(1..=100000).contains(&limits.cells) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTED_CHART",
            "Choose bounded projected chart work",
        ));
    }
    let r = qualify_body_transverse_pole_atlas(body, limits.transverse)?;
    let mut out = ProjectedChartReport {
        certificate: None,
        exact_work: r.exact_work,
        source_cells: r.cells,
        projection_columns: r.columns,
        projected_cells: 0,
        completed_poles: 0,
        uncertain_pole: r.uncertain_pole,
        reason: r.reason,
    };
    let Some(transverse) = r.certificate else {
        return Ok(out);
    };
    let shell = body.geometry().shell();
    let mut charts = Vec::new();
    let mut outer = BTreeMap::<usize, f64>::new();
    for ((address, projection), (owned, pole)) in transverse.projections().iter().zip(shell.poles())
    {
        out.uncertain_pole = Some(*address);
        if address != owned || !std::ptr::eq(projection.surface(), pole.source().surface()) {
            out.reason = "source-projected-chart-original-ownership-unproven";
            return Ok(out);
        }
        if out.projected_cells == limits.cells {
            out.reason = "source-projected-chart-cell-limit";
            return Ok(out);
        }
        let p = certify_projected_polar_collar(
            pole.source().surface(),
            projection.boundary(),
            projection.axis(),
            projection.inward_extent(),
            limits.subdivisions,
            limits.cells - out.projected_cells,
        )?;
        out.projected_cells += p.cells;
        let Some(p) = p.certificate else {
            out.reason = p.reason;
            return Ok(out);
        };
        let vertex = shell.vertices()[address.face][address.wire][address.edge][0];
        outer
            .entry(vertex)
            .and_modify(|r| *r = r.min(p.outer_radius_lower()))
            .or_insert(p.outer_radius_lower());
        charts.push((*address, p));
        out.completed_poles += 1;
    }
    if charts.len() != shell.poles().len() {
        out.reason = "source-projected-chart-incomplete-atlas";
        return Ok(out);
    }
    out.certificate = Some(ProjectedChartAtlas {
        transverse,
        charts,
        outer_radii: outer.into_iter().collect(),
    });
    out.uncertain_pole = None;
    out.reason = "source-body-all-original-pole-chart-projections-injective";
    Ok(out)
}
