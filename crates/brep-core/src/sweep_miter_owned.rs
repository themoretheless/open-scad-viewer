//! Original-request-owned miter boundary construction. Numerical certificates
//! are derived here; callers cannot inject a model, sections or boundary bound.
//! Global material validity and full boundary smoothness remain separate audits.
use crate::{
    Model, Result, invalid, sweep_cap_contacts, sweep_miter_layout, sweep_retained_caps,
    sweep_retained_decomposition, sweep_retained_walls,
};
use nurbs_core::{
    curve::Curve,
    miter_section_correction::{self, Budget, CapCorrection},
    numerics::error_upper,
    progressive_miter::{self, Sweep},
    sweeps::filled_cap_error::{self, BoundaryCertificate, Premises},
};

pub mod annular;
pub struct DirectRequest<'a> {
    pub loops: &'a [Vec<Curve>],
    pub points: &'a [[f64; 3]],
    pub normal: [f64; 3],
    pub closed: bool,
    pub miter_limit: f64,
    pub max_deviation: f64,
    pub circle: Option<Budget>,
    pub caps: Option<CapCorrection>,
    pub cap_endpoints: Option<[bool; 2]>,
}

mod cap_repair;
pub use cap_repair::automatic_cap_correction;

/// Shared default for retained-wall chart proof in native transports.
pub const DEFAULT_WALL_CELLS: usize = 10_000;

