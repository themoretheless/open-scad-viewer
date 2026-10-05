//! Damped numerical section seeds followed by independent interval root checks.
//! Samples/Newton residuals never certify a whole contact branch or fillet solid.
use crate::{check, surface::Surface, surface_contact::Verdict, surface_offset, Result};
pub struct Report {
    pub fixed_axis: usize,
    pub fixed: f64,
    pub first_uv: [f64; 2],
    pub second_uv: [f64; 2],
    /// Numerical midpoint of the two offset samples, not an exact root enclosure.
    pub center: [f64; 3],
    pub residual_mm: f64,
    pub iterations: usize,
    pub line_search_evaluations: usize,
    /// Independent interval evidence for this fixed-axis section only.
    pub certificate: Option<Verdict>,
    pub reason: &'static str,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn solve(mut a: [[f64; 3]; 3], mut b: [f64; 3]) -> Option<[f64; 3]> {
    for k in 0..3 {
        let pivot = (k..3).max_by(|&i, &j| a[i][k].abs().total_cmp(&a[j][k].abs()))?;
        a.swap(k, pivot);
        b.swap(k, pivot);
        let scale = a[k][k];
        if !scale.is_finite() || scale == 0. {
            return None;
        }
        for j in k..3 {
            a[k][j] /= scale;
        }
        b[k] /= scale;
        for i in 0..3 {
            if i == k {
                continue;
            }
            let factor = a[i][k];
            for j in k..3 {
                a[i][j] -= factor * a[k][j];
            }
            b[i] -= factor * b[k];
        }
    }
    b.iter().all(|v| v.is_finite()).then_some(b)
}
fn residual(a: &surface_offset::Evaluation, b: &surface_offset::Evaluation) -> ([f64; 3], f64) {
    let vector = std::array::from_fn(|k| a.point[k] - b.point[k]);
    (vector, vector[0].hypot(vector[1]).hypot(vector[2]))
}
pub fn propose(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    seeds: [[f64; 2]; 2],
    fixed_axis: usize,
    numerical_tolerance_mm: f64,
    max_iterations: usize,
    padding_fraction: f64,
    max_spans: usize,
) -> Result<Report> {
    for s in surfaces {
        s.validate()?;
    }
    check(
        fixed_axis < 2
            && distances.iter().all(|d| d.is_finite() && *d != 0.)
            && distances[0].abs() == distances[1].abs(),
        "Section predictor needs a fixed source axis and equal positive radius magnitudes",
    )?;
    check(
        numerical_tolerance_mm.is_finite()
            && numerical_tolerance_mm > 0.
            && (1..=32).contains(&max_iterations)
            && padding_fraction.is_finite()
            && padding_fraction > 0.
            && padding_fraction <= 0.25
            && (1..=100000).contains(&max_spans),
        "Section predictor needs finite tolerances, bounded Newton work and a positive natural-domain padding fraction",
    )?;
    let domains = surfaces.map(domain);
    for side in 0..2 {
        for axis in 0..2 {
            check(
                seeds[side][axis].is_finite()
                    && seeds[side][axis] >= domains[side][axis][0]
                    && seeds[side][axis] <= domains[side][axis][1],
                "Section seed must be inside the original surface domain",
            )?;
        }
    }
    let free = 1 - fixed_axis;
    let fixed = seeds[0][fixed_axis];
    let mut uv = seeds;
    let mut jets = [
        surface_offset::evaluate(surfaces[0], uv[0], distances[0])?,
        surface_offset::evaluate(surfaces[1], uv[1], distances[1])?,
    ];
    crate::numeric(
        residual(&jets[0], &jets[1]).1.is_finite(),
        "Offset-center residual exceeded numerical range",
    )?;
    let mut iterations = 0;
    let mut searches = 0;
    let mut reason = "predictor-work-limit";
    for _ in 0..max_iterations {
        let (f, norm) = residual(&jets[0], &jets[1]);
        if norm <= numerical_tolerance_mm {
            reason = "numerical-section-candidate";
            break;
        }
        iterations += 1;
        let first = if free == 0 { jets[0].du } else { jets[0].dv };
        let matrix = std::array::from_fn(|k| [first[k], -jets[1].du[k], -jets[1].dv[k]]);
        let Some(step) = solve(matrix, f.map(|x| -x)) else {
            reason = "predictor-jacobian-singular";
            break;
        };
        let mut accepted = None;
        for trial in 0..12 {
            let factor = 2f64.powi(-(trial as i32));
            let mut candidate = uv;
            candidate[0][free] += factor * step[0];
            candidate[1][0] += factor * step[1];
            candidate[1][1] += factor * step[2];
            if (0..2).any(|side| {
                (0..2).any(|axis| {
                    !candidate[side][axis].is_finite()
                        || candidate[side][axis] < domains[side][axis][0]
                        || candidate[side][axis] > domains[side][axis][1]
                })
            }) {
                continue;
            }
            searches += 1;
            let next = [
                surface_offset::evaluate(surfaces[0], candidate[0], distances[0])?,
                surface_offset::evaluate(surfaces[1], candidate[1], distances[1])?,
            ];
            if residual(&next[0], &next[1]).1 < norm {
                accepted = Some((candidate, next));
                break;
            }
        }
        if let Some((candidate, next)) = accepted {
            uv = candidate;
            jets = next;
        } else {
            reason = "predictor-natural-domain-or-descent-stop";
            break;
        }
    }
    let residual_mm = residual(&jets[0], &jets[1]).1;
    let mut out = Report {
        fixed_axis,
        fixed,
        first_uv: uv[0],
        second_uv: uv[1],
        center: std::array::from_fn(|k| jets[0].point[k] * 0.5 + jets[1].point[k] * 0.5),
        residual_mm,
        iterations,
        line_search_evaluations: searches,
        certificate: None,
        reason,
    };
    if residual_mm <= numerical_tolerance_mm {
        let box_at = |side: usize, axis: usize| {
            let d = domains[side][axis];
            let pad = (d[1] - d[0]) * padding_fraction;
            [
                (uv[side][axis] - pad).max(d[0]),
                (uv[side][axis] + pad).min(d[1]),
            ]
        };
        let first_box = box_at(0, free);
        let second_box = [box_at(1, 0), box_at(1, 1)];
        if first_box[0] >= first_box[1] || second_box.iter().any(|b| b[0] >= b[1]) {
            out.reason = "predictor-padding-unrepresentable";
            return Ok(out);
        }
        let certificate = surface_offset::certify_contact_section(
            surfaces, distances, fixed_axis, fixed, first_box, second_box, max_spans,
        )?;
        out.reason = match certificate {
            Verdict::Witness(_) => "interval-section-root-proven",
            Verdict::Excluded => "numerical-candidate-interval-excluded",
            Verdict::Unresolved => "numerical-candidate-interval-unresolved",
        };
        out.certificate = Some(certificate);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn plane() -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 1., 0.]],
                vec![vec![1., 0., 0.], vec![1., 1., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn parallel_separated_offsets_stop_without_a_certificate() {
        let a = plane();
        let mut b = plane();
        for p in b.control_points.iter_mut().flatten() {
            p[2] = 1.;
        }
        let r = propose([&a, &b], [0.2; 2], [[0.5; 2]; 2], 0, 1e-8, 8, 1e-3, 16).unwrap();
        assert_eq!(r.reason, "predictor-jacobian-singular");
        assert!(r.certificate.is_none());
        assert_eq!(r.residual_mm, 1.);
        assert_eq!(r.line_search_evaluations, 0);
    }
    #[test]
    fn invalid_seed_radius_and_work_are_rejected_before_iteration() {
        let a = plane();
        assert!(propose(
            [&a, &a],
            [0.2; 2],
            [[-0.1, 0.5], [0.5; 2]],
            0,
            1e-8,
            8,
            1e-3,
            16
        )
        .is_err());
        assert!(propose([&a, &a], [0.2, 0.3], [[0.5; 2]; 2], 0, 1e-8, 8, 1e-3, 16).is_err());
        assert!(propose([&a, &a], [0.2; 2], [[0.5; 2]; 2], 0, 1e-8, 0, 1e-3, 16).is_err());
    }
}
