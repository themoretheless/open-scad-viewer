//! Constructor-owned original-law premises for authored sweep error.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{dot_tight, sub as interval_sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate, scalar_certificate::Status, vector_certificate,
};
mod arc_position;
mod spatial_parameter;
mod spatial_first;
mod corrected_regular;
mod arc_pose;
pub(super) use arc_pose::SharedArcPose;

#[derive(Clone, Debug)]
pub struct InitialCoordinatesReport {
    pub status: Status,
    pub cells: usize,
    pub coordinates: Option<Vec<[[f64; 2]; 3]>>,
    pub reason: Option<&'static str>,
}












#[path = "authored_error/transport/arc_chord.rs"]
mod arc_chord;
pub(super) use arc_chord::{GlobalChord,global as shared_global_chord};

#[derive(Clone, Debug)]
pub struct SectionInterpolationReport {
    pub status: Status,
    pub cells: usize,
    pub error_upper: Option<f64>,
    /// Original enclosure used only to preserve adaptive station selection.
    /// The complete continuous certificate remains `error_upper`.
    pub refinement_error_upper: Option<f64>,
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
    section_interpolation_with_length(sweep,count,max_cells,None,None)
}
fn section_interpolation_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,
    shared_length:Option<&crate::curve_measure::DivisionReport>,shared_chord:Option<&GlobalChord>)->Result<SectionInterpolationReport>{
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
        refinement_error_upper: None,
        endpoint_displacement_upper: None,
        cap_endpoint_displacement_upper: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    if sweep.options.spacing != Spacing::Parameter && !original_line(sweep.path) {
        return original_arc_length_section_interpolation_with_chord(sweep,count,max_cells,shared_length,shared_chord);
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
    original_arc_length_section_interpolation_with_chord(sweep,count,max_cells,shared_length,None)
}
fn original_arc_length_section_interpolation_with_chord(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_chord:Option<&GlobalChord>)->Result<SectionInterpolationReport>{
    let mut out=SectionInterpolationReport {status:Status::Unresolved,cells:0,error_upper:None,
        refinement_error_upper:None,endpoint_displacement_upper:None,cap_endpoint_displacement_upper:None,reason:Some("arc-length-correspondence-unproved")};
    if shared_chord.is_some_and(|c|c.budget_stopped) {out.reason=Some("arc-chord-work-limit");return Ok(out);}
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
    // ONE complete original-path chord cover is shared by every uniform
    // station interval in this profile. The endpoint trajectory errors below
    // already own inverse-length residuals. This geometric proposal never
    // depends on the admission tolerance or a rounded inverse path.
    let global_chord=if let Some(shared)=shared_chord {shared.bound}
    else if path.control_points.len()==path.degree+1 {
        let global=shared_global_chord(path,count,division.total.bounds,max_cells-out.cells)?;
        out.cells+=global.cells;
        if global.budget_stopped {out.reason=Some("arc-chord-work-limit");return Ok(out);}
        global.bound
    }else {None};
    let mut error=0_f64;
    for station in 0..count-1{
        let interval=[station as f64/(count-1) as f64,(station+1) as f64/(count-1) as f64];
        let width=I::point(interval[1]).sub(I::point(interval[0]))?;
        let coarse=I::point(division.total.bounds[1]).mul(width)?.div(I::point(2.))?;
        let position=I::new(0.,global_chord.map_or(coarse.hi,|c|c.min(coarse.hi)))?;
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
    /// Keeps previously accepted adaptive grids unchanged while allowing a
    /// tighter certificate to admit the caller's final permitted grid.
    pub refinement_error_upper: Option<f64>,
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
    patch_error_with_path_evidence(sweep,count,max_cells,max_products,shared_length,None)
}
pub(super) fn patch_error_with_path_evidence(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,
    shared_length:Option<&crate::curve_measure::DivisionReport>,shared_chord:Option<&GlobalChord>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    let section=section_interpolation_with_length(sweep,count,max_cells,shared_length,shared_chord)?;
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
    fixed_patch_error_with_path_evidence(sweep,count,max_cells,max_products,shared_length,None)
}
pub(super) fn fixed_patch_error_with_path_evidence(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_chord:Option<&GlobalChord>)->Result<PatchErrorReport>{
    check(max_products<=1000000,"Patch decomposition work exceeds1000000 products")?;
    if shared_length.is_some() && sweep.options.orientation==Orientation::Fixed && sweep.frame_laws.is_none()
        && sweep.orientation_guide.is_none() && sweep.contact_point.is_none() && !original_line(sweep.path)
        && sweep.options.spacing!=Spacing::Parameter {
        check(count>=sweep.options.initial_sections&&count<=sweep.options.max_sections,"Section count outside configured sweep")?;
        check(max_cells<=100000,"Fixed section work exceeds100000 cells")?;
        return patch_error_from_section(sweep,count,max_products,original_arc_length_section_interpolation_with_chord(sweep,count,max_cells,shared_length,shared_chord)?);
    }
    patch_error_from_section(sweep,count,max_products,fixed_section_interpolation(sweep,count,max_cells)?)
}
// For positive weights, a two-pole degree-one rational curve is
// P(t) = P0 + rho(t)(P1-P0), with rho'(t) > 0. Its ideal tangent
// direction is constant, including oblique and reversed lines. Both RMF
// reflections preserve the perpendicular seed; actual station arithmetic
// is charged by retained endpoint displacement, rather than treated as exact.


pub(super) fn rmf_planar_patch_error(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    rmf_planar_patch_error_with_length(sweep,count,max_cells,max_products,None)
}
pub(super) fn rmf_planar_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    rmf_planar_patch_error_with_pose(sweep,count,max_cells,max_products,shared_length,None)
}
fn rmf_planar_patch_error_with_pose(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_pose:Option<&mut SharedArcPose>)->Result<PatchErrorReport>{
    check(max_cells<=100000&&max_products<=1000000,"RMF section/decomposition work exceeds limits")?;
    let line=super::source_line::certify(sweep.path,max_cells)?;
    let mut section=if line.certified {
        guided_section_interpolation_mode_with_pose(sweep,count,max_cells-line.cells,OriginalTransport::RmfCollinear,shared_pose)?
    }else if shared_length.is_some()&&sweep.options.orientation==Orientation::RotationMinimizing&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        original_frame_arc_length_section_interpolation(sweep,count,max_cells-line.cells,shared_length,OriginalTransport::RmfPlanar,None)?
    }else{guided_section_interpolation_mode(sweep,count,max_cells-line.cells,OriginalTransport::RmfPlanar)?};
    section.cells+=line.cells;
    patch_error_from_section(sweep,count,max_products,section)
}
// Default RMF construction must not stop at a failed seed-aligned plane
// candidate. Reuse the general original Bishop owner, without altering the
// retained frames or resetting work already spent by the cheaper candidate.
pub(super) fn rmf_default_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,
    shared_length:Option<&crate::curve_measure::DivisionReport>,shared_rmf:Option<&super::rmf_transport::OriginalRmfTransportReport>,mut shared_pose:Option<&mut SharedArcPose>)->Result<PatchErrorReport>{
    if sweep.options.spacing==Spacing::Parameter && (shared_rmf.is_some()
        || path_is_closed(sweep.path)? && original_planar_rmf(sweep.path,sweep.options.normal)) {
        // The aggregate owner has already paid for this original source.
        // Reuse its frame/position proof instead of starting a per-profile
        // candidate that cannot share that expenditure.
        return rmf_spatial_patch_error_with_transport(sweep,count,512,max_cells,max_products,shared_length,shared_rmf,shared_pose);
    }
    let planar=rmf_planar_patch_error_with_pose(sweep,count,max_cells,max_products,shared_length,shared_pose.as_deref_mut())?;
    if !matches!(planar.reason,Some("source-plane-coefficients-nonzero"|"rmf-original-frame-correspondence-unproved"|"rmf-closed-correction-correspondence-unproved"))
        || planar.cells>=max_cells || planar.products>max_products {return Ok(planar);}
    let mut general=rmf_spatial_patch_error_with_transport(sweep,count,512,max_cells-planar.cells,
        max_products-planar.products,shared_length,shared_rmf,shared_pose)?;
    general.cells+=planar.cells;
    general.products+=planar.products;
    Ok(general)
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
#[allow(dead_code)]
pub(super) fn corrected_patch_error_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    corrected_patch_error_with_shared_arc_pose(sweep,count,max_cells,max_products,shared_length,None)
}
pub(super) fn corrected_patch_error_with_shared_arc_pose(sweep:&Sweep<'_>,count:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_pose:Option<&mut SharedArcPose>)->Result<PatchErrorReport>{
    check(max_cells<=100000&&max_products<=1000000,"Corrected frame error work exceeds limits")?;
    if shared_pose.as_ref().is_some_and(|p|p.regular_certified) {
        let section=original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,OriginalTransport::CorrectedRegular,None,512,None,shared_pose)?;
        return patch_error_from_section(sweep,count,max_products,section);
    }

    let line=super::source_line::certify(sweep.path,max_cells)?;
    let remaining=max_cells-line.cells;
    let mode=if original_line(sweep.path){OriginalTransport::CorrectedStraight}else if line.certified {OriginalTransport::CorrectedCollinear}else{OriginalTransport::CorrectedPlanar};
    let mut section=if mode==OriginalTransport::CorrectedPlanar&&shared_length.is_some()
        &&sweep.options.orientation==Orientation::CorrectedFrenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none()
        &&sweep.options.spacing!=Spacing::Parameter {
        original_frame_arc_length_section_interpolation(sweep,count,remaining,shared_length,mode,None)?
    }else{guided_section_interpolation_mode(sweep,count,remaining,mode)?};
    if matches!(section.reason,Some("source-plane-coefficients-nonzero"|"corrected-closed-frame-correspondence-unproved")) {
        let spent=section.cells;
        section=if sweep.options.spacing!=Spacing::Parameter {
            original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,remaining-spent,shared_length,OriginalTransport::CorrectedRegular,None,512,None,shared_pose)?
        }else{guided_section_interpolation_mode(sweep,count,remaining-spent,OriginalTransport::CorrectedRegular)?};
        section.cells+=spent;
    }
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