pub struct Request<'a> {
    pub loops: &'a [Vec<Curve>],
    pub points: &'a [[f64; 3]],
    pub scale: &'a Curve,
    pub twist: &'a Curve,
    pub options: progressive_miter::Options,
    pub affine: Option<(&'a Curve, &'a Curve)>,
    pub frames: Option<(&'a Curve, &'a Curve)>,
    pub guide: Option<&'a Curve>,
    pub circle: Option<Budget>,
    pub caps: Option<CapCorrection>,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub max_products: usize,
    pub max_faces: usize,
    pub correspondence_faces: usize,
    pub cap_max_edges: usize,
    pub wall_cells: usize,
    pub exact_work: u64,
    pub domain_exact_work: u64,
    pub projection_exact_work: u64,
    pub domain_tolerance: f64,
    pub domain_pairs: usize,
    pub domain_cells: usize,
    pub projection_cells: usize,
    pub cap_regions: sweep_cap_contacts::Budgets,
}
/// Preserve uniform periodic polynomial control relations before Bezier
/// extraction. The actual retained jets are still independently audited.
fn periodic_profile_lattice(
    sections: &[Vec<Curve>],
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<Option<(Vec<Vec<Curve>>, f64, u64)>> {
    if !sections
        .iter()
        .flatten()
        .all(|c| c.periodic && c.weights.iter().all(|&w| w == c.weights[0]))
    {
        return Ok(None);
    }
    use nurbs_core::interval_eval::Interval;
    let mut result = sections.to_vec();
    let mut maximum: f64 = 0.;
    let mut work = 0;
    for (source, target) in sections.iter().flatten().zip(result.iter_mut().flatten()) {
        // Four-pole quadratic periodic profiles admit a centrally symmetric
        // lattice proposal. This is only a numerical proposal: displacement
        // is bounded against every original pole and retained G2 is audited.
        if source.degree == 2
            && source.control_points.len() == 6
            && source.knots.len() == 9
            && source
                .knots
                .windows(2)
                .all(|w| w[1] - w[0] == source.knots[1] - source.knots[0])
            && source.knots[1] > source.knots[0]
        {
            let mut proposed = target.control_points.clone();
            let mut admissible = true;
            for k in 0..3 {
                if max_work - work < 3 {
                    return Err(invalid("Periodic profile lattice work exhausted"));
                }
                work += 3;
                let p = &source.control_points;
                let center = ((p[0][k] + p[2][k]) * 0.5 / quantum).round();
                let a = ((p[0][k] - p[2][k]) * 0.5 / quantum).round();
                let b = ((p[1][k] - p[3][k]) * 0.5 / quantum).round();
                let values = [
                    center + a,
                    center + b,
                    center - a,
                    center - b,
                    center + a,
                    center + b,
                ];
                for (point, value) in proposed.iter_mut().zip(values) {
                    admissible &= value.is_finite() && value.abs() < 2_f64.powi(46);
                    point[k] = value * quantum;
                }
            }
            let mut displacement: f64 = 0.;
            if admissible {
                for (a, b) in source.control_points.iter().zip(&proposed) {
                    let mut squared = Interval::point(0.);
                    for k in 0..3 {
                        if work == max_work {
                            return Err(invalid("Periodic profile lattice work exhausted"));
                        }
                        work += 1;
                        let difference = Interval::point(b[k]).sub(Interval::point(a[k]))?;
                        squared = squared.add(difference.mul(difference)?)?;
                    }
                    displacement = displacement.max(squared.hi.max(0.).sqrt().next_up());
                }
                admissible &= displacement.is_finite() && displacement <= tolerance;
            }
            if admissible {
                target.control_points = proposed;
                maximum = maximum.max(displacement);
                continue;
            }
        }
        for (a, b) in source
            .control_points
            .iter()
            .zip(target.control_points.iter_mut())
        {
            let mut squared = Interval::point(0.);
            for k in 0..3 {
                if work == max_work {
                    return Err(invalid("Periodic profile lattice work exhausted"));
                }
                work += 1;
                let integer = (a[k] / quantum).round();
                if !integer.is_finite() || integer.abs() >= 2_f64.powi(46) {
                    return Err(invalid("Periodic profile lattice numeric range unproved"));
                }
                b[k] = integer * quantum;
                let difference = Interval::point(b[k]).sub(Interval::point(a[k]))?;
                squared = squared.add(difference.mul(difference)?)?;
            }
            maximum = maximum.max(squared.hi.max(0.).sqrt().next_up());
        }
    }
    if !maximum.is_finite() || maximum > tolerance {
        return Err(invalid(
            "Periodic profile lattice displacement exceeds budget",
        ));
    }
    Ok(Some((result, maximum, work)))
}
/// Same-basis bounded control proposal for rational periodic profiles.
/// Integer strides remove odd denominators only when exact native extraction
/// independently accepts every coefficient. Constant coordinates stay intact.
fn rational_periodic_lattice(
    sections: &[Vec<Curve>],
    quantum: f64,
    tolerance: f64,
    max_work: u64,
) -> Result<Option<(Vec<Vec<Curve>>, f64, u64)>> {
    use nurbs_core::interval_eval::Interval;
    if sections.iter().flatten().any(|c| {
        !c.periodic
            || c.degree != 2
            || c.control_points.len() > 32
            || c.weights.iter().all(|w| *w == c.weights[0])
    }) {
        return Ok(None);
    }
    let mut work = 0;
    let mut proposal = sections.to_vec();
    for (stride_idx, stride) in [1., 3., 5., 7., 15., 21., 35., 105.].into_iter().enumerate() {
        let step = quantum * stride;
        if stride_idx > 0 {
            for (src_sec, dst_sec) in sections.iter().zip(proposal.iter_mut()) {
                for (src_c, dst_c) in src_sec.iter().zip(dst_sec.iter_mut()) {
                    for (src_p, dst_p) in src_c
                        .control_points
                        .iter()
                        .zip(dst_c.control_points.iter_mut())
                    {
                        dst_p.copy_from_slice(src_p);
                    }
                }
            }
        }
        let mut maximum: f64 = 0.;
        let mut exact = true;
        for (source, target) in sections.iter().flatten().zip(proposal.iter_mut().flatten()) {
            let p0 = &source.control_points[0];
            let varying = [
                source.control_points.iter().any(|p| p[0] != p0[0]),
                source.control_points.iter().any(|p| p[1] != p0[1]),
                source.control_points.iter().any(|p| p[2] != p0[2]),
            ];
            for (a, b) in source
                .control_points
                .iter()
                .zip(target.control_points.iter_mut())
            {
                let mut squared = Interval::point(0.);
                for k in 0..3 {
                    if work == max_work {
                        return Err(invalid("Rational periodic lattice work exhausted"));
                    }
                    work += 1;
                    if varying[k] {
                        let units = (a[k] / step).round();
                        if !units.is_finite() || units.abs() >= 2_f64.powi(46) {
                            return Ok(None);
                        }
                        b[k] = units * step;
                    }
                    let delta = Interval::point(b[k]).sub(Interval::point(a[k]))?;
                    squared = squared.add(delta.mul(delta)?)?;
                }
                maximum = maximum.max(squared.hi.max(0.).sqrt().next_up());
            }
            let rows = (target.control_points.len() - target.degree) * (target.degree + 1);
            if max_work - work < rows as u64 {
                return Err(invalid("Rational periodic lattice work exhausted"));
            }
            work += rows as u64;
            exact &= nurbs_core::retained_wall_coefficients::exact_bezier_controls(target, rows)
                .is_some();
        }
        if exact && maximum.is_finite() && maximum <= tolerance {
            return Ok(Some((proposal, maximum, work)));
        }
    }
    Ok(None)
}

fn nonplanar_caps(model: &Model, max_work: u64) -> (bool, u64) {
    use cad_predicates::{
        AuthoredScalar, Outcome, PredicateContext, Sign, SourceArena, ToleranceContext,
    };
    let mut work = 0;
    for face in &model.faces[model.faces.len() - 2..] {
        let s = &face.surface;
        if s.degree_u != 1
            || s.degree_v != 1
            || s.control_points.len() != 2
            || s.control_points.iter().any(|r| r.len() != 2)
        {
            continue;
        }
        let mut values = Vec::with_capacity(12);
        values.extend(
            s.control_points
                .iter()
                .flatten()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits())),
        );
        let Ok(arena) = SourceArena::authored("owned-cap-planarity", 1, values) else {
            continue;
        };
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            cad_predicates::Limits {
                max_work: max_work - work,
                ..Default::default()
            },
            None,
        );
        let p = |i: usize| std::array::from_fn(|k| arena.leaf(3 * i + k).unwrap());
        let result = cad_predicates::orient3d(&mut ctx, p(0), p(1), p(2), p(3));
        work += ctx.work_used();
        if result.is_ok_and(|r| matches!(r.outcome, Outcome::Sign(Sign::Positive | Sign::Negative)))
        {
            return (true, work);
        }
    }
    (false, work)
}
/// Private owned evidence: no public constructor from caller-authored scalars.
pub struct BoundaryBody {
    model: Model,
    sections: Vec<Vec<Vec<Curve>>>,
    source_sections: Vec<Vec<Curve>>,
    correction: Option<nurbs_core::sweep_section_projection::Report>,
    wall_charts: crate::sweep_retained_charts::Report,
    edges: usize,
    max_steps: usize,
    levels: Vec<progressive_miter::Report>,
    boundary: BoundaryCertificate,
    sharp: Vec<usize>,
    closed: bool,
    budget: f64,
    caps_deferred:bool,
    cap_probe_work:u64,
}
/// Conditional reconstruction from private constructor evidence. This report
/// does not admit material validity or transfer ownership of its certificate.
pub struct StationBoundary {
    pub reconstruction: Option<crate::sweep_station_reconstruction::Reconstruction>,
    pub boundary: Option<BoundaryCertificate>,
    pub reason: &'static str,
}
impl BoundaryBody {
    pub fn reconstruct_stations(
        &self,
        quantum: f64,
        tolerance: f64,
        max_work: u64,
        budget: Option<f64>,
    ) -> Result<StationBoundary> {
        let mut out = StationBoundary {
            reconstruction: None,
            boundary: None,
            reason: "source-bound-unproved",
        };
        if self.complete_boundary().is_none() {
            return Ok(out);
        }
        let mut reconstruction = crate::sweep_station_reconstruction::reconstruct(
            &self.model,
            &self.sections,
            &self.sharp,
            self.closed,
            quantum,
            tolerance,
            max_work,
        )?;
        let Some(displacement) = reconstruction.candidate.wall_displacement_upper else {
            out.reason = reconstruction.candidate.reason;
            return Ok(out);
        };
        if reconstruction.model.is_none() {
            out.reason = reconstruction.candidate.reason;
            return Ok(out);
        }
        let wall = self
            .boundary
            .wall_error_upper
            .and_then(|source| error_upper::add(source, displacement));
        let boundary = filled_cap_error::compose_boundary(
            wall,
            self.boundary.filled_cap_error_upper,
            self.closed,
            budget,
        );
        if !boundary.continuous_bound || boundary.within_budget != Some(true) {
            reconstruction.model = None;
            out.reason = "boundary-budget-unproved";
        } else {
            out.reason = "owned-station-complete-boundary";
        }
        out.reconstruction = Some(reconstruction);
        out.boundary = Some(boundary);
        Ok(out)
    }
    pub fn closed(&self) -> bool {
        self.closed
    }
    pub fn budget(&self) -> f64 {
        self.budget
    }
    pub fn place(
        &self,
        matrix: [[f64; 4]; 4],
        quantum: f64,
        max_work: u64,
        budget: Option<f64>,
    ) -> Result<crate::sweep_affine_boundary::Report> {
        crate::sweep_affine_boundary::place(
            &self.model,
            &crate::sweep_affine_boundary::Premises {
                wall: self.boundary.wall_error_upper,
                caps: self.boundary.filled_cap_error_upper,
                closed: self.closed,
                source_budget: Some(self.budget),
            },
            matrix,
            quantum,
            max_work,
            budget,
        )
    }
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn sections(&self) -> &[Vec<Vec<Curve>>] {
        &self.sections
    }
    pub fn source_sections(&self) -> &[Vec<Curve>] {
        &self.source_sections
    }
    pub fn correction(&self) -> Option<&nurbs_core::sweep_section_projection::Report> {
        self.correction.as_ref()
    }
    pub fn wall_charts(&self) -> &crate::sweep_retained_charts::Report {
        &self.wall_charts
    }
    pub fn edges(&self) -> usize {
        self.edges
    }
    pub fn max_steps(&self) -> usize {
        self.max_steps
    }
    pub fn levels(&self) -> &[progressive_miter::Report] {
        &self.levels
    }
    pub fn boundary(&self) -> &BoundaryCertificate {
        &self.boundary
    }
    pub fn sharp_stations(&self) -> &[usize] {
        &self.sharp
    }
    /// Completeness and tolerance are necessary; this is not Solid admission.
    pub fn complete_boundary(&self) -> Option<&BoundaryCertificate> {
        (self.boundary.continuous_bound && self.boundary.within_budget == Some(true))
            .then_some(&self.boundary)
    }
}

