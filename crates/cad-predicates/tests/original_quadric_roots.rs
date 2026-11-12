use cad_predicates::{
    AuthoredScalar, Limits, OriginalQuadricRootsDecision, OriginalQuadricRootsOutcome as Outcome,
    PredicateContext, SourceArena, ToleranceContext, rational_bspline_quadric_roots,
};
/// Rational B-spline space curve under test.
struct Curve<'a> {
    points: &'a [[f64; 3]],
    weights: &'a [f64],
    knots: &'a [f64],
    degree: usize,
    span: usize,
}
fn query(
    curve: Curve<'_>,
    q: [[f64; 4]; 4],
    products: &[[f64; 2]],
    budget: u64,
) -> OriginalQuadricRootsDecision {
    let Curve {
        points,
        weights,
        knots,
        degree: p,
        span,
    } = curve;
    let mut v = Vec::new();
    for (point, w) in points.iter().zip(weights) {
        v.extend(point);
        v.push(*w);
    }
    let k = v.len();
    v.extend(knots);
    let m = v.len();
    v.extend(q.into_iter().flatten());
    let f = v.len();
    v.extend(products.iter().flatten());
    let arena = SourceArena::authored(
        "original-quadric-roots",
        1,
        v.into_iter()
            .map(|x: f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let controls: Vec<_> = (0..points.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(i * 4 + j).unwrap()))
        .collect();
    let ks: Vec<_> = (0..knots.len())
        .map(|i| arena.leaf(k + i).unwrap())
        .collect();
    let matrix =
        std::array::from_fn(|i| std::array::from_fn(|j| arena.leaf(m + i * 4 + j).unwrap()));
    let factors: Vec<_> = (0..products.len())
        .map(|i| std::array::from_fn(|j| arena.leaf(f + i * 2 + j).unwrap()))
        .collect();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: budget,
            ..Limits::default()
        },
        None,
    );
    let r = rational_bspline_quadric_roots(
        &mut ctx,
        cad_predicates::SplineLeafRefs {
            controls: &controls,
            knots: &ks,
            degree: p,
        },
        span,
        matrix,
        &factors,
        1e-10,
    )
    .unwrap();
    assert!(r.work_used <= budget);
    r
}
fn sphere(radius_squared: f64) -> [[f64; 4]; 4] {
    [
        [1., 0., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 1., 0.],
        [0., 0., 0., -radius_squared],
    ]
}
fn roots(r: OriginalQuadricRootsDecision, expected: &[(f64, usize)]) {
    let Outcome::Roots(roots) = r.outcome else {
        panic!("{r:?}")
    };
    assert_eq!(roots.len(), expected.len());
    for (r, (t, m)) in roots.iter().zip(expected) {
        assert!(r.interval[0] <= *t && *t <= r.interval[1], "{r:?} vs {t}");
        assert_eq!(r.multiplicity, *m);
    }
}
#[test]
fn sphere_transverse_tangent_exterior_and_endpoint_contacts() {
    roots(
        query(
            Curve {
                points: &[[-2., 0., 0.], [2., 0., 0.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            100000,
        ),
        &[(0.25, 1), (0.75, 1)],
    );
    roots(
        query(
            Curve {
                points: &[[-1., 1., 0.], [1., 1., 0.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            100000,
        ),
        &[(0.5, 2)],
    );
    roots(
        query(
            Curve {
                points: &[[-1., 2., 0.], [1., 2., 0.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            100000,
        ),
        &[],
    );
    roots(
        query(
            Curve {
                points: &[[-1., 0., 0.], [1., 0., 0.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            100000,
        ),
        &[(0., 1), (1., 1)],
    );
}
#[test]
fn original_rational_weights_have_nonbinary_quadric_roots() {
    roots(
        query(
            Curve {
                points: &[[-2., 0., 0.], [2., 0., 0.]],
                weights: &[1., 3.],
                knots: &[-2., -2., 5., 5.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            100000,
        ),
        &[(0.1, 1), (0.5, 1)],
    );
}
#[test]
fn unclamped_original_spline_span_has_complete_quadric_roots() {
    roots(
        query(
            Curve {
                points: &[[-3., 0., 0.], [-1., 0., 0.], [1., 0., 0.], [3., 0., 0.]],
                weights: &[1.; 4],
                knots: &[-3., -2., -1., 0., 1., 2., 3., 4.],
                degree: 3,
                span: 3,
            },
            sphere(0.25),
            &[],
            1_000_000,
        ),
        &[(0.25, 1), (0.75, 1)],
    );
}
#[test]
fn exact_rational_circle_is_coincident_without_fitting_or_rounded_extraction() {
    let r = query(
        Curve {
            points: &[[75., 0., 0.], [75., 100., 0.], [-21., 72., 0.]],
            weights: &[5., 3., 5.],
            knots: &[0., 0., 0., 1., 1., 1.],
            degree: 2,
            span: 2,
        },
        sphere(5625.),
        &[],
        100000,
    );
    assert!(matches!(r.outcome, Outcome::Coincident), "{r:?}");
}
#[test]
fn original_radius_products_are_distinct_from_rounded_squared_radius() {
    let r = 1.1;
    let p = [[r, 0., 0.], [r, 0., 0.]];
    let exact = query(
        Curve {
            points: &p,
            weights: &[1., 1.],
            knots: &[0., 0., 1., 1.],
            degree: 1,
            span: 1,
        },
        sphere(0.),
        &[[r, -r]],
        100000,
    );
    assert!(matches!(exact.outcome, Outcome::Coincident), "{exact:?}");
    roots(
        query(
            Curve {
                points: &p,
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(r * r),
            &[],
            100000,
        ),
        &[],
    );
}
#[test]
fn degree_25_source_keeps_degree_50_endpoint_root_order() {
    let points: Vec<_> = (0..26)
        .map(|i| [if i == 25 { 1. } else { 0. }, 0., 0.])
        .collect();
    let knots = [vec![0.; 26], vec![1.; 26]].concat();
    let mut q = [[0.; 4]; 4];
    q[0][0] = 1.;
    roots(
        query(
            Curve {
                points: &points,
                weights: &[1.; 26],
                knots: &knots,
                degree: 25,
                span: 25,
            },
            q,
            &[],
            1_000_000,
        ),
        &[(0., 50)],
    );
}
#[test]
fn asymmetric_quadric_matrix_cross_terms_and_cylinder_are_not_ignored() {
    // (X+Y)^2 - 1, with nonsymmetric authored matrix entries.
    let q = [
        [1., 2., 0., 0.],
        [0., 1., 0., 0.],
        [0., 0., 0., 0.],
        [0., 0., 0., -1.],
    ];
    roots(
        query(
            Curve {
                points: &[[-2., 0.5, 100.], [2., 0.5, -100.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            q,
            &[],
            100000,
        ),
        &[(0.125, 1), (0.625, 1)],
    );
    let mut cylinder = sphere(1.);
    cylinder[2][2] = 0.;
    roots(
        query(
            Curve {
                points: &[[-2., 0., 100.], [2., 0., -100.]],
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            cylinder,
            &[],
            100000,
        ),
        &[(0.25, 1), (0.75, 1)],
    );
}
#[test]
fn exhausted_work_never_admits_roots_or_coincidence() {
    for points in [[[-2., 0., 0.], [2., 0., 0.]], [[1., 0., 0.], [1., 0., 0.]]] {
        let r = query(
            Curve {
                points: &points,
                weights: &[1., 1.],
                knots: &[0., 0., 1., 1.],
                degree: 1,
                span: 1,
            },
            sphere(1.),
            &[],
            0,
        );
        assert!(
            matches!(
                r.outcome,
                Outcome::Indeterminate(cad_predicates::Reason::ResourceLimit)
            ),
            "{r:?}"
        );
    }
}
