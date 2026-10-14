//! Exact binary64 knot insertion. A rounded or out-of-range operation refuses
//! the identity proof; floating decomposition alone never supplies identity.
use crate::curve::Curve;
fn dyadic(x: f64) -> Option<(i128, i32)> {
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let raw = ((bits >> 52) & 2047) as i32;
    let mut m = (bits & ((1u64 << 52) - 1)) as i128;
    let mut e = if raw == 0 {
        -1074
    } else {
        m += 1i128 << 52;
        raw - 1023 - 52
    };
    if bits >> 63 != 0 {
        m = -m;
    }
    if m == 0 {
        return Some((0, 0));
    }
    while m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    Some((m, e))
}
fn normalized(mut m: i128, mut e: i32) -> (i128, i32) {
    if m == 0 {
        return (0, 0);
    }
    while m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    (m, e)
}
pub(crate) fn exact_add(a: f64, b: f64) -> Option<f64> {
    let (am, ae) = dyadic(a)?;
    let (bm, be) = dyadic(b)?;
    if am == 0 {
        return Some(b);
    }
    if bm == 0 {
        return Some(a);
    }
    let e = ae.min(be);
    let shift = |m: i128, s: i32| {
        if s <= 126 {
            m.checked_mul(1i128 << s)
        } else {
            None
        }
    };
    let exact = normalized(shift(am, ae - e)?.checked_add(shift(bm, be - e)?)?, e);
    let result = a + b;
    (dyadic(result)? == exact).then_some(result)
}
pub(crate) fn exact_mul(a: f64, b: f64) -> Option<f64> {
    let (am, ae) = dyadic(a)?;
    let (bm, be) = dyadic(b)?;
    let result = a * b;
    (dyadic(result)? == normalized(am.checked_mul(bm)?, ae + be)).then_some(result)
}
fn exact_div(a: f64, b: f64) -> Option<f64> {
    if b == 0. {
        return None;
    }
    let result = a / b;
    let (rm, re) = dyadic(result)?;
    let (bm, be) = dyadic(b)?;
    (normalized(rm.checked_mul(bm)?, re + be) == dyadic(a)?).then_some(result)
}
/// An exact invertible change of units for a topology-only contour proof.
/// Every retained coordinate must equal the original coordinate times scale;
/// rounded multiplication is rejected rather than treated as correspondence.
pub(crate) fn scaled_curve(c: &Curve, scale: f64) -> Option<Curve> {
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let mut out = c.clone();
    out.control_points = c
        .control_points
        .iter()
        .map(|p| {
            p.iter()
                .map(|x| exact_mul(*x, scale))
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()?;
    out.validate().ok()?;
    Some(out)
}
/// Exact active seam identity from matching rational basis stencils. This
/// does not materialize rounded Bernstein poles, and proves no other topology.
pub(crate) fn translated_active_seam_closed(c: &Curve) -> bool {
    if c.validate().is_err() || c.degree == 0 {
        return false;
    }
    let p = c.degree;
    let n = c.control_points.len();
    let [a, b] = c.domain();
    // Strict neighboring knots make the endpoint zero-basis convention
    // identical on the two one-sided active spans.
    if !(c.knots[p - 1] < a && a < c.knots[p + 1] && c.knots[n - 1] < b && b < c.knots[n + 1]) {
        return false;
    }
    let Some(period) = exact_add(b, -a) else {
        return false;
    };
    let shift = n - p;
    (0..=2 * p).all(|j| exact_add(c.knots[j], period) == Some(c.knots[j + shift]))
        && (0..p).all(|j| {
            c.control_points[j] == c.control_points[j + shift]
                && c.weights[j] == c.weights[j + shift]
        })
}
pub(crate) fn inspect(c: &Curve) -> Option<Vec<Curve>> {
    c.validate().ok()?;
    let p = c.degree;
    let n = c.control_points.len();
    let [a, b] = c.domain();
    if p == 0 {
        return None;
    }
    if c.periodic {
        let pieces = exact_blossom_segments(c)?;
        // Periodic input validation permits rounded exterior knot translations.
        // Certify the actual active-domain seam, not the periodic flag alone.
        if pieces.first()?.control_points.first()? != pieces.last()?.control_points.last()? {
            return None;
        }
        return Some(pieces);
    }
    if c.knots[..=p].iter().any(|k| *k != a) || c.knots[n..].iter().any(|k| *k != b) {
        return exact_blossom_segments(c);
    }
    let mut knots = c.knots.clone();
    let mut controls = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(point, w)| {
            let mut h = point
                .iter()
                .map(|v| exact_mul(*v, *w))
                .collect::<Option<Vec<_>>>()?;
            h.push(*w);
            Some(h)
        })
        .collect::<Option<Vec<_>>>()?;
    let mut breaks = c
        .knots
        .iter()
        .copied()
        .filter(|k| *k > a && *k < b)
        .collect::<Vec<_>>();
    breaks.dedup();
    for u in breaks {
        while knots.iter().filter(|k| **k == u).count() < p {
            if controls.len() >= 4096 {
                return None;
            }
            let s = knots.iter().filter(|k| **k == u).count();
            let k = knots.iter().rposition(|v| *v <= u)?;
            let n = controls.len() - 1;
            let mut next = vec![Vec::new(); n + 2];
            next[..=k - p].clone_from_slice(&controls[..=k - p]);
            next[k - s + 1..n + 2].clone_from_slice(&controls[k - s..n + 1]);
            for i in k - p + 1..=k - s {
                let denominator = exact_add(knots[i + p], -knots[i])?;
                if denominator <= 0. {
                    return None;
                }
                let alpha = exact_div(exact_add(u, -knots[i])?, denominator)?;
                let beta = exact_add(1., -alpha)?;
                next[i] = controls[i]
                    .iter()
                    .zip(&controls[i - 1])
                    .map(|(x, y)| exact_add(exact_mul(alpha, *x)?, exact_mul(beta, *y)?))
                    .collect::<Option<Vec<_>>>()?;
            }
            knots.insert(k + 1, u);
            controls = next;
        }
    }
    let mut pieces = Vec::new();
    for i in p..controls.len() {
        if knots[i] >= knots[i + 1] {
            continue;
        }
        let homogeneous = &controls[i - p..=i];
        let weights = homogeneous
            .iter()
            .map(|h| *h.last().unwrap())
            .collect::<Vec<_>>();
        let points = homogeneous
            .iter()
            .map(|h| {
                h[..h.len() - 1]
                    .iter()
                    .map(|v| exact_div(*v, *h.last().unwrap()))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()?;
        pieces.push(Curve {
            degree: p,
            knots: std::iter::repeat_n(knots[i], p + 1)
                .chain(std::iter::repeat_n(knots[i + 1], p + 1))
                .collect(),
            control_points: points,
            weights,
            periodic: false,
        });
    }
    Some(pieces)
}
/// Endpoint clamping need not be representable as a global knot insertion.
/// Compute each active span's Bernstein controls by exact blossom evaluation.
/// Every operation must agree with its checked dyadic arithmetic identity.
fn exact_blossom_segments(c: &Curve) -> Option<Vec<Curve>> {
    let p = c.degree;
    let spans = crate::certificates::audit::curve_span_domains(c);
    if spans.len().checked_mul(p + 1)? > 4096 {
        return None;
    }
    let mut pieces = Vec::with_capacity(spans.len());
    for (span, [a, b]) in spans {
        let initial = c.control_points[span - p..=span]
            .iter()
            .zip(&c.weights[span - p..=span])
            .map(|(point, weight)| {
                let mut h = point
                    .iter()
                    .map(|v| exact_mul(*v, *weight))
                    .collect::<Option<Vec<_>>>()?;
                h.push(*weight);
                Some(h)
            })
            .collect::<Option<Vec<_>>>()?;
        let mut points = Vec::with_capacity(p + 1);
        let mut weights = Vec::with_capacity(p + 1);
        for k in 0..=p {
            let mut q = initial.clone();
            for r in 1..=p {
                let parameter = if r - 1 < p - k { a } else { b };
                for j in (r..=p).rev() {
                    let i = span - p + j;
                    let denominator = exact_add(c.knots[i + p - r + 1], -c.knots[i])?;
                    if denominator <= 0. {
                        return None;
                    }
                    let alpha = exact_div(exact_add(parameter, -c.knots[i])?, denominator)?;
                    if !(0. ..=1.).contains(&alpha) {
                        return None;
                    }
                    let beta = exact_add(1., -alpha)?;
                    q[j] = q[j]
                        .iter()
                        .zip(&q[j - 1])
                        .map(|(x, y)| exact_add(exact_mul(alpha, *x)?, exact_mul(beta, *y)?))
                        .collect::<Option<Vec<_>>>()?;
                }
            }
            let weight = *q[p].last()?;
            points.push(
                q[p][..q[p].len() - 1]
                    .iter()
                    .map(|v| exact_div(*v, weight))
                    .collect::<Option<Vec<_>>>()?,
            );
            weights.push(weight);
        }
        let piece = Curve {
            degree: p,
            knots: std::iter::repeat_n(a, p + 1)
                .chain(std::iter::repeat_n(b, p + 1))
                .collect(),
            control_points: points,
            weights,
            periodic: false,
        };
        piece.validate().ok()?;
        pieces.push(piece);
    }
    Some(pieces)
}
#[cfg(test)]
mod tests {
    #[test]
    fn translated_seam_identity_covers_degrees_and_rational_weights() {
        for degree in 1..=6 {
            let n = 2 * degree + 2;
            let shift = n - degree;
            let mut c = Curve {
                degree,
                knots: (0..n + degree + 1).map(|k| k as f64).collect(),
                control_points: (0..n)
                    .map(|j| vec![j as f64 / 7., (j * j) as f64 / 11., 0.])
                    .collect(),
                weights: (0..n).map(|j| (j + 1) as f64).collect(),
                periodic: false,
            };
            for j in 0..degree {
                c.control_points[j + shift] = c.control_points[j].clone();
                c.weights[j + shift] = c.weights[j];
            }
            assert!(translated_active_seam_closed(&c));
            let [a, b] = c.domain();
            let left = c.evaluate(a).unwrap().point;
            let right = c.evaluate(b).unwrap().point;
            // Numerical evaluations are only independent falsification checks.
            assert!(left.iter().zip(right).all(|(x, y)| (x - y).abs() < 1e-12));
            let mut bad = c.clone();
            bad.control_points[shift][0] = f64::from_bits(0f64.to_bits() + 1);
            assert!(!translated_active_seam_closed(&bad));
            let mut bad = c.clone();
            bad.weights[shift] = 2.;
            assert!(!translated_active_seam_closed(&bad));
            let mut bad = c.clone();
            bad.knots[1] = 1f64.next_up();
            assert!(!translated_active_seam_closed(&bad));
        }
    }
    use super::*;
    #[test]
    fn unclamped_exact_blossoms_certify_hollow_original_domains() {
        let c = Curve {
            degree: 2,
            knots: (0..9).map(|i| i as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![1., 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let before = c.clone();
        let pieces = inspect(&c).unwrap();
        assert_eq!(pieces.len(), 4);
        for (span, piece) in (2..6).zip(pieces) {
            assert_eq!(piece.domain(), [span as f64, (span + 1) as f64]);
            let bound =
                crate::curve_decomposition_certificate::inspect(&c, span, &piece, 9).unwrap();
            assert!(bound.error_upper.unwrap() < 1e-10, "{bound:?}");
        }
        let mut hole = c.clone();
        for point in &mut hole.control_points {
            for x in point {
                *x *= 0.25;
            }
        }
        let report =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()], vec![hole]], 1e-6, 10000, 10000)
                .unwrap();
        assert!(report.cap_domain_certified, "{report:?}");
        let scale = Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let twist = Curve {
            control_points: vec![vec![0., 0., 0.]; 2],
            ..scale.clone()
        };
        let points = [[0., 0., 0.], [0., 0., 10.]];
        let profiles = [c.clone()];
        let sweep = crate::sweeps::progressive_miter::Sweep::new(
            &profiles,
            &points,
            &scale,
            &twist,
            crate::sweeps::progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.001,
            },
        )
        .unwrap();
        let sections = sweep.sections_at(1).unwrap();
        // Zero analysis budgets isolate the exact ownership gate, without
        // claiming wall injectivity or intersection classification.
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(&sections, &[1], 0., 1e-6, 0, 0, 0)
                .is_ok()
        );
        let mut open = sections.clone();
        open[1][0].control_points[5][0] += 0.125;
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(&open, &[1], 0., 1e-6, 0, 0, 0)
                .is_err()
        );
        let mut rounded = c.clone();
        // The endpoint's weighted mean now contains 1/3, not a dyadic.
        rounded.weights[1] = 2.;
        assert!(inspect(&rounded).is_none());
        assert!(translated_active_seam_closed(&rounded) == false);
        let mut nonrepresentable = c.clone();
        nonrepresentable.control_points[0][0] = 1.;
        nonrepresentable.control_points[1][0] = f64::from_bits(1f64.to_bits() + 1);
        nonrepresentable.control_points[4] = nonrepresentable.control_points[0].clone();
        nonrepresentable.control_points[5] = nonrepresentable.control_points[1].clone();
        assert!(inspect(&nonrepresentable).is_none());
        assert!(translated_active_seam_closed(&nonrepresentable));
        let mut nonrepresentable_sections = sections.clone();
        nonrepresentable_sections[1][0] = nonrepresentable.clone();
        assert!(
            sweep
                .inspect_wall_geometry_with_loops(
                    &nonrepresentable_sections,
                    &[1],
                    0.,
                    1e-6,
                    0,
                    0,
                    0
                )
                .is_ok()
        );

        let mut bad_knots = nonrepresentable.clone();
        bad_knots.knots[1] = 1f64.next_up();
        assert!(!translated_active_seam_closed(&bad_knots));
        let mut bad_weight = nonrepresentable.clone();
        bad_weight.weights[5] = 2.;
        assert!(!translated_active_seam_closed(&bad_weight));

        let mut periodic = c.clone();
        periodic.periodic = true;
        let periodic_pieces = inspect(&periodic).unwrap();
        assert_eq!(periodic_pieces.len(), 4);
        for (span, piece) in (2..6).zip(periodic_pieces) {
            let bound = crate::curve_decomposition_certificate::inspect(&periodic, span, &piece, 9)
                .unwrap();
            assert!(bound.error_upper.unwrap() < 1e-10, "{bound:?}");
            assert!(
                crate::curve_decomposition_certificate::inspect(&periodic, span, &piece, 8)
                    .unwrap()
                    .error_upper
                    .is_none()
            );
            let mut shifted = piece.clone();
            for point in &mut shifted.control_points {
                point[2] += 0.125;
            }
            assert!(
                crate::curve_decomposition_certificate::inspect(&periodic, span, &shifted, 9)
                    .unwrap()
                    .error_upper
                    .unwrap()
                    >= 0.125
            );
        }
        let mut hole = periodic.clone();
        for point in &mut hole.control_points {
            for x in point {
                *x *= 0.25;
            }
        }
        let domain = crate::sweep_contour_audit::inspect(
            &[vec![periodic.clone()], vec![hole]],
            1e-6,
            10000,
            10000,
        )
        .unwrap();
        assert!(domain.cap_domain_certified, "{domain:?}");
        let mut almost_periodic = periodic.clone();
        almost_periodic.knots[1] = 1f64.next_up();
        almost_periodic.validate().unwrap();
        assert!(inspect(&almost_periodic).is_none());
        // A periodic flag is not evidence of active-domain endpoint ownership.
        // Isolate that premise using zero downstream analysis budgets.
        for (profile, expected_closed) in [(periodic, true), (almost_periodic, false)] {
            let profiles = [profile];
            let sweep = crate::sweeps::progressive_miter::Sweep::new(
                &profiles,
                &points,
                &scale,
                &twist,
                crate::sweeps::progressive_miter::Options {
                    normal: [1., 0., 0.],
                    closed: false,
                    miter_limit: 4.,
                    initial_steps: 1,
                    max_steps: 1,
                    max_deviation: 0.001,
                },
            )
            .unwrap();
            let sections = sweep.sections_at(1).unwrap();
            let result = sweep.inspect_wall_geometry_with_loops(&sections, &[1], 0., 1e-6, 0, 0, 0);
            assert_eq!(result.is_ok(), expected_closed, "{result:?}");
        }
        assert_eq!(c, before);
    }
    #[test]
    fn refuses_rounded_arithmetic_and_certifies_exact_spans() {
        assert_eq!(exact_add(1., 2f64.powi(-54)), None);
        assert_eq!(exact_mul(f64::from_bits(1), 0.5), None);
        assert_eq!(exact_div(1., 3.), None);
        assert_eq!(exact_mul(f64::MAX, 2.), None);
        assert_eq!(exact_div(f64::from_bits(1), 2.), None);
        assert_eq!(exact_add(f64::MAX, f64::MAX), None);
        assert_eq!(exact_mul(f64::from_bits(1), 1.), Some(f64::from_bits(1)));
        assert_eq!(exact_div(f64::from_bits(1), 1.), Some(f64::from_bits(1)));
        assert_eq!(exact_add(-0., f64::from_bits(1)), Some(f64::from_bits(1)));
        assert_eq!(exact_add(1., -1.), Some(0.));
        let c = Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.25, 0.5, 0.75, 1., 1., 1.],
            control_points: vec![
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![-1., 1., 0.],
                vec![-1., -1., 0.],
                vec![1., -1., 0.],
                vec![1., 0., 0.],
            ],
            weights: vec![1.; 6],
            periodic: false,
        };
        let domain =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()]], 1e-6, 10000, 10000).unwrap();
        assert!(domain.cap_domain_certified, "{domain:?}");
        let mut hole = c.clone();
        for point in &mut hole.control_points {
            point[0] *= 0.25;
            point[1] *= 0.25;
        }
        let hollow =
            crate::sweep_contour_audit::inspect(&[vec![c.clone()], vec![hole]], 1e-6, 10000, 10000)
                .unwrap();
        assert!(hollow.cap_domain_certified, "{hollow:?}");
        let pieces = inspect(&c).unwrap();
        assert_eq!(pieces.len(), 4);
        for piece in pieces {
            let [a, b] = piece.domain();
            for t in [0., 0.25, 0.5, 0.75, 1.] {
                let u = a + (b - a) * t;
                let original = c.evaluate(u).unwrap().point;
                let retained = piece.evaluate(u).unwrap().point;
                assert!(
                    original
                        .iter()
                        .zip(retained)
                        .all(|(a, b)| (a - b).abs() < 1e-12)
                );
            }
        }
    }
}
