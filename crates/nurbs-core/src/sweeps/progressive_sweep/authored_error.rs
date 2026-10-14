//! Constructor-owned original-law premises for authored sweep error.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{dot_tight, sub as interval_sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate, scalar_certificate::Status, vector_certificate,
};
mod spatial_parameter;

mod coordinates;
pub use coordinates::InitialCoordinatesReport;
pub(super) use coordinates::{initial_coordinates, fixed_initial_coordinates, fixed_normal_initial_coordinates, guided_initial_coordinates, control_trajectory};
use coordinates::*;

#[derive(Clone, Debug)]
pub struct SectionInterpolationReport {
    pub status: Status,
    pub cells: usize,
    pub error_upper: Option<f64>,
    pub endpoint_displacement_upper: Option<f64>,
    /// Only the two complete sweep endpoint contours; station-wide bound above
    /// stays separate. None uses that conservative station-wide fallback.
    pub cap_endpoint_displacement_upper: Option<[f64;2]>,
    pub reason: Option<&'static str>,
}
pub(super) fn section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
) -> Result<SectionInterpolationReport> {
    section_interpolation_with_length(sweep,count,max_cells,None)
}
fn section_interpolation_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,
    shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    check(
        count >= sweep.options.initial_sections && count <= sweep.options.max_sections,
        "Section count outside configured sweep",
    )?;
    check(
        max_cells <= 100000,
        "Section interpolation work exceeds100000 cells",
    )?;
    let mut out = SectionInterpolationReport {
        status: Status::Unresolved,
        cells: 0,
        error_upper: None,
        endpoint_displacement_upper: None,
        cap_endpoint_displacement_upper: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    if sweep.options.spacing != Spacing::Parameter && !original_line(sweep.path) {
        return original_arc_length_section_interpolation(sweep,count,max_cells,shared_length);
    }
    // Authored frame laws use normalized station fraction. For a rational
    // line, normalized arc-length position is exactly this uniform line;
    // keep actual constructor sections and charge inverse-length rounding.
    let length_reference = (sweep.options.spacing != Spacing::Parameter).then(|| Curve {
        degree:1,knots:vec![0.,0.,1.,1.],control_points:sweep.path.control_points.clone(),
        weights:vec![1.,1.],periodic:false,
    });
    let reference_path=length_reference.as_ref().unwrap_or(sweep.path);
    // Fixed authored transport has zero constructor holonomy correction.
    // The copied closing section is still compared to the original t=1
    // trajectory below; its discrepancy is charged to the last interval.
    if path_is_closed(sweep.path)? && sweep.options.orientation != Orientation::Fixed {
        out.reason = Some("closed-frame-correction-unproved");
        return Ok(out);
    }
    let initial = initial_coordinates(sweep, max_cells)?;
    out.cells += initial.cells;
    if initial.status != Status::Certified {
        out.reason = initial.reason;
        return Ok(out);
    }
    let coordinates = initial.coordinates.unwrap();
    let sections = sweep.sections(count)?.0;
    // Positive, unchanged rational profile bases make the profile error a
    // convex combination of corresponding control-trajectory errors.
    check(
        sections.iter().all(|c| {
            c.degree == sweep.profile.degree
                && c.knots == sweep.profile.knots
                && c.weights == sweep.profile.weights
                && c.periodic == sweep.profile.periodic
                && c.control_points.len() == coordinates.len()
        }),
        "Retained section basis correspondence changed",
    )?;
    let mut endpoint_errors = vec![vec![0.; coordinates.len()]; count];
    let mut endpoint_upper = 0_f64;
    for (station, section) in sections.iter().enumerate() {
        let t=station as f64/(count-1) as f64;
        let batch=authored_frame_certificate::certify_control_values(reference_path,sweep.scale,sweep.twist,
            axis,normal,sweep.affine_laws,&coordinates,[t,t],max_cells-out.cells)?;
        out.cells+=batch.cells;
        if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
        let Some(values)=batch.values else {out.reason=Some("authored-control-values-unresolved");return Ok(out);};
        for (control,value) in values.into_iter().enumerate() {
            // Source work was charged once above; this view only applies
            // the original owner's interval to the owned retained endpoint.
            let value=authored_frame_certificate::ControlValueReport {status:Status::Certified,cells:0,value:Some(value),reason:None};
            let p=&section.control_points[control];
            let upper=value.retained_displacement_upper([p[0],p[1],p[2]])?.unwrap();
            endpoint_errors[station][control]=upper;
            endpoint_upper=endpoint_upper.max(upper);
        }
    }
    let mut error = 0_f64;
    for station in 0..count-1 {
        let interval=[station as f64/(count-1) as f64,(station+1) as f64/(count-1) as f64];
        let batch=authored_frame_certificate::certify_control_trajectories(reference_path,sweep.scale,sweep.twist,
            axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
        out.cells+=batch.cells;
        if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
        let single_span=batch.single_span;
        let Some(jets)=batch.jets else {out.reason=Some("authored-control-jets-unresolved");return Ok(out);};
        let values=if !single_span {
            let values=authored_frame_certificate::certify_control_values(reference_path,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
            out.cells+=values.cells;
            if values.status!=Status::Certified {out.reason=values.reason;return Ok(out);}
            let Some(values)=values.values else {out.reason=Some("section-knot-value-bound-unproved");return Ok(out);};
            Some(values)
        }else {None};
        for (control,jet) in jets.into_iter().enumerate() {
            let jet=authored_frame_certificate::TrajectoryReport {traversal:interval,status:Status::Certified,
                cells:0,jet:Some(jet),single_span,reason:None};
            let upper=if let Some(upper)=jet.linear_error_upper([endpoint_errors[station][control],endpoint_errors[station+1][control]])? {upper}
            else {
                // At original knot transitions use value-image/retained-segment
                // correspondence, never an unsupported global C2 remainder.
                let value=authored_frame_certificate::ControlValueReport {status:Status::Certified,cells:0,
                    value:Some(values.as_ref().unwrap()[control]),reason:None};
                let a=&sections[station].control_points[control];let b=&sections[station+1].control_points[control];
                let Some(upper)=value.retained_segment_displacement_upper([[a[0],a[1],a[2]],[b[0],b[1],b[2]]])?
                    else {out.reason=Some("section-knot-value-bound-unproved");return Ok(out);};
                upper
            };
            error=error.max(upper);
        }
    }
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.endpoint_displacement_upper = Some(endpoint_upper);
    out.reason = None;
    Ok(out)
}

// Original arc-length position is total-length Lipschitz in normalized time.
// Its chord defect is <= L*h/2, including continuous C0 path corners. Authored
// relative pose is certified independently in the original station fraction.
fn original_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    let mut out=SectionInterpolationReport {status:Status::Unresolved,cells:0,error_upper:None,
        endpoint_displacement_upper:None,cap_endpoint_displacement_upper:None,reason:Some("arc-length-correspondence-unproved")};
    let path=sweep.path;
    let domain=path.domain();
    // Interior multiplicity > degree permits a jump, invalidating Lipschitz
    // position in cumulative length. No sampled continuity premise is used.
    // A closed Fixed authored frame has no holonomy correction. The same
    // length-Lipschitz chord bound applies to its last interval; the endpoint
    // loop below charges the copied closing section against the original pose.
    let unsupported_closed=path_is_closed(path)?
        && !(sweep.frame_laws.is_some() && sweep.options.orientation==Orientation::Fixed);
    if path.periodic || unsupported_closed || path.knots.iter().any(|k|
        *k>domain[0]&&*k<domain[1]&&path.knots.iter().filter(|x|*x==k).count()>path.degree){return Ok(out);}
    let Spacing::ArcLength {tolerance,max_cells:constructor_budget}=sweep.options.spacing else {return Ok(out);};
    if max_cells==0{return Ok(out);}
    let owned;
    let division=if let Some(shared)=shared_length {shared} else {
        owned=crate::curve_measure::divide_by_length(path,count-1,tolerance,max_cells.min(constructor_budget))?;
        out.cells=owned.cells;&owned
    };
    if !division.within_tolerance || division.points.len()!=count{return Ok(out);}
    let parameters=sweep.parameters(count)?.0;
    if division.points.iter().zip(parameters).any(|(point,parameter)|point.parameter!=parameter){return Ok(out);}
    let initial=if sweep.frame_laws.is_some(){initial_coordinates(sweep,max_cells-out.cells)?}
        else{fixed_initial_coordinates(sweep,max_cells-out.cells)?};
    out.cells+=initial.cells;
    let Some(coordinates)=initial.coordinates.filter(|_|initial.status==Status::Certified) else {out.reason=initial.reason;return Ok(out);};
    let sections=sweep.sections(count)?.0;
    if sections.iter().any(|section|section.degree!=sweep.profile.degree||section.knots!=sweep.profile.knots
        ||section.weights!=sweep.profile.weights||section.periodic!=sweep.profile.periodic||section.control_points.len()!=coordinates.len()){
        out.reason=Some("retained-section-basis-correspondence-unproved");return Ok(out);
    }
    let zero=constant_vector_law([0.;3])?;
    let frame_laws=sweep.frame_laws;
    let mut endpoints=vec![vec![0.;coordinates.len()];count];
    let mut endpoint_upper=0_f64;
    for station in 0..count{
        let t=station as f64/(count-1) as f64;
        let point=&division.points[station];
        let batch=if let Some((axis,normal))=frame_laws {
            let batch=authored_frame_certificate::certify_control_values(&zero,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,[t,t],max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            let Some(values)=batch.values else {return Ok(out);};Some(values)
        }else {None};
        for (control,&q) in coordinates.iter().enumerate(){
            let value=if let Some(values)=&batch {
                authored_frame_certificate::ControlValueReport {status:Status::Certified,cells:0,value:Some(values[control]),reason:None}
            }else{authored_frame_certificate::certify_fixed_relative_value(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,q,[t,t],max_cells-out.cells)?};
            out.cells+=value.cells;
            let Some(relative)=value.value.filter(|_|value.status==Status::Certified) else {out.reason=value.reason;return Ok(out);};
            let mut delta=[I::point(0.);3];
            for k in 0..3{delta[k]=I::new(relative[k][0],relative[k][1])?
                .add(I::new(point.point_bounds[k][0],point.point_bounds[k][1])?)?
                .sub(I::point(sections[station].control_points[control][k]))?;}
            let upper=crate::numerics::interval_vec3::norm(delta)?.add(I::point(point.residual_upper))?.hi;
            endpoints[station][control]=upper;endpoint_upper=endpoint_upper.max(upper);
        }
    }
    let mut error=0_f64;
    for station in 0..count-1{
        let interval=[station as f64/(count-1) as f64,(station+1) as f64/(count-1) as f64];
        let width=I::point(interval[1]).sub(I::point(interval[0]))?;
        let position=I::point(division.total.bounds[1]).mul(width)?.div(I::point(2.))?;
        let batch=if let Some((axis,normal))=frame_laws {
            let batch=authored_frame_certificate::certify_control_trajectories(&zero,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            Some(batch)
        }else {None};
        let values=if batch.as_ref().is_some_and(|batch|!batch.single_span) {
            let (axis,normal)=frame_laws.unwrap();
            let values=authored_frame_certificate::certify_control_values(&zero,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
            out.cells+=values.cells;
            if values.status!=Status::Certified {out.reason=values.reason;return Ok(out);}
            values.values
        }else {None};
        for (control,&q) in coordinates.iter().enumerate(){
            let jet=if let Some(batch)=&batch {
                let Some(jets)=&batch.jets else {return Ok(out);};
                authored_frame_certificate::TrajectoryReport {traversal:interval,status:Status::Certified,cells:0,
                    jet:Some(jets[control].clone()),single_span:batch.single_span,reason:None}
            }else{authored_frame_certificate::certify_fixed_relative_trajectory(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,q,interval,max_cells-out.cells)?};
            out.cells+=jet.cells;
            let relative=if let Some(relative)=jet.linear_error_upper([endpoints[station][control],endpoints[station+1][control]])? {relative}
            else if let Some(values)=&values {
                // Original value image versus retained relative endpoint hull.
                // True endpoint positions lie in the evaluated point enclosure
                // enlarged by their certified length residual. This avoids a
                // global second-derivative premise across original frame knots.
                let mut delta=[I::point(0.);3];
                for k in 0..3 {
                    let endpoint=|s:usize|->Result<I>{
                        let point=&division.points[s];
                        let position=I::new(point.point_bounds[k][0],point.point_bounds[k][1])?
                            .add(I::new(-point.residual_upper,point.residual_upper)?)?;
                        I::point(sections[s].control_points[control][k]).sub(position)
                    };
                    let a=endpoint(station)?;let b=endpoint(station+1)?;
                    delta[k]=I::new(values[control][k][0],values[control][k][1])?
                        .sub(I::new(a.lo.min(b.lo),a.hi.max(b.hi))?)?;
                }
                crate::numerics::interval_vec3::norm(delta)?.hi
            }else {out.reason=jet.reason.or(Some("arc-length-relative-pose-remainder-unproved"));return Ok(out);};
            error=error.max(position.add(I::point(relative))?.hi);
        }
    }
    out.status=Status::Certified;out.error_upper=Some(error);out.endpoint_displacement_upper=Some(endpoint_upper);out.reason=None;
    Ok(out)
}

#[derive(Clone, Debug)]
pub struct PatchErrorReport {
    pub status: Status,
    pub cells: usize,
    pub products: usize,
    pub section_error_upper: Option<f64>,
    /// Stored original-section control displacement; excludes decomposition/caps.
    pub original_section_endpoint_error_upper: Option<f64>,
    /// Both retained end contours, including profile decomposition.
    pub retained_endpoint_error_upper: Option<f64>,
    pub endpoint_contour_error_upper: Option<[f64;2]>,
    pub decomposition_error_upper: Option<f64>,
    pub error_upper: Option<f64>,
    pub within_budget: bool,
    pub patches: Option<Vec<Surface>>,
    pub reason: Option<&'static str>,
}
pub(super) fn patch_error(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
    max_products: usize,
) -> Result<PatchErrorReport> {
    check(
        max_products <= 1000000,
        "Patch decomposition work exceeds1000000 products",
    )?;
    let section = section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
pub(super) fn patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,
    shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    let section=section_interpolation_with_length(sweep,count,max_cells,shared_length)?;
    patch_error_from_section(sweep,count,max_products,section)
}
pub(super) fn guided_patch_error(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
    max_products: usize,
) -> Result<PatchErrorReport> {
    check(
        max_products <= 1000000,
        "Patch decomposition work exceeds1000000 products",
    )?;
    let section = guided_section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
pub(super) fn guided_patch_error_with_lengths(
    sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,
    path:Option<&crate::curve_measure::DivisionReport>,guide:Option<&crate::curve_measure::DivisionReport>,
)->Result<PatchErrorReport>{
    if path.is_some()&&sweep.orientation_guide.is_some()&&sweep.contact_point.is_none()
        &&sweep.frame_laws.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000&&max_products<=1000000,"Guided section/decomposition work exceeds limits")?;
        return patch_error_from_section(sweep,count,max_products,original_frame_arc_length_section_interpolation(sweep,count,max_cells,path,OriginalTransport::Guided,guide)?);
    }
    guided_patch_error(sweep,count,max_cells,max_products)
}
pub(super) fn contact_patch_error(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, max_products: usize,
) -> Result<PatchErrorReport> {
    check(max_products <= 1000000, "Patch decomposition work exceeds1000000 products")?;
    let section = contact_section_interpolation(sweep, count, max_cells)?;
    patch_error_from_section(sweep, count, max_products, section)
}
pub(super) fn contact_patch_error_with_lengths(
    sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,
    path:Option<&crate::curve_measure::DivisionReport>,guide:Option<&crate::curve_measure::DivisionReport>,
)->Result<PatchErrorReport>{
    if path.is_some()&&sweep.contact_source.is_some()&&sweep.orientation_guide.is_some()
        &&sweep.frame_laws.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000&&max_products<=1000000,"Contact section/decomposition work exceeds limits")?;
        return patch_error_from_section(sweep,count,max_products,original_frame_arc_length_section_interpolation(sweep,count,max_cells,path,OriginalTransport::Contact,guide)?);
    }
    contact_patch_error(sweep,count,max_cells,max_products)
}
pub(super) fn fixed_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    fixed_patch_error_with_length(sweep,count,max_cells,max_products,None)
}
pub(super) fn fixed_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    if shared_length.is_some() && sweep.options.orientation==Orientation::Fixed && sweep.frame_laws.is_none()
        && sweep.orientation_guide.is_none() && sweep.contact_point.is_none() && !original_line(sweep.path)
        && sweep.options.spacing!=Spacing::Parameter {
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000,"Fixed section work exceeds100000 cells")?;
        return patch_error_from_section(sweep,count,max_products,original_arc_length_section_interpolation(sweep,count,max_cells,shared_length)?);
    }
    patch_error_from_section(sweep,count,max_products,fixed_section_interpolation(sweep,count,max_cells)?)
}
// For positive weights, a two-pole degree-one rational curve is
// P(t) = P0 + rho(t)(P1-P0), with rho'(t) > 0. Its ideal tangent
// direction is constant, including oblique and reversed lines. Both RMF
// reflections preserve the perpendicular seed; actual station arithmetic
// is charged by retained endpoint displacement, rather than treated as exact.
pub(super) fn original_line(path:&Curve)->bool{
    path.degree==1&&!path.periodic&&path.control_points.len()==2&&path.weights.len()==2
        &&(0..3).any(|k|path.control_points[0][k]!=path.control_points[1][k])
}
/// An original rational path with constant coordinate has tangent orthogonal
/// to that coordinate axis. An axial seed normal is therefore a constant
/// Bishop normal (N'=0); regularity is proved by the transport certificate.
/// This is coefficient identity, not sampled planarity or straightness.
pub(super) fn original_planar_rmf(path:&Curve,normal:[f64;3])->bool {
    !path.periodic && !path.control_points.is_empty() && normal.iter().all(|v|v.is_finite())
        && (0..3).any(|axis|normal[axis]!=0. && (0..3).all(|k|k==axis||normal[k]==0.)
            && path.control_points.iter().all(|p|p.len()==3&&p[axis]==path.control_points[0][axis]))
        // Closed sources additionally require constructor-owned exact normal
        // and zero-correction evidence before retained error admission.
}
pub(super) fn rmf_planar_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    rmf_planar_patch_error_with_length(sweep,count,max_cells,max_products,None)
}
pub(super) fn rmf_planar_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_cells<=100000&&max_products<=1000000,"RMF section/decomposition work exceeds limits")?;
    let line=super::source_line::certify(sweep.path,max_cells)?;
    let mut section=if line.certified {
        guided_section_interpolation_mode(sweep,count,max_cells-line.cells,OriginalTransport::RmfCollinear)?
    }else if shared_length.is_some()&&sweep.options.orientation==Orientation::RotationMinimizing&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        original_frame_arc_length_section_interpolation(sweep,count,max_cells-line.cells,shared_length,OriginalTransport::RmfPlanar,None)?
    }else{guided_section_interpolation_mode(sweep,count,max_cells-line.cells,OriginalTransport::RmfPlanar)?};
    section.cells+=line.cells;
    patch_error_from_section(sweep,count,max_products,section)
}
pub(super) fn rmf_straight_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    let section=guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::RmfStraight)?;
    patch_error_from_section(sweep,count,max_products,section)
}
/// Original corrected planar world transport is independent of the constant
/// transverse phase when only scalar scale/twist act on the profile. For
/// P=B x T, both N=P and N=B therefore reconstruct the same world field from
/// original profile coordinates. Actual corrected sections (not RMF replay)
/// remain the owner of endpoint displacement and retained decomposition.
/// Anisotropic affine/center laws use a separate original initial-principal
/// phase proof when available. Original lines use their constant seed.
pub(super) fn corrected_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_cells<=100000&&max_products<=1000000,"Corrected frame error work exceeds limits")?;
    let line=super::source_line::certify(sweep.path,max_cells)?;
    let remaining=max_cells-line.cells;
    let mode=if original_line(sweep.path){OriginalTransport::CorrectedStraight}else if line.certified {OriginalTransport::CorrectedCollinear}else{OriginalTransport::CorrectedPlanar};
    let mut section=if mode==OriginalTransport::CorrectedPlanar&&shared_length.is_some()
        &&sweep.options.orientation==Orientation::CorrectedFrenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()
        &&sweep.options.spacing!=Spacing::Parameter {
        original_frame_arc_length_section_interpolation(sweep,count,remaining,shared_length,mode,None)?
    }else{guided_section_interpolation_mode(sweep,count,remaining,mode)?};
    section.cells+=line.cells;
    patch_error_from_section(sweep,count,max_products,section)
}
pub(super) fn fixed_normal_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    fixed_normal_patch_error_with_length(sweep,count,max_cells,max_products,None)
}
pub(super) fn fixed_normal_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    if shared_length.is_some()&&sweep.options.orientation==Orientation::FixedNormal&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000,"FixedNormal section work exceeds100000 cells")?;
        return patch_error_from_section(sweep,count,max_products,fixed_normal_arc_length_section_interpolation(sweep,count,max_cells,shared_length)?);
    }
    let section=guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::FixedNormal)?;
    patch_error_from_section(sweep,count,max_products,section)
}
pub(super) fn frenet_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    frenet_patch_error_with_length(sweep,count,max_cells,max_products,None)
}
pub(super) fn frenet_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    if shared_length.is_some()&&sweep.options.orientation==Orientation::Frenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000,"Frenet section work exceeds100000 cells")?;
        return patch_error_from_section(sweep,count,max_products,original_frame_arc_length_section_interpolation(sweep,count,max_cells,shared_length,OriginalTransport::Frenet,None)?);
    }
    let section=guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::Frenet)?;
    patch_error_from_section(sweep,count,max_products,section)
}
fn patch_error_from_section(
    sweep: &Sweep<'_>,
    count: usize,
    max_products: usize,
    section: SectionInterpolationReport,
) -> Result<PatchErrorReport> {
    let mut out = PatchErrorReport {
        status: Status::Unresolved,
        cells: section.cells,
        products: 0,
        section_error_upper: section.error_upper,
        original_section_endpoint_error_upper: section.endpoint_displacement_upper,
        retained_endpoint_error_upper: None,
        endpoint_contour_error_upper: None,
        decomposition_error_upper: None,
        error_upper: None,
        within_budget: false,
        patches: None,
        reason: section.reason,
    };
    if section.status != Status::Certified {
        return Ok(out);
    }
    let sections = sweep.sections(count)?.0;
    let rows = sections
        .iter()
        .map(profile_parts)
        .collect::<Result<Vec<_>>>()?;
    let mut decomposition = 0_f64;
    if sweep.profile.control_points.len() > 32 {
        for (source, row) in sections.iter().zip(&rows) {
            let proof = crate::curve_decomposition_certificate::inspect_partition(
                source, row, max_products - out.products,
            )?;
            out.products += proof.products;
            let Some(error) = proof.error_upper else {
                out.reason = proof.reason;
                return Ok(out);
            };
            decomposition = decomposition.max(error);
        }
    }
    // Constant rational U weights across stations make V interpolation a
    // convex interpolation of the corresponding decomposed section curves.
    let first = &rows[0];
    if rows.iter().any(|row| {
        row.len() != first.len()
            || row.iter().zip(first).any(|(a, b)| {
                a.degree != b.degree
                    || a.knots != b.knots
                    || a.weights != b.weights
                    || a.periodic != b.periodic
            })
    }) {
        out.reason = Some("decomposed-station-basis-correspondence-unproved");
        return Ok(out);
    }
    let error = I::point(section.error_upper.unwrap())
        .add(I::point(decomposition))?
        .hi;
    let retained = patches(&sections)?;
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.decomposition_error_upper = Some(decomposition);
    out.retained_endpoint_error_upper = section.endpoint_displacement_upper
        .map(|endpoint| I::point(endpoint).add(I::point(decomposition)).map(|bound|bound.hi))
        .transpose()?;
    out.endpoint_contour_error_upper=section.cap_endpoint_displacement_upper
        .or_else(||section.endpoint_displacement_upper.map(|e|[e;2]))
        .map(|ends|->Result<[f64;2]>{Ok([I::point(ends[0]).add(I::point(decomposition))?.hi,I::point(ends[1]).add(I::point(decomposition))?.hi])}).transpose()?;
    out.within_budget = error <= sweep.options.max_deviation;
    out.patches = Some(retained);
    out.reason = None;
    Ok(out)
}

