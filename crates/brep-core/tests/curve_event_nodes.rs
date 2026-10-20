use brep_core::profile_certificate as p;
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
use nurbs_core::curve::Curve;
fn eval(poly: &p::ExactPolynomial, t: &Q) -> Q {
    poly.iter().rev().fold(Q::zero(), |v, c| {
        v * t + Q::new(c[0].parse().unwrap(), c[1].parse().unwrap())
    })
}
fn own(a: &Curve, b: &Curve, self_pair: bool) -> p::CurveEventNodes {
    let c = p::certify_span_intersections(a, b, self_pair, 1e-12).unwrap();
    let n = p::own_curve_event_nodes(a, b, &c).unwrap();
    n.reverify_source().unwrap();
    n
}
#[test]
fn irrational_source_intersections_own_exact_positions_and_natural_parameters() {
    let a = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 6., 6., 6.],
        control_points: vec![vec![-2., 0.], vec![0., 4.], vec![2., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let mut b = Curve::from_polyline(vec![vec![-3., 1.], vec![3., 1.]]).unwrap();
    for k in &mut b.knots {
        *k = 10. + 6. * *k;
    }
    let owner = own(&a, &b, false);
    assert_eq!(owner.nodes().len(), 2);
    for n in owner.nodes() {
        assert!(!n.stationary);
        assert!(n.factor.is_some());
        assert_ne!(n.exact.isolating_interval[0], n.exact.isolating_interval[1]);
        let decode = |q: &p::ExactRatio| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap());
        let mid = (decode(&n.exact.isolating_interval[0]) + decode(&n.exact.isolating_interval[1]))
            / Q::from_integer(2.into());
        let y = eval(&n.exact.position[1][0], &mid) / eval(&n.exact.position[1][1], &mid);
        assert_eq!(y, Q::one());
        let u = eval(&n.exact.parameters[0], &mid);
        let v = eval(&n.exact.parameters[1], &mid);
        assert!(u > Q::from_integer(2.into()) && u < Q::from_integer(6.into()));
        assert!(v > Q::from_integer(10.into()) && v < Q::from_integer(16.into()));
        // These are symbolic source parameters, not f64 event midpoints.
        assert!(n.exact.parameters[0].len() > 1 && n.exact.position[0][0].len() > 1);
    }
    let mut changed = a;
    changed.control_points[1][1] = 4_f64.next_up();
    assert!(p::own_curve_event_nodes(&changed, &b, owner.census()).is_err());
}
#[test]
fn endpoint_nodes_bind_original_endpoint_parameters_without_false_primitive_motion() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![1., 1.], vec![2., 0.]]).unwrap();
    let owner = own(&a, &b, false);
    assert_eq!(owner.nodes().len(), 1);
    let n = &owner.nodes()[0];
    assert!(n.factor.is_none());
    for axis in 0..2 {
        assert_eq!(
            eval(&n.exact.position[axis][0], &Q::zero())
                / eval(&n.exact.position[axis][1], &Q::zero()),
            Q::one()
        );
    }
    assert_eq!(eval(&n.exact.parameters[0], &Q::zero()), Q::one());
    assert_eq!(eval(&n.exact.parameters[1], &Q::zero()), Q::zero());
}
#[test]
fn stationary_original_germs_and_overlap_census_are_kept_distinct() {
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![vec![-1., 0.], vec![1., 0.], vec![-1., 0.], vec![1., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    };
    let owner = own(&c, &c, true);
    assert_eq!(owner.nodes().len(), 1);
    assert!(owner.nodes()[0].stationary);
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![1., 0.], vec![3., 0.]]).unwrap();
    let owner = own(&a, &b, false);
    assert!(owner.nodes().is_empty());
    assert!(owner.census().pairs.iter().any(|p| p.kind == "overlap"));
    assert!(
        !p::verify_span_intersections(&a, &b, owner.census())
            .unwrap()
            .overlaps()
            .is_empty()
    );
}
#[test]
fn exact_original_knot_ownership_does_not_repeat_a_shared_endpoint_event() {
    let a = Curve::from_polyline(vec![vec![-1., -1.], vec![0., 0.], vec![1., 1.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![-1., 1.], vec![0., 0.], vec![1., -1.]]).unwrap();
    let owner = own(&a, &b, false);
    assert_eq!(owner.nodes().len(), 1);
    assert_eq!(owner.nodes()[0].source_cells, [1, 1]);
}

fn curve(points: Vec<Vec<f64>>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        weights: vec![1.; points.len()],
        control_points: points,
        periodic: false,
    }
}
fn midpoint(n: &p::AlgebraicEventNode) -> Q {
    let d = |q: &p::ExactRatio| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap());
    (d(&n.exact.isolating_interval[0]) + d(&n.exact.isolating_interval[1]))
        / Q::from_integer(2.into())
}
#[test]
fn separated_primitive_fibers_preserve_distinct_source_traversals_at_one_point() {
    for rotated in [false, true] {
        let mut a = curve(vec![vec![0., 0.], vec![0., 1.]]);
        let mut b = curve(vec![vec![0.75, 0.5], vec![-1.25, 0.5], vec![0.75, 0.5]]);
        if rotated {
            for c in [&mut a, &mut b] {
                for p in &mut c.control_points {
                    let (x, y) = (p[0], p[1]);
                    p[0] = x - y + 32.;
                    p[1] = 2. * x + y - 16.;
                }
            }
        }
        let owner = own(&a, &b, false);
        assert_eq!(owner.nodes().len(), 2);
        assert!(owner.census().pairs.iter().any(|p| p.shear > 0));
        let expected = if rotated {
            [Q::new(63.into(), 2.into()), Q::new((-31).into(), 2.into())]
        } else {
            [Q::zero(), Q::new(1.into(), 2.into())]
        };
        let mut traversal = Vec::new();
        for n in owner.nodes() {
            let root = midpoint(n);
            for axis in 0..2 {
                assert_eq!(
                    eval(&n.exact.position[axis][0], &root)
                        / eval(&n.exact.position[axis][1], &root),
                    expected[axis]
                );
            }
            assert_eq!(
                eval(&n.exact.parameters[0], &root),
                Q::new(1.into(), 2.into())
            );
            traversal.push(eval(&n.exact.parameters[1], &root));
        }
        traversal.sort();
        assert_eq!(
            traversal,
            vec![Q::new(1.into(), 4.into()), Q::new(3.into(), 4.into())]
        );
    }
}
#[test]
fn self_intersection_and_rational_coordinate_denominators_keep_exact_identity() {
    let a = curve(vec![
        vec![3., -1.5],
        vec![-1., 3.5],
        vec![-1., -3.5],
        vec![3., 1.5],
    ]);
    let owner = own(&a, &a, true);
    assert_eq!(owner.nodes().len(), 1);
    assert!(!owner.nodes()[0].stationary);
    let mut a = curve(vec![vec![-2., 0.], vec![0., 4.], vec![2., 0.]]);
    a.weights = vec![1., 2., 1.];
    let b = curve(vec![vec![-3., 1.], vec![3., 1.]]);
    let owner = own(&a, &b, false);
    assert_eq!(owner.nodes().len(), 2);
    for n in owner.nodes() {
        let root = midpoint(n);
        assert_eq!(
            eval(&n.exact.position[1][0], &root) / eval(&n.exact.position[1][1], &root),
            Q::one()
        );
        assert_ne!(eval(&n.exact.position[1][1], &root), Q::one());
    }
}

