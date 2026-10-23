//! Public-API construction checks with separate small-integer rational arithmetic.
//! This oracle uses affine Cramer's rule, not production homogeneous expansions.
//! It is candidate regression evidence, not an independent qualification review.
use cad_predicates::{
    AuthoredScalar, ConstructedPoint2, InputError, Limits, LineIntersection2,
    MAX_CONSTRUCTION_DEPTH, MAX_CONSTRUCTION_NODES, Outcome, Point2Input, PredicateContext, Reason,
    Sign, SourceArena, ToleranceContext, intersect_authored_lines2d, intersect_lines2d,
    orient2d_points,
};
use std::cmp::Ordering;
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Q {
    n: i128,
    d: i128,
}
impl Q {
    fn new(n: i128, d: i128) -> Self {
        assert_ne!(d, 0);
        let (mut a, mut b) = (n.abs(), d.abs());
        while b != 0 {
            (a, b) = (b, a % b);
        }
        let sign = d.signum();
        Self {
            n: n / a * sign,
            d: d / a * sign,
        }
    }
    fn int(n: i128) -> Self {
        Self::new(n, 1)
    }
    fn add(self, b: Self) -> Self {
        Self::new(self.n * b.d + b.n * self.d, self.d * b.d)
    }
    fn sub(self, b: Self) -> Self {
        Self::new(self.n * b.d - b.n * self.d, self.d * b.d)
    }
    fn mul(self, b: Self) -> Self {
        Self::new(self.n * b.n, self.d * b.d)
    }
    fn div(self, b: Self) -> Self {
        Self::new(self.n * b.d, self.d * b.n)
    }
    fn scalar(self) -> AuthoredScalar {
        AuthoredScalar::RationalConstant {
            numerator: self.n.try_into().unwrap(),
            denominator: self.d.try_into().unwrap(),
        }
    }
}
type P = [Q; 2];
fn difference(a: P, b: P) -> P {
    [a[0].sub(b[0]), a[1].sub(b[1])]
}
fn determinant(a: P, b: P) -> Q {
    a[0].mul(b[1]).sub(a[1].mul(b[0]))
}
enum Expected {
    Unique(P, [Q; 2]),
    Parallel,
    Coincident,
    Degenerate(usize),
}
fn oracle(points: [P; 4]) -> Expected {
    let [a, b, c, d] = points;
    let u = difference(b, a);
    let v = difference(d, c);
    let w = difference(c, a);
    if u.iter().all(|q| q.n == 0) {
        return Expected::Degenerate(0);
    }
    if v.iter().all(|q| q.n == 0) {
        return Expected::Degenerate(1);
    }
    let det = determinant(u, v);
    if det.n == 0 {
        return if determinant(w, u).n == 0 {
            Expected::Coincident
        } else {
            Expected::Parallel
        };
    }
    let t = determinant(w, v).div(det);
    let s = determinant(w, u).div(det);
    Expected::Unique([a[0].add(t.mul(u[0])), a[1].add(t.mul(u[1]))], [t, s])
}
fn sign(q: Q) -> Sign {
    match q.n.cmp(&0) {
        Ordering::Less => Sign::Negative,
        Ordering::Equal => Sign::Zero,
        Ordering::Greater => Sign::Positive,
    }
}

