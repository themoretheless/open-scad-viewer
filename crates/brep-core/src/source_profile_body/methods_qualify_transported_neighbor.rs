//! source profile body operations: qualify_transported_neighbor.
use super::*;
impl CutPartition<'_, '_, '_> {
/// Transport the existing selected root through its admitted original
    /// carrier, then freshly prove both restricted compositions and endpoints.
    /// No independently rounded neighbor cutter or root parameter is authored.
    pub fn qualify_transported_neighbor(
        &self,
        max_spans: usize,
        max_work: u64,
        max_driver: usize,
    ) -> Result<NeighborPartitionReport> {
        if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound transported neighbor work",
            ));
        }
        let shell = self.endpoint.proposal.body.geometry().shell();
        let edge = &shell.edges()[self.endpoint.edge];
        let addresses = &shell.uses()[self.endpoint.edge];
        let side = usize::from(addresses[0].face != self.endpoint.face);
        let transport = crate::source_carrier_point::qualify(edge, &self.point, side, max_spans)?;
        let mut out = NeighborPartitionReport {
            partition: None,
            carrier_spans: transport.spans,
            mapping_cells: 0,
            exact_work: 0,
            root_checks: 0,
            driver_cells: 0,
            reason: transport.reason,
        };
        let Some(point) = transport.point else {
            return Ok(out);
        };
        let fragments = edge.uses()[1 - side].split_at_carrier(&point, 1 - side)?;
        let ranges = crate::source_seam_tangency::ranges(edge);
        let mut halves = Vec::new();
        for piece in 0..2 {
            if out.exact_work == max_work {
                out.reason = "profile-cut-neighbor-work-limit";
                return Ok(out);
            }
            let checked = crate::source_mapped_edge::qualify(
                edge.world(),
                [&self.selected[piece], &fragments[1 - piece]],
                [ranges[side], ranges[1 - side]],
                [None, None],
                max_work - out.exact_work,
                max_driver - out.driver_cells,
            )?;
            out.exact_work += checked.work_used;
            out.root_checks += checked.root_checks;
            out.driver_cells += checked.driver_cells;
            out.reason = checked.reason;
            let Some(half) = checked.edge else {
                return Ok(out);
            };
            halves.push(half);
        }
        out.partition = Some(NeighborPartition {
            fragments,
            edges: [halves.remove(0), halves.remove(0)],
        });
        out.reason = "profile-cut-transported-neighbor-qualified";
        Ok(out)
    }
/// Treat the neighbor cutter as a proposal: requalify its root and both
    /// restricted world-edge halves. Parameter enclosure overlap is not identity.
    pub fn qualify_neighbor(
        &self,
        contact: &nurbs_core::curve::Curve,
        selector: [[f64; 2]; 2],
        mapping_cells: usize,
        max_work: u64,
        max_driver: usize,
    ) -> Result<NeighborPartitionReport> {
        if !(1..=100000000).contains(&max_work) || max_driver > 100000 {
            return Err(Error::new(
                "BREP_PROFILE_CUT",
                "Bound fresh neighbor edge work",
            ));
        }
        let shell = self.endpoint.proposal.body.geometry().shell();
        let index = self.endpoint.edge;
        let edge = &shell.edges()[index];
        let addresses = &shell.uses()[index];
        let side = usize::from(addresses[0].face != self.endpoint.face);
        let other = &edge.uses()[1 - side];
        let root = crate::source_contact_point::qualify(
            other.surface(),
            other.curve(),
            contact,
            selector,
            mapping_cells,
        )?;
        let mut out = NeighborPartitionReport {
            partition: None,
            carrier_spans: 0,
            mapping_cells: root.mapping_cells,
            exact_work: 0,
            root_checks: 0,
            driver_cells: 0,
            reason: root.reason,
        };
        let Some(point) = root.point else {
            return Ok(out);
        };
        let fragments = other.split_at(&point, crate::source_boundary_fragment::Role::Boundary)?;
        let ranges = crate::source_seam_tangency::ranges(edge);
        let mut halves = Vec::new();
        for piece in 0..2 {
            if out.exact_work == max_work {
                out.reason = "profile-cut-neighbor-work-limit";
                return Ok(out);
            }
            let checked = crate::source_mapped_edge::qualify(
                edge.world(),
                [&self.selected[piece], &fragments[1 - piece]],
                [ranges[side], ranges[1 - side]],
                [None, None],
                max_work - out.exact_work,
                max_driver - out.driver_cells,
            )?;
            out.exact_work += checked.work_used;
            out.root_checks += checked.root_checks;
            out.driver_cells += checked.driver_cells;
            out.reason = checked.reason;
            let Some(half) = checked.edge else {
                return Ok(out);
            };
            halves.push(half);
        }
        out.partition = Some(NeighborPartition {
            fragments,
            edges: [halves.remove(0), halves.remove(0)],
        });
        out.reason = "profile-cut-neighbor-partition-qualified";
        Ok(out)
    }
pub fn endpoint(&self) -> &CutEndpoint<'_, '_> {
        self.endpoint
    }
pub fn point(&self) -> &crate::source_contact_point::SourcePoint {
        &self.point
    }
pub fn selected_fragments(&self) -> &[crate::source_boundary_fragment::Fragment; 2] {
        &self.selected
    }
pub fn neighbor_ranges(&self) -> [[[f64; 2]; 2]; 2] {
        self.neighbor_ranges
    }
}
