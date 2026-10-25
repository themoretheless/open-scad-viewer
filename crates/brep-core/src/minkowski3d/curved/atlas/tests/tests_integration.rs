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
    fn collapsed_tetrahedron() -> Model {
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
                coedges.push(crate::Coedge::new(
                    edge,
                    a > b,
                    crate::line(uv[i].to_vec(), uv[(i + 1) % 4].to_vec()),
                ));
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
        m
    }
    #[test]
    fn closed_tetrahedron_with_collapsed_rectangular_boundaries() {
        let m = collapsed_tetrahedron();
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

    #[test]
    fn rational_nonmonomial_poles_roundtrip_a_closed_source_model() {
        let mut m = collapsed_tetrahedron();
        // Symmetric strictly monotone rational clock, stationary at both ends.
        // R(1-t)=1-R(t) exactly, so shared boundary clocks agree in both directions.
        let t = [0., 0., 1., 1.];
        let w = [1., 0.5, 0.5, 1.];
        for f in &mut m.faces {
            let old = f.surface.control_points.clone();
            f.surface.degree_u = 3;
            f.surface.degree_v = 3;
            let knots = vec![0., 0., 0., 0., 1., 1., 1., 1.];
            f.surface.knots_u = knots.clone();
            f.surface.knots_v = knots;
            f.surface.control_points = (0..4)
                .map(|i| {
                    (0..4)
                        .map(|j| {
                            (0..3)
                                .map(|k| {
                                    (1. - t[i]) * (1. - t[j]) * old[0][0][k]
                                        + t[i] * (1. - t[j]) * old[1][0][k]
                                        + (1. - t[i]) * t[j] * old[0][1][k]
                                        + t[i] * t[j] * old[1][1][k]
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect();
            f.surface.weights = (0..4)
                .map(|i| (0..4).map(|j| w[i] * w[j]).collect())
                .collect();
        }
        let vertices = m.vertices.clone();
        for e in &mut m.edges {
            let a = vertices[e.vertices[0]].point;
            let b = vertices[e.vertices[1]].point;
            e.curve = crate::Curve {
                degree: 3,
                knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
                control_points: t
                    .iter()
                    .map(|t| (0..3).map(|k| (1. - t) * a[k] + t * b[k]).collect())
                    .collect(),
                weights: w.to_vec(),
                periodic: false,
            };
        }
        m.rebuild_topology_ids();
        let c = derive(&m, 0).unwrap();
        assert_eq!(c.triangles.len(), 4);
        verify_source_atlas(&m, &c).unwrap();
        save_fixture("rational-nonmonomial-poles-tetrahedron", &m);
    }

    #[test]
    fn collapsed_source_pole_with_stationary_angular_and_radial_parameters() {
        let mut m = collapsed_tetrahedron();
        let outer = m.faces[0].outer;
        let old = m.faces[0].surface.control_points.clone();
        let (a, b, c) = (old[0][0].clone(), old[1][0].clone(), old[0][1].clone());
        let s = &mut m.faces[0].surface;
        s.degree_u = 3;
        s.degree_v = 2;
        s.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        s.knots_v = vec![0., 0., 0., 1., 1., 1.];
        s.weights = vec![vec![1.; 3]; 4];
        s.control_points = (0..4)
            .map(|i| {
                vec![
                    if i == 3 { b.clone() } else { a.clone() },
                    c.clone(),
                    c.clone(),
                ]
            })
            .collect();
        for ei in [0, 1, 3] {
            let e = m.loops[outer].coedges[ei].clone();
            let mut points = match ei {
                0 => vec![a.clone(), a.clone(), a.clone(), b.clone()],
                1 => vec![b.clone(), c.clone(), c.clone()],
                _ => vec![c.clone(), c.clone(), a.clone()],
            };
            if e.reversed {
                points.reverse();
            }
            let degree = points.len() - 1;
            let mut knots = vec![0.; degree + 1];
            knots.extend(vec![1.; degree + 1]);
            m.edges[e.edge].curve = crate::Curve {
                degree,
                knots: knots.clone(),
                control_points: points.clone(),
                weights: vec![1.; degree + 1],
                periodic: false,
            };
            for li in 0..m.loops.len() {
                if li != outer {
                    for ce in &mut m.loops[li].coedges {
                        if ce.edge == e.edge {
                            let mut p = points.clone();
                            if ce.reversed {
                                p.reverse();
                            }
                            let axis = (0..3).find(|k| p[0][*k] != p[degree][*k]).unwrap();
                            let start = ce.pcurve.control_points[0].clone();
                            let end = ce.pcurve.control_points.last().unwrap().clone();
                            let control_points = p
                                .iter()
                                .map(|q| {
                                    let t = (q[axis] - p[0][axis]) / (p[degree][axis] - p[0][axis]);
                                    (0..2).map(|k| start[k] + t * (end[k] - start[k])).collect()
                                })
                                .collect();
                            ce.pcurve = crate::Curve {
                                degree,
                                knots: knots.clone(),
                                control_points,
                                weights: vec![1.; degree + 1],
                                periodic: false,
                            };
                        }
                    }
                }
            }
        }
        m.rebuild_topology_ids();
        let proof = derive(&m, 0).unwrap();
        assert_eq!(proof.triangles.len(), 4);
        verify_source_atlas(&m, &proof).unwrap();
        save_fixture("stationary-angular-radial-pole", &m);
    }

    #[test]
    fn nonsmooth_rational_cone_pole_is_source_bound() {
        for (bottom, top, name) in [
            (1., 0., "nonsmooth-rational-cone"),
            (0., 1., "inverted-rational-cone"),
        ] {
            let m = crate::frustum(bottom, top, 2.).unwrap();
            let c = derive(&m, 0).unwrap();
            verify_source_atlas(&m, &c).unwrap();
            save_fixture(name, &m);
        }
    }

    #[test]
    fn arbitrary_polynomial_clock_on_straight_seams_is_not_a_projective_clock() {
        let mut m = crate::cuboid([0.; 3], [4.; 3]).unwrap();
        let original = derive(&m, 0).unwrap();
        let old = m.faces[0].surface.control_points.clone();
        m.faces[0].surface.degree_u = 2;
        m.faces[0].surface.knots_u = vec![0., 0., 0., 1., 1., 1.];
        m.faces[0].surface.control_points = [0., 0.25, 1.]
            .iter()
            .map(|u| {
                (0..2)
                    .map(|j| {
                        (0..3)
                            .map(|k| (1. - u) * old[0][j][k] + u * old[1][j][k])
                            .collect()
                    })
                    .collect()
            })
            .collect();
        m.faces[0].surface.weights = vec![vec![1.; 2]; 3];
        let outer = m.faces[0].outer;
        for ei in [0, 2] {
            let e = m.loops[outer].coedges[ei].clone();
            let mut points = (0..3)
                .map(|i| m.faces[0].surface.control_points[i][if ei == 0 { 0 } else { 1 }].clone())
                .collect::<Vec<_>>();
            if ei == 2 {
                points.reverse();
            }
            if e.reversed {
                points.reverse();
            }
            m.edges[e.edge].curve = crate::Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                weights: vec![1.; 3],
                periodic: false,
                control_points: points.clone(),
            };
            for li in 0..m.loops.len() {
                if li != outer {
                    for c in &mut m.loops[li].coedges {
                        if c.edge == e.edge {
                            let mut points = points.clone();
                            if c.reversed {
                                points.reverse();
                            }
                            let a = c.pcurve.control_points[0].clone();
                            let b = c.pcurve.control_points.last().unwrap().clone();
                            let axis = (0..3).find(|k| points[0][*k] != points[2][*k]).unwrap();
                            let progress = points
                                .iter()
                                .map(|p| {
                                    (p[axis] - points[0][axis])
                                        / (points[2][axis] - points[0][axis])
                                })
                                .collect::<Vec<_>>();
                            c.pcurve = crate::Curve {
                                degree: 2,
                                knots: vec![0., 0., 0., 1., 1., 1.],
                                weights: vec![1.; 3],
                                periodic: false,
                                control_points: progress
                                    .iter()
                                    .map(|t| (0..2).map(|k| a[k] + t * (b[k] - a[k])).collect())
                                    .collect(),
                            };
                        }
                    }
                }
            }
        }
        m.rebuild_topology_ids();
        let proof = derive(&m, 0).unwrap();
        assert_eq!(proof.positions, original.positions);
        assert_eq!(proof.triangles, original.triangles);
        verify_source_atlas(&m, &proof).unwrap();
        save_fixture("nonprojective-polynomial-clock-box", &m);
    }

    #[test]
    fn generic_atlas_proves_a_curved_through_hole() {
        let outside = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
        let inside = [
            [0.375, 0.375],
            [0.375, 0.625],
            [0.625, 0.625],
            [0.625, 0.375],
        ];
        let outer = (0..4)
            .map(|i| crate::line(outside[i].to_vec(), outside[(i + 1) % 4].to_vec()))
            .collect::<Vec<_>>();
        let hole = (0..4)
            .map(|i| {
                let (a, b) = (inside[i], inside[(i + 1) % 4]);
                let e = [b[0] - a[0], b[1] - a[1]];
                crate::Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    weights: vec![1.; 3],
                    periodic: false,
                    control_points: vec![
                        a.to_vec(),
                        vec![
                            (a[0] + b[0]) / 2. - e[1] / 8.,
                            (a[1] + b[1]) / 2. + e[0] / 8.,
                        ],
                        b.to_vec(),
                    ],
                }
            })
            .collect::<Vec<_>>();
        let m = crate::prism::extrude(&[outer, hole], 0., 1.).unwrap();
        let c = derive(&m, 0).unwrap();
        verify_source_atlas(&m, &c).unwrap();
        let edges = c
            .triangles
            .iter()
            .flat_map(|t| (0..3).map(move |k| (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]))))
            .collect::<BTreeSet<_>>();
        assert_eq!(c.positions.len() + c.triangles.len(), edges.len());
        save_fixture("generic-curved-through-hole", &m);
    }

    #[test]
    fn generic_atlas_proves_an_authored_region_with_two_holes() {
        let outer = vec![[0., 0.], [8., 0.], [8., 8.], [0., 8.]];
        let holes = vec![
            vec![[1., 1.], [1., 2.], [2., 2.], [2., 1.]],
            vec![[5., 2.], [5., 4.], [6., 4.], [6., 2.]],
        ];
        let m = crate::extrude_polygon_with_holes(&outer, &holes, 0., 2.).unwrap();
        let c = derive(&m, 0).unwrap();
        verify_source_atlas(&m, &c).unwrap();
        let edges = c
            .triangles
            .iter()
            .flat_map(|t| (0..3).map(move |k| (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]))))
            .collect::<BTreeSet<_>>();
        assert_eq!(
            c.positions.len() as i64 - edges.len() as i64 + c.triangles.len() as i64,
            -2
        );
        save_fixture("generic-two-holed-region", &m);
    }