/// Compare the small exact rational to the exact bits of an f64 endpoint.
/// Exponent/bit-length comparison handles subnormals without float conversion
/// or constructing an overflowing 2^1074 denominator in this test oracle.
fn compare_float(q: Q, f: f64) -> Ordering {
    assert!(f.is_finite());
    if f == 0. {
        return q.n.cmp(&0);
    }
    let fs = if f > 0. { 1 } else { -1 };
    if q.n.signum() != fs {
        return q.n.signum().cmp(&fs);
    }
    let bits = f.abs().to_bits();
    let encoded = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & 0x000f_ffff_ffff_ffff) | if encoded == 0 { 0 } else { 1 << 52 };
    let exponent = if encoded == 0 {
        -1074
    } else {
        encoded - 1023 - 52
    };
    let mut left = q.n.unsigned_abs();
    let mut right = (q.d as u128).checked_mul(mantissa as u128).unwrap();
    let lb = 128 - left.leading_zeros() as i32;
    let rb = 128 - right.leading_zeros() as i32;
    let comparison = if exponent >= 0 && rb + exponent > 128 {
        Ordering::Less
    } else if exponent < 0 && lb - exponent > 128 {
        Ordering::Greater
    } else {
        if exponent >= 0 {
            right <<= exponent as u32;
        } else {
            left <<= (-exponent) as u32;
        }
        left.cmp(&right)
    };
    if fs < 0 {
        comparison.reverse()
    } else {
        comparison
    }
}
fn enclosed(q: Q, bound: [f64; 2]) {
    assert!(bound[0] <= bound[1]);
    assert_ne!(
        compare_float(q, bound[0]),
        Ordering::Less,
        "{q:?} below {bound:?}"
    );
    assert_ne!(
        compare_float(q, bound[1]),
        Ordering::Greater,
        "{q:?} above {bound:?}"
    );
}
fn refs(arena: &SourceArena, offset: usize) -> [cad_predicates::LeafRef; 2] {
    [arena.leaf(offset).unwrap(), arena.leaf(offset + 1).unwrap()]
}
fn unique(result: LineIntersection2) -> ConstructedPoint2 {
    match result {
        LineIntersection2::Unique(point) => *point,
        other => panic!("Expected unique recipe, got {other:?}"),
    }
}

