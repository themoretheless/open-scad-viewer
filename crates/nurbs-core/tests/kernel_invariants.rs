//! Generated-input invariants of the curve kernel: every exact knot and
//! degree operation must leave the point set and its parameterization
//! unchanged, and the measuring queries must be consistent with them.
//!
//! Inputs come from a fixed-seed generator, so a failure reproduces as is.
use nurbs_core::curve::Curve;
use nurbs_core::curve_measure::length;
use nurbs_core::point_inversion::{PointInversionOptions, invert_point};

struct Lcg(u64);
impl Lcg {
    fn unit(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn below(&mut self, bound: usize) -> usize {
        (self.unit() * bound as f64) as usize
    }
    fn range(&mut self, lo: f64, hi: f64) -> f64 {
        lo + (hi - lo) * self.unit()
    }
}

/// A clamped curve of degree 1..=5 in two or three dimensions, with up to six
/// distinct interior knots; about two in three are rational.
fn curve(rng: &mut Lcg) -> Curve {
    let degree = 1 + rng.below(5);
    let interior = rng.below(7);
    let count = degree + 1 + interior;
    let dimension = 2 + rng.below(2);
    let end = rng.range(1., 5.);
    let mut inner: Vec<f64> = (0..interior)
        .map(|i| end * (i as f64 + rng.range(0.2, 0.8)) / interior as f64)
        .collect();
    inner.sort_by(f64::total_cmp);
    let knots = [vec![0.; degree + 1], inner, vec![end; degree + 1]].concat();
    let rational = rng.below(3) > 0;
    let curve = Curve {
        degree,
        knots,
        control_points: (0..count)
            .map(|i| {
                (0..dimension)
                    .map(|axis| {
                        if axis == 0 {
                            // A drift keeps the curve from folding onto itself.
                            2. * i as f64 + rng.range(-0.5, 0.5)
                        } else {
                            rng.range(-3., 3.)
                        }
                    })
                    .collect()
            })
            .collect(),
        weights: (0..count)
            .map(|_| if rational { rng.range(0.5, 2.) } else { 1. })
            .collect(),
        periodic: false,
    };
    curve.validate().unwrap();
    curve
}

fn point(curve: &Curve, u: f64) -> Vec<f64> {
    curve.evaluate(u).unwrap().point
}

/// Both curves trace the same points at the same parameters over `[a, b]`.
fn assert_same_on(original: &Curve, other: &Curve, [a, b]: [f64; 2], what: &str) {
    for step in 0..=32 {
        let u = (a + (b - a) * step as f64 / 32.).clamp(a, b);
        let (p, q) = (point(original, u), point(other, u));
        for (x, y) in p.iter().zip(&q) {
            assert!((x - y).abs() <= 1e-9, "{what} moved the curve at {u}: {p:?} vs {q:?}");
        }
    }
}

#[test]
fn exact_operations_preserve_points_and_parameters() {
    let mut rng = Lcg(0xC0FFEE);
    for _ in 0..300 {
        let source = curve(&mut rng);
        let domain = source.domain();
        let [a, b] = domain;
        let inside = |rng: &mut Lcg| rng.range(a + 0.05 * (b - a), b - 0.05 * (b - a));

        let inserted = source.insert(inside(&mut rng), 1).unwrap();
        assert_eq!(inserted.control_points.len(), source.control_points.len() + 1);
        assert_same_on(&source, &inserted, domain, "knot insertion");

        let mut new_knots: Vec<f64> = (0..4).map(|_| inside(&mut rng)).collect();
        new_knots.sort_by(f64::total_cmp);
        let refined = source.refine(&new_knots).unwrap();
        assert_eq!(refined.control_points.len(), source.control_points.len() + 4);
        assert_same_on(&source, &refined, domain, "knot refinement");

        let elevated = source.elevate(source.degree + 1 + rng.below(2)).unwrap();
        assert_same_on(&source, &elevated, domain, "degree elevation");

        let cut = inside(&mut rng);
        let [left, right] = source.split(cut).unwrap();
        assert_eq!((left.domain(), right.domain()), ([a, cut], [cut, b]));
        assert_same_on(&source, &left, [a, cut], "split (left)");
        assert_same_on(&source, &right, [cut, b], "split (right)");

        let segments = source.decompose().unwrap();
        assert_eq!(segments.first().unwrap().domain()[0], a);
        assert_eq!(segments.last().unwrap().domain()[1], b);
        for pair in segments.windows(2) {
            assert_eq!(pair[0].domain()[1], pair[1].domain()[0]);
        }
        for segment in &segments {
            let bezier = segment.definition();
            assert_eq!(bezier.control_points.len(), source.degree + 1);
            assert_same_on(&source, bezier, segment.domain(), "Bezier decomposition");
        }

        let reversed = source.reverse().unwrap();
        assert_eq!(reversed.domain(), domain);
        for step in 0..=32 {
            let u = a + (b - a) * step as f64 / 32.;
            let (p, q) = (point(&source, u), point(&reversed, (a + b - u).clamp(a, b)));
            for (x, y) in p.iter().zip(&q) {
                assert!((x - y).abs() <= 1e-9, "reversal moved the curve at {u}");
            }
        }
        let restored = reversed.reverse().unwrap();
        assert_same_on(&source, &restored, domain, "double reversal");
    }
}

#[test]
fn derivatives_survive_refinement_and_elevation() {
    let mut rng = Lcg(0xD1FF);
    for _ in 0..200 {
        let source = curve(&mut rng);
        let [a, b] = source.domain();
        let knot = rng.range(a + 0.1 * (b - a), b - 0.1 * (b - a));
        let changed = source
            .insert(knot, 1)
            .unwrap()
            .elevate(source.degree + 1)
            .unwrap();
        // Compare in the middle of a span of the original that the new knot
        // does not split: there the derivatives are two-sided for both.
        let mut breaks = source.knots.clone();
        breaks.push(knot);
        breaks.sort_by(f64::total_cmp);
        let widest = breaks
            .windows(2)
            .max_by(|x, y| (x[1] - x[0]).total_cmp(&(y[1] - y[0])))
            .unwrap();
        let u = 0.5 * (widest[0] + widest[1]);
        let order = source.degree.min(3);
        let before = source.derivatives(u, order).unwrap();
        let after = changed.derivatives(u, order).unwrap();
        for (k, (x, y)) in before.iter().zip(&after).enumerate() {
            let scale = 1. + x.iter().map(|v| v.abs()).fold(0., f64::max);
            for (p, q) in x.iter().zip(y) {
                assert!(
                    (p - q).abs() <= 1e-7 * scale,
                    "order {k} at {u}: {x:?} vs {y:?}"
                );
            }
        }
    }
}

#[test]
fn length_is_additive_over_a_split_and_invariant_under_exact_edits() {
    let mut rng = Lcg(0x1E46);
    for _ in 0..10 {
        let source = curve(&mut rng);
        let [a, b] = source.domain();
        let whole = length(&source, 1e-5, 20_000).unwrap();
        if !whole.within_tolerance {
            continue;
        }
        let cut = rng.range(a + 0.2 * (b - a), b - 0.2 * (b - a));
        let [left, right] = source.split(cut).unwrap();
        let (left, right) = (
            length(&left, 1e-5, 20_000).unwrap(),
            length(&right, 1e-5, 20_000).unwrap(),
        );
        if left.within_tolerance && right.within_tolerance {
            let slack = whole.error_upper + left.error_upper + right.error_upper + 1e-9;
            assert!(
                (whole.value - left.value - right.value).abs() <= slack,
                "{} vs {} + {}",
                whole.value,
                left.value,
                right.value
            );
        }
        let reversed = length(&source.reverse().unwrap(), 1e-5, 20_000).unwrap();
        if reversed.within_tolerance {
            assert!(
                (whole.value - reversed.value).abs() <= whole.error_upper + reversed.error_upper + 1e-9
            );
        }
        // The reported bounds enclose the value and the chord.
        assert!(whole.bounds[0] <= whole.value && whole.value <= whole.bounds[1]);
        let chord = point(&source, a)
            .iter()
            .zip(point(&source, b))
            .map(|(p, q)| (p - q) * (p - q))
            .sum::<f64>()
            .sqrt();
        assert!(chord <= whole.bounds[1] + 1e-9);
    }
}

#[test]
fn point_inversion_recovers_points_of_the_curve() {
    let mut rng = Lcg(0x1247);
    let options = PointInversionOptions {
        samples: 256,
        max_iterations: 64,
        ..PointInversionOptions::default()
    };
    let (mut converged, mut total) = (0, 0);
    for _ in 0..120 {
        let source = curve(&mut rng);
        let [a, b] = source.domain();
        for _ in 0..4 {
            let u = rng.range(a, b);
            let target = point(&source, u);
            let report = invert_point(&source, &target, options).unwrap();
            total += 1;
            // Whatever parameter comes back, a converged report must name a
            // point of the curve that coincides with the query: the curve may
            // pass through the same point twice.
            if report.converged {
                converged += 1;
                let found = point(&source, report.parameter);
                let distance = found
                    .iter()
                    .zip(&target)
                    .map(|(p, q)| (p - q) * (p - q))
                    .sum::<f64>()
                    .sqrt();
                assert!(distance <= 1e-6, "inverted to {distance} away at {u}");
                assert!((a..=b).contains(&report.parameter));
            }
        }
    }
    // A query on the curve is the easy case. Before coincidence was judged
    // against rounding error, one such query in five ran out of iterations.
    assert!(converged * 100 >= total * 99, "{converged} of {total} converged");
}
