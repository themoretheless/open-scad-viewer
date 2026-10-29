//! Replay projective shell incidence from qualified material regions.
pub use crate::source_shell_restore::Limits;
#[cfg(feature = "codec")]
use crate::{
    source_projective_edge::Edge,
    source_region_restore,
    source_shell_incidence::{
        self, Address, Pair, Pole, ProjectiveMaps, RootCandidates, RootFractionCandidates,
        RootPlanes, Shell,
    },
};
#[cfg(feature = "codec")]
use nurbs_core::{Error, Result, curve::Curve};
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Value, json};
#[cfg(feature = "codec")]
fn invalid() -> Error {
    Error::new(
        "BREP_PROJECTIVE_SHELL_RESTORE",
        "Invalid original projective shell definition",
    )
}
#[cfg(feature = "codec")]
fn decode<T: for<'a> Deserialize<'a>>(v: Value) -> Result<T> {
    T::from_value(v).map_err(|_| invalid())
}
#[cfg(feature = "codec")]
impl Shell<Edge> {
    pub fn definition(&self) -> Result<Value> {
        let regions = self.regions().ok_or_else(invalid)?;
        let pairs:Vec<_>=self.uses().iter().enumerate().map(|(i,uses)|
            json!({"uses":uses.map(|a|[a.face,a.wire,a.edge]),"edge":self.edges()[i].definition()})).collect();
        Ok(
            json!({"version":1,"kind":"projective-shell","regions":regions.iter().map(|r|r.definition()).collect::<Vec<_>>(),"pairs":pairs,"poles":self.poles().iter().map(|(a,p)|json!({"use":[a.face,a.wire,a.edge],"point":p.point()})).collect::<Vec<_>>()}),
        )
    }
}
/// No serialized vertex IDs, bounds or admission flags grant authority.
/// Original material regions, root selectors and all edge maps are replayed.
#[cfg(feature = "codec")]
pub fn restore(value: Value, limits: &Limits) -> Result<source_shell_incidence::Report<Edge>> {
    if value["version"].as_u64() != Some(1)
        || value["kind"].as_str() != Some("projective-shell")
        || !(1..=100000000).contains(&limits.exact_work)
        || limits.driver_cells > 100000
    {
        return Err(invalid());
    }
    let definitions = value["regions"]
        .as_array()
        .filter(|a| (2..=4096).contains(&a.len()))
        .ok_or_else(invalid)?;
    let regions = definitions
        .iter()
        .map(|v| source_region_restore::restore(v.clone(), &limits.regions))
        .collect::<Result<Vec<_>>>()?;
    let definitions = value["pairs"]
        .as_array()
        .filter(|a| a.len() <= 32768)
        .ok_or_else(invalid)?;
    let mut pairs = Vec::new();
    let mut maps = Vec::new();
    let mut planes = Vec::new();
    let mut candidates = Vec::new();
    let mut fractions = Vec::new();
    for (pair, definition) in definitions.iter().enumerate() {
        let addresses: [[usize; 3]; 2] = decode(definition["uses"].clone())?;
        let uses = addresses.map(|[face, wire, edge]| Address { face, wire, edge });
        let edge = &definition["edge"];
        if edge["version"].as_u64() != Some(1) {
            return Err(invalid());
        }
        let recorded = edge["uses"]
            .as_array()
            .filter(|a| a.len() == 2)
            .ok_or_else(invalid)?;
        let mut worlds = Vec::new();
        let mut ranges = Vec::new();
        let mut weights = Vec::new();
        for side in 0..2 {
            if recorded[side]["version"].as_u64() != Some(1) {
                return Err(invalid());
            }
            let a = uses[side];
            let original = regions
                .get(a.face)
                .and_then(|r| r.loops().get(a.wire))
                .and_then(|w| w.get(a.edge))
                .ok_or_else(invalid)?;
            let fragment = crate::source_boundary_fragment::restore(
                recorded[side]["fragment"].clone(),
                limits.regions.mapping_cells,
            )?;
            if original.definition() != fragment.definition() {
                return Err(invalid());
            }
            worlds.push(decode::<Curve>(recorded[side]["world"].clone())?);
            ranges.push(decode::<[[f64; 2]; 2]>(recorded[side]["range"].clone())?);
            weights.push(decode::<[f64; 2]>(recorded[side]["weights"].clone())?);
        }
        if worlds[0] != worlds[1] {
            return Err(invalid());
        }
        pairs.push(Pair {
            uses,
            world: worlds.remove(0),
            world_reversed: [false; 2],
            cutters: [None, None],
        });
        maps.push(ProjectiveMaps {
            pair,
            ranges: [ranges[0], ranges[1]],
            weights: [weights[0], weights[1]],
        });
        planes.push(RootPlanes {
            pair,
            planes: decode(edge["planes"].clone())?,
        });
        candidates.push(RootCandidates {
            pair,
            candidates: decode(edge["candidates"].clone())?,
        });
        if !edge["fractionCandidates"].is_null() {
            fractions.push(RootFractionCandidates {
                pair,
                candidates: decode(edge["fractionCandidates"].clone())?,
            });
        }
    }
    let mut poles = Vec::new();
    if !value["poles"].is_null() {
        let values = value["poles"]
            .as_array()
            .filter(|a| a.len() <= 65536)
            .ok_or_else(invalid)?;
        for p in values {
            let [face, wire, edge] = decode(p["use"].clone())?;
            poles.push(Pole {
                use_: Address { face, wire, edge },
                point: decode(p["point"].clone())?,
            });
        }
    }
    source_shell_incidence::assemble_regions_projective_with_fraction_candidates(
        &regions,
        &pairs,
        &planes,
        &maps,
        &candidates,
        &fractions,
        &poles,
        limits.exact_work,
        limits.driver_cells,
    )
}
