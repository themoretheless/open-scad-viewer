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
    fn equivalent_original_pair(&self, faces: [usize; 2]) -> Option<[usize; 2]> {
        let owners = faces.map(|i| &self.faces[i]);
        let [ShellFaceKind::Original(a), ShellFaceKind::Original(b)] = owners.map(|f| f.kind)
        else {
            return None;
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let shell = root.restricted.proposal.body.geometry().shell();
        for (face, original) in owners.iter().zip([a, b]) {
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
        }
        Some([a, b])
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
        if faces[0] == faces[1]
            || faces.iter().any(|i| *i >= self.faces.len())
            || max_work > 100000000
            || max_spans > 100000
            || max_driver_cells > 100000
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
                0,
            )?;
            out.exact_work += report.exact_work;
            out.spans += report.spans;
            out.driver_cells += report.driver_cells;
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
