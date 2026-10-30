//! Continuous original-span field coverage remains distinct from joined G2.
use super::*;
impl CentralLoftCandidate<'_, '_, '_> {
    /// Recompute positions and complete one-sided normal/tensor field bounds
    /// on each actual original rim trace. No internal join or pole is admitted.
    pub fn inspect_original_span_fields(
        &self,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<nurbs_core::surface_g2_pcurves::FieldCoverage>> {
        require(
            (6..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound original span field coverage work",
        )?;
        let mut used = 0;
        let mut exact = 0;
        let mut reports = Vec::new();
        for i in 0..6 {
            let report = nurbs_core::surface_g2_pcurves::inspect_span_fields(
                &self.curves[i],
                [&self.originals[i].1, &self.pcurves[i]],
                [&self.originals[i].0, &self.surface],
                [false; 2],
                self.options.tolerances[0],
                self.options.tolerances[1],
                self.options.tolerances[2],
                (max_cells - used) / (6 - i),
                (max_exact - exact) / (6 - i) as u64,
            )?;
            used += report.cells;
            exact += report.positions.exact_work;
            require(
                used <= max_cells && exact <= max_exact,
                "Central field coverage exceeded shared work",
            )?;
            reports.push(report);
        }
        Ok(reports)
    }
}

impl CentralLoftCandidate<'_, '_, '_> {
    /// Bound both original one-sided fields at each literal C0 shared column.
    /// Comparing the same surface includes all left/right branch pairs; it
    /// does not assume those branches already agree or certify a sewn body.
    pub fn inspect_internal_span_fields(
        &self,
        max_cells: usize,
        max_exact: u64,
    ) -> Result<Vec<(f64, nurbs_core::surface_g2_pcurves::FieldCoverage)>> {
        require(
            (1..=100000).contains(&max_cells) && max_exact <= 1000000,
            "Bound central internal span work",
        )?;
        let s = &self.surface;
        s.validate()?;
        let p = s.degree_u;
        let n = s.control_points.len();
        let mut columns = Vec::new();
        for right in p..n {
            let knot = s.knots_u[right];
            if knot == s.knots_u[p] || knot == s.knots_u[n] || knot == s.knots_u[right + 1] {
                continue;
            }
            let left = (p..right)
                .rfind(|&i| s.knots_u[i] < s.knots_u[i + 1] && s.knots_u[i + 1] == knot)
                .ok_or_else(|| refuse("Internal central knot lost its left original span"))?;
            require(
                s.knots_u.iter().filter(|&&k| k == knot).count() == p && left == right - p,
                "Internal central trace needs a literal shared C0 column",
            )?;
            columns.push((knot, right - p));
        }
        let mut used = 0;
        let mut exact = 0;
        let mut reports = Vec::new();
        let v = [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]];
        for (i, &(knot, column)) in columns.iter().enumerate() {
            let c = Curve {
                degree: s.degree_v,
                knots: s.knots_v.clone(),
                control_points: s.control_points[column].clone(),
                weights: s.weights[column].clone(),
                periodic: s.periodic_v,
            };
            c.validate()?;
            let pc = Curve::from_polyline(vec![vec![knot, v[0]], vec![knot, v[1]]])?;
            let remaining = columns.len() - i;
            let fields = nurbs_core::surface_g2_pcurves::inspect_span_fields(
                &c,
                [&pc, &pc],
                [s, s],
                [false; 2],
                self.options.tolerances[0],
                self.options.tolerances[1],
                self.options.tolerances[2],
                (max_cells - used) / remaining,
                (max_exact - exact) / remaining as u64,
            )?;
            used += fields.cells;
            exact += fields.positions.exact_work;
            require(
                used <= max_cells && exact <= max_exact,
                "Internal central trace exceeded shared work",
            )?;
            reports.push((knot, fields));
        }
        Ok(reports)
    }
}

impl CentralLoftCandidate<'_, '_, '_> {
    /// Correlated complete original-jet comparisons at all actual interior
    /// U knots. Positive results concern these joins only, not the poles,
    /// exterior rim, embedding, material thickness or sewn volume.
    pub fn inspect_internal_span_joins(
        &self,
        max_cells: usize,
    ) -> Result<Vec<(f64, nurbs_core::surface_g2_pcurves::SpanJoinReport)>> {
        self.inspect_surface_internal_span_joins(&self.surface, max_cells)
    }
    pub(super) fn inspect_surface_internal_span_joins(
        &self,
        surface: &Surface,
        max_cells: usize,
    ) -> Result<Vec<(f64, nurbs_core::surface_g2_pcurves::SpanJoinReport)>> {
        require(
            (1..=100000).contains(&max_cells),
            "Bound correlated internal join work",
        )?;
        let s = surface;
        let domain = [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]];
        let mut knots: Vec<_> = s
            .knots_u
            .iter()
            .copied()
            .filter(|&u| domain[0] < u && u < domain[1])
            .collect();
        knots.dedup();
        let mut used = 0;
        let mut reports = Vec::new();
        for (i, &u) in knots.iter().enumerate() {
            let report = nurbs_core::surface_g2_pcurves::inspect_original_span_join(
                s,
                u,
                self.options.tolerances[1],
                self.options.tolerances[2],
                (max_cells - used) / (knots.len() - i),
            )?;
            used += report.cells;
            require(
                used <= max_cells,
                "Correlated internal joins exceeded shared work",
            )?;
            reports.push((u, report));
        }
        Ok(reports)
    }
}
