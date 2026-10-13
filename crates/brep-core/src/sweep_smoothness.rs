//! Native ownership and sufficient exact jets of retained wall seams.
//! Caps, trim/world-edge identity and global shell validity remain separate.
use crate::{Error, MAX_COEDGES, MAX_ENTITIES, MAX_FACES, Model, Result};
use nurbs_core::{
    continuity::{self, ProjectiveSeam, ProjectiveSeamCollectionReport},
    curve::Curve,
    surface::Surface,
    sweep_cap_wall::{self, Boundary},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Profile,
    Station,
}
pub struct AxisReport {
    pub max_work: u64,
    pub exact_work: u64,
    pub extraction_complete: bool,
    pub unclassified_faces: Vec<usize>,
    pub unpaired_edges: Vec<usize>,
    pub cap_edges: Vec<usize>,
    pub edge_ids: Vec<usize>,
    pub g2: ProjectiveSeamCollectionReport,
    pub g1: Option<ProjectiveSeamCollectionReport>,
    pub g1_certified: bool,
    pub g2_certified: bool,
}
pub struct ProfileReport {
    pub profile: AxisReport,
    pub station: AxisReport,
    pub total_exact_work: u64,
    pub cap_continuity: &'static str,
    pub station_continuity: &'static str,
}
fn require(value: bool, message: &str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error::new("BREP_SWEEP_SMOOTHNESS_INVALID", message))
    }
}
fn boundary(s: &Surface, p: &Curve, axis: Axis) -> Option<&'static str> {
    if p.degree != 1
        || p.control_points.len() != 2
        || p.weights.len() != 2
        || p.weights[0] != p.weights[1]
        || p.weights[0] <= 0.
        || p.control_points.iter().any(|v| v.len() != 2)
        || s.validate().is_err()
    {
        return None;
    }
    let coordinate = usize::from(axis == Axis::Station);
    let other = 1 - coordinate;
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    let a = &p.control_points[0];
    let b = &p.control_points[1];
    if a[coordinate] != b[coordinate]
        || !((a[other] == domains[other][0] && b[other] == domains[other][1])
            || (a[other] == domains[other][1] && b[other] == domains[other][0]))
    {
        return None;
    }
    let end = if a[coordinate] == domains[coordinate][0] {
        0
    } else if a[coordinate] == domains[coordinate][1] {
        1
    } else {
        return None;
    };
    let (label, side) = match (axis, end) {
        (Axis::Profile, 0) => ("uMin", Boundary::UMin),
        (Axis::Profile, _) => ("uMax", Boundary::UMax),
        (Axis::Station, 0) => ("vMin", Boundary::VMin),
        (Axis::Station, _) => ("vMax", Boundary::VMax),
    };
    sweep_cap_wall::covers_boundary(s, p, side)
        .ok()
        .filter(|&v| v)
        .map(|_| label)
}
fn inspect_owned_seams(patches:&[Surface],seams:&[ProjectiveSeam],max_work:u64)->Result<ProjectiveSeamCollectionReport> {
    // Sharp joins invalidate the whole-axis claim, but cannot hide later
    // applicable profile or station joins. Preserve the shared work ceiling.
    continuity::inspect_projective_seam_collection_with_rational_scale_proposals(patches,seams,max_work)
}
pub fn inspect_axis(model:&Model,cap_faces:&[usize],axis:Axis,max_work:u64)->Result<AxisReport> {
    inspect_axis_impl(model,cap_faces,axis,max_work,false)
}
fn inspect_axis_impl(
    model: &Model,
    cap_faces: &[usize],
    axis: Axis,
    max_work: u64,
    defer_profile: bool,
) -> Result<AxisReport> {
    require(
        max_work <= 2000000
            && model.faces.len() <= MAX_FACES
            && model.edges.len() <= MAX_ENTITIES
            && model.loops.len() <= MAX_ENTITIES,
        "Invalid retained smoothness limits",
    )?;
    require(
        model.loops.iter().map(|w| w.coedges.len()).sum::<usize>() <= MAX_COEDGES,
        "Too many retained smoothness coedges",
    )?;
    let mut caps = vec![false; model.faces.len()];
    let mut cap_owned = vec![false; model.edges.len()];
    for &face in cap_faces {
        require(
            face < caps.len() && !caps[face],
            "Invalid station smoothness cap scope",
        )?;
        caps[face] = true;
        let f = &model.faces[face];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            let wire = model
                .loops
                .get(wire)
                .ok_or_else(|| Error::new("BREP_SWEEP_SMOOTHNESS_INVALID", "Invalid cap loop"))?;
            for c in &wire.coedges {
                require(c.edge < cap_owned.len(), "Invalid cap edge")?;
                cap_owned[c.edge] = true;
            }
        }
    }
    let mut uses = vec![Vec::<(usize, &'static str)>::new(); model.edges.len()];
    let mut edge_order = Vec::new();
    let mut unclassified_faces = Vec::new();
    for (index, face) in model.faces.iter().enumerate() {
        if caps[index] {
            continue;
        }
        let mut found = [false; 2];
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            let wire = model
                .loops
                .get(wire)
                .ok_or_else(|| Error::new("BREP_SWEEP_SMOOTHNESS_INVALID", "Invalid wall loop"))?;
            for c in &wire.coedges {
                require(c.edge < uses.len(), "Invalid wall edge")?;
                if let Some(label) = boundary(&face.surface, &c.pcurve, axis) {
                    found[usize::from(label.ends_with("Max"))] = true;
                    if uses[c.edge].is_empty() {
                        edge_order.push(c.edge);
                    }
                    uses[c.edge].push((index, label));
                }
            }
        }
        if !found.into_iter().all(|v| v) {
            unclassified_faces.push(index);
        }
    }
    let mut seams = Vec::new();
    let mut edge_ids = Vec::new();
    let mut cap_edges = Vec::new();
    let mut unpaired_edges = Vec::new();
    for edge in edge_order {
        let list = &uses[edge];
        if axis == Axis::Station && list.len() == 1 && cap_owned[edge] {
            cap_edges.push(edge);
            continue;
        }
        if list.len() != 2 || axis == Axis::Station && cap_owned[edge] {
            unpaired_edges.push(edge);
            continue;
        }
        let (a, ab) = list[0];
        let (b, bb) = list[1];
        let normal_scale = if axis == Axis::Profile {
            1.
        } else {
            continuity::propose_boundary_normal_scale(
                &model.faces[a].surface,
                &model.faces[b].surface,
                ab,
                bb,
            )?
        };
        seams.push(ProjectiveSeam {
            patches: [a, b],
            boundaries: [ab.into(), bb.into()],
            order: 2,
            normal_scale,
        });
        edge_ids.push(edge);
    }
    let patches = model
        .faces
        .iter()
        .map(|f| f.surface.clone())
        .collect::<Vec<_>>();
    let extraction_complete = unclassified_faces.is_empty() && unpaired_edges.is_empty();
    let inspect=|seams:&[ProjectiveSeam],budget| {
        if defer_profile && axis==Axis::Profile {
            continuity::inspect_projective_seam_collection_until_refused(&patches,seams,budget)
        } else {inspect_owned_seams(&patches,seams,budget)}
    };
    let mut g2 = inspect(&seams,max_work)?;
    if !extraction_complete {
        g2.certified = false;
        g2.certified_order = None;
    }
    let mut exact_work = g2.exact_work;
    let g1 = if g2.certified {
        None
    } else {
        for seam in &mut seams {
            seam.order = 1;
        }
        let mut proof =
            inspect(&seams,max_work-exact_work)?;
        if !extraction_complete {
            proof.certified = false;
            proof.certified_order = None;
        }
        exact_work += proof.exact_work;
        Some(proof)
    };
    let g2_certified = g2.certified;
    let g1_certified =
        extraction_complete && (g2_certified || g1.as_ref().is_some_and(|r| r.certified));
    Ok(AxisReport {
        max_work,
        exact_work,
        extraction_complete,
        unclassified_faces,
        unpaired_edges,
        cap_edges,
        edge_ids,
        g2,
        g1,
        g1_certified,
        g2_certified,
    })
}
/// Continue only skipped owned profile seams using unspent shared work.
/// Previously proved prefix/station results are never replayed or discarded.
fn finish_deferred_profile(model:&Model,caps:&[usize],profile:&mut AxisReport,max_work:u64)->Result<()> {
    let deferred=|r:&ProjectiveSeamCollectionReport|r.seams.iter().any(|s|s.reason=="seam-audit-skipped-after-refusal");
    if !deferred(&profile.g2) && !profile.g1.as_ref().is_some_and(deferred) {return Ok(());}
    let patches=model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>();
    let mut owned=vec![Vec::new();model.edges.len()];
    for (face,f) in model.faces.iter().enumerate() {
        if caps.contains(&face) {continue;}
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[wire].coedges {
                require(c.edge<owned.len(),"Invalid deferred profile owner edge")?;
                if let Some(label)=boundary(&f.surface,&c.pcurve,Axis::Profile) {owned[c.edge].push((face,label));}
            }
        }
    }
    let declarations=|indices:&[usize],order|->Result<Vec<ProjectiveSeam>> {
        indices.iter().map(|&i| {
            let uses=&owned[profile.edge_ids[i]];
            require(uses.len()==2,"Deferred profile seam lost native ownership")?;
            Ok(ProjectiveSeam {patches:[uses[0].0,uses[1].0],
                boundaries:[uses[0].1.into(),uses[1].1.into()],order,normal_scale:1.})
        }).collect()
    };
    let skipped=|r:&ProjectiveSeamCollectionReport|r.seams.iter().enumerate()
        .filter(|(_,s)|s.reason=="seam-audit-skipped-after-refusal").map(|(i,_)|i).collect::<Vec<_>>();
    let mut spent=0;
    let indices=skipped(&profile.g2);
    if !indices.is_empty() {
        let seams=declarations(&indices,2)?;
        let proof=inspect_owned_seams(&patches,&seams,max_work)?;
        spent+=proof.exact_work;profile.g2.exact_work+=proof.exact_work;
        for (i,proof) in indices.into_iter().zip(proof.seams) {profile.g2.seams[i]=proof;}
    }
    if let Some(g1)=&mut profile.g1 {
        let mut indices=skipped(g1);
        // An already owned G2 proof implies G1 without spending proof work twice.
        indices.retain(|&i| {
            let g2=&profile.g2.seams[i];
            if g2.certified {
                g1.seams[i]=continuity::ExactStripJetReport {certified:true,exact_identity:g2.exact_identity,
                    regularity_certified:g2.regularity_certified,work:0,reason:"g1-implied-by-owned-g2"};
                false
            } else {true}
        });
        if !indices.is_empty() {
            let seams=declarations(&indices,1)?;
            let proof=inspect_owned_seams(&patches,&seams,max_work-spent)?;
            spent+=proof.exact_work;g1.exact_work+=proof.exact_work;
            for (i,proof) in indices.into_iter().zip(proof.seams) {g1.seams[i]=proof;}
        }
    }
    require(spent<=max_work,"Deferred profile exceeded native shared work")?;
    profile.exact_work+=spent;
    let refresh=|r:&mut ProjectiveSeamCollectionReport,order| {
        r.unresolved_seams=r.seams.iter().enumerate().filter(|(_,s)|!s.certified).map(|(i,_)|i).collect();
        r.certified=profile.extraction_complete && !r.seams.is_empty() && r.unresolved_seams.is_empty();
        r.certified_order=r.certified.then_some(order);
    };
    refresh(&mut profile.g2,2);
    if let Some(g1)=&mut profile.g1 {refresh(g1,1);}
    profile.g2_certified=profile.g2.certified;
    profile.g1_certified=profile.extraction_complete && (profile.g2_certified||profile.g1.as_ref().is_some_and(|r|r.certified));
    Ok(())
}

