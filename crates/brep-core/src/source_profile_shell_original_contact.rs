//! Fresh source contact authority for geometry-preserving expression refinements.
use super::*;
pub struct OriginalPlanarContact<'x> {
    faces: [usize; 2],
    owners: [&'x ShellFace; 2],
    source: crate::source_allowed_contact::Certificate,
}
impl OriginalPlanarContact<'_> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_allowed_contact::Certificate {
        &self.source
    }
}
pub struct OriginalContactReport<'x> {
    pub certificate: Option<OriginalPlanarContact<'x>>,
    pub exact_work: u64,
    pub spans: usize,
    pub reason: &'static str,
}
fn full_partition(
    source: &crate::source_boundary_fragment::Fragment,
    parts: &[crate::source_boundary_fragment::Fragment],
) -> bool {
    if parts.len() != 3
        || parts.iter().any(|p| {
            p.surface() != source.surface()
                || p.curve() != source.curve()
                || p.reversed() != source.reversed()
        })
    {
        return false;
    }
    let definition = source.definition();
    parts[0].definition()["endpoints"][0] == definition["endpoints"][0]
        && parts[2].definition()["endpoints"][1] == definition["endpoints"][1]
        && parts[0].definition()["endpoints"][1] == parts[1].definition()["endpoints"][0]
        && parts[1].definition()["endpoints"][1] == parts[2].definition()["endpoints"][0]
        && parts[0].joins(&parts[1])
        && parts[1].joins(&parts[2])
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub(super) fn equivalent_original_face(&self, index: usize) -> Option<usize> {
        let face = self.faces.get(index)?;
        let ShellFaceKind::Original(original) = face.kind else {
            return None;
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let expected = shell.regions().unwrap()[original].loops();
        if face.loops.len() != expected.len() {
            return None;
        }
        for (wire_index, (wire, source)) in face.loops.iter().zip(expected).enumerate() {
            let mut cursor = 0;
            for (edge, fragment) in source.iter().enumerate() {
                let mut replacement = None;
                for rail in 0..2 {
                    let seam = self.assembly.patch.rails.face.boundary.rooted[rail];
                    let neighbor = 1 - seam.points[0].source_side();
                    let address = shell.uses()[seam.restricted.edge][neighbor];
                    if address.face == original
                        && address.wire == wire_index
                        && address.edge == edge
                    {
                        if replacement.is_some() {
                            return None;
                        }
                        let contour = &self.assembly.patch.rails.rails[rail].neighbor_contour;
                        let [w, middle] = contour.middle;
                        let parts = &contour.loops[w][middle - 1..=middle + 1];
                        if !full_partition(fragment, parts) {
                            return None;
                        }
                        replacement = Some(parts);
                    }
                }
                let fragments = replacement.unwrap_or(std::slice::from_ref(fragment));
                for expected_fragment in fragments {
                    if !matches!(wire.get(cursor),Some(ShellUse::Source(actual)) if actual.definition()==expected_fragment.definition())
                    {
                        return None;
                    }
                    cursor += 1;
                }
            }
            if cursor != wire.len() {
                return None;
            }
        }
        Some(original)
    }
    fn equivalent_original_pair(&self, faces: [usize; 2]) -> Option<[usize; 2]> {
        Some([
            self.equivalent_original_face(faces[0])?,
            self.equivalent_original_face(faces[1])?,
        ])
    }
    /// Fresh proof transfers only over identical original geometry and complete
    /// contours, allowing owned full-arc subdivisions. Retained faces refuse.
    pub fn qualify_original_planar_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_spans: usize,
    ) -> Result<OriginalContactReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_work > 100000000
            || max_spans > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Require distinct owned faces and bounded contact work",
            ));
        }
        let mut out = OriginalContactReport {
            certificate: None,
            exact_work: 0,
            spans: 0,
            reason: "expression-original-contact-face-changed",
        };
        let owners = faces.map(|i| &self.faces[i]);
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        if max_work == 0 || max_spans == 0 {
            out.reason = "expression-original-contact-work-limit";
            return Ok(out);
        }
        let report = crate::source_allowed_contact::certify(shell, original, max_work, max_spans)?;
        out.exact_work = report.exact_work;
        out.spans = report.spans;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        out.certificate = Some(OriginalPlanarContact {
            faces,
            owners,
            source,
        });
        out.reason = "expression-unchanged-planar-contact-qualified";
        Ok(out)
    }
}

