//! Complete adaptive coverage of original same-surface endpoint pairs.
use crate::{
    Result, check, distance_bounds::Interval as I, normal_alignment, surface::Surface,
    surface_distance, surface_self_chord,
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
    pub angular_exclusions: usize,
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
        angular_exclusions: 0,
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
        if let Some(threshold) = necessary.filter(|_| out.spans < limits.spans) {
            let r =
                normal_alignment::inspect_pair([s, s], pair, threshold, limits.spans - out.spans)?;
            out.spans += r.spans;
            if r.aligned == Some(false) {
                out.angular_exclusions += 1;
                continue;
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
    fn complete_subdivision_proves_strongly_curved_graph_and_missing_work_refuses() {
        let s = graph();
        assert!(
            surface_self_chord::qualify(&s, 1e-6, 1)
                .unwrap()
                .certificate
                .is_none()
        );
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
        assert!(
            c.intrinsic_certificates()
                .iter()
                .all(|p| std::ptr::eq(p.surface(), &s))
        );
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
        assert!(
            qualify(
                &s,
                0.,
                1e-6,
                Limits {
                    cells: 2000,
                    spans: 10000
                }
            )
            .is_err()
        );
    }
}
