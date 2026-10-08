//! Complete adaptive coverage of original same-surface endpoint pairs.
use crate::{
    check, distance_bounds::Interval as I, normal_alignment, surface::Surface, surface_distance,
    surface_self_chord, Result,
};
type Rectangle = [[f64; 2]; 2];
type Pair = [Rectangle; 2];
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
}
pub struct Certificate<'a> {
    surface: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    intrinsic: Vec<surface_self_chord::Certificate<'a>>,
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn minimum_mm(&self) -> f64 {
        self.minimum_mm
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn intrinsic_certificates(&self) -> &[surface_self_chord::Certificate<'a>] {
        &self.intrinsic
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub cells: usize,
    pub spans: usize,
    pub intrinsic_exclusions: usize,
    pub polar_exclusions: usize,
    pub coordinate_path_exclusions: usize,
    pub coordinate_path_attempts: usize,
    pub coordinate_path_refusals: usize,
    pub local_coordinate_path_exclusions: usize,
    pub local_coordinate_path_attempts: usize,
    pub local_coordinate_path_refusals: usize,
    pub polar_factor_refusals: usize,
    pub angular_exclusions: usize,
    pub factored_angular_exclusions: usize,
    pub direction_exclusions: usize,
    pub distance_exclusions: usize,
    pub pending: usize,
    pub uncertain: Option<Pair>,
    pub reason: &'static str,
}
fn span_count(s: &Surface, domain: Rectangle) -> usize {
    let count = |degree: usize, knots: &[f64], n: usize, d: [f64; 2]| {
        (degree..n)
            .filter(|&i| knots[i] < knots[i + 1] && knots[i] <= d[1] && knots[i + 1] >= d[0])
            .count()
    };
    count(s.degree_u, &s.knots_u, s.control_points.len(), domain[0])
        * count(s.degree_v, &s.knots_v, s.control_points[0].len(), domain[1])
}
fn squared_norm(v: [I; 3]) -> Result<I> {
    let mut sum = I::point(0.);
    for x in v {
        let lo = if x.lo > 0. {
            x.lo
        } else if x.hi < 0. {
            -x.hi
        } else {
            0.
        };
        let hi = x.lo.abs().max(x.hi.abs());
        sum = sum.add(I::new((lo * lo).next_down().max(0.), (hi * hi).next_up())?)?;
    }
    I::new(sum.lo.max(0.), sum.hi)
}
/// Original position boxes enclose every secant; original normal boxes enclose
/// both endpoint normals. One endpoint strictly oblique excludes the whole pair.
fn secant_exclusion(
    s: &Surface,
    pair: Pair,
    boxes: [&Vec<[f64; 2]>; 2],
    sine: f64,
    max_spans: usize,
    factored: Option<&crate::factored_surface_normals::Certificate<'_>>,
) -> Result<(bool, usize)> {
    let mut d = [I::point(0.); 3];
    for k in 0..3 {
        d[k] =
            I::new(boxes[1][k][0], boxes[1][k][1])?.sub(I::new(boxes[0][k][0], boxes[0][k][1])?)?;
    }
    let scale = d
        .iter()
        .map(|x| x.lo.abs().max(x.hi.abs()))
        .fold(0_f64, f64::max);
    if scale < f64::MIN_POSITIVE {
        return Ok((false, 0));
    }
    for x in &mut d {
        *x = x.div(I::point(scale))?;
    }
    let length = squared_norm(d)?;
    if length.lo <= 0. {
        return Ok((false, 0));
    }
    let threshold = I::point(1.).sub(I::point(sine))?;
    let mut spans = 0;
    for rect in pair {
        if spans == max_spans {
            break;
        }
        let (bounds, used) = if let Some(proof) = factored {
            // A prepared fact covers one original Bezier span. Its exact
            // preparation budget is separate from this bounded coverage call.
            (Some(proof.bounds(rect)?), 1)
        } else {
            normal_alignment::normal_bounds(s, rect, max_spans - spans)?
        };
        spans += used;
        let Some(bounds) = bounds else {
            continue;
        };
        let scale = bounds
            .iter()
            .flatten()
            .map(|x| x.abs())
            .fold(0_f64, f64::max);
        if scale < f64::MIN_POSITIVE {
            continue;
        }
        let mut n = [I::point(0.); 3];
        for k in 0..3 {
            n[k] = I::new(bounds[k][0], bounds[k][1])?.div(I::point(scale))?;
        }
        let denominator = length.mul(squared_norm(n)?)?;
        if denominator.lo <= 0. {
            continue;
        }
        let mut dot = I::point(0.);
        for k in 0..3 {
            dot = dot.add(n[k].mul(d[k])?)?;
        }
        let upper = dot.lo.abs().max(dot.hi.abs());
        let numerator = I::point(upper).mul(I::point(upper))?;
        if numerator.div(denominator)?.hi < threshold.lo {
            return Ok((true, spans));
        }
    }
    Ok((false, spans))
}

/// Covers the Cartesian product of the full original UV chart with itself.
/// Every split preserves both closed child rectangles, including the diagonal.
/// A local intrinsic certificate covers both endpoints through their convex UV
/// hull. Otherwise original normal separation or original Cartesian distance
/// must exclude the pair. Missing work never constructs a global certificate.
pub fn qualify<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    qualify_impl(s, minimum_mm, max_sine_squared, limits, None, None)
}
/// Reuse only a sealed exact boundary-factor fact from this same immutable
/// source. Original exact preparation work is paid before coverage begins.
pub fn qualify_with_factored_normals<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    proof: &crate::factored_surface_normals::Certificate<'a>,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, proof.surface()),
        "Use factored normals from the same original surface",
    )?;
    qualify_impl(s, minimum_mm, max_sine_squared, limits, Some(proof), None)
}
/// Reuses exact differential and normal facts from the same source and pole.
pub fn qualify_with_rational_pole<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    normals: &crate::factored_surface_normals::Certificate<'a>,
    differential: &crate::rational_pole_differential::Certificate<'a>,
) -> Result<Report<'a>> {
    check(
        std::ptr::eq(s, normals.surface())
            && std::ptr::eq(s, differential.surface())
            && normals.end() == differential.end(),
        "Use pole facts from the same original surface and end",
    )?;
    qualify_impl(
        s,
        minimum_mm,
        max_sine_squared,
        limits,
        Some(normals),
        Some(differential),
    )
}
fn qualify_impl<'a>(
    s: &'a Surface,
    minimum_mm: f64,
    max_sine_squared: f64,
    limits: Limits,
    factored: Option<&crate::factored_surface_normals::Certificate<'a>>,
    differential: Option<&crate::rational_pole_differential::Certificate<'a>>,
) -> Result<Report<'a>> {
    s.validate()?;
    check(
        minimum_mm.is_finite()
            && minimum_mm > 0.
            && max_sine_squared.is_finite()
            && (0. ..1.).contains(&max_sine_squared)
            && (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.spans),
        "Bound same-face coverage work and choose positive threshold",
    )?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let necessary = if max_sine_squared < 0.5 {
        let sine = I::point(max_sine_squared);
        let upper = I::point(4.).mul(sine)?.mul(I::point(1.).sub(sine)?)?.hi;
        (upper < 1.).then_some(upper)
    } else {
        None
    };
    let mut out = Report {
        certificate: None,
        cells: 0,
        spans: 0,
        intrinsic_exclusions: 0,
        polar_exclusions: 0,
        coordinate_path_exclusions: 0,
        coordinate_path_attempts: 0,
        coordinate_path_refusals: 0,
        local_coordinate_path_exclusions: 0,
        local_coordinate_path_attempts: 0,
        local_coordinate_path_refusals: 0,
        polar_factor_refusals: 0,
        angular_exclusions: 0,
        factored_angular_exclusions: 0,
        direction_exclusions: 0,
        distance_exclusions: 0,
        pending: 1,
        uncertain: None,
        reason: "self-coverage-work-limit",
    };
    let mut pending = vec![[domain, domain]];
    let mut intrinsic = Vec::new();
    while let Some(pair) = pending.pop() {
        if out.cells == limits.cells || out.spans == limits.spans {
            pending.push(pair);
            break;
        }
        out.cells += 1;
        let union = std::array::from_fn(|axis| {
            [
                pair[0][axis][0].min(pair[1][axis][0]),
                pair[0][axis][1].max(pair[1][axis][1]),
            ]
        });
        let r = surface_self_chord::qualify_rectangle(
            s,
            union,
            max_sine_squared,
            limits.spans - out.spans,
        )?;
        out.spans += r.spans;
        if let Some(c) = r.certificate {
            out.intrinsic_exclusions += 1;
            intrinsic.push(c);
            continue;
        }
        if let Some(proof) = differential.filter(|_| out.spans < limits.spans) {
            let r = surface_self_chord::qualify_rational_polar_rectangle(
                s,
                union,
                proof,
                max_sine_squared,
                limits.spans - out.spans,
            )?;
            out.spans += r.spans;
            if let Some(c) = r.certificate {
                out.intrinsic_exclusions += 1;
                out.polar_exclusions += 1;
                intrinsic.push(c);
                continue;
            }
        }
        if let (Some(differential), Some(normals)) = (differential, factored) {
            if out.spans < limits.spans {
                let r = surface_self_chord::rational_pole_pair_report(
                    s,
                    pair,
                    differential,
                    normals,
                    max_sine_squared,
                    limits.spans - out.spans,
                )?;
                out.spans += r.spans;
                out.coordinate_path_attempts += usize::from(r.coordinate_path_attempted);
                out.coordinate_path_refusals +=
                    usize::from(r.coordinate_path_attempted && !r.coordinate_path_available);
                out.coordinate_path_exclusions += usize::from(r.coordinate_path_excluded);
                out.local_coordinate_path_exclusions += usize::from(r.local_path_excluded);
                out.local_coordinate_path_attempts += usize::from(r.local_path_attempted);
                out.local_coordinate_path_refusals +=
                    usize::from(r.local_path_attempted && !r.local_path_available);
                if r.excluded {
                    out.direction_exclusions += 1;
                    continue;
                }
            }
        }
        if let Some(proof) = factored.filter(|_| out.spans < limits.spans) {
            let r = surface_self_chord::qualify_polar_rectangle(
                s,
                union,
                proof.end(),
                max_sine_squared,
                limits.spans - out.spans,
            )?;
            out.spans += r.spans;
            if r.reason == "polar-chord-factor-unproven" {
                out.polar_factor_refusals += 1;
            }
            if let Some(c) = r.certificate {
                out.intrinsic_exclusions += 1;
                out.polar_exclusions += 1;
                intrinsic.push(c);
                continue;
            }
        }
        if let Some(threshold) = necessary.filter(|_| out.spans < limits.spans) {
            if let Some(proof) = factored {
                if limits.spans - out.spans >= 2 {
                    let components = [proof.bounds(pair[0])?, proof.bounds(pair[1])?];
                    out.spans += 2;
                    let r = normal_alignment::inspect_pair_bounds(components, threshold)?;
                    if r.aligned == Some(false) {
                        out.angular_exclusions += 1;
                        out.factored_angular_exclusions += 1;
                        continue;
                    }
                }
            } else {
                let r = normal_alignment::inspect_pair(
                    [s, s],
                    pair,
                    threshold,
                    limits.spans - out.spans,
                )?;
                out.spans += r.spans;
                if r.aligned == Some(false) {
                    out.angular_exclusions += 1;
                    continue;
                }
            }
        }
        let work = span_count(s, pair[0]) + span_count(s, pair[1]);
        if work <= limits.spans - out.spans {
            let a = surface_distance::rectangle_bounds(s, pair[0])?;
            let b = surface_distance::rectangle_bounds(s, pair[1])?;
            out.spans += work;
            if surface_distance::enclosure_distance(&a, &b)?.0 >= minimum_mm {
                out.distance_exclusions += 1;
                continue;
            }
            if out.spans < limits.spans {
                let (excluded, used) = secant_exclusion(
                    s,
                    pair,
                    [&a, &b],
                    max_sine_squared,
                    limits.spans - out.spans,
                    factored,
                )?;
                out.spans += used;
                if excluded {
                    out.direction_exclusions += 1;
                    continue;
                }
            }
        }
        if out.spans == limits.spans {
            pending.push(pair);
            break;
        }
        let mut selected = (0, 0);
        let mut largest = 0.;
        for side in 0..2 {
            for axis in 0..2 {
                let width = (pair[side][axis][1] - pair[side][axis][0])
                    / (domain[axis][1] - domain[axis][0]);
                if width > largest {
                    largest = width;
                    selected = (side, axis)
                }
            }
        }
        let (side, axis) = selected;
        let d = pair[side][axis];
        let mid = d[0] * 0.5 + d[1] * 0.5;
        if mid <= d[0] || mid >= d[1] {
            pending.push(pair);
            out.reason = "self-coverage-resolution-limit";
            break;
        }
        let mut left = pair;
        let mut right = pair;
        left[side][axis][1] = mid;
        right[side][axis][0] = mid;
        pending.push(right);
        pending.push(left);
    }
    out.pending = pending.len();
    out.uncertain = pending.last().copied();
    if pending.is_empty() {
        out.reason = "self-coverage-qualified";
        out.certificate = Some(Certificate {
            surface: s,
            minimum_mm,
            max_sine_squared,
            intrinsic,
        });
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph() -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { 2. } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn factored_original_pole_normals_exclude_oblique_secants_but_refuse_foreign_source() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.]; 2],
                vec![vec![0.5, 0., 0.]; 2],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let prepared = crate::factored_surface_normals::prepare(&s, false, 1_000_000).unwrap();
        let proof = prepared.certificate.unwrap();
        let pair = [[[0., 0.], [0., 1.]], [[0.5, 0.6], [0., 1.]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        assert!(
            !secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, None)
                .unwrap()
                .0
        );
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 1, Some(&proof)).unwrap();
        assert!(excluded && used == 1);
        let foreign = s.clone();
        assert!(qualify_with_factored_normals(
            &foreign,
            0.1,
            1e-6,
            Limits {
                cells: 10,
                spans: 100
            },
            &proof
        )
        .is_err());
        let r = qualify_with_factored_normals(&s, 0.1, 1e-6, Limits { cells: 1, spans: 1 }, &proof)
            .unwrap();
        assert!(r.certificate.is_none() && r.pending > 0 && r.spans <= 1);
    }
    #[test]
    fn original_secant_direction_excludes_oblique_chords_and_refuses_overlap_or_no_work() {
        let s = graph();
        let pair = [[[0., 0.01], [0., 0.01]], [[0.1, 0.11], [0., 0.01]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
        assert!(excluded);
        assert!(used > 0 && used <= 100);
        assert_eq!(
            secant_exclusion(&s, pair, [&a, &b], 1e-6, 0, None).unwrap(),
            (false, 0)
        );
        assert_eq!(
            secant_exclusion(&s, [pair[0]; 2], [&a, &a], 1e-6, 100, None).unwrap(),
            (false, 0)
        );
    }
    #[test]
    fn complete_subdivision_proves_strongly_curved_graph_and_missing_work_refuses() {
        let s = graph();
        assert!(surface_self_chord::qualify(&s, 1e-6, 1)
            .unwrap()
            .certificate
            .is_none());
        let r = qualify(
            &s,
            0.1,
            1e-6,
            Limits {
                cells: 10000,
                spans: 100000,
            },
        )
        .unwrap();
        eprintln!(
            "self adaptive {} cells={} spans={} intrinsic={} angular={} distance={} pending={}",
            r.reason,
            r.cells,
            r.spans,
            r.intrinsic_exclusions,
            r.angular_exclusions,
            r.distance_exclusions,
            r.pending
        );
        let c = r
            .certificate
            .expect("all original endpoint pairs must be covered");
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.minimum_mm(), 0.1);
        assert_eq!(c.max_sine_squared(), 1e-6);
        assert!(r.intrinsic_exclusions > 1);
        assert_eq!(r.pending, 0);
        assert!(r.uncertain.is_none());
        assert!(c
            .intrinsic_certificates()
            .iter()
            .all(|p| std::ptr::eq(p.surface(), &s)));
        for limits in [
            Limits {
                cells: 1,
                spans: 100000,
            },
            Limits {
                cells: 10000,
                spans: 1,
            },
        ] {
            let r = qualify(&s, 0.1, 1e-6, limits).unwrap();
            assert!(r.certificate.is_none());
            assert!(r.pending > 0);
            assert!(r.uncertain.is_some());
        }
    }
    #[test]
    fn rational_half_cylinder_with_short_exact_normal_chord_cannot_qualify() {
        let points = [[1., 0.], [1., 1.], [0., 1.], [-1., 1.], [-1., 0.]];
        let w = 0.5_f64.sqrt();
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: points
                .iter()
                .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 1.]])
                .collect(),
            weights: [1., w, 1., w, 1.].iter().map(|w| vec![*w; 2]).collect(),
            periodic_u: false,
            periodic_v: false,
        };
        // Original endpoints at U=0/1 have distance exactly 2 and normals
        // parallel to their connecting line, independent of rounded arc weight.
        let pair = [[[0., 0.001], [0.4, 0.401]], [[0.999, 1.], [0.4, 0.401]]];
        let a = surface_distance::rectangle_bounds(&s, pair[0]).unwrap();
        let b = surface_distance::rectangle_bounds(&s, pair[1]).unwrap();
        let (excluded, used) = secant_exclusion(&s, pair, [&a, &b], 1e-6, 100, None).unwrap();
        assert!(!excluded, "the exact diameter must remain a candidate");
        assert!(used > 0 && used <= 100);
        let r = qualify(
            &s,
            2.1,
            1e-6,
            Limits {
                cells: 2000,
                spans: 10000,
            },
        )
        .unwrap();
        eprintln!(
            "short original normal chord refusal {} cells={} spans={} pending={}",
            r.reason, r.cells, r.spans, r.pending
        );
        assert!(r.certificate.is_none());
        assert!(r.pending > 0);
        assert!(r.uncertain.is_some());
        assert!(r.cells <= 2000 && r.spans <= 10000);
        assert!(qualify(
            &s,
            0.,
            1e-6,
            Limits {
                cells: 2000,
                spans: 10000
            }
        )
        .is_err());
    }
}
