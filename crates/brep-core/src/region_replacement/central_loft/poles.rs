//! Original central pole limits and local quotient ownership.
use super::*;
impl CentralLoftCandidate<'_, '_, '_> {
    /// Propose each height axis from the actual admitted stock planes, then
    /// prove the literal pole factors and local quotient on the actual net.
    /// `band` caps the search neighborhood. Each returned certificate records
    /// its ACTUAL proved band; no uncovered seam interior or closed corner
    /// is admitted by this local result.
    pub fn inspect_pole_limits(
        &self,
        band: f64,
        max_cells: usize,
    ) -> Result<Vec<nurbs_core::surface_shape_operator::FlatPoleInspection<'_>>> {
        self.inspect_surface_pole_limits(&self.surface, band, max_cells)
    }
    pub(super) fn inspect_surface_pole_limits<'a>(
        &self,
        surface: &'a Surface,
        band: f64,
        max_cells: usize,
    ) -> Result<Vec<nurbs_core::surface_shape_operator::FlatPoleInspection<'a>>> {
        require(
            band.is_finite() && band > 0. && band <= 1. && (2..=100000).contains(&max_cells),
            "Bound original central pole neighborhoods",
        )?;
        let mut used = 0;
        let mut reports = Vec::new();
        for (i, end) in [false, true].into_iter().enumerate() {
            let row = if end {
                surface.control_points.len() - 1
            } else {
                0
            };
            let p = &surface.control_points[row][0];
            let axis = (0..3)
                .min_by(|&a, &b| {
                    let distance = |k: usize| {
                        (p[k] - self.stock[k][0])
                            .abs()
                            .min((p[k] - self.stock[k][1]).abs())
                    };
                    distance(a).total_cmp(&distance(b))
                })
                .unwrap();
            let lower = nurbs_core::interval_eval::Interval::point(p[axis]).sub(
                nurbs_core::interval_eval::Interval::point(self.stock[axis][0]),
            )?;
            let upper = nurbs_core::interval_eval::Interval::point(p[axis]).sub(
                nurbs_core::interval_eval::Interval::point(self.stock[axis][1]),
            )?;
            let distance = lower
                .lo
                .abs()
                .max(lower.hi.abs())
                .min(upper.lo.abs().max(upper.hi.abs()));
            require(
                distance <= self.options.tolerances[0],
                "Original central pole does not meet a stock support",
            )?;
            let allowance = (max_cells - used) / (2 - i);
            let degree = surface.degree_u;
            let first = if end {
                surface.control_points.len() - degree - 1
            } else {
                0
            };
            let weights = surface.weights[first..first + degree + 1]
                .iter()
                .flatten()
                .copied()
                .collect::<Vec<_>>();
            let low = weights.iter().copied().fold(f64::INFINITY, f64::min);
            let high = weights.iter().copied().fold(0., f64::max);
            // Weight ratios propose a local scale only. Every trial must
            // still prove the ORIGINAL factors, field bounds and quotient.
            let proposed = band * (low / high);
            let mut actual_band = if proposed > 0. {
                proposed.min(band)
            } else {
                band
            };
            let mut spent = 0;
            let proof = loop {
                let mut trial = nurbs_core::surface_shape_operator::inspect_original_flat_pole(
                    surface,
                    end,
                    axis,
                    actual_band,
                    (allowance - spent).min(512),
                )?;
                spent += trial.report.cells;
                if trial.report.proven
                    || trial.report.cells == 0
                    || spent == allowance
                    || actual_band * 0.5 == 0.
                {
                    trial.report.cells = spent;
                    break trial;
                }
                actual_band *= 0.5;
            };
            used += proof.report.cells;
            require(
                used <= max_cells,
                "Original central poles exceeded shared work",
            )?;
            reports.push(proof);
        }
        Ok(reports)
    }
}
