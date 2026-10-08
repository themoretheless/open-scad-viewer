    use super::*;
    #[test]
    fn source_identity_checks_geometry_and_topology_independently_of_naming_history() {
        let section=|z|vec![vec![Curve::from_polyline(vec![vec![0.,0.,z],vec![1.,0.,z],vec![1.,1.,z],vec![0.,1.,z],vec![0.,0.,z]]).unwrap()]];
        let mut sections=vec![section(0.),section(2.)];
        let mut model=rational_section_loft(&sections).unwrap();
        model.1.change_set.changes[0].provenance.operation="independent-source-history".into();
        let before=model.clone();
        assert!(section_loft_source_matches(&model,&sections,false).unwrap());
        for ring in &mut sections[1] {for curve in ring {for p in &mut curve.control_points {p[2]+=0.125;}}}
        assert!(!section_loft_source_matches(&model,&sections,false).unwrap());
        assert_eq!(model,before);
    }
    #[test]
    fn periodic_hollow_sections_decompose_without_bypassing_join_or_orientation_checks() {
        let outer = Curve { degree: 2, knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![vec![1.,0.,0.],vec![0.,1.,0.],vec![-1.,0.,0.],
                vec![0.,-1.,0.],vec![1.,0.,0.],vec![0.,1.,0.]],
            weights: vec![1.;6], periodic: true };
        let mut hole = outer.clone();
        hole.control_points.reverse();
        for point in &mut hole.control_points { for x in point { *x *= 0.25; } }
        let start = vec![vec![outer], vec![hole]];
        let mut end = start.clone();
        for curve in end.iter_mut().flatten() { for point in &mut curve.control_points { point[2] += 10.; } }
        let sections = vec![start,end];
        let before = sections.clone();
        let model = rational_section_loft(&sections).unwrap();
        model.validate().unwrap();
        assert_eq!(model.faces.len(),10);
        assert_eq!(model.edges.len(),24);
        assert_eq!(model.faces.iter().filter(|f| !f.holes.is_empty()).count(),2);
        let mut wrong_orientation = sections.clone();
        for section in &mut wrong_orientation { section[1][0].control_points.reverse(); }
        assert!(rational_section_loft(&wrong_orientation).is_err());
        let mut open = sections.clone();
        open[1][0][0].periodic = false;
        open[1][0][0].control_points[5][0] += 0.125;
        assert!(rational_section_loft(&open).is_err());
        assert_eq!(sections,before);
    }
    #[test]
    fn segmented_curve_retains_coefficients_without_homogeneous_rounding() {
        let c = Curve {
            degree: 2,
            knots: vec![3., 3., 3., 5., 5., 7., 7., 7.],
            control_points: vec![
                vec![0.13, 0., 0.],
                vec![0.27, 1., 0.],
                vec![0.39, 2., 0.],
                vec![0.51, 1., 0.],
                vec![0.67, 0., 0.],
            ],
            weights: vec![0.7, 1.3, 0.9, 1.7, 0.8],
            periodic: false,
        };
        let pieces = retained_bezier_pieces(&c).unwrap();
        assert_eq!(pieces.len(), 2);
        for (j, p) in pieces.iter().enumerate() {
            assert_eq!(p.control_points, c.control_points[2 * j..2 * j + 3]);
            assert_eq!(p.weights, c.weights[2 * j..2 * j + 3]);
            for t in [0.13, 0.37, 0.81] {
                let expected = c.evaluate(3. + 2. * j as f64 + 2. * t).unwrap().point;
                let actual = p.evaluate(t).unwrap().point;
                for k in 0..3 {
                    assert!((expected[k] - actual[k]).abs() < 1e-12);
                }
            }
        }
    }
