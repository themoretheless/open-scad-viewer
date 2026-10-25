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
        // TransitionFace stores this immutable boundary plus a proof projection.
        // Different admitted projections retain the same transition expression.
        if !std::ptr::eq(
            region.face().boundary(),
            self.assembly.patch.rails.face.boundary(),
        ) {
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

/// Strict coordinate separation of the complete owned transition projection.
/// Bounds come from the exact clipped profile, not its full source hull.
pub struct TransitionXyBoundsContact<'x> {
    owners: [&'x ShellFace; 2],
    bounds: [[[f64; 2]; 2]; 2],
    axis: usize,
    material: Option<crate::source_planar_material_hull::Certificate>,
}
impl TransitionXyBoundsContact<'_> {
    pub fn material_hull(&self) -> Option<&crate::source_planar_material_hull::Certificate> {
        self.material.as_ref()
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn bounds(&self) -> [[[f64; 2]; 2]; 2] {
        self.bounds
    }
    pub fn axis(&self) -> usize {
        self.axis
    }
}
pub struct TransitionXyBoundsReport<'x> {
    pub exact_work: u64,
    pub certificate: Option<TransitionXyBoundsContact<'x>>,
    pub spans: usize,
    pub reason: &'static str,
}
fn separated_axis(bounds: [[[f64; 2]; 2]; 2]) -> Option<usize> {
    if bounds
        .iter()
        .flatten()
        .any(|b| !b[0].is_finite() || !b[1].is_finite() || b[0] > b[1])
    {
        return None;
    }
    (0..2).find(|&a| bounds[0][a][1] < bounds[1][a][0] || bounds[1][a][1] < bounds[0][a][0])
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_transition_xy_bounds_contact(
        &self,
        faces: [usize; 2],
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        max_spans: usize,
    ) -> Result<TransitionXyBoundsReport<'_>> {
        self.qualify_transition_xy_material_bounds_contact(faces, region, max_spans, 0)
    }
    /// Original material hull substitution requires complete unchanged contours.
    pub fn qualify_transition_xy_material_bounds_contact(
        &self,
        faces: [usize; 2],
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        max_spans: usize,
        max_work: u64,
    ) -> Result<TransitionXyBoundsReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_spans > 100000
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned transition XY enclosure work",
            ));
        }
        let mut out = TransitionXyBoundsReport {
            exact_work: 0,
            certificate: None,
            spans: 0,
            reason: "transition-xy-bounds-owner-unproven",
        };
        // TransitionFace stores this immutable boundary plus a proof projection.
        // Different admitted projections retain the same transition expression.
        if !std::ptr::eq(
            region.face().boundary(),
            self.assembly.patch.rails.face.boundary(),
        ) {
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        if !std::ptr::eq(region.profile().profile(), profile) {
            return Ok(out);
        }
        let (original, original_side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Original(a), ShellFaceKind::Transition] => (a, 0),
            [ShellFaceKind::Transition, ShellFaceKind::Original(a)] => (a, 1),
            _ => {
                out.reason = "transition-xy-bounds-face-kind-unproven";
                return Ok(out);
            }
        };
        let shell = root.restricted.proposal.body.geometry().shell();
        let regions = shell.regions().unwrap();
        let surface = regions[original].loops()[0][0].surface();
        if profile.source() != regions[root.face].loops()[0][0].surface()
            || self.faces[faces[original_side]]
                .loops
                .iter()
                .flatten()
                .any(|u| !matches!(u,ShellUse::Source(f) if f.surface()==surface))
        {
            out.reason = "transition-xy-bounds-controls-unproven";
            return Ok(out);
        }
        // An unchanged rational surface lies in its raw control hull only
        // with positive finite weights. Trimming can only reduce that image.
        if surface
            .weights
            .iter()
            .flatten()
            .any(|w| !w.is_finite() || *w <= 0.)
        {
            out.reason = "transition-xy-bounds-weights-unproven";
            return Ok(out);
        }
        if max_spans == 0 {
            out.reason = "transition-xy-bounds-span-limit";
            return Ok(out);
        }
        let (enclosure, spent) = profile.enclose(region.source_domain(), max_spans)?;
        out.spans = spent;
        let Some(enclosure) = enclosure else {
            out.reason = "transition-xy-bounds-span-limit";
            return Ok(out);
        };
        let mut original_bounds = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
        for p in surface.control_points.iter().flatten() {
            for a in 0..2 {
                if !p[a].is_finite() {
                    out.reason = "transition-xy-bounds-nonfinite-control";
                    return Ok(out);
                }
                original_bounds[a][0] = original_bounds[a][0].min(p[a]);
                original_bounds[a][1] = original_bounds[a][1].max(p[a]);
            }
        }
        let mut bounds = [[[0.; 2]; 2]; 2];
        bounds[original_side] = original_bounds;
        bounds[1 - original_side] = [enclosure[0], enclosure[1]];
        let mut material = None;
        let mut axis = separated_axis(bounds);
        if axis.is_none() && max_work > 0 {
            if self.equivalent_original_face(faces[original_side]) != Some(original) {
                out.reason = "transition-xy-material-contour-identity-unproven";
                return Ok(out);
            }
            let report = crate::source_planar_material_hull::certify(shell, original, max_work)?;
            out.exact_work = report.exact_work;
            let Some(hull) = report.certificate else {
                out.reason = report.reason;
                return Ok(out);
            };
            {
                let mut b = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
                for point in hull.points() {
                    for a in 0..2 {
                        b[a][0] = b[a][0].min(point[a]);
                        b[a][1] = b[a][1].max(point[a]);
                    }
                }
                bounds[original_side] = b;
                axis = separated_axis(bounds);
                material = Some(hull);
            }
        }
        let Some(axis) = axis else {
            out.reason = "transition-xy-bounds-overlap-unproven";
            return Ok(out);
        };
        out.certificate = Some(TransitionXyBoundsContact {
            owners: faces.map(|i| &self.faces[i]),
            bounds,
            axis,
            material,
        });
        out.reason = "transition-profile-clipped-xy-bounds-qualified";
        Ok(out)
    }
}
#[cfg(test)]
mod bounds_tests {
    use super::*;
    #[test]
    fn strict_xy_bounds_refuse_touch_overlap_and_invalid_intervals() {
        let a = [[0., 1.], [-10., 10.]];
        let b = [[2., 3.], [-10., 10.]];
        assert_eq!(separated_axis([a, b]), Some(0));
        assert_eq!(separated_axis([b, a]), Some(0));
        assert_eq!(separated_axis([a, [[1., 2.], [-10., 10.]]]), None);
        assert_eq!(separated_axis([a, a]), None);
        assert_eq!(separated_axis([a, [[f64::NAN, 3.], [-10., 10.]]]), None);
        assert_eq!(separated_axis([a, [[3., 2.], [-10., 10.]]]), None);
    }
}