#[cfg(test)]
#[path = "tests/authored_error.rs"]
mod tests;

pub(super) fn guided_control_value(
    sweep: &Sweep<'_>,
    profile_control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::ControlValueReport> {
    check(
        profile_control < sweep.profile.control_points.len(),
        "Profile control outside source",
    )?;
    check(
        max_cells <= 100000,
        "Guided control work exceeds100000 cells",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::ControlValueReport {
        status: Status::Unresolved,
        cells,
        value: None,
        reason: Some(reason),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(unresolved(0, "mode-not-guided"));
    };
    if sweep.options.spacing != Spacing::Parameter {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    if sweep.contact_point.is_some() {
        return Ok(unresolved(0, "contact-width-law-unproved"));
    }
    let initial = guided_initial_coordinates(sweep, max_cells)?;
    if initial.status != Status::Certified {
        return Ok(unresolved(initial.cells, initial.reason.unwrap()));
    }
    let mut report = authored_frame_certificate::certify_path_guide_control_value(
        sweep.path,
        guide,
        sweep.scale,
        sweep.twist,
        sweep.affine_laws,
        initial.coordinates.unwrap()[profile_control],
        traversal,
        max_cells - initial.cells,
    )?;
    report.cells += initial.cells;
    Ok(report)
}

pub(super) fn guided_section_interpolation(
    sweep: &Sweep<'_>,
    count: usize,
    max_cells: usize,
) -> Result<SectionInterpolationReport> {
    guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::Guided)
}

pub(super) fn contact_section_interpolation(sweep: &Sweep<'_>,count: usize,max_cells:usize)->Result<SectionInterpolationReport> {
    guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::Contact)
}

// Exact affine pieces may meet at C0 knots. Dyadic station fractions and
// source domain [0,1] make the interval correspondence exact, without a
// floating knot normalization premise. Constant rational poles define a
// constant transform even with nonuniform positive weights.
fn fixed_affine_interval(sweep:&Sweep<'_>,count:usize,interval:[f64;2])->bool {
    let constant=|c:&Curve|c.control_points.iter().all(|p|p==&c.control_points[0]);
    sweep.path.degree==1 && sweep.path.domain()==[0.,1.]
        && sweep.path.weights.iter().all(|w|*w==sweep.path.weights[0])
        && (count-1).is_power_of_two()
        && !sweep.path.knots.iter().any(|t|*t>interval[0]&&*t<interval[1])
        && constant(sweep.scale) && constant(sweep.twist)
        && sweep.affine_laws.is_none_or(|(axes,center)|constant(axes)&&constant(center))
}

// Exact normalized-domain identity avoids inferring knot locations from rounded
// affine parameter conversions. No source law may jump inside the open span.
fn original_laws_have_no_interior_knots(sweep:&Sweep<'_>,interval:[f64;2])->bool {
    let mut curves=vec![sweep.path,sweep.scale,sweep.twist];
    if let Some((a,b))=sweep.affine_laws {curves.extend([a,b]);}
    if let Some((a,b))=sweep.frame_laws {curves.extend([a,b]);}
    if let Some(guide)=sweep.orientation_guide {curves.push(guide);}
    curves.into_iter().all(|curve|curve.domain()==[0.,1.] &&
        !curve.knots.iter().any(|&k|k>interval[0]&&k<interval[1]))
}

pub(super) fn fixed_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize)->Result<SectionInterpolationReport>{
    guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::Fixed)
}

