//! Sufficient whole-chart exclusion of chords aligned with endpoint normals.
//! Original rational derivative bounds cover every nonempty knot rectangle.
use crate::{Result, check, distance_bounds::Interval as I, normal_alignment, surface::Surface};

pub struct Certificate<'a> {
    surface: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    cosine_squared_upper: f64,
}
impl<'a> Certificate<'a> {
    pub fn surface(&self) -> &'a Surface {
        self.surface
    }
    pub fn domain(&self) -> [[f64; 2]; 2] {
        self.domain
    }
    pub fn max_sine_squared(&self) -> f64 {
        self.max_sine_squared
    }
    pub fn cosine_squared_upper(&self) -> f64 {
        self.cosine_squared_upper
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub spans: usize,
    pub complete: bool,
    pub cosine_squared_upper: Option<f64>,
    pub reason: &'static str,
}
fn separated_square(x: I) -> Result<Option<I>> {
    let lower = if x.lo > 0. {
        x.lo
    } else if x.hi < 0. {
        -x.hi
    } else {
        return Ok(None);
    };
    let upper = x.lo.abs().max(x.hi.abs());
    Ok(Some(I::new(
        (lower * lower).next_down().max(0.),
        (upper * upper).next_up(),
    )?))
}
/// If J ranges in H, the chord is M*delta with M in H by integration along
/// the straight parameter segment. A uniformly nonsingular coordinate minor
/// gives |chord| >= |det(M)|/||M||F * |delta|. At either endpoint n.J=0, so
/// |n.chord| <= |n| * ||J-J(endpoint)||F * |delta|. The squared ratio below
/// bounds cosine to every endpoint normal. This suffices to exclude aligned
/// chords, including arbitrarily close pairs, without dividing by UV distance.
pub fn qualify<'a>(s: &'a Surface, max_sine_squared: f64, max_spans: usize) -> Result<Report<'a>> {
    s.validate()?;
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    qualify_rectangle(s, domain, max_sine_squared, max_spans)
}
/// Excludes all aligned self chords whose endpoints lie in this original
/// parameter rectangle. A local certificate does not cover the full chart.
pub fn qualify_rectangle<'a>(
    s: &'a Surface,
    domain: [[f64; 2]; 2],
    max_sine_squared: f64,
    max_spans: usize,
) -> Result<Report<'a>> {
    check(
        max_sine_squared.is_finite() && (0. ..1.).contains(&max_sine_squared),
        "Self chord exclusion requires squared sine in [0,1)",
    )?;
    s.validate()?;
    check(
        domain.iter().all(|a| a[0] < a[1]),
        "Self chord rectangle requires positive widths",
    )?;
    normal_alignment::validate_rectangle(s, domain, max_spans)?;
    let mut out = Report {
        certificate: None,
        spans: 0,
        complete: false,
        cosine_squared_upper: None,
        reason: "self-chord-unproven",
    };
    let degrees = [s.degree_u, s.degree_v];
    let counts = [s.control_points.len(), s.control_points[0].len()];
    let knots = [&s.knots_u, &s.knots_v];
    // Integration across a knot needs a continuous chart. Repeated internal
    // knots beyond degree permit jumps; never infer continuity from samples.
    for axis in 0..2 {
        if degrees[axis] == 0 {
            out.reason = "self-chord-continuity-unproven";
            return Ok(out);
        }
        let mut i = degrees[axis] + 1;
        while i < counts[axis] {
            let knot = knots[axis][i];
            let mut end = i + 1;
            while end < knots[axis].len() && knots[axis][end] == knot {
                end += 1
            }
            if knot > domain[axis][0] && knot < domain[axis][1] && end - i > degrees[axis] {
                out.reason = "self-chord-continuity-unproven";
                return Ok(out);
            }
            i = end;
        }
    }
    let mut hull = [[[f64::INFINITY, f64::NEG_INFINITY]; 2]; 3];
    for u in degrees[0]..counts[0] {
        for v in degrees[1]..counts[1] {
            if knots[0][u] == knots[0][u + 1] || knots[1][v] == knots[1][v + 1] {
                continue;
            }
            let section = [
                [
                    knots[0][u].max(domain[0][0]),
                    knots[0][u + 1].min(domain[0][1]),
                ],
                [
                    knots[1][v].max(domain[1][0]),
                    knots[1][v + 1].min(domain[1][1]),
                ],
            ];
            if section.iter().any(|d| d[0] > d[1]) {
                continue;
            }
            if out.spans == max_spans {
                out.reason = "self-chord-span-limit";
                return Ok(out);
            }
            let j = normal_alignment::jacobian_on(s, [u, v], section)?;
            out.spans += 1;
            for k in 0..3 {
                for a in 0..2 {
                    hull[k][a][0] = hull[k][a][0].min(j[k][a].lo);
                    hull[k][a][1] = hull[k][a][1].max(j[k][a].hi);
                }
            }
        }
    }
    out.complete = true;
    let mut j = [[I::point(0.); 2]; 3];
    let mut variation = I::point(0.);
    for k in 0..3 {
        for a in 0..2 {
            j[k][a] = I::new(hull[k][a][0], hull[k][a][1])?;
            let width = I::point(j[k][a].hi).sub(I::point(j[k][a].lo))?;
            variation = variation.add(width.mul(width)?)?;
        }
    }
    let mut best = 1_f64;
    for [a, b] in [[0, 1], [0, 2], [1, 2]] {
        let det = j[a][0].mul(j[b][1])?.sub(j[a][1].mul(j[b][0])?)?;
        let Some(det_squared) = separated_square(det)? else {
            continue;
        };
        if det_squared.lo <= 0. {
            continue;
        }
        let mut norm = I::point(0.);
        for axis in [a, b] {
            for direction in 0..2 {
                let upper = j[axis][direction].lo.abs().max(j[axis][direction].hi.abs());
                norm = norm.add(I::point(upper).mul(I::point(upper))?)?;
            }
        }
        let ratio = variation.mul(norm)?.div(det_squared)?;
        best = best.min(ratio.hi);
    }
    out.cosine_squared_upper = Some(best);
    let threshold = I::point(1.).sub(I::point(max_sine_squared))?;
    if best < threshold.lo {
        out.certificate = Some(Certificate {
            surface: s,
            domain,
            max_sine_squared,
            cosine_squared_upper: best,
        });
        out.reason = "self-chord-normal-excluded";
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn graph(height: f64) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|v| vec![i as f64 / 2., v as f64, if i == 2 { height } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn whole_curved_graph_excludes_aligned_chords_and_retains_source_identity() {
        let s = graph(0.01);
        let r = qualify(&s, 1e-6, 1).unwrap();
        assert!(r.complete);
        assert_eq!(r.spans, 1);
        let c = r.certificate.expect("small curvature full chart proof");
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.max_sine_squared(), 1e-6);
        assert!(c.cosine_squared_upper() < 0.001);
        let strong = graph(2.);
        assert!(qualify(&strong, 1e-6, 1).unwrap().certificate.is_none());
        let mut collapsed = s.clone();
        for row in &mut collapsed.control_points {
            for p in row {
                p[1] = 0.
            }
        }
        assert!(qualify(&collapsed, 1e-6, 1).unwrap().certificate.is_none());
        assert!(qualify(&s, 1., 1).is_err());
        assert!(qualify(&s, 1e-6, 0).is_err());
    }
    #[test]
    fn missing_knot_span_or_discontinuous_chart_never_excludes() {
        let mut s = graph(0.01);
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let r = qualify(&s, 1e-6, 1).unwrap();
        assert!(!r.complete);
        assert_eq!(r.spans, 1);
        assert!(r.certificate.is_none());
        assert!(qualify(&s, 1e-6, 2).unwrap().certificate.is_some());
        s.control_points.insert(2, s.control_points[1].clone());
        s.weights.insert(2, vec![1.; 2]);
        s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        assert!(qualify(&s, 1e-6, 2).is_err());
    }
    #[test]
    fn independent_parabola_chords_and_reversed_coordinates_stay_inside_bound() {
        let mut s = graph(0.01);
        for sign in [1., -1.] {
            for row in &mut s.control_points {
                for p in row {
                    p[0] = sign * p[0].abs();
                }
            }
            let proof = qualify(&s, 1e-6, 1).unwrap().certificate.unwrap();
            for a in 0..16 {
                for b in 0..16 {
                    if a == b {
                        continue;
                    }
                    let u = a as f64 / 15.;
                    let v = b as f64 / 15.;
                    // Independent analytic S(u,t)=(sign*u,t,0.01*u^2).
                    let dx = sign * (v - u);
                    let dz = 0.01 * (v * v - u * u);
                    let nx = -sign * 0.02 * u;
                    let cosine2 = (nx * dx + dz).powi(2) / ((nx * nx + 1.) * (dx * dx + dz * dz));
                    assert!(cosine2 <= proof.cosine_squared_upper());
                }
            }
        }
    }
    #[test]
    fn original_rational_weights_have_continuous_exclusion_with_analytic_oracle() {
        let mut s = graph(0.01);
        let w = 1.001;
        s.weights[2] = vec![w; 2];
        let proof = qualify(&s, 1e-6, 1)
            .unwrap()
            .certificate
            .expect("mild rational graph");
        let point = |u: f64| {
            let d = 1. + (w - 1.) * u * u;
            [(u + (w - 1.) * u * u) / d, w * 0.01 * u * u / d]
        };
        for a in 0..16 {
            for b in 0..16 {
                if a == b {
                    continue;
                }
                let u = a as f64 / 15.;
                let v = b as f64 / 15.;
                let d = 1. + (w - 1.) * u * u;
                let dxdu = ((1. + 2. * (w - 1.) * u) * d
                    - (u + (w - 1.) * u * u) * 2. * (w - 1.) * u)
                    / (d * d);
                let dzdu = 2. * w * 0.01 * u / (d * d);
                let pa = point(u);
                let pb = point(v);
                let dx = pb[0] - pa[0];
                let dz = pb[1] - pa[1];
                let cosine2 = (-dzdu * dx + dxdu * dz).powi(2)
                    / ((dxdu * dxdu + dzdu * dzdu) * (dx * dx + dz * dz));
                assert!(cosine2 <= proof.cosine_squared_upper());
            }
        }
    }
    #[test]
    fn local_certificate_retains_its_rectangle_and_does_not_prove_full_chart() {
        let s = graph(2.);
        let domain = [[0.2, 0.21], [0.3, 0.8]];
        assert!(qualify(&s, 1e-6, 1).unwrap().certificate.is_none());
        let c = qualify_rectangle(&s, domain, 1e-6, 1)
            .unwrap()
            .certificate
            .unwrap();
        assert_eq!(c.domain(), domain);
        assert!(std::ptr::eq(c.surface(), &s));
        assert!(c.cosine_squared_upper() < 0.01);
        assert!(qualify_rectangle(&s, [[0.2, 0.2], [0., 1.]], 1e-6, 1).is_err());
        assert!(qualify_rectangle(&s, [[-0.1, 0.2], [0., 1.]], 1e-6, 1).is_err());
    }
    #[test]
    fn local_rectangle_spends_only_intersecting_original_spans() {
        let mut s = graph(0.01);
        s.degree_u = 1;
        s.knots_u = vec![0., 0., 0.5, 1., 1.];
        let domain = [[0.6, 0.8], [0., 1.]];
        let r = qualify_rectangle(&s, domain, 1e-6, 1).unwrap();
        assert!(r.complete);
        assert_eq!(r.spans, 1);
        assert!(r.certificate.is_some());
        let r = qualify_rectangle(&s, [[0.4, 0.6], [0., 1.]], 1e-6, 1).unwrap();
        assert!(!r.complete);
        assert!(r.certificate.is_none());
    }
}