#[cfg(test)]
#[path = "authored_error/tests/tests_integration.rs"]
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

// Prove absence of knots in the OPEN traversal span in each original
// authored domain. Endpoint limits are independently owned above. Interval
// comparisons handle separated knots; ambiguous affine maps use exact sums
// of binary64 products, never rounded normalized knot coordinates.
fn original_laws_have_no_interior_knots(sweep:&Sweep<'_>,interval:[f64;2],cells:&mut usize,max_cells:usize)->Result<bool> {
    let mut curves=[Some(sweep.path),Some(sweep.scale),Some(sweep.twist),None,None,None,None,sweep.orientation_guide];
    if let Some((a,b))=sweep.affine_laws {curves[3]=Some(a);curves[4]=Some(b);}
    if let Some((a,b))=sweep.frame_laws {curves[5]=Some(a);curves[6]=Some(b);}
    for curve in curves.into_iter().flatten() {
        let [a,b]=curve.domain();
        let mut previous=None;
        let width=I::point(b).sub(I::point(a))?;
        let mapped=[
            I::point(a).add(width.mul(I::point(interval[0]))?)?,
            I::point(a).add(width.mul(I::point(interval[1]))?)?,
        ];
        for &k in &curve.knots {
            if k<=a||k>=b||previous==Some(k){continue;}
            previous=Some(k);
            if [a,b]==[0.,1.] {
                if k>interval[0]&&k<interval[1]{return Ok(false);}
                continue;
            }
            if k<=mapped[0].lo||k>=mapped[1].hi{continue;}
            if k>mapped[0].hi&&k<mapped[1].lo{return Ok(false);}
            // k - (a + (b-a)*t), as four exact dyadic products.
            if max_cells.saturating_sub(*cells)<4{return Ok(false);}
            *cells+=4;
            let lower=crate::exact_products::sum_sign(&[(1.,k),(-1.,a),(-b,interval[0]),(a,interval[0])]);
            if lower!=std::cmp::Ordering::Greater{continue;}
            if max_cells.saturating_sub(*cells)<4{return Ok(false);}
            *cells+=4;
            let upper=crate::exact_products::sum_sign(&[(1.,k),(-1.,a),(-b,interval[1]),(a,interval[1])]);
            if upper==std::cmp::Ordering::Less{return Ok(false);}
        }
    }
    Ok(true)
}

