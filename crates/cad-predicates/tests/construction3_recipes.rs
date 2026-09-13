use cad_predicates::*;
use std::cmp::Ordering;
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

type P = [Q; 3];
fn sub(a: P, b: P) -> P {
    std::array::from_fn(|i| a[i].sub(b[i]))
}
fn cross(a: P, b: P) -> P {
    [
        a[1].mul(b[2]).sub(a[2].mul(b[1])),
        a[2].mul(b[0]).sub(a[0].mul(b[2])),
        a[0].mul(b[1]).sub(a[1].mul(b[0])),
    ]
}
fn dot(a: P, b: P) -> Q {
    (0..3).fold(Q::int(0), |s, i| s.add(a[i].mul(b[i])))
}
fn input(arena: &SourceArena, n: usize) -> Point3Input<'_> {
    Point3Input::Authored(std::array::from_fn(|i| arena.leaf(n * 3 + i).unwrap()))
}
fn unique(out: LinePlaneIntersection3) -> ConstructedPoint3 {
    match out {
        LinePlaneIntersection3::Unique(p) => *p,
        other => panic!("{other:?}"),
    }
}
#[test]
fn affine_rational_oracle_checks_coordinates_parameters_and_incidence() {
    let tolerance = ToleranceContext::default_valid();
    let mut checked = 0;
    for seed in 1..=240i128 {
        let points: [P; 5] = std::array::from_fn(|j| {
            std::array::from_fn(|i| {
                Q::new(
                    (seed * (j as i128 + 3) * (i as i128 + 7)
                        + j as i128 * j as i128 * 11
                        + i as i128 * 13)
                        % 37
                        - 18,
                    (seed + j as i128 + i as i128) % 3 + 1,
                )
            })
        });
        let [a, b, c, d, e] = points;
        let v = sub(b, a);
        let u = sub(d, c);
        let w = sub(e, c);
        let normal = cross(u, w);
        let den = dot(normal, v);
        if den.n == 0 {
            continue;
        }
        let t = dot(normal, sub(c, a)).div(den);
        let p: P = std::array::from_fn(|i| a[i].add(t.mul(v[i])));
        let arena = SourceArena::authored(
            "3d-oracle",
            1,
            points.into_iter().flatten().map(Q::scalar).collect(),
        )
        .unwrap();
        for order in [[2, 3, 4], [3, 4, 2], [4, 3, 2]] {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let result = unique(
                intersect_line_plane3d(
                    &mut ctx,
                    input(&arena, 0),
                    input(&arena, 1),
                    input(&arena, order[0]),
                    input(&arena, order[1]),
                    input(&arena, order[2]),
                )
                .unwrap()
                .outcome,
            );
            for (q, bound) in p.into_iter().zip(result.enclosure()) {
                enclosed(q, bound);
            }
            enclosed(t, result.line_parameter_enclosure());
            let c = points[order[0]];
            let u = sub(points[order[1]], c);
            let v = sub(points[order[2]], c);
            let q = sub(p, c);
            for (i, j) in [(0, 1), (0, 2), (1, 2)] {
                let det = u[i].mul(v[j]).sub(u[j].mul(v[i]));
                if det.n != 0 {
                    let uv = result.plane_parameter_enclosures();
                    let u_exact = q[i].mul(v[j]).sub(q[j].mul(v[i])).div(det);
                    let v_exact = u[i].mul(q[j]).sub(u[j].mul(q[i])).div(det);
                    enclosed(u_exact, uv[0]);
                    enclosed(v_exact, uv[1]);
                    let barycentric = [Q::int(1).sub(u_exact).sub(v_exact), u_exact, v_exact];
                    let triangle = if barycentric.iter().any(|q| q.n < 0) {
                        TriangleLocation3::Outside
                    } else {
                        match barycentric.iter().filter(|q| q.n == 0).count() {
                            0 => TriangleLocation3::Interior,
                            1 => TriangleLocation3::Edge(
                                barycentric.iter().position(|q| q.n == 0).unwrap(),
                            ),
                            2 => TriangleLocation3::Vertex(
                                barycentric.iter().position(|q| q.n > 0).unwrap(),
                            ),
                            _ => unreachable!(),
                        }
                    };
                    let segment = if t.n < 0 {
                        SegmentLocation3::BeforeStart
                    } else if t.n == 0 {
                        SegmentLocation3::Start
                    } else if t.n > t.d {
                        SegmentLocation3::AfterEnd
                    } else if t.n == t.d {
                        SegmentLocation3::End
                    } else {
                        SegmentLocation3::Interior
                    };
                    assert_eq!(
                        classify_line_plane_domains3d(&mut ctx, &result)
                            .unwrap()
                            .outcome,
                        Ok(FiniteLocation3 { segment, triangle })
                    );
                    break;
                }
            }
            assert_eq!(
                orient3d_points(
                    &mut ctx,
                    input(&arena, 2),
                    input(&arena, 3),
                    input(&arena, 4),
                    Point3Input::Constructed(&result)
                )
                .unwrap()
                .outcome,
                Outcome::Sign(Sign::Zero)
            );
        }
        checked += 1;
    }
    assert!(checked > 200, "{checked}");
}
#[test]
fn retained_third_and_nested_plane_inputs_preserve_exact_zero() {
    let data = [
        0.,
        0.,
        0.,
        1.,
        0.,
        3.,
        0.,
        0.,
        1.,
        1.,
        0.,
        1.,
        0.,
        1.,
        1.,
        1. / 3.,
        0.,
        0.,
        1. / 3.,
        1.,
        0.,
        1. / 3.,
        0.,
        1.,
    ];
    let arena = SourceArena::authored(
        "third",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let p = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    enclosed(Q::new(1, 3), p.enclosure()[0]);
    assert_eq!(
        orient3d_points(
            &mut ctx,
            input(&arena, 5),
            input(&arena, 6),
            input(&arena, 7),
            Point3Input::Constructed(&p)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Positive)
    );
    let mut last = p;
    for depth in 2..=MAX_CONSTRUCTION_DEPTH {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        last = unique(
            intersect_line_plane3d(
                &mut ctx,
                input(&arena, 0),
                Point3Input::Constructed(&last),
                Point3Input::Constructed(&last),
                input(&arena, 3),
                input(&arena, 4),
            )
            .unwrap()
            .outcome,
        );
        assert_eq!(last.recipe_depth(), depth);
        assert_eq!(last.recipe_node_count(), depth);
        assert_eq!(
            classify_line_plane_domains3d(&mut ctx, &last)
                .unwrap()
                .outcome,
            Ok(FiniteLocation3 {
                segment: SegmentLocation3::End,
                triangle: TriangleLocation3::Vertex(0)
            })
        );
    }
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert!(matches!(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            Point3Input::Constructed(&last),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4)
        )
        .unwrap()
        .outcome,
        LinePlaneIntersection3::Indeterminate(Reason::ResourceLimit)
    ));
    assert!(format!("{last:?}").len() < 600);
    let mut overlap_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let overlap = match intersect_coplanar_segment_triangle3d(
        &mut overlap_ctx,
        Point3Input::Constructed(&last),
        input(&arena, 3),
        Point3Input::Constructed(&last),
        input(&arena, 3),
        input(&arena, 4),
    )
    .unwrap()
    .outcome
    {
        CoplanarIntersection3::Overlap(p) => p,
        other => panic!("{other:?}"),
    };
    assert!(!overlap.is_singleton());
    enclosed(Q::int(0), overlap.parameter_enclosures()[0]);
    enclosed(Q::int(1), overlap.parameter_enclosures()[1]);
    let [a, b, c, d, e] = overlap.inputs();
    let mut repeat_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert!(matches!(
        intersect_coplanar_segment_triangle3d(&mut repeat_ctx, a, b, c, d, e)
            .unwrap()
            .outcome,
        CoplanarIntersection3::Overlap(_)
    ));
    let other = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &other, Limits::default(), None);
    assert_eq!(
        orient3d_points(
            &mut ctx,
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
            Point3Input::Constructed(&last)
        ),
        Err(InputError::ContextMismatch)
    );
}
#[test]
fn exact_degenerate_parallel_and_contained_classification() {
    let data = [0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 1., 0., 1.];
    let arena = SourceArena::authored(
        "cases",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    for (line, plane, expected) in [
        ([0, 0], [0, 1, 2], 0),
        ([0, 3], [0, 0, 2], 1),
        ([3, 4], [0, 1, 2], 2),
        ([0, 1], [0, 1, 2], 3),
    ] {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let result = intersect_line_plane3d(
            &mut ctx,
            input(&arena, line[0]),
            input(&arena, line[1]),
            input(&arena, plane[0]),
            input(&arena, plane[1]),
            input(&arena, plane[2]),
        )
        .unwrap()
        .outcome;
        assert!(matches!(
            (expected, result),
            (0, LinePlaneIntersection3::DegenerateLine)
                | (1, LinePlaneIntersection3::DegeneratePlane)
                | (2, LinePlaneIntersection3::Parallel)
                | (3, LinePlaneIntersection3::Contained)
        ));
    }
}

#[test]
fn closed_domains_classify_every_triangle_feature_and_segment_boundary() {
    let tolerance = ToleranceContext::default_valid();
    for x in -1..=5 {
        for y in -1..=5 {
            for (z0, z1, segment) in [
                (-1., 1., SegmentLocation3::Interior),
                (0., 1., SegmentLocation3::Start),
                (-1., 0., SegmentLocation3::End),
                (1., 2., SegmentLocation3::BeforeStart),
                (-2., -1., SegmentLocation3::AfterEnd),
            ] {
                let data = [
                    x as f64, y as f64, z0, x as f64, y as f64, z1, 0., 0., 0., 4., 0., 0., 0., 4.,
                    0.,
                ];
                let arena = SourceArena::authored(
                    "finite-grid",
                    1,
                    data.into_iter()
                        .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                        .collect(),
                )
                .unwrap();
                for permutation in [[2, 3, 4], [3, 4, 2], [4, 3, 2]] {
                    let mut ctx =
                        PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                    let p = unique(
                        intersect_line_plane3d(
                            &mut ctx,
                            input(&arena, 0),
                            input(&arena, 1),
                            input(&arena, permutation[0]),
                            input(&arena, permutation[1]),
                            input(&arena, permutation[2]),
                        )
                        .unwrap()
                        .outcome,
                    );
                    let weights = [4 - x - y, x, y];
                    let weights = permutation.map(|i| weights[i - 2]);
                    let triangle = if weights.iter().any(|w| *w < 0) {
                        TriangleLocation3::Outside
                    } else {
                        match weights.iter().filter(|w| **w == 0).count() {
                            0 => TriangleLocation3::Interior,
                            1 => TriangleLocation3::Edge(
                                weights.iter().position(|w| *w == 0).unwrap(),
                            ),
                            2 => TriangleLocation3::Vertex(
                                weights.iter().position(|w| *w > 0).unwrap(),
                            ),
                            _ => unreachable!(),
                        }
                    };
                    let result = classify_line_plane_domains3d(&mut ctx, &p)
                        .unwrap()
                        .outcome
                        .unwrap();
                    assert_eq!(result, FiniteLocation3 { segment, triangle });
                    assert_eq!(
                        result.is_closed_domain_hit(),
                        x >= 0 && y >= 0 && x + y <= 4 && z0 <= 0. && z1 >= 0.
                    );
                }
            }
        }
    }
}

#[test]
fn finite_domain_uses_exact_boundary_signs_not_enclosure_centers_or_tolerance() {
    let tolerance = ToleranceContext::default_valid();
    for (x, expected) in [
        (
            f64::from_bits(1f64.to_bits() - 1),
            TriangleLocation3::Interior,
        ),
        (1., TriangleLocation3::Edge(0)),
        (
            f64::from_bits(1f64.to_bits() + 1),
            TriangleLocation3::Outside,
        ),
    ] {
        let data = [x, 1., -1., x, 1., 1., 0., 0., 0., 2., 0., 0., 0., 2., 0.];
        let arena = SourceArena::authored(
            "near-edge",
            1,
            data.into_iter()
                .map(|v| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let p = unique(
            intersect_line_plane3d(
                &mut ctx,
                input(&arena, 0),
                input(&arena, 1),
                input(&arena, 2),
                input(&arena, 3),
                input(&arena, 4),
            )
            .unwrap()
            .outcome,
        );
        assert_eq!(
            classify_line_plane_domains3d(&mut ctx, &p)
                .unwrap()
                .outcome
                .unwrap()
                .triangle,
            expected
        );
        let mut limited = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        assert_eq!(
            classify_line_plane_domains3d(&mut limited, &p)
                .unwrap()
                .outcome,
            Err(Reason::ResourceLimit)
        );
        let cancel = std::sync::atomic::AtomicBool::new(true);
        let mut cancelled =
            PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
        assert_eq!(
            classify_line_plane_domains3d(&mut cancelled, &p)
                .unwrap()
                .outcome,
            Err(Reason::Cancelled)
        );
        let foreign = ToleranceContext::default_valid();
        let mut wrong = PredicateContext::new(&arena, &foreign, Limits::default(), Some(&cancel));
        assert!(matches!(
            classify_line_plane_domains3d(&mut wrong, &p),
            Err(InputError::ContextMismatch)
        ));
    }
}

// Independent affine oracle: enumerate boundary intersections and contained
// endpoints, then sort valid candidates. Production incrementally clips bounds.
fn coplanar_oracle(a: [i128; 2], b: [i128; 2]) -> Option<[Q; 2]> {
    let mut candidates = Vec::new();
    let mut accept = |t: Q| {
        if t.n < 0 || t.n > t.d {
            return;
        }
        let x = Q::int(a[0]).add(t.mul(Q::int(b[0] - a[0])));
        let y = Q::int(a[1]).add(t.mul(Q::int(b[1] - a[1])));
        if x.n >= 0 && y.n >= 0 && Q::int(4).sub(x).sub(y).n >= 0 {
            candidates.push(t);
        }
    };
    accept(Q::int(0));
    accept(Q::int(1));
    for (start, end) in [
        (a[0], b[0]),
        (a[1], b[1]),
        (4 - a[0] - a[1], 4 - b[0] - b[1]),
    ] {
        if start != end {
            accept(Q::new(-start, end - start));
        }
    }
    candidates.sort_by(|a, b| (a.n * b.d).cmp(&(b.n * a.d)));
    candidates
        .first()
        .copied()
        .map(|first| [first, *candidates.last().unwrap()])
}
#[test]
fn coplanar_clipping_matches_boundary_enumeration_under_affine_placements() {
    let tolerance = ToleranceContext::default_valid();
    let mut hits = 0;
    let mut misses = 0;
    let mut contacts = 0;
    for ax in -1..=5i128 {
        for ay in -1..=5i128 {
            for b in [[-1, 2], [5, 2], [0, 0], [4, 0], [0, 4], [2, 2], [1, 1]] {
                let a = [ax, ay];
                if a == b {
                    continue;
                }
                let expected = coplanar_oracle(a, b);
                for reflected in [false, true] {
                    // Non-axis plane with exact integer affine shear/translation.
                    let place = |p: [i128; 2]| {
                        let [x, y] = p;
                        let x = if reflected { -x } else { x };
                        [x + 2 * y + 3, 2 * x - y - 5, x + y + 7]
                    };
                    let data = [a, b, [0, 0], [4, 0], [0, 4]].map(place);
                    let arena = SourceArena::authored(
                        "coplanar-matrix",
                        1,
                        data.into_iter()
                            .flatten()
                            .map(|n| Q::int(n).scalar())
                            .collect(),
                    )
                    .unwrap();
                    for order in [[2, 3, 4], [4, 3, 2]] {
                        let mut ctx =
                            PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                        let result = intersect_coplanar_segment_triangle3d(
                            &mut ctx,
                            input(&arena, 0),
                            input(&arena, 1),
                            input(&arena, order[0]),
                            input(&arena, order[1]),
                            input(&arena, order[2]),
                        )
                        .unwrap()
                        .outcome;
                        match (expected, result) {
                            (None, CoplanarIntersection3::Empty) => misses += 1,
                            (Some([lo, hi]), CoplanarIntersection3::Overlap(p)) => {
                                enclosed(lo, p.parameter_enclosures()[0]);
                                enclosed(hi, p.parameter_enclosures()[1]);
                                assert_eq!(p.is_singleton(), lo == hi);
                                assert_eq!(p.context(), ctx.identity());
                                for (endpoint, t) in
                                    [(OverlapEndpoint3::Lower, lo), (OverlapEndpoint3::Upper, hi)]
                                {
                                    let mut check = PredicateContext::new(
                                        &arena,
                                        &tolerance,
                                        Limits::default(),
                                        None,
                                    );
                                    let point =
                                        construct_overlap_endpoint3d(&mut check, &p, endpoint)
                                            .unwrap()
                                            .outcome
                                            .unwrap();
                                    for axis in 0..3 {
                                        let expected = Q::int(data[0][axis])
                                            .add(t.mul(Q::int(data[1][axis] - data[0][axis])));
                                        enclosed(expected, point.enclosure()[axis]);
                                    }
                                    enclosed(t, point.line_parameter_enclosure());
                                    assert_eq!(
                                        orient3d_points(
                                            &mut check,
                                            input(&arena, 2),
                                            input(&arena, 3),
                                            input(&arena, 4),
                                            Point3Input::Constructed(&point)
                                        )
                                        .unwrap()
                                        .outcome,
                                        Outcome::Sign(Sign::Zero)
                                    );
                                    assert!(
                                        classify_line_plane_domains3d(&mut check, &point)
                                            .unwrap()
                                            .outcome
                                            .unwrap()
                                            .is_closed_domain_hit()
                                    );
                                }

                                if lo == hi {
                                    contacts += 1;
                                }
                                hits += 1;
                            }
                            (expected, result) => panic!("{a:?} {b:?}: {expected:?} vs {result:?}"),
                        }
                    }
                }
            }
        }
    }
    assert!(
        hits > 100 && misses > 100 && contacts > 20,
        "{hits} {misses} {contacts}"
    );
}
#[test]
fn coplanar_query_distinguishes_off_plane_degeneracy_and_resource_failure() {
    let data = [0., 0., 0., 1., 1., 0., 0., 0., 1., 4., 0., 0., 0., 4., 0.];
    let arena = SourceArena::authored(
        "coplanar-cases",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    for (a, b, c, d, e, expected) in [(0, 2, 0, 3, 4, 0), (0, 0, 0, 3, 4, 1), (0, 1, 0, 0, 4, 2)] {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let result = intersect_coplanar_segment_triangle3d(
            &mut ctx,
            input(&arena, a),
            input(&arena, b),
            input(&arena, c),
            input(&arena, d),
            input(&arena, e),
        )
        .unwrap()
        .outcome;
        assert!(matches!(
            (expected, result),
            (0, CoplanarIntersection3::NotCoplanar)
                | (1, CoplanarIntersection3::DegenerateSegment)
                | (2, CoplanarIntersection3::DegenerateTriangle)
        ));
    }
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        intersect_coplanar_segment_triangle3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 0),
            input(&arena, 3),
            input(&arena, 4)
        )
        .unwrap()
        .outcome,
        CoplanarIntersection3::Indeterminate(Reason::ResourceLimit)
    ));
}

