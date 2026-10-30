//! One-sided shape enclosure on one literal original knot rectangle.
use super::*;

/// Bound one original nonzero span, including its one-sided endpoints.
/// The span indices identify the side explicitly. No cropping, refitting or
/// continuity across another span is inferred. This is not a G2 join proof.
pub fn span_rectangle_bounds(
    surface: &Surface,
    spans: [usize; 2],
    query: [[f64; 2]; 2],
) -> Result<RegionReport> {
    surface.validate()?;
    crate::check(
        !surface.periodic_u && !surface.periodic_v,
        "One-sided shape rectangle requires a nonperiodic original chart",
    )?;
    for (axis, (degree, knots, n)) in [
        (
            surface.degree_u,
            &surface.knots_u,
            surface.control_points.len(),
        ),
        (
            surface.degree_v,
            &surface.knots_v,
            surface.control_points[0].len(),
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let span = spans[axis];
        crate::check(
            degree <= span && span < n,
            "Shape span index lies outside the original active basis",
        )?;
        crate::check(
            knots[span] < knots[span + 1]
                && query[axis].iter().all(|x| x.is_finite())
                && knots[span] <= query[axis][0]
                && query[axis][0] <= query[axis][1]
                && query[axis][1] <= knots[span + 1],
            "One-sided shape query must lie in its original nonzero span",
        )?;
    }
    // Midpoint normals only propose a conditioning frame. Original interval
    // jets and the full projected Jacobian independently verify that frame.
    let mid = query.map(|r| r[0] * 0.5 + r[1] * 0.5);
    let proposal = from_jets(&jets::calculate_partial_stable(
        surface,
        spans,
        mid.map(|x| [x, x]),
        [None; 2],
    )?)?;
    let frame = proposal.and_then(|shape| {
        let normal = shape.normal.map(|x| x.lo * 0.5 + x.hi * 0.5);
        let z = (0..3).max_by(|&a, &b| normal[a].abs().total_cmp(&normal[b].abs()))?;
        if normal[z] == 0. {
            return None;
        }
        let axes = [(z + 1) % 3, (z + 2) % 3, z];
        let plane = [-normal[axes[0]] / normal[z], -normal[axes[1]] / normal[z]];
        plane.iter().all(|x| x.is_finite()).then_some((axes, plane))
    });
    let graph = frame
        .map(|f| jets::calculate_partial_stable_graph(surface, spans, query, [None; 2], f))
        .transpose()?;
    let proposed = match (frame, graph.as_ref()) {
        (Some(f), Some(j)) => from_graph_jets(j, f)?,
        _ => None,
    };
    let shape = match proposed {
        Some(s) => Some(s),
        None => from_jets(&jets::calculate_partial_stable(
            surface, spans, query, [None; 2],
        )?)?,
    };
    let shape = match shape {
        Some(s) => Report {
            status: Status::Available,
            normal: Some(s.normal.map(|x| [x.lo, x.hi])),
            tensor: Some(s.tensor.map(|r| r.map(|x| [x.lo, x.hi]))),
        },
        None => Report {
            status: Status::RegularityNotProven,
            normal: None,
            tensor: None,
        },
    };
    Ok(RegionReport { shape, cells: 1 })
}

#[cfg(test)]
#[path = "span/tests.rs"]
mod tests;
