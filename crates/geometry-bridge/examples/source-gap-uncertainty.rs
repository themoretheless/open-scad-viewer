//! Inspect original chart obligations; no material or whole-wall certificate.
use value_codec::{Deserialize, Value, json};
type Result<T> = std::result::Result<T, String>;
fn field<T: for<'a> Deserialize<'a>>(v: &Value, key: &str) -> Result<T> {
    T::from_value(v[key].clone()).map_err(|e| e.to_string())
}
fn limits(v: &Value) -> Result<brep_core::source_body_restore::Limits> {
    let shell = &v["shell"];
    let regions = &shell["regions"];
    let region = &regions["region"];
    let embedding = &v["embedding"];
    let pairs = &embedding["pairs"];
    let volume = &v["volume"];
    Ok(brep_core::source_body_restore::Limits {
        shell: brep_core::source_shell_restore::Limits {
            exact_work: field(shell, "exactWork")?,
            driver_cells: field(shell, "driverCells")?,
            regions: brep_core::source_region_restore::Limits {
                region: brep_core::trimmed_face_recipe::Limits {
                    pairs: field(region, "pairs")?,
                    region_cells: field(region, "regionCells")?,
                    domain_cells: field(region, "domainCells")?,
                    agreement_cells: field(region, "agreementCells")?,
                },
                search_cells: field(regions, "searchCells")?,
                point_checks: field(regions, "pointChecks")?,
                mapping_cells: field(regions, "mappingCells")?,
                driver_cells: field(regions, "driverCells")?,
                membership_cells: field(regions, "membershipCells")?,
                controls: field(regions, "controls")?,
                winding_cells: field(regions, "windingCells")?,
                steps: field(regions, "steps")?,
            },
        },
        embedding: brep_core::source_shell_geometry::Limits {
            tolerance_uv: field(embedding, "toleranceUv")?,
            corners: field(embedding, "corners")?,
            spans: field(embedding, "spans")?,
            linear_cells: field(embedding, "linearCells")?,
            exact_work: field(embedding, "exactWork")?,
            driver_cells: field(embedding, "driverCells")?,
            pairs: brep_core::face_contacts::Limits {
                pairs: field(pairs, "pairs")?,
                cells: field(pairs, "cells")?,
                domain_cells: field(pairs, "domainCells")?,
                cells_per_pair: field(pairs, "cellsPerPair")?,
                domain_cells_per_pair: field(pairs, "domainCellsPerPair")?,
            },
        },
        volume: brep_core::source_volume::Limits {
            axis: field(volume, "axis")?,
            origin: field(volume, "origin")?,
            absolute_error: field(volume, "absoluteError")?,
            tolerance_uv: field(volume, "toleranceUv")?,
            cells: field(volume, "cells")?,
            spans: field(volume, "spans")?,
            domain_cells: field(volume, "domainCells")?,
        },
    })
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let request: Value =
        value_codec::from_str(&std::fs::read_to_string(&args[1]).unwrap()).unwrap();
    let admitted = brep_core::source_body_restore::restore(
        request["definition"].clone(),
        limits(&request["limits"]).unwrap(),
    )
    .unwrap();
    let body = admitted.body().expect("original body admission");
    let inventory: Value =
        value_codec::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap();
    let pairs = inventory["cases"].as_array().unwrap().last().unwrap()["coverage"]["pairs"]
        .as_array()
        .unwrap();
    let mut results = Vec::new();
    for pair in pairs
        .iter()
        .filter(|p| p["reason"] == json!("source-face-gap-resolution-limit"))
    {
        let a = pair["faces"][0].as_u64().unwrap() as usize;
        let b = pair["faces"][1].as_u64().unwrap() as usize;
        let r = brep_core::source_face_gap::qualify(
            body,
            [&[a], &[b]],
            5.99,
            brep_core::source_face_gap::Limits {
                cells: 100000,
                spans: 100000,
            },
        )
        .unwrap();
        results.push(json!({"faces":[a,b],"reason":r.reason,"cells":r.cells,"spans":r.spans,"uncertainUv":r.uncertain_uv,"qualified":r.certificate.is_some()}));
    }
    std::fs::write(&args[3],value_codec::to_string_pretty(&json!({"scope":"Original full chart gap obligations; retained trims not witnessed", "pairs":results})).unwrap()).unwrap();
}