fn recipe_arena(horizontal_count: usize) -> SourceArena {
    let mut coordinates: Vec<f64> = vec![0., 0., 1., 3.];
    for height in 1..=horizontal_count {
        coordinates.extend([0., height as f64, 1., height as f64]);
    }
    SourceArena::authored(
        "recipe-graph",
        1,
        coordinates
            .into_iter()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap()
}
fn horizontal(arena: &SourceArena, height: usize) -> [Point2Input<'_>; 2] {
    let offset = 4 + (height - 1) * 4;
    [
        Point2Input::Authored(refs(arena, offset)),
        Point2Input::Authored(refs(arena, offset + 2)),
    ]
}
fn at_height(
    arena: &SourceArena,
    tolerance: &ToleranceContext,
    a: Point2Input<'_>,
    b: Point2Input<'_>,
    height: usize,
) -> ConstructedPoint2 {
    let mut ctx = PredicateContext::new(arena, tolerance, Limits::default(), None);
    let [c, d] = horizontal(arena, height);
    let report = intersect_lines2d(&mut ctx, a, b, c, d).unwrap();
    match report.outcome {
        LineIntersection2::Unique(point) => *point,
        other => panic!("height {height}: {other:?}, work {}", report.work_used),
    }
}

#[test]
fn construction_chains_keep_exact_recipes_and_refuse_the_depth_boundary_atomically() {
    let arena = recipe_arena(MAX_CONSTRUCTION_DEPTH + 1);
    let tolerance = ToleranceContext::default_valid();
    let origin = Point2Input::Authored(refs(&arena, 0));
    let mut point = at_height(
        &arena,
        &tolerance,
        origin,
        Point2Input::Authored(refs(&arena, 2)),
        1,
    );
    assert!(point.source_points().is_some());
    for height in 2..=MAX_CONSTRUCTION_DEPTH {
        let previous_bound = point.diameter_bound();
        point = at_height(
            &arena,
            &tolerance,
            origin,
            Point2Input::Constructed(&point),
            height,
        );
        assert_eq!(point.recipe_depth(), height);
        assert_eq!(point.recipe_node_count(), height);
        assert!(point.source_points().is_none());
        assert!(point.diameter_bound() >= previous_bound);
        enclosed(Q::new(height as i128, 3), point.enclosure()[0]);
        enclosed(Q::int(height as i128), point.enclosure()[1]);
    }
    // Earlier local handles have been dropped; the last point still owns its DAG.
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert_eq!(
        orient2d_points(
            &mut ctx,
            origin,
            Point2Input::Authored(refs(&arena, 2)),
            Point2Input::Constructed(&point)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );
    let before = point.enclosure();
    let [a, b] = horizontal(&arena, MAX_CONSTRUCTION_DEPTH + 1);
    assert!(matches!(
        intersect_lines2d(&mut ctx, origin, Point2Input::Constructed(&point), a, b)
            .unwrap()
            .outcome,
        LineIntersection2::Indeterminate(Reason::ResourceLimit)
    ));
    assert_eq!(point.enclosure(), before);
}

#[test]
fn diamond_dag_shares_ancestors_without_exponential_evaluation_or_debug_output() {
    let arena = recipe_arena(25);
    let tolerance = ToleranceContext::default_valid();
    let origin = Point2Input::Authored(refs(&arena, 0));
    let mut point = at_height(
        &arena,
        &tolerance,
        origin,
        Point2Input::Authored(refs(&arena, 2)),
        1,
    );
    for round in 0..8 {
        let shared = point.clone();
        let a = at_height(
            &arena,
            &tolerance,
            origin,
            Point2Input::Constructed(&point),
            2 + round * 3,
        );
        let b = at_height(
            &arena,
            &tolerance,
            origin,
            Point2Input::Constructed(&shared),
            3 + round * 3,
        );
        point = at_height(
            &arena,
            &tolerance,
            Point2Input::Constructed(&a),
            Point2Input::Constructed(&b),
            4 + round * 3,
        );
        assert_eq!(point.recipe_node_count(), 1 + (round + 1) * 3);
        assert_eq!(point.recipe_depth(), 1 + (round + 1) * 2);
    }
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let result = orient2d_points(
        &mut ctx,
        origin,
        Point2Input::Authored(refs(&arena, 2)),
        Point2Input::Constructed(&point),
    )
    .unwrap();
    assert_eq!(result.outcome, Outcome::Sign(Sign::Zero));
    assert!(format!("{point:?}").len() < 512);
    enclosed(Q::new(25, 3), point.enclosure()[0]);
    eprintln!(
        "shared diamond: {} nodes, depth {}, exact orientation work {}",
        point.recipe_node_count(),
        point.recipe_depth(),
        result.work_used
    );
}

#[test]
fn combined_graph_node_cap_counts_unique_dependencies_and_preserves_inputs() {
    fn tree(
        arena: &SourceArena,
        tolerance: &ToleranceContext,
        level: usize,
        next: &mut usize,
    ) -> ConstructedPoint2 {
        let children = if level > 0 {
            Some([
                tree(arena, tolerance, level - 1, next),
                tree(arena, tolerance, level - 1, next),
            ])
        } else {
            None
        };
        let height = *next;
        *next += 1;
        match children {
            Some([a, b]) => at_height(
                arena,
                tolerance,
                Point2Input::Constructed(&a),
                Point2Input::Constructed(&b),
                height,
            ),
            None => at_height(
                arena,
                tolerance,
                Point2Input::Authored(refs(arena, 0)),
                Point2Input::Authored(refs(arena, 2)),
                height,
            ),
        }
    }
    assert_eq!(MAX_CONSTRUCTION_NODES, 256);
    let arena = recipe_arena(520);
    let tolerance = ToleranceContext::default_valid();
    let mut next = 1;
    let left = tree(&arena, &tolerance, 7, &mut next);
    assert_eq!(left.recipe_node_count(), 255);
    let at_cap = at_height(
        &arena,
        &tolerance,
        Point2Input::Authored(refs(&arena, 0)),
        Point2Input::Constructed(&left),
        next,
    );
    next += 1;
    assert_eq!(at_cap.recipe_node_count(), MAX_CONSTRUCTION_NODES);
    let right = tree(&arena, &tolerance, 7, &mut next);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let [a, b] = horizontal(&arena, next);
    let before = at_cap.enclosure();
    let report = intersect_lines2d(
        &mut ctx,
        Point2Input::Authored(refs(&arena, 0)),
        Point2Input::Constructed(&at_cap),
        a,
        b,
    )
    .unwrap();
    assert!(matches!(
        report.outcome,
        LineIntersection2::Indeterminate(Reason::ResourceLimit)
    ));
    assert_eq!(at_cap.enclosure(), before);
    let combined = orient2d_points(
        &mut ctx,
        Point2Input::Constructed(&left),
        Point2Input::Constructed(&right),
        Point2Input::Authored(refs(&arena, 0)),
    )
    .unwrap();
    assert_eq!(
        combined.outcome,
        Outcome::Indeterminate(Reason::ResourceLimit)
    );
}

#[test]
fn constructed_endpoints_detect_exact_degeneracy_and_validate_before_cancel() {
    let arena = recipe_arena(3);
    let tolerance = ToleranceContext::default_valid();
    let origin = Point2Input::Authored(refs(&arena, 0));
    let axis = Point2Input::Authored(refs(&arena, 2));
    let first = at_height(&arena, &tolerance, origin, axis, 1);
    let equivalent = at_height(&arena, &tolerance, origin, axis, 1);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let [a, b] = horizontal(&arena, 2);
    assert!(matches!(
        intersect_lines2d(
            &mut ctx,
            Point2Input::Constructed(&first),
            Point2Input::Constructed(&equivalent),
            a,
            b
        )
        .unwrap()
        .outcome,
        LineIntersection2::DegenerateLine(0)
    ));
    let parent = at_height(
        &arena,
        &tolerance,
        origin,
        Point2Input::Constructed(&first),
        2,
    );
    let cancelled = AtomicBool::new(true);
    let foreign_tolerance = ToleranceContext::default_valid();
    let foreign = at_height(&arena, &foreign_tolerance, origin, axis, 3);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancelled));
    assert!(matches!(
        intersect_lines2d(&mut ctx, origin, Point2Input::Constructed(&parent), a, b)
            .unwrap()
            .outcome,
        LineIntersection2::Indeterminate(Reason::Cancelled)
    ));
    assert!(matches!(
        intersect_lines2d(
            &mut ctx,
            origin,
            Point2Input::Constructed(&parent),
            Point2Input::Constructed(&foreign),
            b
        ),
        Err(InputError::ContextMismatch)
    ));
    let mut limited = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 100,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        intersect_lines2d(
            &mut limited,
            origin,
            Point2Input::Constructed(&parent),
            a,
            b
        )
        .unwrap()
        .outcome,
        LineIntersection2::Indeterminate(Reason::ResourceLimit)
    ));
    assert!(limited.work_used() > 0 && limited.work_used() <= 100);
}