pub(super) fn fixed_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize)->Result<SectionInterpolationReport>{
    guided_section_interpolation_mode(sweep,count,max_cells,OriginalTransport::Fixed)
}

#[derive(Clone,Copy,PartialEq)]
enum OriginalTransport {Guided,Contact,Fixed,RmfStraight,RmfCollinear,RmfPlanar,RmfSpatial,CorrectedStraight,CorrectedCollinear,CorrectedPlanar,CorrectedRegular,FixedNormal,Frenet}












fn guided_section_interpolation_mode(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, mode:OriginalTransport,
) -> Result<SectionInterpolationReport> {
    guided_section_interpolation_mode_with_pose(sweep,count,max_cells,mode,None)
}




























































































// Position uses original cumulative length; relative pose uses original source
// parameter brackets and independent normalized station-law intervals. The
// interval image remainder needs no unproved inverse-length derivative jets.




fn original_frame_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,mode,shared_guide,512,None,None)
}






#[cfg(test)]
fn check_nonaxial_planar_rmf_original_image(spacing:Spacing){
    // C(u)=(u,-u,u²), N=(1,1,0)/sqrt(2). The independent Bishop
    // binormal is (-sqrt(2)u,sqrt(2)u,sqrt(2))/sqrt(2+4u²).
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.5,-0.5,0.],vec![1.,-1.,1.]],weights:vec![1.;3],periodic:false};
    let profile=crate::primitives::line([1.,1.,0.],[2.,2.,0.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,1.,0.],orientation:Orientation::RotationMinimizing,
        spacing,
        initial_sections:3,max_sections:17,max_deviation:2.};
    assert!(!original_planar_rmf(&path,opts.normal));
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    let upper=proof.error_upper.unwrap();assert!(upper.is_finite());
    eprintln!("nonaxial original-plane RMF upper={upper}, cells={}",proof.cells);
    assert!(proof.within_budget&&upper<2.);
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let root=2_f64.sqrt();
    let length=|u:f64|u*(2.+4.*u*u).sqrt()/2.+(root*u).asinh()/2.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=if spacing==Spacing::Parameter{t}else{(lo+hi)/2.};let angle=0.125*t;
        let x=(1.+t).powi(2)*root*(1.+q)+t/8.;let y=t/4.;
        let n=x*angle.cos()-y*angle.sin();let side=x*angle.sin()+y*angle.cos();
        let d=(2.+4.*u*u).sqrt();
        let expected=[u+n/root-root*u*side/d,-u+n/root+root*u*side/d,u*u+root*side/d];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.rmf_planar_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let mut off_plane=path.clone();off_plane.control_points[1][1]=(-0.5_f64).next_up();
    let wrong=Sweep::new(&profile,&off_plane,&scale,&twist,opts).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(wrong.status,Status::Unresolved);assert!(wrong.error_upper.is_none());
    let mut closed=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
    for p in &mut closed.control_points{*p=vec![p[0],-p[0],p[1]];}
    let closed=Sweep::new(&profile,&closed,&scale,&twist,Options{normal:[1.,0.,1.],..opts}).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(closed.status,Status::Unresolved);assert!(closed.error_upper.is_none());
}
#[cfg(test)]
fn check_original_planar_frame_arc_length(orientation:Orientation){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    // Exact original polynomial C(u)=(0,u²,u). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let certify=|cells|if orientation==Orientation::FixedNormal {sweep.fixed_normal_patch_error_bound(17,cells,1000000)}else{sweep.rmf_planar_patch_error_bound(17,cells,1000000)};
    let proof=certify(10000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let angle=0.125*t;
        let x=(1.+t).powi(2)*(1.+q)+t/8.;let y=t/4.;
        let d=(1.+4.*u*u).sqrt();let side=x*angle.sin()+y*angle.cos();
        let expected=[x*angle.cos()-y*angle.sin(),u*u+side/d,u-2.*u*side/d];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=certify(proof.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    if orientation==Orientation::RotationMinimizing{
        let mut off_plane=path.clone();off_plane.control_points[1][0]=f64::from_bits(1);
        let wrong=Sweep::new(&profile,&off_plane,&scale,&twist,opts).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
        assert_eq!(wrong.status,Status::Unresolved);assert!(wrong.error_upper.is_none());
        assert!(matches!(wrong.reason,Some("source-plane-coefficients-nonzero"|"source-plane-exact-work-unproved")));
        let closed=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
        let one=constant_vector_law([1.,0.,0.]).unwrap();let zero=constant_vector_law([0.;3]).unwrap();
        let unresolved=Sweep::new(&profile,&closed,&one,&zero,Options {normal:[1.,0.,1.],initial_sections:5,..opts}).unwrap().rmf_planar_patch_error_bound(17,10000,1000000).unwrap();
        assert_eq!(unresolved.status,Status::Unresolved);assert!(unresolved.error_upper.is_none());
    }
}















































pub(super) fn rmf_spatial_patch_error(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize)->Result<PatchErrorReport>{
    rmf_spatial_patch_error_with_length(sweep,count,steps,max_cells,max_products,None)
}
pub(super) fn rmf_spatial_patch_error_with_length(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<PatchErrorReport>{
    rmf_spatial_patch_error_with_transport(sweep,count,steps,max_cells,max_products,shared_length,None,None)
}
pub(super) fn rmf_spatial_patch_error_with_transport(sweep:&Sweep<'_>,count:usize,steps:usize,max_cells:usize,max_products:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_rmf:Option<&super::rmf_transport::OriginalRmfTransportReport>,shared_pose:Option<&mut SharedArcPose>)->Result<PatchErrorReport>{
    check(count>=sweep.options.initial_sections && count<=sweep.options.max_sections
        && steps.is_power_of_two() && (2..=16384).contains(&steps)
        && max_cells<=100000 && max_products<=1000000,"Invalid spatial RMF proof limits")?;
    check(sweep.options.orientation==Orientation::RotationMinimizing && sweep.frame_laws.is_none()
        && sweep.orientation_guide.is_none() && sweep.contact_point.is_none(),
        "Spatial RMF bound requires an unguided source")?;
    if sweep.options.spacing==Spacing::Parameter {
        return patch_error_from_section(sweep,count,max_products,
            spatial_parameter::section_interpolation(sweep,count,steps,max_cells,shared_rmf,shared_pose)?);
    }
    patch_error_from_section(sweep,count,max_products,
        original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,OriginalTransport::RmfSpatial,None,steps,shared_rmf,shared_pose)?)
}










pub(super) fn corrected_regular_frame_values(sweep:&Sweep<'_>,twist:&Curve,t:[f64;2],budget:usize)->Result<authored_frame_certificate::ValuesReport>{
 let refused=|cells|authored_frame_certificate::ValuesReport{status:Status::Unresolved,cells,longitudinal:None,transverse:None,binormal:None};
 if sweep.options.orientation!=Orientation::CorrectedFrenet||sweep.path.periodic||path_is_closed(sweep.path)?||sweep.frame_laws.is_some()||sweep.orientation_guide.is_some()||sweep.contact_point.is_some(){return Ok(refused(0));}
 let eligibility=corrected_regular::certify(sweep.path,budget)?;
 if !eligibility.certified{return Ok(refused(eligibility.cells));}
 let mut values=authored_frame_certificate::certify_frenet_path_values(sweep.path,twist,t,budget-eligibility.cells)?;
 values.cells+=eligibility.cells;Ok(values)
}
















































#[cfg(test)]
#[path = "authored_error/tests/extended_regressions.rs"]
mod extended_regressions;

#[path = "authored_error/initial_coordinates_owned.rs"]
mod initial_coordinates_owned;
pub(super) use initial_coordinates_owned::initial_coordinates;
pub(super) use initial_coordinates_owned::fixed_initial_coordinates;
pub(super) use initial_coordinates_owned::fixed_normal_initial_coordinates;
use initial_coordinates_owned::seed_initial_coordinates;
use initial_coordinates_owned::seed_initial_coordinates_as;
use initial_coordinates_owned::principal_initial_coordinates;
pub(super) use initial_coordinates_owned::coordinates_in_basis;
pub(super) use initial_coordinates_owned::guided_initial_coordinates;
pub(super) use initial_coordinates_owned::control_trajectory;

#[path = "authored_error/guided_interpolation_owned.rs"]
mod guided_interpolation_owned;
use guided_interpolation_owned::guided_section_interpolation_mode_with_pose;

#[path = "authored_error/arc_frame_owned.rs"]
mod arc_frame_owned;
use arc_frame_owned::original_frame_arc_length_section_interpolation_with_rmf_steps;
use arc_frame_owned::closed_frenet_position_seam;
use arc_frame_owned::fixed_normal_arc_length_section_interpolation;

#[path = "authored_error/arc_position_owned.rs"]
mod arc_position_owned;

#[path = "authored_error/retained_patch_assembly.rs"]
mod retained_patch_assembly;
use retained_patch_assembly::patch_error_from_section;
pub(super) use retained_patch_assembly::original_line;
pub(super) use retained_patch_assembly::original_planar_rmf;