#[test]
fn overlap_endpoint_recipes_survive_source_handle_drop_and_enforce_depth_limit() {
    let data = [-1., 1., 0., 2., 2., 0., 0., 0., 0., 4., 0., 0., 0., 4., 0.];
    let arena = SourceArena::authored(
        "overlap-chain",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut last = None;
    for depth in 1..=MAX_CONSTRUCTION_DEPTH + 1 {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let a = last
            .as_ref()
            .map(Point3Input::Constructed)
            .unwrap_or(input(&arena, 0));
        let overlap = match intersect_coplanar_segment_triangle3d(
            &mut ctx,
            a,
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome
        {
            CoplanarIntersection3::Overlap(p) => p,
            other => panic!("{other:?}"),
        };
        let result = construct_overlap_endpoint3d(&mut ctx, &overlap, OverlapEndpoint3::Lower)
            .unwrap()
            .outcome;
        if depth > MAX_CONSTRUCTION_DEPTH {
            assert!(matches!(result, Err(Reason::ResourceLimit)));
            break;
        }
        let point = result.unwrap();
        assert_eq!(point.recipe_depth(), depth);
        assert_eq!(point.recipe_node_count(), depth);
        enclosed(Q::int(0), point.enclosure()[0]);
        enclosed(Q::new(4, 3), point.enclosure()[1]);
        assert_eq!(
            orient3d_points(
                &mut ctx,
                input(&arena, 2),
                input(&arena, 3),
                input(&arena, 4),
                Point3Input::Constructed(&point)
            )
            .unwrap()
            .outcome,
            Outcome::Sign(Sign::Zero)
        );
        drop(overlap);
        last = Some(point);
    }
    let last = last.unwrap();
    assert!(format!("{last:?}").len() < 600);
    let mut export_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let graph = export_point3d(&mut export_ctx, &last)
        .unwrap()
        .outcome
        .unwrap();
    let binary = encode_recipe_graph3(&graph).unwrap();
    let decoded = decode_recipe_graph3(&binary).unwrap();
    let mut replay_ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let replayed = replay_point3d(&mut replay_ctx, &decoded)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(replayed.recipe_depth(), MAX_CONSTRUCTION_DEPTH);
    assert_eq!(replayed.recipe_node_count(), MAX_CONSTRUCTION_DEPTH);
    enclosed(Q::new(4, 3), replayed.enclosure()[1]);

    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert_eq!(
        orient3d_points(
            &mut ctx,
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
            Point3Input::Constructed(&last)
        )
        .unwrap()
        .outcome,
        Outcome::Indeterminate(Reason::Cancelled)
    );
}
#[test]
fn singleton_overlap_endpoints_are_exactly_equal_and_context_checked() {
    let data = [-1., 1., 0., 1., -1., 0., 0., 0., 0., 4., 0., 0., 0., 4., 0.];
    let arena = SourceArena::authored(
        "singleton",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let overlap = match intersect_coplanar_segment_triangle3d(
        &mut ctx,
        input(&arena, 0),
        input(&arena, 1),
        input(&arena, 2),
        input(&arena, 3),
        input(&arena, 4),
    )
    .unwrap()
    .outcome
    {
        CoplanarIntersection3::Overlap(p) => p,
        other => panic!("{other:?}"),
    };
    assert!(overlap.is_singleton());
    let lower = construct_overlap_endpoint3d(&mut ctx, &overlap, OverlapEndpoint3::Lower)
        .unwrap()
        .outcome
        .unwrap();
    let upper = construct_overlap_endpoint3d(&mut ctx, &overlap, OverlapEndpoint3::Upper)
        .unwrap()
        .outcome
        .unwrap();
    assert!(matches!(
        intersect_line_plane3d(
            &mut ctx,
            Point3Input::Constructed(&lower),
            Point3Input::Constructed(&upper),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4)
        )
        .unwrap()
        .outcome,
        LinePlaneIntersection3::DegenerateLine
    ));
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert!(matches!(
        construct_overlap_endpoint3d(&mut cancelled, &overlap, OverlapEndpoint3::Lower)
            .unwrap()
            .outcome,
        Err(Reason::Cancelled)
    ));
    let foreign = ToleranceContext::default_valid();
    let mut wrong = PredicateContext::new(&arena, &foreign, Limits::default(), Some(&cancel));
    assert!(matches!(
        construct_overlap_endpoint3d(&mut wrong, &overlap, OverlapEndpoint3::Lower),
        Err(InputError::ContextMismatch)
    ));
    let mut limited = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        construct_overlap_endpoint3d(&mut limited, &overlap, OverlapEndpoint3::Upper)
            .unwrap()
            .outcome,
        Err(Reason::ResourceLimit)
    ));
}

