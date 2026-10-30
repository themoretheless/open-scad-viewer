//! Source-body restoration and exact edge selection transport for the editor.
use super::{Result, Value, field};
use value_codec::json;
pub(crate) fn limits(v: &Value) -> Result<brep_core::source_body_restore::Limits> {
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
trait EditorCarrier: brep_core::source_exchange_endpoints::ExchangeCarrier + brep_core::source_shared_chord_map::ChordCarrier {
    fn endpoint_limits(&self, canonical:f64, angle:f64, budget:u64)->Result<Value>;
}
impl EditorCarrier for brep_core::source_shared_edge::SharedEdge {
    fn endpoint_limits(&self, canonical:f64, angle:f64, budget:u64)->Result<Value> {
        let d=brep_core::source_seam_tangency::inspect_endpoint_limits(self,canonical,angle,budget)?;
        Ok(json!({"canonical":d.canonical,"exactWork":d.exact_work,"reason":d.reason,
            "sides":d.sides.iter().map(|s|json!({"side":s.side,"boundaryEnd":s.boundary_end,"collapsedPoint":s.limits.collapsed_point,
                "components":s.limits.components,"sineSquaredBounds":s.limits.sine_squared_interval,"distinctTangentLimits":s.limits.distinct_tangent_limits})).collect::<Vec<_>>()}))
    }
}
impl EditorCarrier for brep_core::source_projective_edge::Edge {
    fn endpoint_limits(&self, canonical:f64, _angle:f64, _budget:u64)->Result<Value> {
        Ok(json!({"canonical":canonical,"exactWork":0,"reason":"projective-endpoint-limits-unproven","sides":[]}))
    }
}
fn restore_diagnostics<E>(reason:&str, incidence:&brep_core::source_shell_incidence::Report<E>,
    embedding:Option<&brep_core::source_shell_geometry::Report<E>>, volume:Option<&brep_core::source_volume::Report<E>>)->Value {
    json!({"reason":reason,
        "incidence":{"work":incidence.work_used,"rootChecks":incidence.root_checks,
            "driverCells":incidence.driver_cells,"uncertainPair":incidence.uncertain_pair},
        "embedding":embedding.map(|r|json!({"reason":r.reason,"pairs":r.pairs,
            "uncertainVertex":r.uncertain_vertex,"uncertainFace":r.uncertain_face,"nextPair":r.next_pair})),
        "volume":volume.map(|r|json!({"reason":r.reason,"bounds":r.signed_bounds,
            "cells":r.cells,"spans":r.spans,"domainCells":r.domain_cells,"uncertainFace":r.uncertain_face,"uncertainUv":r.uncertain_uv}))})
}
pub fn restore(v: Value) -> Result<Value> {
    if v["definition"]["kind"].as_str()==Some("projective-body") {
        restore_using::<brep_core::source_projective_edge::Edge>(v,|definition,config| {
            let mut r=brep_core::source_projective_body_restore::restore(definition,config)?;
            let diagnostics=restore_diagnostics(r.reason(),&r.incidence,r.embedding.as_ref(),r.volume.as_ref());
            Ok((r.volume.as_mut().and_then(|r|r.body.take()),diagnostics))
        })
    } else {
        restore_using::<brep_core::source_shared_edge::SharedEdge>(v,|definition,config| {
            let mut r=brep_core::source_body_restore::restore(definition,config)?;
            let diagnostics=restore_diagnostics(r.reason(),&r.incidence,r.embedding.as_ref(),r.volume.as_ref());
            Ok((r.volume.as_mut().and_then(|r|r.body.take()),diagnostics))
        })
    }
}
fn restore_using<E:EditorCarrier>(v:Value, restore_body:impl FnOnce(Value,brep_core::source_body_restore::Limits)
    ->Result<(Option<brep_core::source_volume::Body<E>>,Value)>)->Result<Value> {

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
        let endpoint_work: Option<u64> = if option["endpointLimitsExactWork"].is_null() {None}
            else {Some(field(option,"endpointLimitsExactWork")?)};
        if endpoint_work.is_some_and(|n|!(1..=100000000).contains(&n))
            || edge >= edge_count || !seam_limits.max_sine_squared.is_finite()
            || !(0. ..1.).contains(&seam_limits.max_sine_squared)
            || [
                seam_limits.cells,
                seam_limits.curve_spans,
                seam_limits.normal_spans,
            ]
            .iter()
            .any(|n| !(1..=100000).contains(n))
        {
            return Err(super::input(
                "Choose an owned source edge and bounded seam qualification limits",
            ));
        }
        Some((edge, seam_limits, endpoint_work))
    };
    let wall_request = if v["wallQualification"].is_null() {
        None
    } else {
        let o = &v["wallQualification"];
        let groups: [Vec<usize>; 2] = field(o, "groups")?;
        let face_count = definition["shell"]["regions"]
            .as_array()
            .map_or(0, Vec::len);
        let minimum: f64 = field(o, "minimumMm")?;
        let tolerance: f64 = field(o, "toleranceMm")?;
        let uv: f64 = field(o, "toleranceUv")?;
        let grid: usize = field(o, "grid")?;
        let attempts: usize = field(o, "maxAttempts")?;
        let w = &o["limits"];
        let gap = brep_core::source_face_gap::Limits {
            cells: field(w, "gapCells")?,
            spans: field(w, "gapSpans")?,
        };
        let chord = brep_core::source_material_chord::Limits {
            cells: field(w, "cells")?,
            domain_cells: field(w, "domainCells")?,
            normal_spans: field(w, "normalSpans")?,
            max_sine_squared: field(w, "maxSineSquared")?,
        };
        if groups.iter().any(|g| {
            g.is_empty()
                || g.iter()
                    .enumerate()
                    .any(|(i, f)| *f >= face_count || g[..i].contains(f))
        }) || groups[0].iter().any(|f| groups[1].contains(f))
            || [minimum, tolerance, uv]
                .iter()
                .any(|x| !x.is_finite() || *x <= 0.)
            || !(1..=8).contains(&grid)
            || !(1..=256).contains(&attempts)
            || [gap.cells, gap.spans, chord.normal_spans]
                .iter()
                .any(|n| !(1..=100000).contains(n))
            || !(1..=1000000).contains(&chord.cells)
            || !(1..=8000000).contains(&chord.domain_cells)
            || !chord.max_sine_squared.is_finite()
            || !(0. ..1.).contains(&chord.max_sine_squared)
        {
            return Err(super::input(
                "Choose disjoint owned wall face groups, positive tolerances and bounded search work",
            ));
        }
        Some((
            groups,
            minimum,
            tolerance,
            uv,
            grid,
            attempts,
            brep_core::source_material_wall::Limits { gap, chord },
        ))
    };
    let scan_request = if v["wallScan"].is_null() {
        None
    } else {
        let o = &v["wallScan"];
        let minimum: f64 = field(o, "minimumMm")?;
        let uv: f64 = field(o, "toleranceUv")?;
        let grid: usize = field(o, "grid")?;
        let attempts: usize = field(o, "maxAttempts")?;
        let w = &o["limits"];
        let work = brep_core::source_material_chord::Limits {
            cells: field(w, "cells")?,
            domain_cells: field(w, "domainCells")?,
            normal_spans: field(w, "normalSpans")?,
            max_sine_squared: field(w, "maxSineSquared")?,
        };
        if [minimum, uv].iter().any(|x| !x.is_finite() || *x <= 0.)
            || !(1..=8).contains(&grid)
            || !(1..=256).contains(&attempts)
            || !(1..=1000000).contains(&work.cells)
            || !(1..=8000000).contains(&work.domain_cells)
            || !(1..=100000).contains(&work.normal_spans)
            || !work.max_sine_squared.is_finite()
            || !(0. ..1.).contains(&work.max_sine_squared)
        {
            return Err(super::input(
                "Choose positive scan threshold and bounded original material work",
            ));
        }
        Some((minimum, uv, grid, attempts, work))
    };
    let coverage_request = if v["wallCoverage"].is_null() {
        None
    } else {
        let o = &v["wallCoverage"];
        let w = &o["limits"];
        let minimum: f64 = field(o, "minimumMm")?;
        let work = brep_core::source_wall_coverage::Limits {
            pairs: field(w, "pairs")?,
            plane_controls: field(w, "planeControls")?,
            normal_spans: field(w, "normalSpans")?,
            gap: brep_core::source_face_gap::Limits {
                cells: field(w, "gapCells")?,
                spans: field(w, "gapSpans")?,
            },
            max_sine_squared: field(w, "maxSineSquared")?,
        };
        if !minimum.is_finite()
            || minimum <= 0.
            || !(1..=100000).contains(&work.pairs)
            || !(1..=1000000).contains(&work.plane_controls)
            || [work.normal_spans, work.gap.cells, work.gap.spans]
                .iter()
                .any(|n| !(1..=100000).contains(n))
            || !work.max_sine_squared.is_finite()
            || !(0. ..1.).contains(&work.max_sine_squared)
        {
            return Err(super::input(
                "Choose positive whole-wall threshold and bounded continuous work",
            ));
        }
        let self_work = if o["adaptiveSelf"].is_null() {
            None
        } else {
            let a = &o["adaptiveSelf"];
            let limits = brep_core::source_wall_self_coverage::Limits {
                plane_controls: field(a, "planeControls")?,
                cells: field(a, "cells")?,
                spans: field(a, "spans")?,
                cells_per_face: field(a, "cellsPerFace")?,
                spans_per_face: field(a, "spansPerFace")?,
            };
            if !(1..=1000000).contains(&limits.plane_controls)
                || [
                    limits.cells,
                    limits.spans,
                    limits.cells_per_face,
                    limits.spans_per_face,
                ]
                .iter()
                .any(|n| !(1..=100000).contains(n))
            {
                return Err(super::input(
                    "Choose bounded adaptive original self-wall work",
                ));
            }
            let pole=if a["pole"].is_null(){None}else{
                let p=&a["pole"];let exact:u64=field(p,"exactWork")?;
                let extra=brep_core::source_wall_self_coverage::Limits{plane_controls:1,
                    cells:field(p,"cells")?,spans:field(p,"spans")?,cells_per_face:field(p,"cellsPerFace")?,spans_per_face:field(p,"spansPerFace")?};
                if !(1..=100000000).contains(&exact)||[extra.cells,extra.spans,extra.cells_per_face,extra.spans_per_face].iter().any(|n|!(1..=100000).contains(n)){
                    return Err(super::input("Choose bounded additional pole self-wall work"));
                }
                Some((extra,exact))
            };
            let refinement=if a["refinement"].is_null(){None}else{
                let p=&a["refinement"];
                let extra=brep_core::source_wall_self_coverage::Limits{plane_controls:1,
                    cells:field(p,"cells")?,spans:field(p,"spans")?,cells_per_face:field(p,"cellsPerFace")?,spans_per_face:field(p,"spansPerFace")?};
                if [extra.cells,extra.spans,extra.cells_per_face,extra.spans_per_face].iter().any(|n|!(1..=100000).contains(n)){
                    return Err(super::input("Choose bounded additional regular self-wall work"));
                }
                Some(extra)
            };
            Some((limits,pole,refinement))
        };
        let products=if o["normalProducts"].is_null() {
            brep_core::source_wall_coverage::NormalProductLimits{cells:0,cells_per_pair:0}
        }else{
            let p=&o["normalProducts"];
            let cells:usize=field(p,"cells")?;let cells_per_pair:usize=field(p,"cellsPerPair")?;
            if !(1..=100000).contains(&cells)||!(1..=100000).contains(&cells_per_pair) {
                return Err(super::input("Choose bounded normal product work"));
            }
            brep_core::source_wall_coverage::NormalProductLimits{cells,cells_per_pair}
        };
        let factors=if o["factoredNormals"].is_null() {
            brep_core::source_wall_coverage::FactoredNormalLimits{exact_work:0,queries:0,cells:0,cells_per_pair:0}
        }else{
            let f=&o["factoredNormals"];
            let exact_work:u64=field(f,"exactWork")?;let queries:usize=field(f,"queries")?;
            let cells:usize=field(f,"cells")?;let cells_per_pair:usize=field(f,"cellsPerPair")?;
            if !(1..=100_000_000).contains(&exact_work)
                || [queries,cells,cells_per_pair].iter().any(|n|!(1..=100000).contains(n)) {
                return Err(super::input("Choose bounded factored normal work"));
            }
            brep_core::source_wall_coverage::FactoredNormalLimits{exact_work,queries,cells,cells_per_pair}
        };
        let retained=if o["retainedGap"].is_null(){None}else{
            let r=&o["retainedGap"];
            let work=brep_core::source_wall_coverage::RetainedGapLimits{
                cells:field(r,"cells")?,spans:field(r,"spans")?,domain_cells:field(r,"domainCells")?,
                cells_per_pair:field(r,"cellsPerPair")?,spans_per_pair:field(r,"spansPerPair")?,
                domain_cells_per_pair:field(r,"domainCellsPerPair")?,domain_cells_per_query:field(r,"domainCellsPerQuery")?,tolerance_uv:field(r,"toleranceUv")?};
            if [work.cells,work.spans,work.cells_per_pair,work.spans_per_pair,work.domain_cells_per_pair,work.domain_cells_per_query].iter().any(|n|!(1..=100000).contains(n))
                ||!(1..=1000000).contains(&work.domain_cells)||!work.tolerance_uv.is_finite()||work.tolerance_uv<=0. {
                return Err(super::input("Choose positive bounded total and per-pair retained domain work"));
            }
            Some(work)
        };
        let mixed=if o["mixedProducts"].is_null(){None}else{
            let r=&o["mixedProducts"];
            let domains=brep_core::source_wall_coverage::RetainedGapLimits{
                cells:field(r,"cells")?,spans:field(r,"spans")?,domain_cells:field(r,"domainCells")?,
                cells_per_pair:field(r,"cellsPerPair")?,spans_per_pair:field(r,"spansPerPair")?,
                domain_cells_per_pair:field(r,"domainCellsPerPair")?,domain_cells_per_query:field(r,"domainCellsPerQuery")?,tolerance_uv:field(r,"toleranceUv")?};
            let normal_spans=field(r,"normalSpans")?;let normal_spans_per_pair=field(r,"normalSpansPerPair")?;
            if [normal_spans,normal_spans_per_pair,domains.cells,domains.spans,domains.cells_per_pair,domains.spans_per_pair,domains.domain_cells_per_pair,domains.domain_cells_per_query].iter().any(|n|!(1..=100000).contains(n))
                ||!(1..=1000000).contains(&domains.domain_cells)||!domains.tolerance_uv.is_finite()||domains.tolerance_uv<=0. {
                return Err(super::input("Choose bounded total and per-pair mixed wall work"));
            }
            Some(brep_core::source_wall_coverage::MixedPairLimits{domains,normal_spans,normal_spans_per_pair})
        };
        Some((minimum,work,self_work,products,factors,retained,mixed))
    };
    let (body,diagnostics)=restore_body(definition,limits(&config)?)?;
    let Some(body)=body.as_ref() else {
        return Ok(json!({"admitted":false,"sourceBody":null,"edges":[],"diagnostics":diagnostics}));
    };
    let seam_qualification = seam_request.map(|(edge, work, endpoint_work)| -> Result<Value> {
        let angle=work.max_sine_squared;
        let r = body.qualify_edge_tangency(edge, work)?;
        let endpoint_limits=if r.reason=="source-seam-endpoint-normal-unresolved"
            && let (Some(budget),Some([a,b]))=(endpoint_work,r.uncertain_canonical)
            && a==b && [0.,1.].contains(&a)
        {
            body.geometry().shell().edges()[edge].endpoint_limits(a,angle,budget)?
        }else {Value::Null};
        Ok(json!({"request":v["seamQualification"],"qualified":r.seam.is_some(),
            "endpointLimits":endpoint_limits,
            "sineSquaredBounds":r.seam.as_ref().map(|s|s.sine_squared_bounds()),
            "reason":r.reason,"cells":r.cells,"curveSpans":r.curve_spans,
            "normalSpans":r.normal_spans,"acceptedCells":r.accepted_cells,
            "uncertainCanonical":r.uncertain_canonical}),
            )
        })
        .transpose()?;
    let wall_qualification=wall_request.map(|(groups,minimum,tolerance,uv,grid,attempts,work)|->Result<Value>{
        let r=brep_core::source_material_wall::search_and_qualify(body,[&groups[0],&groups[1]],minimum,tolerance,uv,grid,attempts,work)?;
        let witness=r.certificate.as_ref().map(|c| {
            let chord=c.chord();
            let endpoints=chord.endpoints().map(|e|json!({"face":e.face,"parameter":e.parameter,"uv":e.uv,"worldMm":e.world_mm}));
            json!({"line":chord.line(),"lengthMm":chord.length_mm(),"endpoints":endpoints})
        });
        Ok(json!({"request":v["wallQualification"],"qualified":r.certificate.is_some(),"converged":r.converged,
            "intervalMm":r.certificate.as_ref().map(|c|c.interval_mm()),"reason":r.reason,"witness":witness,
            "clearance":{"reason":r.clearance.reason,"cells":r.clearance.cells,"spans":r.clearance.spans,
                "uncertainFaces":r.clearance.uncertain_faces,"uncertainUv":r.clearance.uncertain_uv},
            "search":{"attempts":r.search.attempts,"refused":r.search.refused,"candidatesExhausted":r.search.candidates_exhausted}}))
    }).transpose()?;
    let wall_scan=scan_request.map(|(minimum,uv,grid,attempts,work)|->Result<Value>{
        let r=brep_core::source_wall_scan::search(body,minimum,grid,attempts,uv,work)?;
        let witness=r.best().map(|c|{
            let endpoints=c.endpoints().map(|e|json!({"face":e.face,"parameter":e.parameter,"uv":e.uv,"worldMm":e.world_mm}));
            json!({"line":c.line(),"lengthMm":c.length_mm(),"endpoints":endpoints})
        });
        Ok(json!({"request":v["wallScan"],"thinFound":r.thin_witness().is_some(),"witness":witness,
            "attempts":r.attempts,"refused":r.refused,"facesVisited":r.faces_visited,"facesTotal":r.faces_total,
            "proposalsExhausted":r.proposals_exhausted,"wholeWallQualified":false}))
    }).transpose()?;
    let wall_coverage=coverage_request.map(|(minimum,work,self_work,products,factors,retained,mixed)|->Result<Value>{
        let (facts,self_report)=if let Some((limits,pole,refinement))=self_work {
            let mut report=brep_core::source_wall_self_coverage::qualify(body,minimum,work.max_sine_squared,limits)?;
            if let Some((extra,exact))=pole {
                brep_core::source_wall_self_coverage::extend_with_pole_facts(body,minimum,work.max_sine_squared,&mut report,extra,exact)?;
            }
            if let Some(extra)=refinement {
                brep_core::source_wall_self_coverage::refine_unqualified(body,minimum,work.max_sine_squared,&mut report,extra)?;
            }
            let diagnostics=json!({"faces":report.faces.iter().map(|f|json!({"face":f.face,"qualified":f.qualified,
                "reason":f.reason,"cells":f.cells,"spans":f.spans,"pending":f.pending,"uncertain":f.uncertain,"unresolvedPairs":f.unresolved_pairs,"endpointExchangeSymmetric":true})).collect::<Vec<_>>(),
                "planeControls":report.plane_controls,"cells":report.cells+report.pole_cells+report.refinement_cells,"spans":report.spans+report.pole_spans+report.refinement_spans,
                "refinementCells":report.refinement_cells,"refinementSpans":report.refinement_spans,"poleExactWork":report.pole_exact_work,"poleCells":report.pole_cells,"poleSpans":report.pole_spans,
                "allSelfPairsQualified":report.all_self_pairs_qualified()});
            (report.into_certificates(),diagnostics)
        }else{(Vec::new(),Value::Null)};
        let r=brep_core::source_wall_coverage::qualify_with_mixed_pair_pass(body,minimum,work,facts,products,factors,Vec::new(),retained,Vec::new(),mixed)?;
        let retained_report=r.retained_gap.as_ref().map(|d|json!({"cells":d.cells,"spans":d.spans,"domainCells":d.domain_cells,"domainQueries":d.domain_queries,"endpointQueries":d.endpoint_queries,
            "pairs":d.pairs.iter().map(|p|json!({"faces":p.faces,"qualified":p.qualified,"reason":p.reason,"cells":p.cells,"spans":p.spans,"domainCells":p.domain_cells,"domainQueries":p.domain_queries,"endpointQueries":p.endpoint_queries,"outsideProducts":p.outside_products,"unresolvedProducts":p.unresolved_products})).collect::<Vec<_>>()}));
        let mixed_report=r.mixed.as_ref().map(|d|json!({"cells":d.cells,"spans":d.spans,"domainCells":d.domain_cells,"normalSpans":d.normal_spans,
            "normalProducts":d.pairs.iter().map(|p|p.normal_products).sum::<usize>(),
            "domainQueries":d.pairs.iter().map(|p|p.retained.domain_queries).sum::<usize>(),
            "endpointQueries":d.pairs.iter().map(|p|p.retained.endpoint_queries).sum::<usize>(),
            "pairs":d.pairs.iter().map(|q|{let p=&q.retained;json!({"faces":p.faces,"qualified":p.qualified,"reason":p.reason,"cells":p.cells,"spans":p.spans,"domainCells":p.domain_cells,"domainQueries":p.domain_queries,"endpointQueries":p.endpoint_queries,"outsideProducts":p.outside_products,"unresolvedProducts":p.unresolved_products,"normalSpans":q.normal_spans,"normalProducts":q.normal_products})}).collect::<Vec<_>>()}));
        Ok(json!({"request":v["wallCoverage"],"wholeWallQualified":r.certificate.is_some(),
            "lowerMm":r.certificate.as_ref().map(|c|c.minimum_mm()),
            "enumerationComplete":r.enumeration_complete,"totalPairs":r.total_pairs,
            "pairs":r.pairs.iter().map(|p|json!({"faces":p.faces,"proven":p.proven,"reason":p.reason})).collect::<Vec<_>>(),
            "planeControls":r.plane_controls,"normalSpans":r.normal_spans,"normalProductCells":r.normal_product_cells,
            "factoredExactWork":r.factored_exact_work,"factoredQueries":r.factored_queries,
            "factoredProductCells":r.factored_product_cells,
            "cells":r.cells,"spans":r.spans,"reason":r.reason,"adaptiveSelf":self_report,"retainedGap":retained_report,"mixedProducts":mixed_report}))
    }).transpose()?;
    let step_exchange = if v["stepExchange"].is_null() {
        Value::Null
    } else {
        let option = &v["stepExchange"];
        let tolerance: f64 = field(option, "toleranceMm")?;
        let work = &option["limits"];
        let candidate = brep_core::source_exchange_step::prepare(
            body,
            tolerance,
            brep_core::source_exchange_endpoints::Limits {
                root_checks: field(work, "rootChecks")?,
                mapping_cells: field(work, "mappingCells")?,
                replay_mapping_per_use: field(work, "replayMappingPerUse")?,
                exact_work: field(work, "exactWork")?,
                driver_cells: field(work, "driverCells")?,
                spans: field(work, "spans")?,
                endpoints: field(work, "endpoints")?,
            },
            field(option, "trimWork")?,
        )?;
        match candidate {
            Some(c) => json!({"prepared":true,"request":option,"text":c.text,
                "endpointErrorUpper":c.endpoint_error_upper,"vertices":c.vertices,
                "edges":c.edges,"faces":c.faces}),
            None => json!({"prepared":false,"request":option,"text":null,
                "reason":"source-step-preparation-unproven"}),
        }
    };
    let shell = body.geometry().shell();
    let boundary_display = if display_segments == 0 {
        None
    } else {
        Some(brep_core::source_boundary_display::prepare(
            shell,
            display_segments,
            max_spans,
            100000,
        )?)
    };
    let edges = (0..shell.edges().len()).map(|index| -> Result<Value> {
        let r = body.edge_restriction(index)?;
        let uses = shell.uses()[index];
        let mut vertices = shell.vertices()[uses[0].face][uses[0].wire][uses[0].edge];
        if shell.edges()[index].restriction_reversed(0) { vertices.reverse(); }
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
        json!({"admitted":true,"sourceBody":E::body_definition(body)?,"edges":edges,"stepExchange":step_exchange,"seamQualification":seam_qualification,"wallQualification":wall_qualification,"wallScan":wall_scan,"wallCoverage":wall_coverage,
        "volume":body.volume(),"reverseOrientation":body.reverse_orientation(),
        "faceCount":shell.faces().len(),"poleCount":shell.poles().len(),"displayFaces":display_faces,"diagnostics":diagnostics}),
    )
}
#[cfg(test)]
#[path = "cad_source_body/tests/mod.rs"]
mod tests;
