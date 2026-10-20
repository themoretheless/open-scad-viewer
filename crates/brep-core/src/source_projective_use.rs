//! Immutable original source use on an exact projective portion of a canonical curve.
//! Endpoint consumers freshly prove cross-face identity; no Model is admitted.
use crate::{source_allowed_contact, source_boundary_fragment::Fragment};
use cad_predicates::{BezierIdentity, Sign};
use value_codec::{Deserialize, Serialize, Value, json};
use nurbs_core::{curve::Curve, interval_eval::Interval as I, Error, Result};
#[derive(Clone)]
pub struct ProjectiveUse {
    world: Curve,
    fragment: Fragment,
    range: [[f64; 2]; 2],
    reversed: bool,
    weights: [f64; 2],
}
impl ProjectiveUse {
    /// Save original definitions only. Authority is reconstructed on restore.
    pub fn definition(&self) -> Value {
        json!({"version":1,"world":self.world.to_value(),"fragment":self.fragment.definition(),
            "range":self.range,"weights":self.weights})
    }
    pub fn world(&self) -> &Curve {
        &self.world
    }
    pub fn fragment(&self) -> &Fragment {
        &self.fragment
    }
    /// Exact normalized rational endpoints for the forward original pcurve.
    pub fn range(&self) -> [[f64; 2]; 2] {
        self.range
    }
    pub fn weights(&self) -> [f64; 2] { self.weights }
    pub fn reversed(&self) -> bool {
        self.reversed
    }
    /// Map an original source interval to normalized canonical parameters.
    pub fn map_normalized(&self,parameter:I)->Result<I> {
        let d=self.fragment.curve().domain();
        let t=parameter.sub(I::point(d[0]))?.div(I::point(d[1]).sub(I::point(d[0]))?)?.intersect(0.,1.)?;
        let a=I::point(self.range[0][0]).div(I::point(self.range[0][1]))?;
        let b=I::point(self.range[1][0]).div(I::point(self.range[1][1]))?;
        let low=I::point(1.).sub(t)?.mul(I::point(self.weights[0]))?;
        let high=t.mul(I::point(self.weights[1]))?;
        let den=low.add(high)?.intersect(self.weights[0].min(self.weights[1]),self.weights[0].max(self.weights[1]))?;
        a.mul(low)?.add(b.mul(high)?)?.div(den)?.intersect(0.,1.)
    }
    /// Unclipped inverse: unsupported canonical tails remain visible to the
    /// caller. Clipping here would hide missing retained-domain coverage.
    pub fn inverse_normalized_unclipped(&self,canonical:I)->Result<I> {
        let a=I::point(self.range[0][0]).div(I::point(self.range[0][1]))?;
        let b=I::point(self.range[1][0]).div(I::point(self.range[1][1]))?;
        let q=canonical.sub(a)?.div_signed(b.sub(a)?)?;
        let numerator=q.mul(I::point(self.weights[0]))?;
        let mut denominator=I::point(1.).sub(q)?.mul(I::point(self.weights[1]))?.add(numerator)?;
        if q.lo>=0. && q.hi<=1. {
            denominator=denominator.intersect(self.weights[0].min(self.weights[1]),self.weights[0].max(self.weights[1]))?;
        }
        let t=numerator.div_signed(denominator)?;
        let d=self.fragment.curve().domain();
        I::point(d[0]).add(t.mul(I::point(d[1]).sub(I::point(d[0]))?)?)
    }
    /// Outward enclosures only: root expressions and original domains remain
    /// authoritative. These bounds must never replace a root by a fixed vertex.
    pub fn parameter_bounds(&self) -> Result<[[f64; 2]; 2]> {
        let target=self.world.domain();
        let map=|r:[f64;2]|->Result<[f64;2]> {
            let q=self.map_normalized(I::new(r[0],r[1])?)?;
            let t=I::point(target[0]).add(q.mul(I::point(target[1]).sub(I::point(target[0]))?)?)?.intersect(target[0],target[1])?;
            Ok([t.lo,t.hi])
        };
        let b = self.fragment.parameter_bounds();
        Ok([map(b[0])?, map(b[1])?])
    }
}
pub struct Report {
    pub mapped: Option<ProjectiveUse>,
    pub exact_work: u64,
    pub reason: &'static str,
}
/// Fragment authority proves actual chart membership. Fresh exact composition
/// then binds its unchanged source curve to the supplied canonical subinterval.
pub fn qualify(
    world: &Curve,
    fragment: &Fragment,
    range: [[f64; 2]; 2],
    weights: [f64; 2],
    max_work: u64,
) -> Result<Report> {
    if !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new(
            "BREP_SOURCE_PROJECTIVE_USE",
            "Choose bounded exact mapping work",
        ));
    }
    let mut out = Report {
        mapped: None,
        exact_work: 0,
        reason: "source-projective-layout-unproven",
    };
    let Some(r) = nurbs_core::curve_surface_affine::verify_projective_algebraic(
        world,
        fragment.curve(),
        fragment.surface(),
        range,
        weights,
        max_work.min(cad_predicates::MAX_WORK),
    )?
    else {
        return Ok(out);
    };
    out.exact_work += r.work_used;
    out.reason = "source-projective-composition-unproven";
    if r.outcome != BezierIdentity::Equal {
        return Ok(out);
    }
    out.reason = "source-projective-orientation-unproven";
    let points = [
        [0., 0., 0.],
        [range[0][0], range[0][1], 0.],
        [range[1][0], range[1][1], 0.],
    ];
    let Some(sign) =
        source_allowed_contact::orient(&points, Some([0, 1]), &mut out.exact_work, max_work)?
    else {
        return Ok(out);
    };
    if sign == Sign::Zero {
        return Ok(out);
    }
    out.mapped = Some(ProjectiveUse {
        world: world.clone(),
        fragment: fragment.clone(),
        range,
        weights,
        reversed: fragment.reversed() ^ (sign == Sign::Positive),
    });
    out.reason = "source-projective-use-qualified";
    Ok(out)
}