enum ConstructionAttempt {
    Complete(BoundaryBody),
    Refine { levels:Vec<progressive_miter::Report>, steps:usize, work:u64, reason:&'static str },
}

pub fn construct(request: &Request<'_>, limits: &Limits) -> Result<BoundaryBody> {
    construct_impl(request, limits, false)
}
fn construct_source_for_annular(request: &Request<'_>, limits: &Limits) -> Result<BoundaryBody> {
    construct_impl(request, limits, true)
}
fn construct_impl(
    request: &Request<'_>,
    limits: &Limits,
    annular_caps: bool,
) -> Result<BoundaryBody> {
    let mut initial = request.options.initial_steps;
    let mut previous = Vec::with_capacity(8);
    let mut spent = 0_u64;
    let mut cached_max_steps: Option<usize> = None;
    loop {
        // Correction work remains caller-owned across rejected candidates.
        // Charge combined correction work conservatively to each supplied
        // allowance; no failed candidate starts a fresh work budget.
        let remaining = |budget: Budget| -> Result<Budget> {
            let max_work = match budget.max_work {
                Some(work) if work.is_finite() && work >= spent as f64 => Some(work - spent as f64),
                Some(_) => {
                    return Err(invalid(
                        "Progressive miter correction work exhausted during refinement",
                    ));
                }
                None => None,
            };
            Ok(Budget { max_work, ..budget })
        };
        let attempt = Request {
            loops: request.loops,
            points: request.points,
            scale: request.scale,
            twist: request.twist,
            options: progressive_miter::Options {
                initial_steps: initial,
                ..request.options
            },
            affine: request.affine,
            frames: request.frames,
            guide: request.guide,
            circle: request.circle.map(remaining).transpose()?,
            caps: request
                .caps
                .as_ref()
                .map(|cap| -> Result<CapCorrection> {
                    Ok(CapCorrection {
                        budget: remaining(cap.budget)?,
                        authored_frame: cap.authored_frame,
                    })
                })
                .transpose()?,
        };
        let attempt_limits = Limits {
            projection_exact_work: limits.projection_exact_work.saturating_sub(spent),
            ..*limits
        };
        match construct_attempt(&attempt, &attempt_limits, false, None, annular_caps, false)? {
            ConstructionAttempt::Complete(mut body) => {
                previous.append(&mut body.levels);
                body.levels = previous;
                if let Some(correction) = &mut body.correction {
                    correction.work = correction
                        .work
                        .checked_add(spent)
                        .ok_or_else(|| invalid("Progressive miter correction work overflow"))?;
                }
                return Ok(body);
            }
            ConstructionAttempt::Refine {
                mut levels,
                steps,
                work,
                reason,
            } => {
                let max_steps = match cached_max_steps {
                    Some(m) => m,
                    None => {
                        let planner = if annular_caps {
                            sweep_miter_layout::plan_annular
                        } else {
                            sweep_miter_layout::plan
                        };
                        let plan = planner(
                            request.loops,
                            request.points.len(),
                            request.options.closed,
                            request.options.initial_steps,
                            request.options.max_steps,
                        )?;
                        cached_max_steps = Some(plan.max_steps);
                        plan.max_steps
                    }
                };
                if steps >= max_steps {
                    return Err(invalid(reason));
                }
                if let Some(last) = levels.last_mut() {
                    last.accepted = false;
                    last.error_certificate_reason = Some("complete-boundary-refinement");
                }
                previous.append(&mut levels);
                spent = spent
                    .checked_add(work)
                    .ok_or_else(|| invalid("Progressive miter correction work overflow"))?;
                initial = (steps * 2).min(max_steps);
            }
        }
    }
}

pub fn construct_direct(request: &DirectRequest<'_>, limits: &Limits) -> Result<BoundaryBody> {
    construct_direct_impl(request, limits, false, false)
}
fn construct_direct_source_for_annular(
    request: &DirectRequest<'_>,
    limits: &Limits,
) -> Result<BoundaryBody> {
    construct_direct_impl(request, limits, true, false)
}
fn construct_direct_source_for_auto(
    request: &DirectRequest<'_>,
    limits: &Limits,
) -> Result<BoundaryBody> {
    construct_direct_impl(request, limits, true, true)
}
fn construct_direct_impl(
    request: &DirectRequest<'_>,
    limits: &Limits,
    annular_caps: bool,
    automatic_caps: bool,
) -> Result<BoundaryBody> {
    let scalar = |value| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![value, 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let scale = scalar(1.);
    let twist = scalar(0.);
    let original = Request {
        loops: request.loops,
        points: request.points,
        scale: &scale,
        twist: &twist,
        options: progressive_miter::Options {
            normal: request.normal,
            closed: request.closed,
            miter_limit: request.miter_limit,
            initial_steps: 1,
            max_steps: 1,
            max_deviation: request.max_deviation,
        },
        affine: None,
        frames: None,
        guide: None,
        circle: request.circle,
        caps: request.caps.as_ref().map(|cap| CapCorrection {
            budget: cap.budget,
            authored_frame: cap.authored_frame,
        }),
    };
    if request.cap_endpoints.is_some() && request.caps.is_none() {
        return Err(invalid(
            "Direct endpoint selection requires explicit cap correction",
        ));
    }
    match construct_attempt(
        &original,
        limits,
        true,
        request.cap_endpoints,
        annular_caps,
        automatic_caps,
    )? {
        ConstructionAttempt::Complete(body) => Ok(body),
        ConstructionAttempt::Refine { reason, .. } => Err(invalid(reason)),
    }
}

pub(crate) fn cap_boundaries_exact(model: &Model, max_work: u64) -> Result<(bool, u64)> {
    let mut work = 0;
    for face in &model.faces[model.faces.len() - 2..] {
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for use_ in &model.loops[wire].coedges {
                let Some(proof) = nurbs_core::curve_surface_agreement::verify_exact(
                    &model.edges[use_.edge].curve,
                    &use_.pcurve,
                    &face.surface,
                    use_.reversed,
                    max_work - work,
                )?
                else {
                    return Ok((false, max_work));
                };
                if proof.work_used > max_work - work {
                    return Err(invalid(
                        "Cap identity probe exceeded its exact-work allowance",
                    ));
                }
                work += proof.work_used;
                if proof.outcome != cad_predicates::BezierIdentity::Equal {
                    return Ok((false, work));
                }
            }
        }
    }
    Ok((true, work))
}


#[cfg(test)]
#[path = "tests/sweep_miter_owned.rs"]
mod tests;

#[cfg(test)]
#[path = "sweep_miter_owned/tests/direct_source_regressions_integration.rs"]
mod direct_source_regressions;

#[path = "sweep_miter_owned/owned_attempt.rs"]
mod owned_attempt;
use owned_attempt::construct_attempt;
