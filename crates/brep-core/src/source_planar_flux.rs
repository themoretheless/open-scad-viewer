//! Original closed carrier Green flux on a coordinate-constant material face.
//! Unsupported root/mapped trims retain the surface-domain volume gate.
use crate::{
    source_boundary_fragment::Endpoint, source_shell_geometry::Geometry,
    source_shell_incidence::Address,
};
use nurbs_core::{Result, curve::Curve, interval_eval::Interval as I};
pub(crate) struct Report {
    pub bound: Option<I>,
    pub cells: usize,
}
pub(crate) fn bound(
    geometry: &Geometry,
    face: usize,
    axis: usize,
    origin: f64,
    error: f64,
    max_cells: usize,
) -> Result<Report> {
    let shell = geometry.shell();
    let region = &shell.regions().unwrap()[face];
    let surface = region.loops()[0][0].surface();
    let fixed = surface.control_points[0][0][axis];
    let mut out = Report {
        bound: None,
        cells: 0,
    };
    if surface
        .control_points
        .iter()
        .flatten()
        .any(|p| p[axis] != fixed)
    {
        return Ok(out);
    }
    let scale = I::point(fixed).sub(I::point(origin))?;
    if fixed == origin {
        out.bound = Some(I::point(0.));
        return Ok(out);
    }
    let axes = [(axis + 1) % 3, (axis + 2) % 3];
    let mut chains = Vec::new();
    for (wire, fragments) in region.loops().iter().enumerate() {
        let mut curves = Vec::new();
        for (edge, fragment) in fragments.iter().enumerate() {
            let address = Address { face, wire, edge };
            let Some((i, slot)) = shell
                .uses()
                .iter()
                .enumerate()
                .find_map(|(i, uses)| uses.iter().position(|a| *a == address).map(|s| (i, s)))
            else {
                return Ok(out);
            };
            let shared = &shell.edges()[i];
            let d = fragment.curve().domain();
            let ends = if fragment.reversed() { [d[1], d[0]] } else { d };
            if shared.ranges().is_some()
                || !shared.covers_complete_canonical_source()
                || !(0..2)
                    .all(|e| matches!(fragment.endpoints()[e],Endpoint::Parameter(t) if t==ends[e]))
                || shared
                    .world()
                    .control_points
                    .iter()
                    .any(|p| p[axis] != fixed)
            {
                return Ok(out);
            }
            let source = shared.world();
            let d = source.domain();
            let n = source.control_points.len();
            if source.periodic
                || source.weights.iter().any(|w| *w <= 0.)
                || n != source.degree + 1
                || source.knots.len() != 2 * n
                || source.knots[..n].iter().any(|t| *t != d[0])
                || source.knots[n..].iter().any(|t| *t != d[1])
            {
                return Ok(out);
            }

            let mut c = Curve {
                control_points: source
                    .control_points
                    .iter()
                    .map(|p| axes.map(|a| p[a]).to_vec())
                    .collect(),
                ..source.clone()
            };
            if shared.reversed()[slot] ^ fragment.reversed() {
                c.control_points.reverse();
                c.weights.reverse();
            }
            curves.push(c);
        }
        if !(2..=256).contains(&curves.len())
            || nurbs_core::trim_domain::exact_loop_joins(&curves)? != Some(true)
        {
            return Ok(out);
        }
        chains.push(curves);
    }
    let magnitude = scale.lo.abs().max(scale.hi.abs());
    let tolerance = error / (magnitude * chains.len() as f64);
    if !tolerance.is_finite() || tolerance <= 0. {
        return Ok(out);
    }
    let mut total = I::point(0.);
    for curves in chains {
        let spans = curves
            .iter()
            .map(|c| {
                (c.degree..c.control_points.len())
                    .filter(|&s| c.knots[s] < c.knots[s + 1])
                    .count()
            })
            .sum::<usize>();
        let available = max_cells.saturating_sub(out.cells).min(100000);
        if available < spans || available == 0 {
            return Ok(out);
        }
        let r = nurbs_core::planar_area::measure(&curves, tolerance, available)?;
        out.cells += r.cells;
        total = total.add(I::new(r.area_interval_mm2[0], r.area_interval_mm2[1])?)?;
    }
    out.bound = Some(total.mul(scale)?);
    Ok(out)
}
