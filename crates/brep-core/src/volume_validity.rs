//! Sufficient geometric volume evidence; unsupported or exhausted stages remain
//! unproven. Uses authored boundary geometry, not a tessellated volume estimate.
use crate::{Error, Model, Result, boundary_embedding, shell_nesting, shell_orientation, solid_audit};
#[derive(Clone, Copy)]
pub struct Limits {
    pub boundary: boundary_embedding::Limits,
    pub nesting_pairs: usize,
    pub nesting_cells: usize,
    pub nesting_domain_cells: usize,
    pub orientation_cells: usize,
    pub orientation_domain_cells: usize,
    pub orientation_spans: usize,
}
pub struct ShellOrientation {
    pub shell: usize,
    pub expected_outward: bool,
    pub outward: Option<bool>,
    pub attempts: Vec<shell_orientation::Report>,
}
pub struct Report {
    pub proven: bool,
    pub boundary: boundary_embedding::Report,
    pub nesting: Option<shell_nesting::Report>,
    pub orientations: Vec<ShellOrientation>,
    pub orientation_cells: usize,
    pub orientation_domain_cells: usize,
}
pub fn inspect(model: &Model, tolerance_uv: f64, limits: Limits) -> Result<Report> {
    inspect_impl(model,tolerance_uv,limits,0,None,&[])
}
/// Generic authored carriers, including nonplanar caps, may request a bounded
/// linear projection proof. No planar cap specialization is inferred.
pub fn inspect_with_linear(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize)->Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,None,&[])
}
/// Joint sweep boundary evidence precedes nesting and material orientation.
/// Empty cap selection is the closed no-cap case; no artificial caps are inferred.
pub fn inspect_sweep(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:&[usize],cap_limits:crate::sweep_cap_contacts::Budgets)->Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,Some((caps,cap_limits)),&[])
}
pub fn inspect_sweep_with_projections(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:&[usize],cap_limits:crate::sweep_cap_contacts::Budgets,proposals:&[Option<[[i8;3];2]>])->Result<Report> {
    inspect_impl(model,tolerance_uv,limits,max_linear_cells,Some((caps,cap_limits)),proposals)
}
fn inspect_impl(model:&Model,tolerance_uv:f64,limits:Limits,max_linear_cells:usize,
    caps:Option<(&[usize],crate::sweep_cap_contacts::Budgets)>,proposals:&[Option<[[i8;3];2]>])->Result<Report> {
    if !(1..=1000000).contains(&limits.orientation_cells)
        || !(1..=8000000).contains(&limits.orientation_domain_cells)
        || !(1..=100000).contains(&limits.orientation_spans)
        || !(1..=100000).contains(&limits.nesting_pairs)
        || !(2..=1000000).contains(&limits.nesting_cells)
        || !(2..=8000000).contains(&limits.nesting_domain_cells) {
        return Err(Error::new("BREP_INVALID_INPUT", "Volume audit requires bounded positive stage budgets"));
    }
    let boundary = match caps {
        Some((caps,budget))=>boundary_embedding::inspect_sweep_with_projections(
            model,tolerance_uv,limits.boundary,max_linear_cells,caps,budget,proposals)?,
        _ if max_linear_cells == 0 => boundary_embedding::inspect(model,tolerance_uv,limits.boundary)?,
        _ => boundary_embedding::inspect_with_linear(model,tolerance_uv,limits.boundary,max_linear_cells)?,
    };
    let mut out = Report { proven: false, boundary, nesting: None,
        orientations: (0..model.shells.len()).map(|shell| ShellOrientation {
            shell, expected_outward: !model.bodies.iter().any(|b| b.inner_shells.contains(&shell)),
            outward: None, attempts: Vec::new() }).collect(),
        orientation_cells: 0, orientation_domain_cells: 0 };
    if !out.boundary.proven { return Ok(out); }
    let nesting = shell_nesting::inspect_with_boundary(model, model.tolerance_mm, tolerance_uv,
        limits.nesting_pairs, limits.nesting_cells, limits.nesting_domain_cells,Some(&out.boundary))?;
    let roles = nesting.roles_consistent == Some(true);
    out.nesting = Some(nesting);
    if !roles { return Ok(out); }
    for i in 0..model.shells.len() {
        // Keep the actual cavity orientation. The extraction helper may invert
        // it for recognition, but that would erase the fact under inspection.
        let shell = solid_audit::isolated_outward_shell(model, i, false)?;
        let mut bounds = [[f64::INFINITY, f64::NEG_INFINITY];3];
        for p in shell.faces.iter().flat_map(|f| &f.surface.control_points).flatten() {
            for k in 0..3 { bounds[k][0] = bounds[k][0].min(p[k]); bounds[k][1] = bounds[k][1].max(p[k]); }
        }
        let point = bounds.map(|b| b[0]*0.5+b[1]*0.5);
        let directions = [[1.,0.317,0.173],[0.239,1.,0.419],[0.137,0.283,1.]];
        let mut rays=directions.into_iter().map(|d|(point,d)).collect::<Vec<_>>();
        // A box centre may lie in the empty hole of a toroidal shell and all
        // rays can miss it. Propose additional rays aimed at a retained chart.
        // Evaluated points only propose origins; every crossing and its normal
        // still require the independent interval ray/orientation certificate.
        let s=&shell.faces[0].surface;
        // Avoid targeting the dyadic subdivision seam itself, where a strict
        // interior root certificate may remain unresolved in both children.
        let u=s.knots_u[s.degree_u]*0.629+s.knots_u[s.control_points.len()]*0.371;
        let v=s.knots_v[s.degree_v]*0.581+s.knots_v[s.control_points[0].len()]*0.419;
        if let Ok(target)=s.evaluate(u,v) {
            let reach=bounds.iter().map(|b|b[1]-b[0]).fold(0_f64,f64::max)*4.;
            for direction in directions {
                let origin=std::array::from_fn(|k|target.point[k]-direction[k]*reach);
                if reach>0. && reach.is_finite() && origin.iter().all(|x|x.is_finite()) {
                    rays.push((origin,direction));
                }
            }
        }
        let ray_count=rays.len();
        for (attempt, (origin,direction)) in rays.into_iter().enumerate() {
            let cells = limits.orientation_cells-out.orientation_cells;
            let domains = limits.orientation_domain_cells-out.orientation_domain_cells;
            if cells == 0 || domains == 0 { break; }
            let remaining = (model.shells.len()-i-1)*6+ray_count-attempt;
            let r = shell_orientation::inspect(&shell, origin, direction, tolerance_uv,
                (cells/remaining).max(1), (domains/remaining).max(1), limits.orientation_spans)?;
            out.orientation_cells += r.ray.cells;
            out.orientation_domain_cells += r.ray.domain_cells;
            out.orientations[i].outward = r.outward;
            out.orientations[i].attempts.push(r);
            if out.orientations[i].outward.is_some() { break; }
        }
    }
    out.proven = out.orientations.iter().all(|r| r.outward == Some(r.expected_outward));
    Ok(out)
}
#[cfg(test)]
mod tests;
