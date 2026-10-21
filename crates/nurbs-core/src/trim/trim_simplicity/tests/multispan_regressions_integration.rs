    use super::*;
    #[test]
    fn multispan_hull_separation_keeps_small_budget_and_refuses_exhaustion() {
        let corners = [[0.,0.],[2.,0.],[2.,2.],[0.,2.]];
        let curves = (0..4).map(|i| {
            let a = corners[i]; let b = corners[(i+1)%4];
            let steps = if i%2==0 {4} else {1};
            Curve::from_polyline((0..=steps).map(|j| {
                let t = j as f64/steps as f64;
                vec![a[0]+t*(b[0]-a[0]),a[1]+t*(b[1]-a[1])]
            }).collect()).unwrap()
        }).collect::<Vec<_>>();
        let before = format!("{curves:?}");
        let short = inspect(&curves,1e-8,100,32).unwrap();
        assert!(short.injective.iter().all(|v|*v));
        assert!(short.proven_simple);
        assert!(short.cells<=32);
        let exhausted=inspect(&curves,1e-8,100,1).unwrap();
        assert!(!exhausted.proven_simple);assert!(exhausted.cells<=1);
        assert!(inspect(&curves,1e-8,100,10000).unwrap().proven_simple);
        assert_eq!(format!("{curves:?}"),before);
    }
    fn polygon(p: &[[f64; 2]]) -> Vec<Curve> {
        (0..p.len())
            .map(|i| {
                Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % p.len()].to_vec()]).unwrap()
            })
            .collect()
    }
    #[test]
    fn multispan_rational_boundary_preserves_original_definition() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![0.5, -0.5], vec![1.5, -0.5], vec![2., 0.]],
            weights: vec![1., 0.8, 1.2, 1.],
            periodic: false,
        };
        let mut curves = polygon(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.]]);
        curves[0] = curve;
        let before = format!("{curves:?}");
        let r = inspect(&curves, 1e-8, 100, 10000).unwrap();
        assert!(r.proven_simple);
        assert!(r.cells > 0 && r.cells <= 10000);
        let reversed = curves
            .iter()
            .rev()
            .map(Curve::reverse)
            .collect::<Result<Vec<_>>>()
            .unwrap();
        assert!(inspect(&reversed, 1e-8, 100, 10000).unwrap().proven_simple);
        assert_eq!(format!("{curves:?}"), before);
        assert!(!inspect(&curves, 1e-8, 100, 1).unwrap().proven_simple);
    }
    #[test]
    fn multispan_returning_boundary_never_receives_injectivity_proof() {
        let curve =
            Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.], vec![1., 0.], vec![3., 0.]])
                .unwrap();
        let mut curves = polygon(&[[0., 0.], [3., 0.], [3., 2.], [0., 2.]]);
        curves[0] = curve;
        let r = inspect(&curves, 1e-8, 100, 10000).unwrap();
        assert!(!r.proven_simple);
        assert!(!r.injective[0]);
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
