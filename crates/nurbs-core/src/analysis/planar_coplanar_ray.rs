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

#[derive(Clone, Copy, Debug)]
pub struct CoplanarRayPartitionOptions {
    pub max_curve_parameter_width: f64,
    pub winding_tolerance: f64,
    pub max_winding_cells: usize,
    pub max_work: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarRayGap {
    /// Open interval in the original ray parameter, disjoint from all contacts.
    pub parameter: [Q; 2],
    pub location: crate::trim::trim_domain::Location,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarRayPartition {
    /// Nonnegative, ordered, disjoint enclosures of all boundary contacts.
    pub contact_bands: Vec<[Q; 2]>,
    pub gaps: Vec<CoplanarRayGap>,
    /// Every parameter strictly greater than this is outside.
    pub outside_after: Q,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoplanarRayPartitionOutcome {
    Partition(CoplanarRayPartition),
    NotCoplanar,
    Refused(Refusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CoplanarRayPartitionReport {
    pub outcome: CoplanarRayPartitionOutcome,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Certify contact-free coplanar ray intervals as inside or outside the
/// nonzero-winding region. Overlapping contact enclosures, including
/// coincident spans, become one band. Unresolved winding stays explicit
/// on a gap. The unbounded ray tail is outside after the last contact band.
pub fn partition_coplanar_face_ray(
    face: &ProjectedTrimmedFace,
    origin: [f64; 3],
    direction: [f64; 3],
    options: CoplanarRayPartitionOptions,
) -> Result<CoplanarRayPartitionReport> {
    check(
        (1..=100_000).contains(&options.max_winding_cells)
            && options.winding_tolerance.is_finite()
            && options.winding_tolerance > 0.,
        "Coplanar ray partition requires a positive winding tolerance and bounded winding work",
    )?;
    let contacts = contact_coplanar_face_ray(
        face,
        origin,
        direction,
        options.max_curve_parameter_width,
        options.max_work,
    )?;
    let mut work = Work {
        used: contacts.work_used,
        limit: options.max_work,
    };
    let mut winding_cells = 0;
    let outcome = match contacts.outcome {
        CoplanarRayOutcome::Contacts(events) => {
            let domain =
                crate::trim::trim_domain::TrimDomain::new(&face.loops, options.winding_tolerance)?;
            let axis = face.projection.retained_axes;
            let projected = [axis.map(|i| origin[i]), axis.map(|i| direction[i])];
            let bands = merge_contact_bands(events);
            partition_gaps(
                bands,
                &domain,
                projected,
                options.max_winding_cells,
                &mut work,
                &mut winding_cells,
            )?
        }
        CoplanarRayOutcome::NotCoplanar => CoplanarRayPartitionOutcome::NotCoplanar,
        CoplanarRayOutcome::Refused(reason) => CoplanarRayPartitionOutcome::Refused(reason),
    };
    Ok(CoplanarRayPartitionReport {
        outcome,
        work_used: work.used,
        winding_cells,
    })
}

fn merge_contact_bands(events: Vec<CoplanarRayContact>) -> Vec<[Q; 2]> {
    use num_traits::Zero;
    let mut intervals: Vec<_> = events
        .into_iter()
        .map(|event| {
            let [lo, hi] = event.ray_parameter;
            [lo.max(Q::zero()), hi]
        })
        .collect();
    intervals.sort_unstable();
    let mut bands: Vec<[Q; 2]> = Vec::new();
    for [lo, hi] in intervals {
        if let Some(last) = bands.last_mut()
            && lo <= last[1]
        {
            if hi > last[1] {
                last[1] = hi;
            }
        } else {
            bands.push([lo, hi]);
        }
    }
    bands
}

fn partition_gaps(
    contact_bands: Vec<[Q; 2]>,
    domain: &crate::trim::trim_domain::TrimDomain,
    projected_ray: [[f64; 2]; 2],
    max_winding_cells: usize,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<CoplanarRayPartitionOutcome> {
    use crate::trim::trim_domain::Location;
    use num_traits::Zero;
    let mut gaps = Vec::new();
    let mut previous = Q::zero();
    for [lo, hi] in &contact_bands {
        if &previous < lo {
            let mut location = Location::Unresolved;
            let remaining = max_winding_cells - *winding_cells;
            if remaining > 0 {
                let rectangle = match gap_midpoint(projected_ray, &previous, lo, work) {
                    Ok(rectangle) => rectangle,
                    Err(reason) => return Ok(CoplanarRayPartitionOutcome::Refused(reason)),
                };
                if let Some(rectangle) = rectangle {
                    let classification = domain.classify(rectangle, remaining)?;
                    *winding_cells += classification.cells;
                    location = classification.location;
                } else {
                    location = Location::Outside;
                }
            }
            gaps.push(CoplanarRayGap {
                parameter: [previous, lo.clone()],
                location,
            });
        }
        previous = hi.clone();
    }
    Ok(CoplanarRayPartitionOutcome::Partition(
        CoplanarRayPartition {
            contact_bands,
            gaps,
            outside_after: previous,
        },
    ))
}

fn gap_midpoint(
    [origin, direction]: [[f64; 2]; 2],
    lo: &Q,
    hi: &Q,
    work: &mut Work,
) -> std::result::Result<Option<[[f64; 2]; 2]>, Refusal> {
    let parameter = work.op(lo, hi, |a, b| (a + b) / Q::from_integer(2.into()))?;
    let cap = Q::from_integer(1_000_000_000.into());
    let mut result = [[0.; 2]; 2];
    for i in 0..2 {
        let origin = work.lift(origin[i])?;
        let direction = work.lift(direction[i])?;
        let offset = work.op(&direction, &parameter, |a, b| a * b)?;
        let point = work.op(&origin, &offset, |a, b| a + b)?;
        if point.abs() > cap {
            return Ok(None);
        }
        let interval = super::planar_face_area::rational_interval(&point)
            .map_err(|_| Refusal::NotRepresentable)?;
        result[i] = [interval.lo, interval.hi];
    }
    Ok(Some(result))
}
