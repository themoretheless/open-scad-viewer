//! Original shell definitions replay material regions and every ownership gate.
use crate::{
    source_region_restore,
    source_shell_incidence::{
        self, Address, AffineMaps, Pair, Pole, RootCandidates, RootPlanes, Shell,
    },
};
use nurbs_core::{Error, Result};
use value_codec::{Deserialize, Value, json};
pub struct Limits {
    /// Region audit budgets are independent per face/replay step.
    pub regions: source_region_restore::Limits,
    pub exact_work: u64,
    pub driver_cells: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_SOURCE_SHELL_RESTORE", message)
}
fn decode<T: for<'de> Deserialize<'de>>(v: Value) -> Result<T> {
    T::from_value(v).map_err(|_| invalid("Invalid source shell field"))
}
fn address(a: Address) -> [usize; 3] {
    [a.face, a.wire, a.edge]
}
fn read_address(v: Value) -> Result<Address> {
    let [face, wire, edge] = decode(v)?;
    Ok(Address { face, wire, edge })
}
pub(crate) fn definition(shell: &Shell) -> Result<Value> {
    let regions = shell
        .regions()
        .ok_or_else(|| invalid("Qualified source material regions required"))?;
    let pairs = shell
        .uses()
        .iter()
        .enumerate()
        .map(|(i, uses)| json!({"uses":uses.map(address),"edge":shell.edges()[i].definition()}))
        .collect::<Vec<_>>();
    let poles = shell
        .poles()
        .iter()
        .map(|(a, p)| json!({"use":address(*a),"point":p.point()}))
        .collect::<Vec<_>>();
    Ok(
        json!({"version":1,"regions":regions.iter().map(|r|r.definition()).collect::<Vec<_>>(),"pairs":pairs,"poles":poles}),
    )
}
pub fn restore(value: Value, limits: &Limits) -> Result<source_shell_incidence::Report> {
    if value["version"].as_u64() != Some(1)
        || !(1..=100_000_000).contains(&limits.exact_work)
        || limits.driver_cells > 100000
    {
        return Err(invalid("Bound source shell version and exact/driver work"));
    }
    let region_values = value["regions"]
        .as_array()
        .filter(|v| (2..=4096).contains(&v.len()))
        .ok_or_else(|| invalid("Bound source region count"))?;
    let regions = region_values
        .iter()
        .map(|r| source_region_restore::restore(r.clone(), &limits.regions))
        .collect::<Result<Vec<_>>>()?;
    let pair_values = value["pairs"]
        .as_array()
        .filter(|v| v.len() <= 524288)
        .ok_or_else(|| invalid("Bound source pair count"))?;
    let mut pairs = Vec::new();
    let mut planes = Vec::new();
    let mut maps = Vec::new();
    let mut candidates = Vec::new();
    for (index, p) in pair_values.iter().enumerate() {
        let uses = p["uses"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or_else(|| invalid("Two source uses required"))?;
        let uses = [
            read_address(uses[0].clone())?,
            read_address(uses[1].clone())?,
        ];
        let edge = &p["edge"];
        if edge["version"].as_u64() != Some(1) {
            return Err(invalid("Unknown shared-edge definition"));
        }
        let recorded = edge["uses"]
            .as_array()
            .filter(|v| v.len() == 2)
            .ok_or_else(|| invalid("Two recorded source definitions required"))?;
        for slot in 0..2 {
            let a = uses[slot];
            let fragment = regions
                .get(a.face)
                .and_then(|r| r.loops().get(a.wire))
                .and_then(|w| w.get(a.edge))
                .ok_or_else(|| invalid("Unknown source use address"))?;
            // JSON/binary transports may encode integral f64 coordinates as
            // integer number variants. Reconstruct typed original expressions
            // before comparing; never relax source identity to proximity.
            let recorded_fragment = crate::source_boundary_fragment::restore(
                recorded[slot].clone(),
                limits.regions.mapping_cells,
            )?;
            if fragment.definition() != recorded_fragment.definition() {
                return Err(invalid(
                    "Recorded edge use disagrees with replayed material boundary",
                ));
            }
        }
        let recipe = &edge["recipe"];
        let root_planes = decode(recipe["planes"].clone())?;
        planes.push(RootPlanes {
            pair: index,
            planes: root_planes,
        });
        let (world_reversed, cutters) = match recipe["kind"].as_str() {
            Some("direct") => (
                decode(recipe["worldReversed"].clone())?,
                decode(recipe["cutters"].clone())?,
            ),
            Some("mapped") => {
                maps.push(AffineMaps {
                    pair: index,
                    ranges: decode(recipe["ranges"].clone())?,
                });
                candidates.push(RootCandidates {
                    pair: index,
                    candidates: decode(recipe["candidates"].clone())?,
                });
                ([false; 2], [None, None])
            }
            _ => return Err(invalid("Unknown canonical source mapping recipe")),
        };
        pairs.push(Pair {
            uses,
            world: decode(edge["world"].clone())?,
            world_reversed,
            cutters,
        });
    }
    let pole_values = value["poles"]
        .as_array()
        .filter(|v| v.len() <= 1048576)
        .ok_or_else(|| invalid("Bound collapsed source use count"))?;
    let poles = pole_values
        .iter()
        .map(|p| {
            Ok(Pole {
                use_: read_address(p["use"].clone())?,
                point: decode(p["point"].clone())?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    source_shell_incidence::assemble_regions_with_pole_inputs(
        &regions,
        &pairs,
        &planes,
        &maps,
        &candidates,
        &poles,
        limits.exact_work,
        limits.driver_cells,
    )
}

#[cfg(test)]
pub(crate) fn assert_replay(shell: &Shell) {
    let definition = shell.definition().unwrap();
    let encoded = value_codec::to_string(&definition).unwrap();
    let value: Value = value_codec::from_str(&encoded).unwrap();
    let limits = Limits {
        regions: source_region_restore::test_limits(),
        exact_work: 100_000_000,
        driver_cells: 100000,
    };
    let report = restore(value, &limits).unwrap();
    let replay = report.shell.expect(report.reason);
    assert_eq!(replay.definition().unwrap(), definition);
    assert_eq!(replay.vertices(), shell.vertices());
    assert_eq!(replay.uses(), shell.uses());
    assert_eq!(
        crate::source_boundary_network::inspect(&replay, 100000, 100000).unwrap(),
        crate::source_boundary_network::inspect(shell, 100000, 100000).unwrap()
    );
    for region in shell.regions().unwrap() {
        source_region_restore::assert_replay(region);
    }
    let topology = crate::source_vertex_links::inspect(&replay, 100000).unwrap();
    assert!(topology.all_manifold);
    let mut missing = definition.clone();
    missing["pairs"].as_array_mut().unwrap().pop();
    missing["certificateQualified"] = json!(true);
    assert!(restore(missing, &limits).is_err());
    if !shell.poles().is_empty() {
        let mut missing_pole = definition.clone();
        missing_pole["poles"].as_array_mut().unwrap().pop();
        missing_pole["certificateQualified"] = json!(true);
        assert!(restore(missing_pole, &limits).is_err());
    }
    let exhausted = Limits {
        regions: source_region_restore::test_limits(),
        exact_work: 1,
        driver_cells: 100000,
    };
    assert!(restore(definition, &exhausted).unwrap().shell.is_none());
}
