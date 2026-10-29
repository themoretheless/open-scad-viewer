//! Contact along an owned profile section from disjoint projected subregions.
use super::*;
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
pub struct TransitionSectionContact<'x> {
    owners: [&'x ShellFace; 2],
    section: &'x Fragment,
    end: usize,
    axis: usize,
    parameter: f64,
}
impl TransitionSectionContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn section(&self) -> &Fragment {
        self.section
    }
    pub fn end(&self) -> usize {
        self.end
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
    pub fn parameter(&self) -> f64 {
        self.parameter
    }
}
pub struct TransitionSectionContactReport<'x> {
    pub certificate: Option<TransitionSectionContact<'x>>,
    pub controls: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Full-profile Jordan authority separates the retained exterior from the
    /// transition image except their common section. The exact bridge recipe
    /// specializes to that same profile section at its across endpoint.
    pub fn qualify_transition_section_contact<'x>(
        &'x self,
        end: usize,
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        max_controls: usize,
    ) -> Result<TransitionSectionContactReport<'x>> {
        if end > 1 || max_controls > 1000000 {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned section contact work",
            ));
        }
        let mut out = TransitionSectionContactReport {
            certificate: None,
            controls: 0,
            reason: "profile-transition-section-contact-unproven",
        };
        let boundary = self.assembly.patch.rails.face.boundary;
        if !std::ptr::eq(region.face().boundary(), boundary) || max_controls == 0 {
            return Ok(out);
        }
        let root = boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        if !std::ptr::eq(region.profile().profile(), profile) {
            return Ok(out);
        }
        let Some(pair) = self
            .pairs
            .iter()
            .find(|p| p.kind == ShellPairKind::Section(end))
        else {
            return Ok(out);
        };
        let owners = pair.uses.map(|a| &self.faces[a.face]);
        if owners[0].kind() != ShellFaceKind::Retained(end)
            || owners[1].kind() != ShellFaceKind::Transition
            || pair.reversed[0] == pair.reversed[1]
        {
            return Ok(out);
        }
        let address = pair.uses[0];
        let ShellUse::Source(contact) = &owners[0].loops[address.wire][address.edge] else {
            return Ok(out);
        };
        if !contact.same_definition(self.assembly.patch.retained[end].last().unwrap())
            || contact.surface() != profile.source()
        {
            return Ok(out);
        }
        let curve = contact.curve();
        let axis = root.axis;
        let parameter = root.parameters[end];
        let free = profile.section(axis, parameter)?.range();
        if curve.degree != 1
            || curve.control_points.len() != 2
            || curve.knots.len() != 4
            || curve.periodic
            || curve.validate().is_err()
            || curve.weights.iter().any(|w| *w <= 0.)
            || curve.knots[0] != curve.knots[1]
            || curve.knots[2] != curve.knots[3]
            || curve.knots[0] >= curve.knots[2]
        {
            return Ok(out);
        }
        for p in &curve.control_points {
            if out.controls == max_controls {
                return Ok(out);
            }
            out.controls += 1;
            if p.len() != 2 || p[axis] != parameter {
                return Ok(out);
            }
        }
        let a = curve.control_points[0][1 - axis];
        let b = curve.control_points[1][1 - axis];
        // The cutter extends beyond the chart; its owned crossing roots restrict
        // the fragment to the natural section endpoints checked below.
        if a.min(b) > free[0] || a.max(b) < free[1] || a == b {
            return Ok(out);
        }
        let descending = (b < a) ^ contact.reversed();
        for endpoint in 0..2 {
            let rail = if descending { 1 - endpoint } else { endpoint };
            let point = &boundary.rooted[rail].points[end];
            if !matches!(&contact.endpoints()[endpoint],Endpoint::Crossing {point:p,role:Role::Contact} if p.same_definition(point.root()))
            {
                return Ok(out);
            }
            let original = &point.edge().uses()[point.source_side()];
            for p in &original.curve().control_points {
                if out.controls == max_controls {
                    return Ok(out);
                }
                out.controls += 1;
                if p.len() != 2 || p[1 - axis] != free[rail] {
                    return Ok(out);
                }
            }
        }
        let retained = self.support_domain(address.face)?;
        let strip = region.source_domain();
        if strip[axis] != root.parameters
            || retained[1 - axis] != free
            || retained[axis][1 - end] != parameter
            || (end == 0 && retained[axis][0] >= parameter)
            || (end == 1 && retained[axis][1] <= parameter)
        {
            return Ok(out);
        }
        out.certificate = Some(TransitionSectionContact {
            owners,
            section: contact,
            end,
            axis,
            parameter,
        });
        out.reason = "profile-transition-section-contact-qualified";
        Ok(out)
    }
}
