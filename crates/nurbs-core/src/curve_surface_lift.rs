//! Shared-endpoint world proposals verified against unchanged original pcurves.
//! Hermite samples propose geometry; complete interval source composition is
//! the only positional admission gate. No exact coedge identity or G1 is claimed.
use crate::{check, curve::Curve, curve_surface_agreement as agreement, surface::Surface, Result};
#[derive(Clone, Copy)]
pub struct Limits {
    pub proposal_cells: usize,
    pub agreement_cells: usize,
    pub cells_per_attempt: usize,
}
pub struct Attempt {
    pub source: usize,
    pub interval: [f64; 2],
    pub curve: Curve,
    pub agreement: agreement::Report,
}
pub struct Piece {
    pub source: usize,
    pub interval: [f64; 2],
    pub attempt: Option<usize>,
    pub admitted: bool,
}
pub struct Report {
    pub attempts: Vec<Attempt>,
    pub pieces: Vec<Piece>,
    pub visited: usize,
    pub agreement_cells: usize,
    pub point_stations: usize,
    pub surface_samples: usize,
    pub shared_endpoints_proven: bool,
    pub source_lift_proven: bool,
    pub tolerance_mm: f64,
}
fn clamped(c: &Curve) -> bool {
    let d = c.domain();
    !c.periodic
        && c.knots[..=c.degree].iter().all(|&x| x == d[0])
        && c.knots[c.control_points.len()..].iter().all(|&x| x == d[1])
}
fn same(a: &[f64], b: &[f64]) -> bool {
    a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}