/// Restore roots from original equations/selectors, then freshly prove the
/// projective composition and orientation. Cached bounds have no authority.
pub fn restore(value:Value,max_mapping_cells:usize,max_work:u64)->Result<Report> {
    let invalid=||Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid projective source definition or restoration budget");
    if value["version"].as_u64()!=Some(1) || !(1..=100000).contains(&max_mapping_cells)
        || !(1..=100000000).contains(&max_work) {return Err(invalid());}
    let world=Curve::from_value(value["world"].clone()).map_err(|_|invalid())?;
    let range=<[[f64;2];2]>::from_value(value["range"].clone()).map_err(|_|invalid())?;
    let weights=<[f64;2]>::from_value(value["weights"].clone()).map_err(|_|invalid())?;
    let fragment=crate::source_boundary_fragment::restore(value["fragment"].clone(),max_mapping_cells)?;
    qualify(&world,&fragment,range,weights,max_work)
}

/// Prove that two retained root expressions identify the same canonical point.
/// Both compositions were proved when the ProjectiveUse values were created;
/// the supplied plane must freshly bind both roots and have one world root on
/// the entire union of their outward parameter bounds.
pub fn same_crossing_endpoint(
    uses: [&ProjectiveUse; 2], endpoints: [usize; 2], plane: [[f64; 3]; 3],
    max_work: u64, max_driver: usize,
) -> Result<(bool, crate::source_shared_edge::Report)> {
    if endpoints.iter().any(|&i|i>1) || !(1..=100_000_000).contains(&max_work) || max_driver>100000 {
        return Err(Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid root correspondence bounds"));
    }
    let mut report=crate::source_shared_edge::Report{edge:None,work_used:0,root_checks:0,driver_cells:0,reason:"projective-root-correspondence-unproven"};
    if uses[0].world()!=uses[1].world() {return Ok((false,report));}
    let ends=[&uses[0].fragment().endpoints()[endpoints[0]],&uses[1].fragment().endpoints()[endpoints[1]]];
    let [crate::source_boundary_fragment::Endpoint::Crossing{point:a,role:ar},crate::source_boundary_fragment::Endpoint::Crossing{point:b,role:br}]=ends else {return Ok((false,report));};
    let aa=uses[0].parameter_bounds()?[endpoints[0]];
    let bb=uses[1].parameter_bounds()?[endpoints[1]];
    let domain=[aa[0].min(bb[0]),aa[1].max(bb[1])];
    let same=crate::source_shared_edge::common_plane_root_in_domain(
        uses[0].world(),plane,[(a,*ar),(b,*br)],domain,&mut report,max_work,max_driver,
    )?;
    if same {report.reason="projective-root-correspondence-qualified";}
    Ok((same,report))
}

/// Compare fixed ends or root ends with freshly verified exact parameter
/// candidates. A candidate is never accepted from its rounded root enclosure.
pub fn same_parameter_endpoint(
    uses:[&ProjectiveUse;2],endpoints:[usize;2],candidates:[Option<[f64;2]>;2],max_work:u64,
)->Result<(bool,u64)> {
    let (same,work,_)=same_parameter_endpoint_counted(uses,endpoints,candidates,max_work)?;
    Ok((same,work))
}
pub(crate) fn same_parameter_endpoint_counted(
    uses:[&ProjectiveUse;2], endpoints:[usize;2], candidates:[Option<[f64;2]>;2], max_work:u64,
 )->Result<(bool,u64,usize)> {
    use crate::source_boundary_fragment::{Endpoint,Role};
    use cad_predicates::{AuthoredScalar,Limits,PredicateContext,SourceArena,ToleranceContext,ParameterIdentity};
    if endpoints.iter().any(|&i|i>1) || !(1..=100_000_000).contains(&max_work) {
        return Err(Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid exact endpoint bounds"));
    }
    if uses[0].world()!=uses[1].world() {return Ok((false,0,0));}
    let mut work=0;let mut checks=0;let mut parameters=Vec::new();
    for i in 0..2 {
        let t=match &uses[i].fragment().endpoints()[endpoints[i]] {
            Endpoint::Parameter(t)=>*t,
            Endpoint::Crossing{point,role}=>{
                let Some(candidate)=candidates[i] else {return Ok((false,work,checks));};
                if work==max_work {return Ok((false,work,checks));}
                checks+=1;
                let r=crate::source_root_parameter::verify(point,candidate,(max_work-work).min(cad_predicates::MAX_WORK))?;
                work+=r.work_used;
                let Some(p)=r.parameters else {return Ok((false,work,checks));};
                p[match role {Role::Boundary=>0,Role::Contact=>1}]
            }
            Endpoint::Carrier{..}=>return Ok((false,work,checks)),
        };
        let d=uses[i].fragment().curve().domain();parameters.extend([t,d[0],d[1]]);
    }
    if work==max_work {return Ok((false,work,checks));}
    let values:Vec<_>=parameters.iter().copied()
        .chain(uses.iter().flat_map(|u|u.range().into_iter().flatten()))
        .chain(uses.iter().flat_map(|u|u.weights()))
        .map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect();
    let source=SourceArena::authored("projective-retained-endpoints",1,values)
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid endpoint source"))?;
    let p=std::array::from_fn(|i|std::array::from_fn(|j|source.leaf(3*i+j).unwrap()));
    let r=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|source.leaf(6+4*i+2*j+k).unwrap())));
    let w=std::array::from_fn(|i|std::array::from_fn(|j|source.leaf(14+2*i+j).unwrap()));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&source,&tolerance,Limits{max_work:(max_work-work).min(cad_predicates::MAX_WORK),..Limits::default()},None);
    let r=cad_predicates::projective_parameter_identity(&mut ctx,p,r,w)
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid endpoint comparison"))?;
    work+=r.work_used;
    Ok((r.outcome==ParameterIdentity::Equal,work,checks))
}

