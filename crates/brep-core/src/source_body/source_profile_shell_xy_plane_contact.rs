//! Fresh vertical plane separation of original material and exact transition XY image.
use super::*;
use cad_predicates::Sign;
fn separator(
    points: &[[f64; 3]],
    bounds: [[f64; 2]; 2],
    work: &mut u64,
    max_work: u64,
) -> Result<Option<[[f64; 3]; 3]>> {
    if points.is_empty()
        || points.iter().flatten().any(|v| !v.is_finite())
        || bounds
            .iter()
            .any(|b| !b[0].is_finite() || !b[1].is_finite() || b[0] > b[1])
        || *work == max_work
    {
        return Ok(None);
    }
    let corners = std::array::from_fn::<_, 4, _>(|i| [bounds[0][i & 1], bounds[1][i >> 1], 0.]);
    for slope in [
        0.125, 0.25, 0.5, 1., 2., 4., 8., -0.125, -0.25, -0.5, -1., -2., -4., -8.,
    ] {
        // Approximate projections propose an offset only. Every plane and
        // strict side is independently verified with exact dyadic predicates.
        let range = |ps: &[[f64; 3]]| {
            ps.iter().fold([f64::INFINITY, f64::NEG_INFINITY], |r, p| {
                let v = p[1] - slope * p[0];
                [r[0].min(v), r[1].max(v)]
            })
        };
        let a = range(points);
        let b = range(&corners);
        let offset = if a[1] < b[0] {
            a[1] / 2. + b[0] / 2.
        } else if b[1] < a[0] {
            b[1] / 2. + a[0] / 2.
        } else {
            0.
        };
        for c in [0., offset] {
            let plane = [[0., c, 0.], [1., c + slope, 0.], [0., c, 1.]];
            if plane.iter().flatten().any(|v| !v.is_finite()) {
                continue;
            }
            if !crate::source_allowed_contact::independent(
                plane[0], plane[1], plane[2], work, max_work,
            )? {
                if *work == max_work {
                    return Ok(None);
                }
                continue;
            }
            let mut sides = [None, None];
            let mut valid = true;
            for (side, ps) in [points, corners.as_slice()].into_iter().enumerate() {
                for &p in ps {
                    let Some(sign) = crate::source_allowed_contact::orient(
                        &[plane[0], plane[1], plane[2], p],
                        None,
                        work,
                        max_work,
                    )?
                    else {
                        return Ok(None);
                    };
                    if sign == Sign::Zero || sides[side].is_some_and(|s| s != sign) {
                        valid = false;
                        break;
                    }
                    sides[side] = Some(sign);
                }
                if !valid {
                    break;
                }
            }
            if valid && sides[0] != sides[1] {
                return Ok(Some(plane));
            }
        }
    }
    Ok(None)
}
pub struct TransitionXyPlaneContact<'x> {
    owners: [&'x ShellFace; 2],
    plane: [[f64; 3]; 3],
    bounds: [[f64; 2]; 2],
    domain: [[f64; 2]; 2],
    material: crate::source_planar_material_hull::Certificate,
}
impl TransitionXyPlaneContact<'_> {
    pub fn owners(&self) -> [&ShellFace; 2] {
        self.owners
    }
    pub fn plane(&self) -> [[f64; 3]; 3] {
        self.plane
    }
    pub fn bounds(&self) -> [[f64; 2]; 2] {
        self.bounds
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn material(&self) -> &crate::source_planar_material_hull::Certificate {
        &self.material
    }
}
pub struct TransitionXyPlaneReport<'x> {
    pub certificate: Option<TransitionXyPlaneContact<'x>>,
    pub exact_work: u64,
    pub spans: usize,
    pub reason: &'static str,
}
impl ExpressionShell<'_, '_, '_, '_, '_, '_, '_, '_> {
    pub fn qualify_transition_xy_material_plane(
        &self,
        faces: [usize; 2],
        region: &ProfileProjectedRegion<'_, '_, '_, '_, '_>,
        max_spans: usize,
        max_work: u64,
    ) -> Result<TransitionXyPlaneReport<'_>> {
        if faces[0] == faces[1]
            || faces.iter().any(|&i| i >= self.faces.len())
            || max_spans > 100000
            || max_work > 100000000
        {
            return Err(Error::new(
                "BREP_PROFILE_CONTACTS",
                "Bound transition XY plane owners and work",
            ));
        }
        let mut out = TransitionXyPlaneReport {
            certificate: None,
            exact_work: 0,
            spans: 0,
            reason: "transition-xy-plane-owner-unproven",
        };
        let boundary = self.assembly.patch.rails.face.boundary;
        let root = boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        if !std::ptr::eq(region.face().boundary(), boundary)
            || !std::ptr::eq(region.profile().profile(), profile)
            || region.profile().axes() != [0, 1]
        {
            return Ok(out);
        }
        let (original, side) = match faces.map(|i| self.faces[i].kind()) {
            [ShellFaceKind::Original(a), ShellFaceKind::Transition] => (a, 0),
            [ShellFaceKind::Transition, ShellFaceKind::Original(a)] => (a, 1),
            _ => {
                out.reason = "transition-xy-plane-face-kind-unproven";
                return Ok(out);
            }
        };
        if self.equivalent_original_face(faces[side]) != Some(original) {
            out.reason = "transition-xy-plane-original-contour-unproven";
            return Ok(out);
        }
        if max_spans == 0 || max_work == 0 {
            out.reason = "transition-xy-plane-work-limit";
            return Ok(out);
        }
        let shell = root.restricted.proposal.body.geometry().shell();
        let (rows, columns, pivot) = profile.factors();
        if profile.source() != shell.regions().unwrap()[root.face].loops()[0][0].surface()
            || !pivot.is_finite()
            || pivot <= 0.
            || rows
                .iter()
                .chain(columns)
                .any(|v| !v.is_finite() || *v <= 0.)
        {
            out.reason = "transition-xy-plane-profile-controls-unproven";
            return Ok(out);
        }
        let domain = region.source_domain();
        let (bounds, spans) = profile.enclose(domain, max_spans)?;
        out.spans = spans;
        let Some(bounds) = bounds else {
            out.reason = "transition-xy-plane-profile-enclosure-unproven";
            return Ok(out);
        };
        let shell = root.restricted.proposal.body.geometry().shell();
        let report = crate::source_planar_material_hull::certify(shell, original, max_work)?;
        out.exact_work = report.exact_work;
        out.reason = report.reason;
        let Some(material) = report.certificate else {
            return Ok(out);
        };
        let bounds = [bounds[0], bounds[1]];
        let Some(plane) = separator(material.points(), bounds, &mut out.exact_work, max_work)?
        else {
            out.reason = "transition-xy-plane-strict-separation-unproven";
            return Ok(out);
        };
        // The owned transition and clipped profile have the same complete XY
        // image. An affine plane has strict signs throughout each convex hull
        // when every material control and every enclosure corner has that sign.
        out.certificate = Some(TransitionXyPlaneContact {
            owners: faces.map(|i| &self.faces[i]),
            plane,
            bounds,
            domain,
            material,
        });
        out.reason = "transition-xy-material-plane-separated";
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn diagonal_plane_proves_overlap_boxes_but_refuses_real_contact_and_spent_work() {
        let points = [[1., 4., 6.], [4., 8., 6.]];
        let bounds = [[3., 4.], [3., 4.5]];
        let mut work = 0;
        let plane = separator(&points, bounds, &mut work, 100000)
            .unwrap()
            .unwrap();
        assert!(work > 0 && work <= 100000);
        assert_eq!(plane[0][2], plane[1][2]);
        let mut work = 0;
        assert!(
            separator(&[[3., 3., 6.]], bounds, &mut work, 100000)
                .unwrap()
                .is_none()
        );
        let mut work = 0;
        assert!(separator(&points, bounds, &mut work, 0).unwrap().is_none());
        assert_eq!(work, 0);
        let mut work = 0;
        assert!(
            separator(&points, [[4., 3.], [3., 4.5]], &mut work, 100000)
                .unwrap()
                .is_none()
        );
        assert_eq!(work, 0);
    }
}