fn sample(s: &Surface, p: &Curve, t: f64) -> Result<([f64; 3], Option<[f64; 3]>)> {
    let pc = p.evaluate(t)?;
    // A validated clamped pcurve has exact Cartesian control-point endpoints.
    // Preserve that original identity instead of a rounded weight cancellation.
    let uv = if t == p.domain()[0] {
        &p.control_points[0]
    } else if t == p.domain()[1] {
        p.control_points.last().unwrap()
    } else {
        &pc.point
    };
    let image = s.evaluate_validated(uv[0], uv[1])?;
    let derivative = match (pc.d1, image.first_derivatives()) {
        (Some(d), Some((u, v))) => {
            let tangent = std::array::from_fn(|k| u[k] * d[0] + v[k] * d[1]);
            tangent.iter().all(|x| x.is_finite()).then_some(tangent)
        }
        _ => None,
    };
    Ok((image.point, derivative))
}
pub fn propose(
    surface: &Surface,
    pcurves: &[Curve],
    tolerance_mm: f64,
    limits: Limits,
) -> Result<Report> {
    surface.validate()?;
    check(
        !pcurves.is_empty()
            && pcurves.len() <= 100000
            && tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=100000).contains(&limits.proposal_cells)
            && (1..=100000).contains(&limits.agreement_cells)
            && (1..=100000).contains(&limits.cells_per_attempt),
        "Choose positive lift tolerance and bounded work",
    )?;
    for p in pcurves {
        p.validate()?;
        check(
            p.control_points[0].len() == 2 && clamped(p),
            "Lift needs original clamped nonperiodic 2D pcurves",
        )?;
    }
    for pair in pcurves.windows(2) {
        check(
            pair[0].domain()[1].to_bits() == pair[1].domain()[0].to_bits()
                && same(
                    pair[0].control_points.last().unwrap(),
                    &pair[1].control_points[0],
                ),
            "Original pcurve pieces must have bit-identical ordered joins",
        )?;
    }
    let mut out = Report {
        attempts: vec![],
        pieces: vec![],
        visited: 0,
        agreement_cells: 0,
        point_stations: 0,
        surface_samples: 0,
        shared_endpoints_proven: false,
        source_lift_proven: false,
        tolerance_mm,
    };
    let mut queue = pcurves
        .iter()
        .enumerate()
        .map(|(source, p)| (source, p.domain(), 0usize))
        .collect::<std::collections::VecDeque<_>>();
    let mut cache = std::collections::BTreeMap::<u64, [f64; 3]>::new();
    while let Some((source, interval, depth)) = queue.pop_front() {
        if out.visited == limits.proposal_cells || out.agreement_cells == limits.agreement_cells {
            out.pieces.push(Piece {
                source,
                interval,
                attempt: None,
                admitted: false,
            });
            continue;
        }
        out.visited += 1;
        let p = &pcurves[source];
        out.surface_samples += 2;
        let mut samples = [
            sample(surface, p, interval[0])?,
            sample(surface, p, interval[1])?,
        ];
        for (end, t) in interval.iter().enumerate() {
            if let Some(point) = cache.get(&t.to_bits()) {
                samples[end].0 = *point;
            } else {
                cache.insert(t.to_bits(), samples[end].0);
                out.point_stations += 1;
            }
        }
        let scale = (interval[1] - interval[0]) / 3.;
        let (degree, control_points) = if let (Some(a), Some(b)) = (samples[0].1, samples[1].1) {
            (
                3,
                vec![
                    samples[0].0.to_vec(),
                    (0..3).map(|k| samples[0].0[k] + scale * a[k]).collect(),
                    (0..3).map(|k| samples[1].0[k] - scale * b[k]).collect(),
                    samples[1].0.to_vec(),
                ],
            )
        } else {
            (1, vec![samples[0].0.to_vec(), samples[1].0.to_vec()])
        };
        let curve = Curve {
            degree,
            knots: [vec![interval[0]; degree + 1], vec![interval[1]; degree + 1]].concat(),
            control_points,
            weights: vec![1.; degree + 1],
            periodic: false,
        };
        let report = agreement::verify_on(
            &curve,
            p,
            surface,
            false,
            interval,
            tolerance_mm,
            (limits.agreement_cells - out.agreement_cells).min(limits.cells_per_attempt),
        )?;
        out.agreement_cells += report.cells;
        let admitted = report.status == agreement::Status::WithinTolerance;
        let index = out.attempts.len();
        out.attempts.push(Attempt {
            source,
            interval,
            curve,
            agreement: report,
        });
        let mid = interval[0] * 0.5 + interval[1] * 0.5;
        if !admitted
            && out.visited < limits.proposal_cells
            && out.agreement_cells < limits.agreement_cells
            && depth < 32
            && mid > interval[0]
            && mid < interval[1]
        {
            queue.push_back((source, [interval[0], mid], depth + 1));
            queue.push_back((source, [mid, interval[1]], depth + 1));
        } else {
            out.pieces.push(Piece {
                source,
                interval,
                attempt: Some(index),
                admitted,
            });
        }
    }
    out.pieces
        .sort_by(|a, b| a.interval[0].total_cmp(&b.interval[0]));
    if out.pieces.iter().all(|p| p.admitted) {
        out.shared_endpoints_proven = out.pieces.windows(2).all(|pair| {
            let a = &out.attempts[pair[0].attempt.unwrap()].curve;
            let b = &out.attempts[pair[1].attempt.unwrap()].curve;
            pair[0].interval[1].to_bits() == pair[1].interval[0].to_bits()
                && same(a.control_points.last().unwrap(), &b.control_points[0])
        });
        out.source_lift_proven = out.shared_endpoints_proven;
    }
    Ok(out)
}
/// Frobenius norm bounds the original point-map Lipschitz constant on a
/// complete continuous natural chart. Periodic/discontinuous charts refuse.
/// This proves positional propagation of UV error, not injectivity or normals.
pub fn lipschitz_upper(surface: &Surface, max_spans: usize) -> Result<Option<f64>> {
    surface.validate()?;
    check(
        (1..=100000).contains(&max_spans),
        "Choose bounded original point-map derivative work",
    )?;
    if surface.periodic_u || surface.periodic_v {
        return Ok(None);
    }
    let natural = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    for (knots, degree, domain) in [
        (&surface.knots_u, surface.degree_u, natural[0]),
        (&surface.knots_v, surface.degree_v, natural[1]),
    ] {
        if knots
            .iter()
            .filter(|&&t| t > domain[0] && t < domain[1])
            .any(|&t| knots.iter().filter(|&&k| k == t).count() > degree)
        {
            return Ok(None);
        }
    }
    let (jets, _) = crate::normal_alignment::jet_bounds(surface, natural, max_spans)?;
    let Some(jets) = jets else {
        return Ok(None);
    };
    use crate::distance_bounds::Interval as I;
    let mut squared = I::point(0.);
    for axis in jets.first {
        for derivative in axis {
            let magnitude = derivative.lo.abs().max(derivative.hi.abs());
            squared = squared.add(I::point(magnitude).mul(I::point(magnitude))?)?;
        }
    }
    Ok(Some(squared.hi.max(0.).sqrt().next_up()))
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
                vec![vec![5., -3., 2.], vec![4.2, -2.4, 2.]],
                vec![vec![5.6, -2.2, 2.], vec![4.8, -1.6, 2.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }
    fn cylinder() -> Surface {
        let mut s = plane();
        s.degree_u = 2;
        s.knots_u = vec![0., 0., 0., 1., 1., 1.];
        s.control_points = [[3., 0.], [3., 3.], [0., 3.]]
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
            .to_vec();
        s.weights = [1., std::f64::consts::FRAC_1_SQRT_2, 1.]
            .map(|w| vec![w; 2])
            .to_vec();
        s
    }
    fn line(a: [f64; 2], b: [f64; 2], domain: [f64; 2]) -> Curve {
        Curve {
            degree: 1,
            knots: vec![domain[0], domain[0], domain[1], domain[1]],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1.; 2],
            periodic: false,
        }
    }
    fn limits() -> Limits {
        Limits {
            proposal_cells: 511,
            agreement_cells: 4096,
            cells_per_attempt: 1,
        }
    }
    fn cover(r: &Report) {
        assert_eq!(r.pieces.first().unwrap().interval[0], 2.);
        assert_eq!(r.pieces.last().unwrap().interval[1], 8.);
        for p in r.pieces.windows(2) {
            assert_eq!(p[0].interval[1], p[1].interval[0]);
        }
    }
    #[test]
    fn rotated_world_lifts_keep_ordered_original_pcurves_and_shared_endpoints() {
        let pc = vec![
            line([0.1, 0.2], [0.5, 0.3], [2., 5.]),
            line([0.5, 0.3], [0.9, 0.6], [5., 8.]),
        ];
        let before = pc.clone();
        let r = propose(&plane(), &pc, 1e-8, limits()).unwrap();
        cover(&r);
        assert!(r.source_lift_proven && r.shared_endpoints_proven);
        assert_eq!(pc, before);
        assert_eq!(r.pieces.len(), 2);
        assert_eq!(r.point_stations, 3);
        assert_eq!(r.surface_samples, 4);
    }
    #[test]
    fn rational_source_lift_is_certified_over_every_original_interval() {
        let pc = vec![line([0.1, 0.25], [0.9, 0.35], [2., 8.])];
        let r = propose(&cylinder(), &pc, 1e-5, limits()).unwrap();
        cover(&r);
        assert!(
            r.source_lift_proven,
            "pieces={}, visits={}, work={}",
            r.pieces.len(),
            r.visited,
            r.agreement_cells
        );
        assert!(r.pieces.len() > 1 && r.shared_endpoints_proven);
        assert!(r
            .pieces
            .iter()
            .all(|p| r.attempts[p.attempt.unwrap()].agreement.status
                == agreement::Status::WithinTolerance));
        let mut work = limits();
        work.proposal_cells = 1;
        let r = propose(&cylinder(), &pc, 1e-10, work).unwrap();
        cover(&r);
        assert!(!r.source_lift_proven && r.visited == 1);
    }
    #[test]
    fn point_map_scale_requires_complete_derivative_coverage() {
        let s = plane();
        let bound = lipschitz_upper(&s, 16).unwrap().unwrap();
        assert!(bound >= 2f64.sqrt() && bound < 1.415);
        let mut source = s.clone();
        source.periodic_u = true;
        // A valid periodic input must still refuse the global point-map gate.
        source.knots_u = vec![-1., 0., 1., 2., 3.];
        source.control_points.push(source.control_points[0].clone());
        source.weights.push(vec![1.; 2]);
        assert!(lipschitz_upper(&source, 16).unwrap().is_none());
    }
    #[test]
    fn clamped_weighted_endpoint_uses_original_uv_control_identity() {
        let mut surface = plane();
        surface.knots_u = vec![0., 0., 3., 3.];
        let mut pc = line([0., 0.4], [3., 0.4], [2., 8.]);
        pc.weights = vec![0.1; 2];
        let r = propose(&surface, &[pc], 1e-8, limits()).unwrap();
        cover(&r);
        assert!(r.source_lift_proven);
    }
}
