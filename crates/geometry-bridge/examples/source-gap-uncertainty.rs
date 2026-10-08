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
fn endpoint_limits(edge: &brep_core::source_shared_edge::SharedEdge, canonical: [f64; 2]) -> Value {
    use nurbs_core::interval_eval::{Interval as I, evaluate_interval};
    let ranges = edge.ranges().unwrap_or_else(|| {
        std::array::from_fn(|side| {
            if edge.reversed()[side] ^ edge.uses()[side].reversed() {
                [[1., 1.], [0., 1.]]
            } else {
                [[0., 1.], [1., 1.]]
            }
        })
    });
    let mut sides = Vec::new();
    for side in 0..2 {
        let fragment = &edge.uses()[side];
        let c = fragment.curve();
        let s = fragment.surface();
        let d = c.domain();
        let first = I::point(ranges[side][0][0])
            .div(I::point(ranges[side][0][1]))
            .unwrap();
        let last = I::point(ranges[side][1][0])
            .div(I::point(ranges[side][1][1]))
            .unwrap();
        let normalized = I::new(canonical[0], canonical[1])
            .unwrap()
            .sub(first)
            .unwrap()
            .div_signed(last.sub(first).unwrap())
            .unwrap();
        let t = I::point(d[0])
            .add(
                normalized
                    .mul(I::point(d[1]).sub(I::point(d[0])).unwrap())
                    .unwrap(),
            )
            .unwrap()
            .intersect(d[0], d[1])
            .unwrap();
        let uv = evaluate_interval(c, t).unwrap();
        let natural = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let rect = std::array::from_fn(|axis| {
            let x = uv[axis]
                .intersect(natural[axis][0], natural[axis][1])
                .unwrap();
            [x.lo, x.hi]
        });
        let (regular, spans) = nurbs_core::normal_alignment::normal_bounds(s, rect, 10000).unwrap();
        let mut factors = Vec::new();
        for end in [false, true] {
            match nurbs_core::factored_surface_normals::prepare(s, end, 100000000) {
                Ok(r) => {
                    let bounds = r.certificate.as_ref().map(|c| c.bounds(rect).unwrap());
                    let u = natural[0][usize::from(end)];
                    let whole = [[u, u], natural[1]];
                    let whole_boundary = r.certificate.as_ref().map(|c| c.bounds(whole).unwrap());
                    let limits = r.certificate.as_ref().map(|c| c.boundary_limits(1e-6).unwrap());
                    let divergence = limits.map(|r| json!({"collapsedPoint":r.collapsed_point,"components":r.components,"sineSquaredBounds":r.sine_squared_interval,"distinctTangentLimits":r.distinct_tangent_limits}));
                    factors.push(json!({"end":end,"qualifiedFactor":r.certificate.is_some(),"work":r.work,"reason":r.reason,"endpointComponents":bounds,"wholeBoundaryComponents":whole_boundary,"boundaryLimits":divergence}));
                }
                Err(e) => factors.push(
                    json!({"end":end,"qualifiedFactor":false,"error":e.code,"message":e.message}),
                ),
            }
        }
        sides.push(json!({"side":side,"parameter":[t.lo,t.hi],"uv":rect,"regularComponents":regular,"normalSpans":spans,"factors":factors}));
    }
    json!({"canonical":canonical,"sides":sides,"scope":"Original limiting normal enclosures; no regular endpoint or seam certificate"})
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
    let mut retained_facts = Vec::new();
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
        let uv = r.uncertain_uv.expect("unresolved original rectangle");
        let shell = body.geometry().shell();
        let surfaces = [
            shell.faces()[a][0].edges()[0].surface(),
            shell.faces()[b][0].edges()[0].surface(),
        ];
        let bounds = [
            nurbs_core::surface_distance::rectangle_bounds(surfaces[0], uv[0]).unwrap(),
            nurbs_core::surface_distance::rectangle_bounds(surfaces[1], uv[1]).unwrap(),
        ];
        let gap = nurbs_core::surface_distance::enclosure_distance(&bounds[0], &bounds[1]).unwrap();
        let point_uv = uv.map(|rect| {
            rect.map(|axis| {
                let t = axis[0] * 0.5 + axis[1] * 0.5;
                [t, t]
            })
        });
        let points = [
            nurbs_core::surface_distance::rectangle_bounds(surfaces[0], point_uv[0]).unwrap(),
            nurbs_core::surface_distance::rectangle_bounds(surfaces[1], point_uv[1]).unwrap(),
        ];
        let point_gap =
            nurbs_core::surface_distance::enclosure_distance(&points[0], &points[1]).unwrap();
        let regions = shell.regions().expect("admitted original source regions");
        let memberships=[(a,uv[0]),(b,uv[1])].map(|(face,rect)|{
            let r=brep_core::source_contour_winding::classify(regions[face].loops(),rect,1e-8,10000).unwrap();
            json!({"location":format!("{:?}",r.location),"cells":r.cells,"endpointQueries":r.endpoint_queries,"reason":r.reason})
        });
        let retained = if args
            .get(4)
            .is_some_and(|v| v == "retained" || v == "retained-automatic")
        {
            let mut r = brep_core::source_retained_face_gap::qualify(
                body,
                [a, b],
                5.99,
                brep_core::source_retained_face_gap::Limits {
                    cells: 10000,
                    spans: 20000,
                    domain_cells: 100000,
                    domain_cells_per_query: 256,
                    tolerance_uv: 1e-8,
                },
            )
            .unwrap();
            assert!(r.cells <= 10000 && r.spans <= 20000 && r.domain_cells <= 100000);
            if let Some(c) = &r.certificate {
                assert!(std::ptr::eq(c.body(), body));
                assert_eq!(c.faces(), [a, b]);
                assert_eq!(c.lower_mm(), 5.99);
                assert!(r.unresolved_products.is_empty());
            } else {
                assert!(!r.unresolved_products.is_empty());
            }
            let qualified = r.certificate.is_some();
            if let Some(c) = r.certificate.take() {
                retained_facts.push(c);
            }
            json!({"qualified":qualified,"reason":r.reason,"cells":r.cells,"spans":r.spans,"domainCells":r.domain_cells,"domainQueries":r.domain_queries,"endpointQueries":r.endpoint_queries,"outsideProducts":r.outside_products,"unresolvedProducts":r.unresolved_products})
        } else {
            Value::Null
        };
        results.push(json!({"faces":[a,b],"reason":r.reason,"cells":r.cells,"spans":r.spans,"uncertainUv":r.uncertain_uv,"qualified":r.certificate.is_some(),"bounds":bounds,"gapInterval":[gap.0,gap.1],"midpointBounds":points,"midpointGapInterval":[point_gap.0,point_gap.1],"retainedDomain":memberships,"retainedGap":retained}));
    }
    let coverage = if !retained_facts.is_empty() {
        use brep_core::{source_wall_coverage as wall, source_wall_self_coverage as own};
        let mut self_facts = own::qualify(
            body,
            5.99,
            1e-6,
            own::Limits {
                plane_controls: 10000,
                cells: 50000,
                spans: 100000,
                cells_per_face: 8192,
                spans_per_face: 10000,
            },
        )
        .unwrap();
        own::extend_with_pole_facts(
            body,
            5.99,
            1e-6,
            &mut self_facts,
            own::Limits {
                plane_controls: 10000,
                cells: 100000,
                spans: 100000,
                cells_per_face: 4096,
                spans_per_face: 100000,
            },
            100000000,
        )
        .unwrap();
        own::refine_unqualified(
            body,
            5.99,
            1e-6,
            &mut self_facts,
            own::Limits {
                plane_controls: 10000,
                cells: 100000,
                spans: 100000,
                cells_per_face: 100000,
                spans_per_face: 100000,
            },
        )
        .unwrap();
        assert!(self_facts.all_self_pairs_qualified());
        let automatic = args.get(4).is_some_and(|v| v == "retained-automatic");
        let expected_faces: Vec<_> = retained_facts.iter().map(|c| c.faces()).collect();
        let r = wall::qualify_with_retained_gap_pass(
            body,
            5.99,
            wall::Limits {
                pairs: 378,
                plane_controls: 10000,
                normal_spans: 20000,
                gap: brep_core::source_face_gap::Limits {
                    cells: 100000,
                    spans: 100000,
                },
                max_sine_squared: 1e-6,
            },
            self_facts.into_certificates(),
            wall::NormalProductLimits {
                cells: 50000,
                cells_per_pair: 128,
            },
            wall::FactoredNormalLimits {
                exact_work: 100000000,
                queries: 10000,
                cells: 10000,
                cells_per_pair: 128,
            },
            if automatic {
                Vec::new()
            } else {
                retained_facts
            },
            if automatic {
                Some(wall::RetainedGapLimits {
                    cells: 100000,
                    spans: 100000,
                    domain_cells: 1000000,
                    cells_per_pair: 256,
                    spans_per_pair: 512,
                    domain_cells_per_pair: 10000,
                    domain_cells_per_query: 256,
                    tolerance_uv: 1e-8,
                })
            } else {
                None
            },
        )
        .unwrap();
        assert!(r.enumeration_complete);
        assert!(r.certificate.is_none());
        let unresolved = r.pairs.iter().filter(|p| !p.proven).count();
        assert!(unresolved <= 46);
        for faces in expected_faces {
            assert!(r.pairs.iter().any(|p| p.faces == faces && p.proven));
        }
        if let Some(d) = &r.retained_gap {
            assert!(d.cells <= 100000 && d.spans <= 100000 && d.domain_cells <= 1000000);
            assert_eq!(d.cells, d.pairs.iter().map(|p| p.cells).sum::<usize>());
            assert_eq!(
                d.domain_cells,
                d.pairs.iter().map(|p| p.domain_cells).sum::<usize>()
            );
            assert!(
                d.pairs
                    .iter()
                    .all(|p| p.qualified == p.unresolved_products.is_empty())
            );
        }
        for old in pairs.iter().filter(|p| p["proven"] == json!(true)) {
            assert!(
                r.pairs
                    .iter()
                    .any(|p| json!(p.faces) == old["faces"] && p.proven)
            );
        }
        json!({"wholeWallQualified":false,"unresolved":unresolved,"retainedGap":r.retained_gap.as_ref().map(|d|json!({"cells":d.cells,"spans":d.spans,"domainCells":d.domain_cells,"pairsAttempted":d.pairs.len(),"qualifiedPairs":d.pairs.iter().filter(|p|p.qualified).count()})),"pairs":r.pairs.iter().map(|p|json!({"faces":p.faces,"proven":p.proven,"reason":p.reason})).collect::<Vec<_>>()})
    } else {
        Value::Null
    };
    let mut shared_seams = Vec::new();
    if args
        .get(4)
        .is_some_and(|v| v == "seams" || v == "seams-refined" || v == "seams-limits")
    {
        for pair in pairs
            .iter()
            .filter(|p| p["reason"] == json!("source-wall-pair-unproven"))
        {
            let target = [
                pair["faces"][0].as_u64().unwrap() as usize,
                pair["faces"][1].as_u64().unwrap() as usize,
            ];
            for (edge, uses) in body.geometry().shell().uses().iter().enumerate() {
                let mut adjacent = uses.map(|u| u.face);
                adjacent.sort();
                if adjacent != target {
                    continue;
                }
                let source = &body.geometry().shell().edges()[edge];
                let tangent = match body.qualify_edge_tangency(
                    edge,
                    brep_core::source_seam_tangency::Limits {
                        max_sine_squared: 1e-6,
                        cells: 256,
                        curve_spans: 4096,
                        normal_spans: 4096,
                    },
                ) {
                    Ok(r) => {
                        json!({"qualified":r.seam.is_some(),"reason":r.reason,"cells":r.cells,"curveSpans":r.curve_spans,"normalSpans":r.normal_spans,"uncertainCanonical":r.uncertain_canonical,"sineSquaredBounds":r.seam.as_ref().map(|s|s.sine_squared_bounds())})
                    }
                    Err(e) => json!({"qualified":false,"error":e.code,"message":e.message}),
                };
                let refinement = if args.get(4).is_some_and(|v| v == "seams-refined")
                    && tangent["reason"] == json!("source-seam-work-limit")
                {
                    let r = body
                        .qualify_edge_tangency(
                            edge,
                            brep_core::source_seam_tangency::Limits {
                                max_sine_squared: 1e-6,
                                cells: 32768,
                                curve_spans: 100000,
                                normal_spans: 100000,
                            },
                        )
                        .unwrap();
                    json!({"qualified":r.seam.is_some(),"reason":r.reason,"cells":r.cells,"curveSpans":r.curve_spans,"normalSpans":r.normal_spans,"uncertainCanonical":r.uncertain_canonical,"sineSquaredBounds":r.seam.as_ref().map(|s|s.sine_squared_bounds())})
                } else {
                    Value::Null
                };
                let endpoint = if args.get(4).is_some_and(|v| v == "seams-limits")
                    && tangent["reason"] == json!("source-seam-endpoint-normal-unresolved")
                {
                    endpoint_limits(
                        source,
                        <[f64; 2]>::from_value(tangent["uncertainCanonical"].clone()).unwrap(),
                    )
                } else {
                    Value::Null
                };
                shared_seams.push(json!({"faces":target,"edge":edge,"worldDegree":source.world().degree,"worldControls":source.world().control_points.len(),"completeCanonicalCoverage":source.covers_complete_canonical_source(),"tangency":tangent,"refinement":refinement,"endpointLimits":endpoint}));
            }
        }
    }
    std::fs::write(&args[3],value_codec::to_string_pretty(&json!({"scope":"Original full chart gap obligations; retained trims not witnessed", "pairs":results,"coverage":coverage,"sharedSeams":shared_seams})).unwrap()).unwrap();
}
