//! source profile body operations: qualify_support_tangency.
use super::*;
impl<'a, 's, 'p, 'b> TransitionFace<'a, 's, 'p, 'b> {
/// Rebuild the exact owned full-Hermite recipe; cover all four source
    /// support joins under a shared budget. Partial coverage cannot certify.
    pub fn qualify_support_tangency<'f>(
        &'f self,
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionSupportTangencyReport<'f, 'a, 's, 'p, 'b>> {
        if cells > 100000 || spans > 1000000 {
            return Err(Error::new(
                "BREP_PROFILE_TANGENCY",
                "Bound whole support-join work",
            ));
        }
        let mut out = TransitionSupportTangencyReport {
            certificate: None,
            boundaries: Vec::new(),
            cells: 0,
            spans: 0,
            reason: "profile-transition-support-tangency-work-limit",
        };
        if controls == 0 || cells == 0 || spans == 0 {
            return Ok(out);
        }
        let root = self.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let original = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            profile.section(root.axis, root.parameters[0])?,
            profile.section(root.axis, root.parameters[1])?,
            [root.parameters[1] - root.parameters[0]; 2],
        )?;
        let (bridge, _) = original.constrain_original_rails(controls)?;
        let Some(bridge) = bridge else {
            out.reason = "profile-transition-support-tangency-recipe-unproven";
            return Ok(out);
        };
        for section in [true, false] {
            for high in [false, true] {
                if out.cells == cells || out.spans == spans {
                    return Ok(out);
                }
                let report = if section {
                    bridge.inspect_transverse_corrected_section_join(
                        high,
                        bridge.section(false).range(),
                        cells - out.cells,
                        spans - out.spans,
                    )?
                } else {
                    bridge.inspect_transverse_corrected_side_join(
                        high,
                        [0., 1.],
                        cells - out.cells,
                        spans - out.spans,
                    )?
                };
                out.cells += report.cells;
                out.spans += report.spans;
                let complete = report.complete_regular && report.pending.is_empty();
                out.boundaries.push(report);
                if !complete {
                    return Ok(out);
                }
            }
        }
        out.certificate = Some(TransitionSupportTangency { face: self });
        out.reason = "profile-transition-support-tangency-qualified";
        Ok(out)
    }
/// Rebuild the owned whole-face recipe and correlated radius coefficients.
    /// No caller-provided diagnostic bound can construct the certificate.
    pub fn qualify_radius_coefficients<'f>(
        &'f self,
        centers: &nurbs_core::curve::Curve,
        radius: &nurbs_core::curve::Curve,
        law_domains: [[f64; 2]; 2],
        tolerance: f64,
        controls: usize,
        spans: usize,
        coefficient_work: usize,
    ) -> Result<TransitionCoefficientRadiusReport<'f, 'a, 's, 'p, 'b>> {
        if !tolerance.is_finite() || tolerance <= 0. {
            return Err(Error::new(
                "BREP_PROFILE_RADIUS",
                "Choose a positive finite radius tolerance",
            ));
        }
        centers.validate()?;
        radius.validate()?;
        let root = self.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
            profile.section(root.axis, root.parameters[0])?,
            profile.section(root.axis, root.parameters[1])?,
            [root.parameters[1] - root.parameters[0]; 2],
        )?;
        let (bridge, _) = bridge.constrain_original_rails(controls)?;
        let Some(bridge) = bridge else {
            return Ok(TransitionCoefficientRadiusReport {
                certificate: None,
                bound: None,
                reason: "profile-transition-radius-recipe-unproven",
            });
        };
        let bound = bridge.inspect_radius_coefficients(
            centers,
            radius,
            law_domains,
            [bridge.section(false).range(), [0., 1.]],
            spans,
            coefficient_work,
        )?;
        let certificate = bound
            .error_upper
            .filter(|error| {
                error.is_finite() && *error <= tolerance && bound.squared_residual.is_some()
            })
            .map(|error_upper| TransitionRadius {
                face: self,
                centers: centers.clone(),
                radius: radius.clone(),
                law_domains,
                error_upper,
            });
        let reason = if certificate.is_some() {
            "profile-transition-radius-coefficients-qualified"
        } else {
            bound.reason
        };
        Ok(TransitionCoefficientRadiusReport {
            certificate,
            bound: Some(bound),
            reason,
        })
    }
