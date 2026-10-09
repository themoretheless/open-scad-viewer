//! Strict XY hull separation composed with the owned transition image proof.
use super::*;
pub struct TransitionXyHullContact<'x> {
    owners: [&'x ShellFace; 2],
    source: crate::source_hull_separation::Certificate,
}
impl TransitionXyHullContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_hull_separation::Certificate {
        &self.source
    }
}
pub struct TransitionXyHullReport<'x> {
    pub certificate: Option<TransitionXyHullContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_transition_xy_hull_contact(
        &self,
        faces: [usize; 2],
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        max_work: u64,
    ) -> Result<TransitionXyHullReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned transition XY hull work",
            ));
        }
        let mut out = TransitionXyHullReport {
            certificate: None,
            exact_work: 0,
            reason: "transition-xy-image-owner-unproven",
        };
        if !std::ptr::eq(region.face(), self.assembly.patch.rails.face) {
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        if !std::ptr::eq(region.profile().profile(), profile) {
            return Ok(out);
        }
        let ids = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Original(a), ShellFaceKind::Transition] => [a, root.face],
            [ShellFaceKind::Transition, ShellFaceKind::Original(b)] => [root.face, b],
            _ => {
                out.reason = "transition-xy-hull-face-kind-unproven";
                return Ok(out);
            }
        };
        let (row, column, pivot) = profile.factors();
        if !pivot.is_finite()
            || pivot <= 0.
            || row.iter().chain(column).any(|x| !x.is_finite() || *x <= 0.)
        {
            out.reason = "transition-xy-profile-weights-unproven";
            return Ok(out);
        }
        let shell = root.restricted.proposal.body.geometry().shell();
        let regions = shell.regions().unwrap();
        if profile.source() != regions[root.face].loops()[0][0].surface() {
            out.reason = "transition-xy-profile-controls-unproven";
            return Ok(out);
        }
        for side in 0..2 {
            if matches!(self.faces[faces[side]].kind(),ShellFaceKind::Original(_))
                && self.faces[faces[side]].loops.iter().flatten().any(|u|!matches!(u,ShellUse::Source(f) if f.surface()==regions[ids[side]].loops()[0][0].surface())) {
                out.reason="transition-xy-original-controls-unproven";return Ok(out);
            }
        }
        if max_work == 0 {
            out.reason = "transition-xy-hull-work-limit";
            return Ok(out);
        }
        // ProfileProjectedRegion covers the whole transition XY image by the
        // clipped positive profile. A fresh vertical raw-control separator
        // covers that image regardless of the transition's corrected Z values.
        let report = crate::source_hull_separation::certify_xy(ids.map(|i| &regions[i]), max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        if let Some(source) = report.certificate {
            out.certificate = Some(TransitionXyHullContact {
                owners: faces.map(|i| &self.faces[i]),
                source,
            });
            out.reason = "transition-profile-xy-hull-separation-qualified";
        }
        Ok(out)
    }
}
