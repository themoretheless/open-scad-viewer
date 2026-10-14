//! Spatial curve to UV: numerical proposals with full-interval lift admission.
use crate::{
    Result, check,
    curve::Curve,
    curve_surface_agreement::{self, Report as Agreement, Status},
    surface::Surface,
};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub tolerance: f64,
    pub max_segments: usize,
    pub max_iterations: usize,
    pub agreement_cells: usize,
}
#[derive(Clone, Debug)]
pub struct Report {
    pub pcurve: Option<Curve>,
    pub agreement: Option<Agreement>,
    pub agreement_cells: usize,
    pub evaluations: usize,
    pub within_tolerance: bool,
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
fn sub(a: &[f64], b: &[f64]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] - b[k])
}
fn invert(a: [f64; 3], b: [f64; 3], d: [f64; 3]) -> Option<[f64; 2]> {
    let (aa, ab, bb) = (dot(a, a), dot(a, b), dot(b, b));
    let det = aa * bb - ab * ab;
    if !det.is_finite() || det <= 0. {
        return None;
    }
    let p = [
        (bb * dot(a, d) - ab * dot(b, d)) / det,
        (aa * dot(b, d) - ab * dot(a, d)) / det,
    ];
    p.iter().all(|x| x.is_finite()).then_some(p)
}
fn domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn affine_candidate(c: &Curve, s: &Surface) -> Option<Curve> {
    if s.degree_u != 1
        || s.degree_v != 1
        || s.control_points.len() != 2
        || s.control_points[0].len() != 2
        || s.periodic_u
        || s.periodic_v
        || s.weights.iter().flatten().any(|w| *w != s.weights[0][0])
    {
        return None;
    }
    let o = &s.control_points[0][0];
    let a = sub(&s.control_points[1][0], o);
    let b = sub(&s.control_points[0][1], o);
    let domain = domains(s);
    let mut p = c.clone();
    p.control_points = c
        .control_points
        .iter()
        .map(|x| {
            let uv = invert(a, b, sub(x, o))?;
            Some(
                (0..2)
                    .map(|k| domain[k][0] + uv[k] * (domain[k][1] - domain[k][0]))
                    .collect(),
            )
        })
        .collect::<Option<Vec<_>>>()?;
    Some(p)
}
/// Find a UV representation, not a unique/nearest inverse. Affine candidates
/// retain degree, weights and knots. General candidates use bounded Newton
/// proposals and dyadic polylines. Every accepted pcurve passes the same full
/// continuous original-surface verifier; samples never certify the result.
/// Ambiguous projections, missing coverage or budget exhaustion remain unproven.
pub fn project(c: &Curve, s: &Surface, limits: Limits) -> Result<Report> {
    c.validate()?;
    s.validate()?;
    check(
        c.control_points[0].len() == 3
            && limits.tolerance.is_finite()
            && limits.tolerance > 0.
            && (1..=256).contains(&limits.max_segments)
            && (1..=64).contains(&limits.max_iterations)
            && (1..=1000000).contains(&limits.agreement_cells),
        "Pullback requires 3D curve, positive tolerance and bounded work",
    )?;
    let mut out = Report {
        pcurve: None,
        agreement: None,
        agreement_cells: 0,
        evaluations: 0,
        within_tolerance: false,
    };
    if let Some(p) = affine_candidate(c, s) {
        let proof = curve_surface_agreement::verify(
            c,
            &p,
            s,
            false,
            limits.tolerance,
            limits.agreement_cells.min(100000),
        )?;
        out.agreement_cells += proof.cells;
        if proof.status == Status::WithinTolerance {
            out.within_tolerance = true;
            out.pcurve = Some(p);
            out.agreement = Some(proof);
            return Ok(out);
        }
        out.agreement = Some(proof);
    }
    let d = domains(s);
    let [a, b] = c.domain();
    let mut segments = 1;
    loop {
        if out.agreement_cells == limits.agreement_cells {
            return Ok(out);
        }
        let mut points = Vec::<Vec<f64>>::new();
        let mut seed = [(d[0][0] + d[0][1]) * 0.5, (d[1][0] + d[1][1]) * 0.5];
        for i in 0..=segments {
            let t = if i == 0 {
                a
            } else if i == segments {
                b
            } else {
                a + (b - a) * i as f64 / segments as f64
            };
            let point = c.evaluate(t)?.point;
            out.evaluations += 1;
            for _ in 0..limits.max_iterations {
                let e = s.evaluate(seed[0], seed[1])?;
                out.evaluations += 1;
                let Some((du, dv)) = e.first_derivatives() else {
                    break;
                };
                let Some(step) = invert(du, dv, sub(&point, &e.point)) else {
                    break;
                };
                let next = std::array::from_fn(|k| (seed[k] + step[k]).clamp(d[k][0], d[k][1]));
                if next == seed {
                    break;
                }
                seed = next;
            }
            points.push(seed.to_vec());
        }
        let mut p = Curve::from_polyline(points)?;
        // Preserve the sampled normalized source traversal exactly.
        p.knots = std::iter::once(0.)
            .chain((0..=segments).map(|i| i as f64 / segments as f64))
            .chain(std::iter::once(1.))
            .collect();
        p.validate()?;
        let proof = curve_surface_agreement::verify(
            c,
            &p,
            s,
            false,
            limits.tolerance,
            (limits.agreement_cells - out.agreement_cells).min(100000),
        )?;
        out.agreement_cells += proof.cells;
        let admitted = proof.status == Status::WithinTolerance;
        out.agreement = Some(proof);
        if admitted {
            out.within_tolerance = true;
            out.pcurve = Some(p);
            return Ok(out);
        }
        if segments * 2 > limits.max_segments || segments * 2 >= 256 {
            return Ok(out);
        }
        segments *= 2;
    }
}