pub enum OriginalFiberProof {
    Boundary(crate::source_fiber_contact::Certificate),
    Paired(crate::source_paired_fiber_contact::Certificate),
}
pub struct OriginalFiberContact<'x> {
    faces: [usize; 2],
    owners: [&'x ShellFace; 2],
    proof: OriginalFiberProof,
}
impl OriginalFiberContact<'_> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn proof(&self) -> &OriginalFiberProof {
        &self.proof
    }
}
pub struct OriginalFiberReport<'x> {
    pub certificate: Option<OriginalFiberContact<'x>>,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh complete natural-boundary fiber qualification in both orientations,
    /// followed by paired curved fibers. Only complete owned subdivisions may inherit.
    pub fn qualify_original_fiber_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_spans: usize,
        max_driver_cells: usize,
    ) -> Result<OriginalFiberReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_work > 100000000
            || max_spans > 100000
            || max_driver_cells > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound original fiber pair and work",
            ));
        }
        let mut out = OriginalFiberReport {
            certificate: None,
            exact_work: 0,
            spans: 0,
            driver_cells: 0,
            reason: "expression-original-contact-face-changed",
        };
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        for order in [original, [original[1], original[0]]] {
            if out.exact_work == max_work
                || out.spans == max_spans
                || out.driver_cells == max_driver_cells
            {
                out.reason = "expression-original-fiber-work-limit";
                return Ok(out);
            }
            let report = crate::source_fiber_contact::certify(
                shell,
                order,
                max_work - out.exact_work,
                max_spans - out.spans,
                max_driver_cells - out.driver_cells,
            )?;
            out.exact_work += report.exact_work;
            out.spans += report.spans;
            out.driver_cells += report.driver_cells;
            out.reason = report.reason;
            if let Some(proof) = report.certificate {
                out.certificate = Some(OriginalFiberContact {
                    faces,
                    owners: faces.map(|i| &self.faces[i]),
                    proof: OriginalFiberProof::Boundary(proof),
                });
                out.reason = "expression-unchanged-boundary-fiber-qualified";
                return Ok(out);
            }
        }
        if out.exact_work == max_work || out.driver_cells == max_driver_cells {
            out.reason = "expression-original-fiber-work-limit";
            return Ok(out);
        }
        let report = crate::source_paired_fiber_contact::certify(
            shell,
            original,
            max_work - out.exact_work,
            max_driver_cells - out.driver_cells,
        )?;
        out.exact_work += report.exact_work;
        out.driver_cells += report.driver_cells;
        out.reason = report.reason;
        if let Some(proof) = report.certificate {
            out.certificate = Some(OriginalFiberContact {
                faces,
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalFiberProof::Paired(proof),
            });
            out.reason = "expression-unchanged-paired-fiber-qualified";
        }
        Ok(out)
    }
}

