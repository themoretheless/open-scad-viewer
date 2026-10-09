    use super::*;
    use crate::nurbs_step_solid::freeform_cuboid_solid;

    fn append(target: &mut Model, source: Model) {
        let Model(source, _) = source;
        let (vo, eo, lo, fo, so) = (
            target.vertices.len(),
            target.edges.len(),
            target.loops.len(),
            target.faces.len(),
            target.shells.len(),
        );
        target.0.vertices.extend(source.vertices);
        target.0.edges.extend(source.edges.into_iter().map(|mut e| {
            e.vertices = [e.vertices[0] + vo, e.vertices[1] + vo];
            e
        }));
        target.0.loops.extend(source.loops.into_iter().map(|mut l| {
            for c in &mut l.coedges {
                c.edge += eo;
            }
            l
        }));
        target.0.faces.extend(source.faces.into_iter().map(|mut f| {
            f.outer += lo;
            for h in &mut f.holes {
                *h += lo;
            }
            f
        }));
        target
            .0
            .shells
            .extend(source.shells.into_iter().map(|mut s| {
                for f in &mut s.faces {
                    f.face += fo;
                }
                s
            }));
        target
            .0
            .bodies
            .extend(source.bodies.into_iter().map(|mut b| {
                b.outer_shell += so;
                for s in &mut b.inner_shells {
                    *s += so;
                }
                b
            }));
    }

    #[test]
    fn partial_holed_revolution_is_not_mistaken_for_a_periodic_band() {
        let loops = vec![
            crate::sketch::polygon_wire(vec![[3.,0.],[6.,0.],[6.,4.],[3.,4.]]).unwrap(),
            crate::sketch::polygon_wire(vec![[4.,1.],[4.,3.],[5.,3.],[5.,1.]]).unwrap(),
        ];
        let model=crate::revolve_region_angle(&loops,1e-7,90.).unwrap();
        assert!(!periodic_patch_grid_compatible(&model,4,1));
        let periodic=periodicized_step_v6(&model).unwrap();
        assert_eq!(value_codec::to_value(&periodic).unwrap(),value_codec::to_value(&model).unwrap());
        let (text,_,_)=export_step_v9(&model).unwrap();
        let (restored,_,_)=import_step_v9(&text).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.bodies.len(),1);
        assert_eq!(restored.faces.iter().filter(|face|face.holes.len()==1).count(),2);
    }

    #[test]
    fn product_assembly_frames_use_their_own_units() {
        let text = include_str!(
            "../../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step"
        )
        .replace("\r\n", "\n");
        let extra = "#700=CARTESIAN_POINT('',(1.,0.,0.));\n#701=AXIS2_PLACEMENT_3D('',#700,$,$);\n#702=CARTESIAN_POINT('',(100.,0.,0.));\n#703=AXIS2_PLACEMENT_3D('',#702,$,$);\n#704=ITEM_DEFINED_TRANSFORMATION('','',#701,#703);\n";
        let moved = text
            .replace(
                "#576,#13)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#8)",
                "#576,#13)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#704)",
            )
            .replace("ENDSEC;\nEND-ISO", &format!("{extra}ENDSEC;\nEND-ISO"));
        let (model, _, _) = import_step_v9(&moved).unwrap();
        let max = model
            .vertices
            .iter()
            .map(|v| v.point[0])
            .fold(f64::NEG_INFINITY, f64::max);
        assert!((max - (304.8 + 100. - 25.4)).abs() < 1e-8);
    }
    #[test]
    fn product_assembly_rejects_cycles_mismatched_products_and_geometry_containers() {
        let text = include_str!(
            "../../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step"
        )
        .replace("\r\n", "\n");
        let bad_item = text.replace(
            "SHAPE_REPRESENTATION('',(#7),#5)",
            "SHAPE_REPRESENTATION('',(#15),#5)",
        );
        assert!(
            import_step_v9(&bad_item)
                .unwrap_err()
                .message
                .contains("Unsupported geometry")
        );
        let bad_product = text.replace("'inch','inch','',#11,#582,$", "'inch','inch','',#11,#11,$");
        assert!(
            import_step_v9(&bad_product)
                .unwrap_err()
                .message
                .contains("endpoints disagree")
        );
        let extra = "#700=NEXT_ASSEMBLY_USAGE_OCCURRENCE('cycle','cycle','',#582,#11,$);\n#701=PRODUCT_DEFINITION_SHAPE('','',#700);\n#702=(REPRESENTATION_RELATIONSHIP('','',#13,#576)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#8)SHAPE_REPRESENTATION_RELATIONSHIP());\n#703=CONTEXT_DEPENDENT_SHAPE_REPRESENTATION(#702,#701);\n";
        let cycle = text.replace("ENDSEC;\nEND-ISO", &format!("{extra}ENDSEC;\nEND-ISO"));
        assert!(
            import_step_v9(&cycle)
                .unwrap_err()
                .message
                .contains("Cyclic")
        );
    }
    #[test]
    fn imports_product_assembly_containers_with_mixed_leaf_units() {
        let text = include_str!(
            "../../../../tests/fixtures/step-v6/self-authored-mixed-unit-product-assembly.step"
        );
        let (model, _, report) = import_step_v9(text).unwrap();
        assert_eq!(model.bodies.len(), 2);
        assert_eq!(report.occurrence_identities.len(), 2);
        let mut max = [f64::NEG_INFINITY; 3];
        for vertex in &model.vertices {
            for (axis, bound) in max.iter_mut().enumerate() {
                *bound = bound.max(vertex.point[axis]);
            }
        }
        for (actual, expected) in max.into_iter().zip([304.8, 76.2, 101.6]) {
            assert!((actual - expected).abs() < 1e-8);
        }
        model.validate().unwrap();
    }
    #[test]
    fn direct_roundtrip_preserves_topology_and_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        assert!(!text.contains("OSCAD_SOLID"));
        let (back, cert, report) = import_step_v3(&text).unwrap();
        assert_eq!(cert.capability, STEP_INTERCHANGE_V3_CAPABILITY);
        assert!(report.identity.preserved);
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.faces.len(),
                back.bodies.len()
            ),
            (8, 12, 6, 1)
        );
        assert_eq!(back.1.faces, model.1.faces);
    }

    #[test]
    fn direct_more_than_32_bodies_and_metadata_free_identity() {
        let mut model = Model::empty(1e-7).unwrap();
        for i in 0..33 {
            append(
                &mut model,
                freeform_cuboid_solid([i as f64 * 2., 0., 0.], [i as f64 * 2. + 1., 1., 1.])
                    .unwrap(),
            );
        }
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let (back, _, report) = import_step_v3(&text).unwrap();
        assert_eq!(back.bodies.len(), 33);
        assert!(report.identity.preserved);

        let stripped = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let (_, _, external) = import_step_v3(&stripped).unwrap();
        assert!(!external.identity.preserved);
        assert!(external.identity.created_count > 32);
    }

    #[test]
    fn entity_reorder_preserves_digest_bound_identity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 2., 3.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let mut rows = text
            .lines()
            .filter(|l| l.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let reordered = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .chain(rows)
            .collect::<Vec<_>>()
            .join("\n");
        let (back, _, report) = import_step_v3(&reordered).unwrap();
        assert!(report.identity.preserved);
        assert_eq!(back.1.edges, model.1.edges);
    }

    #[test]
    fn multiple_cavities_shared_edges_senses_and_metadata_mutation() {
        let mut model = Model::empty(1e-7).unwrap();
        append(
            &mut model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([1., 1., 1.], [3., 3., 3.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([6., 6., 6.], [8., 8., 8.]).unwrap(),
        );
        model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1, 2],
        }];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let (back, _, _) = import_step_v3(&text).unwrap();
        assert_eq!(back.bodies[0].inner_shells.len(), 2);
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.loops.len(),
                back.faces.len(),
                back.shells.len(),
                back.bodies.len()
            ),
            (24, 36, 18, 18, 3, 1)
        );
        let uses = back
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .fold(BTreeMap::new(), |mut m, c| {
                *m.entry(c.edge).or_insert(0usize) += 1;
                m
            });
        assert!(uses.values().all(|count| *count == 2));
        assert_eq!(
            back.loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.reversed)
                .count(),
            model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.reversed)
                .count()
        );

        let mutated = text.replacen("0.00000000000000000E0", "1.00000000000000000E-1", 1);
        let error = import_step_v3(&mutated).unwrap_err();
        assert!(matches!(
            error.code,
            "BREP_STEP_V3_REFUSED" | "BREP_INVALID_TOPOLOGY"
        ));
        let partial = text.replacen("OSCAD_TOPO/3|", "REMOVED_TOPO/3|", 1);
        assert!(
            import_step_v3(&partial)
                .unwrap_err()
                .message
                .contains("Partial")
        );
    }

    #[test]
    fn lexer_accepts_comments_case_escapes_d_exponents_and_forward_refs() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let text = text
            .replace("DATA;", "DATA;/* 'ignored; text' */")
            .replace("CARTESIAN_POINT", "cartesian_point")
            .replace("0.00000000000000000e0", "0.0D0")
            .replace("OpenSCAD Viewer", "OpenSCAD ''Viewer''");
        import_step_v3(&text).unwrap();
    }

    #[test]
    fn refuses_duplicate_missing_cycle_depth_and_unknown_reachable_geometry() {
        assert!(
            parse("#1=CARTESIAN_POINT('',(0.,0.,0.));#1=CARTESIAN_POINT('',(0.,0.,0.));").is_err()
        );
        let entities = parse("#1=PRODUCT('',#2);#2=PRODUCT('',#1);").unwrap();
        assert!(reachable(&entities, &[1]).is_err());
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        assert!(import_step_v3(&text.replacen("B_SPLINE_SURFACE_WITH_KNOTS", "PLANE", 1)).is_err());
        assert!(
            import_step_v3(&text.replace(
                "#1=CARTESIAN_POINT",
                "#1=CARTESIAN_POINT('',#999);#65000=CARTESIAN_POINT"
            ))
            .is_err()
        );
    }

    #[test]
    fn parses_self_authored_analytic_and_conversion_fixtures() {
        let tetra =
            include_str!("../../../../tests/fixtures/step-v3/self-authored-analytic-tetra.step");
        let (model, _, report) = import_step_v3(tetra).unwrap();
        assert_eq!(
            (
                model.vertices.len(),
                model.edges.len(),
                model.loops.len(),
                model.faces.len(),
                model.shells.len(),
                model.bodies.len()
            ),
            (4, 6, 4, 4, 1, 1)
        );
        assert!(!report.identity.preserved);
        let (roundtrip, _, _) = export_step_v3(&model).unwrap();
        let (roundtrip, _, _) = import_step_v3(&roundtrip).unwrap();
        roundtrip.validate().unwrap();

        let analytic =
            include_str!("../../../../tests/fixtures/step-v3/self-authored-analytic-units.step");
        let entities = parse(analytic).unwrap();
        let arc = curve(&entities, 20, 3, 1.).unwrap();
        let ellipse = curve(&entities, 21, 3, 1.).unwrap();
        assert_eq!(arc.degree, 2);
        assert_eq!(ellipse.control_points.len(), 9);
        assert_eq!(
            length_scale(&entities, &entities.keys().copied().collect(), true).unwrap(),
            1.
        );

        let inch = include_str!("../../../../tests/fixtures/step-v3/self-authored-inch-chain.stp");
        let entities = parse(inch).unwrap();
        assert!(
            (length_scale(&entities, &entities.keys().copied().collect(), true).unwrap() - 25.4)
                .abs()
                < 1e-12
        );

        let cyclic =
            include_str!("../../../../tests/fixtures/step-v3/self-authored-malformed-cycle.stp");
        let entities = parse(cyclic).unwrap();
        assert!(reachable(&entities, &[1]).is_err());
    }

    #[test]
    fn periodic_analytic_surfaces_have_specific_typed_refusals() {
        for ty in [
            "CYLINDRICAL_SURFACE",
            "CONICAL_SURFACE",
            "SPHERICAL_SURFACE",
            "TOROIDAL_SURFACE",
        ] {
            let text = format!("#1={ty}('',#2,1.);");
            let entities = parse(&text).unwrap();
            let error = surface(&entities, 1, 1.).unwrap_err();
            assert_eq!(error.code, "BREP_STEP_V3_REFUSED");
            assert!(error.message.contains("parameterization"));
        }
    }

    #[test]
    fn v4_maps_finite_analytic_carriers_and_endpoint_point_selectors() {
        let entities=parse("\
#1=CARTESIAN_POINT('',(0.,0.,0.));#2=DIRECTION('',(0.,0.,1.));#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=CONICAL_SURFACE('',#4,2.,0.2);#7=SPHERICAL_SURFACE('',#4,2.);#8=TOROIDAL_SURFACE('',#4,4.,1.);
#9=CARTESIAN_POINT('',(1.,0.,0.));#10=DIRECTION('',(1.,0.,0.));#11=VECTOR('',#10,1.);
#12=LINE('',#9,#11);#13=CARTESIAN_POINT('',(2.,0.,0.));#14=TRIMMED_CURVE('',#12,(#9),(#13),.T.,.PARAMETER.);
").unwrap();
        for id in 5..=8 {
            let s = surface_v4(&entities, id, 1.).unwrap();
            assert!(
                (s.knots_u[s.knots_u.len() - s.degree_u - 1] - std::f64::consts::TAU).abs() < 1e-12
            );
            s.validate().unwrap();
        }
        let c = curve(&entities, 14, 3, 1.).unwrap();
        assert_eq!(c.control_points[0], vec![1., 0., 0.]);
        assert_eq!(c.control_points[1], vec![2., 0., 0.]);
    }

    #[test]
    fn orphan_transform_does_not_move_direct_geometry() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let rows = "\
#60000=CARTESIAN_POINT('',(0.,0.,0.));
#60001=CARTESIAN_POINT('',(9.,8.,7.));
#60002=DIRECTION('',(0.,0.,1.));
#60003=DIRECTION('',(1.,0.,0.));
#60004=AXIS2_PLACEMENT_3D('',#60000,#60002,#60003);
#60005=AXIS2_PLACEMENT_3D('',#60001,#60002,#60003);
#60006=ITEM_DEFINED_TRANSFORMATION('orphan','',#60004,#60005);
";
        let with_orphan = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, report) = import_step_v3(&with_orphan).unwrap();
        assert_eq!(back.vertices[0].point, model.vertices[0].point);
        assert!(
            report
                .ignored_entities
                .contains(&"ITEM_DEFINED_TRANSFORMATION".into())
        );
    }

    #[test]
    fn only_representation_reachable_rigid_transform_moves_geometry() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v3(&model).unwrap();
        let entities = parse(&text).unwrap();
        let body = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "MANIFOLD_SOLID_BREP").then_some(*id))
            })
            .unwrap();
        let rows = format!(
            "\
#60000=CARTESIAN_POINT('',(0.,0.,0.));
#60001=CARTESIAN_POINT('',(9.,8.,7.));
#60002=DIRECTION('',(0.,0.,1.));
#60003=DIRECTION('',(1.,0.,0.));
#60004=AXIS2_PLACEMENT_3D('',#60000,#60002,#60003);
#60005=AXIS2_PLACEMENT_3D('',#60001,#60002,#60003);
#60006=ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005);
#60007=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body},#60006),#60008);
#60008=GEOMETRIC_REPRESENTATION_CONTEXT(3);
"
        );
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, _) = import_step_v3(&placed).unwrap();
        for i in 0..3 {
            assert!(
                (back.vertices[0].point[i] - model.vertices[0].point[i] - [9., 8., 7.][i]).abs()
                    < 1e-9
            );
        }

        let nonrigid = placed.replace(
            "ITEM_DEFINED_TRANSFORMATION('reachable','',#60004,#60005)",
            "CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#60002,#60003,#60000,2.,$)",
        );
        assert!(
            import_step_v3(&nonrigid)
                .unwrap_err()
                .message
                .contains("nonuniform")
        );
    }

    #[test]
    fn aggregate_parser_and_graph_budgets_refuse() {
        assert!(
            lex(&" ".repeat(MAX_BYTES + 1))
                .unwrap_err()
                .message
                .contains("16 MiB")
        );
        let nested = format!("#1=PRODUCT('',{});", "(".repeat(MAX_PARSE_DEPTH + 1))
            + &")".repeat(MAX_PARSE_DEPTH + 1);
        assert!(parse(&nested).unwrap_err().message.contains("depth"));
        let mut chain = String::new();
        for i in 1..=MAX_GRAPH_DEPTH + 2 {
            if i == MAX_GRAPH_DEPTH + 2 {
                chain.push_str(&format!("#{i}=PRODUCT('end');"));
            } else {
                chain.push_str(&format!("#{i}=PRODUCT('',#{});", i + 1));
            }
        }
        let entities = parse(&chain).unwrap();
        assert!(
            reachable(&entities, &[1])
                .unwrap_err()
                .message
                .contains("depth")
        );
        assert!(enforce_output_limit(&"x".repeat(MAX_OUTPUT_BYTES)).is_ok());
        assert!(
            enforce_output_limit(&"x".repeat(MAX_OUTPUT_BYTES + 1))
                .unwrap_err()
                .message
                .contains("16 MiB")
        );

        let mut instances = String::new();
        for id in 1..=MAX_INSTANCES + 1 {
            instances.push_str(&format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));"))
        }
        assert!(parse(&instances).unwrap_err().message.contains("65536"));
    }

    #[test]
    fn v5_emits_selected_ap242_context_and_roundtrips() {
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (text, certificate, report) = export_step_v5(&model).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V5_CAPABILITY);
        assert!(text.contains("FILE_NAME("));
        assert!(text.contains("SHAPE_DEFINITION_REPRESENTATION("));
        assert!(text.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT"));
        assert!(!text.contains("B_SPLINE_SURFACE_WITH_KNOTS('',1,1"));
        assert!(report.metadata_loss.is_empty());
        let (back, _, import_report) = import_step_v5(&text).unwrap();
        assert_eq!(
            (
                back.vertices.len(),
                back.edges.len(),
                back.faces.len(),
                back.bodies.len()
            ),
            (8, 12, 6, 1)
        );
        assert!(import_report.identity.preserved);

        let no_selection = text.replace(
            "SHAPE_DEFINITION_REPRESENTATION",
            "PRESENTATION_LAYER_ASSIGNMENT",
        );
        assert!(
            import_step_v5(&no_selection)
                .unwrap_err()
                .message
                .contains("selection")
        );
        let orphan_context = text.replace("GLOBAL_UNIT_ASSIGNED_CONTEXT", "ORPHAN_UNIT_CONTEXT");
        assert!(
            import_step_v5(&orphan_context)
                .unwrap_err()
                .message
                .contains("length unit")
        );
        assert!(import_step_v5("ISO-10303-21;DATA;ENDSEC;END-ISO-10303-21;").is_err());
        assert!(
            import_step_v5(&text.replace(
                "AP242_MANAGED_MODEL_BASED_3D_ENGINEERING",
                "AUTOMOTIVE_DESIGN"
            ))
            .unwrap_err()
            .message
            .contains("AP242")
        );
        assert!(
            import_step_v5(&text.replacen("DATA;", "DATA;\nDATA;", 1))
                .unwrap_err()
                .message
                .contains("exactly one DATA")
        );
        let comment_spoof = text.replacen("DATA;", "/*\nDATA;\n*/\nNOT_DATA;", 1);
        assert!(
            import_step_v5(&comment_spoof)
                .unwrap_err()
                .message
                .contains("DATA")
        );
        let string_spoof = text.replace("FILE_SCHEMA((", "NOT_SCHEMA((").replace(
            "OpenSCAD Viewer retained AP242 B-rep",
            "FILE_SCHEMA(('AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'))",
        );
        assert!(
            import_step_v5(&string_spoof)
                .unwrap_err()
                .message
                .contains("HEADER requires")
        );

        let mut rows = data_payload(&text)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let data_start = text.find("DATA;").unwrap() + 5;
        let data_end = text[data_start..].find("ENDSEC;").unwrap() + data_start;
        let reordered = format!(
            "{}\n{}\n{}",
            &text[..data_start],
            rows.join("\n"),
            &text[data_end..]
        );
        assert_eq!(import_step_v5(&reordered).unwrap().0.bodies.len(), 1);
        let encoded = text.replace("'model.step'", "'\\X2\\006D006F00640065006C\\X0\\.step'");
        assert_eq!(import_step_v5(&encoded).unwrap().0.bodies.len(), 1);

        let metre = text.replace("SI_UNIT(.MILLI.,.METRE.)", "SI_UNIT($,.METRE.)");
        let (metre_model, _, _) = import_step_v5(&metre).unwrap();
        assert!(
            metre_model
                .vertices
                .iter()
                .any(|vertex| vertex.point[0] > 1000.)
        );

        let parsed = parse(&text).unwrap();
        let unit = parsed
            .iter()
            .find_map(|(id, entity)| match &entity.value {
                Value::List(parts)
                    if parts
                        .iter()
                        .any(|part| matches!(part,Value::Call(name,_)if name=="SI_UNIT")) =>
                {
                    Some(*id)
                }
                _ => None,
            })
            .unwrap();
        let inch_rows = "\
#60000=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));
#60001=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.0254),#60000);
#60002=(CONVERSION_BASED_UNIT('INCH',#60001)LENGTH_UNIT()NAMED_UNIT(*));
";
        let inch = text
            .replace(
                &format!("GLOBAL_UNIT_ASSIGNED_CONTEXT((#{unit}))"),
                "GLOBAL_UNIT_ASSIGNED_CONTEXT((#60002))",
            )
            .replacen(
                "ENDSEC;\nEND-ISO",
                &format!("{inch_rows}ENDSEC;\nEND-ISO"),
                1,
            );
        let (inch_model, _, _) = import_step_v5(&inch).unwrap();
        assert!(
            inch_model
                .vertices
                .iter()
                .any(|vertex| vertex.point[0] > 50. && vertex.point[0] < 51.)
        );
    }

    #[test]
    fn v5_uses_rational_complexes_and_oriented_void_shells() {
        let rational = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
            weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
            periodic: false,
        };
        let mut writer = Writer::new();
        emit_curve_v5(&mut writer, &rational);
        let row = writer.rows.values().last().unwrap();
        assert!(row.contains("RATIONAL_B_SPLINE_CURVE"));
        assert!(row.contains("B_SPLINE_CURVE_WITH_KNOTS(("));

        let mut model = Model::empty(1e-7).unwrap();
        append(
            &mut model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        model.rebuild_topology_ids();
        model.validate().unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("ORIENTED_CLOSED_SHELL('',*,#"));
        let (back, _, _) = import_step_v5(&text).unwrap();
        assert_eq!(back.bodies[0].inner_shells.len(), 1);
    }

    #[test]
    fn v5_orders_distinct_occurrence_transforms_and_refuses_cycle_or_ambiguity() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(&text).unwrap();
        let root = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .unwrap();
        let (_, root_args) = call(&entities, root).unwrap();
        let body = list(&root_args[1], "items")
            .unwrap()
            .iter()
            .find_map(|value| match value {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .unwrap();
        let context = one_ref(&root_args[2], "context").unwrap();
        let rows=format!("\
#60000=CARTESIAN_POINT('',(10.,0.,0.));
#60001=DIRECTION('',(1.,0.,0.));
#60002=DIRECTION('',(0.,1.,0.));
#60003=DIRECTION('',(0.,0.,1.));
#60004=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',#60001,#60002,#60000,1.,#60003);
#60005=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#60006=(REPRESENTATION_RELATIONSHIP('','',#{root},#60005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());
");
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        let (back, _, _) = import_step_v5(&placed).unwrap();
        assert_eq!(back.bodies.len(), 2);
        let xs = back
            .vertices
            .iter()
            .map(|vertex| vertex.point[0])
            .collect::<Vec<_>>();
        assert!(xs.iter().any(|x| *x < 2.));
        assert!(xs.iter().any(|x| *x >= 10.));

        let cycle=placed.replacen("ENDSEC;\nEND-ISO",&format!(
            "#60007=(REPRESENTATION_RELATIONSHIP('','',#60005,#{root})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());\nENDSEC;\nEND-ISO"),1);
        assert!(
            import_step_v5(&cycle)
                .unwrap_err()
                .message
                .contains("Cycle")
        );
        let ambiguous=placed.replacen("ENDSEC;\nEND-ISO",&format!(
            "#60007=(REPRESENTATION_RELATIONSHIP('','',#{root},#60005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#60004)SHAPE_REPRESENTATION_RELATIONSHIP());\nENDSEC;\nEND-ISO"),1);
        assert!(
            import_step_v5(&ambiguous)
                .unwrap_err()
                .message
                .contains("Ambiguous")
        );
    }

    #[test]
    fn v5_roundtrips_rational_analytic_bodies_and_sense_combinations() {
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(3., 1., 4.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v5(&model).unwrap();
            assert!(text.contains("RATIONAL_B_SPLINE_"));
            let (back, _, _) = import_step_v5(&text).unwrap();
            assert_eq!(back.bodies.len(), 1);
            assert_eq!(back.faces.len(), model.faces.len());
            back.validate().unwrap();
        }
        let model = crate::cylinder(2., 3.).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("ORIENTED_EDGE('',*,*,#") && text.contains(",.F.);"));
        assert!(text.contains("ADVANCED_FACE('',("));
    }

    #[test]
    fn v6_roundtrips_certified_pole_boundaries() {
        for model in [
            crate::sphere(2.).unwrap(),
            crate::frustum(2., 0., 3.).unwrap(),
        ] {
            let degenerate = model.edges.iter().filter(|edge| edge.degenerate).count();
            if degenerate > 0 {
                assert!(export_step_v5(&model).is_err())
            }
            let (text, certificate, _) = export_step_v6(&model).unwrap();
            assert_eq!(certificate.capability, STEP_INTERCHANGE_V6_CAPABILITY);
            let (back, _, _) = import_step_v6(&text).unwrap();
            assert_eq!(
                back.edges.iter().filter(|edge| edge.degenerate).count(),
                if model.faces.len() == 8 {
                    2
                } else {
                    degenerate
                }
            );
            back.validate().unwrap();
        }
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(2., 1., 3.).unwrap(),
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v6(&model).unwrap();
            assert!(text.contains("SEAM_CURVE("));
            import_step_v6(&text).unwrap().0.validate().unwrap();
        }
        let mut void_model = Model::empty(1e-7).unwrap();
        append(
            &mut void_model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut void_model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        void_model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        void_model.rebuild_topology_ids();
        void_model.validate().unwrap();
        let (void_text, _, _) = export_step_v6(&void_model).unwrap();
        let same_oriented = void_text
            .lines()
            .map(|line| {
                let line = if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                };
                if line.contains("ORIENTED_CLOSED_SHELL(") {
                    line.replacen(",.F.)", ",.T.)", 1)
                } else {
                    line
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert_eq!(
            import_step_v6(&same_oriented).unwrap().0.bodies[0]
                .inner_shells
                .len(),
            1
        );
    }

    #[test]
    fn v7_requires_whole_domain_boundary_identities_for_periodic_carriers() {
        for model in [
            crate::cylinder(2., 3.).unwrap(),
            crate::frustum(2., 1., 3.).unwrap(),
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
        ] {
            let (text, _, _) = export_step_v7(&model).unwrap();
            let (back, import_certificate, _) = import_step_v7(&text).unwrap();
            assert!(
                import_certificate
                    .notes
                    .contains(&"whole_domain_rational_boundary_and_constant_pole_identity")
            );
            back.validate().unwrap();
        }
    }

    #[test]
    fn v8_carries_whole_domain_regularity_in_the_topology_certificate() {
        let expected = [
            (crate::cylinder(2., 3.).unwrap(), "cylinder", 0),
            (crate::frustum(2., 1., 3.).unwrap(), "cone", 0),
            (crate::frustum(2., 0., 3.).unwrap(), "cone", 1),
            (crate::sphere(2.).unwrap(), "sphere", 2),
            (crate::torus(4., 1.).unwrap(), "torus", 0),
        ];
        for (model, carrier, collapsed) in expected {
            let (text, export_certificate, _) = export_step_v8(&model).unwrap();
            let row = export_certificate
                .regularity
                .iter()
                .find(|row| row.carrier == carrier)
                .unwrap();
            assert!(row.denominator_lower_bound > 0.);
            assert_eq!(row.collapsed_boundaries.len(), collapsed);
            assert!(row.lifted_periods[0] > 0.);
            if carrier == "torus" {
                assert!(row.lifted_periods[1] > 0.)
            }
            let (back, import_certificate, _) = import_step_v8(&text).unwrap();
            assert_eq!(import_certificate.coupled_sense_cases, 128);
            assert_eq!(
                import_certificate
                    .regularity
                    .iter()
                    .find(|row| row.carrier == carrier)
                    .unwrap()
                    .identity,
                row.identity
            );
            back.validate().unwrap();
        }
    }

    fn toggle_first_entity_boolean(text: &str, entity: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if !changed && line.contains(&format!("={entity}")) {
                    changed = true;
                    if line.contains(",.T.);") {
                        line.replacen(",.T.);", ",.F.);", 1)
                    } else {
                        line.replacen(",.F.);", ",.T.);", 1)
                    }
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn reverse_first_shell_members(text: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if changed || !line.contains("=CLOSED_SHELL") {
                    return line.to_string();
                }
                let Some(open) = line.find(",(") else {
                    return line.to_string();
                };
                let Some(close) = line[open + 2..].find("))").map(|index| index + open + 2) else {
                    return line.to_string();
                };
                let mut members = line[open + 2..close].split(',').collect::<Vec<_>>();
                members.reverse();
                changed = true;
                format!(
                    "{}{}{}",
                    &line[..open + 2],
                    members.join(","),
                    &line[close..]
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn reverse_first_seam_pcurves(text: &str) -> String {
        let mut changed = false;
        text.lines()
            .map(|line| {
                if changed || !line.contains("=SEAM_CURVE") {
                    return line.to_string();
                }
                let Some(open) = line.find(",(#") else {
                    return line.to_string();
                };
                let Some(close) = line[open + 2..].find(')').map(|index| index + open + 2) else {
                    return line.to_string();
                };
                let mut members = line[open + 2..close].split(',').collect::<Vec<_>>();
                members.reverse();
                changed = true;
                format!(
                    "{}{}{}",
                    &line[..open + 2],
                    members.join(","),
                    &line[close..]
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn topology_signature(
        model: &Model,
    ) -> (usize, usize, usize, usize, usize, usize, Vec<[u64; 3]>) {
        let mut vertices = model
            .vertices
            .iter()
            .map(|vertex| vertex.point.map(f64::to_bits))
            .collect::<Vec<_>>();
        vertices.sort();
        (
            model.vertices.len(),
            model.edges.len(),
            model.loops.len(),
            model.faces.len(),
            model.shells.len(),
            model.bodies.len(),
            vertices,
        )
    }

    #[test]
    fn v8_exhausts_all_coupled_sense_masks_with_equivalence_or_refusal() {
        let mut void_model = Model::empty(1e-7).unwrap();
        append(
            &mut void_model,
            freeform_cuboid_solid([0., 0., 0.], [10., 10., 10.]).unwrap(),
        );
        append(
            &mut void_model,
            freeform_cuboid_solid([2., 2., 2.], [4., 4., 4.]).unwrap(),
        );
        void_model.0.bodies = vec![Body {
            outer_shell: 0,
            inner_shells: vec![1],
        }];
        void_model.rebuild_topology_ids();
        void_model.validate().unwrap();
        let fixtures = [
            crate::sphere(2.).unwrap(),
            crate::torus(4., 1.).unwrap(),
            void_model,
        ];
        let exported = fixtures
            .iter()
            .map(|model| export_step_v8(model).unwrap().0)
            .collect::<Vec<_>>();
        let baselines = exported
            .iter()
            .map(|text| topology_signature(&import_step_v8(text).unwrap().0))
            .collect::<Vec<_>>();
        let mut refusals = 0usize;
        let mut equivalents = 0usize;
        for mask in 0usize..128 {
            for (fixture, text) in exported.iter().enumerate() {
                let mut candidate = text.clone();
                if mask & 1 != 0 {
                    candidate = reverse_first_seam_pcurves(&candidate);
                }
                if mask & 2 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "EDGE_CURVE")
                }
                if mask & 4 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ORIENTED_EDGE")
                }
                if mask & 8 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "FACE_OUTER_BOUND");
                }
                if mask & 16 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ADVANCED_FACE")
                }
                if mask & 32 != 0 {
                    candidate = reverse_first_shell_members(&candidate)
                }
                if mask & 64 != 0 {
                    candidate = toggle_first_entity_boolean(&candidate, "ORIENTED_CLOSED_SHELL");
                }
                match import_step_v8(&candidate) {
                    Ok((model, certificate, _)) => {
                        assert!(certificate.complete);
                        assert_eq!(topology_signature(&model), baselines[fixture]);
                        equivalents += 1;
                    }
                    Err(_) => refusals += 1,
                }
            }
        }
        assert_eq!(equivalents + refusals, 128 * fixtures.len());
        assert!(equivalents > 0 && refusals > 0);
        println!(
            "step-v8-sense-matrix cases={} equivalents={equivalents} refusals={refusals}",
            128 * fixtures.len()
        );
        let same_sense = toggle_first_entity_boolean(&exported[0], "EDGE_CURVE");
        assert!(
            import_step_v8(&same_sense)
                .unwrap_err()
                .message
                .contains("same_sense")
        );
        let wrong_void = toggle_first_entity_boolean(&exported[2], "ORIENTED_CLOSED_SHELL");
        assert!(
            import_step_v8(&wrong_void)
                .unwrap_err()
                .message
                .contains("must oppose")
        );
    }

    #[test]
    fn v6_applies_nonuniform_occurrence_affine_and_refuses_singular() {
        let model = freeform_cuboid_solid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let (text, _, _) = export_step_v5(&model).unwrap();
        let text = text
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(data_payload(&text).unwrap()).unwrap();
        let root = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_BREP_SHAPE_REPRESENTATION").then_some(*id))
            })
            .unwrap();
        let (_, root_args) = call(&entities, root).unwrap();
        let body = list(&root_args[1], "items")
            .unwrap()
            .iter()
            .find_map(|value| match value {
                Value::Ref(id) => Some(*id),
                _ => None,
            })
            .unwrap();
        let context = one_ref(&root_args[2], "context").unwrap();
        let product_definition = entities
            .iter()
            .find_map(|(id, _)| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "PRODUCT_DEFINITION").then_some(*id))
            })
            .unwrap();
        let (_, definition_args) = call(&entities, product_definition).unwrap();
        let formation = one_ref(&definition_args[2], "formation").unwrap();
        let definition_context = one_ref(&definition_args[3], "definition context").unwrap();
        let rows=format!("\
#61000=CARTESIAN_POINT('',(10.,20.,30.));
#61001=DIRECTION('',(1.,0.,0.));
#61002=DIRECTION('',(0.,1.,0.));
#61003=DIRECTION('',(0.,0.,1.));
#61004=(CARTESIAN_TRANSFORMATION_OPERATOR('',#61001,#61002,#61000,2.)CARTESIAN_TRANSFORMATION_OPERATOR_3D(#61003)CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(3.,4.)GEOMETRIC_REPRESENTATION_ITEM()REPRESENTATION_ITEM(''));
#61005=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#{body}),#{context});
#61006=(REPRESENTATION_RELATIONSHIP('','',#{root},#61005)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#61004)SHAPE_REPRESENTATION_RELATIONSHIP());
#61007=PRODUCT_DEFINITION('child','',#{formation},#{definition_context});
#61008=NEXT_ASSEMBLY_USAGE_OCCURRENCE('occ-1','child occurrence','',#{product_definition},#61007,$);
");
        let placed = text.replacen("ENDSEC;\nEND-ISO", &format!("{rows}ENDSEC;\nEND-ISO"), 1);
        assert!(import_step_v5(&placed).is_err());
        let (back, _, report) = import_step_v6(&placed).unwrap();
        assert_eq!(back.bodies.len(), 2);
        assert!(
            back.vertices
                .iter()
                .any(|vertex| vertex.point == [12., 23., 34.])
        );
        assert_eq!(report.definition_identities.len(), 1);
        assert_eq!(report.occurrence_identities.len(), 2);
        assert!(
            report
                .product_hierarchy
                .iter()
                .any(|row| row.contains("assembly-use:#61008"))
        );
        let mut rows = data_payload(&placed)
            .unwrap()
            .lines()
            .filter(|line| line.starts_with('#'))
            .collect::<Vec<_>>();
        rows.reverse();
        let data_start = placed.find("DATA;").unwrap() + 5;
        let data_end = placed[data_start..].find("ENDSEC;").unwrap() + data_start;
        let reordered = format!(
            "{}\n{}\n{}",
            &placed[..data_start],
            rows.join("\n"),
            &placed[data_end..]
        );
        let (_, _, reordered_report) = import_step_v6(&reordered).unwrap();
        assert_eq!(
            reordered_report.definition_identities,
            report.definition_identities
        );
        assert_eq!(
            reordered_report.occurrence_identities,
            report.occurrence_identities
        );
        let singular = placed.replace(
            "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(3.,4.)",
            "CARTESIAN_TRANSFORMATION_OPERATOR_3DNON_UNIFORM(0.,4.)",
        );
        assert!(
            import_step_v6(&singular)
                .unwrap_err()
                .message
                .contains("positive")
        );
        let skewed = placed.replace(
            "#61002=DIRECTION('',(0.,1.,0.));",
            "#61002=DIRECTION('',(1.,1.,0.));",
        );
        assert!(
            import_step_v6(&skewed)
                .unwrap_err()
                .message
                .contains("non-orthogonal")
        );
        let (sheared, shear_certificate, shear_report) = import_step_v9(&skewed).unwrap();
        assert_eq!(shear_certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert_eq!(sheared.bodies.len(), 2);
        assert!(!shear_report.identity.preserved);
        let reflected = placed.replace(
            "#61003=DIRECTION('',(0.,0.,1.));",
            "#61003=DIRECTION('',(0.,0.,-1.));",
        );
        let (reflected, _, _) = import_step_v9(&reflected).unwrap();
        assert!(
            reflected
                .shells
                .iter()
                .flat_map(|shell| &shell.faces)
                .any(|usage| usage.reversed)
        );
        assert!(
            import_step_v9(&singular)
                .unwrap_err()
                .message
                .contains("nonzero")
        );
    }

    #[test]
    fn v6_exact_and_plus_one_resource_boundaries() {
        assert!(lex(&" ".repeat(MAX_BYTES)).is_ok());
        assert!(lex(&" ".repeat(MAX_BYTES + 1)).is_err());
        assert!(enforce_output_limit(&"é".repeat(MAX_OUTPUT_BYTES / 2)).is_ok());
        assert!(enforce_output_limit(&("é".repeat(MAX_OUTPUT_BYTES / 2) + "a")).is_err());
        assert!(
            parse(&format!(
                "#1=PRODUCT('',{});",
                "(".repeat(MAX_PARSE_DEPTH) + &")".repeat(MAX_PARSE_DEPTH)
            ))
            .is_ok()
        );
        assert!(
            parse(&format!(
                "#1=PRODUCT('',{});",
                "(".repeat(MAX_PARSE_DEPTH + 1) + &")".repeat(MAX_PARSE_DEPTH + 1)
            ))
            .is_err()
        );
        let exact = (1..=MAX_INSTANCES)
            .map(|id| format!("#{id}=CARTESIAN_POINT('',(0.,0.,0.));"))
            .collect::<String>();
        assert_eq!(parse(&exact).unwrap().len(), MAX_INSTANCES);
        assert!(
            parse(&(exact + &format!("#{}=CARTESIAN_POINT('',(0.,0.,0.));", MAX_INSTANCES + 1)))
                .is_err()
        );
        let graph = (1..=MAX_GRAPH_DEPTH + 1)
            .map(|id| {
                if id == MAX_GRAPH_DEPTH + 1 {
                    format!("#{id}=PRODUCT('end');")
                } else {
                    format!("#{id}=PRODUCT('',#{});", id + 1)
                }
            })
            .collect::<String>();
        assert_eq!(
            reachable(&parse(&graph).unwrap(), &[1]).unwrap().len(),
            MAX_GRAPH_DEPTH + 1
        );
        let graph_plus = graph.replace(
            &format!("#{}=PRODUCT('end');", MAX_GRAPH_DEPTH + 1),
            &format!(
                "#{}=PRODUCT('',#{});#{}=PRODUCT('end');",
                MAX_GRAPH_DEPTH + 1,
                MAX_GRAPH_DEPTH + 2,
                MAX_GRAPH_DEPTH + 2
            ),
        );
        assert!(reachable(&parse(&graph_plus).unwrap(), &[1]).is_err());

        let units=(0..8).map(|index|format!("#{}=(CONVERSION_BASED_UNIT('u',#{})LENGTH_UNIT()NAMED_UNIT(*));#{}=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#{});",
            100+2*index,101+2*index,101+2*index,if index==7{200}else{102+2*index})).collect::<String>()
            +"#200=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));";
        let parsed = parse(&units).unwrap();
        assert_eq!(
            unit_scale(&parsed, 100, 0, &mut BTreeSet::new()).unwrap(),
            1000.
        );
        let unit_plus=units.replace("#200=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));",
            "#200=(CONVERSION_BASED_UNIT('u',#201)LENGTH_UNIT()NAMED_UNIT(*));#201=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(1.),#202);#202=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT($,.METRE.));");
        assert!(unit_scale(&parse(&unit_plus).unwrap(), 100, 0, &mut BTreeSet::new()).is_err());

        let placement_graph = |depth: usize| {
            let mut text="#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#1))REPRESENTATION_CONTEXT('','3D'));#3=CARTESIAN_POINT('',(0.,0.,0.));#4=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#3,1.,$);".to_string();
            for index in 0..=depth {
                let id = 1000 + index;
                text.push_str(&format!(
                    "#{id}=ADVANCED_BREP_SHAPE_REPRESENTATION('',{},#2);",
                    if index == depth { "(#900)" } else { "()" }
                ));
                if index < depth {
                    text.push_str(&format!("#{}=(REPRESENTATION_RELATIONSHIP('','',#{id},#{})REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#4)SHAPE_REPRESENTATION_RELATIONSHIP());",2000+index,id+1))
                }
            }
            text.push_str("#900=MANIFOLD_SOLID_BREP();");
            parse(&text).unwrap()
        };
        assert_eq!(
            representation_occurrences(&placement_graph(32), &[1000], true, false, false)
                .unwrap()
                .len(),
            1
        );
        assert!(
            representation_occurrences(&placement_graph(33), &[1000], true, false, false).is_err()
        );
        let occurrence_graph = |children: usize| {
            let mut text="#1=(LENGTH_UNIT()NAMED_UNIT(*)SI_UNIT(.MILLI.,.METRE.));#2=(GEOMETRIC_REPRESENTATION_CONTEXT(3)GLOBAL_UNIT_ASSIGNED_CONTEXT((#1))REPRESENTATION_CONTEXT('','3D'));#3=MANIFOLD_SOLID_BREP();#4=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#3),#2);#5=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#3),#2);".to_string();
            for index in 0..children {
                text.push_str(&format!("#{}=CARTESIAN_POINT('',({index}.,0.,0.));#{}=CARTESIAN_TRANSFORMATION_OPERATOR_3D('',$,$,#{},1.,$);#{}=(REPRESENTATION_RELATIONSHIP('','',#4,#5)REPRESENTATION_RELATIONSHIP_WITH_TRANSFORMATION(#{})SHAPE_REPRESENTATION_RELATIONSHIP());",
                    1000+index*3,1001+index*3,1000+index*3,1002+index*3,1001+index*3));
            }
            parse(&text).unwrap()
        };
        assert_eq!(
            representation_occurrences(
                &occurrence_graph(MAX_OCCURRENCES - 1),
                &[4],
                true,
                false,
                false
            )
            .unwrap()
            .len(),
            MAX_OCCURRENCES
        );
        assert!(
            representation_occurrences(
                &occurrence_graph(MAX_OCCURRENCES),
                &[4],
                true,
                false,
                false
            )
            .is_err()
        );
    }

    #[test]
    fn v6_exhausts_valid_edge_oriented_bound_face_and_void_senses() {
        fn exchange(header: &str, entities: &BTreeMap<usize, Entity>) -> String {
            let start = header.find("DATA;").unwrap() + 5;
            let end = header[start..].find("ENDSEC;").unwrap() + start;
            let rows = entities
                .iter()
                .map(|(id, entity)| format!("#{id}={};", render_entity_value(&entity.value)))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{}\n{}\n{}", &header[..start], rows, &header[end..])
        }
        fn reverse_curve(entities: &mut BTreeMap<usize, Entity>, id: usize) {
            let entity = entities.get_mut(&id).unwrap();
            let reverse_list = |value: &mut Value| {
                if let Value::List(items) = value {
                    items.reverse()
                }
            };
            match &mut entity.value {
                Value::Call(name, args) if name == "B_SPLINE_CURVE_WITH_KNOTS" => {
                    reverse_list(&mut args[2]);
                    reverse_list(&mut args[6]);
                    reverse_list(&mut args[7]);
                    if let Value::List(knots) = &mut args[7] {
                        let values = knots
                            .iter()
                            .map(|value| number(value, "knot").unwrap())
                            .collect::<Vec<_>>();
                        let sum = values[0] + values[values.len() - 1];
                        for (value, next) in knots.iter_mut().zip(values) {
                            *value = Value::Number(sum - next)
                        }
                    }
                }
                Value::List(parts) => {
                    for part in parts {
                        if let Value::Call(name, args) = part {
                            if name == "B_SPLINE_CURVE" {
                                reverse_list(&mut args[1])
                            }
                            if name == "B_SPLINE_CURVE_WITH_KNOTS" {
                                reverse_list(&mut args[0]);
                                if let Value::List(knots) = &mut args[1] {
                                    let values = knots
                                        .iter()
                                        .map(|value| number(value, "knot").unwrap())
                                        .collect::<Vec<_>>();
                                    let sum = values[0] + values[values.len() - 1];
                                    knots.reverse();
                                    for value in knots {
                                        if let Value::Number(knot) = value {
                                            *knot = sum - *knot
                                        }
                                    }
                                }
                            }
                            if name == "RATIONAL_B_SPLINE_CURVE" {
                                reverse_list(&mut args[0])
                            }
                        }
                    }
                }
                _ => panic!("unexpected curve entity"),
            }
        }
        fn pcurve_curve(entities: &BTreeMap<usize, Entity>, pcurve: usize) -> usize {
            let reference =
                one_ref(&call(entities, pcurve).unwrap().1[2], "reference_to_curve").unwrap();
            let (ty, args) = call(entities, reference).unwrap();
            if ty == "DEFINITIONAL_REPRESENTATION" {
                one_ref(&list(&args[1], "items").unwrap()[0], "curve").unwrap()
            } else {
                reference
            }
        }
        fn reverse_loop_encoding(
            variant: &mut BTreeMap<usize, Entity>,
            loop_id: usize,
            surface: usize,
        ) {
            let members = {
                let Value::Call(_, args) = &mut variant.get_mut(&loop_id).unwrap().value else {
                    unreachable!()
                };
                let Value::List(members) = &mut args[1] else {
                    unreachable!()
                };
                members.reverse();
                members
                    .iter()
                    .map(|value| one_ref(value, "oriented").unwrap())
                    .collect::<Vec<_>>()
            };
            for oriented in members {
                let edge = {
                    let Value::Call(_, args) = &mut variant.get_mut(&oriented).unwrap().value
                    else {
                        unreachable!()
                    };
                    args[4] = Value::Enum(
                        if boolean(&args[4], "orientation").unwrap() {
                            "F"
                        } else {
                            "T"
                        }
                        .into(),
                    );
                    one_ref(&args[3], "edge").unwrap()
                };
                let geometry = one_ref(&call(variant, edge).unwrap().1[3], "geometry").unwrap();
                let pcurve = list(&call(variant, geometry).unwrap().1[2], "pcurves")
                    .unwrap()
                    .iter()
                    .find_map(|value| {
                        let id = one_ref(value, "pcurve").ok()?;
                        let args = call(variant, id).ok()?.1;
                        (one_ref(&args[1], "surface").ok() == Some(surface)).then_some(id)
                    })
                    .unwrap();
                let curve = pcurve_curve(variant, pcurve);
                reverse_curve(variant, curve);
            }
        }
        let model = freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let (base, _, _) = export_step_v6(&model).unwrap();
        let base = base
            .lines()
            .map(|line| {
                if let Some(start) = line.find("'OSCAD_TOPO/3|") {
                    let end = line[start + 1..].find('\'').unwrap() + start + 1;
                    format!("{}''{}", &line[..start], &line[end + 1..])
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let entities = parse(data_payload(&base).unwrap()).unwrap();
        let edge_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "EDGE_CURVE").then_some(*id))
            })
            .collect::<Vec<_>>();
        for edge_id in edge_ids {
            let mut variant = entities.clone();
            let geometry = {
                let Value::Call(_, args) = &mut variant.get_mut(&edge_id).unwrap().value else {
                    unreachable!()
                };
                args[4] = Value::Enum("F".into());
                one_ref(&args[3], "geometry").unwrap()
            };
            let (curve3, pcurves) = {
                let (_, args) = call(&variant, geometry).unwrap();
                (
                    one_ref(&args[1], "curve").unwrap(),
                    list(&args[2], "pcurves")
                        .unwrap()
                        .iter()
                        .map(|value| one_ref(value, "pcurve").unwrap())
                        .collect::<Vec<_>>(),
                )
            };
            reverse_curve(&mut variant, curve3);
            let mut reversed_pcurves = BTreeSet::new();
            for pcurve in pcurves {
                let curve = pcurve_curve(&variant, pcurve);
                if reversed_pcurves.insert(curve) {
                    reverse_curve(&mut variant, curve)
                }
            }
            assert!(
                import_step_v6(&exchange(&base, &variant)).is_ok(),
                "edge #{edge_id}"
            );
        }
        let loop_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "EDGE_LOOP").then_some(*id))
            })
            .collect::<Vec<_>>();
        for mask in 0..(1usize << loop_ids.len()) {
            let mut variant = entities.clone();
            for (bit, loop_id) in loop_ids.iter().enumerate() {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let bound = variant
                    .iter()
                    .find_map(|(id, _)| {
                        call(&variant, *id).ok().and_then(|(name, args)| {
                            (matches!(name, "FACE_OUTER_BOUND" | "FACE_BOUND")
                                && one_ref(&args[1], "loop").ok() == Some(*loop_id))
                            .then_some(*id)
                        })
                    })
                    .unwrap();
                let surface = variant
                    .iter()
                    .find_map(|(id, _)| {
                        call(&variant, *id).ok().and_then(|(name, args)| {
                            if name != "ADVANCED_FACE" {
                                return None;
                            }
                            list(&args[1], "bounds")
                                .ok()?
                                .iter()
                                .any(|value| one_ref(value, "bound").ok() == Some(bound))
                                .then(|| one_ref(&args[2], "surface").unwrap())
                        })
                    })
                    .unwrap();
                reverse_loop_encoding(&mut variant, *loop_id, surface);
                let Value::Call(_, args) = &mut variant.get_mut(&bound).unwrap().value else {
                    unreachable!()
                };
                args[2] = Value::Enum("F".into());
            }
            if let Err(error) = import_step_v6(&exchange(&base, &variant)) {
                panic!("loop mask {mask}: {}", error.message)
            }
        }
        let face_ids = entities
            .keys()
            .filter_map(|id| {
                call(&entities, *id)
                    .ok()
                    .and_then(|(ty, _)| (ty == "ADVANCED_FACE").then_some(*id))
            })
            .collect::<Vec<_>>();
        for mask in 0..(1usize << face_ids.len()) {
            let mut variant = entities.clone();
            for (bit, face) in face_ids.iter().enumerate() {
                if mask & (1 << bit) == 0 {
                    continue;
                }
                let (_, face_args) = call(&variant, *face).unwrap();
                let surface = one_ref(&face_args[2], "surface").unwrap();
                let loops = list(&face_args[1], "bounds")
                    .unwrap()
                    .iter()
                    .map(|value| {
                        let bound = one_ref(value, "bound").unwrap();
                        one_ref(&call(&variant, bound).unwrap().1[1], "loop").unwrap()
                    })
                    .collect::<Vec<_>>();
                for loop_id in loops {
                    reverse_loop_encoding(&mut variant, loop_id, surface)
                }
                let Value::Call(_, args) = &mut variant.get_mut(face).unwrap().value else {
                    unreachable!()
                };
                args[3] = Value::Enum("F".into());
            }
            if let Err(error) = import_step_v6(&exchange(&base, &variant)) {
                panic!("face mask {mask}: {}", error.message)
            }
        }
    }

    #[test]
    fn v9_certifies_interior_point_inverse_and_ambiguity() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
            weights: vec![1., 0.75, 1.],
            periodic: false,
        };
        curve.validate().unwrap();
        let wanted = curve.evaluate(0.37).unwrap().point;
        let parameter = isolate_curve_point(&curve, &wanted).unwrap();
        assert!((parameter - 0.37).abs() < 1e-8);
        assert!(isolate_curve_point(&curve, &[9., 9., 9.]).is_err());
    }

    #[test]
    fn v9_roundtrips_retained_open_shell() {
        let control_points = (0..4)
            .map(|u| {
                (0..4)
                    .map(|v| vec![u as f64 / 3., v as f64 / 3., (u * v) as f64 / 90.])
                    .collect()
            })
            .collect();
        let surface = Surface {
            degree_u: 3,
            degree_v: 3,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points,
            weights: vec![vec![1.; 4]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let model = crate::bicubic_open_face(surface).unwrap();
        assert!(export_step_v8(&model).is_err());
        let (text, certificate, _) = export_step_v9(&model).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert!(text.contains("MANIFOLD_SURFACE_SHAPE_REPRESENTATION"));
        assert!(text.contains("SHELL_BASED_SURFACE_MODEL"));
        let (back, certificate, _) = import_step_v9(&text).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V9_CAPABILITY);
        assert!(back.bodies.is_empty());
        assert_eq!(back.shells.len(), 1);
        assert!(!back.shells[0].closed);
    }

    #[test]
    fn v10_retains_affine_occurrence_graph_and_detects_mutation() {
        let text =
            include_str!("../../../../tests/fixtures/step-v6/self-authored-ap242-assembly.step");
        let (projection, certificate, document) = import_step_v10(text).unwrap();
        assert_eq!(certificate.capability, STEP_INTERCHANGE_V10_CAPABILITY);
        assert_eq!(projection.bodies.len(), 3);
        assert_eq!(document.definition_identities.len(), 1);
        assert_eq!(document.occurrence_identities.len(), 3);
        assert!(!document.operator_identities.is_empty());
        assert_eq!(export_step_v10(&document).unwrap(), text);
        let mut mutation = document.clone();
        mutation.source =
            mutation
                .source
                .replacen("2.00000000000000000E0", "2.50000000000000000E0", 1);
        assert!(export_step_v10(&mutation).is_err());
    }