#[test]
fn unified_segment_triangle_covers_crossing_coplanar_empty_and_degenerate_cases() {
    let tolerance = ToleranceContext::default_valid();
    for (a, b, kind) in [
        ([1., 1., -1.], [1., 1., 1.], 1),
        ([1., 1., 0.], [1., 1., 1.], 1),
        ([1., 1., 1.], [1., 1., 2.], 0),
        ([5., 1., -1.], [5., 1., 1.], 0),
        ([-1., 1., 0.], [5., 1., 0.], 2),
        ([-1., 1., 0.], [1., -1., 0.], 1),
        ([-2., -1., 0.], [-1., -2., 0.], 0),
        ([1., 1., 1.], [2., 1., 1.], 0),
        ([1., 1., 0.], [1., 1., 0.], 3),
    ] {
        let points = [a, b, [0., 0., 0.], [4., 0., 0.], [0., 4., 0.]];
        let arena = SourceArena::authored(
            "unified",
            1,
            points
                .into_iter()
                .flatten()
                .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        for reverse in [false, true] {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let [ai, bi] = if reverse { [1, 0] } else { [0, 1] };
            let report = intersect_segment_triangle3d(
                &mut ctx,
                input(&arena, ai),
                input(&arena, bi),
                input(&arena, 2),
                input(&arena, 3),
                input(&arena, 4),
            )
            .unwrap();
            assert_eq!(report.context, ctx.identity());
            match report.outcome {
                SegmentTriangleIntersection3::Empty => assert_eq!(kind, 0),
                SegmentTriangleIntersection3::Point(p) => {
                    assert_eq!(kind, 1);
                    assert!(p.location.is_closed_domain_hit());
                    assert_eq!(
                        orient3d_points(
                            &mut ctx,
                            input(&arena, 2),
                            input(&arena, 3),
                            input(&arena, 4),
                            Point3Input::Constructed(&p.point)
                        )
                        .unwrap()
                        .outcome,
                        Outcome::Sign(Sign::Zero)
                    );
                }
                SegmentTriangleIntersection3::Segment(p) => {
                    assert_eq!(kind, 2);
                    for (i, expected_x) in (if reverse { [3, 0] } else { [0, 3] })
                        .into_iter()
                        .enumerate()
                    {
                        assert!(p[i].location.is_closed_domain_hit());
                        enclosed(Q::int(expected_x), p[i].point.enclosure()[0]);
                        enclosed(Q::int(1), p[i].point.enclosure()[1]);
                    }
                }
                SegmentTriangleIntersection3::DegenerateSegment => assert_eq!(kind, 3),
                other => panic!("{other:?}"),
            }
        }
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let out = intersect_segment_triangle3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 2),
            input(&arena, 4),
        )
        .unwrap()
        .outcome;
        if a != b {
            assert!(matches!(
                out,
                SegmentTriangleIntersection3::DegenerateTriangle
            ));
        }
    }
}
#[test]
fn unified_query_never_publishes_partial_geometry_when_work_expires() {
    let data = [-1., 1., 0., 5., 1., 0., 0., 0., 0., 4., 0., 0., 0., 4., 0.];
    let arena = SourceArena::authored(
        "atomic-unified",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let complete = intersect_segment_triangle3d(
        &mut ctx,
        input(&arena, 0),
        input(&arena, 1),
        input(&arena, 2),
        input(&arena, 3),
        input(&arena, 4),
    )
    .unwrap();
    assert!(matches!(
        complete.outcome,
        SegmentTriangleIntersection3::Segment(_)
    ));
    for max_work in [1, 100, complete.work_used / 2, complete.work_used - 1] {
        let mut limited = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work,
                ..Limits::default()
            },
            None,
        );
        assert!(matches!(
            intersect_segment_triangle3d(
                &mut limited,
                input(&arena, 0),
                input(&arena, 1),
                input(&arena, 2),
                input(&arena, 3),
                input(&arena, 4)
            )
            .unwrap()
            .outcome,
            SegmentTriangleIntersection3::Indeterminate(Reason::ResourceLimit)
        ));
    }
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert!(matches!(
        intersect_segment_triangle3d(
            &mut cancelled,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4)
        )
        .unwrap()
        .outcome,
        SegmentTriangleIntersection3::Indeterminate(Reason::Cancelled)
    ));
    let foreign = SourceArena::authored(
        "foreign",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    assert!(matches!(
        intersect_segment_triangle3d(
            &mut cancelled,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&foreign, 4)
        ),
        Err(InputError::ProvenanceMismatch)
    ));
}

