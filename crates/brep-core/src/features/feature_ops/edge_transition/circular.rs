//! Original circular-edge/plane-wall round proposals. Three-point frames and
//! support-normal recognition have NO authority: original cuts/G1, whole radius
//! and whole sphere-envelope/closure gates independently admit the saved body.
use super::super::{dot, plane, sub, unit};
use super::*;
fn point(p: &[f64]) -> [f64; 3] {
    [p[0], p[1], p[2]]
}
fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|k| a[k] + b[k])
}
fn scale(a: [f64; 3], b: f64) -> [f64; 3] {
    a.map(|a| a * b)
}
fn outward_at_edge(source: &Model, face: usize, edge: usize) -> Result<[f64; 3]> {
    let f = &source.faces[face];
    let c = std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .find_map(|w| source.loops[w].coedges.iter().find(|c| c.edge == edge))
        .ok_or_else(|| refused("Missing original circular edge use"))?;
    let d = c.pcurve.domain();
    let uv = c.pcurve.evaluate(d[0] * 0.5 + d[1] * 0.5)?.point;
    let mut n = f
        .surface
        .evaluate(uv[0], uv[1])?
        .unit_normal()
        .ok_or_else(|| refused("Original circular support normal unresolved"))?;
    let uses = source
        .shells
        .iter()
        .flat_map(|s| &s.faces)
        .filter(|u| u.face == face)
        .collect::<Vec<_>>();
    if uses.len() != 1 {
        return Err(refused("Circular support needs one original shell use"));
    }
    if uses[0].reversed {
        n = n.map(|x| -x);
    }
    Ok(n)
}
fn center(curve: &Curve) -> Result<([f64; 3], f64, [f64; 3])> {
    let d = curve.domain();
    let a = point(&curve.evaluate(d[0])?.point);
    let m = point(&curve.evaluate(d[0] * 0.5 + d[1] * 0.5)?.point);
    let b = point(&curve.evaluate(d[1])?.point);
    let x = sub(m, a);
    let y = sub(b, a);
    let xx = dot(x, x);
    let xy = dot(x, y);
    let yy = dot(y, y);
    let det = xx * yy - xy * xy;
    if !det.is_finite() || det <= 1e-12 * xx * yy {
        return Err(refused("Circular spine frame is collinear or unresolved"));
    }
    let delta = add(
        scale(x, yy * (xx - xy) / (2. * det)),
        scale(y, xx * (yy - xy) / (2. * det)),
    );
    let c = add(a, delta);
    let radius = dot(delta, delta).sqrt();
    if !radius.is_finite() || radius <= 0. {
        return Err(refused("Circular spine radius unresolved"));
    }
    Ok((c, radius, m))
}
pub(super) struct Frame {
    pub(super) origin: [f64; 3],
    pub(super) radius: f64,
    pub(super) normal: [f64; 3],
    pub(super) sign: f64,
}
pub(super) fn frame(source: &Model, edge: usize) -> Result<Frame> {
    let (supports, _) = topology(source, edge)?;
    let planar = supports
        .iter()
        .filter_map(|f| plane(source, *f).ok().map(|(_, n)| (*f, n)))
        .collect::<Vec<_>>();
    if planar.len() != 1 {
        return Err(refused(
            "Circular round needs one original planar support and one nonplanar wall",
        ));
    }
    let (plane_face, n) = planar[0];
    let wall = *supports.iter().find(|f| **f != plane_face).unwrap();
    let original = &source.edges[edge].curve;
    if original.periodic || original.weights.iter().any(|w| *w <= 0.) {
        return Err(refused(
            "Circular round requires a positive nonperiodic original carrier",
        ));
    }
    let (origin, r, middle) = center(original)?;
    let radial = unit(sub(middle, origin))?;
    let normal = outward_at_edge(source, wall, edge)?;
    let cosine = dot(radial, normal);
    if cosine.abs() < 0.999 || dot(radial, n).abs() > 1e-6 {
        return Err(refused(
            "Original wall/plane frame is not a resolved concentric contact family",
        ));
    }
    let sign = if cosine > 0. { 1. } else { -1. };
    Ok(Frame {
        origin,
        radius: r,
        normal: n,
        sign,
    })
}
/// Automatically round a nonperiodic circular original edge. One original
/// support must be planar; the other must admit the generated concentric
/// translated contact rail and its G1 field. Both radial end contours must
/// freshly lift on the original end faces. Convex outer and inner-wall rounds
/// are proposed from actual shell-normal orientation, never a UI axis/frame.
/// No periodic/higher-valence network, general curved spine or embedding claim.
pub fn round_circular_edge(
    source: &Model,
    edge: usize,
    radius: f64,
    limits: Limits,
) -> Result<Transition> {
    source.validate()?;
    if !radius.is_finite()
        || radius <= source.tolerance_mm
        || !(256..=100000).contains(&limits.cells)
        || !(1..=1000000).contains(&limits.exact_work)
    {
        return Err(refused(
            "Choose a positive resolved circular round and bounded original work",
        ));
    }
    let Frame {
        origin,
        radius: r,
        normal: n,
        sign,
    } = frame(source, edge)?;
    let original = &source.edges[edge].curve;
    if r - sign * radius <= source.tolerance_mm * 16. {
        return Err(refused("Circular round collapses its centerline"));
    }
    let factor = (r - sign * radius) / r;
    let contact = original
        .control_points
        .iter()
        .map(|p| add(origin, scale(sub(point(p), origin), factor)))
        .collect::<Vec<_>>();
    let mut centers = original.clone();
    centers.control_points = contact
        .iter()
        .map(|p| sub(*p, scale(n, radius)).to_vec())
        .collect();
    let surface = Surface {
        degree_u: 2,
        degree_v: original.degree,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: original.knots.clone(),
        control_points: vec![
            contact.iter().map(|p| p.to_vec()).collect(),
            original.control_points.clone(),
            original
                .control_points
                .iter()
                .map(|p| sub(point(p), scale(n, radius)).to_vec())
                .collect(),
        ],
        weights: [1., 0.5f64.sqrt(), 1.]
            .iter()
            .map(|a| original.weights.iter().map(|w| a * w).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    surface.validate()?;
    let d = original.domain();
    let (contours, curves) = boundaries(&surface, d)?;
    let mut law = Curve::from_polyline(vec![vec![radius, 0.]; 2])?;
    for t in &mut law.knots {
        *t = d[0] + *t * (d[1] - d[0]);
    }
    insert_with_curves(
        source,
        edge,
        RadiusPatch {
            surface: &surface,
            contour: contours.each_ref(),
            centers: &centers,
            radius: &law,
            spine_axis: Axis::V,
        },
        Some(&curves),
        limits,
    )
}

pub(super) fn boundaries(surface: &Surface, d: [f64; 2]) -> Result<([Curve; 4], [Curve; 4])> {
    let vertices = [[0., d[1]], [0., d[0]], [1., d[0]], [1., d[1]]];
    let contours = (0..4)
        .map(|i| Curve::from_polyline(vec![vertices[i].to_vec(), vertices[(i + 1) % 4].to_vec()]))
        .collect::<nurbs_core::Result<Vec<_>>>()?;
    let curves = [
        surface.iso(Axis::U, 0.)?.reverse()?,
        surface.iso(Axis::V, d[0])?,
        surface.iso(Axis::U, 1.)?,
        surface.iso(Axis::V, d[1])?.reverse()?,
    ];
    let contours = contours
        .try_into()
        .map_err(|_| refused("Circular contour construction failed"))?;
    Ok((contours, curves))
}
