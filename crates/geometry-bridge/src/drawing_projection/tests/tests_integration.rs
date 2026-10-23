    use super::*;
    fn body(a: [f64; 3], b: [f64; 3]) -> Value {
        let model = brep_core::cuboid(a, b).unwrap();
        let mesh = crate::brep::nurbs(&model, 1).unwrap().built.mesh;
        json!({"brep":model,"mesh":mesh})
    }
    #[test]
    fn curved_cylinder_hides_a_body_behind_its_retained_wall() {
        let model = brep_core::cylinder(2., 4.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let behind = body([-0.2, 4., 1.], [0.2, 5., 2.]);
        let source: brep_core::Model = field(&behind, "brep").unwrap();
        let mut vertical_paths = Vec::new();
        for edge in &source.edges {
            if edge.curve.control_points.first().unwrap()[2]
                == edge.curve.control_points.last().unwrap()[2]
            {
                continue;
            }
            let mut curve = edge.curve.clone();
            for p in &mut curve.control_points {
                *p = project(p, "front").to_vec();
            }
            vertical_paths.push(curve_path(&curve).unwrap());
        }
        assert_eq!(vertical_paths.len(), 4);
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"front","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in vertical_paths {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"].as_str() == Some(&path) && p["hidden"] == true)
            );
        }
        assert!(
            result["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"] == true)
        );
    }
    #[test]
    fn sphere_hides_separate_body_without_display_mesh() {
        let model = brep_core::sphere(2.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let behind = body([0.3, 0.3, -4.], [0.5, 0.5, -3.]);
        let alone = drawing(
            &json!({"bodies":[behind.clone()],"view":"top","hidden":"show","occlusion":"brep"}),
        )
        .unwrap();
        for view in ["front", "right"] {
            drawing(&json!({"bodies":[front.clone(),behind.clone()],"view":view,"hidden":"dash","occlusion":"brep"})).unwrap_or_else(|e|panic!("{view}: {e:?}"));
        }
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in alone["paths"].as_array().unwrap() {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"] == path["d"] && p["hidden"] == true),
                "missing hidden path: {}",
                path["d"]
            );
        }
    }
    #[test]
    fn curved_retained_face_uses_uniform_surface_proof() {
        let model = brep_core::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
        let surface = nurbs_core::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 3.], vec![0.5, 1., 3.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        // The cuboid contributes only its authored unit-square UV trim; the
        // surface under test is the independent curved chart with that domain.
        let face = RetainedFace::Curved {
            surface,
            domain: brep_core::face_domain::FaceDomain::new(&model, 0, 1e-9).unwrap(),
        };
        let behind = Curve::from_polyline(vec![vec![0.4, 0.4, -1.], vec![0.6, 0.6, -1.]]).unwrap();
        let mut work = 0;
        assert_eq!(face.state(&behind, 1e-5, &mut work).unwrap(), 1);
        assert!(work > 0);
        let front = Curve::from_polyline(vec![vec![0.4, 0.4, 5.], vec![0.6, 0.6, 5.]]).unwrap();
        assert_eq!(face.state(&front, 1e-5, &mut work).unwrap(), 0);
    }
    #[test]
    fn curved_chart_uv_hole_keeps_the_body_behind_it_visible() {
        let outer =
            brep_core::sketch::polygon_wire(vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]).unwrap();
        let hole = brep_core::sketch::polygon_wire(vec![
            [0.35, 0.35],
            [0.65, 0.35],
            [0.65, 0.65],
            [0.35, 0.65],
        ])
        .unwrap();
        let loops = brep_core::planar_trim::orient_even_odd(&[outer, hole], 1e-7).unwrap();
        let model = brep_core::prism::extrude(&loops, 0., 1.).unwrap();
        let index = model
            .faces
            .iter()
            .position(|face| !face.holes.is_empty())
            .unwrap();
        let surface = nurbs_core::surface::Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![0.5, 0., 3.], vec![0.5, 1., 3.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let face = RetainedFace::Curved {
            surface,
            domain: brep_core::face_domain::FaceDomain::new(&model, index, 1e-9).unwrap(),
        };
        let through_hole =
            Curve::from_polyline(vec![vec![0.45, 0.5, -1.], vec![0.55, 0.5, -1.]]).unwrap();
        let behind_material =
            Curve::from_polyline(vec![vec![0.1, 0.1, -1.], vec![0.2, 0.1, -1.]]).unwrap();
        let mut work = 0;
        assert_eq!(face.state(&through_hole, 1e-5, &mut work).unwrap(), 0);
        assert_eq!(face.state(&behind_material, 1e-5, &mut work).unwrap(), 1);
    }
    #[test]
    fn retained_box_edges_and_hidden_lines_have_native_proof() {
        let b = body([0., 0., 0.], [10., 8., 6.]);
        let all = drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"show"})).unwrap();
        assert_eq!(all["paths"].as_array().unwrap().len(), 12);
        assert_eq!(all["approximate"].as_bool(), Some(false));
        let dashed_box =
            drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"dash"})).unwrap();
        assert_eq!(dashed_box["paths"].as_array().unwrap().len(), 12);
        let behind = body([2., 2., -4.], [8., 6., -2.]);
        let dashed =
            drawing(&json!({"bodies":[b.clone(),behind.clone()],"view":"top","hidden":"dash"}))
                .unwrap();
        assert!(
            dashed["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"].as_bool() == Some(true))
        );
        let hidden = drawing(&json!({"bodies":[b,behind],"view":"top","hidden":"hide"})).unwrap();
        assert!(
            hidden["paths"]
                .as_array()
                .unwrap()
                .iter()
                .all(|p| p["hidden"].as_bool() == Some(false))
        );
        assert!(
            hidden["paths"].as_array().unwrap().len() < dashed["paths"].as_array().unwrap().len()
        );
    }
    #[test]
    fn planar_brep_occlusion_is_independent_of_display_mesh() {
        let front = body([0., 0., 0.], [10., 8., 6.]);
        let mut behind = body([2., 2., -4.], [8., 6., -2.]);
        let baseline=drawing(&json!({"bodies":[front.clone(),behind.clone()],"view":"top","hidden":"dash","occlusion":"brep"})).unwrap();
        assert!(
            baseline["paths"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["hidden"] == true)
        );
        behind["mesh"] = front["mesh"].clone();
        let changed = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        assert_eq!(baseline["paths"], changed["paths"]);
        assert_eq!(baseline["bounds"], changed["bounds"]);
    }
    #[test]
    fn retained_planar_hole_does_not_occlude_a_body_behind_it() {
        let outer = brep_core::sketch::polygon_wire(vec![[0., 0.], [10., 0.], [10., 8.], [0., 8.]])
            .unwrap();
        let mut inner = brep_core::sketch::circle_wire(1.).unwrap();
        for curve in &mut inner {
            for p in &mut curve.control_points {
                p[0] += 5.;
                p[1] += 4.;
            }
        }
        let loops = brep_core::planar_trim::orient_even_odd(&[outer, inner], 1e-7).unwrap();
        let model = brep_core::prism::extrude(&loops, 0., 6.).unwrap();
        let front = json!({"brep":model,"mesh":crate::brep::nurbs(&model,2).unwrap().built.mesh});
        let behind = body([4.7, 3.7, -4.], [5.3, 4.3, -2.]);
        let all = drawing(
            &json!({"bodies":[behind.clone()],"view":"top","hidden":"show","occlusion":"brep"}),
        )
        .unwrap();
        let result = drawing(
            &json!({"bodies":[front,behind],"view":"top","hidden":"dash","occlusion":"brep"}),
        )
        .unwrap();
        for path in all["paths"].as_array().unwrap() {
            assert!(
                result["paths"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["d"] == path["d"] && p["hidden"] == false)
            );
        }
    }
    #[test]
    fn durable_dimensions_survive_subdivision_reordering_and_transform() {
        let source = brep_core::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let second = source
            .vertices
            .iter()
            .position(|v| v.point[0] != source.vertices[0].point[0])
            .unwrap();
        let ids = [source.1.vertices[0], source.1.vertices[second]];
        let request = json!({"bodyId":"part","vertices":[0,second],"topologySignature":topology_signature(&source),"vertexIds":ids,"axis":"horizontal"});
        let wire =
            brep_core::sketch::polygon_wire(vec![[0., 0.], [1., 0.], [2., 0.], [2., 1.], [0., 1.]])
                .unwrap();
        let mut changed = brep_core::prism::extrude(&[wire], 0., 1.).unwrap();
        changed.inherit_topology_ids(&[&source]);
        changed.vertices.swap(0, 1);
        changed.1.vertices.swap(0, 1);
        for edge in &mut changed.edges {
            for v in &mut edge.vertices {
                if *v == 0 {
                    *v = 1;
                } else if *v == 1 {
                    *v = 0;
                }
            }
        }
        let changed = brep_core::transform::affine(
            &changed,
            [
                [2., 0., 0., 0.],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        let body = json!({"id":"part","brep":changed,"mesh":crate::brep::nurbs(&changed,1).unwrap().built.mesh});
        let result =
            drawing(&json!({"bodies":[body],"view":"top","hidden":"show","dimensions":[request]}))
                .unwrap();
        assert_eq!(result["dimensions"][0]["valueMm"].as_f64(), Some(4.));
        assert_eq!(result["dimensions"][0]["vertexIds"], json!(ids));
        assert_eq!(
            result["dimensions"][0]["referenceStatus"],
            json!(["persistent", "persistent"])
        );
        assert_eq!(
            result["references"][0]["vertexIds"]
                .as_array()
                .unwrap()
                .len(),
            10
        );
    }
    #[test]
    fn durable_dimension_split_or_deleted_vertex_requires_explicit_reattachment() {
        let mut model = brep_core::cuboid([0., 0., 0.], [2., 1., 1.]).unwrap();
        let ids = [model.1.vertices[0], model.1.vertices[1]];
        let request = json!({"bodyId":"part","vertices":[0,1],"topologySignature":topology_signature(&model),"vertexIds":ids,"axis":"horizontal"});
        model.1.change_set.changes.push(brep_core::TopologyChange {
            kind: brep_core::ChangeKind::Split,
            topo_kind: brep_core::TopoKind::Vertex,
            parents: vec![ids[0]],
            children: ids.to_vec(),
            provenance: brep_core::ChangeProvenance {
                operation: "drawing-test".into(),
                operand: None,
                occurrence: "part".into(),
            },
            role: "vertex".into(),
            anchor: None,
        });
        let body = json!({"id":"part","brep":model,"mesh":crate::brep::nurbs(&model,1).unwrap().built.mesh});
        let error = drawing(
            &json!({"bodies":[body],"view":"top","hidden":"show","dimensions":[request.clone()]}),
        )
        .unwrap_err();
        assert!(error.message.contains("surviving vertices"));
        let mut replacement = self::body([10., 0., 0.], [12., 1., 1.]);
        replacement["id"] = json!("part");
        let error = drawing(
            &json!({"bodies":[replacement],"view":"top","hidden":"show","dimensions":[request]}),
        )
        .unwrap_err();
        assert!(error.message.contains("deleted"));
    }
    #[test]
    fn linked_vertex_dimensions_recompute_and_reject_changed_topology() {
        let mut b = body([0., 0., 0.], [10., 8., 6.]);
        b["id"] = json!("part");
        let first = drawing(&json!({"bodies":[b.clone()],"view":"top","hidden":"show"})).unwrap();
        let signature = &first["references"][0]["topologySignature"];
        let model: brep_core::Model = field(&b, "brep").unwrap();
        let pair = (0..model.vertices.len())
            .flat_map(|i| (i + 1..model.vertices.len()).map(move |j| [i, j]))
            .find(|p| (model.vertices[p[0]].point[0] - model.vertices[p[1]].point[0]).abs() > 1.)
            .unwrap();
        let request = json!({"bodyId":"part","vertices":pair,"topologySignature":signature,"axis":"horizontal"});
        let original = drawing(
            &json!({"bodies":[b],"view":"top","hidden":"show","dimensions":[request.clone()]}),
        )
        .unwrap();
        let mut changed = body([0., 0., 0.], [20., 8., 6.]);
        changed["id"] = json!("part");
        let next=drawing(&json!({"bodies":[changed.clone()],"view":"top","hidden":"show","dimensions":[request.clone()]})).unwrap();
        assert_eq!(
            next["dimensions"][0]["valueMm"].as_f64().unwrap(),
            2. * original["dimensions"][0]["valueMm"].as_f64().unwrap()
        );
        let mut invalid = request;
        invalid["topologySignature"] = json!("stale");
        assert!(
            drawing(
                &json!({"bodies":[changed],"view":"top","hidden":"show","dimensions":[invalid]})
            )
            .is_err()
        );
    }
    #[test]
    fn original_cubic_and_conic_paths_are_not_polyline_approximations() {
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 0.]]).unwrap();
        assert!(curve_path(&c).unwrap().contains('L'));
        let c = Curve {
            periodic: false,
            degree: 3,
            control_points: vec![
                vec![0., 0., 0.],
                vec![0., 1., 0.],
                vec![1., 1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 4],
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        };
        assert!(curve_path(&c).unwrap().contains('C'));
        let c = Curve {
            periodic: false,
            degree: 2,
            control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
            weights: vec![1., 0.5_f64.sqrt(), 1.],
            knots: vec![0., 0., 0., 1., 1., 1.],
        };
        assert!(curve_path(&c).unwrap().contains('A'));
    }