#[test]
fn exact_point_order_deduplicates_recipes_without_merging_rounded_neighbors() {
    let data = [
        0.,
        0.,
        0.,
        1.,
        0.,
        3.,
        0.,
        0.,
        1.,
        1.,
        0.,
        1.,
        0.,
        1.,
        1.,
        1. / 3.,
        0.,
        1.,
        1. / 3.,
        1.,
        1.,
        1. / 3.,
        0.,
        2.,
    ];
    let arena = SourceArena::authored(
        "point-order",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let p = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    let q = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 1),
            input(&arena, 0),
            input(&arena, 4),
            input(&arena, 3),
            input(&arena, 2),
        )
        .unwrap()
        .outcome,
    );
    for (a, b, expected) in [
        (
            Point3Input::Constructed(&p),
            Point3Input::Constructed(&q),
            Sign::Zero,
        ),
        (
            Point3Input::Constructed(&p),
            input(&arena, 5),
            Sign::Positive,
        ),
        (
            input(&arena, 5),
            Point3Input::Constructed(&p),
            Sign::Negative,
        ),
        (input(&arena, 5), input(&arena, 6), Sign::Negative),
        (input(&arena, 5), input(&arena, 7), Sign::Negative),
    ] {
        assert_eq!(
            compare_points3d(&mut ctx, a, b).unwrap().outcome,
            Outcome::Sign(expected)
        );
    }
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert_eq!(
        compare_points3d(
            &mut cancelled,
            Point3Input::Constructed(&p),
            Point3Input::Constructed(&p)
        )
        .unwrap()
        .outcome,
        Outcome::Indeterminate(Reason::Cancelled)
    );
    let foreign = ToleranceContext::default_valid();
    let mut wrong = PredicateContext::new(&arena, &foreign, Limits::default(), Some(&cancel));
    assert_eq!(
        compare_points3d(&mut wrong, Point3Input::Constructed(&p), input(&arena, 0)),
        Err(InputError::ContextMismatch)
    );
}

