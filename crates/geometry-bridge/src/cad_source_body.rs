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
    let display_segments: usize = if v["displaySegments"].is_null() {
        0
    } else {
        field(&v, "displaySegments")?
    };
    let edge_count = definition["shell"]["pairs"].as_array().map_or(0, Vec::len);
    if display_segments > 4096
        || edge_count
            .checked_mul(display_segments)
            .is_none_or(|n| n > 65536)
    {
        return Err(super::input(
            "Bound display to 4096 segments per edge and 65536 total",
        ));
    }
    let face_display = if v["faceDisplay"].is_null() {
        None
    } else {
        let option = &v["faceDisplay"];
        let divisions: usize = field(option, "divisions")?;
        let domain_cells: usize = field(option, "domainCellsPerFace")?;
        let tolerance_uv: f64 = field(option, "toleranceUv")?;
        let faces = definition["shell"]["regions"]
            .as_array()
            .map_or(0, Vec::len);
        if !(1..=64).contains(&divisions)
            || !(1..=100000).contains(&domain_cells)
            || !tolerance_uv.is_finite()
            || tolerance_uv <= 0.
            || faces
                .checked_mul(divisions * divisions)
                .is_none_or(|n| n > 65536)
            || faces.checked_mul(domain_cells).is_none_or(|n| n > 1000000)
        {
            return Err(super::input("Bound face display cells and domain work"));
        }
        Some((divisions, tolerance_uv, domain_cells))
    };
    let max_spans: usize = field(&v, "endpointSpans")?;
    if !(1..=100000).contains(&max_spans) {
        return Err(super::input("Choose endpointSpans in 1..100000"));
    }
    let seam_request = if v["seamQualification"].is_null() {
        None
    } else {
        let option = &v["seamQualification"];
        let edge: usize = field(option, "edge")?;
        let work = &option["limits"];
        let seam_limits = brep_core::source_seam_tangency::Limits {
            max_sine_squared: field(work, "maxSineSquared")?,
            cells: field(work, "cells")?,
            curve_spans: field(work, "curveSpans")?,
            normal_spans: field(work, "normalSpans")?,
        };
        if edge >= edge_count || !seam_limits.max_sine_squared.is_finite()
            || !(0. ..1.).contains(&seam_limits.max_sine_squared)
            || [seam_limits.cells, seam_limits.curve_spans, seam_limits.normal_spans]
                .iter().any(|n| !(1..=100000).contains(n))
        {
            return Err(super::input("Choose an owned source edge and bounded seam qualification limits"));
        }
        Some((edge, seam_limits))
    };
    let wall_request = if v["wallQualification"].is_null() { None } else {
        let o=&v["wallQualification"];
        let groups:[Vec<usize>;2]=field(o,"groups")?;
        let face_count=definition["shell"]["regions"].as_array().map_or(0,Vec::len);
        let minimum:f64=field(o,"minimumMm")?;
        let tolerance:f64=field(o,"toleranceMm")?;
        let uv:f64=field(o,"toleranceUv")?;
        let grid:usize=field(o,"grid")?;
        let attempts:usize=field(o,"maxAttempts")?;
        let w=&o["limits"];
        let gap=brep_core::source_face_gap::Limits{cells:field(w,"gapCells")?,spans:field(w,"gapSpans")?};
        let chord=brep_core::source_material_chord::Limits{cells:field(w,"cells")?,domain_cells:field(w,"domainCells")?,
            normal_spans:field(w,"normalSpans")?,max_sine_squared:field(w,"maxSineSquared")?};
        if groups.iter().any(|g|g.is_empty() || g.iter().enumerate().any(|(i,f)|*f>=face_count || g[..i].contains(f)))
            || groups[0].iter().any(|f|groups[1].contains(f))
            || [minimum,tolerance,uv].iter().any(|x|!x.is_finite() || *x<=0.)
            || !(1..=8).contains(&grid) || !(1..=256).contains(&attempts)
            || [gap.cells,gap.spans,chord.normal_spans].iter().any(|n|!(1..=100000).contains(n))
            || !(1..=1000000).contains(&chord.cells) || !(1..=8000000).contains(&chord.domain_cells)
            || !chord.max_sine_squared.is_finite() || !(0. ..1.).contains(&chord.max_sine_squared) {
            return Err(super::input("Choose disjoint owned wall face groups, positive tolerances and bounded search work"));
        }
        Some((groups,minimum,tolerance,uv,grid,attempts,brep_core::source_material_wall::Limits{gap,chord}))
    };
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
    let seam_qualification = seam_request.map(|(edge, work)| -> Result<Value> {
        let r = body.qualify_edge_tangency(edge, work)?;
        Ok(json!({"request":v["seamQualification"],"qualified":r.seam.is_some(),
            "sineSquaredBounds":r.seam.as_ref().map(|s|s.sine_squared_bounds()),
            "reason":r.reason,"cells":r.cells,"curveSpans":r.curve_spans,
            "normalSpans":r.normal_spans,"acceptedCells":r.accepted_cells,
            "uncertainCanonical":r.uncertain_canonical}))
    }).transpose()?;
    let wall_qualification=wall_request.map(|(groups,minimum,tolerance,uv,grid,attempts,work)|->Result<Value>{
        let r=brep_core::source_material_wall::search_and_qualify(body,[&groups[0],&groups[1]],minimum,tolerance,uv,grid,attempts,work)?;
        Ok(json!({"request":v["wallQualification"],"qualified":r.certificate.is_some(),"converged":r.converged,
            "intervalMm":r.certificate.as_ref().map(|c|c.interval_mm()),"reason":r.reason,
            "clearance":{"reason":r.clearance.reason,"cells":r.clearance.cells,"spans":r.clearance.spans,
                "uncertainFaces":r.clearance.uncertain_faces,"uncertainUv":r.clearance.uncertain_uv},
            "search":{"attempts":r.search.attempts,"refused":r.search.refused,"candidatesExhausted":r.search.candidates_exhausted}}))
    }).transpose()?;
    let step_exchange = if v["stepExchange"].is_null() {
        Value::Null
    } else {
        let option = &v["stepExchange"];
        let tolerance: f64 = field(option,"toleranceMm")?;
        let work = &option["limits"];
        let candidate = brep_core::source_exchange_step::prepare(body,tolerance,
            brep_core::source_exchange_endpoints::Limits {
                root_checks:field(work,"rootChecks")?,
                mapping_cells:field(work,"mappingCells")?,
                replay_mapping_per_use:field(work,"replayMappingPerUse")?,
                exact_work:field(work,"exactWork")?,
                driver_cells:field(work,"driverCells")?,
                spans:field(work,"spans")?,
                endpoints:field(work,"endpoints")?,
            },field(option,"trimWork")?)?;
        match candidate {
            Some(c) => json!({"prepared":true,"request":option,"text":c.text,
                "endpointErrorUpper":c.endpoint_error_upper,"vertices":c.vertices,
                "edges":c.edges,"faces":c.faces}),
            None => json!({"prepared":false,"request":option,"text":null,
                "reason":"source-step-preparation-unproven"}),
        }
    };
    let shell = body.geometry().shell();
    let boundary_display = if display_segments == 0 { None } else {
        Some(brep_core::source_boundary_display::prepare(shell, display_segments, max_spans, 100000)?)
    };
    let edges = (0..shell.edges().len()).map(|index| -> Result<Value> {
        let r = body.edge_restriction(index)?;
        let uses = shell.uses()[index];
        let mut vertices = shell.vertices()[uses[0].face][uses[0].wire][uses[0].edge];
        if shell.edges()[index].reversed()[0] { vertices.reverse(); }
        let display = if display_segments == 0 { None } else { Some(boundary_display.as_ref().unwrap().edges[index].segments.iter().map(|(range, points, bounds)| json!([range, points, bounds])).collect::<Vec<_>>()) };
        Ok(json!({"index":index,"displaySegments":display,"definition":r.definition(),"parameterBounds":r.parameter_bounds()?,
            "endpointBoxes":r.endpoint_boxes(max_spans)?,"vertices":vertices,
            "uses":uses.map(|a|[a.face,a.wire,a.edge])}))
    }).collect::<Result<Vec<_>>>()?;
    let display_faces = face_display.map(|(divisions,tolerance_uv,domain_cells)| -> Result<Vec<Value>> {
        shell.regions().unwrap().iter().enumerate().map(|(index,region)| -> Result<Value> {
            let preview=brep_core::source_region_display::prepare(region,divisions,tolerance_uv,domain_cells)?;
            let tiles=preview.tiles.into_iter().map(|tile| {
                let mut corners=tile.corners;
                if body.reverse_orientation() ^ (region.chart_winding()<0) {corners.reverse();}
                json!({"uv":tile.uv,"corners":corners})
            }).collect::<Vec<_>>();
            let surface=region.loops()[0][0].surface();
            let unresolved_boxes=preview.unresolved.iter().map(|uv| -> Result<Value> {
                use nurbs_core::interval_eval::{self,Interval};
                let bounds=interval_eval::evaluate_surface_interval(surface,Interval::new(uv[0][0],uv[0][1])?,Interval::new(uv[1][0],uv[1][1])?)?;
                Ok(json!(bounds.into_iter().map(|b|[b.lo,b.hi]).collect::<Vec<_>>()))
            }).collect::<Result<Vec<_>>>()?;
            Ok(json!({"index":index,"tiles":tiles,"unresolved":preview.unresolved,"unresolvedBoxes":unresolved_boxes,"outside":preview.outside,"domainCells":preview.domain_cells}))
        }).collect()
    }).transpose()?;
    Ok(
        json!({"admitted":true,"sourceBody":body.definition()?,"edges":edges,"stepExchange":step_exchange,"seamQualification":seam_qualification,"wallQualification":wall_qualification,
        "volume":body.volume(),"reverseOrientation":body.reverse_orientation(),
        "faceCount":shell.faces().len(),"poleCount":shell.poles().len(),"displayFaces":display_faces,"diagnostics":diagnostics}),
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
        let network = brep_core::source_boundary_network::inspect(&shell, 10000, 10000).unwrap();
        assert!(!network.vertices.is_empty());
        assert_eq!(
            network.vertices.iter().map(|v| v.ends.len()).sum::<usize>(),
            2 * shell.edges().len()
        );
        assert_eq!(
            network
                .vertices
                .iter()
                .map(|v| v.poles.len())
                .sum::<usize>(),
            shell.poles().len()
        );
        assert!(brep_core::source_boundary_network::inspect(&shell, 1, 10000).is_err());
        assert!(brep_core::source_boundary_network::inspect(&shell, 10000, 1).is_err());
        let recovered = brep_core::source_shell_restore::restore(
            shell.definition().unwrap(),
            &limits(&config(50000)).unwrap().shell,
        )
        .unwrap()
        .shell
        .unwrap();
        assert_eq!(
            network,
            brep_core::source_boundary_network::inspect(&recovered, 10000, 10000).unwrap()
        );
        for region in shell.regions().unwrap() {
            assert!(region.whole_chart_material());
            let preview =
                brep_core::source_region_display::prepare(region, 4, 1e-8, 10000).unwrap();
            assert_eq!(preview.tiles.len(), 16);
            assert_eq!(preview.domain_cells, 0);
            assert!(preview.unresolved.is_empty());
            for tile in preview.tiles {
                assert!(tile.corners.iter().flatten().all(|x| x.is_finite()));
            }
        }
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
            let request = json!({"op":"cad_source_body_restore","definition":wire_definition.clone(),"limits":config(cells),"endpointSpans":10000,"displaySegments":8,"faceDisplay":{"divisions":4,"toleranceUv":1e-8,"domainCellsPerFace":10000}});
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
        let seam_option = json!({"edge":0,"limits":{"maxSineSquared":1e-6,"cells":100,"curveSpans":100,"normalSpans":100}});
        let checked = super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
            "seamQualification":seam_option.clone()})).unwrap();
        assert_eq!(checked["admitted"], json!(true));
        let seam = &checked["seamQualification"];
        assert_eq!(seam["request"], seam_option);
        assert!(seam["qualified"].as_bool().is_some());
        assert!(seam["reason"].as_str().unwrap().starts_with("source-seam-"));
        assert!(seam["cells"].as_u64().unwrap() <= 100);
        assert!(seam["normalSpans"].as_u64().unwrap() <= 100);
        if seam["qualified"] == json!(true) {
            assert!(seam["uncertainCanonical"].is_null());
            assert!(seam["sineSquaredBounds"].as_array().is_some());
        } else {
            assert!(seam["sineSquaredBounds"].is_null());
        }
        for bad in [json!({"edge":999999,"limits":seam_option["limits"]}),
            json!({"edge":0,"limits":{"maxSineSquared":1e-6,"cells":0,"curveSpans":100,"normalSpans":100}})] {
            assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
                "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
                "seamQualification":bad})).is_err());
        }

        let wall_option=json!({"groups":[[0],[1]],"minimumMm":0.1,"toleranceMm":0.01,"toleranceUv":1e-8,
            "grid":1,"maxAttempts":1,"limits":{"gapCells":1,"gapSpans":100,"cells":100,
                "domainCells":100,"normalSpans":100,"maxSineSquared":1e-6}});
        let checked_wall=super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(50000),"endpointSpans":10000,
            "wallQualification":wall_option.clone()})).unwrap();
        let wall=&checked_wall["wallQualification"];
        assert_eq!(wall["request"],wall_option);
        assert_eq!(wall["qualified"],json!(false));assert_eq!(wall["converged"],json!(false));
        assert!(wall["intervalMm"].is_null());
        assert!(wall["clearance"]["cells"].as_u64().unwrap()<=1);
        assert!(wall["search"]["attempts"].as_u64().unwrap()<=1);
        for bad_groups in [json!([[0],[0]]),json!([[999999],[1]]),json!([[],[1]])] {
            let mut bad=wall_option.clone();bad["groups"]=bad_groups;
            assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
                "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
                "wallQualification":bad})).is_err());
        }
        let mut bad=wall_option.clone();bad["limits"]["cells"]=json!(0);
        assert!(super::super::dispatch(json!({"op":"cad_source_body_restore",
            "definition":wire_definition.clone(),"limits":config(1),"endpointSpans":10000,
            "wallQualification":bad})).is_err());
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
            let display = edge["displaySegments"].as_array().unwrap();
            assert_eq!(display.len(), 8);
            for segment in display {
                assert_eq!(segment.as_array().unwrap().len(), 3);
            }
            assert_eq!(
                edge["uses"],
                json!(shell.uses()[index].map(|a| [a.face, a.wire, a.edge]))
            );
        }
        let mut anchors = std::collections::BTreeMap::new();
        for edge in restored["edges"].as_array().unwrap() {
            let segments = edge["displaySegments"].as_array().unwrap();
            for end in 0..2 {
                let id = edge["vertices"][end].as_u64().unwrap();
                let point = segments[if end == 0 { 0 } else { segments.len() - 1 }][1][end].clone();
                if let Some(previous) = anchors.insert(id, point.clone()) {
                    assert_eq!(previous, point);
                }
            }
        }
        let faces = restored["displayFaces"].as_array().unwrap();
        assert_eq!(faces.len(), shell.faces().len());
        for face in faces {
            assert_eq!(face["tiles"].as_array().unwrap().len(), 16);
            assert!(face["unresolved"].as_array().unwrap().is_empty());
        }
        let fresh = brep_core::source_body_restore::restore(
            restored["sourceBody"].clone(), limits(&config(50000)).unwrap(),
        ).unwrap();
        let source_model = brep_core::source_body_topology::convert(fresh.body().unwrap(), 1e-7, 10000, 10000).unwrap();
        assert_eq!(source_model.definition(), &restored["sourceBody"]);
        let topology = source_model.topology();
        assert_eq!(topology.edges.len(), shell.edges().len()+shell.poles().len());
        assert_eq!(topology.edges.iter().filter(|e| e.degenerate).count(), shell.poles().len());
        for edge in &topology.edges {
            match &edge.curve {
                brep_core::source_body_topology::Carrier::Edge(_) => assert!(!edge.degenerate),
                brep_core::source_body_topology::Carrier::Pole(pole) => {
                    assert!(edge.degenerate);
                    assert_eq!(edge.vertices[0], edge.vertices[1]);
                    let bounds = topology.vertices[edge.vertices[0]].point.bounds;
                    assert_eq!(bounds, pole.point().map(|p| [p,p]));
                }
            }
        }
        let oracle = 2. * std::f64::consts::PI / 3.;
        assert!(
            restored["volume"][0].as_f64().unwrap() <= oracle
                && oracle <= restored["volume"][1].as_f64().unwrap()
        );
    }
}
