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
    let restored = brep_core::source_body_restore::restore(
        request["definition"].clone(),
        limits(&request["limits"]).unwrap(),
    )
    .unwrap();
    let body = restored.body().expect("original body admission");
    let pairs: Vec<[usize; 2]> = Vec::from_value(
        value_codec::from_str(&std::fs::read_to_string(&args[2]).unwrap()).unwrap(),
    )
    .unwrap();
    let cells=args.get(5).map(|v|v.parse::<usize>().expect("product cell limit")).unwrap_or(4096);
    let mut results = Vec::new();
    for faces in pairs {
        let r = brep_core::source_wall_pair::qualify(
            body,
            faces,
            5.99,
            brep_core::source_wall_pair::Limits {
                normal_spans: 10000,
                max_sine_squared: 1e-6,
                cells,
                spans: 10000,
                domain_cells: 10000,
                domain_cells_per_query: 256,
                tolerance_uv: 1e-8,
            },
        )
        .unwrap();
        results.push(json!({"faces":faces,"qualified":r.certificate.is_some(),"cells":r.cells,
            "spans":r.spans,"normalSpans":r.normal_spans,"normalProducts":r.normal_products,
            "domainCells":r.domain_cells,"unresolvedProducts":r.unresolved_products.len(),"reason":r.reason}));
    }
    let automatic = if args.get(4).is_some_and(|v| v == "pass") {
        use brep_core::source_wall_coverage as w;
        let domains = w::RetainedGapLimits {
            cells: 100000,
            spans: 100000,
            domain_cells: 1000000,
            cells_per_pair: 256,
            spans_per_pair: 512,
            domain_cells_per_pair: 10000,
            domain_cells_per_query: 256,
            tolerance_uv: 1e-8,
        };
        let mut mixed_domains = domains;
        mixed_domains.cells_per_pair = 1024;
        mixed_domains.spans_per_pair = 4096;
        let r = w::qualify_with_mixed_pair_pass(
            body,
            5.99,
            w::Limits {
                pairs: 378,
                plane_controls: 10000,
                normal_spans: 20000,
                gap: brep_core::source_face_gap::Limits {
                    cells: 100000,
                    spans: 100000,
                },
                max_sine_squared: 1e-6,
            },
            Vec::new(),
            w::NormalProductLimits {
                cells: 50000,
                cells_per_pair: 128,
            },
            w::FactoredNormalLimits {
                exact_work: 100000000,
                queries: 10000,
                cells: 10000,
                cells_per_pair: 128,
            },
            Vec::new(),
            Some(domains),
            Vec::new(),
            Some(w::MixedPairLimits {
                domains: mixed_domains,
                normal_spans: 100000,
                normal_spans_per_pair: 10000,
            }),
        )
        .unwrap();
        let d = r.mixed.unwrap();
        json!({"factoredQueries":r.factored_queries,"factoredExactWork":r.factored_exact_work,"wholeWallQualified":r.certificate.is_some(),"cells":d.cells,"spans":d.spans,
            "normalSpans":d.normal_spans,"domainCells":d.domain_cells,
            "pairs":d.pairs.iter().map(|p|json!({"faces":p.retained.faces,"qualified":p.retained.qualified,
                "reason":p.retained.reason,"cells":p.retained.cells,"normalProducts":p.normal_products,
                "unresolvedProducts":p.retained.unresolved_products.len()})).collect::<Vec<_>>()})
    } else {
        Value::Null
    };
    std::fs::write(
        &args[3],
        value_codec::to_string(&json!({"results":results,"automatic":automatic})).unwrap(),
    )
    .unwrap();
}