#[test]
fn nested_general_intersections_match_the_separate_rational_oracle() {
    let tolerance = ToleranceContext::default_valid();
    let mut state = 0x2948ab15u32;
    let mut random = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        (state % 13) as i128 - 6
    };
    let mut verified = 0;
    for attempt in 0..300 {
        let sources: [[[Q; 2]; 4]; 4] =
            std::array::from_fn(|_| std::array::from_fn(|_| [Q::int(random()), Q::int(random())]));
        let mut expected_parents = Vec::new();
        for source in sources {
            if let Expected::Unique(point, _) = oracle(source) {
                expected_parents.push(point);
            }
        }
        if expected_parents.len() != 4 {
            continue;
        }
        let arena = SourceArena::authored(
            "nested-general",
            attempt,
            sources
                .into_iter()
                .flatten()
                .flatten()
                .map(Q::scalar)
                .collect(),
        )
        .unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let mut parents = Vec::new();
        for index in 0..4 {
            let offset = index * 8;
            parents.push(unique(
                intersect_authored_lines2d(
                    &mut ctx,
                    refs(&arena, offset),
                    refs(&arena, offset + 2),
                    refs(&arena, offset + 4),
                    refs(&arena, offset + 6),
                )
                .unwrap()
                .outcome,
            ));
        }
        let expected = oracle(std::array::from_fn(|i| expected_parents[i]));
        let report = intersect_lines2d(
            &mut ctx,
            Point2Input::Constructed(&parents[0]),
            Point2Input::Constructed(&parents[1]),
            Point2Input::Constructed(&parents[2]),
            Point2Input::Constructed(&parents[3]),
        )
        .unwrap();
        match (expected, report.outcome) {
            (Expected::Unique(point, parameters), LineIntersection2::Unique(actual)) => {
                assert_eq!(actual.recipe_depth(), 2);
                assert_eq!(actual.recipe_node_count(), 5);
                for axis in 0..2 {
                    enclosed(point[axis], actual.enclosure()[axis]);
                    enclosed(parameters[axis], actual.parameter_enclosures()[axis]);
                }
                for (a, b) in [(0, 1), (2, 3)] {
                    assert_eq!(
                        orient2d_points(
                            &mut ctx,
                            Point2Input::Constructed(&parents[a]),
                            Point2Input::Constructed(&parents[b]),
                            Point2Input::Constructed(&actual)
                        )
                        .unwrap()
                        .outcome,
                        Outcome::Sign(Sign::Zero)
                    );
                }
            }
            (Expected::Parallel, LineIntersection2::Parallel)
            | (Expected::Coincident, LineIntersection2::Coincident) => {}
            (Expected::Degenerate(a), LineIntersection2::DegenerateLine(b)) => assert_eq!(a, b),
            (_, other) => panic!("nested case {attempt}: unexpected {other:?}"),
        }
        verified += 1;
        if verified == 100 {
            break;
        }
    }
    assert_eq!(verified, 100);
}