pub fn qualify_radius<'f>(
        &'f self,
        centers: &nurbs_core::curve::Curve,
        radius: &nurbs_core::curve::Curve,
        tolerance: f64,
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionRadiusReport<'f, 'a, 's, 'p, 'b>> {
        centers.validate()?;
        radius.validate()?;
        self.qualify_radius_on_domains(
            centers,
            radius,
            [centers.domain(), radius.domain()],
            tolerance,
            controls,
            cells,
            spans,
        )
    }
pub fn qualify_radius_on_domains<'f>(
        &'f self,
        centers: &nurbs_core::curve::Curve,
        radius: &nurbs_core::curve::Curve,
        law_domains: [[f64; 2]; 2],
        tolerance: f64,
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionRadiusReport<'f, 'a, 's, 'p, 'b>> {
        let root = self.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let first = profile.section(root.axis, root.parameters[0])?;
        let last = profile.section(root.axis, root.parameters[1])?;
        let width = root.parameters[1] - root.parameters[0];
        let bridge =
            nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
        let (bridge, _) = bridge.constrain_original_rails(controls)?;
        let Some(bridge) = bridge else {
            return Ok(TransitionRadiusReport {
                certificate: None,
                coverage: None,
                reason: "profile-transition-radius-recipe-unproven",
            });
        };
        let coverage = bridge.inspect_radius_on_domains(
            centers,
            radius,
            law_domains,
            tolerance,
            cells,
            spans,
        )?;
        let certificate = coverage.complete.then(|| TransitionRadius {
            face: self,
            centers: centers.clone(),
            radius: radius.clone(),
            law_domains,
            error_upper: coverage.accepted_error_upper,
        });
        let reason = coverage.reason;
        Ok(TransitionRadiusReport {
            certificate,
            coverage: Some(coverage),
            reason,
        })
    }
/// Fresh retained-edge admission followed by opposite directed incidence.
    pub fn qualify_rail_incidence<'f>(
        &'f self,
        work_per_rail: u64,
        drivers_per_rail: usize,
        fragments_per_rail: usize,
    ) -> Result<TransitionRailReport<'f, 'a, 's, 'p, 'b>> {
        let mut out = TransitionRailReport {
            incidence: None,
            work: 0,
            reason: "profile-transition-rail-incidence-unproven",
        };
        let mut rails = Vec::new();
        let mut orientation = None;
        for (rail, root) in self.boundary.rooted.iter().enumerate() {
            let report = root.qualify_incident_edge(work_per_rail, drivers_per_rail)?;
            out.work += report.work_used;
            out.reason = report.reason;
            let Some(edge) = report.edge else {
                return Ok(out);
            };
            let contours = root.propose_incident_contours(fragments_per_rail)?;
            out.reason = contours.reason;
            let Some(contours) = contours.proposal else {
                return Ok(out);
            };
            let selected = root.points[0].source_side();
            let neighbor = 1 - selected;
            let ranges = root.points.each_ref().map(|p| p.canonical_bounds());
            let forward = if ranges[0][1] < ranges[1][0] {
                true
            } else if ranges[1][1] < ranges[0][0] {
                false
            } else {
                return Ok(out);
            };
            let transition_reversed = (!forward) ^ (rail == 1);
            let reverse = transition_reversed == edge.reversed()[neighbor];
            if orientation.is_some_and(|value| value != reverse) {
                out.reason = "profile-transition-rail-orientation-conflict";
                return Ok(out);
            }
            orientation = Some(reverse);
            let neighbor_contour = &contours.contours[neighbor];
            let [wire, index] = neighbor_contour.middle;
            if !neighbor_contour.loops[wire][index].same_definition(&edge.uses()[neighbor]) {
                return Ok(out);
            }
            rails.push(TransitionRail {
                edge,
                neighbor_contour: contours.contours.into_iter().nth(neighbor).unwrap(),
                neighbor,
                transition_reversed,
            });
        }
        out.incidence = Some(TransitionRailIncidence {
            face: self,
            rails: [rails.remove(0), rails.remove(0)],
            reverse_face: orientation.unwrap(),
        });
        out.reason = "profile-transition-rail-incidence-qualified";
        Ok(out)
    }
pub fn boundary(&self) -> &TransitionBoundary<'_, '_, '_> {
        self.boundary
    }
pub fn projection(&self) -> [[f64; 3]; 2] {
        self.projection
    }
}
