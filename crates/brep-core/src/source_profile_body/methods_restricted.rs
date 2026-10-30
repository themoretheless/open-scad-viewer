//! source profile body operations: restricted.
use super::*;
impl<'p, 'b> RootEndedSeam<'p, 'b> {
pub fn restricted(&self) -> &RestrictedSeam<'_, '_> {
        &self.restricted
    }
/// Persist authored cuts and their exact owner; carrier roots are rebuilt.
    #[cfg(feature = "codec")]
    pub fn definition(&self) -> Result<value_codec::Value> {
        let end = |parameter| {
            value_codec::json!({"version":1,"kind":"owned-profile-section-end",
            "face":self.face,"edge":self.restricted.edge,"axis":self.axis,
            "parameter":parameter,"highFreeEnd":self.high})
        };
        Ok(
            value_codec::json!({"version":1,"kind":"owned-root-ended-profile-seam",
            "proposal":self.restricted.proposal.definition()?,
            "ends":[end(self.parameters[0]),end(self.parameters[1])]}),
        )
    }
pub fn points(&self) -> &[crate::source_carrier_point::Point; 2] {
        &self.points
    }
/// Bind only the full Hermite-corrected expression with this exact recipe.
    pub fn qualify_transition_side<'s>(
        &'s self,
        bridge: &nurbs_core::normal_alignment::RailConstrainedBridge<'_, '_, '_, '_, '_>,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionSideReport<'s, 'p, 'b>> {
        let mut out = TransitionSideReport {
            side: None,
            regularity: None,
            reason: "profile-transition-side-recipe-unproven",
        };
        let owned = self.restricted.proposal.profile(self.face).unwrap();
        for high in [false, true] {
            let section = bridge.section(high);
            if !std::ptr::eq(section.profile(), owned)
                || section.axis() != self.axis
                || section.parameter() != self.parameters[usize::from(high)]
            {
                return Ok(out);
            }
        }
        let report =
            bridge.inspect_transverse_corrected_side_join(self.high, [0., 1.], cells, spans)?;
        out.reason = report.reason;
        if report.complete_regular {
            out.side = Some(TransitionSide { rooted: self });
            out.reason = "profile-transition-side-qualified";
        }
        out.regularity = Some(report);
        Ok(out)
    }
/// Retain exactly the original interval between both roots on each use.
    /// Traversal follows the original incident use, preserving opposite sides.
    pub fn incident_fragments(&self) -> Result<[crate::source_boundary_fragment::Fragment; 2]> {
        use crate::source_boundary_fragment::{Endpoint, Fragment};
        let mut fragments = Vec::new();
        for side in 0..2 {
            let source = &self.points[0].edge().uses()[side];
            let ranges = [
                self.points[0].parameter_bounds()[side],
                self.points[1].parameter_bounds()[side],
            ];
            let order = if ranges[0][1] < ranges[1][0] {
                [0, 1]
            } else if ranges[1][1] < ranges[0][0] {
                [1, 0]
            } else {
                return Err(Error::new(
                    "BREP_PROFILE_SEAM",
                    "Require strictly ordered carrier ends",
                ));
            };
            let order = if source.reversed() {
                [order[1], order[0]]
            } else {
                order
            };
            fragments.push(Fragment::new(
                source.surface(),
                source.curve(),
                Endpoint::Carrier {
                    point: Box::new(self.points[order[0]].clone()),
                    side,
                },
                Endpoint::Carrier {
                    point: Box::new(self.points[order[1]].clone()),
                    side,
                },
            )?);
        }
        Ok([fragments.remove(0), fragments.remove(0)])
    }
/// Split both original incident contours at the same carrier roots.
    /// The middle use is ready for sewing to the exact transition rail.
    pub fn propose_incident_contours(&self, max_fragments: usize) -> Result<IncidentContourReport> {
        use crate::source_boundary_fragment::Fragment;
        if max_fragments > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTOUR",
                "Bound incident contour work",
            ));
        }
        let shell = self.restricted.proposal.body.geometry().shell();
        let addresses = shell.uses()[self.restricted.edge];
        let regions = shell.regions().unwrap();
        let cost: usize = addresses
            .iter()
            .map(|a| regions[a.face].loops().iter().map(Vec::len).sum::<usize>() + 2)
            .sum();
        let mut out = IncidentContourReport {
            proposal: None,
            fragments: 0,
            reason: "profile-incident-contour-work-limit",
        };
        if cost > max_fragments {
            return Ok(out);
        }
        let middle = self.incident_fragments()?;
        let mut contours = Vec::new();
        for side in 0..2 {
            let address = addresses[side];
            let mut loops = regions[address.face].loops().to_vec();
            let source = &loops[address.wire][address.edge];
            let owned = &self.points[0].edge().uses()[side];
            if !source.same_definition(owned) {
                out.reason = "profile-incident-contour-source-unproven";
                return Ok(out);
            }
            let head = Fragment::new(
                source.surface(),
                source.curve(),
                source.endpoints()[0].clone(),
                middle[side].endpoints()[0].clone(),
            )?;
            let tail = Fragment::new(
                source.surface(),
                source.curve(),
                middle[side].endpoints()[1].clone(),
                source.endpoints()[1].clone(),
            )?;
            loops[address.wire].splice(
                address.edge..=address.edge,
                [head, middle[side].clone(), tail],
            );
            if loops
                .iter()
                .any(|wire| (0..wire.len()).any(|i| !wire[i].joins(&wire[(i + 1) % wire.len()])))
            {
                out.reason = "profile-incident-contour-joins-unproven";
                return Ok(out);
            }
            contours.push(IncidentContour {
                face: address.face,
                loops,
                middle: [address.wire, address.edge + 1],
            });
        }
        out.fragments = cost;
        out.proposal = Some(IncidentContours {
            contours: [contours.remove(0), contours.remove(0)],
        });
        out.reason = "profile-incident-contours-source-joins-qualified";
        Ok(out)
    }
/// Rebuild shared-edge authority over both retained original uses.
    pub fn qualify_incident_edge(
        &self,
        max_work: u64,
        max_driver: usize,
    ) -> Result<crate::source_shared_edge::Report> {
        let fragments = self.incident_fragments()?;
        let original = self.points[0].edge();
        crate::source_mapped_edge::qualify(
            original.world(),
            [&fragments[0], &fragments[1]],
            crate::source_seam_tangency::ranges(original),
            [None, None],
            max_work,
            max_driver,
        )
    }
}
