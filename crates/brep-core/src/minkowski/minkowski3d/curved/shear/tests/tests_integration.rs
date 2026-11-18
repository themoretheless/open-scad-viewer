    use super::*;
    /// Construct Bernstein coefficients of the composition, rather than
    /// transforming control points (which would not represent F(surface)).
    fn deform(mut m: Model, f: &Poly2) -> Model {
        // Degree-five affine coordinates require lengths divisible by five
        // for exactly dyadic Bernstein controls.
        for face in &mut m.0.faces {
            for p in face.surface.control_points.iter_mut().flatten() {
                for q in p {
                    *q *= 5.;
                }
            }
        }
        for v in &mut m.vertices {
            for q in &mut v.point {
                *q *= 5.;
            }
        }
        for e in &mut m.edges {
            for p in &mut e.curve.control_points {
                for q in p {
                    *q *= 5.;
                }
            }
        }
        for face in &mut m.0.faces {
            // Normalize the parameter domain for this exact Bernstein fixture;
            // authored extrusion caps may retain their physical UV extent.
            let surface = &mut face.surface;
            let d = [[surface.knots_u[surface.degree_u],surface.knots_u[surface.control_points.len()]],
                [surface.knots_v[surface.degree_v],surface.knots_v[surface.control_points[0].len()]]];
            for knot in &mut surface.knots_u { *knot = (*knot-d[0][0])/(d[0][1]-d[0][0]); }
            for knot in &mut surface.knots_v { *knot = (*knot-d[1][0])/(d[1][1]-d[1][0]); }
            for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                for coedge in &mut m.0.loops[wire].coedges {
                    for uv in &mut coedge.pcurve.control_points {
                        for axis in 0..2 {uv[axis]=(uv[axis]-d[axis][0])/(d[axis][1]-d[axis][0]);}
                    }
                }
            }
            let s = &face.surface;
            let d = intervals(s);
            assert_eq!(d[0], vec![Q::zero(), Q::one()]);
            assert_eq!(d[1], vec![Q::zero(), Q::one()]);
            let h = sb::cell(s, [Q::zero(), Q::one()], [Q::zero(), Q::one()]).unwrap();
            let x = power(&h, 0).unwrap();
            let y = power(&h, 1).unwrap();
            let z = add(
                &power(&h, 2).unwrap(),
                &compose(f, &x, &y, &mut Work::default()).unwrap(),
            );
            let p = [x, y, z];
            let n = p.iter().map(Vec::len).max().unwrap().max(2) - 1;
            let k = p.iter().map(|p| p[0].len()).max().unwrap().max(2) - 1;
            let control_points = (0..=n)
                .map(|i| {
                    (0..=k)
                        .map(|j| {
                            (0..3)
                                .map(|a| {
                                    let q = (0..=i)
                                        .flat_map(|u| (0..=j).map(move |v| (u, v)))
                                        .map(|(u, v)| {
                                            coefficient(&p[a], u, v) * choose(i, u) / choose(n, u)
                                                * choose(j, v)
                                                / choose(k, v)
                                        })
                                        .sum::<Q>();
                                    let v = q.to_f64().unwrap();
                                    assert_eq!(
                                        Q::from_float(v).unwrap(),
                                        q,
                                        "fixture must be represented exactly"
                                    );
                                    v
                                })
                                .collect()
                        })
                        .collect()
                })
                .collect();
            let knots = |degree: usize| {
                std::iter::repeat_n(0., degree + 1)
                    .chain(std::iter::repeat_n(1., degree + 1))
                    .collect()
            };
            face.surface = nurbs_core::surface::Surface {
                degree_u: n,
                degree_v: k,
                knots_u: knots(n),
                knots_v: knots(k),
                control_points,
                weights: vec![vec![1.; k + 1]; n + 1],
                periodic_u: false,
                periodic_v: false,
            };
        }
        for e in &mut m.edges {
            let c = &mut e.curve;
            assert_eq!(c.degree, 1);
            let a = c.control_points.first().unwrap();
            let b = c.control_points.last().unwrap();
            let mut p = (0..3)
                .map(|k| {
                    vec![
                        vec![Q::from_float(a[k]).unwrap()],
                        vec![Q::from_float(b[k] - a[k]).unwrap()],
                    ]
                })
                .collect::<Vec<_>>();
            p[2] = add(
                &p[2],
                &compose(f, &p[0], &p[1], &mut Work::default()).unwrap(),
            );
            let n = p.iter().map(Vec::len).max().unwrap().max(2) - 1;
            c.control_points = (0..=n)
                .map(|i| {
                    (0..3)
                        .map(|a| {
                            let q = (0..=i)
                                .map(|u| coefficient(&p[a], u, 0) * choose(i, u) / choose(n, u))
                                .sum::<Q>();
                            let v = q.to_f64().unwrap();
                            assert_eq!(Q::from_float(v).unwrap(), q);
                            v
                        })
                        .collect()
                })
                .collect();
            c.degree = n;
            c.knots = std::iter::repeat_n(0., n + 1)
                .chain(std::iter::repeat_n(1., n + 1))
                .collect();
            c.weights = vec![1.; n + 1];
        }
        for v in &mut m.vertices {
            v.point[2] += eval2(
                f,
                &Q::from_float(v.point[0]).unwrap(),
                &Q::from_float(v.point[1]).unwrap(),
            )
            .to_f64()
            .unwrap();
        }
        m.rebuild_topology_ids();
        m
    }
    fn fixture_f() -> Poly2 {
        // Vanishes at every coarse boundary station, but genuinely bends the
        // full original cap between them. All Bernstein controls are dyadic.
        let mut p = constant(Q::new(10.into(), (65536i64 * 15625).into()));
        for i in 0..=4 {
            p = mul(
                &p,
                &affine(-integer(5 * i), Q::one(), Q::zero()),
                &mut Work::default(),
            )
            .unwrap();
        }
        mul(
            &p,
            &affine(Q::zero(), Q::zero(), Q::one()),
            &mut Work::default(),
        )
        .unwrap()
    }
    fn check(m: Model) -> Report {
        let f = fixture_f();
        let m = deform(m, &f);
        save_fixture(
            if m.shells.len() > 1 {
                "sheared-sealed-cavity"
            } else if m.faces.iter().any(|f| !f.holes.is_empty()) {
                "sheared-through-hole"
            } else {
                "sheared-nonstar"
            },
            &m,
        );
        trimmed::preflight(&m, &mut Work::default()).unwrap();
        let (input, _, _) = geometry(&m, 0, &integer(100), &mut Work::default(), 3).unwrap();
        let nominal = decompose(&input, &mut Work::default()).unwrap();
        assert!(
            prismatic::inspect(&Source::Brep(m.clone()), 0, &nominal, &mut Work::default())
                .is_none()
        );
        let r = attempt(&m, &nominal, 2, &f, &mut Work::default()).unwrap();

        assert!(prove(&Source::Brep(m), &nominal, &mut Work::default()).is_some());
        r
    }
    #[test]
    fn freeform_nonstar_source_has_a_global_ambient_inverse() {
        check(
            crate::extrude_polygon(
                &[
                    [0., 0.],
                    [4., 0.],
                    [4., 1.],
                    [1., 1.],
                    [1., 3.],
                    [4., 3.],
                    [4., 4.],
                    [0., 4.],
                ],
                0.,
                2.,
            )
            .unwrap(),
        );
    }
    #[test]
    fn freeform_through_hole_does_not_require_spherical_topology() {
        check(
            crate::extrude_polygon_with_holes(
                &[[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
                &[vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]]],
                0.,
                2.,
            )
            .unwrap(),
        );
    }
    #[test]
    fn freeform_sealed_cavity_preserves_the_complement_component() {
        let mut outer = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
        let inner = crate::cuboid([1., 1., 1.], [3., 3., 3.]).unwrap();
        let (vo, eo, lo, fo, so) = (
            outer.vertices.len(),
            outer.edges.len(),
            outer.loops.len(),
            outer.faces.len(),
            outer.shells.len(),
        );
        outer.0.vertices.extend(inner.vertices.iter().cloned());
        outer
            .0
            .edges
            .extend(inner.edges.iter().cloned().map(|mut e| {
                for v in &mut e.vertices {
                    *v += vo;
                }
                e
            }));
        outer
            .0
            .loops
            .extend(inner.loops.iter().cloned().map(|mut w| {
                for c in &mut w.coedges {
                    c.edge += eo;
                }
                w
            }));
        outer
            .0
            .faces
            .extend(inner.faces.iter().cloned().map(|mut f| {
                f.outer += lo;
                for h in &mut f.holes {
                    *h += lo;
                }
                f
            }));
        outer
            .0
            .shells
            .extend(inner.shells.iter().cloned().map(|mut s| {
                for u in &mut s.faces {
                    u.face += fo;
                    u.reversed = !u.reversed;
                }
                s
            }));
        outer.bodies[0].inner_shells.push(so);
        outer.rebuild_topology_ids();
        outer.validate().unwrap();
        check(outer);
    }
    #[test]
    fn whole_chart_identity_rejects_a_control_point_perturbation() {
        let f = fixture_f();
        let mut m = deform(
            crate::extrude_polygon(&[[0., 0.], [4., 0.], [4., 4.], [0., 4.]], 0., 2.).unwrap(),
            &f,
        );
        assert!(reference_charts(&m, &f, 2, &mut Work::default()).is_ok());
        m.faces.last_mut().unwrap().surface.control_points[1][1][2] += 0.125;
        assert!(reference_charts(&m, &f, 2, &mut Work::default()).is_err());
    }
