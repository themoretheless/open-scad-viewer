//! Joint storage of the ordered material intervals along a support line.
use super::{
    Candidate, Curve, PlanarSegmentError, PlanarSurfaceProjection, Q, Refusal, Work, candidates,
    curves, preserves_order, segment_error,
};
use crate::{
    Result,
    analysis::exact_plane_intersection::{
        CoplanarRayPartitionOptions, ExactPlanarIntersectionSegment, ExactSupportLine,
        PlanarIntersectionSegments, ProjectedTrimmedFace, build_planar_intersection_segments,
    },
    check,
};

#[derive(Clone, Debug)]
pub struct StoredPlanarIntersectionEdge {
    pub parameter: [Q; 2],
    pub spatial: Curve,
    pub pcurves: [Curve; 2],
    pub squared_error: PlanarSegmentError,
}

#[derive(Clone, Debug)]
pub enum StoredPlanarIntersection {
    Transverse {
        line: ExactSupportLine,
        edges: Vec<StoredPlanarIntersectionEdge>,
        /// Coverage from the exact trim calculation in the original support clock.
        possible: Vec<[Q; 2]>,
    },
    Parallel,
    Coincident,
    /// No tested endpoint sequence satisfies both tolerance and separation constraints.
    NoAdmissibleStorage,
    Refused(Refusal),
}

#[derive(Clone, Debug)]
pub struct StoredPlanarIntersectionReport {
    pub outcome: StoredPlanarIntersection,
    pub work_used: u64,
    pub winding_cells: usize,
}

/// Clip and store all certified material segments atomically. Adjacent
/// binary64 endpoint choices must retain a strict gap between successive
/// edges along the canonical axis and in both projected charts.
pub fn store_trimmed_planar_intersection(
    left: &ProjectedTrimmedFace,
    right: &ProjectedTrimmedFace,
    options: CoplanarRayPartitionOptions,
    xyz_tolerance: f64,
) -> Result<StoredPlanarIntersectionReport> {
    check(
        xyz_tolerance.is_finite() && xyz_tolerance >= 0.,
        "Intersection storage requires a finite nonnegative XYZ tolerance",
    )?;
    let exact = build_planar_intersection_segments(left, right, options)?;
    let mut work = Work {
        used: exact.work_used,
        limit: options.max_work,
    };
    let outcome = match exact.outcome {
        PlanarIntersectionSegments::Transverse {
            line,
            segments,
            possible,
        } => {
            let projections = [&left.projection, &right.projection];
            match store_ordered(&segments, &line, projections, xyz_tolerance, &mut work) {
                Ok(Some(edges)) => StoredPlanarIntersection::Transverse {
                    line,
                    edges,
                    possible,
                },
                Ok(None) => StoredPlanarIntersection::NoAdmissibleStorage,
                Err(reason) => StoredPlanarIntersection::Refused(reason),
            }
        }
        PlanarIntersectionSegments::Parallel => StoredPlanarIntersection::Parallel,
        PlanarIntersectionSegments::Coincident => StoredPlanarIntersection::Coincident,
        PlanarIntersectionSegments::Refused(reason) => StoredPlanarIntersection::Refused(reason),
    };
    Ok(StoredPlanarIntersectionReport {
        outcome,
        work_used: work.used,
        winding_cells: exact.winding_cells,
    })
}

struct Choice {
    start: usize,
    previous: Option<usize>,
}

struct Layer {
    starts: Vec<Candidate>,
    ends: Vec<Candidate>,
    choices: Vec<Option<Choice>>,
}

fn store_ordered(
    segments: &[ExactPlanarIntersectionSegment],
    line: &ExactSupportLine,
    projections: [&PlanarSurfaceProjection; 2],
    tolerance: f64,
    work: &mut Work,
) -> std::result::Result<Option<Vec<StoredPlanarIntersectionEdge>>, Refusal> {
    if segments.is_empty() {
        return Ok(Some(Vec::new()));
    }
    let tolerance = work.lift(tolerance)?;
    let limit = work.op(&tolerance, &tolerance, |a, b| a * b)?;
    let ordering = std::array::from_fn(|i| line.direction[i].cmp(&Q::default()).reverse());
    let axis = Some(line.clock_axis);
    let mut layers: Vec<Layer> = Vec::with_capacity(segments.len());
    for segment in segments {
        let mut starts = candidates(segment, 0, projections, true, work)?;
        let mut ends = candidates(segment, 1, projections, true, work)?;
        starts.retain(|p| p.error.worst() <= &limit);
        ends.retain(|p| p.error.worst() <= &limit);
        let mut choices: Vec<Option<Choice>> = (0..ends.len()).map(|_| None).collect();
        for (start_index, start) in starts.iter().enumerate() {
            let previous = if let Some(layer) = layers.last() {
                let mut reachable = None;
                for (i, (end, path)) in layer.ends.iter().zip(&layer.choices).enumerate() {
                    work.charge(1)?;
                    if path.is_some() && preserves_order(end, start, ordering, projections, axis) {
                        reachable = Some(i);
                        break;
                    }
                }
                if reachable.is_none() {
                    continue;
                }
                reachable
            } else {
                None
            };
            for (end, choice) in ends.iter().zip(&mut choices) {
                work.charge(1)?;
                if choice.is_none() && preserves_order(start, end, ordering, projections, axis) {
                    *choice = Some(Choice {
                        start: start_index,
                        previous,
                    });
                }
            }
        }
        if choices.iter().all(Option::is_none) {
            return Ok(None);
        }
        layers.push(Layer {
            starts,
            ends,
            choices,
        });
    }
    let mut next = layers
        .last()
        .and_then(|layer| layer.choices.iter().position(Option::is_some));
    let mut edges = Vec::with_capacity(segments.len());
    for (layer, segment) in layers.into_iter().zip(segments).rev() {
        let index = next.ok_or(Refusal::InvalidOutput)?;
        let choice = layer.choices[index]
            .as_ref()
            .ok_or(Refusal::InvalidOutput)?;
        let start = &layer.starts[choice.start];
        let end = &layer.ends[index];
        let (spatial, pcurves) = curves([start.coordinates, end.coordinates], projections)?;
        edges.push(StoredPlanarIntersectionEdge {
            parameter: segment.parameter.clone(),
            spatial,
            pcurves,
            squared_error: segment_error(start, end),
        });
        next = choice.previous;
    }
    edges.reverse();
    Ok(Some(edges))
}
