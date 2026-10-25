    use super::*;
    #[test]
    fn multispan_hull_separation_does_not_require_impossible_fair_share_cells() {
        let points = [[0., 0.], [2., 0.], [2., 2.], [0., 2.]];
        let curves = (0..4)
            .map(|i| {
                let a = points[i];
                let b = points[(i + 1) % 4];
                Curve {
                    degree: 1,
                    knots: vec![0., 0., 0.5, 1., 1.],
                    control_points: vec![
                        a.to_vec(),
                        vec![(a[0] + b[0]) / 2., (a[1] + b[1]) / 2.],
                        b.to_vec(),
                    ],
                    weights: vec![1., 1., 1.],
                    periodic: false,
                }
            })
            .collect::<Vec<_>>();
        let r = inspect(&curves, 1e-6, 6, 2).unwrap();
        assert!(r.proven_simple);
        assert_eq!(r.cells, 2);
        let mut rotated = curves.clone();
        for c in &mut rotated {
            for p in &mut c.control_points {
                let (x, y) = (p[0], p[1]);
                p[0] = x - y;
                p[1] = x + y;
            }
        }
        assert!(!inspect(&rotated, 1e-6, 6, 1).unwrap().proven_simple);
        assert!(inspect(&rotated, 1e-6, 6, 100).unwrap().proven_simple);
        let mut crossing = curves.clone();
        crossing[2].control_points[1] = vec![1., -1.];
        assert!(!inspect(&crossing, 1e-6, 6, 1).unwrap().proven_simple);
    }
    #[test]
    fn normalized_separators_check_whole_rational_control_cones() {
        let a = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.25, 0.5], vec![0.5, 0.25], vec![1., 1.]],
            weights: vec![1., 0.5, 0.5, 1.],
            periodic: false,
        };
        let mut b = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.05, 0.5], vec![0., 1.]],
            weights: vec![1., 0.7, 1.],
            periodic: false,
        };
        assert!(control_cones_separated(&a, &b, &[0., 0.]));
        assert!(control_cones_separated(&b, &a, &[0., 0.]));
        // Same endpoint and similar tangent are insufficient: a later pole
        // overlaps the other control cone and must prevent this certificate.
        b.control_points[1] = vec![0.9, 0.5];
        assert!(!control_cones_separated(&a, &b, &[0., 0.]));
        assert!(!control_cones_separated(&a, &a, &[0., 0.]));
    }
    fn polygon(p: &[[f64; 2]]) -> Vec<Curve> {
        (0..p.len())
            .map(|i| {
                Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % p.len()].to_vec()]).unwrap()
            })
            .collect()
    }
    #[test]
    fn square_bowtie_and_incomplete_work_are_distinct() {
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        assert!(inspect(&square, 1e-8, 100, 1000).unwrap().proven_simple);
        let crossed = polygon(&[[0., 0.], [1., 1.], [0., 1.], [1., 0.]]);
        assert!(!inspect(&crossed, 1e-8, 100, 1000).unwrap().proven_simple);
        let r = inspect(&square, 1e-8, 1, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.pairs.len(), 1);
        assert_eq!(r.total_pairs, 6);
    }
    #[test]
    fn coincident_edges_and_small_work_never_bypass_pair_checks() {
        let retraced = polygon(&[[0., 0.], [1., 0.], [0., 0.], [0., 1.]]);
        assert!(!inspect(&retraced, 1e-8, 100, 1000).unwrap().proven_simple);
        let square = polygon(&[[0., 0.], [1., 0.], [1., 1.], [0., 1.]]);
        let r = inspect(&square, 1e-8, 100, 1).unwrap();
        assert!(!r.proven_simple);
        assert!(r.cells <= 1);
        assert_eq!(r.pairs.len(), 6);
        let mut gap = square.clone();
        gap[1].control_points[0][1] = 1e-12;
        let r = inspect(&gap, 1e-6, 100, 1000).unwrap();
        assert!(!r.proven_simple);
        assert_eq!(r.exact_joins, Some(false));
    }
    #[test]
    fn rational_quarter_arcs_form_a_simple_loop() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        let curves = points
            .iter()
            .map(|p| Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: p.iter().map(|v| v.to_vec()).collect(),
                weights: vec![1., 0.5_f64.sqrt(), 1.],
                periodic: false,
            })
            .collect::<Vec<_>>();
        let before = format!("{curves:?}");
        let r = inspect(&curves, 1e-8, 100, 10000).unwrap();
        assert!(r.proven_simple);
        assert!(r.cells <= 10000);
        assert_eq!(format!("{curves:?}"), before);
    }
    #[test]
    fn rotated_rational_arcs_use_interval_separating_directions() {
        let points = [
            [[1., 0.], [1., 1.], [0., 1.]],
            [[0., 1.], [-1., 1.], [-1., 0.]],
            [[-1., 0.], [-1., -1.], [0., -1.]],
            [[0., -1.], [1., -1.], [1., 0.]],
        ];
        for angle in [0.13_f64, 0.61, 1.17, 2.31] {
            let (sin, cos) = angle.sin_cos();
            let curves = points
                .iter()
                .map(|points| Curve {
                    degree: 2,
                    knots: vec![0., 0., 0., 1., 1., 1.],
                    control_points: points
                        .iter()
                        .map(|p| vec![cos * p[0] - sin * p[1], sin * p[0] + cos * p[1]])
                        .collect(),
                    weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
                    periodic: false,
                })
                .collect::<Vec<_>>();
            assert!(inspect(&curves, 1e-10, 100, 10000).unwrap().proven_simple);
        }
    }

    #[test]
    fn rational_arc_and_chord_require_owned_endpoints_and_strict_open_side() {
        let arc=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],weights:vec![1.,0.5,1.],periodic:false};
        let chord=Curve::from_polyline(vec![vec![1.,1.],vec![0.,0.]]).unwrap();
        let two=vec![arc.clone(),chord];let before=format!("{two:?}");
        let proof=inspect(&two,1e-8,100,1000).unwrap();assert!(proof.proven_simple);assert!(proof.cells>0 && proof.cells<=1000);
        assert!(!inspect(&two,1e-8,100,1).unwrap().proven_simple);
        let reversed=two.iter().rev().map(Curve::reverse).collect::<Result<Vec<_>>>().unwrap();assert!(inspect(&reversed,1e-8,100,1000).unwrap().proven_simple);
        let mut rotated=two.clone();for c in &mut rotated {for p in &mut c.control_points {let [x,y]=[p[0],p[1]];p[0]=1024.+x-y;p[1]=-2048.+x+y;}}
        assert!(inspect(&rotated,1e-8,100,1000).unwrap().proven_simple);
        let three=vec![arc.clone(),Curve::from_polyline(vec![vec![1.,1.],vec![0.5,0.5]]).unwrap(),Curve::from_polyline(vec![vec![0.5,0.5],vec![0.,0.]]).unwrap()];
        assert!(inspect(&three,1e-8,100,1000).unwrap().proven_simple);
        let crossed=vec![arc.clone(),Curve::from_polyline(vec![vec![1.,1.],vec![-1.,-1.]]).unwrap(),Curve::from_polyline(vec![vec![-1.,-1.],vec![0.,0.]]).unwrap()];
        assert!(!inspect(&crossed,1e-8,100,1000).unwrap().proven_simple);
        let mut coincident=two.clone();coincident[0].control_points[1]=vec![0.5,0.5];assert!(!inspect(&coincident,1e-8,100,1000).unwrap().proven_simple);
        assert_eq!(format!("{two:?}"),before);
    }
