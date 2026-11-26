//! World-space error for a certified curvature contour's stored polyline.
use super::{
    curvature_gap_graph::CurvatureGapGraph,
    curvature_gap_polyline::{CurvatureGapPolyline, certify_curvature_gap_polyline},
    curvature_gap_slope::curvature_gap_graph_slope,
    *,
};
use crate::{
    analysis::surface_measure::jets,
    distance_bounds::{Interval as I, box_distance},
    numerics::interval_vec3::norm,
};

#[derive(Clone, Debug)]
pub struct CurvatureGapWorldPolyline {
    pub uv: CurvatureGapPolyline,
    pub points: Option<Vec<[f64; 3]>>,
    /// Euclidean model-space error for each stored chord at matching transverse
    /// coordinates, including between its vertices.
    pub world_error_upper: Option<Vec<f64>>,
}

struct Vertex {
    uv: [f64; 2],
    point: [f64; 3],
    root_rounding: f64,
    world_rounding: f64,
}

/// Lift a verified UV contour polyline to a stored 3D polyline. The bound
/// includes root-bracket error, UV mapping roundoff, endpoint storage, and
/// interpolation curvature of the original rational patch.
pub fn certify_curvature_gap_world_polyline(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapWorldPolyline> {
    let uv =
        certify_curvature_gap_polyline(source, span, axis, level, transverse, max_depth, max_work)?;
    let (Some(samples), Some(uv_errors)) = (&uv.samples, &uv.segment_error_upper) else {
        return Ok(CurvatureGapWorldPolyline {
            uv,
            points: None,
            world_error_upper: None,
        });
    };
    let domain = uv.domain;
    let root = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let widths = [difference(domain[0])?, difference(domain[1])?];
    let j = jets::calculate(source, span, domain, None)?;
    let root_norm = if root == 0 {
        norm(j[1][0])?
    } else {
        norm(j[0][1])?
    };
    let [uu, uv_norm, vv] = [norm(j[2][0])?, norm(j[1][1])?, norm(j[0][2])?];

    let vertices = samples
        .iter()
        .map(|sample| {
            let unit = sample.root_interval[0].midpoint(sample.root_interval[1]);
            let [a, b] = domain[root];
            let absolute = (a + unit * (b - a)).clamp(a, b);
            let enclosure = I::point(a).add(I::point(unit).mul(widths[root])?)?;
            let root_rounding = max_abs(enclosure.sub(I::point(absolute))?);
            let mut uv = [sample.transverse; 2];
            uv[root] = absolute;
            let (point, world_rounding) = surface_vertex(source, span, domain, uv)?;
            Ok(Vertex {
                uv,
                point,
                root_rounding,
                world_rounding,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let mut errors = Vec::with_capacity(uv_errors.len());
    for (pair, &uv_error) in vertices.windows(2).zip(uv_errors) {
        let delta = |axis: usize| {
            difference([pair[0].uv[axis], pair[1].uv[axis]])?
                .div(widths[axis])
                .map(|x| I::point(max_abs(x)))
        };
        let du = delta(0)?;
        let dv = delta(1)?;
        let chord_error = uu
            .mul(du.mul(du)?)?
            .add(uv_norm.mul(du.mul(dv)?)?.mul(I::point(2.))?)?
            .add(vv.mul(dv.mul(dv)?)?)?
            .mul(I::point(0.125))?;
        let root_error = I::point(uv_error)
            .add(I::point(pair[0].root_rounding.max(pair[1].root_rounding)))?
            .div(widths[root])?;
        let root_shift = root_error.mul(root_norm)?;
        let endpoint = I::point(pair[0].world_rounding.max(pair[1].world_rounding));
        errors.push(chord_error.add(root_shift)?.add(endpoint)?.hi);
    }

    Ok(CurvatureGapWorldPolyline {
        uv,
        points: Some(vertices.into_iter().map(|vertex| vertex.point).collect()),
        world_error_upper: Some(errors),
    })
}

fn difference([a, b]: [f64; 2]) -> Result<I> {
    I::point(b).sub(I::point(a))
}

fn max_abs(i: I) -> f64 {
    i.lo.abs().max(i.hi.abs())
}

fn surface_vertex(
    surface: &Surface,
    span: [usize; 2],
    domain: [[f64; 2]; 2],
    uv: [f64; 2],
) -> Result<([f64; 3], f64)> {
    let mut box_ = domain;
    let mut corner = [0; 2];
    for i in 0..2 {
        if uv[i] > domain[i][0] {
            box_[i][1] = uv[i];
            corner[i] = 1;
        }
    }
    let j = jets::calculate(surface, span, box_, Some(corner))?;
    let origin = &surface.control_points[span[0] - surface.degree_u][span[1] - surface.degree_v];
    let mut position = j[0][0];
    for (i, &offset) in position.iter_mut().zip(origin) {
        *i = i.add(I::point(offset))?;
    }
    let point = position.map(|i| i.lo.midpoint(i.hi));
    let error = box_distance(&position, &point.map(I::point))?.1;
    Ok((point, error))
}

#[derive(Clone, Debug)]
pub struct CurvatureGapLength {
    pub graph: Option<CurvatureGapGraph>,
    /// Euclidean arc length bounds over each transverse interval, in order.
    pub segment_bounds: Option<Vec<[f64; 2]>>,
    pub length_bounds: Option<[f64; 2]>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Enclose the length of a certified contour in model units. The lower
/// bounds use endpoint distances minus certified position errors. Interval
/// bounds on the surface velocity composed with the contour slope supply
/// upper bounds and an independent speed-based lower bound. Both proofs
/// share the caller's exact work budget.
pub fn measure_curvature_gap_contour(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapLength> {
    let world = certify_curvature_gap_world_polyline(
        source, span, axis, level, transverse, max_depth, max_work,
    )?;
    let mut report = CurvatureGapLength {
        graph: world.uv.graph,
        segment_bounds: None,
        length_bounds: None,
        work_used: world.uv.work_used,
        refusal: world.uv.refusal,
    };
    let (Some(points), Some(errors)) = (world.points, world.world_error_upper) else {
        return Ok(report);
    };
    let remaining = max_work - report.work_used;
    if remaining == 0 {
        report.graph = None;
        report.refusal = Some(Refusal::ResourceLimit);
        return Ok(report);
    }
    let slope = curvature_gap_graph_slope(source, span, axis, level, remaining)?;
    report.work_used += slope.work_used;
    if let Some(reason) = slope.refusal {
        report.graph = None;
        report.refusal = Some(reason);
    }
    let Some([min_slope, max_slope]) = slope.slope else {
        return Ok(report);
    };

    let domain = world.uv.domain;
    let j = jets::calculate(source, span, domain, None)?;
    let widths = [difference(domain[0])?, difference(domain[1])?];
    let (root, root_jet, other_jet) = match axis {
        Axis::U => (0, j[1][0], j[0][1]),
        Axis::V => (1, j[0][1], j[1][0]),
    };
    let root_rate = I::new(min_slope, max_slope)?.div(widths[root])?;
    let other_rate = I::point(1.).div(widths[1 - root])?;
    let mut velocity = [I::point(0.); 3];
    for i in 0..3 {
        velocity[i] = root_jet[i]
            .mul(root_rate)?
            .add(other_jet[i].mul(other_rate)?)?;
    }
    let speed = norm(velocity)?;
    let mut segments = Vec::with_capacity(errors.len());
    let mut total = I::point(0.);
    for ((points, params), error) in points.windows(2).zip(transverse.windows(2)).zip(errors) {
        let integrated = speed.mul(difference([params[0], params[1]])?)?;
        let (chord_lower, _) = box_distance(&points[0].map(I::point), &points[1].map(I::point))?;
        let doubled_error = I::point(error).mul(I::point(2.))?;
        let lower = I::point(chord_lower).sub(doubled_error)?.lo;
        let segment = I::new(integrated.lo.max(lower).max(0.), integrated.hi)?;
        segments.push([segment.lo, segment.hi]);
        total = total.add(segment)?;
    }
    report.segment_bounds = Some(segments);
    report.length_bounds = Some([total.lo.max(0.), total.hi]);
    Ok(report)
}

#[derive(Clone, Debug)]
pub struct CurvatureGapWorldBounds {
    pub uv: CurvatureGapPolyline,
    /// Axis-aligned world boxes in transverse order, one per chord.
    pub segment_bounds: Option<Vec<[[f64; 2]; 3]>>,
    /// World-axis ranges enclosing the entire original contour in this cell.
    pub bounds: Option<[[f64; 2]; 3]>,
}

/// Bound the proven branch by axis-aligned world boxes. Each chord's
/// continuous Euclidean error expands all three coordinate intervals
/// outwards. Boxes can reject spatial overlap but cannot prove contact.
pub fn bound_curvature_gap_contour(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapWorldBounds> {
    let world = certify_curvature_gap_world_polyline(
        source, span, axis, level, transverse, max_depth, max_work,
    )?;
    let mut report = CurvatureGapWorldBounds {
        uv: world.uv,
        segment_bounds: None,
        bounds: None,
    };
    let (Some(points), Some(errors)) = (world.points, world.world_error_upper) else {
        return Ok(report);
    };
    let mut segments = Vec::with_capacity(errors.len());
    let mut overall = [[f64::INFINITY, f64::NEG_INFINITY]; 3];
    for (pair, error) in points.windows(2).zip(errors) {
        let bounds = chord_box(pair[0], pair[1], error)?.map(|i| [i.lo, i.hi]);
        for (overall, [lo, hi]) in overall.iter_mut().zip(bounds) {
            *overall = [overall[0].min(lo), overall[1].max(hi)];
        }
        segments.push(bounds);
    }
    report.segment_bounds = Some(segments);
    report.bounds = Some(overall);
    Ok(report)
}

pub(super) fn chord_box(a: [f64; 3], b: [f64; 3], error: f64) -> Result<[I; 3]> {
    let mut bounds = [I::point(0.); 3];
    let offset = I::new(-error, error)?;
    for ((bound, a), b) in bounds.iter_mut().zip(a).zip(b) {
        *bound = I::new(a.min(b), a.max(b))?.add(offset)?;
    }
    Ok(bounds)
}
