use brep_core::{bezier_profile, prism};
use nurbs_core::curve::Curve;
fn profile(degree: usize) -> Vec<Vec<Curve>> {
    let floor = Curve {
        degree,
        knots: [vec![0.; degree + 1], vec![1.; degree + 1]].concat(),
        control_points: (0..=degree)
            .map(|i| vec![8. * i as f64 / degree as f64, -(i.min(degree - i) as f64)])
            .collect(),
        weights: vec![1.; degree + 1],
        periodic: false,
    };
    let line = |a: Vec<f64>, b: Vec<f64>| Curve::from_polyline(vec![a, b]).unwrap();
    vec![vec![
        floor,
        line(vec![8., 0.], vec![8., 8.]),
        line(vec![8., 8.], vec![0., 8.]),
        line(vec![0., 8.], vec![0., 0.]),
    ]]
}
#[test]
fn two_arc_lens_with_two_endpoint_contacts_is_usable_as_a_solid_profile() {
    let arc=|y:f64|Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.,0.],vec![1.,y],vec![2.,0.]],weights:vec![1.;3],periodic:false};
    let loops=vec![vec![arc(-1.),arc(1.).reverse().unwrap()]];
    assert!((bezier_profile::validate_profile(&loops,1e-7).unwrap()-4./3.).abs()<1e-10);
    prism::extrude(&loops,0.,2.).unwrap().validate().unwrap();
}
#[test]
fn quartic_and_degree_eight_profiles_keep_curves_through_boolean_and_extrusion() {
    for degree in [4, 8] {
        let loops = profile(degree);
        assert!(bezier_profile::validate_profile(&loops, 1e-7).unwrap() > 64.);
        let solid = prism::extrude(&loops, 0., 3.).unwrap();
        solid.validate().unwrap();
        assert!(solid.edges.iter().any(|e| e.curve.degree == degree));
        let union = bezier_profile::boolean(&loops, &loops, "union", 1e-7).unwrap();
        assert!(union.iter().flatten().any(|c| c.degree == degree));
        assert!(
            bezier_profile::boolean(&loops, &loops, "difference", 1e-7)
                .unwrap()
                .is_empty()
        );
        let hole = vec![
            brep_core::sketch::polygon_wire(vec![[2., 2.], [6., 2.], [6., 6.], [2., 6.]]).unwrap(),
        ];
        let difference = bezier_profile::boolean(&loops, &hole, "difference", 1e-7).unwrap();
        assert_eq!(difference.len(), 2);
        prism::extrude(&difference, 0., 3.)
            .unwrap()
            .validate()
            .unwrap();
        let cutter = vec![
            brep_core::sketch::polygon_wire(vec![[4., -4.], [12., -4.], [12., 4.], [4., 4.]])
                .unwrap(),
        ];
        let union = bezier_profile::boolean(&loops, &cutter, "union", 1e-7).unwrap();
        let intersection = bezier_profile::boolean(&loops, &cutter, "intersection", 1e-7).unwrap();
        let expected = bezier_profile::validate_profile(&loops, 1e-7).unwrap() + 64.;
        let actual = bezier_profile::validate_profile(&union, 1e-7).unwrap()
            + bezier_profile::validate_profile(&intersection, 1e-7).unwrap();
        assert!((expected - actual).abs() < 1e-6);
        prism::extrude(&intersection, 0., 3.)
            .unwrap()
            .validate()
            .unwrap();
    }
}

#[test]
fn rational_quartic_preserves_weights_through_a_transverse_cut() {
    let mut loops=profile(4);loops[0][0].weights=vec![1.,2.,4.,2.,1.];
    assert!(bezier_profile::validate_profile(&loops,1e-7).unwrap()>64.);
    let cutter=vec![brep_core::sketch::polygon_wire(vec![[4.,-4.],[12.,-4.],[12.,4.],[4.,4.]]).unwrap()];
    let intersection=bezier_profile::boolean(&loops,&cutter,"intersection",1e-7).unwrap();
    assert!(intersection.iter().flatten().any(|c|c.degree==4 && c.weights.windows(2).any(|w|w[0]!=w[1])));
    prism::extrude(&intersection,0.,3.).unwrap().validate().unwrap();
}
