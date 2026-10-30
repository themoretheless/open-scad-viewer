//! Original central pole limits and local quotient ownership.
use super::*;
impl CentralLoftCandidate<'_, '_, '_> {
    /// Propose each height axis from the actual admitted stock planes, then
    /// prove the literal pole factors and local quotient on the actual net.
    /// This does not admit the exterior seam interiors or a closed corner.
    pub fn inspect_pole_limits(
        &self,
        band: f64,
        max_cells: usize,
    ) -> Result<Vec<nurbs_core::surface_shape_operator::FlatPoleInspection<'_>>> {
        require(
            band.is_finite() && band > 0. && band <= 1. && (2..=100000).contains(&max_cells),
            "Bound original central pole neighborhoods",
        )?;
        let mut used = 0;
        let mut reports = Vec::new();
        for (i, end) in [false, true].into_iter().enumerate() {
            let row = if end {
                self.surface.control_points.len() - 1
            } else {
                0
            };
            let p = &self.surface.control_points[row][0];
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
            let proof = nurbs_core::surface_shape_operator::inspect_original_flat_pole(
                &self.surface,
                end,
                axis,
                band,
                (max_cells - used) / (2 - i),
            )?;
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
