//! Preserve the parameter laws of a shared spatial edge and all its face-local uses.
use crate::{Model, Result, invalid, require};
use nurbs_core::editing::exact_knot_removal::compress_curve_batch;

impl Model {
    /// Compress one edge and all its UV curves atomically. Each curve keeps its
    /// original parameter domain and orientation, including opposite traversals.
    /// An exhausted proof budget yields no model.
    pub fn compress_edge_with_trims(&self, edge: usize, max_work: u64) -> Result<Option<Self>> {
        require(
            (1..=cad_predicates::MAX_WORK).contains(&max_work),
            "Invalid compression work limit",
        )?;
        let spatial = &self
            .edges
            .get(edge)
            .ok_or_else(|| invalid("Invalid edge index"))?
            .curve;
        self.validate()?;
        let mut locations = Vec::new();
        let mut sources = vec![spatial];
        let mut controls = spatial.control_points.len() as u64;
        for (loop_index, boundary) in self.loops.iter().enumerate() {
            for (index, usage) in boundary
                .coedges
                .iter()
                .enumerate()
                .filter(|(_, usage)| usage.edge == edge)
            {
                controls += usage.pcurve.control_points.len() as u64;
                if controls >= max_work {
                    return Ok(None);
                }
                locations.push((loop_index, index));
                sources.push(&usage.pcurve);
            }
        }
        if controls >= max_work {
            return Ok(None);
        }
        let curves = sources.into_iter().cloned().collect::<Vec<_>>();
        let compressed = compress_curve_batch(&curves, max_work)?;
        let Some(segments) = compressed.segments else {
            return Ok(None);
        };
        let mut segments = segments.into_iter();
        let mut model = self.clone();
        model.edges[edge].curve = segments
            .next()
            .ok_or_else(|| invalid("Missing spatial curve"))?
            .curve;
        for ((boundary, index), segment) in locations.into_iter().zip(segments) {
            model.loops[boundary].coedges[index].pcurve = segment.curve;
        }
        model.validate()?;
        Ok(Some(model))
    }
}
