//! Bounded numeric trim parameters for exchange, retaining original carriers.
//! This is an endpoint certificate, not a STEP or whole-curve certificate.
use crate::source_exchange_endpoints::Prepared;
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
};

pub struct Edge {
    canonical: [f64; 2],
    uses: [[f64; 2]; 2],
    error_upper: [[f64; 2]; 3],
}
impl Edge {
    pub fn canonical(&self) -> [f64; 2] {
        self.canonical
    }
    /// Parameters in each original fragment's directed endpoint order.
    pub fn uses(&self) -> [[f64; 2]; 2] {
        self.uses
    }
    /// Canonical 3D carrier then both surface-composed pcurves; canonical end order.
    pub fn error_upper(&self) -> [[f64; 2]; 3] {
        self.error_upper
    }
}
pub struct Trims {
    edges: Vec<Edge>,
    work: usize,
}
impl Trims {
    pub fn edges(&self) -> &[Edge] {
        &self.edges
    }
    pub fn work(&self) -> usize {
        self.work
    }
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_EXCHANGE_TRIMS", message)
}
fn midpoint(b: [f64; 2]) -> f64 {
    (b[0] * 0.5 + b[1] * 0.5).clamp(b[0], b[1])
}
fn distance(point: [f64; 3], bounds: &[I]) -> Result<f64> {
    if bounds.len() != 3 {
        return Err(invalid("Expected three world coordinates"));
    }
    let mut sum = I::point(0.);
    for a in 0..3 {
        let delta = bounds[a].sub(I::point(point[a]))?;
        sum = sum.add(I::point(delta.lo.abs().max(delta.hi.abs())))?;
    }
    Ok(sum.hi)
}
/// All carriers and vertex identities come from the same privately admitted Body.
/// Work bounds original control counts used by interval evaluations, with no
/// reliance on caller supplied or persisted approximation certificates.
pub fn prepare(
    body: &crate::source_volume::Body,
    endpoints: &Prepared,
    tolerance_mm: f64,
    max_work: usize,
) -> Result<Option<Trims>> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. || !(1..=10_000_000).contains(&max_work) {
        return Err(invalid("Choose positive tolerance and bounded trim work"));
    }
    if endpoints.original() != &body.definition()? {
        return Err(invalid("Endpoint preparation belongs to another Body"));
    }
    let shell = body.geometry().shell();
    if endpoints.restrictions().len() != shell.edges().len() {
        return Err(invalid("Incomplete carrier preparation"));
    }
    let mut work = 0usize;
    let mut edges = Vec::new();
    for (index, restriction) in endpoints.restrictions().iter().enumerate() {
        let shared = restriction.edge();
        let cost = 2 * shared.world().control_points.len()
            + shared
                .uses()
                .iter()
                .map(|f| {
                    2 * (f.curve().control_points.len()
                        + f.surface()
                            .control_points
                            .iter()
                            .map(Vec::len)
                            .sum::<usize>())
                })
                .sum::<usize>();
        work = work
            .checked_add(cost)
            .ok_or_else(|| invalid("Trim work overflow"))?;
        if work > max_work {
            return Ok(None);
        }
        let address = shell.uses()[index][0];
        let mut ids = shell.vertices()[address.face][address.wire][address.edge];
        if shared.reversed()[0] {
            ids.reverse();
        }
        let mut points = [[0.; 3]; 2];
        for end in 0..2 {
            points[end] = endpoints
                .vertices()
                .iter()
                .find(|v| v.source_id == ids[end])
                .ok_or_else(|| invalid("Missing original vertex identity"))?
                .point;
        }
        let canonical = restriction.parameter_bounds()?.map(midpoint);
        if canonical[0] >= canonical[1] {
            return Ok(None);
        }
        let mut errors = [[0.; 2]; 3];
        for end in 0..2 {
            errors[0][end] = distance(
                points[end],
                &interval_eval::evaluate_interval(shared.world(), I::point(canonical[end]))?,
            )?;
        }
        let mut uses = [[0.; 2]; 2];
        for (slot, fragment) in shared.uses().iter().enumerate() {
            uses[slot] = fragment.parameter_bounds().map(midpoint);
            if uses[slot][0] == uses[slot][1]
                || ((uses[slot][0] > uses[slot][1]) != fragment.reversed())
            {
                return Ok(None);
            }
            for local in 0..2 {
                let end = if shared.reversed()[slot] {
                    1 - local
                } else {
                    local
                };
                let uv = interval_eval::evaluate_interval(
                    fragment.curve(),
                    I::point(uses[slot][local]),
                )?;
                if uv.len() != 2 {
                    return Err(invalid("Expected original 2D pcurve"));
                }
                let world =
                    interval_eval::evaluate_surface_interval(fragment.surface(), uv[0], uv[1])?;
                errors[slot + 1][end] = distance(points[end], &world)?;
            }
        }
        if errors.iter().flatten().any(|e| *e > tolerance_mm) {
            return Ok(None);
        }
        edges.push(Edge {
            canonical,
            uses,
            error_upper: errors,
        });
    }
    Ok(Some(Trims { edges, work }))
}