#[derive(Clone,Copy,PartialEq)]
enum OriginalTransport {Guided,Contact,Fixed,RmfStraight,RmfCollinear,RmfPlanar,RmfSpatial,CorrectedStraight,CorrectedCollinear,CorrectedPlanar,FixedNormal,Frenet}

mod transport;
use transport::*;

pub(super) fn rmf_spatial_patch_error(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    rmf_spatial_patch_error_with_length(sweep,count,steps,max_cells,max_products,None)
}
pub(super) fn rmf_spatial_patch_error_with_length(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    rmf_spatial_patch_error_with_transport(sweep,count,steps,max_cells,max_products,shared_length,None)
}
pub(super) fn rmf_spatial_patch_error_with_transport(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_rmf:Option<&super::rmf_transport::OriginalRmfTransportReport>)->Result<PatchErrorReport>{
    check(count>=sweep.options.initial_sections && count<=sweep.options.max_sections
        && steps.is_power_of_two() && (2..=16384).contains(&steps)
        && max_cells<=100000 && max_products<=1000000,"Invalid spatial RMF proof limits")?;
    check(sweep.options.orientation==Orientation::RotationMinimizing && sweep.frame_laws.is_none()
        && sweep.orientation_guide.is_none() && sweep.contact_point.is_none(),
        "Spatial RMF bound requires an unguided source")?;
    if sweep.options.spacing==Spacing::Parameter {
        return patch_error_from_section(sweep,count,max_products,
            spatial_parameter::section_interpolation(sweep,count,steps,max_cells,shared_rmf)?);
    }
    patch_error_from_section(sweep,count,max_products,
        original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,OriginalTransport::RmfSpatial,None,steps,shared_rmf)?)
}

#[cfg(test)]
#[path = "tests/authored_error_regressions.rs"]
mod regression_tests;