/// Freshly bind exact rational candidates to original root expressions,
/// then compare their mapped canonical parameters without rounded division.
pub(crate) fn same_fraction_endpoint_counted(
    uses:[&ProjectiveUse;2],endpoints:[usize;2],candidates:[Option<[[f64;2];2]>;2],max_work:u64,
)->Result<(bool,u64,usize)> {
    use crate::source_boundary_fragment::{Endpoint,Role};
    use cad_predicates::{AuthoredScalar,Limits,ParameterIdentity,PredicateContext,SourceArena,ToleranceContext};
    if endpoints.iter().any(|&i|i>1) || !(1..=100000000).contains(&max_work) {
        return Err(Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid rational endpoint bounds"));
    }
    if uses[0].world()!=uses[1].world() {return Ok((false,0,0));}
    let mut work=0;let mut checks=0;let mut values=Vec::new();
    for i in 0..2 {
        let parameter=match &uses[i].fragment().endpoints()[endpoints[i]] {
            Endpoint::Parameter(t)=>[*t,1.],
            Endpoint::Crossing{point,role}=> {
                let Some(candidate)=candidates[i] else {return Ok((false,work,checks));};
                if work==max_work {return Ok((false,work,checks));}
                checks+=1;
                let proof=crate::source_root_parameter::verify_fraction(point,candidate,(max_work-work).min(cad_predicates::MAX_WORK))?;
                work+=proof.work_used;
                let Some(parameters)=proof.parameters else {return Ok((false,work,checks));};
                parameters[match role {Role::Boundary=>0,Role::Contact=>1}]
            }
            Endpoint::Carrier{..}=>return Ok((false,work,checks)),
        };
        let d=uses[i].fragment().curve().domain();values.extend([parameter[0],parameter[1],d[0],d[1]]);
    }
    if work==max_work {return Ok((false,work,checks));}
    values.extend(uses.iter().flat_map(|u|u.range().into_iter().flatten()));values.extend(uses.iter().flat_map(|u|u.weights()));
    let arena=SourceArena::authored("projective-fraction-root-identity",1,values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid rational root source"))?;
    let parameters=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(4*i+j).unwrap()));
    let ranges=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(8+4*i+2*j+k).unwrap())));
    let weights=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(16+2*i+j).unwrap()));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:(max_work-work).min(cad_predicates::MAX_WORK),..Limits::default()},None);
    let proof=cad_predicates::projective_parameter_fraction_identity(&mut ctx,parameters,ranges,weights)
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid exact rational root comparison"))?;
    work+=proof.work_used;Ok((proof.outcome==ParameterIdentity::Equal,work,checks))
}

