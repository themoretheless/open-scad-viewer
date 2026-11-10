//! source profile body operations: split_source_use.
use super::*;
impl<'p, 'b> CutEndpoint<'p, 'b> {
/// Split by an original UV crossing expression, preserving its root on both
    /// directed fragments. No rounded world point or parameter becomes an end.
    pub fn split_source_use<'c>(
        &'c self,
        max_mapping_cells: usize,
    ) -> Result<CutPartitionReport<'c, 'p, 'b>> {
        use nurbs_core::{curve::Curve, interval_eval::Interval as I};
        let shell = self.proposal.body.geometry().shell();
        let edge = &shell.edges()[self.edge];
        let addresses = &shell.uses()[self.edge];
        let side = usize::from(addresses[0].face != self.face);
        let source = &edge.uses()[side];
        let other = &edge.uses()[1 - side];
        let profile = self.proposal.profile(self.face).unwrap();
        let section = profile.section(self.axis, self.parameter)?;
        let [lo, hi] = section.range();
        let width = hi - lo;
        let ends = [lo - width, hi + width];
        if !ends.iter().all(|x| x.is_finite()) || ends[0] >= lo || ends[1] <= hi {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound the original UV cut line",
            ));
        }
        let contact = Curve::from_polyline(
            ends.map(|free| {
                let mut p = vec![self.parameter; 2];
                p[1 - self.axis] = free;
                p
            })
            .to_vec(),
        )?;
        let t = crate::source_seam_tangency::inverse(
            edge,
            side,
            I::new(self.canonical[0], self.canonical[1])?,
        )?;
        let contact_t = I::point(section.range()[usize::from(self.high)])
            .sub(I::point(ends[0]))?
            .div(I::point(ends[1]).sub(I::point(ends[0]))?)?;
        let d = source.curve().domain();
        let margin = (d[1] - d[0]) * 0.05;
        let selector = [
            [(t.lo - margin).max(d[0]), (t.hi + margin).min(d[1])],
            [(contact_t.lo - 0.05).max(0.), (contact_t.hi + 0.05).min(1.)],
        ];
        let report = crate::source_contact_point::qualify(
            source.surface(),
            source.curve(),
            &contact,
            selector,
            max_mapping_cells,
        )?;
        let mut out = CutPartitionReport {
            partition: None,
            mapping_cells: report.mapping_cells,
            reason: report.reason,
        };
        let Some(point) = report.point else {
            return Ok(out);
        };
        let selected = source.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let root = point.root_parameters()[0];
        let canonical = crate::source_seam_tangency::map(edge, side, I::new(root[0], root[1])?)?
            .intersect(0., 1.)?;
        let transported = crate::source_seam_tangency::inverse(edge, 1 - side, canonical)?;
        let bounds = other.parameter_bounds();
        let cut = [transported.lo, transported.hi];
        if cut[0] <= bounds[0][1].min(bounds[1][1]) || cut[1] >= bounds[0][0].max(bounds[1][0]) {
            out.reason = "profile-cut-neighbor-order-unproven";
            return Ok(out);
        }
        out.partition = Some(CutPartition {
            endpoint: self,
            point,
            selected,
            neighbor_ranges: [[bounds[0], cut], [cut, bounds[1]]],
        });
        out.reason = "profile-cut-source-partition-qualified";
        Ok(out)
    }
pub fn proposal(&self) -> &Proposal<'_> {
        self.proposal
    }
pub fn faces(&self) -> [usize; 2] {
        [self.face, self.neighbor]
    }
pub fn edge(&self) -> usize {
        self.edge
    }
pub fn canonical_bounds(&self) -> [f64; 2] {
        self.canonical
    }
pub fn neighbor_uv_bounds(&self) -> [[f64; 2]; 2] {
        self.neighbor_uv
    }
pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
#[cfg(feature = "codec")]
    pub fn definition(&self) -> value_codec::Value {
        value_codec::json!({"version":1,"kind":"owned-profile-section-end","face":self.face,
            "edge":self.edge,"axis":self.axis,"parameter":self.parameter,"highFreeEnd":self.high})
    }
}
