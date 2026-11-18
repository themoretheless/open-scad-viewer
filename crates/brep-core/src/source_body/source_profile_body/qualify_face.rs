//! source profile body operations: qualify_face.
use super::*;
impl<'e, 'a, 's, 'p, 'b> ExteriorSection<'e, 'a, 's, 'p, 'b> {
/// Fresh global quotient proof plus complete owned-region regularity.
    pub fn qualify_face<'r>(
        &'r self,
        collapsed_end: usize,
        subdivisions: usize,
        quotient_cells: usize,
        cells: usize,
        spans: usize,
    ) -> Result<RetainedProfileFaceReport<'r, 'e, 'a, 's, 'p, 'b>> {
        let quotient =
            self.inspect_quotient_injectivity(collapsed_end, subdivisions, quotient_cells)?;
        let mut out = RetainedProfileFaceReport {
            face: None,
            quotient,
            ordinary: None,
            pole: None,
            reason: "profile-retained-global-injectivity-unproven",
        };
        if !out.quotient.proven {
            return Ok(out);
        }
        let (profile, domain) = self.profile_domain();
        let source = profile.source();
        let pole_u = source.knots_u[if collapsed_end == 0 {
            source.degree_u
        } else {
            source.control_points.len()
        }];
        if domain[0][0] <= pole_u && pole_u <= domain[0][1] {
            let report = self.inspect_factored_pole_regularity(collapsed_end == 1, cells, spans)?;
            out.reason = report.reason;
            if report.complete_factored {
                out.face = Some(RetainedProfileFace {
                    exterior: self,
                    factored_boundary: Some(collapsed_end),
                });
            }
            out.pole = Some(report);
        } else {
            let report = self.inspect_regularity(cells, spans)?;
            out.reason = report.reason;
            if report.complete_regular {
                out.face = Some(RetainedProfileFace {
                    exterior: self,
                    factored_boundary: None,
                });
            }
            out.ordinary = Some(report);
        }
        if out.face.is_some() {
            out.reason = "profile-retained-face-embedded-qualified";
        }
        Ok(out)
    }
/// Inspect a rectangle containing the complete qualified exterior region.
    /// No original-surface immersion certificate is inherited by the profile.
    pub fn inspect_regularity(
        &self,
        cells: usize,
        spans: usize,
    ) -> Result<nurbs_core::normal_alignment::TransitionRegularityReport> {
        let (profile, domain) = self.profile_domain();
        profile.inspect_regularity(domain, cells, spans)
    }
pub fn inspect_factored_pole_regularity(
        &self,
        high: bool,
        cells: usize,
        spans: usize,
    ) -> Result<nurbs_core::normal_alignment::ProfilePoleCover> {
        let (profile, domain) = self.profile_domain();
        profile.inspect_factored_pole_regularity(domain, high, cells, spans)
    }
/// Full-chart quotient injectivity also covers this retained subregion.
    pub fn inspect_quotient_injectivity(
        &self,
        collapsed_end: usize,
        subdivisions: usize,
        cells: usize,
    ) -> Result<nurbs_core::surface_quotient_injectivity::Report> {
        let (profile, _) = self.profile_domain();
        nurbs_core::surface_quotient_injectivity::certify_profile_source_frame(
            profile,
            collapsed_end,
            subdivisions,
            cells,
        )
    }
pub(super) fn profile_domain(&self) -> (&WeightProfile<'_>, [[f64; 2]; 2]) {
        let recipe = self.section.boundary.rooted[0];
        let profile = recipe.restricted.proposal.profile(recipe.face).unwrap();
        let source = profile.source();
        let mut domain = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
        domain[recipe.axis][usize::from(!self.section.high)] =
            recipe.parameters[usize::from(self.section.high)];
        (profile, domain)
    }
pub fn section(&self) -> &TransitionSection<'_, '_, '_, '_> {
        self.section
    }
pub fn kept_loop(&self) -> &[crate::source_boundary_fragment::Fragment] {
        &self.section.contours.loops[self.kept]
    }
pub fn removed_loop(&self) -> &[crate::source_boundary_fragment::Fragment] {
        &self.section.contours.loops[1 - self.kept]
    }
}
