use nurbs_core::{editing::exact_surface_derivative, surface::Surface};
#[test]
fn mixed_variable_positive_weights_can_hide_an_exact_affine_plane() {
    // Actual binary64 poles encode S=(30t,30s,3), W=2(1+t+s), where
    // t=(u+2)/4 and s=(v-1)/8. The weight net is not separable by rows/columns.
    let source = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -2., -2., 2., 2., 2.],
        knots_v: vec![1., 1., 1., 9., 9., 9.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let i = i as f64;
                        let j = j as f64;
                        let w = 2. + i + j;
                        vec![15. * i * (2. * i + j) / w, 15. * j * (2. * j + i) / w, 3.]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..3)
            .map(|i| (0..3).map(|j| (2 + i + j) as f64).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    source.validate().unwrap();
    let before = source.clone();
    let r = exact_surface_derivative::area_normal(&source, 1000000).unwrap();
    assert!(r.refusal.is_none(), "{:?}", r.refusal);
    let out = r.patches.unwrap();
    assert_eq!(source, before);
    assert_eq!(out.len(), 1);
    assert_eq!((out[0].degree_u, out[0].degree_v), (1, 1));
    assert_eq!(out[0].weights, vec![vec![1.; 2]; 2]);
    assert_eq!(
        out[0].control_points,
        vec![vec![vec![0., 0., 28.125]; 2]; 2]
    );
    assert_eq!(out[0].knots_u, vec![-2., -2., 2., 2.]);
    assert_eq!(out[0].knots_v, vec![1., 1., 9., 9.]);
    #[cfg(feature = "codec")]
    println!(
        "exact-area-normal-oracle={}",
        value_codec::json!({"source":source,"patches":out})
    );
}