#[test]
fn triangle_pair_assembles_exact_shared_segments_contacts_and_explicit_coplanarity() {
    let tolerance = ToleranceContext::default_valid();
    for (second, kind, expected) in [
        (
            [[1., -1., -1.], [1., 3., -1.], [1., 1., 1.]],
            2,
            [[1., 0., 0.], [1., 2., 0.]],
        ),
        (
            [[0., 0., 0.], [-1., 0., 1.], [0., -1., 1.]],
            1,
            [[0., 0., 0.]; 2],
        ),
        (
            [[5., 0., -1.], [5., 4., -1.], [5., 0., 1.]],
            0,
            [[0.; 3]; 2],
        ),
        ([[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]], 3, [[0.; 3]; 2]),
        ([[0., 0., 0.], [0., 0., 0.], [0., 1., 0.]], 4, [[0.; 3]; 2]),
    ] {
        let points = [
            [0., 0., 0.],
            [4., 0., 0.],
            [0., 4., 0.],
            second[0],
            second[1],
            second[2],
        ];
        let arena = SourceArena::authored(
            "triangle-pair",
            1,
            points
                .into_iter()
                .flatten()
                .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        for reverse in [false, true] {
            for swap in [false, true] {
                let a = if reverse { [2, 1, 0] } else { [0, 1, 2] };
                let b = if reverse { [5, 4, 3] } else { [3, 4, 5] };
                let (a, b) = if swap { (b, a) } else { (a, b) };
                let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                match intersect_triangles3d(
                    &mut ctx,
                    a.map(|i| input(&arena, i)),
                    b.map(|i| input(&arena, i)),
                )
                .unwrap()
                .outcome
                {
                    TriangleTriangleIntersection3::Empty => assert_eq!(kind, 0),
                    TriangleTriangleIntersection3::Point(p) => {
                        assert_eq!(kind, 1);
                        for (i, q) in expected[0].iter().enumerate() {
                            enclosed(Q::int(*q as i128), p.enclosure()[i]);
                        }
                    }
                    TriangleTriangleIntersection3::Segment(p) => {
                        assert_eq!(kind, 2);
                        for j in 0..2 {
                            for (coordinate, bound) in expected[j].into_iter().zip(p[j].enclosure())
                            {
                                enclosed(Q::int(coordinate as i128), bound);
                            }
                        }
                    }
                    TriangleTriangleIntersection3::Polygon { vertices, .. } => {
                        assert_eq!(kind, 3);
                        assert_eq!(vertices.len(), 3);
                    }
                    TriangleTriangleIntersection3::DegenerateTriangle(i) => {
                        assert_eq!(kind, 4);
                        assert_eq!(i, if swap { 0 } else { 1 });
                    }
                    other => panic!("{other:?}"),
                }
            }
        }
    }
}

#[test]
fn coplanar_hexagon_is_canonical_convex_and_exact_in_every_projection() {
    let tolerance = ToleranceContext::default_valid();
    let source = [[0, 0], [6, 0], [3, 6], [0, 4], [6, 4], [3, -2]];
    let expected = [[1, 2], [2, 0], [4, 0], [5, 2], [4, 4], [2, 4]];
    for placement in 0..4 {
        let place = |[x, y]: [i128; 2]| match placement {
            0 => [x, y, 0],
            1 => [x, 0, y],
            2 => [0, x, y],
            _ => [-x + 2 * y + 3, 2 * x - y - 5, x + y + 7],
        };
        let arena = SourceArena::authored(
            "hexagon",
            1,
            source
                .map(place)
                .into_iter()
                .flatten()
                .map(|n| Q::int(n).scalar())
                .collect(),
        )
        .unwrap();
        for reverse in [false, true] {
            for swap in [false, true] {
                let a = if reverse { [2, 1, 0] } else { [0, 1, 2] };
                let b = if reverse { [5, 4, 3] } else { [3, 4, 5] };
                let (a, b) = if swap { (b, a) } else { (a, b) };
                let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                let report = intersect_triangles3d(
                    &mut ctx,
                    a.map(|i| input(&arena, i)),
                    b.map(|i| input(&arena, i)),
                )
                .unwrap();
                let TriangleTriangleIntersection3::Polygon {
                    vertices,
                    projection_axes: axes,
                } = report.outcome
                else {
                    panic!("{:?}", report.outcome);
                };
                assert_eq!(vertices.len(), 6);
                let mut expected = expected.map(place).to_vec();
                let mut ordered = Vec::new();
                for p in &vertices {
                    let index = expected
                        .iter()
                        .position(|q| {
                            q.iter().zip(p.enclosure()).all(|(n, b)| {
                                compare_float(Q::int(*n), b[0]) != Ordering::Less
                                    && compare_float(Q::int(*n), b[1]) != Ordering::Greater
                            })
                        })
                        .unwrap();
                    ordered.push(expected.remove(index));
                    assert_eq!(
                        orient3d_points(
                            &mut ctx,
                            input(&arena, 0),
                            input(&arena, 1),
                            input(&arena, 2),
                            Point3Input::Constructed(p)
                        )
                        .unwrap()
                        .outcome,
                        Outcome::Sign(Sign::Zero)
                    );
                }
                assert!(expected.is_empty());
                assert_eq!(ordered[0], *ordered.iter().min().unwrap());
                for i in 0..6 {
                    let [a, b, c] = [ordered[i], ordered[(i + 1) % 6], ordered[(i + 2) % 6]];
                    let [x, y] = axes;
                    assert!((b[x] - a[x]) * (c[y] - a[y]) - (b[y] - a[y]) * (c[x] - a[x]) > 0);
                }
            }
        }
    }
}
#[test]
fn coplanar_face_contacts_reduce_to_point_or_segment_without_duplicate_vertices() {
    let tolerance = ToleranceContext::default_valid();
    for (b, count) in [
        ([[0., 0., 0.], [4., 0., 0.], [0., -4., 0.]], 2),
        ([[4., 0., 0.], [5., 0., 0.], [4., 1., 0.]], 1),
        ([[5., 0., 0.], [6., 0., 0.], [5., 1., 0.]], 0),
    ] {
        let points = [[0., 0., 0.], [4., 0., 0.], [0., 4., 0.], b[0], b[1], b[2]];
        let arena = SourceArena::authored(
            "coplanar-contact",
            1,
            points
                .into_iter()
                .flatten()
                .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        match intersect_triangles3d(
            &mut ctx,
            [0, 1, 2].map(|i| input(&arena, i)),
            [3, 4, 5].map(|i| input(&arena, i)),
        )
        .unwrap()
        .outcome
        {
            TriangleTriangleIntersection3::Empty => assert_eq!(count, 0),
            TriangleTriangleIntersection3::Point(_) => assert_eq!(count, 1),
            TriangleTriangleIntersection3::Segment(p) => {
                assert_eq!(count, 2);
                enclosed(Q::int(0), p[0].enclosure()[0]);
                enclosed(Q::int(4), p[1].enclosure()[0]);
            }
            other => panic!("{other:?}"),
        }
    }
}

fn affine_turn2(a: [Q; 2], b: [Q; 2], p: [Q; 2]) -> Q {
    b[0].sub(a[0])
        .mul(p[1].sub(a[1]))
        .sub(b[1].sub(a[1]).mul(p[0].sub(a[0])))
}
// Separate rational Sutherland-Hodgman oracle, not edge enumeration + hull.
fn clipped_triangle_oracle(a: [[Q; 2]; 3], mut b: [[Q; 2]; 3]) -> Vec<[Q; 2]> {
    if affine_turn2(b[0], b[1], b[2]).n < 0 {
        b.swap(1, 2);
    }
    let mut polygon = a.to_vec();
    for i in 0..3 {
        if polygon.is_empty() {
            break;
        }
        let mut output = Vec::new();
        for j in 0..polygon.len() {
            let p = polygon[j];
            let q = polygon[(j + 1) % polygon.len()];
            let dp = affine_turn2(b[i], b[(i + 1) % 3], p);
            let dq = affine_turn2(b[i], b[(i + 1) % 3], q);
            if (dp.n < 0) != (dq.n < 0) {
                let t = dp.div(dp.sub(dq));
                output.push(std::array::from_fn(|axis| {
                    p[axis].add(t.mul(q[axis].sub(p[axis])))
                }));
            }
            if dq.n >= 0 {
                output.push(q);
            }
        }
        polygon = output;
    }
    polygon.dedup();
    if polygon.len() > 1 && polygon.first() == polygon.last() {
        polygon.pop();
    }
    if polygon.len() >= 3
        && polygon
            .iter()
            .all(|p| affine_turn2(polygon[0], polygon[1], *p).n == 0)
    {
        polygon.sort_by(|a, b| {
            (a[0].n * b[0].d)
                .cmp(&(b[0].n * a[0].d))
                .then_with(|| (a[1].n * b[1].d).cmp(&(b[1].n * a[1].d)))
        });
        polygon.dedup();
        if polygon.len() > 2 {
            return vec![polygon[0], *polygon.last().unwrap()];
        }
        return polygon;
    }
    loop {
        if polygon.len() < 3 {
            break;
        }
        let redundant = (0..polygon.len()).find(|i| {
            affine_turn2(
                polygon[(*i + polygon.len() - 1) % polygon.len()],
                polygon[*i],
                polygon[(*i + 1) % polygon.len()],
            )
            .n == 0
        });
        if let Some(i) = redundant {
            polygon.remove(i);
        } else {
            break;
        }
    }
    polygon.sort_by(|a, b| {
        (a[0].n * b[0].d)
            .cmp(&(b[0].n * a[0].d))
            .then_with(|| (a[1].n * b[1].d).cmp(&(b[1].n * a[1].d)))
    });
    polygon.dedup();
    polygon
}
#[test]
fn rational_polygon_matrix_matches_independent_sequential_clipping() {
    let tolerance = ToleranceContext::default_valid();
    let mut cases = 0;
    let mut fractional = 0;
    let mut areas = 0;
    let mut empty = 0;
    let mut state = 123456789u64;
    for _ in 0..180 {
        let mut coordinate = || {
            state = state
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            Q::int(((state >> 32) % 13) as i128 - 6)
        };
        let a = std::array::from_fn(|_| [coordinate(), coordinate()]);
        let b = std::array::from_fn(|_| [coordinate(), coordinate()]);
        if affine_turn2(a[0], a[1], a[2]).n == 0 || affine_turn2(b[0], b[1], b[2]).n == 0 {
            continue;
        }
        let expected = clipped_triangle_oracle(a, b);
        fractional += expected.iter().flatten().filter(|q| q.d != 1).count();
        let values = a
            .into_iter()
            .chain(b)
            .flat_map(|[x, y]| [x, y, Q::int(0)])
            .map(Q::scalar)
            .collect();
        let arena = SourceArena::authored("polygon-rational-matrix", 1, values).unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let result = intersect_triangles3d(
            &mut ctx,
            [0, 1, 2].map(|i| input(&arena, i)),
            [3, 4, 5].map(|i| input(&arena, i)),
        )
        .unwrap()
        .outcome;
        let mut actual = match result {
            TriangleTriangleIntersection3::Empty => {
                empty += 1;
                Vec::new()
            }
            TriangleTriangleIntersection3::Point(p) => vec![*p],
            TriangleTriangleIntersection3::Segment(p) => Vec::from(*p),
            TriangleTriangleIntersection3::Polygon { vertices, .. } => {
                areas += 1;
                vertices
            }
            other => panic!("{a:?} {b:?}: {other:?}"),
        };
        assert_eq!(actual.len(), expected.len(), "{a:?} {b:?}");
        for q in expected {
            let index = actual
                .iter()
                .position(|p| {
                    q.into_iter().zip(p.enclosure()).all(|(q, b)| {
                        compare_float(q, b[0]) != Ordering::Less
                            && compare_float(q, b[1]) != Ordering::Greater
                    })
                })
                .unwrap();
            let point = actual.remove(index);
            enclosed(Q::int(0), point.enclosure()[2]);
            for triangle in [[0, 1, 2], [3, 4, 5]] {
                assert!(matches!(
                    classify_point_triangle3d(
                        &mut ctx,
                        Point3Input::Constructed(&point),
                        triangle.map(|i| input(&arena, i))
                    )
                    .unwrap()
                    .outcome,
                    PointTriangleLocation3::OnPlane(
                        TriangleLocation3::Interior
                            | TriangleLocation3::Edge(_)
                            | TriangleLocation3::Vertex(_)
                    )
                ));
            }
        }
        cases += 1;
    }
    assert!(
        cases > 140 && fractional > 100 && areas > 80 && empty > 5,
        "{cases} {fractional} {areas} {empty}"
    );
}

#[test]
fn point_triangle_reports_all_features_and_exact_off_plane_sign() {
    let tolerance = ToleranceContext::default_valid();
    for (p, expected) in [
        (
            [0., 0., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Vertex(0)),
        ),
        (
            [4., 0., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Vertex(1)),
        ),
        (
            [0., 4., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Vertex(2)),
        ),
        (
            [2., 2., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Edge(0)),
        ),
        (
            [0., 2., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Edge(1)),
        ),
        (
            [2., 0., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Edge(2)),
        ),
        (
            [1., 1., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Interior),
        ),
        (
            [3., 3., 0.],
            PointTriangleLocation3::OnPlane(TriangleLocation3::Outside),
        ),
        (
            [1., 1., 1e-12],
            PointTriangleLocation3::OffPlane(Sign::Positive),
        ),
        (
            [1., 1., -1e-12],
            PointTriangleLocation3::OffPlane(Sign::Negative),
        ),
    ] {
        let arena = SourceArena::authored(
            "point-triangle",
            1,
            [p, [0., 0., 0.], [4., 0., 0.], [0., 4., 0.]]
                .into_iter()
                .flatten()
                .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
                .collect(),
        )
        .unwrap();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        assert_eq!(
            classify_point_triangle3d(
                &mut ctx,
                input(&arena, 0),
                [1, 2, 3].map(|i| input(&arena, i))
            )
            .unwrap()
            .outcome,
            expected
        );
        assert_eq!(
            classify_point_triangle3d(
                &mut ctx,
                input(&arena, 0),
                [1, 1, 3].map(|i| input(&arena, i))
            )
            .unwrap()
            .outcome,
            PointTriangleLocation3::DegenerateTriangle
        );
    }
}

#[test]
fn portable_graph_preserves_shared_dependencies_and_authored_rationals() {
    let mut values = [0, 0, 0, 1, 0, 3, 0, 0, 1, 1, 0, 1, 0, 1, 1]
        .map(|n| Q::int(n).scalar())
        .to_vec();
    values[3] = Q::new(1, 3).scalar();
    let arena = SourceArena::authored("portable-source", 17, values).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let p = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    let q = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            Point3Input::Constructed(&p),
            Point3Input::Constructed(&p),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    drop(p);
    let graph = export_point3d(&mut ctx, &q).unwrap().outcome.unwrap();
    assert_eq!(graph.source_id, "portable-source");
    assert_eq!(graph.source_revision, 17);
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.root, 1);
    assert_eq!(graph.nodes[1].inputs[1], RecipeInput3Record::Node(0));
    assert_eq!(graph.nodes[1].inputs[2], RecipeInput3Record::Node(0));
    let replayed = replay_point3d(&mut ctx, &graph).unwrap().outcome.unwrap();
    assert_eq!(replayed.recipe_node_count(), 2);
    assert_eq!(
        compare_points3d(
            &mut ctx,
            Point3Input::Constructed(&q),
            Point3Input::Constructed(&replayed)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );

    let RecipeInput3Record::Authored { indices, values } = &graph.nodes[0].inputs[1] else {
        panic!("Expected authored B");
    };
    assert_eq!(*indices, [3, 4, 5]);
    assert_eq!(values[0], Q::new(1, 3).scalar());
    for (index, node) in graph.nodes.iter().enumerate() {
        for input in &node.inputs {
            if let RecipeInput3Record::Node(dependency) = input {
                assert!(*dependency < index);
            }
        }
    }
    assert_eq!(
        export_point3d(&mut ctx, &q).unwrap().outcome.unwrap().nodes,
        graph.nodes
    );
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert!(matches!(
        export_point3d(&mut cancelled, &q).unwrap().outcome,
        Err(Reason::Cancelled)
    ));
    let foreign = ToleranceContext::default_valid();
    let mut wrong = PredicateContext::new(&arena, &foreign, Limits::default(), Some(&cancel));
    assert!(matches!(
        export_point3d(&mut wrong, &q),
        Err(InputError::ContextMismatch)
    ));
}

#[test]
fn graph_replay_rebinds_verified_source_and_rejects_structural_or_value_tampering() {
    let values = [0, 0, 0, 1, 0, 3, 0, 0, 1, 1, 0, 1, 0, 1, 1]
        .map(|n| Q::int(n).scalar())
        .to_vec();
    let arena = SourceArena::authored("replay", 2, values.clone()).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let point = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    let graph = export_point3d(&mut ctx, &point).unwrap().outcome.unwrap();
    let restored_arena = SourceArena::authored("replay", 2, values).unwrap();
    let restored_tolerance = ToleranceContext::new(tolerance.specification().clone()).unwrap();
    let mut restored = PredicateContext::new(
        &restored_arena,
        &restored_tolerance,
        Limits::default(),
        None,
    );
    let replayed = replay_point3d(&mut restored, &graph)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(replayed.context(), restored.identity());
    enclosed(Q::new(1, 3), replayed.enclosure()[0]);
    assert_eq!(
        orient3d_points(
            &mut restored,
            input(&restored_arena, 2),
            input(&restored_arena, 3),
            input(&restored_arena, 4),
            Point3Input::Constructed(&replayed)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );
    for mutation in 0..5 {
        let mut bad = graph.clone();
        match mutation {
            0 => bad.nodes[0].inputs[0] = RecipeInput3Record::Node(0),
            1 => {
                if let RecipeInput3Record::Authored { values, .. } = &mut bad.nodes[0].inputs[0] {
                    values[0] = Q::int(999).scalar();
                }
            }
            2 => bad.source_revision += 1,
            3 => bad.tolerance.on_tol *= 2.,
            _ => {
                bad.nodes.push(bad.nodes[0].clone());
                bad.root = 1;
            }
        }
        assert!(replay_point3d(&mut restored, &bad).is_err());
    }
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(
        &restored_arena,
        &restored_tolerance,
        Limits::default(),
        Some(&cancel),
    );
    assert!(matches!(
        replay_point3d(&mut cancelled, &graph).unwrap().outcome,
        Err(Reason::Cancelled)
    ));
    let mut invalid_operation = graph.clone();
    invalid_operation.nodes[0].kind = ConstructionKind3::CoplanarEndpoint(OverlapEndpoint3::Lower);
    assert!(matches!(
        replay_point3d(&mut restored, &invalid_operation)
            .unwrap()
            .outcome,
        Err(Reason::MissingProof)
    ));
}
#[test]
fn coplanar_endpoint_replay_retains_exact_coordinates() {
    let data = [-1., 1., 0., 2., 2., 0., 0., 0., 0., 4., 0., 0., 0., 4., 0.];
    let arena = SourceArena::authored(
        "clip-replay",
        1,
        data.into_iter()
            .map(|v: f64| AuthoredScalar::Binary64Bits(v.to_bits()))
            .collect(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let CoplanarIntersection3::Overlap(overlap) = intersect_coplanar_segment_triangle3d(
        &mut ctx,
        input(&arena, 0),
        input(&arena, 1),
        input(&arena, 2),
        input(&arena, 3),
        input(&arena, 4),
    )
    .unwrap()
    .outcome
    else {
        panic!("Expected overlap");
    };
    let point = construct_overlap_endpoint3d(&mut ctx, &overlap, OverlapEndpoint3::Lower)
        .unwrap()
        .outcome
        .unwrap();
    let graph = export_point3d(&mut ctx, &point).unwrap().outcome.unwrap();
    let replayed = replay_point3d(&mut ctx, &graph).unwrap().outcome.unwrap();
    assert_eq!(
        compare_points3d(
            &mut ctx,
            Point3Input::Constructed(&point),
            Point3Input::Constructed(&replayed)
        )
        .unwrap()
        .outcome,
        Outcome::Sign(Sign::Zero)
    );
    enclosed(Q::new(4, 3), replayed.enclosure()[1]);
}

#[test]
fn binary_recipe_roundtrip_replays_exactly_and_rejects_every_truncation() {
    let mut values = [0, 0, 0, 1, 0, 3, 0, 0, 1, 1, 0, 1, 0, 1, 1]
        .map(|n| Q::int(n).scalar())
        .to_vec();
    values[3] = Q::new(-1, 3).scalar();
    let arena = SourceArena::authored("binary-source", 9, values).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let p = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    let graph = export_point3d(&mut ctx, &p).unwrap().outcome.unwrap();
    let bytes = encode_recipe_graph3(&graph).unwrap();
    let decoded = decode_recipe_graph3(&bytes).unwrap();
    assert_eq!(decoded.nodes, graph.nodes);
    assert_eq!(encode_recipe_graph3(&decoded).unwrap(), bytes);
    let replayed = replay_point3d(&mut ctx, &decoded).unwrap().outcome.unwrap();
    enclosed(Q::new(-1, 9), replayed.enclosure()[0]);
    for end in 0..bytes.len() {
        assert!(
            decode_recipe_graph3(&bytes[..end]).is_err(),
            "accepted prefix {end}"
        );
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_recipe_graph3(&trailing).is_err());
    let mut wrong = bytes.clone();
    wrong[4] = 99;
    assert!(decode_recipe_graph3(&wrong).is_err());
    let mut huge_string = bytes.clone();
    huge_string[5..9].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_recipe_graph3(&huge_string).is_err());
    assert!(decode_recipe_graph3(&vec![0; MAX_RECIPE_GRAPH_BYTES + 1]).is_err());
}

#[test]
fn batch_replay_is_atomic_and_uses_one_budget() {
    let arena = SourceArena::authored(
        "batch",
        1,
        [0, 0, 0, 1, 0, 3, 0, 0, 1, 1, 0, 1, 0, 1, 1]
            .map(|n| Q::int(n).scalar())
            .to_vec(),
    )
    .unwrap();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let p = unique(
        intersect_line_plane3d(
            &mut ctx,
            input(&arena, 0),
            input(&arena, 1),
            input(&arena, 2),
            input(&arena, 3),
            input(&arena, 4),
        )
        .unwrap()
        .outcome,
    );
    let mut saving = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let saved = export_point3d_batch(&mut saving, &[&p]).unwrap();
    let save_cost = saved.work_used;
    let saved = saved.outcome.unwrap();
    let mut limited_save = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: save_cost,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        export_point3d_batch(&mut limited_save, &[&p, &p])
            .unwrap()
            .outcome,
        Err(Reason::ResourceLimit)
    ));
    let mut full_save = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert_eq!(
        export_point3d_batch(&mut full_save, &[&p, &p])
            .unwrap()
            .outcome
            .unwrap(),
        vec![saved[0].clone(), saved[0].clone()]
    );
    assert_eq!(full_save.work_used(), save_cost * 2);
    assert!(matches!(
        export_point3d_batch(&mut full_save, &vec![&p; MAX_RECIPE_BATCH_POINTS + 1])
            .unwrap()
            .outcome,
        Err(Reason::ResourceLimit)
    ));
    let graph = export_point3d(&mut ctx, &p).unwrap().outcome.unwrap();
    let bytes = encode_recipe_graph3(&graph).unwrap();
    let mut single = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    let report = replay_point3d_batch(&mut single, &[&bytes]).unwrap();
    let cost = report.work_used;
    enclosed(Q::new(1, 3), report.outcome.unwrap()[0].enclosure()[0]);
    let mut limited = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: cost,
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        replay_point3d_batch(&mut limited, &[&bytes, &bytes])
            .unwrap()
            .outcome,
        Err(Reason::ResourceLimit)
    ));
    let mut full = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert_eq!(
        replay_point3d_batch(&mut full, &[&bytes, &bytes])
            .unwrap()
            .outcome
            .unwrap()
            .len(),
        2
    );
    assert_eq!(full.work_used(), cost * 2);
    let mut corrupt = bytes.clone();
    corrupt[0] = 0;
    assert!(replay_point3d_batch(&mut full, &[&bytes, &corrupt]).is_err());
    let mut foreign = graph.clone();
    foreign.source_revision += 1;
    let foreign = encode_recipe_graph3(&foreign).unwrap();
    assert!(replay_point3d_batch(&mut full, &[&bytes, &foreign]).is_err());
    assert!(matches!(
        replay_point3d_batch(
            &mut full,
            &vec![bytes.as_slice(); MAX_RECIPE_BATCH_POINTS + 1]
        )
        .unwrap()
        .outcome,
        Err(Reason::ResourceLimit)
    ));
    let oversized = vec![0; MAX_RECIPE_GRAPH_BYTES + 1];
    assert!(matches!(
        replay_point3d_batch(&mut full, &[&oversized])
            .unwrap()
            .outcome,
        Err(Reason::ResourceLimit)
    ));
    let max_record = vec![0; MAX_RECIPE_GRAPH_BYTES];
    assert!(matches!(
        replay_point3d_batch(
            &mut full,
            &vec![max_record.as_slice(); MAX_RECIPE_BATCH_BYTES / MAX_RECIPE_GRAPH_BYTES + 1]
        )
        .unwrap()
        .outcome,
        Err(Reason::ResourceLimit)
    ));
    assert!(
        replay_point3d_batch(&mut full, &[])
            .unwrap()
            .outcome
            .unwrap()
            .is_empty()
    );
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut cancelled = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert!(matches!(
        replay_point3d_batch(&mut cancelled, &[&bytes])
            .unwrap()
            .outcome,
        Err(Reason::Cancelled)
    ));
}

#[test]
fn empty_batches_observe_cancellation_and_deadlines() {
    let arena = SourceArena::authored("empty-batch", 1, vec![]).unwrap();
    let tolerance = ToleranceContext::default_valid();
    let cancel = std::sync::atomic::AtomicBool::new(true);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), Some(&cancel));
    assert!(matches!(
        export_point3d_batch(&mut ctx, &[]).unwrap().outcome,
        Err(Reason::Cancelled)
    ));
    assert!(matches!(
        replay_point3d_batch(&mut ctx, &[]).unwrap().outcome,
        Err(Reason::Cancelled)
    ));
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            deadline: Some(std::time::Instant::now()),
            ..Limits::default()
        },
        None,
    );
    assert!(matches!(
        export_point3d_batch(&mut ctx, &[]).unwrap().outcome,
        Err(Reason::DeadlineExceeded)
    ));
    assert!(matches!(
        replay_point3d_batch(&mut ctx, &[]).unwrap().outcome,
        Err(Reason::DeadlineExceeded)
    ));
}