pub enum OriginalAdditionalProof {
    Vertex(crate::source_vertex_contact::Certificate),
    Interior(crate::source_interior_contact::Certificate),
}
pub struct OriginalAdditionalContact<'x> {
    faces: [usize; 2],
    owners: [&'x ShellFace; 2],
    proof: OriginalAdditionalProof,
}
impl OriginalAdditionalContact<'_> {
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn proof(&self) -> &OriginalAdditionalProof {
        &self.proof
    }
}
pub struct OriginalAdditionalReport<'x> {
    pub certificate: Option<OriginalAdditionalContact<'x>>,
    pub exact_work: u64,
    pub spans: usize,
    pub driver_cells: usize,
    pub linear_cells: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh owned-vertex and supported interior-fiber proofs. As with boundary
    /// transfer, final contours must be complete owned source refinements.
    pub fn qualify_original_additional_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_spans: usize,
        max_driver_cells: usize,
    ) -> Result<OriginalAdditionalReport<'_>> {
        self.qualify_original_additional_contact_with_linear_chart(
            faces,
            max_work,
            max_spans,
            max_driver_cells,
            0,
        )
    }
    /// Optional fresh injectivity fallback, with its own shared cell budget
    /// across both contact orders. Zero cells preserves the original protocol.
    pub fn qualify_original_additional_contact_with_linear_chart(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_spans: usize,
        max_driver_cells: usize,
        max_linear_cells: usize,
    ) -> Result<OriginalAdditionalReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_work > 100000000
            || max_spans > 100000
            || max_driver_cells > 100000
            || max_linear_cells > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound original vertex/interior pair work",
            ));
        }
        let mut out = OriginalAdditionalReport {
            certificate: None,
            exact_work: 0,
            spans: 0,
            driver_cells: 0,
            linear_cells: 0,
            reason: "expression-original-contact-face-changed",
        };
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        if max_work == 0 {
            out.reason = "expression-original-contact-work-limit";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let report = crate::source_vertex_contact::certify(shell, original, max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        if let Some(proof) = report.certificate {
            out.certificate = Some(OriginalAdditionalContact {
                faces,
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalAdditionalProof::Vertex(proof),
            });
            out.reason = "expression-unchanged-vertex-contact-qualified";
            return Ok(out);
        }
        for order in [original, [original[1], original[0]]] {
            if out.exact_work == max_work
                || out.spans == max_spans
                || out.driver_cells == max_driver_cells
            {
                out.reason = "expression-original-contact-work-limit";
                return Ok(out);
            }
            let report = crate::source_interior_contact::certify_with_linear_chart(
                shell,
                order,
                max_work - out.exact_work,
                max_spans - out.spans,
                max_driver_cells - out.driver_cells,
                max_linear_cells - out.linear_cells,
            )?;
            out.exact_work += report.exact_work;
            out.spans += report.spans;
            out.driver_cells += report.driver_cells;
            out.linear_cells += report.linear_cells;
            out.reason = report.reason;
            if let Some(proof) = report.certificate {
                out.certificate = Some(OriginalAdditionalContact {
                    faces,
                    owners: faces.map(|i| &self.faces[i]),
                    proof: OriginalAdditionalProof::Interior(proof),
                });
                out.reason = "expression-unchanged-interior-contact-qualified";
                return Ok(out);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod refinement_tests {
    use super::full_partition;
    use crate::source_boundary_fragment::{Endpoint, Fragment};
    use nurbs_core::{curve::Curve, surface::Surface};
    #[test]
    fn complete_source_partition_refuses_gaps_reversal_and_changed_curve() {
        let surface = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let curve = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        let fragment = |a, b| {
            Fragment::new(
                &surface,
                &curve,
                Endpoint::Parameter(a),
                Endpoint::Parameter(b),
            )
            .unwrap()
        };
        let full = fragment(0., 1.);
        let good = [fragment(0., 0.25), fragment(0.25, 0.75), fragment(0.75, 1.)];
        assert!(full_partition(&full, &good));
        assert!(full_partition(
            &fragment(1., 0.),
            &[fragment(1., 0.75), fragment(0.75, 0.25), fragment(0.25, 0.)]
        ));
        assert!(!full_partition(
            &full,
            &[fragment(0., 0.25), fragment(0.5, 0.75), fragment(0.75, 1.)]
        ));
        assert!(!full_partition(
            &full,
            &[fragment(0., 0.25), fragment(0.75, 0.25), fragment(0.75, 1.)]
        ));
        assert!(!full_partition(&full, &good[..2]));
        let other = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.9]]).unwrap();
        let changed = Fragment::new(
            &surface,
            &other,
            Endpoint::Parameter(0.25),
            Endpoint::Parameter(0.75),
        )
        .unwrap();
        assert!(!full_partition(
            &full,
            &[good[0].clone(), changed, good[2].clone()]
        ));
    }
}

/// Fresh strict hull separation transferred only over complete unchanged regions.
pub struct OriginalHullContact<'x> {
    owners: [&'x ShellFace; 2],
    source: crate::source_hull_separation::Certificate,
}
impl OriginalHullContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_hull_separation::Certificate {
        &self.source
    }
}
pub struct OriginalHullReport<'x> {
    pub certificate: Option<OriginalHullContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_original_hull_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
    ) -> Result<OriginalHullReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound distinct owned hull pair work",
            ));
        }
        let mut out = OriginalHullReport {
            certificate: None,
            exact_work: 0,
            reason: "expression-original-contact-face-changed",
        };
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        if max_work == 0 {
            out.reason = "expression-original-hull-work-limit";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let report = crate::source_hull_separation::certify_shell(shell, original, max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        if let Some(source) = report.certificate {
            out.certificate = Some(OriginalHullContact {
                owners: faces.map(|i| &self.faces[i]),
                source,
            });
            out.reason = "expression-unchanged-hull-separation-qualified";
        }
        Ok(out)
    }
}

