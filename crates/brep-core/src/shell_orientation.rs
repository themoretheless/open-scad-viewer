//! Orientation evidence at the last certified crossing of an individual shell.
//! Interpreting this as global outwardness requires an embedded closed boundary;
//! neither ray parity nor a normal sign alone certifies that precondition.
use crate::{Error, Model, Result, ray_parity};
pub struct Report {
    pub ray: ray_parity::Report,
    pub face: Option<usize>,
    pub normal_dot_direction: Option<[f64;2]>,
    pub normal_method: &'static str,
    pub outward: Option<bool>,
}
pub fn inspect(model: &Model, point: [f64;3], direction: [f64;3], tolerance_uv: f64,
    max_cells: usize, max_domain_cells: usize, max_spans: usize) -> Result<Report> {
    model.validate()?;
    if model.shells.len()!=1 || !model.shells[0].closed || model.shells[0].faces.len()!=model.faces.len()
        || !(1..=100000).contains(&max_spans) {
        return Err(Error::new("BREP_INVALID_INPUT","Orientation requires one closed shell and a bounded positive span budget"));
    }
    let ray=ray_parity::classify_ray(model,point,direction,tolerance_uv,max_cells,max_domain_cells)?;
    let mut result=Report{ray,face:None,normal_dot_direction:None,normal_method:"original-source-normal",outward:None};
    if result.ray.parity.is_none() {return Ok(result);}
    let Some(crossing)=result.ray.crossings.last() else {return Ok(result)};
    result.face=Some(crossing.face);
    let original=&model.faces[crossing.face].surface;
    let equivalent=nurbs_core::surface_linear_monotonicity::polynomial_profile_image(original)?;
    let (surface,uv,scale)=if let Some((polynomial,parameterization))=&equivalent {
        let (u,derivative)=parameterization.map_interval(crossing.uv[0])?;
        result.normal_method="positive-profile-bijection-normal";
        (polynomial,[u,crossing.uv[1]],Some(derivative))
    } else {(original,crossing.uv,None)};
    // A whole-chart bound avoids cancellation when restricting a translated
    // planar patch to a tiny root enclosure. It is still a bound at the root.
    // Reserve half the span budget for the local fallback when needed.
    let whole=[[surface.knots_u[surface.degree_u],surface.knots_u[surface.control_points.len()]],
        [surface.knots_v[surface.degree_v],surface.knots_v[surface.control_points[0].len()]]];
    let coarse_budget=(max_spans/2).max(1);
    let coarse=nurbs_core::surface_injectivity::normal_direction_bounds(surface,whole,direction,coarse_budget)?;
    let bounds=if coarse.is_some_and(|b| b[0]>0. || b[1]<0.) { coarse }
        else if max_spans>coarse_budget {
            nurbs_core::surface_injectivity::normal_direction_bounds(surface,uv,direction,max_spans-coarse_budget)?
        } else { None };
    let Some(mut bounds)=bounds else {return Ok(result)};
    if let Some(scale)=scale {
        // The original normal equals the polynomial normal times the strictly
        // positive profile derivative. Report an enclosure for the original
        // source parameterization and keep the actual crossing UV unchanged.
        let normal=nurbs_core::interval_eval::Interval::new(bounds[0],bounds[1])?
            .mul(nurbs_core::interval_eval::Interval::new(scale[0],scale[1])?)?;
        bounds=[normal.lo,normal.hi];
    }
    let use_=model.shells[0].faces.iter().find(|u|u.face==crossing.face).unwrap();
    if use_.reversed {bounds=[-bounds[1],-bounds[0]];}
    result.normal_dot_direction=Some(bounds);
    if bounds[0]>0. {result.outward=Some(true);}
    else if bounds[1]<0. {result.outward=Some(false);}
    Ok(result)
}
/// Orientation with proven trim-region validity at the crossing face.
/// The model convention requires a counterclockwise outer UV wire.
pub struct TrimmedReport {
    pub chart: Report,
    pub trim: Option<nurbs_core::trim_region_audit::Report>,
    pub outward: Option<bool>,
    pub cells: usize,
    pub domain_cells: usize,
}
/// Conditional on an embedded, consistently sewn closed shell. Work for the
/// ray and the crossing face's trim region shares the supplied cell budgets.
pub fn inspect_trimmed(model: &Model, point: [f64; 3], direction: [f64; 3],
    tolerance_uv: f64, max_cells: usize, max_domain_cells: usize,
    max_spans: usize, max_pairs: usize) -> Result<TrimmedReport> {
    if !(1..=100000).contains(&max_pairs) {
        return Err(Error::new("BREP_INVALID_INPUT", "Trim orientation requires a bounded positive pair budget"));
    }
    let chart = inspect(model, point, direction, tolerance_uv, max_cells, max_domain_cells, max_spans)?;
    let mut out = TrimmedReport { cells: chart.ray.cells, domain_cells: chart.ray.domain_cells,
        chart, trim: None, outward: None };
    let Some(face) = out.chart.face else { return Ok(out) };
    if out.chart.outward.is_none() || out.cells >= max_cells || out.domain_cells >= max_domain_cells {
        return Ok(out);
    }
    let face = &model.faces[face];
    let loops = std::iter::once(face.outer).chain(face.holes.iter().copied())
        .map(|wire| model.loops[wire].coedges.iter().map(|c| c.pcurve.clone()).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    if loops.len() > 16 || loops.iter().any(|l| l.len() < 2) || loops.iter().map(Vec::len).sum::<usize>() > 256 {
        return Ok(out);
    }
    let trim = nurbs_core::trim_region_audit::inspect(&loops, tolerance_uv, max_pairs,
        (max_cells-out.cells).min(100000), (max_domain_cells-out.domain_cells).min(1000000))?;
    out.cells += trim.cells;
    out.domain_cells += trim.domain_cells;
    if trim.valid == Some(true) && trim.winding[0] == Some(1) {
        out.outward = out.chart.outward;
    }
    out.trim = Some(trim);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn trim_validity_and_face_use_are_required_for_orientation() {
        let mut m = crate::cuboid([0.;3],[1.;3]).unwrap();
        let run = |m: &Model, pairs| inspect_trimmed(m,[0.3,0.4,0.5],[1.,0.317,0.173],1e-7,100000,1000000,100,pairs).unwrap();
        assert_eq!(run(&m,1000).outward,Some(true));
        for f in &mut m.shells[0].faces { f.reversed = !f.reversed; }
        let before = format!("{m:?}");
        let r = run(&m,1000);
        assert_eq!(r.outward,Some(false));
        assert!(r.cells<=100000 && r.domain_cells<=1000000);
        let limited = run(&m,1);
        assert!(limited.chart.outward.is_some());
        assert_eq!(limited.outward,None);
        assert_eq!(format!("{m:?}"),before);
        // Reversing the entire wire is not an alternative model convention.
        for wire in &mut m.loops {
            wire.coedges.reverse();
            for c in &mut wire.coedges { c.reversed = !c.reversed; c.pcurve = c.pcurve.reverse().unwrap(); }
        }
        assert!(inspect_trimmed(&m,[0.3,0.4,0.5],[1.,0.317,0.173],1e-7,100000,1000000,100,1000).is_err());
    }
    #[test]
    fn outward_and_inward_cube_and_rational_sphere() {
        for (mut m,point) in [(crate::cuboid([0.;3],[1.;3]).unwrap(),[0.3,0.4,0.5]),(crate::sphere(2.).unwrap(),[0.;3])] {
            let direction=[1.,0.317,0.173];
            let r=inspect(&m,point,direction,1e-7,100000,1000000,100).unwrap();
            assert_eq!(r.outward,Some(true));
            for f in &mut m.shells[0].faces {f.reversed=!f.reversed;}
            let r=inspect(&m,point,direction,1e-7,100000,1000000,100).unwrap();
            assert_eq!(r.outward,Some(false));
        }
    }
    #[test]
    fn missing_crossings_and_unresolved_rays_do_not_prove_orientation() {
        let m=crate::cuboid([0.;3],[1.;3]).unwrap();
        for (point,direction,cells,domains) in [([2.;3],[1.,0.1,0.2],10000,100000),([0.,0.4,0.5],[1.,0.1,0.2],10000,100000),([0.3,0.4,0.5],[1.,0.1,0.2],1,1)] {
            let r=inspect(&m,point,direction,1e-7,cells,domains,10).unwrap();
            assert!(r.outward.is_none());
        }
    }
}
