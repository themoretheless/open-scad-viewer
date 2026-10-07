//! Allowed coordinate-plane cap/wall contacts over the actual retained topology.
//! Wall/wall intersections, cap/cap relations and shell containment stay separate.
use crate::{Error, Model, Result};
use nurbs_core::sweep_cap_wall::{self, Boundary};
#[derive(Clone, Debug)]
pub struct Report {
    pub cap_certified: bool,
    pub planar_control_hull_certified: bool,
    pub all_cap_wall_contacts_certified: bool,
    pub separated_walls: Vec<usize>,
    pub allowed_boundaries: Vec<[usize; 2]>, // wall face, shared edge
    pub unresolved_walls: Vec<usize>,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
#[derive(Clone, Copy, Debug)]
pub struct Budgets {
    pub max_walls: usize,
    pub max_exact_work: u64,
    pub max_chart_cells: usize,
    pub max_trim_pairs: usize,
    pub max_trim_cells: usize,
    pub max_trim_domain_cells: usize,
}
fn invalid(message: &str) -> Error {
    Error::new("BREP_SWEEP_CAP_CONTACT_INVALID", message)
}
/// Original binary64 controls in one nondegenerate exact plane. Positive
/// weights are checked by surface validation before this predicate is called.
fn planar_control_hull(surface:&nurbs_core::surface::Surface,work:&mut u64,max_work:u64)->bool {
    if *work>=max_work{return false;}
    let report=nurbs_core::progressive_miter::cap_retained_plane_certificate::inspect(surface,max_work-*work);
    *work+=report.work;
    report.planar_control_hull_certified
}

fn uses(model: &Model, face: usize) -> Vec<&crate::Coedge> {
    let f = &model.faces[face];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|l| model.loops[l].coedges.iter())
        .collect()
}
fn boundary(model: &Model, wall: usize, p: &nurbs_core::curve::Curve) -> Option<Boundary> {
    let s = &model.faces[wall].surface;
    let candidates = [
        (0, s.knots_u[s.degree_u], Boundary::UMin),
        (0, s.knots_u[s.control_points.len()], Boundary::UMax),
        (1, s.knots_v[s.degree_v], Boundary::VMin),
        (1, s.knots_v[s.control_points[0].len()], Boundary::VMax),
    ];
    let found: Vec<_> = candidates
        .into_iter()
        .filter(|(k, x, _)| p.control_points.iter().all(|v| v[*k] == *x))
        .collect();
    if found.len() == 1 {
        Some(found[0].2)
    } else {
        None
    }
}
fn opposite(model: &Model, cap: usize, wall: usize, a: bool, b: bool) -> bool {
    let face_uses = |face| {
        model
            .shells
            .iter()
            .enumerate()
            .flat_map(move |(shell, s)| {
                s.faces
                    .iter()
                    .filter(move |f| f.face == face)
                    .map(move |f| (shell, f.reversed))
            })
            .collect::<Vec<_>>()
    };
    let c = face_uses(cap);
    let w = face_uses(wall);
    c.len() == 1 && w.len() == 1 && c[0].0 == w[0].0 && (a != c[0].1) != (b != w[0].1)
}
/// Positive rational control hulls exclude contacts with an exact oblique plane.
/// All predicates share the same budget as coedge identities in this report.
fn oblique_plane_contact(cap: &nurbs_core::surface::Surface,
    wall: &nurbs_core::surface::Surface, boundary: Option<Boundary>,
    work: &mut u64, max_work: u64) -> (bool,bool) {
    use cad_predicates::{AuthoredScalar,SourceArena,ToleranceContext,PredicateContext,Limits,Outcome,Sign};
    if *work>=max_work { return (false,false); }
    let anchors=[&cap.control_points[0][0],
        &cap.control_points[cap.control_points.len()-1][0],
        &cap.control_points[0][cap.control_points[0].len()-1]];
    let cap_points=cap.control_points.iter().flatten().collect::<Vec<_>>();
    let points=anchors.into_iter().chain(cap_points.iter().copied())
        .chain(wall.control_points.iter().flatten()).collect::<Vec<_>>();
    let values=points.iter().flat_map(|p|p.iter()).map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect();
    let Ok(source)=SourceArena::authored("sweep-cap-plane",1,values) else { return (false,false); };
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:max_work-*work,..Limits::default()},None);
    let refs=|i:usize|std::array::from_fn(|k|source.leaf(3*i+k).unwrap());
    let result=(|| -> Option<(bool,bool)> {
        let mut noncollinear=false;
        for axes in [[0,1],[1,2],[0,2]] {
            let p=|i:usize|axes.map(|k|source.leaf(3*i+k).unwrap());
            match cad_predicates::orient2d(&mut ctx,p(0),p(1),p(2)).ok()?.outcome {
                Outcome::Sign(Sign::Positive|Sign::Negative)=>{noncollinear=true;break;},
                Outcome::Sign(Sign::Zero)=>(), _=>return None,
            }
        }
        if !noncollinear { return None; }
        for i in 3..3+cap_points.len() {
            if cad_predicates::orient3d(&mut ctx,refs(0),refs(1),refs(2),refs(i)).ok()?.outcome
                != Outcome::Sign(Sign::Zero) { return None; }
        }
        let declared=boundary.map(|b|match b {
            Boundary::UMin=>(0,0),Boundary::UMax=>(0,wall.control_points.len()-1),
            Boundary::VMin=>(1,0),Boundary::VMax=>(1,wall.control_points[0].len()-1),
        });
        let mut side=None; let mut all_strict=true; let mut restricted=declared.is_some();
        let mut index=3+cap_points.len();
        for (u,row) in wall.control_points.iter().enumerate() {
            for (v,_) in row.iter().enumerate() {
                let Outcome::Sign(sign)=cad_predicates::orient3d(&mut ctx,refs(0),refs(1),refs(2),refs(index)).ok()?.outcome
                    else { return None; };
                index+=1;
                let on_boundary=declared.is_some_and(|(axis,end)|[u,v][axis]==end);
                if sign==Sign::Zero { all_strict=false; restricted &= on_boundary; }
                else {
                    restricted &= !on_boundary;
                    if side.is_some_and(|s|s!=sign) { return None; }
                    side=Some(sign);
                }
            }
        }
        Some((all_strict && side.is_some(),restricted && side.is_some()))
    })().unwrap_or((false,false));
    *work+=ctx.work_used();
    result
}
/// All listed caps are excluded from wall scope; cap/cap relations are not proved.
/// Invalid topology is refused. Unproved geometry or exhausted budgets preserve
/// unresolved face IDs and never promote allowed contact.
pub fn inspect(model: &Model, cap: usize, caps: &[usize], budget: Budgets) -> Result<Report> {
    model.validate()?;
    if caps.is_empty()
        || caps.len() > 16
        || model.faces.len() > 1024
        || !caps.contains(&cap)
        || caps.iter().any(|f| *f >= model.faces.len())
        || caps.iter().collect::<std::collections::BTreeSet<_>>().len() != caps.len()
        || budget.max_walls > 1024
        || budget.max_exact_work > 1000000
        || budget.max_chart_cells > 100000
        || budget.max_trim_pairs > 100000
        || budget.max_trim_cells > 100000
        || budget.max_trim_domain_cells > 1000000
    {
        return Err(invalid("Invalid cap selection or budgets"));
    }
    let walls: Vec<_> = (0..model.faces.len())
        .filter(|f| !caps.contains(f))
        .collect();
    let mut out = Report {
        cap_certified: false,
        planar_control_hull_certified: false,
        all_cap_wall_contacts_certified: false,
        separated_walls: Vec::new(),
        allowed_boundaries: Vec::new(),
        unresolved_walls: walls.clone(),
        exact_work: 0,
        reason: Some("cap-prerequisites-unproved"),
    };
    if walls.is_empty()
        || budget.max_chart_cells == 0
        || budget.max_trim_pairs == 0
        || budget.max_trim_cells == 0
        || budget.max_trim_domain_cells == 0
    {
        return Ok(out);
    }
    let face = &model.faces[cap];
    out.planar_control_hull_certified=planar_control_hull(&face.surface,&mut out.exact_work,budget.max_exact_work);
    if !nurbs_core::surface_regularity::inspect(&face.surface, budget.max_chart_cells)?
        .spanwise_regular
        || !nurbs_core::surface_injectivity::certify(&face.surface, 1)?.proven
    {
        return Ok(out);
    }
    let loops: Vec<_> = std::iter::once(face.outer)
        .chain(face.holes.iter().copied())
        .map(|l| {
            model.loops[l]
                .coedges
                .iter()
                .map(|c| c.pcurve.clone())
                .collect()
        })
        .collect();
    if loops.len() > 16 || loops.iter().map(Vec::len).sum::<usize>() > 256 {
        return Ok(out);
    }
    let domain = nurbs_core::trim_region_audit::inspect(
        &loops,
        1e-10,
        budget.max_trim_pairs,
        budget.max_trim_cells,
        budget.max_trim_domain_cells,
    )?;
    if domain.valid != Some(true) {
        return Ok(out);
    }
    let cap_uses = uses(model, cap);
    let exact = |coedge: &crate::Coedge, face: usize, work: &mut u64| -> Result<bool> {
        let proof = nurbs_core::curve_surface_agreement::verify_exact(
            &model.edges[coedge.edge].curve,
            &coedge.pcurve,
            &model.faces[face].surface,
            coedge.reversed,
            budget.max_exact_work - *work,
        )?;
        let Some(proof) = proof else {
            return Ok(false);
        };
        *work += proof.work_used;
        Ok(proof.outcome == cad_predicates::BezierIdentity::Equal)
    };
    let mut incidence = std::collections::BTreeMap::new();
    for f in 0..model.faces.len() {
        for c in uses(model, f) {
            incidence
                .entry(c.edge)
                .or_insert_with(Vec::new)
                .push((f, c));
        }
    }
    for c in &cap_uses {
        let incidents = &incidence[&c.edge];
        let other: Vec<_> = incidents.iter().filter(|(f, _)| *f != cap).collect();
        if incidents.len() != 2
            || other.len() != 1
            || !walls.contains(&other[0].0)
            || !opposite(model, cap, other[0].0, c.reversed, other[0].1.reversed)
            || !exact(c, cap, &mut out.exact_work)?
        {
            return Ok(out);
        }
    }
    out.cap_certified = true;
    out.reason = None;
    out.unresolved_walls.clear();
    for (index, &wall) in walls.iter().enumerate() {
        if index >= budget.max_walls {
            out.unresolved_walls.extend_from_slice(&walls[index..]);
            out.reason = Some("cap-wall-budget-exhausted");
            break;
        }
        let shared: Vec<_> = uses(model, wall)
            .into_iter()
            .filter(|w| cap_uses.iter().any(|c| c.edge == w.edge))
            .collect();
        let declaration = if shared.len() == 1 {
            boundary(model, wall, &shared[0].pcurve)
        } else {
            None
        };
        let plane = sweep_cap_wall::inspect(
            &face.surface,
            &[model.faces[wall].surface.clone()],
            &[declaration],
            1,
        )?;
        let oblique=if plane.plane_axis.is_none() {
            oblique_plane_contact(&face.surface,&model.faces[wall].surface,declaration,
                &mut out.exact_work,budget.max_exact_work)
        } else { (false,false) };
        if (plane.separated_walls == vec![0] || oblique.0) && shared.is_empty() {
            out.separated_walls.push(wall);
            continue;
        }
        let mut allowed = false;
        if (plane.boundary_restricted_walls == vec![0] || oblique.1) && shared.len() == 1 {
            if let Some(side) = declaration {
                allowed = sweep_cap_wall::covers_boundary(
                    &model.faces[wall].surface,
                    &shared[0].pcurve,
                    side,
                )? && exact(shared[0], wall, &mut out.exact_work)?;
            }
        }
        if allowed {
            out.allowed_boundaries.push([wall, shared[0].edge]);
        } else {
            out.unresolved_walls.push(wall);
            out.reason = Some("cap-wall-contact-unproved");
        }
    }
    out.all_cap_wall_contacts_certified = out.unresolved_walls.is_empty();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retained_oblique_plane_requires_exact_original_controls_and_shared_work(){
        let mut surface=nurbs_core::surface::Surface {
            degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,1.]],vec![vec![1.,0.,1.],vec![1.,1.,2.]]],
            weights:vec![vec![1.,0.5],vec![2.,1.]],periodic_u:false,periodic_v:false,
        };
        surface.validate().unwrap();
        let mut work=0;
        assert!(planar_control_hull(&surface,&mut work,1000000));
        assert!(work>0 && work<=1000000);
        let mut exhausted=work;
        assert!(!planar_control_hull(&surface,&mut exhausted,work));
        assert_eq!(exhausted,work);
        surface.control_points[1][1][2]=f64::from_bits(2f64.to_bits()+1);
        assert!(!planar_control_hull(&surface,&mut 0,1000000));
        surface.control_points[1]=surface.control_points[0].clone();
        assert!(!planar_control_hull(&surface,&mut 0,1000000));
    }
    #[test]
    fn corrected_spatial_cap_excludes_all_wall_interiors_with_shared_exact_work() {
        let profiles=vec![
            nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.5).unwrap(),
            nurbs_core::primitives::circle([0.;3],[0.,0.,1.],0.2).unwrap().reverse().unwrap(),
        ];
        let points=[[0.,0.,0.],[0.,0.,10.],[10.,0.,10.],[10.,10.,15.]];
        let mut sections=nurbs_core::paths::miter_sections(&profiles,&points,[1.,0.,0.],2.).unwrap();
        let last=sections.len()-1;
        sections[last]=nurbs_core::section_projection::project(&sections[last],1,[0.,-0.5],17.5,
            2_f64.powi(-40),1e-9,1000000).unwrap().curves.unwrap();
        let sections=sections.into_iter().map(|row|row.into_iter().map(|c|vec![c]).collect()).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let budgets=Budgets{max_walls:24,max_exact_work:1000000,max_chart_cells:10000,
            max_trim_pairs:10000,max_trim_cells:100000,max_trim_domain_cells:1000000};
        let report=inspect(&model,25,&[24,25],budgets).unwrap();
        assert!(report.cap_certified && report.planar_control_hull_certified && report.all_cap_wall_contacts_certified,"{report:?}");
        assert_eq!(report.allowed_boundaries.len(),8);
        assert_eq!(report.separated_walls.len(),16);
        assert!(report.exact_work<=budgets.max_exact_work);
        let joint=crate::boundary_embedding::inspect_sweep(&model,1e-8,
            crate::boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits{
                    pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
            20000,&[24,25],budgets).unwrap();
        assert!(joint.agreement.all_equal && joint.intersections.faces.all_faces_injective);
        assert!(joint.cap_contacts.iter().all(|(_,r)|r.all_cap_wall_contacts_certified));
        let unresolved=joint.intersections.pairs.pairs.iter().filter(|p|p.reason=="pair-unresolved")
            .map(|p|p.faces).collect::<Vec<_>>();
        println!("spatial remaining pairs: {unresolved:?}; boundary proven: {}",joint.proven);
        assert!(joint.proven);
        for pair in [[9,18],[15,20]] {
            assert!(crate::boundary_hull_contact::certify(&model,pair).unwrap().vertex.is_some());
        }
        let volume_limits=crate::volume_validity::Limits {
                boundary:crate::boundary_embedding::Limits{exact_work:1000000,trim_pairs:10000,trim_cells:100000,
                    trim_domain_cells:1000000,spans:1000,contacts:crate::face_contacts::Limits{
                        pairs:10000,cells:100000,domain_cells:1000000,cells_per_pair:1000,domain_cells_per_pair:10000}},
                nesting_pairs:1000,nesting_cells:100000,nesting_domain_cells:1000000,
                orientation_cells:100000,orientation_domain_cells:1000000,orientation_spans:100,
            };
        let volume=crate::volume_validity::inspect_sweep(&model,1e-8,volume_limits,20000,&[24,25],budgets).unwrap();
        println!("spatial volume proven: {}; orientations: {:?}",volume.proven,
            volume.orientations.iter().map(|o|(o.shell,o.outward,o.attempts.len())).collect::<Vec<_>>());
        assert!(volume.proven);
        let mut inward=model.clone();
        for face in &mut inward.shells[0].faces {face.reversed = !face.reversed;}
        let wrong=crate::volume_validity::inspect_sweep(&inward,1e-8,volume_limits,20000,&[24,25],budgets).unwrap();
        assert!(wrong.boundary.proven);
        assert!(!wrong.proven);
        assert_eq!(wrong.orientations[0].outward,Some(false));
        for pair in [[16,19],[17,18],[20,23],[21,22]] {
            assert!(joint.intersections.pairs.pairs.iter().any(|p|p.faces==pair && p.boundary.is_some()));
        }
        for pair in [[16,20],[16,21],[16,22],[16,23],[18,20],[18,21],[18,22],[18,23],[21,23]] {
            assert!(joint.intersections.pairs.pairs.iter().any(|p|p.faces==pair && p.reason=="pair-disjoint"));
        }
        for pair in [[9,16],[10,19],[13,20],[14,23]] {
            let certificate=crate::boundary_hull_contact::certify(&model,pair).unwrap();
            assert!(certificate.vertex.is_some());
            assert!(certificate.edges.is_empty());
        }
        let vertex=crate::boundary_hull_contact::certify(&model,[9,16]).unwrap().vertex.unwrap();
        let extra=model.faces[9].surface.control_points.iter().flatten()
            .find(|p|p.as_slice()!=model.vertices[vertex].point).unwrap().clone();
        let mut overlap=model.clone();
        overlap.faces[16].surface.control_points[0][0]=extra;
        assert!(crate::boundary_hull_contact::certify(&overlap,[9,16]).is_none());
        let limited=inspect(&model,25,&[24,25],Budgets{max_exact_work:0,..budgets}).unwrap();
        assert!(!limited.all_cap_wall_contacts_certified);
        let mut cap=model.faces[25].surface.clone();
        cap.control_points[0][0][1]=cap.control_points[0][0][1].next_up();
        let mut work=0;
        assert_eq!(oblique_plane_contact(&cap,&model.faces[16].surface,Some(Boundary::VMax),
            &mut work,1000000),(false,false));
    }
}
