//! Fresh boundary/pole plane contacts on actual retained source contours.
use super::*;
use crate::source_fiber_boundary::Locus;

pub struct ProfileBoundaryContact<'x, 'b> {
    owners: [&'x ShellFace; 2],
    source: crate::source_pole_planar_contact::Certificate,
    image: crate::source_profile_boundary_identity::PlaneImage<'x, 'b>,
    shared: Vec<[Address; 2]>,
}
impl ProfileBoundaryContact<'_, '_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_pole_planar_contact::Certificate {
        &self.source
    }
    pub fn image(&self) -> &crate::source_profile_boundary_identity::PlaneImage<'_, '_> {
        &self.image
    }
    pub fn shared(&self) -> &[[Address; 2]] {
        &self.shared
    }
}
pub struct ProfileBoundaryReport<'x, 'b> {
    pub certificate: Option<ProfileBoundaryContact<'x, 'b>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
// Source contours use traversal-ordered ends. A possible fiber endpoint must
// touch an entire owned fiber fragment on the same closed contour. Mere shared
// vertex counts or nearby Cartesian positions are insufficient.
fn boundary_owned(loci: &[Locus], owned: &[bool]) -> bool {
    if loci.is_empty() || loci.len() != owned.len() {
        return false;
    }
    let n = loci.len();
    loci.iter().enumerate().all(|(i, locus)| match locus {
        Locus::EntireFragment => owned[i],
        Locus::Away => true,
        Locus::Endpoints(ends) => {
            (!ends[0] || (loci[(i + n - 1) % n] == Locus::EntireFragment && owned[(i + n - 1) % n]))
                && (!ends[1] || (loci[(i + 1) % n] == Locus::EntireFragment && owned[(i + 1) % n]))
        }
        Locus::Unresolved => false,
    })
}
impl<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b> {
    pub fn qualify_profile_boundary_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_driver: usize,
    ) -> Result<ProfileBoundaryReport<'_, 'b>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
            || max_driver > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound distinct profile boundary owners and native work",
            ));
        }
        let mut out = ProfileBoundaryReport {
            certificate: None,
            exact_work: 0,
            driver_cells: 0,
            reason: "profile-boundary-face-kind-unproven",
        };
        let (original, side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Retained(_), ShellFaceKind::Original(a)] => (a, 0),
            [ShellFaceKind::Original(a), ShellFaceKind::Retained(_)] => (a, 1),
            _ => return Ok(out),
        };
        if self.equivalent_original_face(faces[1 - side]) != Some(original) {
            out.reason = "profile-boundary-original-contour-unproven";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let shared = self
            .pairs
            .iter()
            .filter(|p| p.uses.iter().all(|a| faces.contains(&a.face)))
            .map(|p| p.uses)
            .collect::<Vec<_>>();
        if shared.is_empty() {
            out.reason = "profile-boundary-current-edge-ownership-unproven";
            return Ok(out);
        }
        if max_work == 0 || max_driver == 0 {
            out.reason = "profile-boundary-work-limit";
            return Ok(out);
        }
        let report = crate::source_pole_planar_contact::certify(
            shell,
            [root.face, original],
            max_work,
            max_driver,
        )?;
        out.exact_work = report.exact_work;
        out.driver_cells = report.driver_cells;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        if source.fiber().surface() != profile.source() {
            out.reason = "profile-boundary-selected-plane-image-unproven";
            return Ok(out);
        }
        if out.exact_work == max_work {
            out.reason = "profile-boundary-work-limit";
            return Ok(out);
        }
        let (axis, upper) = source.fiber().boundary();
        let report = crate::source_profile_boundary_identity::certify_plane_image(
            profile,
            source.fiber().plane(),
            axis,
            upper,
            max_work - out.exact_work,
        )?;
        out.exact_work += report.exact_work;
        out.reason = report.reason;
        let Some(image) = report.certificate else {
            return Ok(out);
        };
        let retained = &self.faces[faces[side]];
        // Each natural boundary fragment must be one of the actual shared uses
        // of this expression shell. Assembly has requalified every truncated
        // Outer pair with its native root-ended canonical source certificate.
        for (wire, loop_) in retained.loops.iter().enumerate() {
            let mut loci = Vec::new();
            let mut owned = Vec::new();
            for (edge, use_) in loop_.iter().enumerate() {
                let ShellUse::Source(fragment) = use_ else {
                    out.reason = "profile-boundary-source-contour-unproven";
                    return Ok(out);
                };
                if out.driver_cells == max_driver {
                    out.reason = "profile-boundary-driver-limit";
                    return Ok(out);
                }
                let report = crate::source_fiber_boundary::inspect_natural(
                    fragment,
                    profile.source(),
                    (axis, upper),
                    max_driver - out.driver_cells,
                )?;
                out.driver_cells += report.driver_cells;
                loci.push(report.locus);
                owned.push(shared.iter().flatten().any(|a| {
                    *a == Address {
                        face: faces[side],
                        wire,
                        edge,
                    }
                }));
            }
            if !boundary_owned(&loci, &owned) {
                out.reason = "profile-boundary-retained-fiber-ownership-unproven";
                return Ok(out);
            }
        }
        // Any included collapsed pole must be an actual natural corner of a
        // current shared source fragment. Excluded poles are checked against
        // the entire conservative retained support rectangle.
        let domain = self.support_domain(faces[side])?;
        let s = profile.source();
        let transverse = 1 - axis;
        let knots = [&s.knots_u, &s.knots_v];
        let degrees = [s.degree_u, s.degree_v];
        let counts = [s.control_points.len(), s.control_points[0].len()];
        for &(high, point) in image.image().poles() {
            let t = knots[transverse][if high {
                counts[transverse]
            } else {
                degrees[transverse]
            }];
            if t < domain[transverse][0] || t > domain[transverse][1] {
                continue;
            }
            let owns = shared
                .iter()
                .flatten()
                .filter(|a| a.face == faces[side])
                .any(|a| match &retained.loops[a.wire][a.edge] {
                    ShellUse::Source(f) => (0..2)
                        .any(|end| crate::source_vertex_contact::corner(f, end) == Some(point)),
                    _ => false,
                });
            if !owns {
                out.reason = "profile-boundary-retained-pole-ownership-unproven";
                return Ok(out);
            }
        }
        out.certificate = Some(ProfileBoundaryContact {
            owners: faces.map(|i| &self.faces[i]),
            source,
            image,
            shared,
        });
        out.reason = "profile-owned-boundary-pole-contact-qualified";
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn endpoint_contacts_require_adjacent_owned_entire_fiber() {
        let good = [
            Locus::EntireFragment,
            Locus::Endpoints([true, false]),
            Locus::Away,
            Locus::Endpoints([false, true]),
        ];
        assert!(boundary_owned(&good, &[true, false, false, false]));
        assert!(!boundary_owned(&good, &[false, false, false, false]));
        assert!(!boundary_owned(
            &[Locus::Endpoints([true, true]), Locus::Away],
            &[true, true]
        ));
        assert!(!boundary_owned(&[Locus::Unresolved], &[true]));
        assert!(!boundary_owned(&[], &[]));
    }
}
