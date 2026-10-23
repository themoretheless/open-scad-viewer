    #[test]
    fn reversed_nonuniform_rational_edges_preserve_parameterized_geometry() {
        let curve = Curve {degree:2,knots:vec![0.,0.,0.,0.25,1.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![0.3,0.4,0.],vec![0.7,-0.2,0.1],vec![1.,0.,0.]],
            weights:vec![1.,0.75,1.5,1.],periodic:false};
        let reversed=super::reversed(curve.clone());
        assert_eq!(reversed.knots,vec![0.,0.,0.,0.75,1.,1.,1.]);
        for t in [0.,0.13,0.25,0.43,0.75,0.91,1.] {
            let a=curve.evaluate(t).unwrap();let b=reversed.evaluate(1.-t).unwrap();
            for (a,b) in a.point.iter().zip(b.point) {assert!((a-b).abs()<1e-13);}
            for (a,b) in a.d1.as_ref().unwrap().iter().zip(b.d1.as_ref().unwrap()) {assert!((a+b).abs()<1e-12);}
        }
    }
    #[test]
    fn partial_caps_reparameterize_multiple_offset_holes() {
        let mut loops = Vec::new();
        for (radius, x, y) in [(1., 4., 2.), (0.2, 4.2, 2.1), (0.1, 3.6, 1.8)] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += x;
                    p[1] += y;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        let model = super::revolve_region_angle(&loops, 1e-7, -120.).unwrap();
        assert_eq!(model.faces.iter().filter(|f| f.holes.len() == 2).count(), 2);
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        let expected = 2. * std::f64::consts::PI.powi(2) * (4. - 4.2 * 0.04 - 3.6 * 0.01) / 3.;
        assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
    }
    #[test]
    fn partial_region_revolution_sews_holes_into_common_caps() {
        let mut loops = Vec::new();
        for radius in [1., 0.5] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += 3.;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        for angle in [90., -90., 210., -210.] {
            let model = super::revolve_region_angle(&loops, 1e-7, angle).unwrap();
            assert_eq!(model.bodies.len(), 1);
            assert_eq!(model.shells.len(), 1);
            assert!(model.bodies[0].inner_shells.is_empty());
            assert_eq!(model.faces.iter().filter(|f| f.holes.len() == 1).count(), 2);
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 4.5 * std::f64::consts::PI.powi(2) * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
        }
    }
    #[test]
    fn region_revolution_owns_cavity_shell_and_nested_island() {
        let mut loops = Vec::new();
        for radius in [1., 0.7, 0.3] {
            let mut wire = crate::sketch::circle_wire(radius).unwrap();
            for c in &mut wire {
                for p in &mut c.control_points {
                    p[0] += 3.;
                }
            }
            loops.push(wire);
        }
        let loops = crate::planar_trim::orient_even_odd(&loops, 1e-7).unwrap();
        let model = super::revolve_region(&loops, 1e-7).unwrap();
        assert_eq!(model.bodies.len(), 2);
        assert_eq!(model.shells.len(), 3);
        assert_eq!(model.bodies[0].inner_shells.len(), 1);
        let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
        let expected = 6. * std::f64::consts::PI.powi(2) * (1. - 0.7_f64.powi(2) + 0.3_f64.powi(2));
        assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
    }
    #[test]
    fn curved_axis_endpoints_form_shared_sphere_poles() {
        let circle = crate::sketch::circle_wire(1.).unwrap();
        let wire = vec![
            circle[3].clone(),
            circle[0].clone(),
            super::line(vec![0., 1.], vec![0., -1. + f64::EPSILON / 2.]),
        ];
        for angle in [360., 90., -210.] {
            let model = super::revolve_wire_angle(&wire, 1e-7, angle).unwrap();
            model.validate().unwrap();
            assert_eq!(
                model
                    .vertices
                    .iter()
                    .filter(|v| v.point[0] == 0. && v.point[1] == 0.)
                    .count(),
                2
            );
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 4. * std::f64::consts::PI / 3. * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
        }
        let mut horn = circle;
        for c in &mut horn {
            for p in &mut c.control_points {
                p[0] += 1.;
            }
        }
        assert!(super::revolve_wire(&horn, 1e-7).is_err());
    }
    #[test]
    fn partial_curved_revolution_caps_and_signed_volume() {
        let mut wire = crate::sketch::circle_wire(1.).unwrap();
        for c in &mut wire {
            for p in &mut c.control_points {
                p[0] += 3.;
            }
        }
        for angle in [90., -90., 210., -210.] {
            let model = super::revolve_wire_angle(&wire, 1e-7, angle).unwrap();
            model.validate().unwrap();
            assert_eq!(
                model.faces.len(),
                4 * (angle.abs() / 90.).ceil() as usize + 2
            );
            let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
            let expected = 6. * std::f64::consts::PI.powi(2) * angle.abs() / 360.;
            assert!((mass.signed_volume_mm3 / expected - 1.).abs() < 1e-6);
            assert_eq!(
                model
                    .faces
                    .iter()
                    .filter(|f| f.surface.degree_u == 1)
                    .count(),
                2
            );
        }
    }
    #[test]
    fn retained_circle_revolution_is_a_closed_rational_torus() {
        let mut wire = crate::sketch::circle_wire(1.).unwrap();
        for c in &mut wire {
            for p in &mut c.control_points {
                p[0] += 3.;
                p[1] += 2.;
            }
        }
        let model = super::revolve_wire(&wire, 1e-7).unwrap();
        model.validate().unwrap();
        assert_eq!(model.faces.len(), 16);
        assert_eq!(model.edges.len(), 32);
        assert_eq!(model.vertices.len(), 16);
        for face in &model.faces {
            for u in [0., 0.13, 0.5, 0.87, 1.] {
                for v in [0., 0.17, 0.5, 0.83, 1.] {
                    let p = face.surface.evaluate(u, v).unwrap().point;
                    let implicit = (p[0].hypot(p[1]) - 3.).powi(2) + (p[2] - 2.).powi(2);
                    assert!((implicit - 1.).abs() < 1e-12);
                }
            }
        }
        assert!(super::revolve_wire(&crate::sketch::circle_wire(1.).unwrap(), 1e-7).is_err());
        wire[0].control_points[0][0] += 1e-9;
        assert!(super::revolve_wire(&wire, 1e-7).is_err());
    }
    use super::*;
    #[test]
    fn round_solids_have_exact_rational_boundaries_and_shared_topology() {
        for (model, expected_union_volume) in [
            (
                cylinder(3., 5.).unwrap(),
                Some(45. * std::f64::consts::PI + 4.),
            ),
            (frustum(3., 1., 5.).unwrap(), None),
            (tube(3., 1., 5.).unwrap(), None),
        ] {
            let report = model.validate().unwrap();
            assert_eq!(report.boundary_edge_count, 0);
            for face in model.faces.iter().filter(|f| f.surface.degree_u == 2) {
                for i in 0..=20 {
                    let v = i as f64 / 20.;
                    let expected = face.surface.control_points[0][0][0]
                        .hypot(face.surface.control_points[0][0][1])
                        * (1. - v)
                        + face.surface.control_points[0][1][0]
                            .hypot(face.surface.control_points[0][1][1])
                            * v;
                    for j in 0..=20 {
                        let p = face.surface.evaluate(j as f64 / 20., v).unwrap().point;
                        assert!((p[0].hypot(p[1]) - expected).abs() < 1e-12);
                    }
                }
            }
            let restored: Model =
                value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
            restored.validate().unwrap();
            assert_eq!(model.1.faces, restored.1.faces);
            let union = boolean(
                &model,
                &cuboid([-1., -1., -1.], [1., 1., 1.]).unwrap(),
                "union",
            );
            if let Some(expected_volume) = expected_union_volume {
                let union = union.unwrap();
                assert_eq!(union.bodies.len(), 1);
                assert_eq!(union.validate().unwrap().boundary_edge_count, 0);
                let properties = crate::analysis::mass_properties(&union, 1e-7, 800_000).unwrap();
                assert!((properties.signed_volume_mm3 - expected_volume).abs() < 1e-6);
            } else {
                // Outside the exact matrix the union either refuses or comes
                // back through the tolerant fallback, which says so through
                // its tolerance and never as an exact-tolerance result.
                match union {
                    Err(_) => {}
                    Ok(union) => {
                        assert!(union.tolerance_mm > 1e-6);
                        assert_eq!(union.bodies.len(), 1);
                        assert_eq!(union.validate().unwrap().boundary_edge_count, 0);
                    }
                }
            }
        }
    }
    fn verify_round_trip(model: &Model) {
        let report = model.validate().unwrap();
        assert_eq!(report.boundary_edge_count, 0);
        assert!(model.edges.iter().all(|e| e.vertices[0] != e.vertices[1]));
        let encoded = value_codec::to_string(model).unwrap();
        let restored: Model = value_codec::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(encoded, value_codec::to_string(&restored).unwrap());
    }
    fn normal(surface: &Surface, u: f64, v: f64) -> [f64; 3] {
        let e = surface.evaluate(u, v).unwrap();
        let value = value_codec::Serialize::to_value(&e);
        value_codec::Deserialize::from_value(value["normal"].clone()).unwrap()
    }
    #[test]
    fn sphere_is_exact_regular_oriented_and_has_ordinary_poles() {
        for radius in [1e-5, 3., 1e6] {
            let model = sphere(radius).unwrap();
            verify_round_trip(&model);
            assert_eq!(
                (model.vertices.len(), model.edges.len(), model.faces.len()),
                (6, 12, 8)
            );
            for usage in &model.shells[0].faces {
                let surface = &model.faces[usage.face].surface;
                for i in 0..=10 {
                    for j in 0..=10 {
                        let (u, v) = (i as f64 / 10., j as f64 / 10.);
                        let p = surface.evaluate(u, v).unwrap().point;
                        assert!(
                            (p.iter().map(|x| x * x).sum::<f64>().sqrt() - radius).abs()
                                <= radius * 1e-14
                        );
                        let n = normal(surface, u, v);
                        let dot = (0..3).map(|k| n[k] * p[k]).sum::<f64>()
                            * if usage.reversed { -1. } else { 1. };
                        assert!(dot > 0., "Sphere normal must point outward including poles");
                    }
                }
            }
        }
    }
    #[test]
    fn torus_is_exact_regular_oriented_and_genus_one() {
        let (major, minor) = (5., 2.);
        let model = torus(major, minor).unwrap();
        verify_round_trip(&model);
        assert_eq!(
            (model.vertices.len(), model.edges.len(), model.faces.len()),
            (16, 32, 16)
        );
        for usage in &model.shells[0].faces {
            let surface = &model.faces[usage.face].surface;
            for i in 0..=12 {
                for j in 0..=12 {
                    let (u, v) = (i as f64 / 12., j as f64 / 12.);
                    let p = surface.evaluate(u, v).unwrap().point;
                    let radial = p[0].hypot(p[1]);
                    assert!(
                        ((radial - major).powi(2) + p[2].powi(2) - minor.powi(2)).abs() < 1e-12
                    );
                    let outward = [
                        p[0] * (1. - major / radial),
                        p[1] * (1. - major / radial),
                        p[2],
                    ];
                    let n = normal(surface, u, v);
                    assert!((0..3).map(|k| outward[k] * n[k]).sum::<f64>() > 0.);
                }
            }
        }
        assert!(torus(2., 2.).is_err());
        assert!(torus(1., 2.).is_err());
        assert!(torus(1e6, 2.).is_err());
        assert!(sphere(0.).is_err());
        assert!(sphere(f64::NAN).is_err());
    }
    #[test]
    fn cones_have_one_certified_pole_without_artificial_caps() {
        for (bottom, top) in [(3., 0.), (0., 3.)] {
            let model = frustum(bottom, top, 5.).unwrap();
            assert_eq!((model.vertices.len(), model.faces.len()), (5, 5));
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert_eq!(model.edges.iter().filter(|e| e.degenerate).count(), 1);
            for usage in &model.shells[0].faces {
                let surface = &model.faces[usage.face].surface;
                if surface.degree_u == 1 {
                    continue;
                }
                for i in 0..=16 {
                    for j in 1..16 {
                        let (u, v) = (i as f64 / 16., j as f64 / 16.);
                        let p = surface.evaluate(u, v).unwrap().point;
                        assert!((p[0].hypot(p[1]) - (bottom * (1. - v) + top * v)).abs() < 1e-12);
                        let n = normal(surface, u, v);
                        assert!(p[0] * n[0] + p[1] * n[1] > 0.);
                    }
                }
            }
            let encoded = value_codec::to_string(&model).unwrap();
            let restored: Model = value_codec::from_str(&encoded).unwrap();
            restored.validate().unwrap();
            assert_eq!(encoded, value_codec::to_string(&restored).unwrap());
            let pole_edge = model.edges.iter().position(|e| e.degenerate).unwrap();
            let mut stray_pole = model.clone();
            for face in 1..4 {
                let outer = stray_pole.faces[face].outer;
                stray_pole.loops[outer]
                    .coedges
                    .retain(|c| c.edge != pole_edge);
            }
            assert!(stray_pole.validate().is_err());
            for mutation in 0..6 {
                let mut bad = model.clone();
                match mutation {
                    0 => bad.edges[pole_edge].degenerate = false,
                    1 => bad.edges[pole_edge].curve.control_points[1][0] += 1e-12,
                    2 => bad.edges[pole_edge].vertices[1] = 1,
                    3 => bad.faces[0].surface.control_points[1][usize::from(top == 0.)][0] += 1e-12,
                    4 => {
                        let mut e = bad.edges[pole_edge].clone();
                        e.degenerate = true;
                        bad.edges.push(e);
                    }
                    _ => bad.vertices.push(Vertex {
                        point: [0., 0., 99.],
                    }),
                }
                bad.rebuild_topology_ids();
                assert!(bad.validate().is_err(), "Invalid cone mutation {mutation}");
            }
        }
        let mut marked_finite = cylinder(3., 5.).unwrap();
        marked_finite.edges[0].degenerate = true;
        assert!(marked_finite.validate().is_err());
    }
    #[test]
    fn collapsed_boundary_cannot_join_disconnected_face_fans() {
        let mut model = frustum(3., 0., 5.).unwrap();
        let other = frustum(4., 0., 5.).unwrap();
        let pole_edge = model.edges.iter().position(|e| e.degenerate).unwrap();
        let pole = model.edges[pole_edge].vertices[0];
        let other_pole_edge = other.edges.iter().position(|e| e.degenerate).unwrap();
        let other_pole = other.edges[other_pole_edge].vertices[0];
        let vertices: Vec<_> = other
            .vertices
            .iter()
            .enumerate()
            .map(|(i, vertex)| {
                if i == other_pole {
                    pole
                } else {
                    let id = model.vertices.len();
                    model.vertices.push(vertex.clone());
                    id
                }
            })
            .collect();
        let edges: Vec<_> = other
            .edges
            .iter()
            .enumerate()
            .map(|(i, edge)| {
                if i == other_pole_edge {
                    pole_edge
                } else {
                    let id = model.edges.len();
                    let mut edge = edge.clone();
                    edge.vertices = edge.vertices.map(|v| vertices[v]);
                    model.edges.push(edge);
                    id
                }
            })
            .collect();
        let loop_offset = model.loops.len();
        for wire in &other.loops {
            let mut wire = wire.clone();
            for c in &mut wire.coedges {
                c.edge = edges[c.edge];
            }
            model.loops.push(wire);
        }
        let face_offset = model.faces.len();
        for face in &other.faces {
            let mut face = face.clone();
            face.outer += loop_offset;
            face.holes.iter_mut().for_each(|l| *l += loop_offset);
            model.faces.push(face);
        }
        for usage in &other.shells[0].faces {
            model.shells[0].faces.push(FaceUse {
                face: usage.face + face_offset,
                reversed: usage.reversed,
            });
        }
        assert!(
            model
                .0
                .validate_topology()
                .unwrap_err()
                .message
                .contains("disconnected")
        );
    }
    #[test]
    fn axis_profiles_revolve_to_closed_solids_with_explicit_poles() {
        for profile in [
            vec![[0., 0.], [3., 0.], [0., 5.]],
            vec![[0., 0.], [3., 0.], [3., 5.], [0., 5.], [0., 3.], [0., 2.]],
            vec![[0., 0.], [3., 0.], [3., 5.], [2., 5.], [2., 2.], [0., 2.]],
        ] {
            let model = revolve(&profile).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            assert!(model.edges.iter().any(|e| e.degenerate));
            let restored: Model =
                value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
            restored.validate().unwrap();
        }
    }
    fn rectangular_surface_volume(model: &Model) -> f64 {
        let quadrature = [
            (0.06943184420297371, 0.17392742256872692),
            (0.33000947820757187, 0.32607257743127305),
            (0.6699905217924281, 0.32607257743127305),
            (0.9305681557970262, 0.17392742256872692),
        ];
        let mut volume = 0.;
        for usage in &model.shells[0].faces {
            for (u, wu) in quadrature {
                for (v, wv) in quadrature {
                    let evaluated = model.faces[usage.face].surface.evaluate(u, v).unwrap();
                    let data = value_codec::Serialize::to_value(&evaluated);
                    let du: [f64; 3] =
                        value_codec::Deserialize::from_value(data["du"].clone()).unwrap();
                    let dv: [f64; 3] =
                        value_codec::Deserialize::from_value(data["dv"].clone()).unwrap();
                    let n = [
                        du[1] * dv[2] - du[2] * dv[1],
                        du[2] * dv[0] - du[0] * dv[2],
                        du[0] * dv[1] - du[1] * dv[0],
                    ];
                    let dot = (0..3).map(|i| evaluated.point[i] * n[i]).sum::<f64>();
                    volume += dot * wu * wv / 3. * if usage.reversed { -1. } else { 1. };
                }
            }
        }
        volume
    }
    #[test]
    fn signed_partial_revolutions_have_exact_caps_and_correct_oriented_volume() {
        for angle in [30., 90., 137., 270., -30., -90., -137., -270., 360., -360.] {
            for inner in [0., 1.] {
                let profile = [[inner, 0.], [3., 0.], [3., 4.], [inner, 4.]];
                let model = revolve_angle(&profile, angle).unwrap();
                assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
                let expected = angle.abs().to_radians() * (9. - inner * inner) * 4. / 2.;
                assert!((rectangular_surface_volume(&model) / expected - 1.).abs() < 5e-5);
                let restored: Model =
                    value_codec::from_str(&value_codec::to_string(&model).unwrap()).unwrap();
                restored.validate().unwrap();
            }
        }
        for angle in [15., 120., -280.] {
            let profile = [[0., 0.], [3., 0.], [3., 1.], [1., 1.], [1., 3.], [0., 3.]];
            let model = revolve_angle(&profile, angle).unwrap();
            assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
            for face in model.faces.iter().rev().take(2) {
                assert_eq!(model.loops[face.outer].coedges.len(), profile.len());
            }
        }
        for invalid in [0., 361., -361., f64::NAN, f64::INFINITY] {
            assert!(revolve_angle(&[[0., 0.], [3., 0.], [0., 4.]], invalid).is_err());
        }
    }
    #[test]
    fn rejects_singular_and_invalid_round_solids() {
        for radius in [0., -1., f64::NAN, f64::INFINITY, 1e7] {
            assert!(cylinder(radius, 2.).is_err());
        }
        assert!(frustum(0., 0., 3.).is_err());
        assert!(tube(2., 2., 3.).is_err());
        assert!(tube(2., 3., 3.).is_err());
    }
