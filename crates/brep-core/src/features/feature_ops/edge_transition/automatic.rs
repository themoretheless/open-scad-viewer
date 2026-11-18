//! Construct straight-spine round/chamfer proposals from original topology.
//! Frames, Newton intersections and interpolation are proposals only. The
//! original four-face owner admits complete boundaries, G1, radius and closure.
use super::super::{dot, sub, unit};
use super::{Limits, RadiusPatch, Transition, insert_with_curves, refused, topology};
use crate::{Model, Result};
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
};
mod ends;
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] + b[k])
}
fn scale(a: [f64; 3], b: f64) -> [f64; 3] {
    a.map(|x| x * b)
}
fn point(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
pub(super) struct Frame {
    pub(super) axis: [f64; 3],
    centers: [[f64; 3]; 2],
    pub(super) surface: Surface,
    pub(super) ends: [usize; 2],
    pub(super) vertices: [usize; 2],
}
pub(super) enum CrossSection {
    Round(f64),
    Chamfer([f64; 2]),
}
pub(super) fn frame(source: &Model, edge: usize, section: CrossSection) -> Result<Frame> {
    let (supports, ends) = topology(source, edge)?;
    let vertices = source.edges[edge].vertices;
    let base = source.vertices[vertices[0]].point;
    let delta = sub(source.vertices[vertices[1]].point, base);
    let axis = unit(delta)?;
    let length = dot(delta, axis);
    if length <= source.tolerance_mm * 16. {
        return Err(refused("Straight transition needs a resolved nonzero edge"));
    }
    // These control-net tests propose a straight/planar construction family.
    // Actual contacts on the unchanged original carriers must still qualify.
    for p in &source.edges[edge].curve.control_points {
        let d = sub(point(p), base);
        let residual = sub(d, scale(axis, dot(d, axis)));
        if dot(residual, residual).sqrt() > source.tolerance_mm / 8. {
            return Err(refused(
                "Automatic straight transition requires a straight original edge",
            ));
        }
    }
    let (_, a) = super::super::plane(source, supports[0])?;
    let (_, b) = super::super::plane(source, supports[1])?;
    if dot(a, axis).abs() > 1e-7 || dot(b, axis).abs() > 1e-7 {
        return Err(refused(
            "Original support frame is not transverse to the edge",
        ));
    }
    let cosine = dot(a, b);
    if !(-0.999..0.999).contains(&cosine) {
        return Err(refused(
            "Parallel or nearly folded supports need another constructor",
        ));
    }
    let (center, controls, weights, allowance) = match section {
        CrossSection::Round(radius) => {
            let center = add(base, scale(add(a, b), -radius / (1. + cosine)));
            (
                center,
                vec![
                    add(center, scale(a, radius)),
                    base,
                    add(center, scale(b, radius)),
                ],
                vec![1., ((1. + cosine) / 2.).sqrt(), 1.],
                radius,
            )
        }
        CrossSection::Chamfer(setbacks) => {
            let direction_a = unit(add(scale(b, -1.), scale(a, cosine)))?;
            let direction_b = unit(add(scale(a, -1.), scale(b, cosine)))?;
            (
                base,
                vec![
                    add(base, scale(direction_a, setbacks[0])),
                    add(base, scale(direction_b, setbacks[1])),
                ],
                vec![1., 1.],
                setbacks[0].max(setbacks[1]),
            )
        }
    };
    let mut low = 0f64;
    let mut high = length;
    for &f in &ends {
        for p in source.faces[f].surface.control_points.iter().flatten() {
            let z = dot(sub(point(p), base), axis);
            low = low.min(z);
            high = high.max(z);
        }
    }
    // Positive-weight source caps lie in their control hulls. This merely
    // chooses a carrier extent; all authored UV poles must remain in it.
    let margin = (high - low) * 0.01 + allowance * 0.01;
    low -= margin;
    high += margin;
    if !low.is_finite() || !high.is_finite() || high <= low {
        return Err(refused("Unresolved automatic transition carrier extent"));
    }
    let surface = Surface {
        degree_u: controls.len() - 1,
        degree_v: 1,
        knots_u: [vec![0.; controls.len()], vec![1.; controls.len()]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: controls
            .into_iter()
            .map(|p| {
                vec![
                    add(p, scale(axis, low)).to_vec(),
                    add(p, scale(axis, high)).to_vec(),
                ]
            })
            .collect(),
        weights: weights.into_iter().map(|w| vec![w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    Ok(Frame {
        axis,
        centers: [
            add(center, scale(axis, low)),
            add(center, scale(axis, high)),
        ],
        surface,
        ends,
        vertices,
    })
}
/// Constant-radius round of a nonperiodic straight original edge between two
/// planar support charts. End faces may be curved: their complete contact
/// curves are constructed and freshly checked on the original charts. No patch,
/// contact rails, neighboring faces or removal seeds are supplied by the caller.
/// This does not cover curved spines, concave/parallel supports, support extension
/// or higher-valence end networks. Failed proposals never publish a partial body.
/// Finite Newton/collocation sites propose geometry; they never admit it.
pub fn round_straight_edge(
    source: &Model,
    edge: usize,
    radius: f64,
    limits: Limits,
) -> Result<Transition> {
    source.validate()?;
    if !radius.is_finite()
        || radius <= source.tolerance_mm
        || limits.cells < 256
        || limits.cells > 100000
        || limits.exact_work == 0
        || limits.exact_work > 1000000
    {
        return Err(refused(
            "Choose a positive resolved radius and bounded transition work",
        ));
    }
    let f = frame(source, edge, CrossSection::Round(radius))?;
    let Boundaries {
        contours,
        curves,
        cells,
    } = boundaries(source, &f, limits)?;
    let centers = Curve::from_polyline(f.centers.map(|p| p.to_vec()).to_vec())?;
    let law = Curve::from_polyline(vec![vec![radius, 0.]; 2])?;
    let mut rest = limits;
    rest.cells = limits
        .cells
        .checked_sub(cells)
        .ok_or_else(|| refused("Automatic contact work exhausted"))?;
    let mut result = insert_with_curves(
        source,
        edge,
        RadiusPatch {
            surface: &f.surface,
            contour: contours.each_ref(),
            centers: &centers,
            radius: &law,
            spine_axis: Axis::V,
        },
        Some(&curves),
        rest,
    )?;
    result.topology.work.cells += cells;
    result.topology.work.limits = limits;
    Ok(result)
}

pub(super) struct Boundaries {
    pub(super) contours: [Curve; 4],
    pub(super) curves: [Curve; 4],
    pub(super) cells: usize,
}
pub(super) fn boundaries(source: &Model, f: &Frame, limits: Limits) -> Result<Boundaries> {
    let mut cells = 0;
    let bottom = ends::construct(
        source,
        f.ends[0],
        f.vertices[0],
        &f.surface,
        f.axis,
        &mut cells,
        limits.cells / 8,
    )?;
    let top = ends::construct(
        source,
        f.ends[1],
        f.vertices[1],
        &f.surface,
        f.axis,
        &mut cells,
        limits.cells / 8,
    )?;
    let endpoint = |p: &Curve, last: bool| -> Result<Vec<f64>> {
        Ok(p.evaluate(p.domain()[usize::from(last)])?.point)
    };
    let b0 = endpoint(&bottom.0, false)?;
    let b1 = endpoint(&bottom.0, true)?;
    let t0 = endpoint(&top.0, false)?;
    let t1 = endpoint(&top.0, true)?;
    let contours = [
        Curve::from_polyline(vec![t0.clone(), b0.clone()])?,
        bottom.0,
        Curve::from_polyline(vec![b1.clone(), t1.clone()])?,
        top.0.reverse()?,
    ];
    let spatial =
        |uv: &[f64]| -> Result<Vec<f64>> { Ok(f.surface.evaluate(uv[0], uv[1])?.point.to_vec()) };
    let curves = [
        Curve::from_polyline(vec![spatial(&t0)?, spatial(&b0)?])?,
        bottom.1,
        Curve::from_polyline(vec![spatial(&b1)?, spatial(&t1)?])?,
        top.1.reverse()?,
    ];
    Ok(Boundaries {
        contours,
        curves,
        cells,
    })
}
