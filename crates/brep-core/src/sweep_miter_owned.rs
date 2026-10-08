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
    for stride in [1., 3., 5., 7., 15., 21., 35., 105.] {
        let step = quantum * stride;
        let mut proposal = sections.to_vec();
        let mut maximum: f64 = 0.;
        let mut exact = true;
        for (source, target) in sections.iter().flatten().zip(proposal.iter_mut().flatten()) {
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
                    if source
                        .control_points
                        .iter()
                        .any(|p| p[k] != source.control_points[0][k])
                    {
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
        let values = s
            .control_points
            .iter()
            .flatten()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect();
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
    let mut initial=request.options.initial_steps;
    let mut previous=Vec::new();
    let mut spent=0_u64;
    loop {
        // Correction work remains caller-owned across rejected candidates.
        // Charge combined correction work conservatively to each supplied
        // allowance; no failed candidate starts a fresh work budget.
        let remaining=|budget:Budget|->Result<Budget>{
            let max_work=match budget.max_work {
                Some(work) if work.is_finite() && work>=spent as f64=>Some(work-spent as f64),
                Some(_)=>return Err(invalid("Progressive miter correction work exhausted during refinement")),
                None=>None,
            };
            Ok(Budget {max_work,..budget})
        };
        let attempt=Request {loops:request.loops,points:request.points,scale:request.scale,twist:request.twist,
            options:progressive_miter::Options {initial_steps:initial,..request.options},
            affine:request.affine,frames:request.frames,guide:request.guide,
            circle:request.circle.map(remaining).transpose()?,
            caps:request.caps.as_ref().map(|cap|->Result<CapCorrection>{Ok(CapCorrection {budget:remaining(cap.budget)?,authored_frame:cap.authored_frame})}).transpose()?,
        };
        let attempt_limits=Limits {projection_exact_work:limits.projection_exact_work.saturating_sub(spent),..*limits};
        match construct_attempt(&attempt,&attempt_limits)? {
            ConstructionAttempt::Complete(mut body)=>{
                previous.append(&mut body.levels);
                body.levels=previous;
                if let Some(correction)=&mut body.correction {correction.work=correction.work.checked_add(spent).ok_or_else(||invalid("Progressive miter correction work overflow"))?;}
                return Ok(body);
            },
            ConstructionAttempt::Refine {mut levels,steps,work,reason}=>{
                let plan=sweep_miter_layout::plan(request.loops,request.points.len(),request.options.closed,
                    request.options.initial_steps,request.options.max_steps)?;
                if steps>=plan.max_steps {return Err(invalid(reason));}
                if let Some(last)=levels.last_mut(){last.accepted=false;last.error_certificate_reason=Some("complete-boundary-refinement");}
                previous.append(&mut levels);
                spent=spent.checked_add(work).ok_or_else(||invalid("Progressive miter correction work overflow"))?;
                initial=(steps*2).min(plan.max_steps);
            },
        }
    }
}

fn construct_attempt(request: &Request<'_>, limits: &Limits) -> Result<ConstructionAttempt> {
    let plan = sweep_miter_layout::plan(
        request.loops,
        request.points.len(),
        request.options.closed,
        request.options.initial_steps,
        request.options.max_steps,
    )?;
    let profiles = request.loops.iter().flatten().cloned().collect::<Vec<_>>();
    let rings = request.loops.iter().map(Vec::len).collect::<Vec<_>>();
    let mut options = request.options;
    options.max_steps = plan.max_steps;
    let mut sweep = Sweep::new(
        &profiles,
        request.points,
        request.scale,
        request.twist,
        options,
    )?;
    if let Some((axes, center)) = request.affine {
        sweep = sweep.with_affine_laws(axes, center)?;
    }
    if let Some((axis, normal)) = request.frames {
        sweep = sweep.with_frame_laws(axis, normal)?;
    }
    if let Some(guide) = request.guide {
        sweep = sweep.with_orientation_guide(guide)?;
    }
    let mut levels = Vec::new();
    let mut accepted = None;
    for level in sweep.by_ref() {
        let level = level?;
        if level.report.accepted {
            accepted = Some(level.sections);
        }
        levels.push(level.report);
    }
    let report = levels
        .last()
        .ok_or_else(|| invalid("Progressive miter produced no level"))?;
    if !report.frame_transport_certified {
        return Err(invalid(
            "Progressive miter frame transport could not be proved; review the path, normal and miter limit",
        ));
    }
    if !report.profile_regularity_certified {
        return Err(invalid(
            "Progressive miter profile tangent regularity could not be proved",
        ));
    }
    if report.wall_regularity_certified == Some(false) {
        return Err(invalid(
            "Progressive miter retained wall Jacobian regularity could not be proved",
        ));
    }
    let Some(source_error) = report.certified_error_upper else {
        return Err(invalid(format!(
            "Progressive miter wall error bound could not be proved: {}",
            report
                .error_certificate_reason
                .unwrap_or("unresolved certificate")
        )));
    };
    let original = accepted.ok_or_else(|| {
        invalid("Progressive miter refinement/phase budget not met within the body face budget")
    })?;
    // Explicit caps share their caller-owned correction/work allowance with
    // periodic profile preservation. Circle repair retains its existing order.
    let explicit_profile = if !options.closed && request.circle.is_none() {
        if let Some(cap) = request.caps.as_ref().filter(|c| c.budget.tolerance > 0.) {
            if let Some(work) = cap
                .budget
                .max_work
                .filter(|w| w.is_finite() && *w >= 1. && *w <= 1000000. && w.fract() == 0.)
            {
                periodic_profile_lattice(
                    &original,
                    2_f64.powi(-40),
                    cap.budget.tolerance.min(options.max_deviation).min(1e-9),
                    work as u64,
                )?
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let (profile_error, profile_work) = explicit_profile
        .as_ref()
        .map_or((0., 0), |(_, e, w)| (*e, *w));
    let correction_source = explicit_profile
        .as_ref()
        .map_or(original.as_slice(), |(s, _, _)| s.as_slice());
    let mut correction = miter_section_correction::correct(
        correction_source,
        request.points,
        options.closed,
        request.circle,
        request.caps.as_ref().map(|c| CapCorrection {
            budget: Budget {
                max_work: c.budget.max_work.map(|w| w - profile_work as f64),
                ..c.budget
            },
            authored_frame: c.authored_frame,
        }),
        request.frames.map(|(axis, _)| axis),
    )?;
    if explicit_profile.is_some() {
        let repair = correction
            .as_mut()
            .ok_or_else(|| invalid("Explicit periodic cap correction missing"))?;
        let upper = repair
            .wall_displacement_upper
            .and_then(|d| error_upper::add(d, profile_error))
            .ok_or_else(|| invalid("Explicit periodic cap displacement bound unproved"))?;
        if upper > request.caps.as_ref().unwrap().budget.tolerance {
            return Err(invalid(
                "Explicit periodic cap displacement exceeds shared budget",
            ));
        }
        repair.wall_displacement_upper = Some(upper);
        repair.work += profile_work;
    }
    if options.closed && request.circle.is_none() && request.caps.is_none() {
        if let Some((sections, displacement, work)) = periodic_profile_lattice(
            &original,
            2_f64.powi(-40),
            options.max_deviation.min(1e-9),
            limits.projection_exact_work,
        )? {
            correction = Some(nurbs_core::sweep_section_projection::Report {
                sections: Some(sections),
                wall_displacement_upper: Some(displacement),
                exact_planar_sections: Vec::new(),
                work,
                reason: "bounded-periodic-profile-interpolation",
            });
        }
    }
    if request.circle.is_none() && request.caps.is_none() && correction.is_none() {
        if let Some((sections, displacement, work)) = rational_periodic_lattice(
            &original,
            2_f64.powi(-40),
            options.max_deviation.min(1e-9),
            limits.projection_exact_work,
        )? {
            correction = Some(nurbs_core::sweep_section_projection::Report {
                sections: Some(sections),
                wall_displacement_upper: Some(displacement),
                exact_planar_sections: Vec::new(),
                work,
                reason: "bounded-rational-periodic-profile-interpolation",
            });
        }
    }
    let retained = correction
        .as_ref()
        .and_then(|c| c.sections.as_deref())
        .unwrap_or(&original);
    let mut sections = sweep_miter_layout::partition(retained, &rings)?;
    let mut model = if options.closed {
        crate::periodic_section_loft(&sections)?
    } else {
        crate::rational_section_loft(&sections)?
    };
    if !options.closed && request.caps.is_none() {
        let (needed, work) = nonplanar_caps(&model, limits.cap_regions.max_exact_work);
        if needed {
            let profile = periodic_profile_lattice(
                retained,
                2_f64.powi(-40),
                options.max_deviation.min(1e-9),
                limits.cap_regions.max_exact_work - work,
            )?;
            let (profile_error, profile_work) =
                profile.as_ref().map_or((0., 0), |(_, e, w)| (*e, *w));
            let projected_source = profile
                .as_ref()
                .map_or(retained, |(curves, _, _)| curves.as_slice());
            let mut repair = miter_section_correction::correct(
                projected_source,
                request.points,
                false,
                None,
                Some(CapCorrection {
                    budget: Budget {
                        quantum: 2_f64.powi(-40),
                        tolerance: options.max_deviation.min(1e-9),
                        max_work: Some(
                            (limits.cap_regions.max_exact_work - work - profile_work) as f64,
                        ),
                    },
                    authored_frame: request.frames.is_some(),
                }),
                request.frames.map(|(a, _)| a),
            )?
            .ok_or_else(|| invalid("Automatic cap correction result missing"))?;
            let before = correction
                .as_ref()
                .and_then(|c| c.wall_displacement_upper)
                .unwrap_or(0.);
            let combined_displacement = repair
                .wall_displacement_upper
                .and_then(|x| error_upper::add(before, x))
                .and_then(|x| error_upper::add(x, profile_error))
                .ok_or_else(|| invalid("Automatic cap correction displacement bound unproved"))?;
            repair.wall_displacement_upper = Some(combined_displacement);
            repair.work += work + profile_work + correction.as_ref().map_or(0, |c| c.work);
            repair.reason = "automatic-bounded-cap-planarity";
            sections = sweep_miter_layout::partition(repair.sections.as_deref().unwrap(), &rings)?;
            model = crate::rational_section_loft(&sections)?;
            correction = Some(repair);
        }
    }
    let displacement = correction
        .as_ref()
        .and_then(|c| c.wall_displacement_upper)
        .unwrap_or(0.);
    let correspondence = sweep_retained_walls::inspect_correspondence(
        &model,
        &sections,
        options.closed,
        limits.correspondence_faces,
        limits.exact_work,
    )?;
    let decomposition = if correspondence.certified {
        Some(0.)
    } else {
        sweep_retained_decomposition::inspect(
            &model,
            &sections,
            options.closed,
            limits.max_products,
            limits.max_faces,
            limits.exact_work,
        )?
        .wall_error_upper
    };
    if correction.is_some() && decomposition.is_none() {
        return Err(invalid(
            "Corrected progressive miter retained wall correspondence unproved",
        ));
    }
    let wall = error_upper::add(source_error, displacement)
        .zip(decomposition)
        .and_then(|(base, extra)| error_upper::add(base, extra));
    if correction.is_some() && !wall.is_some_and(|x| x <= options.max_deviation) {
        let reason="Corrected progressive miter retained wall error exceeds max_deviation or is unproved";
        if wall.is_some() {
            let steps=report.steps;
            return Ok(ConstructionAttempt::Refine {levels,steps,work:correction.as_ref().map_or(0,|c|c.work),reason});
        }
        return Err(invalid(reason));
    }
    let caps = if options.closed {
        None
    } else {
        let endpoints = [sections[0].clone(), sections.last().unwrap().clone()];
        let exact = sweep_retained_caps::inspect(
            &model,
            &endpoints,
            limits.cap_regions,
            limits.cap_max_edges,
        )?;
        let (regions, decomposition) = if exact.exact {
            (true, Some([0., 0.]))
        } else {
            let r = sweep_retained_decomposition::inspect_caps(
                &model,
                &endpoints,
                limits.cap_regions,
                limits.max_products,
                limits.cap_max_edges,
            )?;
            (r.certified, r.cap_error_upper)
        };
        if correction.is_some() && !regions {
            return Err(invalid(
                "Corrected progressive miter filled retained cap regions unproved",
            ));
        }
        let domains = sweep.certify_ideal_cap_domains(
            &rings,
            limits.domain_tolerance,
            limits.domain_pairs,
            limits.domain_cells,
            limits.domain_exact_work,
        )?;
        let planes = [
            &model.faces[model.faces.len() - 2].surface,
            &model.faces[model.faces.len() - 1].surface,
        ];
        let projection = sweep.certify_endpoint_cap_projection(
            planes,
            limits.projection_cells,
            limits.projection_exact_work,
        )?;
        let parallel = sweep.certify_endpoint_cap_parallelism(
            planes,
            limits.projection_cells,
            limits.projection_exact_work,
        )?;
        filled_cap_error::filled_caps(&Premises {
            ideal_domains_certified: domains.ideal_cap_domains_certified,
            retained_regions_exact: regions,
            projection_normal_dots: projection.normal_dots,
            endpoint_error: report.endpoint_contour_error_upper,
            correction: Some(displacement),
            decomposition,
            parallel_planes: parallel.parallel.unwrap_or([false; 2]),
        })
    };
    let boundary =
        filled_cap_error::compose_boundary(wall, caps, options.closed, Some(options.max_deviation));
    if boundary.within_budget == Some(false) {
        let steps=report.steps;
        return Ok(ConstructionAttempt::Refine {levels,steps,work:correction.as_ref().map_or(0,|c|c.work),
            reason:"Progressive miter complete boundary error exceeds max_deviation"});
    }
    let cap_faces = if options.closed {
        Vec::new()
    } else {
        vec![model.faces.len() - 2, model.faces.len() - 1]
    };
    let wall_charts = crate::sweep_retained_charts::inspect(&model, &cap_faces, limits.wall_cells)?;
    if correction.is_some() && !wall_charts.certified {
        #[cfg(all(test,feature="codec"))]
        if let Ok(path)=std::env::var("OSV_MITER_UNPROVED_MODEL") {
            std::fs::write(path,value_codec::to_string(&model).unwrap()).unwrap();
        }
        #[cfg(test)]
        eprintln!("Unproved corrected wall charts: cells={} unresolved={:?} proofs={:?}",
            wall_charts.cells, wall_charts.unresolved_faces,
            wall_charts.charts.iter().filter(|(_, r)| r.cells > 256 || !r.certified).collect::<Vec<_>>());
        return Err(invalid(
            "Corrected progressive miter retained wall regularity unproved",
        ));
    }
    let sharp = sweep_miter_layout::sharp_stations(
        sections.len(),
        plan.edges,
        report.steps,
        options.closed,
    )?;
    Ok(ConstructionAttempt::Complete(BoundaryBody {
        model,
        sections,
        source_sections: original,
        correction,
        wall_charts,
        edges: plan.edges,
        max_steps: plan.max_steps,
        levels,
        boundary,
        sharp,
        closed: options.closed,
        budget: options.max_deviation,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rational_lattice_preserves_basis_and_charges_bounded_atomic_displacement() {
        let c = |z: f64| Curve {
            degree: 2,
            knots: (0..=8).map(|i| i as f64).collect(),
            control_points: [
                [1., 0., z],
                [0., 1., z],
                [-1., 0., z],
                [0., -1., z],
                [1., 0., z],
                [0., 1., z],
            ]
            .map(Vec::from)
            .to_vec(),
            weights: vec![1., 0.5, 1., 1., 1., 0.5],
            periodic: true,
        };
        let original = vec![vec![c(0.)], vec![c(5.)], vec![c(10.)]];
        let before = original.clone();
        let q = 2_f64.powi(-40);
        let (repaired, error, work) = rational_periodic_lattice(&original, q, 1e-9, 10000)
            .unwrap()
            .unwrap();
        assert!(error > 0. && error < 1e-9);
        assert_eq!(original, before);
        for (a, b) in original.iter().flatten().zip(repaired.iter().flatten()) {
            assert_eq!(a.weights, b.weights);
            assert_eq!(a.knots, b.knots);
            assert!(
                a.control_points
                    .iter()
                    .zip(&b.control_points)
                    .all(|(a, b)| a[2] == b[2])
            );
            assert!(nurbs_core::retained_wall_coefficients::exact_bezier_controls(b, 12).is_some());
            assert!(nurbs_core::retained_wall_coefficients::exact_bezier_controls(b, 11).is_none());
        }
        assert!(
            rational_periodic_lattice(&original, q, 0., 10000)
                .unwrap()
                .is_none()
        );
        assert!(rational_periodic_lattice(&original, q, 1e-9, 0).is_err());
        assert!(rational_periodic_lattice(&original, q, 1e-9, work - 1).is_err());
        assert_eq!(
            rational_periodic_lattice(&original, q, 1e-9, work)
                .unwrap()
                .unwrap()
                .0,
            repaired
        );
        let sections = sweep_miter_layout::partition(&repaired, &[1]).unwrap();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        assert!(
            crate::sweep_smoothness::inspect_profile(&model, &caps, 2_000_000)
                .unwrap()
                .profile
                .g1_certified
        );
    }
    #[test]
    fn complete_boundary_refines_after_cap_correction_and_keeps_shared_work() {
        let outer=nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.5).unwrap();
        let hole=nurbs_core::primitives::circle([0.;3],[0.,0.,-1.],0.2).unwrap();
        let loops=vec![vec![outer],vec![hole]];
        let before=loops.clone();
        let mut scale=scalar(1.);scale.control_points[1][0]=1.25;
        let mut twist=scalar(0.);twist.control_points[1][0]=std::f64::consts::PI/12.;
        let vector=|a:[f64;3],b:[f64;3]|Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![a.to_vec(),b.to_vec()],weights:vec![1.,1.],periodic:false};
        let axis=vector([0.,0.,2.],[0.,2.,2.]);
        let normal=vector([3.,0.,0.],[3.,0.,0.]);
        let axes=vector([2.,1.,1.],[2.,1.,1.]);
        let center=vector([0.125,-0.25,0.],[0.125,-0.25,0.]);
        let mut request=Request {loops:&loops,points:&[[0.,0.,0.],[0.,0.,10.]],scale:&scale,twist:&twist,
            options:progressive_miter::Options {normal:[1.,0.,0.],closed:false,miter_limit:4.,initial_steps:1,max_steps:16,max_deviation:2.},
            affine:Some((&axes,&center)),frames:Some((&axis,&normal)),guide:None,circle:None,
            caps:Some(CapCorrection {budget:Budget {quantum:2_f64.powi(-40),tolerance:2.,max_work:Some(1000000.)},authored_frame:false})};
        let body=construct(&request,&limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert!(body.levels().len()>1);
        assert!(!body.levels()[0].accepted);
        let rejected=body.levels().iter().find(|r|r.error_certificate_reason==Some("complete-boundary-refinement")).expect("The cap budget must cause additional refinement");
        assert!(!rejected.accepted);
        assert!(body.levels().last().unwrap().steps>rejected.steps);
        assert!(body.levels().last().unwrap().accepted);
        let work=body.correction().unwrap().work;
        request.caps.as_mut().unwrap().budget.max_work=Some((work-1) as f64);
        assert!(construct(&request,&limits()).is_err());
        request.caps.as_mut().unwrap().budget.max_work=Some(1000000.);
        request.options.max_steps=1;
        assert!(construct(&request,&limits()).is_err());
        assert_eq!(loops,before);
        eprintln!("complete miter refinement levels={} last steps={} total correction work={work} error={:?}",body.levels().len(),body.levels().last().unwrap().steps,body.boundary().error_upper);
        let guide=nurbs_core::primitives::line([1.,0.,0.],[1.,0.,10.]).unwrap();
        request.guide=Some(&guide);
        request.options.max_steps=16;
        let guided=construct(&request,&limits()).unwrap();
        assert!(guided.complete_boundary().is_some());
    }

    fn limits() -> Limits {
        Limits {
            max_products: 100000,
            max_faces: 1024,
            correspondence_faces: 1024,
            cap_max_edges: 1024,
            wall_cells: 10000,
            exact_work: 1000000,
            domain_exact_work: 1000000,
            projection_exact_work: 1000000,
            domain_tolerance: 0.001,
            domain_pairs: 1000,
            domain_cells: 10000,
            projection_cells: 10000,
            cap_regions: sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: 1000000,
                max_chart_cells: 10000,
                max_trim_pairs: 10000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
        }
    }
    fn scalar(value: f64) -> Curve {
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![value, 0., 0.], vec![value, 0., 0.]],
            weights: vec![1., 1.],
            periodic: false,
        }
    }
    #[test]
    fn periodic_profile_lattice_has_bounded_atomic_displacement() {
        let curve = Curve {
            degree: 2,
            knots: vec![0., 1., 2., 3., 4., 5., 6., 7., 8.],
            control_points: vec![
                vec![0.1, 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![0.1, 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let sections = vec![vec![curve]];
        let before = sections.clone();
        let result = periodic_profile_lattice(&sections, 2_f64.powi(-40), 1e-9, 1000)
            .unwrap()
            .unwrap();
        assert!(result.1 > 0. && result.1 < 1e-9);
        assert_eq!(sections, before);
        assert!(periodic_profile_lattice(&sections, 2_f64.powi(-40), 1e-9, result.2 - 1).is_err());
        assert!(periodic_profile_lattice(&sections, 2_f64.powi(-40), 0., 1000).is_err());
        // A nearly symmetric profile must be repaired with a charged bound;
        // a genuinely asymmetric profile above retains independent rounding.
        let mut symmetric = before.clone();
        symmetric[0][0].control_points[0][0] = 1. + 1e-13;
        symmetric[0][0].control_points[4][0] = 1. + 1e-13;
        let coupled = periodic_profile_lattice(&symmetric, 2_f64.powi(-40), 1e-9, 1000)
            .unwrap()
            .unwrap();
        assert!(coupled.1 > 0. && coupled.1 < 1e-9);
        let p = &coupled.0[0][0].control_points;
        for k in 0..3 {
            assert_eq!(p[0][k] + p[2][k], p[1][k] + p[3][k]);
        }
        assert_eq!(p[0], p[4]);
        assert_eq!(p[1], p[5]);
        assert!(
            periodic_profile_lattice(&symmetric, 2_f64.powi(-40), 1e-9, coupled.2 - 1).is_err()
        );
        assert_ne!(result.0[0][0].control_points[0][0], 1.);
        let mut rational = sections;
        rational[0][0].weights[1] = 0.5;
        assert!(
            periodic_profile_lattice(&rational, 2_f64.powi(-40), 1e-9, 1000)
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn derives_owned_hollow_boundary_and_correction_from_original_request() {
        let outer = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.5).unwrap();
        let mut hole = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.2).unwrap();
        hole.control_points.reverse();
        hole.weights.reverse();
        let loops = vec![vec![outer], vec![hole]];
        let before = loops.clone();
        let scale = scalar(1.);
        let twist = scalar(0.);
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [0., 0., 10.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.01,
            },
            affine: None,
            frames: None,
            guide: None,
            circle: Some(Budget {
                quantum: 2_f64.powi(-40),
                tolerance: 1e-9,
                max_work: Some(100000.),
            }),
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert_eq!(body.model().faces.len(), 10);
        assert!(body.boundary().filled_cap_error_upper.is_some());
        let rebuilt = body
            .reconstruct_stations(2_f64.powi(-40), 1., 100000, Some(1.))
            .unwrap();
        assert_eq!(rebuilt.reason, "owned-station-complete-boundary");
        assert!(rebuilt.reconstruction.unwrap().model.is_some());
        assert_eq!(
            rebuilt.boundary.unwrap().filled_cap_error_upper,
            body.boundary().filled_cap_error_upper
        );
        let denied = body
            .reconstruct_stations(2_f64.powi(-40), 1., 100000, Some(0.))
            .unwrap();
        assert_eq!(denied.reason, "boundary-budget-unproved");
        assert!(denied.reconstruction.unwrap().model.is_none());
        let exhausted = body
            .reconstruct_stations(2_f64.powi(-40), 1., 0, Some(1.))
            .unwrap();
        assert!(exhausted.reconstruction.is_none());
        assert_eq!(loops, before);
        assert!(body.wall_charts().certified);
        let mut charts_limited = limits();
        charts_limited.wall_cells = 0;
        assert!(
            construct(&request, &charts_limited)
                .err()
                .unwrap()
                .to_string()
                .contains("retained wall regularity unproved")
        );
        assert_eq!(body.edges(), 1);
        assert_eq!(body.max_steps(), 1);
        assert!(body.correction().unwrap().wall_displacement_upper.is_some());

        // Cap audit work allowances do not change retained-wall work or each other.
        let mut domain_limited = limits();
        domain_limited.domain_exact_work = 1;
        let domain_denied = construct(&request, &domain_limited).unwrap();
        assert!(domain_denied.complete_boundary().is_none());
        assert_eq!(
            domain_denied.boundary().wall_error_upper,
            body.boundary().wall_error_upper
        );
        let mut projection_limited = limits();
        projection_limited.projection_exact_work = 1;
        let projection_denied = construct(&request, &projection_limited).unwrap();
        assert!(projection_denied.complete_boundary().is_none());
        assert_eq!(
            projection_denied.boundary().wall_error_upper,
            body.boundary().wall_error_upper
        );

        let shear = [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [1., 1., 1., 0.],
            [0., 0., 0., 1.],
        ];
        let placed = body
            .place(shear, 2_f64.powi(-40), 100000, Some(1e-9))
            .unwrap();
        assert!(placed.placement.unwrap().model.is_some());
        let denied = body
            .place(shear, 2_f64.powi(-40), 100000, Some(1e-15))
            .unwrap();
        assert!(denied.placement.unwrap().model.is_none());
        let vector = |value: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![value.to_vec(), value.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let axes = vector([2., 1., 1.]);
        let center = vector([0.125, 0.25, 0.]);
        let axis = vector([0., 0., 2.]);
        let normal = vector([3., 0., 0.]);
        let guide = nurbs_core::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
        let joint = Request {
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            loops: request.loops,
            points: request.points,
            scale: request.scale,
            twist: request.twist,
            options: request.options,
            circle: request.circle,
            caps: None,
        };
        let body = construct(&joint, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        let report = body.levels().last().unwrap();
        assert!(
            report.authored_frames_applied
                && report.orientation_guide_applied
                && report.affine_laws_applied
        );
        let denied = Request {
            circle: Some(Budget {
                max_work: Some(1.),
                ..request.circle.unwrap()
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
    #[test]
    fn owns_closed_path_and_sharp_stations_without_filled_caps() {
        let profile = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.5).unwrap();
        let loops = vec![vec![profile]];
        let scale = scalar(1.);
        let twist = scalar(0.);
        let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 0., 0.]];
        let request = Request {
            loops: &loops,
            points: &points,
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: true,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 1,
                max_deviation: 0.01,
            },
            affine: None,
            frames: None,
            guide: None,
            circle: None,
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.closed());
        assert!(body.complete_boundary().is_some());
        assert!(body.boundary().filled_cap_error_upper.is_none());
        assert_eq!(body.sharp_stations(), &[0, 1, 2, 3]);
        assert_eq!(body.model().faces.len(), 16);
        let denied = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 0.125,
                    tolerance: 0.01,
                    max_work: Some(100000.),
                },
                authored_frame: false,
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
    #[test]
    fn closed_periodic_profile_preserves_g2_without_caps() {
        let profile = |r: f64, sign: f64| Curve {
            degree: 2,
            knots: (0..9).map(|x| x as f64).collect(),
            control_points: vec![
                vec![0., r, 0.],
                vec![0., 0., sign * r],
                vec![0., -r, 0.],
                vec![0., 0., -sign * r],
                vec![0., r, 0.],
                vec![0., 0., sign * r],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let loops = vec![vec![profile(0.25, 1.)], vec![profile(0.1, -1.)]];
        let before = loops.clone();
        let vector = |values: Vec<Vec<f64>>| Curve {
            degree: 1,
            knots: vec![0., 0., 0.25, 0.5, 0.75, 1., 1.],
            control_points: values,
            weights: vec![1.; 5],
            periodic: false,
        };
        let axis = vector(vec![
            vec![1., -1., 0.],
            vec![1., 1., 0.],
            vec![-1., 1., 0.],
            vec![-1., -1., 0.],
            vec![1., -1., 0.],
        ]);
        let guide = vector(vec![
            vec![0., 0., 100.],
            vec![10., 0., 100.],
            vec![10., 10., 100.],
            vec![0., 10., 100.],
            vec![0., 0., 100.],
        ]);
        let normal = nurbs_core::progressive_sweep::constant_vector_law([0., 0., 1.]).unwrap();
        let axes = nurbs_core::progressive_sweep::constant_vector_law([2., 1., 1.]).unwrap();
        let center = nurbs_core::progressive_sweep::constant_vector_law([0.; 3]).unwrap();
        let scale = scalar(1.);
        let twist = scalar(0.);
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [0., 0., 1.],
                closed: true,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 8,
                max_deviation: 10.,
            },
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            circle: None,
            caps: None,
        };
        let body = construct(&request, &limits()).unwrap();
        assert!(body.complete_boundary().is_some());
        assert!(body.boundary().filled_cap_error_upper.is_none());
        assert!(body.wall_charts().certified);
        let report = crate::sweep_smoothness::inspect_profile(body.model(), &[], 2000000).unwrap();
        assert!(
            report.profile.g2_certified,
            "Closed actual profile G2 unproved"
        );
        assert_eq!(report.station_continuity, "C0");
        assert_eq!(
            body.correction().unwrap().reason,
            "bounded-periodic-profile-interpolation"
        );
        assert!(body.correction().unwrap().wall_displacement_upper.unwrap() <= 1e-9);
        assert_eq!(loops, before);
        let mut denied = limits();
        denied.projection_exact_work = 0;
        assert!(construct(&request, &denied).is_err());
    }

    #[test]
    fn derives_periodic_moving_frame_caps_from_original_laws() {
        let outer = Curve {
            degree: 2,
            knots: (0..9).map(|x| x as f64).collect(),
            control_points: vec![
                vec![1., 0., 0.],
                vec![0., 1., 0.],
                vec![-1., 0., 0.],
                vec![0., -1., 0.],
                vec![1., 0., 0.],
                vec![0., 1., 0.],
            ],
            weights: vec![1.; 6],
            periodic: true,
        };
        let mut hole = outer.clone();
        hole.control_points.reverse();
        for p in &mut hole.control_points {
            for x in p {
                *x *= 0.25;
            }
        }
        let loops = vec![vec![outer], vec![hole]];
        let scale = scalar(1.);
        let mut twist = scalar(0.);
        twist.control_points[1][0] = 0.25;
        let vector = |a: [f64; 3], b: [f64; 3]| Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![a.to_vec(), b.to_vec()],
            weights: vec![1., 1.],
            periodic: false,
        };
        let axis = vector([0., 0., 1.], [0., 0.5, 1.]);
        let normal = vector([3., 0., 0.], [3., 0., 0.]);
        let axes = vector([1., 1., 1.], [2., 1., 1.]);
        let center = vector([0., 0., 0.], [0., 0.125, 0.25]);
        let guide = nurbs_core::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
        let request = Request {
            loops: &loops,
            points: &[[0., 0., 0.], [0., 0., 10.]],
            scale: &scale,
            twist: &twist,
            options: progressive_miter::Options {
                normal: [1., 0., 0.],
                closed: false,
                miter_limit: 4.,
                initial_steps: 1,
                max_steps: 32,
                max_deviation: 0.01,
            },
            affine: Some((&axes, &center)),
            frames: Some((&axis, &normal)),
            guide: Some(&guide),
            circle: None,
            caps: None,
        };
        let raw = construct(&request, &limits()).unwrap();
        assert!(raw.complete_boundary().is_some());
        assert_eq!(
            raw.correction().unwrap().reason,
            "automatic-bounded-cap-planarity"
        );
        assert!(raw.boundary().error_upper.unwrap() < 0.01);
        assert!(raw.wall_charts().certified);
        let smooth = crate::sweep_smoothness::inspect_profile(
            raw.model(),
            &[raw.model().faces.len() - 2, raw.model().faces.len() - 1],
            2000000,
        )
        .unwrap();
        assert!(
            smooth.profile.g1_certified,
            "Periodic profile G1 not preserved"
        );
        assert!(
            smooth.profile.g2_certified,
            "Periodic profile G2 not preserved"
        );
        let mut no_repair_work = limits();
        no_repair_work.cap_regions.max_exact_work = 0;
        let unproved = construct(&request, &no_repair_work).unwrap();
        assert!(unproved.complete_boundary().is_none());
        let mut no_wall_work = limits();
        no_wall_work.wall_cells = 0;
        assert!(construct(&request, &no_wall_work).is_err());
        assert!(raw.boundary().filled_cap_error_upper.is_some());
        let request = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    quantum: 2_f64.powi(-40),
                    tolerance: 1e-9,
                    max_work: Some(1000000.),
                },
                authored_frame: true,
            }),
            ..request
        };
        let owned = construct(&request, &limits()).unwrap();
        assert!(owned.complete_boundary().is_some());
        assert_eq!(owned.levels().last().unwrap().steps, 16);
        assert!(owned.boundary().error_upper.unwrap() < 0.01);
        let smooth = crate::sweep_smoothness::inspect_profile(
            owned.model(),
            &[owned.model().faces.len() - 2, owned.model().faces.len() - 1],
            2000000,
        )
        .unwrap();
        assert!(
            smooth.profile.g2_certified,
            "Explicit authored cap profile G2 unproved"
        );
        assert!(owned.correction().unwrap().wall_displacement_upper.unwrap() <= 1e-9);
        let denied = Request {
            caps: Some(CapCorrection {
                budget: Budget {
                    max_work: Some(1.),
                    ..request.caps.as_ref().unwrap().budget
                },
                authored_frame: true,
            }),
            ..request
        };
        assert!(construct(&denied, &limits()).is_err());
    }
}