#[test]
fn a_nonbinary_rational_parameter_stays_symbolic_instead_of_becoming_a_midpoint() {
    let a = curve(vec![vec![0., 0.], vec![3., 3.]]);
    let b = curve(vec![vec![0., 1.], vec![3., 1.]]);
    let owner = own(&a, &b, false);
    assert_eq!(owner.nodes().len(), 1);
    let n = &owner.nodes()[0];
    let third = Q::new(1.into(), 3.into());
    assert!(eval(&n.exact.primitive, &third).is_zero());
    assert_ne!(n.exact.isolating_interval[0], n.exact.isolating_interval[1]);
    // A rational dyadic midpoint cannot be the actual parameter 1/3.
    let mid = midpoint(n);
    assert_ne!(eval(&n.exact.parameters[0], &mid), third);
    for axis in 0..2 {
        assert_eq!(
            eval(&n.exact.position[axis][0], &mid) / eval(&n.exact.position[axis][1], &mid),
            Q::one()
        );
    }
}

#[test]
fn partial_overlap_endpoints_keep_original_natural_parameters_and_reverse_clock() {
    let a = Curve::from_polyline(vec![vec![0., 0.], vec![2., 0.]]).unwrap();
    let b = Curve::from_polyline(vec![vec![3., 0.], vec![1., 0.]]).unwrap();
    let owner = own(&a, &b, false);
    let ends = owner.overlap_endpoints().unwrap();
    assert_eq!(ends.len(), 1);
    for (i, n) in ends[0].1.iter().enumerate() {
        let decode = |q: &p::ExactRatio| Q::new(q[0].parse().unwrap(), q[1].parse().unwrap());
        let mid = (decode(&n.exact.isolating_interval[0]) + decode(&n.exact.isolating_interval[1])) / Q::from_integer(2.into());
        let x = eval(&n.exact.position[0][0], &mid) / eval(&n.exact.position[0][1], &mid);
        assert_eq!(x, Q::from_integer((i + 1).into()));
        assert_eq!(eval(&n.exact.parameters[0], &mid), Q::new((i + 1).into(), 2.into()));
        assert_eq!(eval(&n.exact.parameters[1], &mid), Q::new((2 - i).into(), 2.into()));
    }
    assert!(owner.nodes().is_empty());
}

#[test]
fn partial_overlap_boundary_keeps_irrational_original_cut() {
    let a = Curve { degree: 2, knots: vec![2.,2.,2.,6.,6.,6.],
        control_points: vec![vec![0.,0.],vec![0.,0.],vec![1.,0.]],
        weights: vec![1.;3], periodic:false };
    let b = Curve::from_polyline(vec![vec![0.5,0.],vec![1.5,0.]]).unwrap();
    let owner = own(&a,&b,false);
    let endpoints = owner.overlap_endpoints().unwrap();
    assert_eq!(endpoints.len(),1);
    let lower = &endpoints[0].1[0];
    assert_ne!(lower.exact.isolating_interval[0],lower.exact.isolating_interval[1]);
    assert!(lower.exact.parameters[0].len()>1);
    assert_eq!(owner.sources()[0].control_points,a.control_points);
    assert_eq!(lower.source_cells,[0,0]);
    assert_eq!(lower.pair,endpoints[0].0);
}
