//! Fresh exterior two-jet comparisons on original rational supports.
use super::*;
impl CentralLoftCandidate<'_, '_, '_> {
    /// Cover all original boundary spans, including both owned flat poles.
    /// This is exterior G2 evidence only: no topology graft or global embedding
    /// is inferred. Internal U joins retain their separate native inspection.
    pub fn inspect_exterior_jets(
        &self,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<nurbs_core::surface_g2_pcurves::ExtrusionBoundaryReport>> {
        require(
            (18..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound central exterior jet work",
        )?;
        let poles = self.inspect_pole_limits(1. / 64., max_cells / 3)?;
        require(
            poles.iter().all(|p| p.report.proven),
            "Central exterior endpoints need both original pole quotient proofs",
        )?;
        let certificates = [poles[0].certificate.as_ref(), poles[1].certificate.as_ref()];
        let mut used = poles.iter().map(|p| p.report.cells).sum::<usize>();
        let mut exact = 0;
        let mut reports = Vec::new();
        let s = &self.surface;
        let p = s.degree_u;
        let q = s.degree_v;
        for i in 0..6 {
            let pc = &self.pcurves[i];
            let originals = &self.originals[i];
            let report = if i % 2 == 0 {
                let u = (pc.control_points[0][0] + pc.control_points[1][0]) * 0.5;
                let span = (p..s.control_points.len())
                    .find(|&j| s.knots_u[j] <= u && u < s.knots_u[j + 1])
                    .ok_or_else(|| refuse("Original exterior span missing"))?;
                let j = span - p + p / 2;
                let row = if pc.control_points[0][1] == s.knots_v[q] {
                    0
                } else {
                    q
                };
                let next = if row == 0 { 1 } else { q - 1 };
                let axes: Vec<_> = (0..3)
                    .filter(|&k| s.control_points[j][row][k] != s.control_points[j][next][k])
                    .collect();
                require(
                    axes.len() == 1,
                    "Central exterior axis proposal is not unique",
                )?;
                nurbs_core::surface_g2_pcurves::inspect_extrusion_boundary(
                    &self.curves[i],
                    [&originals.1, pc],
                    [&originals.0, s],
                    axes[0],
                    certificates,
                    self.options.tolerances,
                    (max_cells - used) / (6 - i),
                    (max_exact - exact) / (6 - i) as u64,
                )?
            } else {
                let source = &originals.0;
                let axes: Vec<_> = (0..3)
                    .filter(|&k| {
                        source
                            .control_points
                            .iter()
                            .flatten()
                            .all(|p| p[k] == source.control_points[0][0][k])
                    })
                    .collect();
                require(
                    axes.len() == 1,
                    "Original connector support plane is not unique",
                )?;
                nurbs_core::surface_g2_pcurves::inspect_planar_boundary(
                    &self.curves[i],
                    [&originals.1, pc],
                    [source, s],
                    axes[0],
                    certificates,
                    self.options.tolerances,
                    (max_cells - used) / (6 - i),
                    (max_exact - exact) / (6 - i) as u64,
                )?
            };
            used += report.cells;
            exact += report.positions.exact_work;
            require(
                used <= max_cells && exact <= max_exact,
                "Central exterior jets exceeded shared work",
            )?;
            reports.push(report);
        }
        Ok(reports)
    }
}