/// Exact canonical identity of two original transverse linear UV crossings.
/// Private SourcePoints supply uniqueness; line equations supply unrounded roots.
pub(crate) fn same_linear_endpoint_counted(
    uses:[&ProjectiveUse;2], endpoints:[usize;2], max_work:u64,
)->Result<(bool,u64,usize)> {
    use crate::source_boundary_fragment::{Endpoint,Role};
    use cad_predicates::{AuthoredScalar,Limits,ParameterIdentity,PredicateContext,SourceArena,ToleranceContext};
    if endpoints.iter().any(|&i|i>1) || !(1..=100000000).contains(&max_work) {
        return Err(Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid linear root comparison bounds"));
    }
    if uses[0].world()!=uses[1].world() {return Ok((false,0,0));}
    let line=|c:&Curve| {
        let d=c.domain();c.degree==1 && c.control_points.len()==2
            && c.knots[..2].iter().all(|&t|t==d[0]) && c.knots[2..].iter().all(|&t|t==d[1])
    };
    let mut values=Vec::new();
    for i in 0..2 {
        let Endpoint::Crossing{point,role}=&uses[i].fragment().endpoints()[endpoints[i]] else {return Ok((false,0,0));};
        let (main,cutter)=match role {Role::Boundary=>(point.boundary(),point.contact()),Role::Contact=>(point.contact(),point.boundary())};
        if !line(main) || !line(cutter) {return Ok((false,0,0));}
        for (p,w) in main.control_points.iter().zip(&main.weights) {values.extend([p[0],p[1],*w]);}
        for p in &cutter.control_points {values.extend([p[0],p[1]]);}
    }
    values.extend(uses.iter().flat_map(|u|u.range().into_iter().flatten()));
    values.extend(uses.iter().flat_map(|u|u.weights()));
    let arena=SourceArena::authored("projective-linear-root-identity",1,
        values.into_iter().map(|v|AuthoredScalar::Binary64Bits(v.to_bits())).collect())
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid original linear source"))?;
    let main=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(10*i+3*j+k).unwrap())));
    let cutter=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(10*i+6+2*j+k).unwrap())));
    let ranges=std::array::from_fn(|i|std::array::from_fn(|j|std::array::from_fn(|k|arena.leaf(20+4*i+2*j+k).unwrap())));
    let weights=std::array::from_fn(|i|std::array::from_fn(|j|arena.leaf(28+2*i+j).unwrap()));
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work:max_work.min(cad_predicates::MAX_WORK),..Limits::default()},None);
    let proof=cad_predicates::line_crossing_parameter_identity_projective(&mut ctx,main,cutter,ranges,weights)
        .map_err(|_|Error::new("BREP_SOURCE_PROJECTIVE_USE","Invalid exact linear root comparison"))?;
    Ok((proof.outcome==ParameterIdentity::Equal,proof.work_used,2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_boundary_fragment::{Endpoint,Role};
    use nurbs_core::surface::Surface;
    #[test]
    fn original_root_endpoint_is_retained_under_projective_mapping() {
        let surface=Surface{degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![1.,0.,0.],vec![1.,1.,0.]]],
            weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let world=Curve{degree:2,knots:vec![-5.,-5.,-5.,3.,3.,3.],
            control_points:vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
        let pc=Curve{degree:2,knots:vec![10.,10.,10.,18.,18.,18.],
            control_points:vec![vec![0.,0.],vec![0.5,0.],vec![1.,1.]],weights:vec![1.,2.,4.],periodic:false};
        let cutter=Curve::from_polyline(vec![vec![0.5,0.],vec![0.5,1.]]).unwrap();
        let root=crate::source_contact_point::qualify(&surface,&pc,&cutter,[[12.5,12.8],[0.2,0.3]],10000).unwrap();
        assert!(root.point.is_some(),"{}",root.reason);
        let fragment=Fragment::new(&surface,&pc,Endpoint::Parameter(10.),Endpoint::Crossing{point:root.point.unwrap(),role:Role::Boundary}).unwrap();
        let range=[[0.,1.],[1.,1.]];
        let mapped=qualify(&world,&fragment,range,[1.,2.],100000000).unwrap().mapped.unwrap();
        assert_eq!(mapped.world(),&world);assert_eq!(mapped.fragment().curve(),&pc);
        assert!(matches!(mapped.fragment().endpoints()[1],Endpoint::Crossing{..}));
        let b=mapped.parameter_bounds().unwrap();
        assert!(b[0][0]<=-5. && -5.<=b[0][1]);
        assert!(b[1][0]<=-1. && -1.<=b[1][1]);
        assert!(!mapped.reversed());
        let q=mapped.map_normalized(I::point(14.)).unwrap();
        assert!(q.lo<=2./3. && 2./3.<=q.hi);
        let back=mapped.inverse_normalized_unclipped(q).unwrap();
        assert!(back.lo<=14. && 14.<=back.hi);
        let outside=mapped.inverse_normalized_unclipped(I::point(-0.1)).unwrap();
        assert!(outside.hi<10.,"Unclipped inverse must expose unsupported tails");

        let definition=mapped.definition();
        let restored=restore(definition.clone(),10000,100000000).unwrap().mapped.unwrap();
        assert_eq!(restored.definition(),definition);
        assert_eq!(restored.parameter_bounds().unwrap(),mapped.parameter_bounds().unwrap());
        assert!(matches!(restored.fragment().endpoints()[1],Endpoint::Crossing{..}));
        let mut forged=definition.clone();forged["weights"]=json!([1.,1.]);
        assert!(restore(forged,10000,100000000).unwrap().mapped.is_none());
        let mut forged=definition.clone();forged["range"]=json!([[0.,1.],[3.,4.]]);
        assert!(restore(forged,10000,100000000).unwrap().mapped.is_none());
        let mut forged=definition.clone();forged["version"]=json!(2);
        assert!(restore(forged,10000,100000000).is_err());
        assert!(restore(definition.clone(),0,100000000).is_err());
        assert!(restore(definition.clone(),10000,1).unwrap().mapped.is_none());

        assert!(same_parameter_endpoint([&mapped,&mapped],[0,0],[None;2],100000000).unwrap().0);
        let mut mixed_pc=pc.clone();mixed_pc.weights=vec![1.,3.,9.];
        let mixed_root=crate::source_contact_point::qualify(&surface,&mixed_pc,&cutter,[[11.9,12.1],[0.2,0.3]],10000).unwrap().point.unwrap();
        let mixed_fragment=Fragment::new(&surface,&mixed_pc,Endpoint::Parameter(10.),Endpoint::Crossing{point:mixed_root,role:Role::Boundary}).unwrap();
        let mixed=qualify(&world,&mixed_fragment,range,[1.,3.],100000000).unwrap().mapped.unwrap();
        let mut fixed_pc=pc.clone();fixed_pc.weights=vec![1.;3];
        let fixed_fragment=Fragment::new(&surface,&fixed_pc,Endpoint::Parameter(14.),Endpoint::Parameter(10.)).unwrap();
        let fixed=qualify(&world,&fixed_fragment,range,[1.,1.],100000000).unwrap().mapped.unwrap();
        assert!(same_parameter_endpoint([&mixed,&fixed],[1,0],[Some([12.,0.25]),None],100000000).unwrap().0);
        assert_eq!(same_parameter_endpoint_counted([&mixed,&fixed],[1,0],[Some([12.,0.25]),None],100000000).unwrap().2,1);
        assert!(!same_parameter_endpoint([&mixed,&fixed],[1,0],[Some([12.01,0.25]),None],100000000).unwrap().0);
        assert!(!same_parameter_endpoint([&mixed,&fixed],[1,0],[Some([12.,0.25]),None],1).unwrap().0);

        let mut other_surface=surface.clone();
        for row in &mut other_surface.control_points {for point in row {point.swap(0,1);}}
        let mut other_pc=pc.clone();for point in &mut other_pc.control_points {point.swap(0,1);}
        let other_cutter=Curve::from_polyline(vec![vec![0.,0.5],vec![1.,0.5]]).unwrap();
        let other_root=crate::source_contact_point::qualify(&other_surface,&other_pc,&other_cutter,[[12.5,12.8],[0.2,0.3]],10000).unwrap().point.unwrap();
        let other_fragment=Fragment::new(&other_surface,&other_pc,
            Endpoint::Crossing{point:other_root,role:Role::Boundary},Endpoint::Parameter(10.)).unwrap();
        let other=qualify(&world,&other_fragment,range,[1.,2.],100000000).unwrap().mapped.unwrap();
        assert!(other.reversed());
        let plane=[[0.5,0.,0.],[0.5,1.,0.],[0.5,0.,1.]];
        let edge_report=crate::source_projective_edge::qualify(&world,[&fragment,&other_fragment],[range;2],[[1.,2.];2],
            [None,Some(plane)],[[None;2];2],100000000,10000).unwrap();
        let edge=edge_report.edge.unwrap();
        let restriction=crate::source_edge_restriction::Restriction::from_edge(&edge);
        let bounds=restriction.parameter_bounds().unwrap();
        assert!(bounds[0][0]<=-5. && -5.<=bounds[0][1]);assert!(bounds[1][0]<=-1. && -1.<=bounds[1][1]);
        let boxes=restriction.endpoint_boxes(100).unwrap();
        for (a,value) in [0.5,0.25,0.].into_iter().enumerate() {assert!(boxes[1][a][0]<=value && value<=boxes[1][a][1]);}
        let bytes=value_codec::encode_binary(&restriction.definition()).unwrap();
        let restriction_replay=crate::source_edge_restriction::restore_projective(value_codec::decode_binary(&bytes).unwrap(),
            crate::source_shared_edge_restore::Limits{mapping_cells_per_use:10000,exact_work:100000000,driver_cells:10000}).unwrap().unwrap();
        assert_eq!(restriction_replay.definition(),restriction.definition());assert_eq!(restriction_replay.parameter_bounds().unwrap(),bounds);
        assert_eq!(restriction_replay.endpoint_boxes(100).unwrap(),boxes);
        assert_eq!(restriction.display_segments(16).unwrap(),restriction_replay.display_segments(16).unwrap());
        assert!(restriction.endpoint_boxes(0).is_err());assert!(restriction.display_segments(0).is_err());

        let seam_limits=||crate::source_seam_tangency::Limits{max_sine_squared:1e-3,cells:10000,curve_spans:20000,normal_spans:20000};
        let seam=crate::source_seam_tangency::qualify(&edge,seam_limits()).unwrap();
        assert!(seam.seam.is_some(),"{}",seam.reason);
        assert!(seam.seam.unwrap().sine_squared_bounds()[1]<=1e-3);
        let synchronized=crate::source_seam_tangency::qualify_with_synchronized_boundaries(&edge,seam_limits(),100000).unwrap();
        assert_eq!(synchronized.synchronized_reason,"synchronized-boundary-unsupported");
        assert!(synchronized.polynomial_work<=100000);
        assert!(synchronized.report.seam.is_some());
        assert_eq!(synchronized.report.seam.unwrap().edge().definition(),edge.definition());
        // The same parabola lies on z=v-u^2, whose tangent plane differs
        // from z=0 everywhere on this retained interval.
        let oblique_surface=Surface{degree_u:2,degree_v:1,knots_u:vec![0.,0.,0.,1.,1.,1.],knots_v:vec![0.,0.,1.,1.],
            control_points:(0..3).map(|u|(0..2).map(|v|vec![u as f64*0.5,v as f64,v as f64-if u==2 {1.}else{0.}]).collect()).collect(),
            weights:vec![vec![1.;2];3],periodic_u:false,periodic_v:false};
        let oblique_root=crate::source_contact_point::qualify(&oblique_surface,&pc,&cutter,[[12.5,12.8],[0.2,0.3]],10000).unwrap().point.unwrap();
        let oblique_fragment=Fragment::new(&oblique_surface,&pc,Endpoint::Crossing{point:oblique_root,role:Role::Boundary},Endpoint::Parameter(10.)).unwrap();
        let oblique=crate::source_projective_edge::qualify(&world,[&fragment,&oblique_fragment],[range;2],[[1.,2.];2],
            [None,Some(plane)],[[None;2];2],100000000,10000).unwrap().edge.unwrap();
        let failed=crate::source_seam_tangency::qualify(&oblique,seam_limits()).unwrap();
        assert!(failed.seam.is_none());
        assert_eq!(failed.reason,"source-seam-angular-envelope-oblique");
        let synchronized_oblique=crate::source_seam_tangency::qualify_with_synchronized_boundaries(&oblique,seam_limits(),100000).unwrap();
        assert!(synchronized_oblique.report.seam.is_none());
        assert_eq!(synchronized_oblique.report.reason,"source-seam-angular-envelope-oblique");
        let endpoint=edge.sample_endpoint(1,100).unwrap().endpoint.unwrap();
        assert!(endpoint.canonical_bounds()[0]<=0.5 && 0.5<=endpoint.canonical_bounds()[1]);
        let sample=edge.sample_interior(0.25,100).unwrap().sample.unwrap();
        assert!(sample.world_bounds()[0][0]<=0.25 && 0.25<=sample.world_bounds()[0][1]);
        for (side,expected) in [[0.25,0.0625],[0.0625,0.25]].iter().enumerate() {
            for axis in 0..2 {
                let b=sample.uv_bounds()[side][axis];
                assert!(b[0]<=expected[axis] && expected[axis]<=b[1]);
                assert!(b[1]-b[0]<1e-10,"Interior UV enclosure should resolve the rational inverse");
            }
        }

        let chain=edge.sample_interior_chain(&[0.,0.125,0.25,0.375,0.5,0.75],1000).unwrap();
        assert_eq!(chain.points.len(),6);assert!(chain.pending>=3);
        assert!(edge.sample_interior(0.25,1).unwrap().sample.is_none());
        let mesh=edge.prepare_boundary_mesh(crate::source_shared_edge::BoundaryMeshLimits{max_deviation_mm:0.1,cells:1000,spans:20000,max_depth:12}).unwrap();
        assert!(mesh.pending.is_empty());assert!(!mesh.segments.is_empty());
        assert!(mesh.segments.iter().all(|segment|segment.deviation_upper_mm<=0.1));
        let saved=edge.definition();
        let text=value_codec::to_string(&saved).unwrap();
        let decoded:Value=value_codec::from_str_strict(&text).unwrap();
        assert_eq!(decoded,saved);
        let bytes=value_codec::encode_binary(&saved).unwrap();
        assert_eq!(value_codec::decode_binary(&bytes).unwrap(),saved);
        let replay=crate::source_projective_edge::restore(decoded,10000,100000000,10000).unwrap().edge.unwrap();
        assert_eq!(replay.definition(),saved);
        assert_eq!(replay.world(),&world);
        assert!(replay.uses()[0].reversed()!=replay.uses()[1].reversed());
        let mut wrong=saved.clone();wrong["uses"][1]["weights"]=json!([1.,1.]);
        assert!(crate::source_projective_edge::restore(wrong,10000,100000000,10000).unwrap().edge.is_none());
        let mut wrong=saved.clone();wrong["planes"][1]=json!([[0.6,0.,0.],[0.6,1.,0.],[0.6,0.,1.]]);
        assert!(crate::source_projective_edge::restore(wrong,10000,100000000,10000).unwrap().edge.is_none());
        assert!(crate::source_projective_edge::restore(saved.clone(),10000,1,10000).unwrap().edge.is_none());
        assert!(crate::source_projective_edge::restore(saved,10000,100000000,0).unwrap().edge.is_none());
        let (same,report)=same_crossing_endpoint([&mapped,&other],[1,0],plane,100000000,10000).unwrap();
        assert!(same,"{}",report.reason);
        assert!(!same_crossing_endpoint([&mapped,&other],[1,0],[[0.6,0.,0.],[0.6,1.,0.],[0.6,0.,1.]],100000000,10000).unwrap().0);
        assert!(!same_crossing_endpoint([&mapped,&other],[1,0],plane,1,10000).unwrap().0);

        assert!(qualify(&world,&fragment,range,[1.,1.],100000000).unwrap().mapped.is_none());
        assert!(qualify(&world,&fragment,range,[1.,2.],1).unwrap().mapped.is_none());
    }
}
