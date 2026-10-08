//! Original profile material region projected into the ideal initial frame.
//! No endpoint cap, retained-region correspondence, error or embedding claim.
use super::*;
use crate::sweeps::progressive_miter::{authored_frame_certificate as frame,
    profile_domain_certificate::{Report, certify_original_profile_projection_domain},
    scalar_certificate::Status};

fn original_frame(sweep:&Sweep<'_>,twist:&Curve,t:[f64;2],budget:usize,initial:bool)->Result<Option<frame::ValuesReport>>{
    if let Some((axis,normal))=sweep.frame_laws {return Ok(Some(frame::certify_twisted_values(axis,normal,twist,t,budget)?));}
    if let Some(guide)=sweep.orientation_guide {return Ok(Some(frame::certify_path_guide_values(sweep.path,guide,twist,t,budget)?));}
    // Explicit spatial RMF endpoint material uses the original Bishop cover,
    // with integration and exact source work charged to this same caller.
    // The initial basis is already a point proof; no integration is needed there.
    if !initial && sweep.options.orientation==Orientation::RotationMinimizing {
        if let Some([steps,_,_])=sweep.spatial_rmf_error_limits {
            let exact=budget/2;
            let transport=super::rmf_transport::certify_original_rmf_transport(
                sweep.path,sweep.options.normal,steps,budget-exact,exact as u64,false)?;
            let work=transport.cells+transport.exact_work as usize;
            if transport.status!=Status::Certified {
                return Ok(Some(frame::ValuesReport {status:Status::Unresolved,cells:work,
                    longitudinal:None,transverse:None,binormal:None}));
            }
            let mut values=super::rmf_transport::frame_values(&transport,twist,t,t,budget-work)?;
            values.cells+=work;
            return Ok(Some(values));
        }
    }
    if matches!(sweep.options.orientation,Orientation::CorrectedFrenet|Orientation::RotationMinimizing)&&!super::authored_error::original_line(sweep.path){
        let line=super::source_line::certify(sweep.path,budget)?;
        if line.certified {
            let mut values=frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget-line.cells)?;
            values.cells+=line.cells;return Ok(Some(values));
        }
        // Unsuccessful source eligibility work also belongs to this caller.
        // Remaining modes below consume the reduced budget; attach the spent
        // eligibility work to their result without restarting the allowance.
        let mut values=original_frame_without_line(sweep,twist,t,budget-line.cells,initial)?;
        if let Some(values)=&mut values{values.cells+=line.cells;}
        return Ok(values);
    }
    original_frame_without_line(sweep,twist,t,budget,initial)
}
fn original_frame_without_line(sweep:&Sweep<'_>,twist:&Curve,t:[f64;2],budget:usize,initial:bool)->Result<Option<frame::ValuesReport>>{
    if sweep.options.orientation==Orientation::CorrectedFrenet && !super::authored_error::original_line(sweep.path) {
        // Scalar scale/twist transport of the original filled region is
        // invariant under constant transverse phase, just like its contour.
        // Use the same canonical plane basis as the retained error owner.
        // Anisotropic affine/center transport uses its original principal phase.
        if sweep.path.periodic||path_is_closed(sweep.path)?{return Ok(None);}
        let plane=super::source_plane::certify(sweep.path,sweep.options.normal,budget as u64)?;
        let cells=plane.exact_work as usize;
        if !plane.proved {
            if plane.reason==Some("source-plane-coefficients-nonzero") {
                let mut values=super::authored_error::corrected_regular_frame_values(sweep,twist,t,budget-cells)?;
                values.cells+=cells;return Ok(Some(values));
            }
            return Ok(Some(frame::ValuesReport{status:Status::Unresolved,cells,
                longitudinal:None,transverse:None,binormal:None}));
        }
        if sweep.affine_laws.is_some(){
            let (phase_cells,phase)=frame::certify_initial_planar_principal_phase(sweep.path,sweep.options.normal,budget-cells)?;
            let cells=cells+phase_cells;
            let Some(phase)=phase else{return Ok(Some(frame::ValuesReport{status:Status::Unresolved,cells,longitudinal:None,transverse:None,binormal:None}));};
            let mut values=frame::certify_planar_principal_values(sweep.path,sweep.options.normal,phase,twist,t,t,budget-cells)?;
            values.cells+=cells;return Ok(Some(values));
        }
        let mut values=if initial {frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget-cells)?}
            else{frame::certify_fixed_normal_path_values(sweep.path,sweep.options.normal,twist,t,budget-cells)?};
        values.cells+=cells;return Ok(Some(values));
    }
    Ok(match sweep.options.orientation {
        Orientation::Fixed=>Some(frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::FixedNormal if initial=>Some(frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::FixedNormal=>Some(frame::certify_fixed_normal_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::Frenet=>Some(frame::certify_frenet_path_values(sweep.path,twist,t,budget)?),
        Orientation::CorrectedFrenet if super::authored_error::original_line(sweep.path)=>Some(frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::RotationMinimizing if initial||super::authored_error::original_line(sweep.path)=>Some(frame::certify_fixed_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::RotationMinimizing if super::authored_error::original_planar_rmf(sweep.path,sweep.options.normal)=>Some(frame::certify_fixed_normal_path_values(sweep.path,sweep.options.normal,twist,t,budget)?),
        Orientation::RotationMinimizing if !sweep.path.periodic=>{
            // Endpoint material transport needs the same exact original
            // plane premise as the surface theorem, with its work charged.
            let plane=super::source_plane::certify(sweep.path,sweep.options.normal,budget as u64)?;
            let cells=plane.exact_work as usize;
            if !plane.proved {
                Some(frame::ValuesReport {status:Status::Unresolved,cells,
                    longitudinal:None,transverse:None,binormal:None})
            }else{
                let mut values=frame::certify_fixed_normal_path_values(sweep.path,sweep.options.normal,twist,t,budget-cells)?;
                values.cells+=cells;Some(values)
            }
        },
        _=>None,
    })
}

impl MultiSweep<'_> {
    pub fn certify_local_profile_domain(
        &self, loop_sizes: &[usize], tolerance: f64, max_pairs: usize,
        max_cells: usize, max_exact_work: u64,
    ) -> Result<Report> {
        check(max_cells <= 100000, "Profile domain frame budget exceeds100000")?;
        let sweep = &self.sweeps[0];
        let zero = constant_vector_law([0.;3])?;
        let original = original_frame(sweep,&zero,[0.,0.],max_cells,true)?;
        let cells = original.as_ref().map_or(0, |r|r.cells);
        let axis = original.filter(|r|r.status == Status::Certified).and_then(|r|r.longitudinal);
        let profiles = self.sweeps.iter().map(|s|s.profile.clone()).collect::<Vec<_>>();
        certify_original_profile_projection_domain(&profiles,loop_sizes,axis,cells,
            tolerance,max_pairs,max_cells,max_exact_work)
    }
}

/// Original filled endpoint domains are affine images of the proved source
/// region. Axes below are frame axes, not normals of an oblique source plane.
#[derive(Clone, Debug)]
pub struct IdealEndpointDomainsReport {
    pub ideal_endpoint_domains_certified: bool,
    pub source: Report,
    pub endpoint_frame_axes: Option<[[[f64;2];3];2]>,
    pub endpoint_contact_fits: Option<[[f64;2];2]>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
impl MultiSweep<'_> {
    pub fn certify_ideal_endpoint_domains(
        &self, loop_sizes: &[usize], tolerance:f64, max_pairs:usize,
        max_cells:usize, max_exact_work:u64,
    )->Result<IdealEndpointDomainsReport>{
        let source=self.certify_local_profile_domain(loop_sizes,tolerance,max_pairs,max_cells,max_exact_work)?;
        let mut out=IdealEndpointDomainsReport {ideal_endpoint_domains_certified:false,
            cells:source.cells,reason:source.reason,source,endpoint_frame_axes:None,endpoint_contact_fits:None};
        if !out.source.local_domain_certified {return Ok(out);}
        let sweep=&self.sweeps[0];
        if path_is_closed(sweep.path)? {out.reason=Some("closed-path-has-no-caps");return Ok(out);}
        let anchor=if sweep.contact_point.is_some(){
            out.reason=Some("contact-anchor-domain-unproved");
            if sweep.contact_source.is_none()||!self.sweeps.iter().all(|s|match (s.contact_source,sweep.contact_source){
                (Some((a,u)),Some((b,v)))=>std::ptr::eq(a,b)&&u==v,_=>false}){return Ok(out);}
            let certificate=super::contact_anchor::certify(sweep,max_cells-out.cells)?;
            out.cells+=certificate.cells;
            if certificate.status!=Status::Certified{out.reason=certificate.reason;return Ok(out);}
            Some(certificate.coordinates.unwrap()[0])
        }else{None};
        let mut fits=[[1.;2];2];
        let mut axes=[[[0.;2];3];2];
        for endpoint in 0..2 {
            let t=[endpoint as f64;2];let remaining=max_cells-out.cells;
            let values=original_frame(sweep,sweep.twist,t,remaining,false)?;
            let Some(values)=values else {out.reason=Some("endpoint-original-frame-unproved");return Ok(out);};
            out.cells+=values.cells;
            if values.status!=Status::Certified {out.reason=Some("endpoint-original-frame-unproved");return Ok(out);}
            axes[endpoint]=values.longitudinal.unwrap();
            if let Some(anchor)=anchor{
                let fit=frame::certify_contact_fit_value(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,sweep.affine_laws,anchor,t,max_cells-out.cells)?;
                out.cells+=fit.cells;
                if fit.status!=Status::Certified{out.reason=Some("endpoint-contact-fit-unproved");return Ok(out);}
                fits[endpoint]=fit.value.unwrap();
                if fits[endpoint][0]<=0.{out.reason=Some("endpoint-contact-fit-positivity-unproved");return Ok(out);}
            }
        }
        // Constructor validation proves positive scale/axis laws. The source
        // region is mapped by an invertible 3D affine map at each endpoint,
        // preserving all holes even if its plane is oblique to the frame axis.
        // This proves ideal domains only: actual cap identity/error is separate.
        out.ideal_endpoint_domains_certified=true;out.endpoint_frame_axes=Some(axes);out.endpoint_contact_fits=anchor.map(|_|fits);out.reason=None;
        Ok(out)
    }
}

/// Unit normal enclosures of the original ideal endpoint planes. These are
/// derived from the source plane, not inferred from retained cap samples.
#[derive(Clone, Debug)]
pub struct IdealEndpointPlanesReport {
    pub domains: IdealEndpointDomainsReport,
    pub endpoint_normals: Option<[[[f64;2];3];2]>,
    pub cells: usize,
    pub reason: Option<&'static str>,
}
impl MultiSweep<'_> {
    pub fn certify_ideal_endpoint_planes(
        &self,loop_sizes:&[usize],tolerance:f64,max_pairs:usize,max_cells:usize,max_exact_work:u64,
    )->Result<IdealEndpointPlanesReport>{
        use crate::distance_bounds::Interval as I;
        use crate::numerics::interval_vec3 as iv;
        use crate::sweeps::progressive_miter::vector_certificate;
        let domains=self.certify_ideal_endpoint_domains(loop_sizes,tolerance,max_pairs,max_cells,max_exact_work)?;
        let mut out=IdealEndpointPlanesReport {cells:domains.cells,reason:domains.reason,domains,endpoint_normals:None};
        if !out.domains.ideal_endpoint_domains_certified {return Ok(out);}
        out.reason=Some("endpoint-plane-normal-unproved");
        let decode=|v:[[f64;2];3]|->Result<[I;3]>{Ok([I::new(v[0][0],v[0][1])?,I::new(v[1][0],v[1][1])?,I::new(v[2][0],v[2][1])?])};
        let sweep=&self.sweeps[0];let zero=constant_vector_law([0.;3])?;
        let Some(initial)=original_frame(sweep,&zero,[0.,0.],max_cells-out.cells,true)? else {return Ok(out);};
        out.cells+=initial.cells;if initial.status!=Status::Certified{return Ok(out);}
        let source=decode(out.domains.source.source_plane_normal.unwrap())?;
        let initial_basis=[initial.transverse.unwrap(),initial.binormal.unwrap(),initial.longitudinal.unwrap()];
        let mut local=[I::point(0.);3];
        for k in 0..3 {local[k]=iv::dot_tight(decode(initial_basis[k])?,source)?;}
        let mut normals=[[[0.;2];3];2];
        for endpoint in 0..2 {
            let t=[endpoint as f64;2];
            let Some(values)=original_frame(sweep,sweep.twist,t,max_cells-out.cells,false)? else {return Ok(out);};
            out.cells+=values.cells;if values.status!=Status::Certified {return Ok(out);}
            let mut axes=if let Some((axes,_))=sweep.affine_laws {
                let report=vector_certificate::certify_values_traversal(axes,t,max_cells-out.cells,false)?;
                out.cells+=report.cells;
                let Some(value)=report.value else {return Ok(out);};decode(value)?
            } else {[I::point(1.);3]};
            if let Some(fits)=out.domains.endpoint_contact_fits{
                axes[0]=axes[0].mul(I::new(fits[endpoint][0],fits[endpoint][1])?)?;
            }
            if axes.iter().any(|a|a.lo<=0.) {return Ok(out);}
            let basis=[values.transverse.unwrap(),values.binormal.unwrap(),values.longitudinal.unwrap()];
            let mut world=[I::point(0.);3];
            for k in 0..3 {world=iv::add(world,iv::scale(decode(basis[k])?,local[k].div(axes[k])?)?)?;}
            // Inverse-transpose transport: R1 D^-1 R0^T N. Positive uniform
            // scale changes length only, so it cancels in this unit direction.
            let length=iv::norm(world)?;if length.lo<=0. {return Ok(out);}
            normals[endpoint]=iv::div(world,length)?.map(|v|[v.lo,v.hi]);
        }
        out.endpoint_normals=Some(normals);out.reason=None;Ok(out)
    }
}

/// Source-to-retained plane projection prerequisite. Actual cap material regions
/// and endpoint ownership are still separate; no boundary error is composed here.
#[derive(Clone, Debug)]
pub struct EndpointCapProjectionReport {
    pub original: IdealEndpointPlanesReport,
    pub normal_dots: Option<[[f64;2];2]>,
    pub reverses_orientation: Option<[bool;2]>,
    pub cells: usize,
    pub exact_work: u64,
    pub reason: Option<&'static str>,
}
impl MultiSweep<'_> {
    pub fn certify_endpoint_cap_projection(
        &self,loop_sizes:&[usize],caps:&[Surface;2],tolerance:f64,max_pairs:usize,max_cells:usize,max_exact_work:u64,
    )->Result<EndpointCapProjectionReport>{
        use crate::sweeps::progressive_miter::{cap_retained_plane_certificate as retained,cap_projection_certificate as projection};
        for cap in caps {cap.validate()?;check(cap.control_points[0][0].len()==3,"Endpoint cap must be3D")?;}
        let original=self.certify_ideal_endpoint_planes(loop_sizes,tolerance,max_pairs,max_cells,max_exact_work)?;
        let mut out=EndpointCapProjectionReport {cells:original.cells,exact_work:original.domains.source.exact_work,
            reason:original.reason,original,normal_dots:None,reverses_orientation:None};
        let Some(ideal)=out.original.endpoint_normals else {return Ok(out);};
        out.reason=Some("retained-cap-plane-unproved");
        let mut dots=[[0.;2];2];let mut reversed=[false;2];
        for endpoint in 0..2 {
            let plane=retained::inspect(&caps[endpoint],max_exact_work-out.exact_work);
            out.exact_work+=plane.work;
            let Some(normal)=plane.normal else {return Ok(out);};
            out.reason=Some("cap-projection-unproved");
            let proof=projection::certify(ideal[endpoint],normal,max_cells-out.cells)?;
            out.cells+=proof.cells;
            if !proof.projection_regular {return Ok(out);}
            dots[endpoint]=proof.normal_dot.unwrap();reversed[endpoint]=proof.reverses_orientation.unwrap();
            out.reason=Some("retained-cap-plane-unproved");
        }
        out.normal_dots=Some(dots);out.reverses_orientation=Some(reversed);out.reason=None;Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_endpoint_domains_transport_oblique_plane_with_positive_fit(){
        let points=[[1.,0.,0.],[1.,0.1,0.],[0.5,0.1,-0.05],[0.5,0.,-0.05]];
        let profiles=(0..4).map(|i|crate::primitives::line(points[i],points[(i+1)%4]).unwrap()).collect::<Vec<_>>();
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
        let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.1,0.,0.]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap().with_contact_guide(&guide,0,0.).unwrap();
        let proof=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.domains.ideal_endpoint_domains_certified,"{:?}",proof.reason);
        let fits=proof.domains.endpoint_contact_fits.unwrap();
        for fit in fits{assert!(fit[0]<=1./2.1&&1./2.1<=fit[1]&&fit[0]>0.);}
        let normals=proof.endpoint_normals.unwrap();let length=(1.+0.105_f64.powi(2)).sqrt();
        // R1 diag((2*fit)^-1,3^-1,1) R0^T N, independently.
        for (normal,expected) in normals.iter().zip([[-0.105/length,0.,1./length],[-0.105/length,2./5f64.sqrt()/length,1./5f64.sqrt()/length]]){
            // Source normal orientation is selected by the exact source plane.
            let sign=if normal[2][1]<0.{-1.}else{1.};
            for (bound,value) in normal.iter().zip(expected){let value=value*sign;assert!(bound[0]<=value&&value<=bound[1],"{normal:?} versus {expected:?}");}
        }
        let short=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,proof.cells-1,1000000).unwrap();assert!(short.endpoint_normals.is_none());
    }
    fn square(lo:f64,hi:f64,reverse:bool)->Vec<Curve>{
        let mut points=vec![[lo,lo,0.],[hi,lo,0.],[hi,hi,0.],[lo,hi,0.]];
        if reverse {points.reverse();}
        (0..4).map(|i|crate::primitives::line(points[i],points[(i+1)%4]).unwrap()).collect()
    }
    #[test]
    fn planar_rmf_endpoint_domains_and_original_normals_follow_curved_tangent(){
        let mut profiles=square(-0.1,0.1,false);
        for curve in &mut profiles {for pole in &mut curve.control_points {*pole=vec![0.,pole[0],pole[1]];}}
        let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let options=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let proof=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.domains.ideal_endpoint_domains_certified,"{:?}",proof.reason);
        let normals=proof.endpoint_normals.unwrap();
        for (normal,expected) in normals.iter().zip([[1.,0.,0.],[1./5f64.sqrt(),2./5f64.sqrt(),0.]]) {
            for (bound,value) in normal.iter().zip(expected) {assert!(bound[0]<=value&&value<=bound[1],"{normal:?}");}
        }
        let short=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,proof.cells-1,1000000).unwrap();
        assert!(short.endpoint_normals.is_none());
    }
    #[test]
    fn nonaxial_rmf_endpoint_planes_preserve_holes_and_refuse_off_plane_source(){
        let mut profiles=square(-0.1,0.1,false);profiles.extend(square(-0.05,0.05,true));
        // Original material plane x=y is perpendicular to C'(0).
        for curve in &mut profiles {for pole in &mut curve.control_points {*pole=vec![pole[0],pole[0],pole[1]];}}
        let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],control_points:vec![vec![0.;3],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let options=Options {normal:[1.,1.,0.],orientation:Orientation::RotationMinimizing,
            spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let level=sweep.preview_at(17).unwrap();
        assert!(level.report.continuous_bound&&level.report.accepted,"{:?}",level.report);
        let proof=sweep.certify_ideal_endpoint_planes(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.domains.ideal_endpoint_domains_certified,"{:?}",proof.reason);
        let normals=proof.endpoint_normals.unwrap();
        let root=2_f64.sqrt();let end=6_f64.sqrt();
        for (normal,expected) in normals.iter().zip([[1./root,-1./root,0.],[1./end,-1./end,2./end]]){
            let sign=if normal[0][1]<0.{-1.}else{1.};
            for (bound,value) in normal.iter().zip(expected){let value=sign*value;assert!(bound[0]<=value&&value<=bound[1],"{normal:?} versus {expected:?}");}
        }
        let short=sweep.certify_ideal_endpoint_planes(&[4,4],1e-9,10000,proof.cells-1,1000000).unwrap();
        assert!(short.endpoint_normals.is_none());
        let mut wrong=path.clone();wrong.control_points[1][1]=(-0.5_f64).next_up();
        let sweep=MultiSweep::new(&profiles,&wrong,&scale,&twist,options).unwrap();
        let unproved=sweep.certify_ideal_endpoint_planes(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(!unproved.domains.ideal_endpoint_domains_certified&&unproved.endpoint_normals.is_none());
    }
    #[test]
    fn original_progressive_domain_proves_holes_and_refuses_partial_budget(){
        let mut profiles=square(-2.,2.,false);profiles.extend(square(-1.,1.,true));
        let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let proof=sweep.certify_local_profile_domain(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.local_domain_certified,"{:?}",proof.reason);
        let short=sweep.certify_local_profile_domain(&[4,4],1e-9,10000,proof.cells-1,1000000).unwrap();
        assert!(!short.local_domain_certified);
        assert!(!sweep.certify_local_profile_domain(&[4,4],1e-9,10000,0,1000000).unwrap().local_domain_certified);
        assert!(sweep.certify_local_profile_domain(&[8,1],1e-9,10000,100000,1000000).is_err());
    }
    #[test]
    fn original_progressive_domain_refuses_touching_holes_and_unproved_initial_frame(){
        let mut profiles=square(-2.,2.,false);profiles.extend(square(-2.,1.,true));
        let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let mut options=Options {normal:[1.,0.,0.],orientation:Orientation::FixedNormal,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        assert!(!sweep.certify_local_profile_domain(&[4,4],1e-9,10000,100000,1000000).unwrap().local_domain_certified);
        options.orientation=Orientation::CorrectedFrenet;
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let proof=sweep.certify_local_profile_domain(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(!proof.local_domain_certified);
        let spatial=crate::paths::bezier(vec![vec![0.;3],vec![0.,0.,3.],vec![1.,0.,6.],vec![1.,1.,10.]],None).unwrap();
        let valid=square(-2.,2.,false);
        let sweep=MultiSweep::new(&valid,&spatial,&scale,&twist,options).unwrap();
        let proof=sweep.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.local_domain_certified,"{proof:?}");assert_eq!(proof.reason,None);
        // A spatial leading straight interval has no original regular principal
        // seed. It must not inherit the newly proved nonzero-curvature branch.
        let zero_initial=crate::paths::bezier(vec![vec![0.;3],vec![0.,0.,2.],vec![0.,0.,4.],vec![1.,0.,6.],vec![1.,1.,10.]],None).unwrap();
        let unproved=MultiSweep::new(&valid,&zero_initial,&scale,&twist,options).unwrap();
        let refused=unproved.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap();
        assert_eq!(refused.reason,Some("initial-frame-unproved"));assert!(!refused.local_domain_certified);
        let line=MultiSweep::new(&valid,&path,&scale,&twist,options).unwrap();
        assert!(line.certify_ideal_endpoint_domains(&[4],1e-9,10000,100000,1000000).unwrap().ideal_endpoint_domains_certified);
    }
    #[test]
    fn original_domain_uses_constructor_frames_guides_and_affine_laws(){
        let profiles=square(-2.,2.,false);
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,5.],vec![1.,0.,10.]],weights:vec![1.;3],periodic:false};
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([1.,-2.,0.]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap();
        assert!(sweep.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap().local_domain_certified);
        let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options {orientation:Orientation::Fixed,..options}).unwrap().with_frame_laws(&axis,&normal).unwrap();
        assert!(sweep.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap().local_domain_certified);
        let guide=Curve {control_points:vec![vec![1.,0.,0.],vec![1.,0.,5.],vec![2.,0.,10.]],..path.clone()};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_orientation_guide(&guide).unwrap();
        assert!(sweep.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap().local_domain_certified);
        let edge_on=constant_vector_law([1.,0.,0.]).unwrap();let transverse=constant_vector_law([0.,1.,0.]).unwrap();
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options {orientation:Orientation::Fixed,..options}).unwrap().with_frame_laws(&edge_on,&transverse).unwrap();
        let proof=sweep.certify_local_profile_domain(&[4],1e-9,10000,100000,1000000).unwrap();
        assert!(!proof.local_domain_certified);assert_eq!(proof.reason,Some("local-projection-unproved"));
    }

    #[test]
    fn ideal_endpoint_domains_preserve_oblique_source_holes_and_discard_partial_frames(){
        let mut profiles=square(-2.,2.,false);profiles.extend(square(-1.,1.,true));
        for curve in &mut profiles {for pole in &mut curve.control_points {pole[2]=pole[0];}}
        let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let axes=constant_vector_law([2.,3.,4.]).unwrap();let center=constant_vector_law([1.,2.,3.]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=sweep.certify_ideal_endpoint_domains(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.ideal_endpoint_domains_certified,"{:?}",proof.reason);
        assert!(proof.endpoint_frame_axes.is_some());
        let short=sweep.certify_ideal_endpoint_domains(&[4,4],1e-9,10000,proof.cells-1,1000000).unwrap();
        assert!(!short.ideal_endpoint_domains_certified);assert!(short.endpoint_frame_axes.is_none());
        let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,5.],vec![1.,0.,5.]],weights:vec![1.;3],periodic:false};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let refused=sweep.certify_ideal_endpoint_domains(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(!refused.ideal_endpoint_domains_certified);assert_eq!(refused.reason,Some("endpoint-original-frame-unproved"));
        let options=Options {orientation:Orientation::FixedNormal,..options};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let singular=sweep.certify_ideal_endpoint_domains(&[4,4],1e-9,10000,100000,1000000).unwrap();
        assert!(!singular.ideal_endpoint_domains_certified);assert!(singular.endpoint_frame_axes.is_none());
    }

    #[cfg(feature="transport")]
    #[test]
    fn endpoint_domain_transport_preserves_scope_and_partial_budget_refusal(){
        use value_codec::json;
        let profiles=square(-2.,2.,false);
        let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let request=json!({"op":"surface_progressive_sweep_cap_domains","profiles":profiles,
            "path":path,"scale":scale,"twist":twist,"orientation":"rmf","spacing":"parameter",
            "normal":[1.,0.,0.],"initial_sections":3,"max_sections":33,"max_deviation":0.01,
            "loopSizes":[4],"tolerance":1e-9,"maxPairs":10000,"maxCells":100000,"maxExactWork":1000000});
        let report=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["idealCapDomainsCertified"],true);
        for key in ["continuousBound","retainedCapRegionsCertified","globalEmbeddingCertified","solidCertified"] {assert_eq!(report[key],false);}
        let mut zero=request.clone();zero["maxCells"]=json!(0);
        let refused=crate::transport::dispatch(zero).unwrap();
        assert_eq!(refused["idealCapDomainsCertified"],false);assert!(refused["endpointFrameAxes"].is_null());
        let mut malformed=request;malformed["loopSizes"]=json!([3]);
        assert!(crate::transport::dispatch(malformed).is_err());
    }

    #[test]
    fn ideal_endpoint_normals_use_inverse_affine_transport_of_oblique_source_plane(){
        let mut profiles=square(-2.,2.,false);
        for curve in &mut profiles {for pole in &mut curve.control_points {pole[2]=pole[0];}}
        let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([2.,0.,0.]).unwrap();
        let twist=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![0.;3],vec![0.25,0.,0.]],weights:vec![1.;2],periodic:false};
        let axes=constant_vector_law([2.,3.,4.]).unwrap();let center=constant_vector_law([1.,2.,3.]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,100000,1000000).unwrap();
        assert!(proof.domains.ideal_endpoint_domains_certified);
        let normals=proof.endpoint_normals.unwrap();
        for endpoint in 0..2 {
            let angle=0.25*endpoint as f64;let q=5_f64.sqrt();
            let expected=[-2.*angle.cos()/q,-2.*angle.sin()/q,1./q];
            for k in 0..3 {assert!(normals[endpoint][k][0]<=expected[k]&&expected[k]<=normals[endpoint][k][1]);}
            assert!(normals[endpoint][0][1]<0.); // The cap normal is not the frame's Z axis.
        }
        let short=sweep.certify_ideal_endpoint_planes(&[4],1e-9,10000,proof.cells-1,1000000).unwrap();
        assert!(short.endpoint_normals.is_none());
    }

    #[test]
    fn endpoint_cap_projection_proves_actual_planes_and_refuses_partial_or_singular_pairs(){
        let profiles=square(-2.,2.,false);let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap();
        let cap=|z|Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,z],vec![0.,1.,z]],vec![vec![1.,0.,z],vec![1.,1.,z]]],weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let mut caps=[cap(0.),cap(10.)];caps[1].control_points.reverse();
        let proof=sweep.certify_endpoint_cap_projection(&[4],&caps,1e-9,10000,100000,1000000).unwrap();
        assert!(proof.normal_dots.is_some(),"{:?}",proof.reason);assert_eq!(proof.reverses_orientation,Some([false,true]));
        let short=sweep.certify_endpoint_cap_projection(&[4],&caps,1e-9,10000,proof.cells-1,1000000).unwrap();assert!(short.normal_dots.is_none());
        let short=sweep.certify_endpoint_cap_projection(&[4],&caps,1e-9,10000,100000,proof.exact_work-1).unwrap();assert!(short.normal_dots.is_none());
        caps[1].control_points=vec![vec![vec![0.,0.,0.],vec![0.,1.,0.]],vec![vec![0.,0.,1.],vec![0.,1.,1.]]];
        let singular=sweep.certify_endpoint_cap_projection(&[4],&caps,1e-9,10000,100000,1000000).unwrap();assert!(singular.normal_dots.is_none());
        caps[1]=cap(10.);caps[1].control_points[1][1][2]+=0.01;
        let nonplanar=sweep.certify_endpoint_cap_projection(&[4],&caps,1e-9,10000,100000,1000000).unwrap();assert!(nonplanar.normal_dots.is_none());
    }

    #[cfg(feature="transport")]
    #[test]
    fn cap_projection_transport_keeps_geometry_and_material_scopes_separate(){
        use value_codec::json;
        let profiles=square(-2.,2.,false);let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
        let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
        let cap=|z|Surface {degree_u:1,degree_v:1,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,1.,1.],control_points:vec![vec![vec![0.,0.,z],vec![0.,1.,z]],vec![vec![1.,0.,z],vec![1.,1.,z]]],weights:vec![vec![1.;2];2],periodic_u:false,periodic_v:false};
        let request=json!({"op":"surface_progressive_sweep_cap_projection","profiles":profiles,"path":path,"scale":scale,"twist":twist,
            "normal":[1.,0.,0.],"orientation":"rmf","spacing":"parameter","initial_sections":3,"max_sections":33,"max_deviation":0.01,
            "loopSizes":[4],"caps":[cap(0.),cap(10.)],"tolerance":1e-9,"maxPairs":10000,"maxCells":100000,"maxExactWork":1000000});
        let report=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(report["capProjectionCertified"],true);assert_eq!(report["idealCapDomainsCertified"],true);
        for key in ["continuousBound","retainedCapRegionsCertified","globalEmbeddingCertified","solidCertified"] {assert_eq!(report[key],false);}
        let mut bad=request;bad["caps"]=json!([cap(0.)]);assert!(crate::transport::dispatch(bad).is_err());
    }

}
