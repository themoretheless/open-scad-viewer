//! Original closed carrier Green flux on a coordinate-constant material face.
//! Unsupported root/mapped trims retain the surface-domain volume gate.
use crate::{
    source_boundary_fragment::Endpoint, source_shell_geometry::Geometry,
    source_shell_incidence::Address,
};
use nurbs_core::{Result, curve::Curve, interval_eval::Interval as I};
/// Only privately qualified native edges can supply complete Green carriers.
pub trait FluxSupport:crate::source_seam_tangency::Carrier {
    fn green_source(&self,side:usize)->Option<(&Curve,bool)>;
}
impl FluxSupport for crate::source_shared_edge::SharedEdge {
    fn green_source(&self,side:usize)->Option<(&Curve,bool)> {
        if self.ranges().is_some() || !self.covers_complete_canonical_source() {return None;}
        Some((self.world(),self.reversed()[side]^self.uses()[side].reversed()))
    }
}
impl FluxSupport for crate::source_projective_edge::Edge {
    fn green_source(&self,side:usize)->Option<(&Curve,bool)> {
        use crate::source_allowed_contact::Support;
        if !self.maps_complete_source() || self.uses().iter().any(|u| {
            let f=u.fragment();let d=f.curve().domain();
            f.reversed() || !f.endpoints().iter().zip(d).all(|(end,t)|matches!(end,Endpoint::Parameter(p) if *p==t))
        }) {return None;}
        Some((self.world(),self.uses()[side].reversed()))
    }
}
pub(crate) struct Report {
    pub bound: Option<I>,
    pub cells: usize,
}
pub(crate) fn bound<E:FluxSupport>(
    geometry: &Geometry<E>,
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
            let Some((source,reverse))=shared.green_source(slot) else {return Ok(out);};
            if !(0..2)
                    .all(|e| matches!(fragment.endpoints()[e],Endpoint::Parameter(t) if t==ends[e]))
                || source
                    .control_points
                    .iter()
                    .any(|p| p[axis] != fixed)
            {
                return Ok(out);
            }
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
            if reverse {
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