#[test]
fn broad_binary_exponent_spans_preserve_the_unreduced_exact_path() {
    let tiny = 2f64.powi(-200);
    let coordinates = [tiny, 0., tiny, 1., 0., 0.5, 1., 0.5];
    let arena = SourceArena::authored(
        "wide-projective-span",
        1,
        coordinates
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .to_vec(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let point = unique(
        intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6),
        )
        .unwrap()
        .outcome,
    );
    let bounds = point.enclosure();
    assert!(bounds[0][0] <= tiny && tiny <= bounds[0][1]);
    enclosed(Q::new(1, 2), bounds[1]);
    assert_eq!(
        orient2d_points(
            &mut ctx,
            Point2Input::Authored(refs(&arena, 0)),
            Point2Input::Authored(refs(&arena, 2)),
            Point2Input::Constructed(&point)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );
}

#[test]
fn constructed_one_third_is_not_its_rounded_center() {
    let mut scalars = [0., 0., 1., 3., 0., 1., 1., 1., 1. / 3., 0., 1. / 3., 2.]
        .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
        .to_vec();
    scalars.extend([Q::new(1, 3).scalar(), Q::int(1).scalar()]);
    let arena = SourceArena::authored("one-third-recipe", 1, scalars).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let point = unique(
        intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6),
        )
        .unwrap()
        .outcome,
    );
    enclosed(Q::new(1, 3), point.enclosure()[0]);
    enclosed(Q::int(1), point.enclosure()[1]);
    for bound in point.parameter_enclosures() {
        enclosed(Q::new(1, 3), bound);
    }
    assert!(point.diameter_bound() <= tolerance.specification().max_entity_error);
    let actual = orient2d_points(
        &mut ctx,
        Point2Input::Authored(refs(&arena, 8)),
        Point2Input::Authored(refs(&arena, 10)),
        Point2Input::Constructed(&point),
    )
    .unwrap();
    assert_eq!(actual.outcome, Outcome::Sign(Sign::Negative));
    // Exact source line incidence survives both the recipe and rational leaf path.
    assert_eq!(
        orient2d_points(
            &mut ctx,
            Point2Input::Authored(refs(&arena, 0)),
            Point2Input::Authored(refs(&arena, 2)),
            Point2Input::Constructed(&point)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );
    assert_eq!(
        orient2d_points(
            &mut ctx,
            Point2Input::Authored(refs(&arena, 8)),
            Point2Input::Authored(refs(&arena, 10)),
            Point2Input::Authored(refs(&arena, 12))
        )
        .unwrap()
        .outcome,
        actual.outcome
    );
}

