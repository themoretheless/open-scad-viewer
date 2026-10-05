//! Source-body restoration and exact edge selection transport for the editor.
use super::{Result, Value, field};
use value_codec::json;
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
pub fn restore(v: Value) -> Result<Value> {
    let definition: Value = field(&v, "definition")?;
    let config: Value = field(&v, "limits")?;
    let max_spans: usize = field(&v, "endpointSpans")?;
    if !(1..=100000).contains(&max_spans) {
        return Err(super::input("Choose endpointSpans in 1..100000"));
    }
    let report = brep_core::source_body_restore::restore(definition, limits(&config)?)?;
    let diagnostics = json!({"reason":report.reason(),
        "incidence":{"work":report.incidence.work_used,"rootChecks":report.incidence.root_checks,
            "driverCells":report.incidence.driver_cells,"uncertainPair":report.incidence.uncertain_pair},
        "embedding":report.embedding.as_ref().map(|r|json!({"reason":r.reason,"pairs":r.pairs,
            "uncertainVertex":r.uncertain_vertex,"uncertainFace":r.uncertain_face,"nextPair":r.next_pair})),
        "volume":report.volume.as_ref().map(|r|json!({"reason":r.reason,"bounds":r.signed_bounds,
            "cells":r.cells,"spans":r.spans,"domainCells":r.domain_cells,"uncertainFace":r.uncertain_face,"uncertainUv":r.uncertain_uv}))});
    let Some(body) = report.body() else {
        return Ok(
            json!({"admitted":false,"sourceBody":null,"edges":[],"diagnostics":diagnostics}),
        );
    };
    let shell = body.geometry().shell();
    let edges = (0..shell.edges().len()).map(|index| -> Result<Value> {
        let r = body.edge_restriction(index)?;
        let uses = shell.uses()[index];
        let mut vertices = shell.vertices()[uses[0].face][uses[0].wire][uses[0].edge];
        if shell.edges()[index].reversed()[0] { vertices.reverse(); }
        Ok(json!({"index":index,"definition":r.definition(),"parameterBounds":r.parameter_bounds()?,
            "endpointBoxes":r.endpoint_boxes(max_spans)?,"vertices":vertices,
            "uses":uses.map(|a|[a.face,a.wire,a.edge])}))
    }).collect::<Result<Vec<_>>>()?;
    Ok(
        json!({"admitted":true,"sourceBody":body.definition()?,"edges":edges,
        "volume":body.volume(),"reverseOrientation":body.reverse_orientation(),
        "faceCount":shell.faces().len(),"poleCount":shell.poles().len(),"diagnostics":diagnostics}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    fn config(cells: usize) -> Value {
        json!({"shell":{"exactWork":100_000_000,"driverCells":10000,"regions":{
            "region":{"pairs":10000,"regionCells":10000,"domainCells":10000,"agreementCells":10000},
            "searchCells":10000,"pointChecks":10000,"mappingCells":10000,"driverCells":10000,
            "membershipCells":10000,"controls":10000,"windingCells":10000,"steps":64}},
            "embedding":{"toleranceUv":1e-8,"corners":10000,"spans":20000,"linearCells":10000,
                "exactWork":10000000,"driverCells":10000,"pairs":{"pairs":10000,"cells":10000,"domainCells":100000,"cellsPerPair":32,"domainCellsPerPair":512}},
            "volume":{"axis":2,"origin":0.,"absoluteError":0.25,"toleranceUv":1e-8,"cells":cells,"spans":100000,"domainCells":100000}})
    }
    #[test]
    fn json_dispatch_restores_body_and_preserves_exact_edge_addresses() {
        let spans = brep_core::linear_canal::construct(
            [[0., 0., 0.], [0., 0., 2.]],
            [0.5, 0.5],
            [1., 0., 0.],
            std::f64::consts::TAU,
        )
        .unwrap();
        let shell = brep_core::linear_canal::to_capped_source_shell(
            &spans,
            1e-7,
            1e-8,
            brep_core::trimmed_face_recipe::Limits {
                pairs: 10000,
                region_cells: 10000,
                domain_cells: 10000,
                agreement_cells: 10000,
            },
            100_000_000,
        )
        .unwrap()
        .shell
        .unwrap();
        let definition = json!({"version":1,"shell":shell.definition().unwrap(),"success":true,"volume":[1.,1.]});
        // Browser binary transport uses integer tags for integral numbers.
        fn wire_numbers(v: &mut Value) {
            match v {
                Value::Number(value_codec::Number::Float(n))
                    if n.fract() == 0.
                        && n.abs() <= 9007199254740991.
                        && !(*n == 0. && n.is_sign_negative()) =>
                {
                    *v = if *n >= 0. {
                        json!(*n as u64)
                    } else {
                        json!(*n as i64)
                    };
                }
                Value::Array(a) => a.iter_mut().for_each(wire_numbers),
                Value::Object(o) => o.values_mut().for_each(wire_numbers),
                _ => {}
            }
        }
        let mut wire_definition = definition.clone();
        wire_numbers(&mut wire_definition);

        let run = |cells| {
            let request = json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(cells),"endpointSpans":10000});
            let text = value_codec::to_string(&request).unwrap();
            let result = super::super::execute(&text);
            {
                let response = value_codec::from_str::<Value>(&result).unwrap();
                assert_eq!(response["ok"], json!(true), "{response:?}");
                response["value"].clone()
            }
        };
        let invalid = super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":definition.clone(),"limits":config(1),"endpointSpans":0}));
        assert!(invalid.is_err());
        let denied = run(1);
        assert_eq!(denied["admitted"], json!(false), "{denied:?}");
        assert!(denied["sourceBody"].is_null());
        assert_eq!(
            denied["diagnostics"]["reason"],
            json!("source-volume-initial-work-limit")
        );
        let restored = run(50000);
        if let Some(path) = std::env::var_os("CAD_SOURCE_BODY_FIXTURE_OUTPUT") {
            let request = json!({"op":"cad_source_body_restore","definition":definition.clone(),"limits":config(50000),"endpointSpans":10000});
            std::fs::write(path, value_codec::to_string(&request).unwrap()).unwrap();
        }
        assert_eq!(restored["admitted"], json!(true), "{restored:?}");
        assert_eq!(restored["sourceBody"]["shell"], definition["shell"]);
        assert_eq!(
            restored["edges"].as_array().unwrap().len(),
            shell.edges().len()
        );
        for (index, edge) in restored["edges"].as_array().unwrap().iter().enumerate() {
            assert_eq!(edge["definition"], shell.edges()[index].definition());
            assert_eq!(edge["index"], json!(index));
            assert_eq!(
                edge["uses"],
                json!(shell.uses()[index].map(|a| [a.face, a.wire, a.edge]))
            );
        }
        let oracle = 2. * std::f64::consts::PI / 3.;
        assert!(
            restored["volume"][0].as_f64().unwrap() <= oracle
                && oracle <= restored["volume"][1].as_f64().unwrap()
        );
    }
}