pub enum OriginalPoleProof {
    Paired(crate::source_pole_paired_contact::Certificate),
    Planar(crate::source_pole_planar_contact::Certificate),
}
pub struct OriginalPoleContact<'x> {
    owners: [&'x ShellFace; 2],
    proof: OriginalPoleProof,
}
impl OriginalPoleContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn proof(&self) -> &OriginalPoleProof {
        &self.proof
    }
}
pub struct OriginalPoleReport<'x> {
    pub certificate: Option<OriginalPoleContact<'x>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    /// Fresh pole/fiber contact authority over identical complete original regions.
    pub fn qualify_original_pole_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
        max_driver: usize,
    ) -> Result<OriginalPoleReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
            || max_driver > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound distinct original pole pair work",
            ));
        }
        let mut out = OriginalPoleReport {
            certificate: None,
            exact_work: 0,
            driver_cells: 0,
            reason: "expression-original-contact-face-changed",
        };
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        if max_work == 0 || max_driver == 0 {
            out.reason = "expression-original-pole-work-limit";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let paired =
            crate::source_pole_paired_contact::certify(shell, original, max_work, max_driver)?;
        out.exact_work += paired.exact_work;
        out.driver_cells += paired.driver_cells;
        out.reason = paired.reason;
        if let Some(c) = paired.certificate {
            out.certificate = Some(OriginalPoleContact {
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalPoleProof::Paired(c),
            });
            out.reason = "expression-original-paired-pole-contact-qualified";
            return Ok(out);
        }
        if out.exact_work == max_work || out.driver_cells == max_driver {
            return Ok(out);
        }
        let planar = crate::source_pole_planar_contact::certify(
            shell,
            original,
            max_work - out.exact_work,
            max_driver - out.driver_cells,
        )?;
        out.exact_work += planar.exact_work;
        out.driver_cells += planar.driver_cells;
        out.reason = planar.reason;
        if let Some(c) = planar.certificate {
            out.certificate = Some(OriginalPoleContact {
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalPoleProof::Planar(c),
            });
            out.reason = "expression-original-planar-pole-contact-qualified";
        }
        Ok(out)
    }
}

