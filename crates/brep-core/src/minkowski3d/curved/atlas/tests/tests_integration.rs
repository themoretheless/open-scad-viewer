    use super::*;
    fn rational_box() -> Model {
        let mut m = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        for (k, f) in m.faces.iter_mut().enumerate() {
            let old = f.surface.control_points.clone();
            f.surface.degree_u = 2;
            f.surface.degree_v = 2;
            f.surface.knots_u = vec![0., 0., 0., 1., 1., 1.];
            f.surface.knots_v = f.surface.knots_u.clone();
            f.surface.control_points = (0..3)
                .map(|i| {
                    (0..3)
                        .map(|j| {
                            (0..3)
                                .map(|d| {
                                    let a = i as f64 / 2.;
                                    let b = j as f64 / 2.;
                                    (1. - a) * (1. - b) * old[0][0][d]
                                        + a * (1. - b) * old[1][0][d]
                                        + (1. - a) * b * old[0][1][d]
                                        + a * b * old[1][1][d]
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect();
            f.surface.weights = vec![vec![1.; 3]; 3];
            f.surface.weights[1][1] = 1.125 + (k as f64) / 64.;
            let a = std::array::from_fn(|d| Q::from_float(old[0][0][d]).unwrap());
            let b = std::array::from_fn(|d| Q::from_float(old[1][0][d]).unwrap());
            let c = std::array::from_fn(|d| Q::from_float(old[0][1][d]).unwrap());
            let normal = cross(&sub(&b, &a), &sub(&c, &a));
            for d in 0..3 {
                f.surface.control_points[1][1][d] += normal[d].to_f64().unwrap() / 128.;
            }
        }
        m
    }
    #[test]
    fn rational_tensor_atlas_is_source_bound_and_roundtrips() {
        let m = rational_box();
        let c = derive(&m, 0).unwrap();
        assert_eq!(c.report.chart_triangles, 12);
        assert_eq!(c.report.vertex_stars, 8);
        let serialized = value_codec::to_string(&c).unwrap();
        let decoded: Certificate = value_codec::from_str(&serialized).unwrap();
        verify_source_atlas(&m, &decoded).unwrap();
        let mut bad = c.clone();
        bad.report.boundary_pairs += 1;
        assert!(verify_source_atlas(&m, &bad).is_err());
        let mut altered = m.clone();
        altered.faces[0].surface.control_points[1][1][2] += 0.125;
        assert!(verify_source_atlas(&altered, &c).is_err());
    }
    #[test]
    fn closed_tetrahedron_with_collapsed_rectangular_boundaries() {
        let mut m = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let surface = m.faces[0].surface.clone();
        m.vertices = vec![[1., 1., 1.], [-1., -1., 1.], [-1., 1., -1.], [1., -1., -1.]]
            .into_iter()
            .map(|point| crate::Vertex { point })
            .collect();
        m.edges.clear();
        m.loops.clear();
        m.faces.clear();
        m.shells[0].faces.clear();
        let mut edges = BTreeMap::new();
        for corners in [[0, 2, 1, 1], [0, 1, 3, 3], [1, 2, 3, 3], [2, 0, 3, 3]] {
            let uv = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
            let mut coedges = Vec::new();
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                let key = (a.min(b), a.max(b));
                let edge = *edges.entry(key).or_insert_with(|| {
                    let id = m.edges.len();
                    let curve = crate::line(
                        m.vertices[key.0].point.to_vec(),
                        m.vertices[key.1].point.to_vec(),
                    );
                    m.edges.push(crate::Edge {
                        degenerate: a == b,
                        vertices: [key.0, key.1],
                        curve,
                    });
                    id
                });
                coedges.push(crate::Coedge {
                    edge,
                    reversed: a > b,
                    pcurve: crate::line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
                });
            }
            let outer = m.loops.len();
            m.loops.push(crate::Loop { coedges });
            let mut surface = surface.clone();
            surface.control_points = vec![
                vec![
                    m.vertices[corners[0]].point.to_vec(),
                    m.vertices[corners[3]].point.to_vec(),
                ],
                vec![
                    m.vertices[corners[1]].point.to_vec(),
                    m.vertices[corners[2]].point.to_vec(),
                ],
            ];
            let face = m.faces.len();
            m.faces.push(crate::Face {
                surface,
                outer,
                holes: vec![],
            });
            m.shells[0].faces.push(crate::FaceUse {
                face,
                reversed: false,
            });
        }
        // Reinitialize persistent IDs after changing the entity counts.
        m.rebuild_topology_ids();
        let c = derive(&m, 0).unwrap();
        assert_eq!(c.positions.len(), 4);
        assert_eq!(c.triangles.len(), 4);
        verify_source_atlas(&m, &c).unwrap();
        save_fixture("collapsed-boundary-tetrahedron", &m);
    }
    #[test]
    fn curved_trim_sphere_uses_regular_charts_at_both_poles() {
        // Radius 3 makes every authored control*weight exact in binary64,
        // including the controls with weight 3. No epsilon closes seams.
        let mut m = crate::sphere(3.).unwrap();
        // Use a rational circle clock with exact weights. The usual sqrt(1/2)
        // weight is a binary64 approximation and cannot prove this identity.
        for f in 0..m.faces.len() {
            let outer = m.faces[f].outer;
            m.loops[outer].coedges[1].pcurve.weights = vec![1., 1., 2.];
            let e = m.loops[outer].coedges[1].clone();
            m.edges[e.edge].curve.weights = if e.reversed {
                vec![2., 1., 1.]
            } else {
                vec![1., 1., 2.]
            };
        }
        m.rebuild_topology_ids();
        save_fixture("curved-trim-regular-poles-sphere", &m);
        for depth in 0..=2 {
            match derive(&m, depth) {
                Ok(c) => {
                    eprintln!("trimmed sphere depth {depth}: {:?}", c.report);
                    verify_source_atlas(&m, &c).unwrap();
                    return;
                }
                Err(e) => eprintln!("trimmed sphere depth {depth}: {}", e.message),
            }
        }
        panic!("curved trimmed sphere unproved");
    }
    #[test]
    fn nonlinear_projective_surface_clocks_keep_the_source_image() {
        let original = rational_box();
        let expected = derive(&original, 0).unwrap();
        let mut changed = original.clone();
        // A positive fractional-linear clock u -> 4u/(1+3u).
        // Homogeneous tensor coefficients scale by 4^i, with no fitting.
        for (i, row) in changed.faces[0].surface.weights.iter_mut().enumerate() {
            for w in row {
                *w *= 4f64.powi(i as i32);
            }
        }
        // Face-local pcurves use the inverse clock, retaining agreement
        // with the independently stored three-dimensional edge traversal.
        let outer = changed.faces[0].outer;
        for e in &mut changed.loops[outer].coedges {
            if e.pcurve.control_points[0][0] != e.pcurve.control_points[1][0] {
                for (p, w) in e.pcurve.control_points.iter().zip(&mut e.pcurve.weights) {
                    *w = if p[0] == 0. { 4. } else { 1. };
                }
            }
        }
        let actual = derive(&changed, 0).unwrap();
        assert_eq!(actual.positions, expected.positions);
        assert_eq!(actual.triangles, expected.triangles);
        assert_ne!(actual.source, expected.source);
        verify_source_atlas(&changed, &actual).unwrap();
        assert!(verify_source_atlas(&original, &actual).is_err());
        save_fixture("nonlinear-projective-clock-box", &changed);
    }
    #[test]
    fn rejects_folded_charts_unmatched_clocks_and_forged_schemas() {
        let mut m = rational_box();
        m.faces[0].surface.weights[1][0] = 1.5;
        assert!(derive(&m, 0).is_err());
        let mut m = rational_box();
        // A parametrization can fold without changing its four corners.
        m.faces[0].surface.control_points[1][1][0] = 128.;
        assert!(derive(&m, 0).is_err());
        let c = derive(&rational_box(), 0).unwrap();
        let mut v = c.to_value();
        v["uncheckedClaim"] = true.to_value();
        assert!(value_codec::from_value::<Certificate>(v).is_err());
        let mut v = c.to_value();
        v["report"]["preservesComplement"] = false.to_value();
        assert!(value_codec::from_value::<Certificate>(v).is_err());
    }
    #[test]
    fn endpoint_binds_to_the_actual_nominal_source_chain() {
        let m = rational_box();
        let (mut input, _, _) = geometry(&m, 0, &integer(100), &mut Work::default(), 4).unwrap();
        assert!(nominal_report(&m, 0, &input).is_ok());
        input.faces[0][0][0] += Q::one();
        assert!(nominal_report(&m, 0, &input).is_err());
        save_fixture("rational-freeform-box", &m);
    }
    #[test]
    fn proves_a_freeform_rational_torus_with_genus_one() {
        let mut m = crate::torus(4., 1.).unwrap();
        m.faces[5].surface.weights[1][1] *= 1.03125;
        m.faces[5].surface.control_points[1][1][2] += 0.015625;
        save_fixture("rational-freeform-torus", &m);
        for depth in 0..=2 {
            match derive(&m, depth) {
                Ok(c) => {
                    eprintln!("torus atlas depth {depth}: {:?}", c.report);
                    let edges = c
                        .triangles
                        .iter()
                        .flat_map(|t| {
                            (0..3)
                                .map(move |k| (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3])))
                        })
                        .collect::<BTreeSet<_>>();
                    assert_eq!(c.positions.len() + c.triangles.len(), edges.len());
                    return;
                }
                Err(e) => eprintln!("torus depth {depth}: {}", e.message),
            }
        }
        panic!("torus atlas unproved");
    }
