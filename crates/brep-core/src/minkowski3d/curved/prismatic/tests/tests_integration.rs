    use super::*;
    fn region_for(m: Model) -> (Source, Vec<Poly>, Report, Option<Data>) {
        let s = Source::Brep(m);
        let Source::Brep(m) = &s else { unreachable!() };
        let (i, _, _) = geometry(m, 0, &integer(4), &mut Work::default(), 3).unwrap();
        let n = decompose(&i, &mut Work::default()).unwrap();
        let (r, d) = inspect(&s, 0, &n, &mut Work::default()).expect("signed prism source proof");
        (s, n, r, d)
    }
    fn sum_for(m: Model, size: [f64; 2]) -> Option<SumReport> {
        let (a, an, _, ad) = region_for(m);
        let mut k = super::super::super::tests::cube([0, 0, 0], [1, 1, 1]);
        for p in k.positions.chunks_mut(3) {
            p[0] *= size[0];
            p[1] *= size[1];
            p[2] *= 0.25;
        }
        let b = Source::Mesh(k.clone());
        let bn = decompose(&input(&k).unwrap(), &mut Work::default()).unwrap();
        let material = sum_material(&an, &bn, &mut Work::default()).unwrap();
        let c = certificate_for(
            &material,
            ["a".into(), "b".into()],
            [an.len(), bn.len()],
            an.len() * bn.len(),
            &mut Work::default(),
        )
        .unwrap();
        sum(&a, &b, &an, &bn, ad.as_ref(), None, &c)
    }
    #[test]
    fn curved_through_hole_has_a_global_ambient_source_proof() {
        let m = crate::tube(2., 1., 2.).unwrap();
        save_fixture("curved-through-hole", &m);
        let (_, _, r, d) = region_for(m);
        assert_eq!(r.parents.iter().filter(|p| p.is_some()).count(), 1);
        assert_eq!(r.shells.len(), 1);
        assert!(d.is_some());
    }
    #[test]
    fn nonstar_c_profile_does_not_need_a_radial_kernel() {
        let m = crate::extrude_polygon(
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
        .unwrap();
        save_fixture("nonstar-c", &m);
        let (_, _, r, _) = region_for(m);
        assert_eq!(r.parents, vec![None]);
    }
    #[test]
    fn multiple_holes_and_a_permuted_height_axis_keep_their_forest() {
        let mut m = crate::extrude_polygon_with_holes(
            &[[0., 0.], [8., 0.], [8., 4.], [0., 4.]],
            &[
                vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]],
                vec![[5., 1.], [5., 3.], [7., 3.], [7., 1.]],
            ],
            0.,
            2.,
        )
        .unwrap();
        for v in &mut m.vertices {
            v.point = [v.point[2], v.point[0], v.point[1]];
        }
        for e in &mut m.edges {
            for p in &mut e.curve.control_points {
                p.rotate_right(1);
            }
        }
        for f in &mut m.faces {
            for p in f.surface.control_points.iter_mut().flatten() {
                p.rotate_right(1);
            }
        }
        m.rebuild_topology_ids();
        save_fixture("two-holes-permuted", &m);
        let (_, _, r, _) = region_for(m);
        assert_eq!(r.axis, 0);
        assert_eq!(r.parents.iter().filter(|p| p.is_some()).count(), 2);
    }
    #[test]
    fn minkowski_checks_both_surviving_and_filled_curved_holes() {
        let r = sum_for(crate::tube(2., 1., 2.).unwrap(), [0.25, 0.25]).expect("surviving hole");
        assert_eq!(r.surviving_holes, 1);
        assert!(r.holes[0].open);
        let r = sum_for(crate::tube(2., 1., 2.).unwrap(), [3., 3.]).expect("filled hole");
        assert_eq!(r.surviving_holes, 0);
        assert!(!r.holes[0].open);
    }
    #[test]
    fn actual_curved_hole_can_survive_when_coarse_sum_fills_it() {
        assert!(sum_for(crate::tube(2., 1., 2.).unwrap(), [1.9, 0.5]).is_none());
    }
    #[test]
    fn source_twist_does_not_pass_as_a_cartesian_prism() {
        let mut m = crate::cylinder(1., 2.).unwrap();
        m.faces[0].surface.control_points[1][1][0] += 0.125;
        assert!(!ruled(
            &m.faces[0].surface,
            2,
            &[Q::zero(), integer(2)],
            false
        ));
    }
    #[test]
    fn a_sealed_curved_cavity_keeps_all_signed_shell_memberships() {
        let mut outer = crate::cylinder(2., 4.).unwrap();
        let mut inner = crate::cylinder(1., 2.).unwrap();
        for v in &mut inner.vertices {
            v.point[2] += 1.;
        }
        for e in &mut inner.edges {
            for p in &mut e.curve.control_points {
                p[2] += 1.;
            }
        }
        for f in &mut inner.faces {
            for p in f.surface.control_points.iter_mut().flatten() {
                p[2] += 1.;
            }
        }
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
        save_fixture("sealed-curved-cavity", &outer);
        let (_, _, r, d) = region_for(outer);
        assert!(d.is_none());
        assert_eq!(r.shells.len(), 2);
        assert_eq!(r.levels.len(), 4);
        assert_eq!(r.shells[0].coefficients, vec![1]);
        assert_eq!(r.shells[1].coefficients, vec![-1]);
        assert_eq!(r.parents, vec![None, Some(0)]);
    }
