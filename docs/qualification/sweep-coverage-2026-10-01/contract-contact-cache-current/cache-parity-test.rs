
    #[test]
    fn immutable_control_patch_cache_matches_original_query_bits_after_eviction() {
        let mut a = plane(0.);
        a.weights = vec![vec![1., 0.75], vec![1.5, 2.]];
        let before = a.clone();
        for z in [0., 0.0001, 3.] {
            let mut b = plane(z);
            for row in &mut b.control_points { for p in row { p[0] += p[1] * 0.7; p[2] += p[0] * 0.1; } }
            let mut ca = PreparedControlSurface::new(&a);
            let mut cb = PreparedControlSurface::new(&b);
            let bits = |x: Option<(f64,[f64;4])>| x.map(|(g,s)|(g.to_bits(),s.map(f64::to_bits)));
            for i in (0..80).chain((0..80).rev()) {
                let lo = i as f64 / 160.;
                let da = [[lo, lo+0.5], [0.,1.]];
                let db = [[0.,1.], [0.,1.]];
                assert_eq!(bits(rectangle_control_query(&a,&b,[da,db]).unwrap()),bits(rectangle_control_query_prepared(&mut ca,&mut cb,[da,db]).unwrap()));
                assert!(ca.entries.len()<=32 && cb.entries.len()<=32);
                assert!(ca.controls<=2048 && cb.controls<=2048);
                assert_eq!(bits(rectangle_control_query(&b,&a,[db,da]).unwrap()),bits(rectangle_control_query_prepared(&mut cb,&mut ca,[db,da]).unwrap()));
            }
        }
        assert_eq!(a.control_points,before.control_points);
        assert_eq!(a.weights,before.weights);
        assert_eq!(a.knots_u,before.knots_u);
        assert_eq!(a.knots_v,before.knots_v);
    }
