use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::chain::{
        self, Limits, Span,
        region::{
            self,
            compound::{self, cap},
        },
    },
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn outer() -> region::Region {
    let a = plane();
    let mut b = plane();
    for p in b.control_points.iter_mut().flatten() {
        p.swap(1, 2);
        p[0] = 3. * p[0] - 1.;
    }
    let span = Span {
        owner: 0,
        axis: 0,
        drive: [0.1, 0.9],
        free: [0.7, 0.9],
        other: [[0.2, 0.8], [0.7, 0.9]],
        reversed: false,
    };
    chain::build(
        [&a, &b],
        [0.8, -0.8],
        &[span],
        1e-7,
        1e-7,
        Limits::default(),
    )
    .unwrap()
    .close_to_boundary(0, 0, region::Limits::default())
    .unwrap()
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let p = [[lo, lo], [hi, lo], [hi, hi], [lo, hi]];
    (0..4)
        .map(|i| Curve::from_polyline(vec![p[i].to_vec(), p[(i + 1) % 4].to_vec()]).unwrap())
        .collect()
}

fn compound() -> compound::Compound {
    outer()
        .with_nested(
            &[square(0.2, 0.7), square(0.3, 0.6)],
            compound::Limits::default(),
        )
        .unwrap()
}
#[test]
fn automatic_carriers_preserve_every_original_rim_and_replay() {
    for prefer_basis in [true, false] {
        let c = compound();
        let uv = c.uv_domain().loops().to_vec();
        let world = c.world_loops().to_vec();
        let limits = cap::Limits {
            prefer_basis,
            ..Default::default()
        };
        let c = c.cap(0.005, 1e-6, limits).unwrap();
        assert_eq!(
            c.origin(),
            if prefer_basis {
                cap::Origin::BasisProposal
            } else {
                cap::Origin::PolynomialProposal
            }
        );
        assert!(c.chart_proof().certified);
        assert_eq!(c.region().uv_domain().loops(), uv);
        assert_eq!(c.region().world_loops(), world);
        assert_eq!(c.boundary_proofs().iter().map(Vec::len).sum::<usize>(), 12);
        c.reverify().unwrap();
    }
}
#[test]
fn normal_error_cannot_waive_actual_rim_agreement() {
    let mut s = plane();
    for p in s.control_points.iter_mut().flatten() {
        p[2] = 0.804;
    }
    let e = cap::with_candidate(compound(), &s, 0.005, 1e-6, Default::default())
        .err()
        .unwrap();
    assert!(e.to_string().contains("rim"), "{e}");
}
#[test]
fn limits_never_publish_partial_caps() {
    for kind in 0..5 {
        let mut l = cap::Limits::default();
        match kind {
            0 => l.normal_cells = 0,
            1 => l.chart_cells = 0,
            2 => l.boundary_cells = 1,
            3 => l.max_poles = 1,
            _ => {
                l.prefer_basis = false;
                l.polynomial.evaluations = 1;
            }
        }
        assert!(compound().cap(0.005, 1e-6, l).is_err());
    }
}
#[test]
fn pointwise_normal_allowance_does_not_admit_folded_carriers() {
    let mut s = plane();
    s.degree_u = 3;
    s.knots_u = vec![0., 0., 0., 0., 1., 1., 1., 1.];
    s.control_points = [0., 3., -2., 1.]
        .map(|x| vec![vec![x, 0., 0.8], vec![x, 1., 0.8]])
        .to_vec();
    s.weights = vec![vec![1.; 2]; 4];
    let e = cap::with_candidate(compound(), &s, 4., 10., Default::default())
        .err()
        .unwrap();
    assert!(e.to_string().contains("injectivity"), "{e}");
}
