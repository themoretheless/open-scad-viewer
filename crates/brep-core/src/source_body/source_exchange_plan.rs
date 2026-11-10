//! Kernel half of the original-source STEP exchange (ADR 0011): proves endpoints and trims of
//! a source body and counts the entities that the writer in `cad-step::source_exchange_step`
//! emits from this plan. No file format lives here.
use crate::source_exchange_endpoints::{self as endpoints, ExchangeCarrier, Prepared};
use crate::source_exchange_trims::{self as trims, Trims};
use crate::source_volume::Body;
use nurbs_core::{Error, Result};
use std::collections::BTreeSet;

pub struct Plan<E: ExchangeCarrier> {
    pub points: Prepared<E>,
    pub trims: Trims,
    pub vertices: usize,
    pub edges: usize,
    pub faces: usize,
    pub endpoint_error_upper: f64,
}

fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_EXCHANGE_STEP", message)
}

/// `Ok(None)` when an endpoint or trim budget is exhausted before every carrier is proved.
pub fn prepare<E: ExchangeCarrier>(
    body: &Body<E>,
    tolerance_mm: f64,
    limits: endpoints::Limits,
    max_trim_work: usize,
) -> Result<Option<Plan<E>>> {
    if !(1..=10_000_000).contains(&max_trim_work) {
        return Err(invalid("Choose bounded positive trim work"));
    }
    let shell = body.geometry().shell();
    shell.regions().ok_or_else(|| invalid("Original source regions required"))?;
    let report = endpoints::prepare(body, tolerance_mm, limits)?;
    let Some(points) = report.prepared else {
        return Ok(None);
    };
    let Some(pole_work) = shell.poles().len().checked_mul(2) else {
        return Err(invalid("Pole trim count overflow"));
    };
    let Some(trim_work) = max_trim_work.checked_sub(pole_work).filter(|w| *w > 0) else {
        return Ok(None);
    };
    let Some(trims) = trims::prepare(body, &points, tolerance_mm, trim_work)? else {
        return Ok(None);
    };
    let endpoint_error_upper = trims
        .edges()
        .iter()
        .flat_map(|e| e.error_upper().into_iter().flatten())
        .chain(points.vertices().iter().map(|v| v.error_upper))
        .fold(0., f64::max);
    let vertices = points.vertices().iter().map(|v| v.source_id).collect::<BTreeSet<_>>().len();
    let edges = points.restrictions().len() + shell.poles().len();
    let faces = shell.faces().len();
    Ok(Some(Plan { points, trims, vertices, edges, faces, endpoint_error_upper }))
}
