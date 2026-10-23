//! Complete original-chart radial/normal agreement after radius qualification.
use crate::sweep_support::interval_vec3::{cross, norm};
use crate::{check, interval_eval::Interval as I, moving_radius, Result};
use std::collections::VecDeque;
pub struct Limits {
    pub max_sine_squared: f64,
    pub cells: usize,
    pub surface_spans: usize,
    pub center_spans: usize,
    pub radial_work: u64,
}
pub struct Envelope {
    radius: moving_radius::Certificate,
    sine_squared: [f64; 2],
    tolerance: f64,
}
impl Envelope {
    pub fn radius(&self) -> &moving_radius::Certificate {
        &self.radius
    }
    pub fn sine_squared_bounds(&self) -> [f64; 2] {
        self.sine_squared
    }
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
}
pub struct Report {
    pub envelope: Option<Envelope>,
    pub cells: usize,
    pub surface_spans: usize,
    pub center_spans: usize,
    pub radial_work: u64,
    pub accepted_cells: usize,
    pub uncertain_uv: Option<[[f64; 2]; 2]>,
    pub reason: &'static str,
    pub angular_break_sine_squared: Option<[f64; 2]>,
}
fn squared_norm(v: [I; 3]) -> Result<I> {
    let n = norm(v)?;
    let x = n.mul(n)?;
    I::new(x.lo.max(0.), x.hi)
}
fn angular(normal: [I; 3], radial: [I; 3]) -> Result<Option<[f64; 2]>> {
    let scale = |v: [I; 3]| {
        v.iter()
            .map(|x| x.lo.abs().max(x.hi.abs()))
            .fold(0_f64, f64::max)
    };
    let ns = scale(normal);
    let rs = scale(radial);
    if ns == 0. || rs == 0. {
        return Ok(None);
    }
    let mut n = [I::point(0.); 3];
    let mut r = n;
    for k in 0..3 {
        n[k] = normal[k].div(I::point(ns))?;
        r[k] = radial[k].div(I::point(rs))?;
    }
    let denominator = squared_norm(n)?.mul(squared_norm(r)?)?;
    if denominator.lo <= 0. {
        return Ok(None);
    }
    let sine = squared_norm(cross(n, r)?)?.div(denominator)?;
    Ok(Some([sine.lo.max(0.), sine.hi.min(1.)]))
}
/// Fresh derivative and point bounds cover every original chart point and all
/// incident knot sides. Radius agreement alone cannot authorize this envelope.
/// Singular normals or zero radial vectors remain unresolved, including poles.
/// This is a tolerance-based radial normal relation, not support-face contact,
/// injectivity, wall thickness or admission of a closed fillet body.
pub fn qualify(radius: moving_radius::Certificate, limits: Limits) -> Result<Report> {
    check(
        limits.max_sine_squared.is_finite()
            && (0. ..1.).contains(&limits.max_sine_squared)
            && (1..=100000).contains(&limits.cells)
            && (1..=100000).contains(&limits.surface_spans)
            && (1..=100000).contains(&limits.center_spans)
            && (1..=100_000_000).contains(&limits.radial_work),
        "Choose finite angular tolerance and bounded original envelope work",
    )?;
    let s = radius.surface();
    let c = radius.centers();
    // The owned radius certificate validates the unchanged original definitions
    // and their C0 knot multiplicities, and only exposes immutable access.
    let domain = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let mut queue = VecDeque::from([domain]);
    let mut bounds = [1_f64, 0_f64];
    let mut out = Report {
        envelope: None,
        cells: 0,
        surface_spans: 0,
        center_spans: 0,
        radial_work: 0,
        accepted_cells: 0,
        uncertain_uv: None,
        reason: "moving-envelope-work-limit",
        angular_break_sine_squared: None,
    };
    // A rigorously enclosed point may disprove full-domain agreement, but
    // can never admit it. This avoids spending the whole budget subdividing
    // unrelated regions before reaching an already provable defect.
    let midpoint = domain.map(|d| d[0] * 0.5 + d[1] * 0.5);
    let point = midpoint.map(|t| [t, t]);
    out.uncertain_uv = Some(point);
    out.cells += 1;
    let count = (c.degree..c.control_points.len())
        .filter(|&i| {
            c.knots[i] < c.knots[i + 1]
                && c.knots[i] <= midpoint[0]
                && midpoint[0] <= c.knots[i + 1]
        })
        .count();
    if count > limits.center_spans {
        return Ok(out);
    }
    out.center_spans += count;
    let center = crate::interval_eval::evaluate_interval(c, I::point(midpoint[0]))?;
    let (jet, spans) = crate::normal_alignment::jet_bounds(s, point, limits.surface_spans)?;
    out.surface_spans += spans;
    if let Some(j) = jet {
        let mut radial = [I::point(0.); 3];
        for k in 0..3 {
            radial[k] = j.point[k].sub(center[k])?;
        }
        if let Some(b) = angular(cross(j.first[0], j.first[1])?, radial)? {
            if b[0] > limits.max_sine_squared {
                out.angular_break_sine_squared = Some(b);
                out.reason = "moving-envelope-angular-break";
                return Ok(out);
            }
        }
    } else {
        return Ok(out);
    }
    while let Some(uv) = queue.pop_front() {
        out.uncertain_uv = Some(uv);
        if out.cells == limits.cells
            || out.surface_spans == limits.surface_spans
            || out.center_spans == limits.center_spans
        {
            return Ok(out);
        }
        out.cells += 1;
        let (jet, spans) =
            crate::normal_alignment::jet_bounds(s, uv, limits.surface_spans - out.surface_spans)?;
        out.surface_spans += spans;
        let Some(j) = jet else { return Ok(out) };
        let (radial, work, center_spans) = moving_radius::radial_box(
            s,
            c,
            uv,
            limits.radial_work - out.radial_work,
            limits.center_spans - out.center_spans,
        )?;
        out.radial_work += work;
        out.center_spans += center_spans;
        let Some(radial) = radial else {
            out.reason = "moving-envelope-radial-bound-unresolved";
            return Ok(out);
        };
        if let Some(b) = angular(cross(j.first[0], j.first[1])?, radial)? {
            if b[1] <= limits.max_sine_squared {
                bounds[0] = bounds[0].min(b[0]);
                bounds[1] = bounds[1].max(b[1]);
                out.accepted_cells += 1;
                continue;
            }
            if b[0] > limits.max_sine_squared {
                out.angular_break_sine_squared = Some(b);
                out.reason = "moving-envelope-angular-break";
                return Ok(out);
            }
        }
        let axis = usize::from(
            (uv[1][1] - uv[1][0]) / (domain[1][1] - domain[1][0])
                > (uv[0][1] - uv[0][0]) / (domain[0][1] - domain[0][0]),
        );
        let mid = uv[axis][0] * 0.5 + uv[axis][1] * 0.5;
        if !(uv[axis][0] < mid && mid < uv[axis][1]) {
            out.reason = "moving-envelope-resolution-limit";
            return Ok(out);
        }
        let mut a = uv;
        let mut b = uv;
        a[axis][1] = mid;
        b[axis][0] = mid;
        queue.push_back(a);
        queue.push_back(b);
    }
    out.envelope = Some(Envelope {
        radius,
        sine_squared: bounds,
        tolerance: limits.max_sine_squared,
    });
    out.uncertain_uv = None;
    out.reason = "moving-envelope-qualified";
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{curve::Curve, surface::Surface};
    fn limits() -> Limits {
        Limits {
            max_sine_squared: 0.02,
            cells: 10000,
            surface_spans: 10000,
            center_spans: 10000,
            radial_work: 100_000_000,
        }
    }
    fn tube(variable: bool) -> moving_radius::Certificate {
        let s = Surface {
            degree_u: 1,
            degree_v: 2,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..2)
                .map(|u| {
                    let r = if variable { 1. + u as f64 } else { 1. };
                    vec![
                        vec![r, 0., u as f64],
                        vec![r, r, u as f64],
                        vec![0., r, u as f64],
                    ]
                })
                .collect(),
            weights: vec![vec![1., 0.5_f64.sqrt(), 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 1.]]).unwrap();
        let r = Curve::from_polyline(vec![vec![1., 0.], vec![if variable { 2. } else { 1. }, 0.]])
            .unwrap();
        moving_radius::qualify(&s, &c, &r, 1e-9, 100, 100_000_000)
            .unwrap()
            .certificate
            .unwrap()
    }
    #[test]
    fn full_original_chart_radial_normals_have_qualified_coverage() {
        let radius = tube(false);
        let source = radius.surface().clone();
        let center = radius.centers().clone();
        let report = qualify(radius, limits()).unwrap();
        assert!(
            report.envelope.is_some(),
            "{} {:?} cells {}",
            report.reason,
            report.uncertain_uv,
            report.cells
        );
        assert!(report.accepted_cells > 1);
        let envelope = report.envelope.unwrap();
        assert!(envelope.sine_squared_bounds()[1] <= 0.02);
        assert_eq!(envelope.radius().surface(), &source);
        assert_eq!(envelope.radius().centers(), &center);
    }
    #[test]
    fn radius_relation_does_not_hide_oblique_normals() {
        let report = qualify(tube(true), limits()).unwrap();
        assert!(report.envelope.is_none());
        assert_eq!(report.reason, "moving-envelope-angular-break");
        assert!(report.uncertain_uv.is_some());
    }
    #[test]
    fn partial_coverage_and_exhausted_budgets_cannot_admit_envelope() {
        for kind in 0..4 {
            let mut l = limits();
            match kind {
                0 => l.cells = 1,
                1 => l.surface_spans = 1,
                2 => l.center_spans = 1,
                _ => l.radial_work = 1,
            };
            let report = qualify(tube(false), l).unwrap();
            assert!(report.envelope.is_none());
            assert!(report.uncertain_uv.is_some());
            if kind < 3 {
                assert_eq!(report.reason, "moving-envelope-work-limit");
            }
        }
    }
    #[test]
    fn discontinuous_source_charts_cannot_enter_the_envelope_pipeline() {
        let proof = tube(false);
        let mut s = proof.surface().clone();
        s.knots_u = vec![0., 0., 0.5, 0.5, 1., 1.];
        s.control_points = [0., 0.5, 0.5, 1.]
            .into_iter()
            .map(|z| vec![vec![1., 0., z], vec![1., 1., z], vec![0., 1., z]])
            .collect();
        s.weights = vec![vec![1., 0.5_f64.sqrt(), 1.]; 4];
        assert!(moving_radius::qualify(
            &s,
            proof.centers(),
            proof.radius(),
            1e-9,
            100,
            100_000_000
        )
        .is_err());
    }
}
