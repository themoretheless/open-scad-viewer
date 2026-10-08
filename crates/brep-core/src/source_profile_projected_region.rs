//! Fresh original Jordan authority paired with an owned transition boundary.
use super::*;
use nurbs_core::interval_eval::Interval as I;

fn original_weights_match(
    weights: &[Vec<f64>],
    rows: &[f64],
    columns: &[f64],
    pivot: f64,
    used: &mut u64,
    max_work: u64,
) -> Result<bool> {
    for (i, row) in weights.iter().enumerate() {
        for (j, &weight) in row.iter().enumerate() {
            // Exact dyadic determinant: rows[i]*columns[j]-weight*pivot.
            // Comparing rounded products would incorrectly accept near matches.
            let points = [[0., 0., 0.], [rows[i], weight, 0.], [pivot, columns[j], 0.]];
            if crate::source_allowed_contact::orient(&points, Some([0, 1]), used, max_work)?
                != Some(cad_predicates::Sign::Zero)
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
/// Both maps have the same simple projected boundary and injective interiors.
/// Jordan separation therefore gives the same closed projected region. This
/// is a planar image certificate, not a closed 3D Body/contact certificate.
pub struct OriginalProjectedRegion<'f, 'a, 's, 'p, 'b> {
    face: &'f TransitionFace<'a, 's, 'p, 'b>,
    original: nurbs_core::surface_projected_jordan::Certificate,
    domain: [[f64; 2]; 2],
}
impl<'f, 'a, 's, 'p, 'b> OriginalProjectedRegion<'f, 'a, 's, 'p, 'b> {
    pub fn face(&self) -> &'f TransitionFace<'a, 's, 'p, 'b> {
        self.face
    }
    pub fn original(&self) -> &nurbs_core::surface_projected_jordan::Certificate {
        &self.original
    }
    pub fn source_domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
}
pub struct OriginalProjectedRegionReport<'f, 'a, 's, 'p, 'b> {
    pub certificate: Option<OriginalProjectedRegion<'f, 'a, 's, 'p, 'b>>,
    pub exact_work: u64,
    pub boundary_cells: usize,
    pub reason: &'static str,
}
impl<'a, 's, 'p, 'b> TransitionFace<'a, 's, 'p, 'b> {
    pub fn qualify_original_xy_region<'f>(
        &'f self,
        tolerance: f64,
        max_work: u64,
        max_cells: usize,
    ) -> Result<OriginalProjectedRegionReport<'f, 'a, 's, 'p, 'b>> {
        if !tolerance.is_finite() || tolerance <= 0. || max_work > 100000000 || max_cells > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_REGION",
                "Bound original projected-region work",
            ));
        }
        let mut out = OriginalProjectedRegionReport {
            certificate: None,
            exact_work: 0,
            boundary_cells: 0,
            reason: "profile-original-xy-region-unproven",
        };
        if max_work == 0 || max_cells == 0 {
            return Ok(out);
        }
        // Admission of this face used a fixed invertible projection of XY.
        if self.projection.iter().any(|p| p[2] != 0.) {
            return Ok(out);
        }
        let det = I::point(self.projection[0][0])
            .mul(I::point(self.projection[1][1]))?
            .sub(I::point(self.projection[0][1]).mul(I::point(self.projection[1][0]))?)?;
        if det.lo <= 0. && det.hi >= 0. {
            return Ok(out);
        }
        let root = self.boundary.rooted[0];
        let profile = root.restricted.proposal.profile(root.face).unwrap();
        let source = profile.source();
        let (rows, columns, pivot) = profile.factors();
        if !original_weights_match(
            &source.weights,
            rows,
            columns,
            pivot,
            &mut out.exact_work,
            max_work,
        )? {
            out.reason = "profile-original-weight-identity-unproven";
            return Ok(out);
        }
        if out.exact_work == max_work {
            return Ok(out);
        }
        let ranges = [
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ],
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ],
        ];
        // The clipped section axis avoids collapsed natural section edges.
        if root.parameters[0] <= ranges[root.axis][0] || root.parameters[1] >= ranges[root.axis][1]
        {
            return Ok(out);
        }
        let report = nurbs_core::surface_projected_jordan::certify(
            source,
            [0, 1],
            tolerance,
            max_work - out.exact_work,
            max_cells,
        )?;
        out.exact_work += report.exact_work;
        out.boundary_cells = report.boundary_cells;
        out.reason = report.reason;
        let Some(original) = report.certificate else {
            return Ok(out);
        };
        // A collapsed boundary in the uncut free direction would prevent a
        // simple subrectangle boundary. Retain refusal in that case.
        if original
            .collapsed_boundaries()
            .iter()
            .any(|&i| [1, 0, 1, 0][i] == 1 - root.axis)
        {
            return Ok(out);
        }
        let mut domain = ranges;
        domain[root.axis] = root.parameters;
        // The owned recipe specializes identically to source sections at
        // across=0/1 and source rails at free-low/high. No samples or caller
        // diagnostics establish this identity.
        out.certificate = Some(OriginalProjectedRegion {
            face: self,
            original,
            domain,
        });
        out.reason = "profile-original-xy-region-qualified";
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projected_region_requires_exact_original_weights_and_complete_work() {
        let mut used = 0;
        assert!(
            original_weights_match(
                &[vec![1., 0.5], vec![2., 1.]],
                &[1., 2.],
                &[1., 0.5],
                1.,
                &mut used,
                100000
            )
            .unwrap()
        );
        assert!(used > 0);
        let mut used = 0;
        assert!(
            !original_weights_match(
                &[vec![1., 1.1], vec![1.1, 1.21]],
                &[1., 1.1],
                &[1., 1.1],
                1.,
                &mut used,
                100000
            )
            .unwrap()
        );
        let mut used = 0;
        assert!(!original_weights_match(&[vec![1.]], &[1.], &[1.], 1., &mut used, 0).unwrap());
    }
}
