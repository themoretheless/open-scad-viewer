use nurbs_core::{
    affine,
    bounds::{self, Bounds},
    curve::Curve,
    surface::Surface,
};
const M: affine::Matrix = [
    [-2., 0.5, 0., 3.],
    [0., 1., 1., -4.],
    [1., 0., 0.25, 2.],
    [0., 0., 0., 1.],
];
fn apply(p: [f64; 3]) -> [f64; 3] {
    [
        -2. * p[0] + 0.5 * p[1] + 3.,
        p[1] + p[2] - 4.,
        p[0] + 0.25 * p[2] + 2.,
    ]
}
fn near(a: &[f64], b: &[f64]) {
    for (x, y) in a.iter().zip(b) {
        assert!((x - y).abs() < 1e-10);
    }
}
fn inside(b: &Bounds, p: &[f64]) {
    for (i, x) in p.iter().enumerate() {
        assert!(*x >= b.min[i] - 1e-12 && *x <= b.max[i] + 1e-12);
    }
}
#[test]
fn weighted_surface_affine_commutes_with_independent_rational_equation() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-4., -4., 2., 2.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 1.]],
            vec![vec![3., 0., 1.], vec![3., 2., 4.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 4.]],
        periodic_u: false,
        periodic_v: false,
    };
    let t = affine::surface(&s, &M).unwrap();
    assert_eq!(t.weights, s.weights);
    assert_eq!(t.knots_u, s.knots_u);
    assert_eq!(t.knots_v, s.knots_v);
    let b = t.bounds().unwrap();
    for i in 0..=40 {
        for j in 0..=40 {
            let u = i as f64 / 40.;
            let v = j as f64 / 40.;
            let factors = [
                (1. - u) * (1. - v),
                2. * (1. - u) * v,
                3. * u * (1. - v),
                4. * u * v,
            ];
            let w = factors.iter().sum::<f64>();
            let p = [
                (3. * factors[2] + 3. * factors[3]) / w,
                (2. * factors[1] + 2. * factors[3]) / w,
                (factors[1] + factors[2] + 4. * factors[3]) / w,
            ];
            let q = t.evaluate(2. + 3. * u, -4. + 6. * v).unwrap().point;
            near(&q, &apply(p));
            inside(&b, &q);
        }
    }
    let singular = [
        [0., 0., 0., 7.],
        [0., 0., 0., 2.],
        [0., 0., 0., 3.],
        [0., 0., 0., 1.],
    ];
    let collapsed = affine::surface(&s, &singular).unwrap();
    near(&collapsed.evaluate(3., 0.).unwrap().point, &[7., 2., 3.]);
    assert_eq!(
        collapsed.bounds().unwrap(),
        Bounds {
            min: vec![7., 2., 3.],
            max: vec![7., 2., 3.]
        }
    );
}
#[test]
fn periodic_curve_transform_preserves_encoding_and_hull_bounds() {
    let c = Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
            vec![-0.5, 1., 0.],
            vec![-1., 0., 0.],
            vec![-0.5, -1., 0.],
            vec![0.5, -1., 0.],
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
        ],
        weights: vec![1., 2., 1., 2., 1., 2., 1., 2.],
        periodic: true,
    };
    let t = affine::curve(&c, &M).unwrap();
    assert!(t.periodic);
    assert_eq!(t.weights, c.weights);
    assert_eq!(t.knots, c.knots);
    let b = t.bounds().unwrap();
    for i in 0..=400 {
        let u = 2. + 6. * i as f64 / 400.;
        let p = c.evaluate(u).unwrap().point;
        let q = t.evaluate(u).unwrap().point;
        near(&q, &apply([p[0], p[1], p[2]]));
        inside(&b, &q);
    }
    let mut invalid = c;
    invalid.weights[0] = -1.;
    assert!(invalid.bounds().is_err());
    assert!(affine::curve(&invalid, &M).is_err());
}
#[test]
fn union_is_exact_and_validates_all_boxes() {
    let a = Bounds {
        min: vec![-3., 2., 0.],
        max: vec![-1., 2., 4.],
    };
    let b = Bounds {
        min: vec![1., -2., 3.],
        max: vec![7., 8., 3.],
    };
    assert_eq!(
        bounds::union(&[a.clone(), b]).unwrap(),
        Bounds {
            min: vec![-3., -2., 0.],
            max: vec![7., 8., 4.]
        }
    );
    assert_eq!(bounds::union(&vec![a.clone(); 2048]).unwrap(), a);
    assert!(bounds::union(&vec![a.clone(); 2049]).is_err());
    assert!(bounds::union(&[]).is_err());
    for bad in [
        Bounds {
            min: vec![1., 2., 3.],
            max: vec![0., 2., 3.],
        },
        Bounds {
            min: vec![f64::NAN, 2., 3.],
            max: vec![4., 5., 6.],
        },
        Bounds {
            min: vec![0., 0.],
            max: vec![1., 1.],
        },
        Bounds {
            min: vec![0.; 3],
            max: vec![1.; 2],
        },
    ] {
        assert!(bounds::union(&[a.clone(), bad]).is_err());
    }
}