/// Strict separation of full control hulls also covers every clipped profile
/// with the same controls and exact positive factorized weights.
pub struct ProfileHullContact<'x> {
    owners: [&'x ShellFace; 2],
    source: crate::source_hull_separation::Certificate,
}
impl ProfileHullContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_hull_separation::Certificate {
        &self.source
    }
}
pub struct ProfileHullReport<'x> {
    pub certificate: Option<ProfileHullContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_profile_hull_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
    ) -> Result<ProfileHullReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned profile hull pair work",
            ));
        }
        let mut out = ProfileHullReport {
            certificate: None,
            exact_work: 0,
            reason: "profile-hull-face-kind-unproven",
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let ids = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Original(a), ShellFaceKind::Retained(_)] => [a, root.face],
            [ShellFaceKind::Retained(_), ShellFaceKind::Original(b)] => [root.face, b],
            _ => return Ok(out),
        };
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let (row, column, pivot) = profile.factors();
        if !pivot.is_finite()
            || pivot <= 0.
            || row.iter().chain(column).any(|x| !x.is_finite() || *x <= 0.)
        {
            out.reason = "profile-hull-positive-weights-unproven";
            return Ok(out);
        }
        if max_work == 0 {
            out.reason = "profile-hull-work-limit";
            return Ok(out);
        }
        let shell = root.restricted.proposal.body.geometry().shell();
        let regions = shell.regions().unwrap();
        if profile.source()!=regions[root.face].loops()[0][0].surface()
            || faces.iter().zip(ids).any(|(&face,original)|self.faces[face].loops.iter().flatten().any(|u|!matches!(u,ShellUse::Source(f) if f.surface()==regions[original].loops()[0][0].surface()))) {
            out.reason="profile-hull-source-controls-unproven";return Ok(out);
        }
        // Raw full control hulls only: source material trims cannot bound the
        // modified profile image. Positive target factors require no rounding.
        let report = crate::source_hull_separation::certify(ids.map(|i| &regions[i]), max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        if let Some(source) = report.certificate {
            out.certificate = Some(ProfileHullContact {
                owners: faces.map(|i| &self.faces[i]),
                source,
            });
            out.reason = "profile-full-control-hull-separation-qualified";
        }
        Ok(out)
    }
}

pub enum OriginalBoundaryProof {
    Paired(crate::source_paired_fiber_contact::Certificate),
    Ruled(crate::source_ruled_projection_contact::Certificate),
}
pub struct OriginalBoundaryContact<'x> {
    owners: [&'x ShellFace; 2],
    proof: OriginalBoundaryProof,
}
impl OriginalBoundaryContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn proof(&self) -> &OriginalBoundaryProof {
        &self.proof
    }
}
pub struct OriginalBoundaryReport<'x> {
    pub certificate: Option<OriginalBoundaryContact<'x>>,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_original_boundary_contact(
        &self,
        faces: [usize; 2],
        tolerance: f64,
        max_work: u64,
        max_driver: usize,
    ) -> Result<OriginalBoundaryReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || !tolerance.is_finite()
            || tolerance <= 0.
            || max_work > 100000000
            || max_driver > 100000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound original boundary contact work",
            ));
        }
        let mut out = OriginalBoundaryReport {
            certificate: None,
            exact_work: 0,
            driver_cells: 0,
            reason: "expression-original-contact-face-changed",
        };
        let Some(original) = self.equivalent_original_pair(faces) else {
            return Ok(out);
        };
        if max_work == 0 || max_driver == 0 {
            out.reason = "expression-original-boundary-work-limit";
            return Ok(out);
        }
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        let paired =
            crate::source_paired_fiber_contact::certify(shell, original, max_work, max_driver)?;
        out.exact_work += paired.exact_work;
        out.driver_cells += paired.driver_cells;
        out.reason = paired.reason;
        if let Some(c) = paired.certificate {
            out.certificate = Some(OriginalBoundaryContact {
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalBoundaryProof::Paired(c),
            });
            out.reason = "expression-original-paired-boundary-qualified";
            return Ok(out);
        }
        if out.exact_work == max_work || out.driver_cells == max_driver {
            return Ok(out);
        }
        let ruled = crate::source_ruled_projection_contact::certify(
            shell,
            original,
            tolerance,
            max_work - out.exact_work,
            max_driver - out.driver_cells,
        )?;
        out.exact_work += ruled.exact_work;
        out.driver_cells += ruled.driver_cells;
        out.reason = ruled.reason;
        if let Some(c) = ruled.certificate {
            out.certificate = Some(OriginalBoundaryContact {
                owners: faces.map(|i| &self.faces[i]),
                proof: OriginalBoundaryProof::Ruled(c),
            });
            out.reason = "expression-original-ruled-boundary-qualified";
        }
        Ok(out)
    }
}
