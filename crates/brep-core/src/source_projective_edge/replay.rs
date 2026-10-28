use super::*;
impl Edge {
    /// Original rational parameter maps and both source uses are requalified.
    pub fn requalify(&self, limits: crate::source_shared_edge::ReplayLimits) -> Result<Report> {
        self.requalify_with_uses(self.uses.each_ref().map(|u| u.fragment()), limits)
    }
    pub fn requalify_with_uses(
        &self,
        fragments: [&Fragment; 2],
        limits: crate::source_shared_edge::ReplayLimits,
    ) -> Result<Report> {
        let mut report = initial(limits.exact_work, limits.driver_cells)?;
        if !(1..=100000).contains(&limits.mapping_cells_per_use)
            || self
                .planes
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || self
                .candidates
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
            || self
                .fraction_candidates
                .iter()
                .flatten()
                .flatten()
                .flatten()
                .flatten()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(invalid());
        }
        let mut uses = Vec::with_capacity(2);
        for (original, fragment) in self.uses.iter().zip(fragments) {
            if report.exact_work == limits.exact_work {
                return Ok(report);
            }
            let fresh = fragment.requalify(limits.mapping_cells_per_use)?;
            let r = source_projective_use::qualify(
                original.world(),
                &fresh,
                original.range(),
                original.weights(),
                limits.exact_work - report.exact_work,
            )?;
            report.exact_work += r.exact_work;
            let Some(u) = r.mapped else {
                report.reason = r.reason;
                return Ok(report);
            };
            uses.push(u);
        }
        finish(
            [uses.remove(0), uses.remove(0)],
            self.planes,
            self.candidates,
            self.fraction_candidates,
            report,
            limits.exact_work,
            limits.driver_cells,
        )
    }
}
