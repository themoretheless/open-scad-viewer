//! Full original-parameter cover: inside winding region OR within a declared
//! distance of an original boundary point. Never an exact set-inclusion claim.
use super::{Location, TrimDomain, exact_loop_joins};
use crate::{
    Result, check,
    curve::Curve,
    interval_eval::{Interval as I, evaluate_interval},
};

#[derive(Clone, Copy)]
pub struct Limits {
    pub cells: usize,
    pub classification_work: usize,
    pub witness_work: usize,
}
pub enum Evidence {
    OriginalBoundary {
        curve: usize,
    },
    Interior(super::Classification),
    BoundaryBand {
        curve: usize,
        parameter: f64,
        error_upper: f64,
    },
}
pub struct Cell {
    pub parameter: [f64; 2],
    pub evidence: Evidence,
}
pub struct Certificate {
    curve: Curve,
    loops: Vec<Vec<Curve>>,
    tolerance: f64,
    limits: Limits,
    cells: Vec<Cell>,
}
impl Certificate {
    pub fn curve(&self) -> &Curve {
        &self.curve
    }
    pub fn loops(&self) -> &[Vec<Curve>] {
        &self.loops
    }
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }
    pub fn reverify(&self) -> Result<bool> {
        Ok(
            inspect(&self.curve, &self.loops, self.tolerance, self.limits)?
                .certificate
                .is_some(),
        )
    }
}
pub struct Report {
    pub certificate: Option<Certificate>,
    pub cells: usize,
    pub classification_work: usize,
    pub witness_work: usize,
    pub pending: Vec<[f64; 2]>,
    pub reason: &'static str,
}
fn proposal(c: &Curve, q: [f64; 2], work: &mut usize, max: usize) -> Result<Option<f64>> {
    let d = c.domain();
    let mut best = (f64::INFINITY, d[0]);
    for seed in [d[0], d[0] / 2. + d[1] / 2., d[1]] {
        let mut t = seed;
        for _ in 0..8 {
            if *work == max {
                return Ok(None);
            }
            *work += 1;
            let e = c.evaluate(t)?;
            let v = [e.point[0] - q[0], e.point[1] - q[1]];
            let distance = v[0].hypot(v[1]);
            if distance < best.0 {
                best = (distance, t);
            }
            let (Some(a), Some(b)) = (e.d1, e.d2) else {
                break;
            };
            let denominator = a[0] * a[0] + a[1] * a[1] + v[0] * b[0] + v[1] * b[1];
            if !denominator.is_finite() || denominator == 0. {
                break;
            }
            let next = (t - (v[0] * a[0] + v[1] * a[1]) / denominator).clamp(d[0], d[1]);
            if !next.is_finite() || next == t {
                break;
            }
            t = next;
        }
    }
    Ok(Some(best.1))
}
fn upper(a: &[I], b: &[I]) -> Result<f64> {
    let mut sum = I::point(0.);
    for k in 0..2 {
        let d = a[k].sub(b[k])?;
        let m = I::point(d.lo.abs().max(d.hi.abs()));
        sum = sum.add(m.mul(m)?)?;
    }
    Ok(sum.hi.sqrt().next_up())
}
/// Each accepted leaf covers its WHOLE original knot interval. Floating-point
/// projection only proposes a boundary witness; outward interval evaluations
/// of both original definitions are the sole distance-band authority.
pub fn inspect(
    curve: &Curve,
    loops: &[Vec<Curve>],
    tolerance: f64,
    limits: Limits,
) -> Result<Report> {
    check(
        tolerance.is_finite()
            && tolerance > 0.
            && tolerance / 64. > 0.
            && (1..=100000).contains(&limits.cells)
            && (1..=1000000).contains(&limits.classification_work)
            && (1..=1000000).contains(&limits.witness_work),
        "Curve cover requires finite tolerance and positive bounded work",
    )?;
    curve.validate()?;
    check(
        curve.control_points[0].len() == 2 && curve.weights.iter().all(|w| *w > 0.),
        "Curve cover needs positive-weight XY curves",
    )?;
    for wire in loops {
        check(
            exact_loop_joins(wire)? == Some(true),
            "Curve cover requires exact original loop joins",
        )?;
        for c in wire {
            check(
                c.weights.iter().all(|w| *w > 0.),
                "Original profile weights must be positive",
            )?;
        }
    }
    let region = TrimDomain::new(loops, tolerance / 64.)?;
    let sources = loops.iter().flatten().collect::<Vec<_>>();
    if let Some(index) = sources.iter().position(|c| **c == *curve) {
        return Ok(Report {
            certificate: Some(Certificate {
                curve: curve.clone(),
                loops: loops.to_vec(),
                tolerance,
                limits,
                cells: vec![Cell {
                    parameter: curve.domain(),
                    evidence: Evidence::OriginalBoundary { curve: index },
                }],
            }),
            cells: 1,
            classification_work: 0,
            witness_work: 0,
            pending: vec![],
            reason: "original-boundary-identity",
        });
    }
    let hulls = sources
        .iter()
        .map(|c| {
            (0..2)
                .map(|k| {
                    I::new(
                        c.control_points
                            .iter()
                            .map(|p| p[k])
                            .fold(f64::INFINITY, f64::min),
                        c.control_points
                            .iter()
                            .map(|p| p[k])
                            .fold(f64::NEG_INFINITY, f64::max),
                    )
                })
                .collect::<Result<Vec<_>>>()
        })
        .collect::<Result<Vec<_>>>()?;
    let mut pending = (curve.degree..curve.control_points.len())
        .filter(|&i| curve.knots[i] < curve.knots[i + 1])
        .map(|i| [curve.knots[i], curve.knots[i + 1]])
        .collect::<Vec<_>>();
    let mut report = Report {
        certificate: None,
        cells: 0,
        classification_work: 0,
        witness_work: 0,
        pending: vec![],
        reason: "cover-complete",
    };
    let mut accepted = Vec::new();
    while let Some(d) = pending.pop() {
        if report.cells == limits.cells || report.classification_work == limits.classification_work
        {
            pending.push(d);
            report.reason = "cover-work-limit";
            break;
        }
        report.cells += 1;
        let bounds = evaluate_interval(curve, I::new(d[0], d[1])?)?;
        let classified = region.classify(
            [[bounds[0].lo, bounds[0].hi], [bounds[1].lo, bounds[1].hi]],
            (limits.classification_work - report.classification_work).min(1024),
        )?;
        report.classification_work += classified.cells;
        if classified.location == Location::Inside {
            accepted.push(Cell {
                parameter: d,
                evidence: Evidence::Interior(classified),
            });
            continue;
        }
        if classified.location == Location::Outside
            && hulls
                .iter()
                .map(|h| crate::distance_bounds::box_distance(&bounds, h).map(|b| b.0 > tolerance))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .all(|b| *b)
        {
            pending.push(d);
            report.reason = "outside-separated";
            break;
        }
        let point = [
            bounds[0].lo / 2. + bounds[0].hi / 2.,
            bounds[1].lo / 2. + bounds[1].hi / 2.,
        ];
        let mut witness = None;
        for (i, c) in sources.iter().enumerate() {
            // Proposal pruning cannot authorize a leaf; a missed proposal can
            // only leave the cover unresolved, never produce a certificate.
            let gap = (0..2)
                .map(|k| {
                    (hulls[i][k].lo - point[k])
                        .max(point[k] - hulls[i][k].hi)
                        .max(0.)
                })
                .collect::<Vec<_>>();
            if gap[0].hypot(gap[1]) > 2. * tolerance {
                continue;
            }
            let Some(t) = proposal(c, point, &mut report.witness_work, limits.witness_work)? else {
                break;
            };
            if report.witness_work == limits.witness_work {
                break;
            }
            report.witness_work += 1;
            let source = evaluate_interval(c, I::point(t))?;
            let error = upper(&bounds, &source)?;
            if error <= tolerance {
                witness = Some(Evidence::BoundaryBand {
                    curve: i,
                    parameter: t,
                    error_upper: error,
                });
                break;
            }
        }
        if let Some(evidence) = witness {
            accepted.push(Cell {
                parameter: d,
                evidence,
            });
            continue;
        }
        if report.witness_work == limits.witness_work {
            pending.push(d);
            report.reason = "witness-work-limit";
            break;
        }
        let mid = d[0] / 2. + d[1] / 2.;
        if mid <= d[0] || mid >= d[1] {
            pending.push(d);
            report.reason = "cover-precision-limit";
            break;
        }
        pending.push([mid, d[1]]);
        pending.push([d[0], mid]);
    }
    report.pending = pending;
    if report.pending.is_empty() {
        report.certificate = Some(Certificate {
            curve: curve.clone(),
            loops: loops.to_vec(),
            tolerance,
            limits,
            cells: accepted,
        });
    }
    Ok(report)
}
