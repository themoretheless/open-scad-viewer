//! Certified contour contacts of a ray lying in a planar rational face.
use super::{
    exact_affine_hull::validate_projection_axes,
    exact_plane_intersection::ExactSupportLine,
    planar_face_ray::{PlaneIntersection, intersect_plane},
    planar_trim_projection::ProjectedTrimmedFace,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve_distance::enclosure,
    distance_bounds::Interval,
    exact_geometry::{
        exact_derivative::Refusal,
        exact_projective::{
            PcurveSeamIsolation,
            pcurve_seam_isolation::{
                AlgebraicBoundaryClock, SpanLineBounds, SpanLineCoverage, exact_span_line_ranges,
                isolate_rational_pcurve_line, quadratic_span_contacts, span_line_bounds,
            },
        },
    },
    trim::trim_domain::exact_loop_joins,
};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};

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
                Ok(clock) => contacts(face, clock, max_width, true, &mut work)?,
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
        Self::from_exact([u, v], [du, dv], work)
    }

    fn from_exact(
        [u, v]: [Q; 2],
        [du, dv]: [Q; 2],
        work: &mut Work,
    ) -> std::result::Result<Self, Refusal> {
        let a = -&dv;
        let p = work.op(&u, &dv, |a, b| a * b)?;
        let q = work.op(&v, &du, |a, b| a * b)?;
        let c = work.op(&p, &q, |a, b| a - b)?;
        let axis = usize::from(dv.abs() > du.abs());
        let (origin, step) = if axis == 0 { (u, du.clone()) } else { (v, dv) };
        Ok(Self {
            equation: [a, du, c],
            axis,
            origin,
            step,
        })
    }

    fn exact_parameter(&self, p: &Q, work: &mut Work) -> std::result::Result<Q, Refusal> {
        let delta = work.op(p, &self.origin, |a, b| a - b)?;
        Ok(work.op(&delta, &self.step, |a, b| a / b)?)
    }

    fn exact_range(&self, [a, b]: [Q; 2], work: &mut Work) -> std::result::Result<[Q; 2], Refusal> {
        let a = self.exact_parameter(&a, work)?;
        let b = self.exact_parameter(&b, work)?;
        Ok(if a <= b { [a, b] } else { [b, a] })
    }

    fn parameter(
        &self,
        interval: Interval,
        work: &mut Work,
    ) -> std::result::Result<[Q; 2], Refusal> {
        let map = |p, work: &mut Work| {
            let p = work.lift(p)?;
            self.exact_parameter(&p, work)
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
    forward_only: bool,
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
                    if !forward_only || !ray_parameter[1].is_negative() {
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
    validate_partition_options(options)?;
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
            let [u, v] = face.projection.retained_axes;
            let projected = (|| -> std::result::Result<_, Refusal> {
                Ok([
                    [work.lift(origin[u])?, work.lift(origin[v])?],
                    [work.lift(direction[u])?, work.lift(direction[v])?],
                ])
            })();
            match projected {
                Ok(projected) => partition_gaps(
                    contact_bands(events, true),
                    &domain,
                    &projected,
                    Q::zero(),
                    options.max_winding_cells,
                    &mut work,
                    &mut winding_cells,
                )?,
                Err(reason) => CoplanarRayPartitionOutcome::Refused(reason),
            }
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

pub(super) fn validate_partition_options(options: CoplanarRayPartitionOptions) -> Result<()> {
    check(
        (1..=100_000).contains(&options.max_winding_cells)
            && options.winding_tolerance.is_finite()
            && options.winding_tolerance > 0.
            && options.max_curve_parameter_width.is_finite()
            && options.max_curve_parameter_width > 0.,
        "Coplanar ray partition requires a positive winding tolerance and bounded winding work",
    )
}

pub(super) fn partition_support_line(
    face: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    options: CoplanarRayPartitionOptions,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<CoplanarRayPartitionOutcome> {
    validate_closed_contours(face)?;
    let domain = crate::trim::trim_domain::TrimDomain::new(&face.loops, options.winding_tolerance)?;
    let axes = face.projection.retained_axes;
    let projected = [
        axes.map(|i| line.origin[i].clone()),
        axes.map(|i| line.direction[i].clone()),
    ];
    let clock = match RayClock::from_exact(projected[0].clone(), projected[1].clone(), work) {
        Ok(clock) => clock,
        Err(reason) => return Ok(CoplanarRayPartitionOutcome::Refused(reason)),
    };
    match contacts(face, clock, options.max_curve_parameter_width, false, work)? {
        CoplanarRayOutcome::Contacts(events) => {
            let bands = contact_bands(events, false);
            let start = bands.first().map_or_else(Q::zero, |band| band[0].clone());
            partition_gaps(
                bands,
                &domain,
                &projected,
                start,
                options.max_winding_cells,
                work,
                winding_cells,
            )
        }
        CoplanarRayOutcome::NotCoplanar => Ok(CoplanarRayPartitionOutcome::NotCoplanar),
        CoplanarRayOutcome::Refused(reason) => Ok(CoplanarRayPartitionOutcome::Refused(reason)),
    }
}

fn contact_bands(events: Vec<CoplanarRayContact>, forward_only: bool) -> Vec<[Q; 2]> {
    merge_bands(
        events
            .into_iter()
            .map(|event| {
                let [lo, hi] = event.ray_parameter;
                [if forward_only { lo.max(Q::zero()) } else { lo }, hi]
            })
            .collect(),
    )
}

pub(super) fn merge_bands(mut intervals: Vec<[Q; 2]>) -> Vec<[Q; 2]> {
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
    projected_ray: &[[Q; 2]; 2],
    mut previous: Q,
    max_winding_cells: usize,
    work: &mut Work,
    winding_cells: &mut usize,
) -> Result<CoplanarRayPartitionOutcome> {
    use crate::trim::trim_domain::Location;
    let mut gaps = Vec::new();
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
    projected: &[[Q; 2]; 2],
    lo: &Q,
    hi: &Q,
    work: &mut Work,
) -> std::result::Result<Option<[[f64; 2]; 2]>, Refusal> {
    let parameter = work.op(lo, hi, |a, b| (a + b) / Q::from_integer(2.into()))?;
    section_sample_rectangle(projected, &parameter, work)
}

pub(super) fn section_sample_rectangle(
    [origin, direction]: &[[Q; 2]; 2],
    parameter: &Q,
    work: &mut Work,
) -> std::result::Result<Option<[[f64; 2]; 2]>, Refusal> {
    let cap = Q::from_integer(1_000_000_000.into());
    let mut result = [[0.; 2]; 2];
    for i in 0..2 {
        let offset = work.op(&direction[i], parameter, |a, b| a * b)?;
        let point = work.op(&origin[i], &offset, |a, b| a + b)?;
        if point.abs() > cap {
            return Ok(None);
        }
        let interval = super::planar_face_area::rational_interval(&point)
            .map_err(|_| Refusal::NotRepresentable)?;
        result[i] = [interval.lo, interval.hi];
    }
    Ok(Some(result))
}

fn validate_closed_contours(face: &ProjectedTrimmedFace) -> Result<()> {
    check(
        (1..=256).contains(&face.loops.len()),
        "Support line needs bounded contours",
    )?;
    for contour in &face.loops {
        check(
            exact_loop_joins(contour)? == Some(true),
            "Support line needs exact closed contours",
        )?;
    }
    Ok(())
}

pub(super) fn exact_boundary_ranges(
    face: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    coverage: SpanLineCoverage,
    work: &mut Work,
) -> Result<std::result::Result<Vec<[Q; 2]>, Refusal>> {
    validate_closed_contours(face)?;
    let axes = face.projection.retained_axes;
    Ok((|| {
        let clock = RayClock::from_exact(
            axes.map(|i| line.origin[i].clone()),
            axes.map(|i| line.direction[i].clone()),
            work,
        )?;
        let mut ranges = Vec::new();
        for curve in face.loops.iter().flatten() {
            for range in exact_span_line_ranges(curve, &clock.equation, clock.axis, coverage, work)?
            {
                ranges.push(clock.exact_range(range, work)?);
            }
        }
        Ok(merge_bands(ranges))
    })())
}

pub(super) fn bound_boundary_ranges(
    face: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    width: f64,
    work: &mut Work,
) -> Result<std::result::Result<SpanLineBounds, Refusal>> {
    validate_closed_contours(face)?;
    let axes = face.projection.retained_axes;
    Ok((|| {
        let clock = RayClock::from_exact(
            axes.map(|i| line.origin[i].clone()),
            axes.map(|i| line.direction[i].clone()),
            work,
        )?;
        let mut output = SpanLineBounds::default();
        for curve in face.loops.iter().flatten() {
            let bounds = span_line_bounds(curve, &clock.equation, clock.axis, width, work)?;
            for (source, target) in [
                (bounds.proven, &mut output.proven),
                (bounds.possible, &mut output.possible),
            ] {
                for range in source {
                    target.push(clock.exact_range(range, work)?);
                }
            }
        }
        Ok(SpanLineBounds {
            proven: merge_bands(output.proven),
            possible: merge_bands(output.possible),
        })
    })())
}

pub(super) fn algebraic_boundary_clocks(
    face: &ProjectedTrimmedFace,
    line: &ExactSupportLine,
    complete: bool,
    work: &mut Work,
) -> Result<std::result::Result<Vec<AlgebraicBoundaryClock>, Refusal>> {
    validate_closed_contours(face)?;
    let axes = face.projection.retained_axes;
    Ok((|| {
        let clock = RayClock::from_exact(
            axes.map(|i| line.origin[i].clone()),
            axes.map(|i| line.direction[i].clone()),
            work,
        )?;
        let mut points = Vec::new();
        for curve in face.loops.iter().flatten() {
            points.extend(quadratic_span_contacts(
                curve,
                &clock.equation,
                clock.axis,
                &clock.origin,
                &clock.step,
                complete,
                work,
            )?);
        }
        points.sort_unstable();
        points.dedup();
        Ok(points)
    })())
}
