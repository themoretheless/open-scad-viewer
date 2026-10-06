//! Bounded exchange representatives of exact privately admitted source vertices.
//! Root recipes and canonical restrictions remain stored separately and unchanged.
//! This prepares endpoint coordinates, not a Model, STEP file or mesh admission.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment},
    source_edge_restriction::Restriction,
};
use nurbs_core::{Error, Result, interval_eval::Interval as I};
use std::collections::BTreeMap;
pub struct Limits {
    /// Global fresh refinement query budget; identity replay queries are reported separately.
    pub root_checks: usize,
    pub mapping_cells: usize,
    /// Independently bounded mapping per original use during shared-edge replay.
    pub replay_mapping_per_use: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub spans: usize,
    pub endpoints: usize,
}
pub struct Vertex {
    pub source_id: usize,
    pub point: [f64; 3],
    pub bounds: [[f64; 2]; 3],
    pub error_upper: f64,
}
pub struct Prepared {
    original: value_codec::Value,
    restrictions: Vec<Restriction>,
    vertices: Vec<Vertex>,
}
impl Prepared {
    pub fn original(&self) -> &value_codec::Value {
        &self.original
    }
    pub fn restrictions(&self) -> &[Restriction] {
        &self.restrictions
    }
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }
}
pub struct Report {
    pub prepared: Option<Prepared>,
    pub root_checks: usize,
    pub identity_root_checks: usize,
    pub mapping_cells: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
    pub spans: usize,
    pub reason: &'static str,
    pub uncertain_vertex: Option<usize>,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_EXCHANGE_ENDPOINTS", message)
}
pub fn prepare(
    body: &crate::source_volume::Body,
    tolerance_mm: f64,
    limits: Limits,
) -> Result<Report> {
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !(1..=100000).contains(&limits.root_checks)
        || !(1..=100000).contains(&limits.mapping_cells)
        || !(1..=100000).contains(&limits.replay_mapping_per_use)
        || !(1..=100_000_000).contains(&limits.exact_work)
        || limits.driver_cells > 100000
        || !(1..=100000).contains(&limits.spans)
        || !(1..=100000).contains(&limits.endpoints)
    {
        return Err(invalid(
            "Choose positive exchange tolerance and bounded endpoint work",
        ));
    }
    let shell = body.geometry().shell();
    if shell
        .edges()
        .len()
        .checked_mul(2)
        .and_then(|n| n.checked_add(shell.poles().len()))
        .is_none_or(|n| n > limits.endpoints)
    {
        return Err(invalid("Endpoint work limit"));
    }
    let mut out = Report {
        prepared: None,
        root_checks: 0,
        identity_root_checks: 0,
        mapping_cells: 0,
        exact_work: 0,
        driver_cells: 0,
        spans: 0,
        reason: "source-exchange-endpoints-work-limit",
        uncertain_vertex: None,
    };
    let mut restrictions = Vec::new();
    let mut boxes: BTreeMap<usize, [[f64; 2]; 3]> = BTreeMap::new();
    let meet = |boxes: &mut BTreeMap<usize, [[f64; 2]; 3]>,
                id: usize,
                bounds: [[f64; 2]; 3]|
     -> Result<()> {
        let common = boxes.entry(id).or_insert(bounds);
        for axis in 0..3 {
            common[axis] = [
                common[axis][0].max(bounds[axis][0]),
                common[axis][1].min(bounds[axis][1]),
            ];
            if !common[axis].iter().all(|v| v.is_finite()) || common[axis][0] > common[axis][1] {
                return Err(invalid("Exact vertex has disjoint refined enclosures"));
            }
        }
        Ok(())
    };
    for (index, edge) in shell.edges().iter().enumerate() {
        let mut value = edge.definition();
        let mut uses = Vec::new();
        for fragment in edge.uses() {
            let mut endpoints = Vec::new();
            for endpoint in fragment.endpoints() {
                endpoints.push(match endpoint {
                    Endpoint::Parameter(t) => Endpoint::Parameter(*t),
                    Endpoint::Crossing { point, role } => {
                        if out.root_checks == limits.root_checks
                            || out.mapping_cells == limits.mapping_cells
                        {
                            return Ok(out);
                        }
                        let target = tolerance_mm / 64.;
                        if target <= 0. {
                            out.reason = "source-exchange-tolerance-unrepresentable";
                            return Ok(out);
                        }
                        let refined = crate::source_root_refinement::qualify(
                            point,
                            target,
                            (limits.root_checks - out.root_checks).min(128),
                            limits.mapping_cells - out.mapping_cells,
                        )?;
                        out.root_checks += refined.root_checks;
                        out.mapping_cells += refined.mapping_cells;
                        let Some(proof) = refined.refinement else {
                            out.reason = refined.reason;
                            return Ok(out);
                        };
                        Endpoint::Crossing {
                            point: proof.refined().clone(),
                            role: *role,
                        }
                    }
                });
            }
            let mut endpoints = endpoints.into_iter();
            uses.push(
                Fragment::new(
                    fragment.surface(),
                    fragment.curve(),
                    endpoints.next().unwrap(),
                    endpoints.next().unwrap(),
                )?
                .definition(),
            );
        }
        value["uses"] = value_codec::Value::Array(uses);
        if out.exact_work == limits.exact_work {
            return Ok(out);
        }
        let replay = crate::source_shared_edge_restore::restore(
            value,
            crate::source_shared_edge_restore::Limits {
                mapping_cells_per_use: limits.replay_mapping_per_use,
                exact_work: limits.exact_work - out.exact_work,
                driver_cells: limits.driver_cells - out.driver_cells,
            },
        )?;
        out.exact_work += replay.work_used;
        out.driver_cells += replay.driver_cells;
        out.identity_root_checks += replay.root_checks;
        let Some(refined) = replay.edge else {
            out.reason = replay.reason;
            return Ok(out);
        };
        if refined.world() != edge.world()
            || refined.reversed() != edge.reversed()
            || refined.ranges() != edge.ranges()
        {
            return Err(invalid(
                "Refined restriction changed original carrier or direction",
            ));
        }
        let restriction = Restriction::from_edge(&refined);
        let curve = edge.world();
        let spans = (curve.degree..curve.control_points.len())
            .filter(|&i| curve.knots[i] < curve.knots[i + 1])
            .count();
        out.spans = out
            .spans
            .checked_add(2 * spans)
            .ok_or_else(|| invalid("Span count overflow"))?;
        if out.spans > limits.spans {
            return Ok(out);
        }
        let address = shell.uses()[index][0];
        let mut ids = shell.vertices()[address.face][address.wire][address.edge];
        if edge.reversed()[0] {
            ids.reverse();
        }
        for (end, bounds) in restriction
            .endpoint_boxes(limits.spans)?
            .into_iter()
            .enumerate()
        {
            meet(&mut boxes, ids[end], bounds)?;
        }
        restrictions.push(restriction);
    }
    for (address, pole) in shell.poles() {
        let id = shell.vertices()[address.face][address.wire][address.edge][0];
        meet(&mut boxes, id, pole.point().map(|v| [v, v]))?;
    }
    let mut vertices = Vec::new();
    for (source_id, bounds) in boxes {
        out.uncertain_vertex = Some(source_id);
        let point = bounds.map(|b| (b[0] * 0.5 + b[1] * 0.5).clamp(b[0], b[1]));
        let mut error = I::point(0.);
        for axis in 0..3 {
            let a = I::point(point[axis]).sub(I::point(bounds[axis][0]))?;
            let b = I::point(bounds[axis][1]).sub(I::point(point[axis]))?;
            error = error.add(I::point(a.hi.max(b.hi)))?;
        }
        if error.hi > tolerance_mm {
            out.reason = "source-exchange-endpoint-tolerance-unproven";
            return Ok(out);
        }
        vertices.push(Vertex {
            source_id,
            point,
            bounds,
            error_upper: error.hi,
        });
    }
    out.prepared = Some(Prepared {
        original: body.definition()?,
        restrictions,
        vertices,
    });
    out.uncertain_vertex = None;
    out.reason = "source-exchange-endpoints-qualified";
    Ok(out)
}
