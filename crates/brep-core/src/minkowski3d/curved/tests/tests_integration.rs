    use super::*;
    #[test]
    fn exact_multispan_restriction_and_unproved_seams_refuse() {
        let mut m = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        m.faces[1].surface = nurbs_core::editing::surface_edit::insert(
            &m.faces[1].surface,
            nurbs_core::surface::Axis::U,
            0.5,
            1,
        )
        .unwrap();
        m.rebuild_topology_ids();
        let (_, tubes, e) = geometry(&m, 0, &integer(1), &mut Work::default(), 2).unwrap();
        assert!(tubes.is_empty());
        assert!(e.is_zero());
        m.faces[1].surface.control_points[1][0][0] += 1e-9;
        assert!(
            geometry(&m, 0, &integer(1), &mut Work::default(), 2)
                .err()
                .unwrap()
                .message
                .contains("seams")
        );
    }
    #[test]
    fn curved_trim_is_source_bound_and_invalid_trims_do_not_fall_back() {
        let model = crate::cylinder(1., 2.).unwrap();
        let source = Source::Brep(model.clone());
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        let r = certify(&source, &b, 2., true).unwrap();
        assert!(r.nominal.volume_mm3 > 0.);
        verify(&source, &b, &r.certificate, true).unwrap();
        assert!(r.certificate.source_topology.iter().all(Option::is_some));
        assert!(r.approximation_topology().is_some());
        let mut invalid = model;
        let outer = invalid.faces[4].outer;
        invalid.loops[outer].coedges[0].pcurve.control_points[1][0] = 2.;
        assert!(certify(&Source::Brep(invalid), &b, 2., false).is_err());
    }
    #[test]
    fn critical_f32_coplanar_rounding_is_proved_without_claiming_sum_topology() {
        let a=Source::Brep(crate::tube(2.,1.,2.).unwrap());
        let mut b=super::super::tests::cube([0,0,0],[1,1,1]);
        for p in b.positions.chunks_exact_mut(3) {p[0]*=1.9;p[1]*=0.5;p[2]*=0.25;}
        let result=certify(&a,&Source::Mesh(b),100.,true).unwrap();
        assert_eq!(result.nominal.volume_mm3,41.7375);
        assert!(!result.nominal.rounding_topology.identity);
        assert!(result.nominal.rounding_topology.separated>0);
        assert_eq!(result.source_approximation_topology(),[true,true]);
        assert!(result.approximation_topology().is_none());
    }
    #[test]
    fn holed_and_nonconvex_trimmed_winding_is_not_convexified() {
        let tube = Source::Brep(crate::tube(2., 1., 2.).unwrap());
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        let mut small = super::super::tests::cube([0, 0, 0], [1, 1, 1]);
        for x in &mut small.positions {
            *x *= 0.25
        }
        let small = Source::Mesh(small);
        let r = certify(&tube, &small, 4., false).unwrap();
        assert!(r.nominal.volume_mm3 > 0.);
        assert!(r.certificate.source_topology[0].is_none());
        assert_eq!(
            r.approximation_topology().unwrap()["survivingHoles"].as_u64(),
            Some(1)
        );
        verify(&tube, &small, &r.certificate, false).unwrap();
        let display = input(&SourceMesh {
            positions: r.nominal.positions.clone(),
            indices: r.nominal.indices.clone(),
        })
        .unwrap();
        assert_eq!(
            winding(
                &[
                    Q::new(1.into(), 8.into()),
                    Q::new(1.into(), 8.into()),
                    integer(1)
                ],
                &display.faces,
                &mut Work::default()
            )
            .unwrap(),
            0
        );
        let l = crate::extrude_polygon(
            &[[0., 0.], [4., 0.], [4., 1.], [1., 1.], [1., 4.], [0., 4.]],
            0.,
            2.,
        )
        .unwrap();
        let r = certify(&Source::Brep(l), &b, 10., false).unwrap();
        assert_eq!(r.nominal.volume_mm3, 48.);
        assert_eq!(r.maximum_set_deviation().unwrap().1, 0.);
    }
    #[test]
    fn topology_receipt_is_checked_and_legacy_enclosures_still_verify() {
        let a = Source::Brep(crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap());
        let b = a.clone();
        let r = certify(&a, &b, 0.1, false).unwrap();
        let mut bad = r.certificate.clone();
        bad.source_topology[0].as_mut().unwrap().center[0] = ["999".into(), "1".into()];
        assert!(verify(&a, &b, &bad, false).is_err());
        let mut bad = r.certificate.clone();
        let SourceAtlasTopologyReport::Prismatic(atlas) =
            bad.source_atlas_topology[0].as_mut().unwrap()
        else {
            panic!("expected atlas")
        };
        atlas.shells[0].coefficients[0] = -1;
        assert!(verify(&a, &b, &bad, false).is_err());
        let legacy_v3 = derive(&a, &b, [0, 0], 0.1, 3).unwrap();
        verify(&a, &b, &legacy_v3, false).unwrap();
        let legacy_v2 = derive(&a, &b, [0, 0], 0.1, 2).unwrap();
        verify(&a, &b, &legacy_v2, false).unwrap();
        let legacy = derive(&a, &b, [0, 0], 0.1, 1).unwrap();
        assert!(legacy.source_topology.iter().all(Option::is_none));
        verify(&a, &b, &legacy, false).unwrap();
    }
    #[test]
    fn nonexact_spherical_equator_refuses_before_approximation_search() {
        let sphere = crate::sphere(1.).unwrap();
        let error = trimmed::preflight(&sphere, &mut Work::default()).unwrap_err();
        assert_eq!(error.code, trimmed::SOURCE_SEAM_CODE);
        let a = Source::Brep(sphere);
        let b = Source::Mesh(super::super::tests::cube([0, 0, 0], [1, 1, 1]));
        assert_eq!(
            certify(&a, &b, 1., true).unwrap_err().code,
            trimmed::SOURCE_SEAM_CODE
        );
    }
    #[test]
    fn curved_triangular_trim_uses_the_original_tensor_composition() {
        let mut m = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let s = &mut m.faces[1].surface;
        s.degree_u = 2;
        s.degree_v = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.knots_v = s.knots_u.clone();
        s.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, if i == 1 && j == 1 { 3. } else { 2. }])
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; 3]; 3];
        let outer = m.faces[1].outer;
        let old = m.loops[outer].coedges.clone();
        let start = m.edges[old[0].edge].vertices[usize::from(old[0].reversed)];
        let end = m.edges[old[1].edge].vertices[usize::from(!old[1].reversed)];
        let edge = m.edges.len();
        m.edges.push(crate::Edge {
            degenerate: false,
            vertices: [start, end],
            curve: crate::Curve {
                degree: 4,
                knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
                weights: vec![1.; 5],
                control_points: vec![
                    vec![0., 0., 2.],
                    vec![0.5, 0.5, 2.],
                    vec![1., 1., 8. / 3.],
                    vec![1.5, 1.5, 2.],
                    vec![2., 2., 2.],
                ],
                periodic: false,
            },
        });
        let pc = crate::Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
        m.loops[outer].coedges = vec![
            old[0].clone(),
            old[1].clone(),
            crate::Coedge::new(
                edge,
                true,
                pc.reverse().unwrap(),
            ),
        ];
        let second = m.loops.len();
        m.loops.push(crate::Loop {
            coedges: vec![
                crate::Coedge::new(
                    edge,
                    false,
                    pc,
                ),
                old[2].clone(),
                old[3].clone(),
            ],
        });
        let face = m.faces.len();
        let surface = m.faces[1].surface.clone();
        m.faces.push(crate::Face {
            surface,
            outer: second,
            holes: vec![],
        });
        m.shells[0].faces.push(crate::FaceUse {
            face,
            reversed: false,
        });
        m.rebuild_topology_ids();
        let a = Source::Brep(m);
        let b = Source::Mesh(super::super::tests::cube([-1, -1, -1], [1, 1, 1]));
        let r = certify(&a, &b, 1., true).unwrap();
        assert!(r.nominal.volume_mm3 >= 64.);
        assert!(r.approximation_topology().is_some());
        verify(&a, &b, &r.certificate, true).unwrap();
    }
    #[test]
    fn exact_original_planar_brep_sum() {
        let a = Source::Brep(crate::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap());
        let b = Source::Brep(crate::cuboid([-1., -1., -1.], [1., 1., 1.]).unwrap());
        let r = certify(&a, &b, 0.1, false).unwrap();
        assert!(r.approximation_topology().is_some());
        assert_eq!(r.nominal.volume_mm3, 120.);
        assert_eq!(
            r.certificate.filled_hausdorff_bound,
            Some(["0".into(), "1".into()])
        );
        verify(&a, &b, &r.certificate, false).unwrap();
        let mut bad = r.certificate;
        bad.source_homotopy_bounds[0] = ["1".into(), "1".into()];
        assert!(verify(&a, &b, &bad, false).is_err())
    }
    #[test]
    fn rational_surface_homotopy_encloses_the_original_body() {
        let mut model = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
        let s = &mut model.faces[1].surface;
        s.degree_u = 2;
        s.degree_v = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.knots_v = s.knots_u.clone();
        s.control_points = (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64, j as f64, if i == 1 && j == 1 { 3. } else { 2. }])
                    .collect()
            })
            .collect();
        s.weights = vec![vec![1.; 3]; 3];
        s.weights[1][1] = 2.;
        model.rebuild_topology_ids();
        let a = Source::Brep(model);
        let b = Source::Mesh(super::super::tests::cube([-1, -1, -1], [1, 1, 1]));
        let r = certify(&a, &b, 1., false).unwrap();
        assert!(r.approximation_topology().is_some());
        assert_ne!(r.certificate.source_homotopy_bounds[0], ["0", "1"]);
        assert!(r.nominal.volume_mm3 >= 64.);
        assert!(r.maximum_set_deviation().is_some());
        assert!(r.source_surface_deviations_mm()[0] <= 1.);
        verify(&a, &b, &r.certificate, false).unwrap();
        let rounded = certify(&a, &b, 1., true).unwrap();
        assert_eq!(rounded.nominal.rounding_topology.precision, "f32");
        verify(&a, &b, &rounded.certificate, true).unwrap();
        let mut bad = r.certificate;
        bad.outer.vertices[0][0][0] = "17".into();
        assert!(verify(&a, &b, &bad, false).is_err())
    }
