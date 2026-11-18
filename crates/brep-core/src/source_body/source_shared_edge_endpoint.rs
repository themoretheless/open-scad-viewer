//! Endpoint identity comes from admitted opposite uses, not interval overlap.
use super::SharedEdge;
use crate::source_seam_tangency::Carrier;
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};
pub struct BoundaryEndpoint<'a,E = SharedEdge> {
    edge: &'a E,
    ends: [usize; 2],
    canonical: [f64; 2],
    parameters: [[f64; 2]; 2],
    uv: [[[f64; 2]; 2]; 2],
    world: [[f64; 2]; 3],
}
impl<E> BoundaryEndpoint<'_,E> {
    pub fn edge(&self) -> &E {
        self.edge
    }
    pub fn ends(&self) -> [usize; 2] {
        self.ends
    }
    pub fn canonical_bounds(&self) -> [f64; 2] {
        self.canonical
    }
    pub fn parameter_bounds(&self) -> [[f64; 2]; 2] {
        self.parameters
    }
    pub fn uv_bounds(&self) -> [[[f64; 2]; 2]; 2] {
        self.uv
    }
    pub fn world_bounds(&self) -> [[f64; 2]; 3] {
        self.world
    }
}
pub struct BoundaryEndpointReport<'a, E = SharedEdge> {
    pub endpoint: Option<BoundaryEndpoint<'a, E>>,
    pub spans: usize,
    pub reason: &'static str,
}
impl SharedEdge {
    pub fn sample_endpoint(
        &self,
        end: usize,
        max_spans: usize,
    ) -> Result<BoundaryEndpointReport<'_>> {
        sample_endpoint(self, end, max_spans)
    }
}

/// `end` follows the traversal of the first incident fragment. The opposite
/// fragment uses `1-end`. Original root objects remain owned by this edge.
/// These enclosures cannot authorize rounded endpoint welding.
pub(crate) fn sample_endpoint<'a, E: Carrier>(
    edge: &'a E,
    end: usize,
    max_spans: usize,
) -> Result<BoundaryEndpointReport<'a, E>> {
    if end > 1 || max_spans > 100000 {
        return Err(Error::new(
            "BREP_SHARED_ENDPOINT",
            "Choose an endpoint and bounded work",
        ));
    }
    let mut report = BoundaryEndpointReport {
        endpoint: None,
        spans: 0,
        reason: "boundary-endpoint-source-map-unproven",
    };
    // Admission owns exact parameter maps of both original pcurves. Endpoint
    // evaluation needs those maps only on the retained ends, not coverage
    // of the complete canonical curve by each original pcurve.
    let ends = [end, 1 - end];
    let parameters = std::array::from_fn(|side| edge.fragment(side).parameter_bounds()[ends[side]]);
    let mut canonical = [I::point(0.); 2];
    for side in 0..2 {
        canonical[side] =
            edge.map_parameter(side, I::new(parameters[side][0], parameters[side][1])?)?;
    }
    // Pair admission independently proves equal endpoints. Intersect only their
    // outward enclosures; the intersection is never the identity proof.
    let lo = canonical[0].lo.max(canonical[1].lo).max(0.);
    let hi = canonical[0].hi.min(canonical[1].hi).min(1.);
    if lo > hi {
        report.reason = "boundary-endpoint-enclosure-unproven";
        return Ok(report);
    }
    let canonical = I::new(lo, hi)?;
    let d = edge.world().domain();
    let world_parameter = I::point(d[0])
        .add(canonical.mul(I::point(d[1]).sub(I::point(d[0]))?)?)?
        .intersect(d[0], d[1])?;
    let queries = [
        (
            edge.fragment(0).curve(),
            I::new(parameters[0][0], parameters[0][1])?,
        ),
        (
            edge.fragment(1).curve(),
            I::new(parameters[1][0], parameters[1][1])?,
        ),
        (edge.world(), world_parameter),
    ];
    let mut values = Vec::new();
    for (curve, t) in queries {
        let spent = (curve.degree..curve.control_points.len())
            .filter(|&i| {
                curve.knots[i] < curve.knots[i + 1]
                    && curve.knots[i] <= t.hi
                    && curve.knots[i + 1] >= t.lo
            })
            .count();
        if spent > max_spans - report.spans {
            report.reason = "boundary-endpoint-work-limit";
            return Ok(report);
        }
        report.spans += spent;
        values.push(interval_eval::evaluate_interval(curve, t)?);
    }
    report.endpoint = Some(BoundaryEndpoint {
        edge,
        ends,
        canonical: [canonical.lo, canonical.hi],
        parameters,
        uv: std::array::from_fn(|s| std::array::from_fn(|k| [values[s][k].lo, values[s][k].hi])),
        world: std::array::from_fn(|k| [values[2][k].lo, values[2][k].hi]),
    });
    report.reason = "boundary-endpoint-qualified";
    Ok(report)
}
