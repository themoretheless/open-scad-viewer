//! source profile body operations: qualify_join.
use super::*;
impl<'a, 's, 'p, 'b> TransitionSection<'a, 's, 'p, 'b> {
/// Rebuild this section's owned full corrected bridge recipe and cover
    /// the entire natural section boundary before issuing join authority.
    pub fn qualify_join<'j>(
        &'j self,
        controls: usize,
        cells: usize,
        spans: usize,
    ) -> Result<TransitionSectionJoinReport<'j, 'a, 's, 'p, 'b>> {
        let root = self.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let first = profile.section(root.axis, root.parameters[0])?;
        let last = profile.section(root.axis, root.parameters[1])?;
        let range = first.range();
        let width = root.parameters[1] - root.parameters[0];
        let bridge =
            nurbs_core::normal_alignment::ProfileSectionBridge::new(first, last, [width; 2])?;
        let (bridge, _) = bridge.constrain_original_rails(controls)?;
        let Some(bridge) = bridge else {
            return Ok(TransitionSectionJoinReport {
                join: None,
                regularity: None,
                reason: "profile-section-join-recipe-unproven",
            });
        };
        let regularity =
            bridge.inspect_transverse_corrected_section_join(self.high, range, cells, spans)?;
        let join = regularity
            .complete_regular
            .then_some(TransitionSectionJoin { section: self });
        let reason = if join.is_some() {
            "profile-section-join-qualified"
        } else {
            "profile-section-join-regularity-unproven"
        };
        Ok(TransitionSectionJoinReport {
            join,
            regularity: Some(regularity),
            reason,
        })
    }
/// Whole-source convex hulls and monotonic implicit-root side proofs.
    pub fn qualify_exterior<'r>(
        &'r self,
        max_controls: usize,
        max_driver_cells: usize,
    ) -> Result<ExteriorSectionReport<'r, 'a, 's, 'p, 'b>> {
        use crate::source_boundary_fragment::{Endpoint, Role};
        if max_controls > 100000 || max_driver_cells > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_SECTION_REGION",
                "Bound exterior section work",
            ));
        }
        let mut out = ExteriorSectionReport {
            region: None,
            controls: 0,
            driver_cells: 0,
            reason: "profile-section-exterior-side-unproven",
        };
        let recipe = self.boundary.rooted[0];
        let axis = recipe.axis;
        let parameter = recipe.parameters[usize::from(self.high)];
        let mut sides = Vec::new();
        for wire in &self.contours.loops {
            let contact = wire.last().unwrap().curve();
            if contact.degree != 1
                || contact.control_points.len() != 2
                || contact.control_points.iter().any(|p| p[axis] != parameter)
            {
                return Ok(out);
            }
            let a = &contact.control_points[0];
            let b = &contact.control_points[1];
            if a[1 - axis] == b[1 - axis] {
                return Ok(out);
            }
            let coefficient_positive = if axis == 1 { b[0] > a[0] } else { b[1] < a[1] };
            let mut admitted = None;
            for above in [false, true] {
                let mut proven = true;
                for fragment in &wire[..wire.len() - 1] {
                    let crossing=fragment.endpoints().iter().any(|e|matches!(e,Endpoint::Crossing{point,role:Role::Boundary} if point.contact()==contact));
                    if crossing {
                        let report = crate::source_halfplane_side::prove(
                            fragment,
                            contact,
                            1,
                            above == coefficient_positive,
                            max_driver_cells - out.driver_cells,
                        )?;
                        out.driver_cells += report.driver_cells;
                        if !report.proven {
                            proven = false;
                            break;
                        }
                    } else {
                        for p in &fragment.curve().control_points {
                            if out.controls == max_controls {
                                out.reason = "profile-section-exterior-work-limit";
                                return Ok(out);
                            }
                            out.controls += 1;
                            if !(if above {
                                p[axis] > parameter
                            } else {
                                p[axis] < parameter
                            }) {
                                proven = false;
                                break;
                            }
                        }
                        if !proven {
                            break;
                        }
                    }
                }
                if proven {
                    admitted = Some(above);
                    break;
                }
            }
            let Some(side) = admitted else {
                return Ok(out);
            };
            sides.push(side);
        }
        if sides[0] == sides[1] {
            return Ok(out);
        }
        let kept = usize::from(sides[1] == self.high);
        out.region = Some(ExteriorSection {
            section: self,
            kept,
        });
        out.reason = "profile-section-exterior-side-qualified";
        Ok(out)
    }
pub fn boundary(&self) -> &TransitionBoundary<'_, '_, '_> {
        self.boundary
    }
pub fn edge_index(&self) -> usize {
        if self.high { 1 } else { 3 }
    }
pub fn contours(&self) -> &CutContourProposal {
        &self.contours
    }
}
