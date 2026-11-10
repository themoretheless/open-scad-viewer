//! Fresh single-point contact for an exact positive-weight retained profile.
use super::*;
pub struct ProfileVertexContact<'x> {
    owners: [&'x ShellFace; 2],
    source: crate::source_vertex_contact::Certificate,
    image: crate::source_corner_plane_image::Certificate,
    excluded_domain: Option<[[f64; 2]; 2]>,
}
impl ProfileVertexContact<'_> {
    pub fn excluded_domain(&self) -> Option<[[f64; 2]; 2]> {
        self.excluded_domain
    }
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn source(&self) -> &crate::source_vertex_contact::Certificate {
        &self.source
    }
    pub fn image(&self) -> &crate::source_corner_plane_image::Certificate {
        &self.image
    }
}
pub struct ProfileVertexReport<'x> {
    pub certificate: Option<ProfileVertexContact<'x>>,
    pub exact_work: u64,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_profile_vertex_contact(
        &self,
        faces: [usize; 2],
        max_work: u64,
    ) -> Result<ProfileVertexReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound owned profile vertex work",
            ));
        }
        let mut out = ProfileVertexReport {
            certificate: None,
            exact_work: 0,
            reason: "profile-vertex-face-kind-unproven",
        };
        let root = self.assembly.patch.rails.face.boundary.rooted[0];
        let (original, profile_side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Retained(_), ShellFaceKind::Original(a)] => (a, 0),
            [ShellFaceKind::Original(a), ShellFaceKind::Retained(_)] => (a, 1),
            _ => return Ok(out),
        };
        if self.equivalent_original_face(faces[1 - profile_side]) != Some(original) {
            out.reason = "profile-vertex-original-contour-unproven";
            return Ok(out);
        }
        let shell = root.restricted.proposal.body.geometry().shell();
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let regions = shell.regions().unwrap();
        if profile.source() != regions[root.face].loops()[0][0].surface() {
            out.reason = "profile-vertex-source-controls-unproven";
            return Ok(out);
        }
        let (rows, columns, pivot) = profile.factors();
        if !pivot.is_finite()
            || pivot <= 0.
            || rows
                .iter()
                .chain(columns)
                .any(|v| !v.is_finite() || *v <= 0.)
        {
            out.reason = "profile-vertex-positive-weights-unproven";
            return Ok(out);
        }
        if max_work == 0 {
            out.reason = "profile-vertex-work-limit";
            return Ok(out);
        }
        let mut ids = [root.face; 2];
        ids[1 - profile_side] = original;
        let report = crate::source_vertex_contact::certify(shell, ids, max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        let Some(source) = report.certificate else {
            return Ok(out);
        };
        // A material hull of the old selected region cannot bound a changed
        // profile. Only the complete raw support's corner plane is transferred.
        if source.material_hulls()[profile_side].is_some() {
            out.reason = "profile-vertex-old-material-hull-unproven";
            return Ok(out);
        }
        let address = source.uses()[profile_side].0;
        let original_fragment = &regions[root.face].loops()[address.wire][address.edge];
        let kept = &self.faces[faces[profile_side]];
        let owns = kept.loops.iter().flatten().any(|u| match u {
            ShellUse::Source(f)
                if f.surface() == profile.source() && f.curve() == original_fragment.curve() =>
            {
                (0..2)
                    .any(|end| crate::source_vertex_contact::corner(f, end) == Some(source.point()))
            }
            _ => false,
        });
        if out.exact_work == max_work {
            out.reason = "profile-vertex-work-limit";
            return Ok(out);
        }
        let report = crate::source_corner_plane_image::certify(
            profile.source(),
            source.plane(),
            source.point(),
            max_work - out.exact_work,
        )?;
        out.exact_work += report.exact_work;
        let Some(image) = report.certificate else {
            out.reason = "profile-vertex-single-point-image-unproven";
            return Ok(out);
        };
        let excluded_domain = if owns {
            None
        } else {
            let domain = self.support_domain(faces[profile_side])?;
            if corner_locus_absent(&image, domain, &mut out.exact_work, max_work)? != Some(true) {
                out.reason = "profile-vertex-retained-corner-unproven";
                return Ok(out);
            }
            Some(domain)
        };
        // This fresh single-Bezier proof uses only exact control-plane signs,
        // positive Bernstein bases and collapsed zero edges. Exact positive
        // factorized weights retain all three properties and the same corner.
        out.certificate = Some(ProfileVertexContact {
            owners: faces.map(|i| &self.faces[i]),
            source,
            image,
            excluded_domain,
        });
        out.reason = if excluded_domain.is_some() {
            "profile-single-point-locus-excluded"
        } else {
            "profile-owned-single-point-contact-qualified"
        };
        Ok(out)
    }
}

