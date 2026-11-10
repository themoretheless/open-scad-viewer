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
