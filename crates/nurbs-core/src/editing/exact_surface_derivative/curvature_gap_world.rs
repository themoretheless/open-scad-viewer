//! World-space error for a certified curvature contour's stored polyline.
use super::{
    curvature_gap_polyline::{CurvatureGapPolyline, certify_curvature_gap_polyline},
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