#[test]
fn rational_line_matrix_matches_separate_affine_oracle_and_parameter_correspondence() {
    let mut state = 0x715ac52bu32;
    let mut next = || {
        state = state.wrapping_mul(1664525).wrapping_add(1013904223);
        state
    };
    let mut counts = [0usize; 4];
    for case in 0..200 {
        let mut points = [[Q::int(0); 2]; 4];
        for p in &mut points {
            for q in p {
                let n = (next() % 19) as i128 - 9;
                let d = (next() % 5 + 1) as i128;
                *q = Q::new(n, d);
            }
        }
        // Include a fixed non-unique inventory, not only generic crossings.
        match case % 25 {
            0 => points[1] = points[0],
            1 => points[3] = points[2],
            2 => points = [points[0], points[1], points[1], points[0]],
            3 => {
                points = [
                    [Q::int(0), Q::int(0)],
                    [Q::int(1), Q::int(1)],
                    [Q::int(0), Q::int(1)],
                    [Q::int(1), Q::int(2)],
                ]
            }
            _ => {}
        }
        let arena = SourceArena::authored(
            "rational-line-matrix",
            case,
            points.into_iter().flatten().map(Q::scalar).collect(),
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let report = intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6),
        )
        .unwrap();
        assert_eq!(report.context, ctx.identity());
        assert!(report.work_used <= cad_predicates::MAX_WORK);
        match (oracle(points), report.outcome) {
            (Expected::Unique(p, parameters), LineIntersection2::Unique(point)) => {
                counts[0] += 1;
                for axis in 0..2 {
                    enclosed(p[axis], point.enclosure()[axis]);
                    enclosed(parameters[axis], point.parameter_enclosures()[axis]);
                }
                for line in [0, 4] {
                    let decision = orient2d_points(
                        &mut ctx,
                        Point2Input::Authored(refs(&arena, line)),
                        Point2Input::Authored(refs(&arena, line + 2)),
                        Point2Input::Constructed(&point),
                    )
                    .unwrap();
                    assert_eq!(decision.outcome, Outcome::Sign(Sign::Zero), "case {case}");
                }
                let decision = orient2d_points(
                    &mut ctx,
                    Point2Input::Authored(refs(&arena, 0)),
                    Point2Input::Authored(refs(&arena, 4)),
                    Point2Input::Constructed(&point),
                )
                .unwrap();
                assert_eq!(
                    decision.outcome,
                    Outcome::Sign(sign(determinant(
                        difference(points[2], points[0]),
                        difference(p, points[0])
                    ))),
                    "case {case}"
                );
            }
            (Expected::Parallel, LineIntersection2::Parallel) => counts[1] += 1,
            (Expected::Coincident, LineIntersection2::Coincident) => counts[2] += 1,
            (Expected::Degenerate(expected), LineIntersection2::DegenerateLine(actual)) => {
                assert_eq!(expected, actual);
                counts[3] += 1;
            }
            (_, other) => panic!("case {case}: classification mismatch: {other:?}"),
        }
    }
    assert!(counts[0] > 150 && counts[1] >= 8 && counts[2] >= 8 && counts[3] >= 16);
    eprintln!(
        "construction oracle: unique={}, parallel={}, coincident={}, degenerate={}",
        counts[0], counts[1], counts[2], counts[3]
    );
}

