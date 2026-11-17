//! Certified contour contacts of a ray lying in a planar rational face.
use super::{
    exact_affine_hull::validate_projection_axes,
    planar_face_ray::{PlaneIntersection, intersect_plane},
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve_distance::enclosure,
    distance_bounds::Interval,
    editing::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveSeamIsolation, pcurve_seam_isolation::isolate_rational_pcurve_line,
        },
    },
    trim::trim_domain::exact_loop_joins,
};
use num_rational::BigRational as Q;
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoplanarRayContactKind {
    Crossing,
    Tangency,
    Endpoint,
    CoincidentSpan,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarRayContact {
    pub contour: usize,
    pub curve: usize,
    pub curve_parameter: [f64; 2],
    /// Enclosure in the original ray clock. For a coincident span it
    /// encloses the entire span image. An interval spanning zero leaves
    /// the contact's relative position to the ray origin unresolved.
    pub ray_parameter: [Q; 2],
    pub kind: CoplanarRayContactKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoplanarRayOutcome {
    /// Contacts in source traversal order, with one-sided endpoint ownership.
    Contacts(Vec<CoplanarRayContact>),
    NotCoplanar,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarRayReport {
    pub outcome: CoplanarRayOutcome,
    pub work_used: u64,
}

/// Locate boundary contacts of a coplanar ray over all authored contours.
/// Line coefficients are formed exactly from the ray. Entirely rearward
/// contacts are discarded. Boundary coincidences and origin-straddling
/// uncertainty are retained for subsequent winding operations.
pub fn contact_coplanar_face_ray(
    face: &ProjectedTrimmedFace,
    origin: [f64; 3],
    direction: [f64; 3],
    max_width: f64,
    max_work: u64,
) -> Result<CoplanarRayReport> {
    validate_projection_axes(&face.projection)?;
    check(
        origin.iter().chain(&direction).all(|x| x.is_finite())
            && direction.iter().any(|&d| d != 0.)
            && max_width.is_finite()
            && max_width > 0.
            && (1..=cad_predicates::MAX_WORK).contains(&max_work)
            && (1..=256).contains(&face.loops.len()),
        "Coplanar ray contact needs a finite nonzero ray, positive width and bounded contours",
    )?;
    for contour in &face.loops {
        check(
            exact_loop_joins(contour)? == Some(true),
            "Coplanar ray needs exact closed contours",
        )?;
    }
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let outcome = match intersect_plane(face, origin, direction, &mut work) {
        Ok(PlaneIntersection::Coplanar) => {
            match RayClock::new(face.projection.retained_axes, origin, direction, &mut work) {
                Ok(clock) => contacts(face, clock, max_width, &mut work)?,
                Err(reason) => CoplanarRayOutcome::Refused(reason),
            }
        }
        Ok(_) => CoplanarRayOutcome::NotCoplanar,
        Err(reason) => CoplanarRayOutcome::Refused(reason.into()),
    };
    Ok(CoplanarRayReport {
        outcome,
        work_used: work.used,
    })
}

struct RayClock {
    equation: [Q; 3],
    axis: usize,
    origin: Q,
    step: Q,
}

impl RayClock {
    fn new(
        axes: [usize; 2],
        origin: [f64; 3],
        direction: [f64; 3],
        work: &mut Work,
    ) -> std::result::Result<Self, Refusal> {
        let [x, y] = axes;
        let [u, v] = [work.lift(origin[x])?, work.lift(origin[y])?];
        let [du, dv] = [work.lift(direction[x])?, work.lift(direction[y])?];
        let a = -&dv;
        let p = work.op(&u, &dv, |a, b| a * b)?;
        let q = work.op(&v, &du, |a, b| a * b)?;
        let c = work.op(&p, &q, |a, b| a - b)?;
        let axis = usize::from(direction[y].abs() > direction[x].abs());
        let (origin, step) = if axis == 0 { (u, du.clone()) } else { (v, dv) };
        Ok(Self {
            equation: [a, du, c],
            axis,
            origin,
            step,
        })
    }

    fn parameter(
        &self,
        interval: Interval,
        work: &mut Work,
    ) -> std::result::Result<[Q; 2], Refusal> {
        let map = |p, work: &mut Work| {
            let p = work.lift(p)?;
            let delta = work.op(&p, &self.origin, |a, b| a - b)?;
            work.op(&delta, &self.step, |a, b| a / b)
        };
        let mut range = [map(interval.lo, work)?, map(interval.hi, work)?];
        if self.step.is_negative() {
            range.swap(0, 1);
        }
        Ok(range)
    }
}

fn contacts(
    face: &ProjectedTrimmedFace,
    clock: RayClock,
    width: f64,
    work: &mut Work,
) -> Result<CoplanarRayOutcome> {
    let mut out = Vec::new();
    for (contour, curves) in face.loops.iter().enumerate() {
        for (curve, input) in curves.iter().enumerate() {
            let isolated = match isolate_rational_pcurve_line(input, &clock.equation, width, work) {
                Ok(isolated) => isolated,
                Err(reason) => return Ok(CoplanarRayOutcome::Refused(reason)),
            };
            let spans = (input.degree..input.control_points.len())
                .filter(|&k| input.knots[k] < input.knots[k + 1]);
            use CoplanarRayContactKind::{CoincidentSpan, Crossing, Endpoint, Tangency};
            for (span, isolated) in spans.zip(isolated) {
                let [a, b] = isolated.parameter;
                let events =
                    match isolated.contact {
                        PcurveSeamIsolation::Coincident => vec![([a, b], CoincidentSpan)],
                        PcurveSeamIsolation::Discrete {
                            at_start,
                            at_end,
                            roots,
                        } => {
                            let mut events = Vec::new();
                            if at_start {
                                events.push(([a; 2], Endpoint));
                            }
                            events.extend(roots.into_iter().map(|r| {
                                (r.parameter, if r.crosses { Crossing } else { Tangency })
                            }));
                            if at_end {
                                events.push(([b; 2], Endpoint));
                            }
                            events
                        }
                    };
                for (curve_parameter, kind) in events {
                    let point = enclosure(
                        input,
                        span,
                        Interval::new(curve_parameter[0], curve_parameter[1])?,
                    )?;
                    let ray_parameter = match clock.parameter(point[clock.axis], work) {
                        Ok(parameter) => parameter,
                        Err(reason) => return Ok(CoplanarRayOutcome::Refused(reason)),
                    };
                    if !ray_parameter[1].is_negative() {
                        out.push(CoplanarRayContact {
                            contour,
                            curve,
                            curve_parameter,
                            ray_parameter,
                            kind,
                        });
                    }
                }
            }
        }
    }
    Ok(CoplanarRayOutcome::Contacts(out))
}