/// The admitted positive Bezier chart has no interior plane preimages.
/// Natural edge interiors vanish only if every edge coefficient vanishes.
fn corner_locus_absent(
    image: &crate::source_corner_plane_image::Certificate,
    domain: [[f64; 2]; 2],
    work: &mut u64,
    max_work: u64,
) -> Result<Option<bool>> {
    use cad_predicates::Sign;
    let s = image.surface();
    let p = s.degree_u;
    let q = s.degree_v;
    let natural = [
        [s.knots_u[p], s.knots_u[s.control_points.len()]],
        [s.knots_v[q], s.knots_v[s.control_points[0].len()]],
    ];
    if (0..2).any(|a| {
        domain[a].iter().any(|v| !v.is_finite())
            || domain[a][0] > domain[a][1]
            || domain[a][0] < natural[a][0]
            || domain[a][1] > natural[a][1]
    }) {
        return Ok(Some(false));
    }
    let plane = image.plane();
    let mut signs = vec![];
    for row in &s.control_points {
        let mut values = vec![];
        for c in row {
            let Some(sign) = crate::source_allowed_contact::orient(
                &[plane[0], plane[1], plane[2], [c[0], c[1], c[2]]],
                None,
                work,
                max_work,
            )?
            else {
                return Ok(None);
            };
            values.push(sign);
        }
        signs.push(values);
    }
    let contains = |a: usize, x: f64| domain[a][0] <= x && x <= domain[a][1];
    for (i, j, u, v) in [
        (0, 0, natural[0][0], natural[1][0]),
        (p, 0, natural[0][1], natural[1][0]),
        (p, q, natural[0][1], natural[1][1]),
        (0, q, natural[0][0], natural[1][1]),
    ] {
        if signs[i][j] == Sign::Zero && contains(0, u) && contains(1, v) {
            return Ok(Some(false));
        }
    }
    for (axis, end, edge) in [
        (0, 0, (0..=q).map(|j| (0, j)).collect::<Vec<_>>()),
        (0, 1, (0..=q).map(|j| (p, j)).collect()),
        (1, 0, (0..=p).map(|i| (i, 0)).collect()),
        (1, 1, (0..=p).map(|i| (i, q)).collect()),
    ] {
        if edge.iter().all(|&(i, j)| signs[i][j] == Sign::Zero)
            && contains(axis, natural[axis][end])
        {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}
#[cfg(test)]
mod exclusion_tests {
    use super::*;
    #[test]
    fn excluded_corner_checks_collapsed_edge_and_closed_trim_boundary() {
        let mut s = nurbs_core::surface::Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 1.]],
                vec![vec![1., 0., 1.], vec![1., 1., 1.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let plane = [[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]];
        let proof = crate::source_corner_plane_image::certify(&s, plane, [0., 0., 0.], 100000)
            .unwrap()
            .certificate
            .unwrap();
        let absent = |proof, domain| corner_locus_absent(proof, domain, &mut 0, 100000).unwrap();
        assert_eq!(absent(&proof, [[0.25, 1.], [0., 1.]]), Some(true));
        assert_eq!(absent(&proof, [[0., 0.75], [0., 1.]]), Some(false));
        assert_eq!(
            corner_locus_absent(&proof, [[0.25, 1.], [0., 1.]], &mut 0, 0).unwrap(),
            None
        );
        s.control_points[0][1] = vec![0., 0., 0.];
        let collapsed = crate::source_corner_plane_image::certify(&s, plane, [0., 0., 0.], 100000)
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(absent(&collapsed, [[0., 0.75], [0.25, 0.75]]), Some(false));
        assert_eq!(absent(&collapsed, [[0.25, 1.], [0.25, 0.75]]), Some(true));
    }
}
