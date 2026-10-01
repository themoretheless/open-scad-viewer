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
    if !(1..=1000000).contains(&limits.orientation_cells)
        || !(1..=8000000).contains(&limits.orientation_domain_cells)
        || !(1..=100000).contains(&limits.orientation_spans)
        || !(1..=100000).contains(&limits.nesting_pairs)
        || !(2..=1000000).contains(&limits.nesting_cells)
        || !(2..=8000000).contains(&limits.nesting_domain_cells) {
        return Err(Error::new("BREP_INVALID_INPUT", "Volume audit requires bounded positive stage budgets"));
    }
    let boundary = boundary_embedding::inspect(model, tolerance_uv, limits.boundary)?;
    let mut out = Report { proven: false, boundary, nesting: None,
        orientations: (0..model.shells.len()).map(|shell| ShellOrientation {
            shell, expected_outward: !model.bodies.iter().any(|b| b.inner_shells.contains(&shell)),
            outward: None, attempts: Vec::new() }).collect(),
        orientation_cells: 0, orientation_domain_cells: 0 };
    if !out.boundary.proven { return Ok(out); }
    let nesting = shell_nesting::inspect(model, model.tolerance_mm, tolerance_uv,
        limits.nesting_pairs, limits.nesting_cells, limits.nesting_domain_cells)?;
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
        for (attempt, direction) in directions.into_iter().enumerate() {
            let cells = limits.orientation_cells-out.orientation_cells;
            let domains = limits.orientation_domain_cells-out.orientation_domain_cells;
            if cells == 0 || domains == 0 { break; }
            let remaining = (model.shells.len()-i-1)*3+3-attempt;
            let r = shell_orientation::inspect(&shell, point, direction, tolerance_uv,
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
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits { boundary: boundary_embedding::Limits { exact_work: 1000000,
            trim_pairs: 10000, trim_cells: 100000, trim_domain_cells: 1000000, spans: 1000,
            contacts: crate::face_contacts::Limits { pairs: 10000, cells: 100000,
                domain_cells: 1000000, cells_per_pair: 1000, domain_cells_per_pair: 10000 } },
            nesting_pairs: 100, nesting_cells: 100000, nesting_domain_cells: 1000000,
            orientation_cells: 100000, orientation_domain_cells: 1000000, orientation_spans: 100 }
    }
    #[test]
    fn binary_exact_sphere_has_certified_volume_validity(){
        let model=crate::analytic::sphere(3.).unwrap();let before=format!("{model:?}");
        let r=inspect(&model,1e-8,limits()).unwrap();
        assert!(r.proven,"boundary={} orientation={:?}",r.boundary.proven,r.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        assert!(r.boundary.intersections.absence_proven);assert_eq!(r.orientations[0].outward,Some(true));
        assert_eq!(format!("{model:?}"),before);
        let rounded=crate::analytic::sphere(2.).unwrap();let r=inspect(&rounded,1e-8,limits()).unwrap();
        assert!(!r.proven);assert!(!r.boundary.agreement.all_equal);
        for offset in [[8.,-4.,6.],[-8.,4.,-6.]]{
            let mut moved=model.clone();
            for v in &mut moved.vertices{for k in 0..3{v.point[k]+=offset[k];}}
            for e in &mut moved.edges{for p in &mut e.curve.control_points{for k in 0..3{p[k]+=offset[k];}}}
            for f in &mut moved.faces{for row in &mut f.surface.control_points{for p in row{for k in 0..3{p[k]+=offset[k];}}}}
            let r=inspect(&moved,1e-8,limits()).unwrap();assert!(r.proven,"offset={offset:?}");
        }
    }
    #[test]
    fn exact_sphere_radius_family_keeps_all_volume_proofs(){
        for radius in [0.000011444091796875,0.375,1.5,6.,12.,786432.]{
            let model=crate::analytic::sphere(radius).unwrap();let before=format!("{model:?}");
            let r=inspect(&model,1e-8,limits()).unwrap();
            assert!(r.proven,"radius={radius}, boundary={}, orientations={:?}",r.boundary.proven,r.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
            assert!(r.boundary.agreement.all_equal);assert!(r.boundary.intersections.absence_proven);
            assert_eq!(format!("{model:?}"),before);
        }
    }
    #[test]
    fn authored_cylinder_has_certified_volume_validity(){
        let m=crate::analytic::cylinder(2.,4.).unwrap();let before=format!("{m:?}");
        let r=inspect(&m,1e-8,limits()).unwrap();
        assert!(r.proven);assert!(r.boundary.intersections.absence_proven);
        assert_eq!(r.orientations[0].outward,Some(true));assert_eq!(format!("{m:?}"),before);
    }
    #[test]
    #[ignore = "Roadmap gate: curved face embedding remains unproven"]
    fn authored_curved_primitives_have_certified_volume_validity(){
        for (name,m) in [("cylinder",crate::analytic::cylinder(2.,4.).unwrap()),("sphere",crate::analytic::sphere(2.).unwrap())]{
            let r=inspect(&m,1e-8,limits()).unwrap();
            if !r.boundary.agreement.all_equal {
                for use_ in &r.boundary.agreement.uses {
                    if !use_.decision.as_ref().is_some_and(|d|d.outcome==cad_predicates::BezierIdentity::Equal) {
                        eprintln!("{name}: boundary face={} wire={} coedge={} edge={} decision={:?}",use_.face,use_.wire,use_.coedge,use_.edge,use_.decision);
                    }
                }
            }
            assert!(r.proven,"{name}: exact={} joins={} trim={} faces={} pairs={} nesting={:?} orientations={:?}",
                r.boundary.agreement.all_equal,r.boundary.agreement.all_joins_exact,r.boundary.trim.all_valid,
                r.boundary.intersections.faces.all_faces_injective,r.boundary.intersections.pairs.all_pairs_classified,
                r.nesting.as_ref().and_then(|n|n.roles_consistent),r.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        }
    }
    #[test]
    fn cube_orientation_and_exhaustion_are_independent_of_embedding() {
        let mut m = crate::cuboid([0.;3],[1.;3]).unwrap();
        assert!(inspect(&m,1e-8,limits()).unwrap().proven);
        let r = inspect(&m,1e-8,Limits { orientation_cells: 1, orientation_domain_cells: 1, ..limits() }).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert!(r.orientation_cells<=1 && r.orientation_domain_cells<=1);
        for u in &mut m.shells[0].faces { u.reversed = !u.reversed; }
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert_eq!(r.orientations[0].outward,Some(false));
    }
    #[test]
    fn cavity_requires_inward_inner_shell_and_preserves_document() {
        let mut m = crate::cuboid([0.;3],[10.;3]).unwrap();
        let mut inner_model = crate::cuboid([2.;3],[8.;3]).unwrap();
        let (v,e,l,f)=(m.vertices.len(),m.edges.len(),m.loops.len(),m.faces.len());
        for edge in &mut inner_model.edges { edge.vertices = edge.vertices.map(|x| x+v); }
        for wire in &mut inner_model.loops { for c in &mut wire.coedges { c.edge+=e; } }
        for face in &mut inner_model.faces { face.outer+=l; for h in &mut face.holes { *h+=l; } }
        for u in &mut inner_model.shells[0].faces { u.face+=f; u.reversed=!u.reversed; }
        m.vertices.extend(inner_model.vertices.clone()); m.edges.extend(inner_model.edges.clone());
        m.loops.extend(inner_model.loops.clone()); m.faces.extend(inner_model.faces.clone());
        m.shells.extend(inner_model.shells.clone()); m.bodies[0].inner_shells.push(1);
        m.rebuild_topology_ids();
        let before = format!("{m:?}");
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.proven, "boundary={} exact={} trim={} intersections={} nesting={:?} orientations={:?}",r.boundary.proven,r.boundary.agreement.all_equal,r.boundary.trim.all_valid,r.boundary.intersections.absence_proven,r.nesting.as_ref().map(|n|n.roles_consistent),r.orientations.iter().map(|o|o.outward).collect::<Vec<_>>());
        assert_eq!(format!("{m:?}"),before);
        let inner = m.bodies[0].inner_shells[0];
        assert_eq!(r.orientations[inner].outward,Some(false));
        for u in &mut m.shells[inner].faces { u.reversed = !u.reversed; }
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.proven); assert!(!r.proven);
        assert_eq!(r.orientations[inner].outward,Some(true));
    }
    #[test]
    fn partitioned_boolean_cavity_uses_exact_hull_boundary_contacts() {
        let m = crate::operations::boolean(&crate::cuboid([0.;3],[10.;3]).unwrap(),
            &crate::cuboid([2.;3],[8.;3]).unwrap(),"difference").unwrap();
        let r = inspect(&m,1e-8,limits()).unwrap();
        assert!(r.boundary.agreement.all_equal && r.boundary.trim.all_valid);
        assert!(r.boundary.intersections.absence_proven,"next={:?} unresolved={:?}",r.boundary.intersections.pairs.next_pair,r.boundary.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved").map(|p|p.faces).collect::<Vec<_>>());
        assert!(!r.boundary.hull_contacts.is_empty());
        assert!(r.proven, "embedding={} nesting={:?}",r.boundary.proven,r.nesting.as_ref().map(|n|n.roles_consistent));
    }

}