#[test]
fn all_constructed_orientation_permutations_preserve_exact_incidence() {
    // Three independent line recipes produce (1/3,1), (1,1/3), (1/2,1/2).
    let points: [[f64; 2]; 12] = [
        [0., 0.],
        [1., 3.],
        [0., 1.],
        [1., 1.],
        [0., 0.],
        [3., 1.],
        [1., 0.],
        [1., 1.],
        [0., 0.],
        [1., 1.],
        [0., 1.],
        [1., 0.],
    ];
    let arena = SourceArena::authored(
        "three-recipes",
        1,
        points
            .into_iter()
            .flatten()
            .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let mut recipes = Vec::new();
    for offset in [0, 8, 16] {
        recipes.push(unique(
            intersect_authored_lines2d(
                &mut ctx,
                refs(&arena, offset),
                refs(&arena, offset + 2),
                refs(&arena, offset + 4),
                refs(&arena, offset + 6),
            )
            .unwrap()
            .outcome,
        ));
    }
    let expected = [
        [Q::new(1, 3), Q::int(1)],
        [Q::int(1), Q::new(1, 3)],
        [Q::new(1, 2), Q::new(1, 2)],
    ];
    for [a, b, c] in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
        [0, 0, 1],
    ] {
        let decision = orient2d_points(
            &mut ctx,
            Point2Input::Constructed(&recipes[a]),
            Point2Input::Constructed(&recipes[b]),
            Point2Input::Constructed(&recipes[c]),
        )
        .unwrap();
        assert_eq!(
            decision.outcome,
            Outcome::Sign(sign(determinant(
                difference(expected[b], expected[a]),
                difference(expected[c], expected[a])
            )))
        );
    }
}

#[test]
fn endpoint_order_and_affine_placement_preserve_exact_intersection() {
    let original = [
        [Q::int(0), Q::int(0)],
        [Q::int(1), Q::int(3)],
        [Q::int(0), Q::int(1)],
        [Q::int(1), Q::int(1)],
    ];
    for [xx, xy, tx, yx, yy, ty] in [
        [1, 0, 0, 0, 1, 0],
        [-2, 0, 0, 0, 3, 0],
        [1, 2, 0, -1, 1, 0],
        [0, -1, 1000, 1, 0, -3000],
    ] {
        let points = original.map(|[x, y]| {
            [
                x.mul(Q::int(xx)).add(y.mul(Q::int(xy))).add(Q::int(tx)),
                x.mul(Q::int(yx)).add(y.mul(Q::int(yy))).add(Q::int(ty)),
            ]
        });
        let arena = SourceArena::authored(
            "affine-recipe",
            1,
            points.into_iter().flatten().map(Q::scalar).collect(),
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        for order in [
            [0, 1, 2, 3],
            [1, 0, 2, 3],
            [0, 1, 3, 2],
            [1, 0, 3, 2],
            [2, 3, 0, 1],
            [3, 2, 0, 1],
            [2, 3, 1, 0],
            [3, 2, 1, 0],
        ] {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let [a, b, c, d] = order.map(|i| refs(&arena, 2 * i));
            let point = unique(
                intersect_authored_lines2d(&mut ctx, a, b, c, d)
                    .unwrap()
                    .outcome,
            );
            let Expected::Unique(expected, parameters) = oracle(order.map(|i| points[i])) else {
                panic!("invertible placement lost intersection")
            };
            for axis in 0..2 {
                enclosed(expected[axis], point.enclosure()[axis]);
                enclosed(parameters[axis], point.parameter_enclosures()[axis]);
            }
        }
    }
}

#[test]
fn nearly_parallel_lines_are_not_coerced_to_parallel_or_coincident() {
    for offset in [0., 1.] {
        let data = [
            0.,
            0.,
            1.,
            1.,
            0.,
            offset,
            1.,
            offset + 1. + f64::EPSILON * 2.,
        ];
        let arena = SourceArena::authored(
            "near-parallel",
            1,
            data.map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .to_vec(),
        )
        .unwrap();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let outcome = intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6),
        )
        .unwrap()
        .outcome;
        if offset == 0. {
            let point = unique(outcome);
            for bound in point.enclosure() {
                enclosed(Q::int(0), bound);
            }
        } else {
            // Huge remote intersection cannot meet the absolute entity error cap.
            assert!(matches!(
                outcome,
                LineIntersection2::Indeterminate(Reason::PrecisionExhausted)
            ));
        }
    }
}

