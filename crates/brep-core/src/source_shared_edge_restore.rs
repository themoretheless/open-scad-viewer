//! Restore original root-owned shared edges by fresh native qualification.
use crate::{source_boundary_fragment, source_shared_edge};
use nurbs_core::{Error, Result, curve::Curve};
use value_codec::{Deserialize, Value};
pub struct Limits {
    /// Root mapping restoration budget independently for each original use.
    pub mapping_cells_per_use: usize,
    pub exact_work: u64,
    pub driver_cells: usize,
}
fn invalid() -> Error {
    Error::new(
        "BREP_SOURCE_EDGE_RESTORE",
        "Invalid original shared-edge definition or budget",
    )
}
fn decode<T: for<'de> Deserialize<'de>>(value: Value) -> Result<T> {
    T::from_value(value).map_err(|_| invalid())
}
/// Cached bounds, direction flags and certificate outcomes grant no authority.
/// Original UV root equations/selectors and all raw mapping proposals are rechecked.
pub fn restore(value: Value, limits: Limits) -> Result<source_shared_edge::Report> {
    if value["version"].as_u64() != Some(1)
        || !(1..=100000).contains(&limits.mapping_cells_per_use)
        || !(1..=100_000_000).contains(&limits.exact_work)
        || limits.driver_cells > 100000
    {
        return Err(invalid());
    }
    let world: Curve = decode(value["world"].clone())?;
    let definitions = value["uses"]
        .as_array()
        .filter(|v| v.len() == 2)
        .ok_or_else(invalid)?;
    let uses = [
        source_boundary_fragment::restore(definitions[0].clone(), limits.mapping_cells_per_use)?,
        source_boundary_fragment::restore(definitions[1].clone(), limits.mapping_cells_per_use)?,
    ];
    let recipe = &value["recipe"];
    let planes = decode(recipe["planes"].clone())?;
    match recipe["kind"].as_str() {
        Some("direct") => {
            let reversed = decode(recipe["worldReversed"].clone())?;
            let cutters: [Option<Curve>; 2] = decode(recipe["cutters"].clone())?;
            source_shared_edge::qualify_with_cutters_and_planes(
                &world,
                uses.each_ref(),
                reversed,
                cutters.each_ref().map(Option::as_ref),
                planes,
                limits.exact_work,
                limits.driver_cells,
            )
        }
        Some("mapped") => crate::source_mapped_edge::qualify_with_candidates(
            &world,
            uses.each_ref(),
            decode(recipe["ranges"].clone())?,
            planes,
            decode(recipe["candidates"].clone())?,
            limits.exact_work,
            limits.driver_cells,
        ),
        _ => Err(invalid()),
    }
}