pub fn inspect_profile(model: &Model, cap_faces: &[usize], max_work: u64) -> Result<ProfileReport> {
    // Preserve the existing successful profile prefix and station proof first.
    // Only unused caller work is then spent on deferred profile seams.
    let mut profile = inspect_axis_impl(model, cap_faces, Axis::Profile, max_work, true)?;
    let station = inspect_axis(
        model,
        cap_faces,
        Axis::Station,
        max_work - profile.exact_work,
    )?;
    let remaining=max_work-profile.exact_work-station.exact_work;
    finish_deferred_profile(model,cap_faces,&mut profile,remaining)?;
    let total_exact_work = profile.exact_work + station.exact_work;
    let station_continuity = if station.g2_certified {
        "G2"
    } else if station.g1_certified {
        "G1"
    } else {
        "C0"
    };
    Ok(ProfileReport {
        profile,
        station,
        total_exact_work,
        cap_continuity: if cap_faces.is_empty() { "absent" } else { "C0" },
        station_continuity,
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_sharp_first_station_does_not_hide_later_smooth_station_joins() {
        let shifts=[[0.,0.,0.],[1.,0.,1.],[1.,0.,2.],[1.,0.,4.]];
        let corners=[[0.,0.],[1.,0.],[1.,1.],[0.,1.],[0.,0.]];
        let sections=shifts.iter().map(|p|vec![corners.windows(2).map(|v|
            nurbs_core::primitives::line([v[0][0]+p[0],v[0][1],p[2]],[v[1][0]+p[0],v[1][1],p[2]]).unwrap()
        ).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();let before=model.clone();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let report=inspect_axis(&model,&caps,Axis::Station,2000000).unwrap();
        assert!(report.extraction_complete);assert!(!report.g2_certified);
        assert_eq!(report.g2.seams.len(),8);
        assert_eq!(report.g2.seams.iter().filter(|s|s.certified).count(),4);
        assert!(report.exact_work<=2000000);assert_eq!(model,before);
        let denied=inspect_axis(&model,&caps,Axis::Station,0).unwrap();
        assert!(!denied.g2_certified);assert_eq!(denied.exact_work,0);
        assert!(denied.g2.seams.iter().all(|s|!s.certified));
    }

    use super::*;
    #[test]
    fn a_sharp_first_profile_does_not_hide_later_smooth_profile_joins() {
        let corners=[[0.,0.],[1.,0.],[1.,1.],[1.,2.],[0.,2.],[0.,0.]];
        let sections=[0.,1.].into_iter().map(|z|vec![corners.windows(2).map(|p|
            nurbs_core::primitives::line([p[0][0],p[0][1],z],[p[1][0],p[1][1],z]).unwrap()
        ).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();let before=model.clone();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let report=inspect_axis(&model,&caps,Axis::Profile,2_000_000).unwrap();
        assert!(report.extraction_complete);assert!(!report.g2_certified);assert!(!report.g1_certified);
        assert_eq!(report.g2.seams.len(),5);assert!(!report.g2.seams[0].certified);
        assert_eq!(report.g2.seams.iter().filter(|s|s.certified).count(),1);
        assert!(report.g2.seams.iter().all(|s|s.reason!="seam-audit-skipped-after-refusal"));
        assert!(report.g1.as_ref().unwrap().seams.iter().any(|s|s.certified));
        assert!(report.exact_work<=2_000_000);assert_eq!(model,before);
        let combined=inspect_profile(&model,&caps,2_000_000).unwrap();
        assert_eq!(combined.profile.g2.seams.iter().filter(|s|s.certified).count(),1);
        assert!(combined.profile.g2.seams.iter().all(|s|s.reason!="seam-audit-skipped-after-refusal"));
        assert!(combined.total_exact_work<=2_000_000);assert_eq!(model,before);
        let denied=inspect_axis(&model,&caps,Axis::Profile,0).unwrap();
        assert!(!denied.g2_certified);assert_eq!(denied.exact_work,0);
        assert!(denied.g2.seams.iter().all(|s|!s.certified));
    }

    #[test]
    fn deferred_profile_completion_preserves_positive_prefix_and_station_proofs() {
        let corners=[[0.,0.],[1.,0.],[2.,0.],[2.,1.],[2.,2.],[0.,2.],[0.,0.]];
        let sections=[0.,1.,3.].into_iter().map(|z|vec![corners.windows(2).map(|p|
            nurbs_core::primitives::line([p[0][0],p[0][1],z],[p[1][0],p[1][1],z]).unwrap()
        ).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();let before=model.clone();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let prefix=inspect_axis_impl(&model,&caps,Axis::Profile,2_000_000,true).unwrap();
        assert!(prefix.g2.seams.iter().any(|s|s.certified));
        assert!(prefix.g2.seams.iter().any(|s|s.reason=="seam-audit-skipped-after-refusal"));
        let report=inspect_profile(&model,&caps,2_000_000).unwrap();
        assert!(!report.profile.g2_certified && report.station.g2_certified);
        assert_eq!(report.profile.g2.seams.iter().filter(|s|s.certified).count(),4);
        assert!(report.profile.g2.seams.iter().all(|s|s.reason!="seam-audit-skipped-after-refusal"));
        for (old,new) in prefix.g2.seams.iter().zip(&report.profile.g2.seams) {
            if old.certified {assert!(new.certified);assert_eq!(new.work,old.work);assert_eq!(new.reason,old.reason);}
        }
        assert_eq!(report.profile.exact_work,report.profile.g2.exact_work+report.profile.g1.as_ref().map_or(0,|r|r.exact_work));
        assert_eq!(report.total_exact_work,report.profile.exact_work+report.station.exact_work);
        assert!(report.total_exact_work<=2_000_000);assert_eq!(model,before);
    }
    #[test]
    fn unequal_profile_strip_speeds_prove_only_the_actual_smooth_seam() {
        let corners=[[0.,0.],[1.,0.],[3.,0.],[3.,1.],[0.,1.],[0.,0.]];
        let sections=[0.,1.].into_iter().map(|z|vec![corners.windows(2).map(|p|
            nurbs_core::primitives::line([p[0][0],p[0][1],z],[p[1][0],p[1][1],z]).unwrap()
        ).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let before=model.clone();
        let unit=continuity::inspect_projective_seam_collection_until_refused(
            &model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>(),
            &[ProjectiveSeam {patches:[0,1],boundaries:["uMax".into(),"uMin".into()],order:2,normal_scale:1.}],
            2000000,
        ).unwrap();
        assert!(unit.certified,"Exact binary-ratio fallback must cover unequal profile speeds even with the unit proposal");
        let proof=inspect_axis(&model,&caps,Axis::Profile,2000000).unwrap();
        assert!(proof.extraction_complete);
        assert!(!proof.g2_certified && !proof.g1_certified,"Sharp corners remain C0");
        assert!(proof.g2.seams[0].certified,"The collinear seam has unequal represented speeds: {:?}",proof.g2.seams);
        assert!(proof.g2.seams.iter().skip(1).any(|s|!s.certified));
        assert_eq!(model.faces[0].surface.control_points,before.faces[0].surface.control_points);
        let zero=inspect_axis(&model,&caps,Axis::Profile,0).unwrap();
        assert!(!zero.g2_certified && zero.g2.seams.iter().all(|s|!s.certified));
        let short=inspect_axis(&model,&caps,Axis::Profile,proof.g2.seams[0].work-1).unwrap();
        assert!(!short.g2.seams[0].certified);
        let mut kink=model.clone();
        for p in &mut kink.faces[0].surface.control_points[0] {p[1]=f64::EPSILON;}
        let damaged=inspect_axis(&kink,&caps,Axis::Profile,2000000).unwrap();
        assert!(!damaged.g2.seams[0].certified,"A nonzero transverse kink must remain unproved");
    }
    #[test]
    fn unequal_profile_closing_seam_uses_owned_surfaces_and_exact_ratio() {
        let corners=[[1.,0.],[3.,0.],[3.,1.],[0.,1.],[0.,0.],[1.,0.]];
        let sections=[0.,1.].into_iter().map(|z|vec![corners.windows(2).map(|p|
            nurbs_core::primitives::line([p[0][0],p[0][1],z],[p[1][0],p[1][1],z]).unwrap()
        ).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let seams=[ProjectiveSeam {patches:[0,4],boundaries:["uMin".into(),"uMax".into()],order:2,normal_scale:1.}];
        assert!(model.loops[model.faces[0].outer].coedges.iter().any(|a|
            boundary(&model.faces[0].surface,&a.pcurve,Axis::Profile)==Some("uMin")
                && model.loops[model.faces[4].outer].coedges.iter().any(|b|
                    a.edge==b.edge && boundary(&model.faces[4].surface,&b.pcurve,Axis::Profile)==Some("uMax"))
        ),"The tested closing seam must be a shared owned world edge");
        let audit=|model:&Model,work|continuity::inspect_projective_seam_collection_until_refused(
            &model.faces.iter().map(|f|f.surface.clone()).collect::<Vec<_>>(),&seams,work
        ).unwrap();
        let proof=audit(&model,2000000);
        assert!(proof.certified && proof.seams[0].regularity_certified,"{:?}",proof.seams);
        assert!(!audit(&model,proof.exact_work-1).certified);
        assert!(!audit(&model,0).certified);
        let mut kink=model.clone();
        for p in &mut kink.faces[0].surface.control_points[1] {p[1]=f64::EPSILON;}
        assert!(!audit(&kink,2000000).certified);
    }
    #[test]
    fn sharp_rational_profiles_preserve_nonuniform_station_g2_work() {
        let corners=[[-1.,-1.],[1.,-1.],[1.,1.],[-1.,1.],[-1.,-1.]];
        let mut z=0.;
        let sections=(0..49).map(|station|{
            if station>0 {z+=if station%2==0 {2.} else {1.};}
            vec![corners.windows(2).map(|pair|Curve {
                degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
                control_points:vec![vec![pair[0][0],pair[0][1],z],
                    vec![(pair[0][0]+pair[1][0])*0.5,(pair[0][1]+pair[1][1])*0.5,z],
                    vec![pair[1][0],pair[1][1],z]],
                weights:vec![1.,0.5,1.],periodic:false,
            }).collect::<Vec<_>>()]
        }).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let isolated=inspect_axis(&model,&caps,Axis::Station,2000000).unwrap();
        assert!(isolated.extraction_complete&&isolated.g2_certified,"station work={}",isolated.exact_work);
        let combined=inspect_profile(&model,&caps,2000000).unwrap();
        assert!(!combined.profile.g1_certified&&!combined.profile.g2_certified);
        assert!(combined.station.g2_certified,"combined work={}",combined.total_exact_work);
        assert_eq!(combined.station_continuity,"G2");
        assert!(combined.total_exact_work<=2000000);
        assert_eq!(combined.profile.g2.seams.len(),combined.profile.edge_ids.len());
        assert_eq!(combined.profile.g2.unresolved_seams.len(),combined.profile.edge_ids.len());
        assert!(combined.profile.g2.seams.iter().skip(1).all(|s| !s.certified && !s.exact_identity));
        assert!(combined.profile.g2.seams.iter().all(|s|s.reason!="seam-audit-skipped-after-refusal"));
        // Deferred negative profile checks are optional extra work. The
        // minimum station control is the original prefix plus station proof,
        // not the total after spending spare work on the deferred profile.
        let prefix=inspect_axis_impl(&model,&caps,Axis::Profile,2_000_000,true).unwrap();
        let required=prefix.exact_work+isolated.exact_work;
        assert!(required<=combined.total_exact_work);
        let short=inspect_profile(&model,&caps,required-1).unwrap();
        assert!(!short.station.g2_certified);
        assert!(short.total_exact_work<=combined.total_exact_work-1);
        eprintln!("sharp-profile independent station G2 work={} combined profile={} total={}",isolated.exact_work,combined.profile.exact_work,combined.total_exact_work);
    }
    #[test]
    fn profile_g1_curvature_jump_keeps_independent_station_g2() {
        let directions=[[1.,0.],[0.,1.],[-1.,0.],[0.,-1.],[1.,0.]];
        let sections=[0.,1.,3.].into_iter().map(|z|vec![directions.windows(2).enumerate().map(|(i,pair)|Curve {
            degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![pair[0][0],pair[0][1],z],
                vec![pair[0][0]+pair[1][0],pair[0][1]+pair[1][1],z],
                vec![pair[1][0],pair[1][1],z]],
            weights:vec![1.,if i==0 {1.} else {0.5},1.],periodic:false,
        }).collect::<Vec<_>>()]).collect::<Vec<_>>();
        let model=crate::rational_section_loft(&sections).unwrap();
        let caps=[model.faces.len()-2,model.faces.len()-1];
        let proof=inspect_profile(&model,&caps,2000000).unwrap();
        assert!(proof.profile.extraction_complete&&proof.profile.g1_certified&&!proof.profile.g2_certified);
        assert_eq!(proof.profile.g1.as_ref().unwrap().certified_order,Some(1));
        assert!(proof.station.g2_certified);
        assert_eq!(proof.station_continuity,"G2");
        assert_eq!(proof.cap_continuity,"C0");
        assert!(proof.total_exact_work<=2000000);
    }
    #[test]
    fn actual_circle_walls_own_profile_station_and_cap_sets_in_one_budget() {
        for stations in [&[0., 5., 10.][..], &[0., 3., 10., 21., 34.][..]] {
            let sections = stations
                .iter()
                .map(|&z| {
                    vec![vec![
                        nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                    ]]
                })
                .collect::<Vec<_>>();
            let model = crate::rational_section_loft(&sections).unwrap();
            let caps = [model.faces.len() - 2, model.faces.len() - 1];
            let r = inspect_profile(&model, &caps, 2000000).unwrap();
            assert!(
                r.profile.extraction_complete && r.profile.g2_certified && r.profile.g1_certified
            );
            assert!(
                r.station.extraction_complete && r.station.g2_certified && r.station.g1_certified
            );
            assert_eq!(r.station.edge_ids.len(), 4 * (stations.len() - 2));
            assert_eq!(r.station.cap_edges.len(), 8);
            assert_eq!(
                r.total_exact_work,
                r.profile.exact_work + r.station.exact_work
            );
            assert!(r.total_exact_work <= 2000000);
            let short = inspect_profile(&model, &caps, r.profile.exact_work - 1).unwrap();
            assert!(!short.profile.g2_certified);
            let mut damaged = model;
            let wire = damaged.faces[0].outer;
            let c = damaged.loops[wire]
                .coedges
                .iter_mut()
                .find(|c| c.pcurve.control_points[0][0] == c.pcurve.control_points[1][0])
                .unwrap();
            c.pcurve.control_points[1][1] = 0.5;
            let r = inspect_profile(&damaged, &caps, 2000000).unwrap();
            assert!(
                !r.profile.extraction_complete
                    && !r.profile.g1_certified
                    && !r.profile.g2_certified
            );
            assert!(
                r.profile.unclassified_faces.contains(&0) && !r.profile.unpaired_edges.is_empty()
            );
        }
    }
    #[test]
    fn malformed_boundary_and_invalid_topology_never_produce_positive_ownership() {
        let sections = [0., 4., 12.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        let mut damaged = model.clone();
        let wire = damaged.faces[0].outer;
        for c in &mut damaged.loops[wire].coedges {
            if c.pcurve.control_points[0][0] == c.pcurve.control_points[1][0] {
                c.pcurve.knots = vec![0., 0., 1.];
            }
        }
        let r = inspect_profile(&damaged, &caps, 2000000).unwrap();
        assert!(
            !r.profile.extraction_complete && !r.profile.g1_certified && !r.profile.g2_certified
        );
        let mut invalid = model.clone();
        invalid.faces[0].outer = invalid.loops.len();
        assert!(inspect_profile(&invalid, &caps, 2000000).is_err());
        let mut invalid = model.clone();
        invalid.loops[wire].coedges[0].edge = invalid.edges.len();
        assert!(inspect_profile(&invalid, &caps, 2000000).is_err());
        assert!(inspect_profile(&model, &[caps[0], caps[0]], 2000000).is_err());
        assert!(inspect_profile(&model, &caps, 2000001).is_err());
        let zero = inspect_profile(&model, &caps, 0).unwrap();
        assert!(zero.profile.extraction_complete && zero.station.extraction_complete);
        assert!(!zero.profile.g1_certified && !zero.station.g1_certified);
        assert_eq!(zero.total_exact_work, 0);
    }

    #[test]
    fn station_g1_is_distinct_from_g2_and_spends_only_shared_remainder() {
        let sections = [0., 6., 12.]
            .iter()
            .map(|&z| {
                vec![vec![
                    nurbs_core::primitives::circle([0., 0., z], [0., 0., 1.], 0.5).unwrap(),
                ]]
            })
            .collect::<Vec<_>>();
        let mut model = crate::rational_section_loft(&sections).unwrap();
        let caps = [model.faces.len() - 2, model.faces.len() - 1];
        for face in &mut model.faces[..caps[0]] {
            let s = &mut face.surface;
            for row in &mut s.control_points {
                let a = row[0].clone();
                let b = row[1].clone();
                let shift = if a[2] == 6. { 0.25 } else { 0. };
                *row = vec![
                    a.clone(),
                    vec![a[0], a[1], a[2] + 2.],
                    vec![b[0] + shift, b[1], b[2] - 2.],
                    b,
                ];
            }
            for row in &mut s.weights {
                *row = vec![row[0], row[0], row[1], row[1]];
            }
            s.degree_v = 3;
            s.knots_v = vec![0., 0., 0., 0., 1., 1., 1., 1.];
        }
        let r = inspect_axis(&model, &caps, Axis::Station, 2000000).unwrap();
        assert!(r.extraction_complete && r.g1_certified && !r.g2_certified);
        assert_eq!(r.g1.as_ref().unwrap().certified_order, Some(1));
        assert_eq!(
            r.exact_work,
            r.g2.exact_work + r.g1.as_ref().unwrap().exact_work
        );
        let cutoff = inspect_axis(&model, &caps, Axis::Station, r.g2.exact_work).unwrap();
        assert!(cutoff.extraction_complete && !cutoff.g1_certified && !cutoff.g2_certified);
        assert_eq!(cutoff.g1.as_ref().unwrap().exact_work, 0);
        let wire = model.faces[0].outer;
        let c = model.loops[wire]
            .coedges
            .iter_mut()
            .find(|c| c.pcurve.control_points[0][1] == c.pcurve.control_points[1][1])
            .unwrap();
        c.pcurve.periodic = true;
        let r = inspect_axis(&model, &caps, Axis::Station, 2000000).unwrap();
        assert!(!r.extraction_complete && !r.g1_certified && !r.g2_certified);
    }

    #[test]
    fn closed_hollow_miter_audits_every_station_including_closure_as_c0() {
        let outer = nurbs_core::primitives::circle([0., 0., 0.], [1., 0., 0.], 0.5).unwrap();
        let mut inner = nurbs_core::primitives::circle([0., 0., 0.], [1., 0., 0.], 0.2).unwrap();
        inner.control_points.reverse();
        inner.weights.reverse();
        let rows = nurbs_core::construct::curves::paths::closed_miter_sections(
            &[outer, inner],
            &[[0., 0., 0.], [10., 0., 0.], [10., 10., 0.], [0., 10., 0.]],
            [0., 0., 1.],
            2.,
        )
        .unwrap();
        let sections = rows
            .into_iter()
            .map(|r| r.into_iter().map(|c| vec![c]).collect())
            .collect::<Vec<_>>();
        let model = crate::periodic_section_loft(&sections).unwrap();
        let r = inspect_profile(&model, &[], 2000000).unwrap();
        assert!(r.profile.extraction_complete && r.profile.g2_certified);
        assert!(
            r.station.extraction_complete && !r.station.g1_certified && !r.station.g2_certified
        );
        assert_eq!(r.profile.edge_ids.len(), 32);
        assert_eq!(r.station.edge_ids.len(), 32);
        assert!(r.station.cap_edges.is_empty());
        assert_eq!(r.cap_continuity, "absent");
        assert_eq!(r.station_continuity, "C0");
        assert!(r.total_exact_work <= 2000000);
        assert_eq!(model.shells.len(), 2);
    }
}