#[test]
fn construction_and_followup_enforce_context_budget_cancellation_and_error_bounds() {
    let source = [0., 0., 1., 3., 0., 1., 1., 1.]
        .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
        .to_vec();
    let arena = SourceArena::authored("guards", 1, source.clone()).unwrap();
    let other = SourceArena::authored("guards", 1, source).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let cancel = AtomicBool::new(false);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    let point = unique(
        intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6),
        )
        .unwrap()
        .outcome,
    );
    for (limits, expected) in [
        (
            Limits {
                max_work: 0,
                ..Limits::default()
            },
            Reason::ResourceLimit,
        ),
        (
            Limits {
                max_work: u64::MAX,
                ..Limits::default()
            },
            Reason::ResourceLimit,
        ),
        (
            Limits {
                deadline: Some(Instant::now() - Duration::from_secs(1)),
                ..Limits::default()
            },
            Reason::DeadlineExceeded,
        ),
    ] {
        let mut limited = PredicateContext::new(&arena, &tolerance, limits, None);
        assert!(
            matches!(intersect_authored_lines2d(&mut limited,refs(&arena,0),refs(&arena,2),refs(&arena,4),refs(&arena,6)).unwrap().outcome,LineIntersection2::Indeterminate(reason) if reason==expected)
        );
        assert_eq!(
            orient2d_points(
                &mut limited,
                Point2Input::Constructed(&point),
                Point2Input::Authored(refs(&arena, 0)),
                Point2Input::Authored(refs(&arena, 2))
            )
            .unwrap()
            .outcome,
            Outcome::Indeterminate(expected)
        );
    }
    cancel.store(true, AtomicOrdering::Relaxed);
    assert_eq!(
        orient2d_points(
            &mut ctx,
            Point2Input::Constructed(&point),
            Point2Input::Authored(refs(&arena, 0)),
            Point2Input::Authored(refs(&arena, 2))
        )
        .unwrap()
        .outcome,
        Outcome::Indeterminate(Reason::Cancelled)
    );
    assert!(matches!(
        intersect_authored_lines2d(
            &mut ctx,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&other, 6)
        ),
        Err(InputError::ProvenanceMismatch)
    ));
    let foreign_tolerance = ToleranceContext::default_valid();
    let mut foreign =
        PredicateContext::new(&arena, &foreign_tolerance, Limits::default(), Some(&cancel));
    assert_eq!(
        orient2d_points(
            &mut foreign,
            Point2Input::Constructed(&point),
            Point2Input::Authored(refs(&arena, 0)),
            Point2Input::Authored(refs(&arena, 2))
        ),
        Err(InputError::ContextMismatch)
    );
    let mut tight = tolerance.specification().clone();
    tight.max_entity_error = 1e-30;
    tight.linear_abs = 1e-30;
    let tight = ToleranceContext::new(tight).unwrap();
    let mut limited = PredicateContext::new(&arena, &tight, Limits::default(), None);
    assert!(matches!(
        intersect_authored_lines2d(
            &mut limited,
            refs(&arena, 0),
            refs(&arena, 2),
            refs(&arena, 4),
            refs(&arena, 6)
        )
        .unwrap()
        .outcome,
        LineIntersection2::Indeterminate(Reason::PrecisionExhausted)
    ));
}
