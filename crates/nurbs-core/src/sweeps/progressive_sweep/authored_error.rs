//! Constructor-owned original-law premises for authored sweep error.
use super::*;
use crate::distance_bounds::Interval as I;
use crate::numerics::interval_vec3::{dot_tight, sub as interval_sub};
use crate::sweeps::progressive_miter::{
    authored_frame_certificate, scalar_certificate::Status, vector_certificate,
};
mod spatial_parameter;

#[derive(Clone, Debug)]
pub struct InitialCoordinatesReport {
    pub status: Status,
    pub cells: usize,
    pub coordinates: Option<Vec<[[f64; 2]; 3]>>,
    pub reason: Option<&'static str>,
}
pub(super) fn initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-authored"),
    };
    let Some((axis, normal)) = sweep.frame_laws else {
        return Ok(out);
    };
    out.reason = Some("path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    // The initial local basis precedes twist and affine transformation.
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_twisted_values(
        axis,
        normal,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.status = Status::Certified;
    out.reason = None;
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    Ok(out)
}
pub(super) fn fixed_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates(sweep,max_cells,Orientation::Fixed)
}
pub(super) fn fixed_normal_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates(sweep,max_cells,Orientation::FixedNormal)
}
fn seed_initial_coordinates(sweep:&Sweep<'_>,max_cells:usize,orientation:Orientation)->Result<InitialCoordinatesReport>{
    seed_initial_coordinates_as(sweep,max_cells,orientation,orientation)
}
fn seed_initial_coordinates_as(sweep:&Sweep<'_>,max_cells:usize,orientation:Orientation,actual:Orientation)->Result<InitialCoordinatesReport>{
    check(max_cells<=100000,"Fixed initial coordinate work exceeds100000 cells")?;
    let mut out=InitialCoordinatesReport {status:Status::Unresolved,cells:0,coordinates:None,reason:Some("mode-not-fixed")};
    if sweep.options.orientation!=actual || sweep.frame_laws.is_some() || sweep.orientation_guide.is_some(){return Ok(out);}
    let start=vector_certificate::certify_values_traversal(sweep.path,[0.,0.],max_cells,false)?;
    out.cells+=start.cells;out.reason=Some("path-start-enclosure-unresolved");
    let Some(start)=start.value else {return Ok(out);};
    let zero=constant_vector_law([0.;3])?;
    let frame=if orientation==Orientation::Frenet {
        authored_frame_certificate::certify_frenet_path_values(sweep.path,&zero,[0.,0.],max_cells-out.cells)?
    } else {authored_frame_certificate::certify_fixed_path_values(sweep.path,sweep.options.normal,&zero,[0.,0.],max_cells-out.cells)?};
    out.cells+=frame.cells;out.reason=Some("fixed-initial-frame-unresolved");
    if frame.status!=Status::Certified {return Ok(out);}
    out.coordinates=Some(coordinates_in_basis(sweep.profile,start,frame)?);
    out.status=Status::Certified;out.reason=None;Ok(out)
}

fn principal_initial_coordinates(sweep:&Sweep<'_>,phase:f64,max_cells:usize)->Result<InitialCoordinatesReport>{
    let start=vector_certificate::certify_values_traversal(sweep.path,[0.,0.],max_cells,false)?;
    let mut out=InitialCoordinatesReport {status:Status::Unresolved,cells:start.cells,coordinates:None,reason:Some("corrected-initial-phase-coordinates-unproved")};
    let Some(start)=start.value else{return Ok(out);};
    let zero=constant_vector_law([0.;3])?;
    let frame=authored_frame_certificate::certify_planar_principal_values(sweep.path,sweep.options.normal,phase,&zero,[0.,0.],[0.,0.],max_cells-out.cells)?;
    out.cells+=frame.cells;
    if frame.status==Status::Certified{out.coordinates=Some(coordinates_in_basis(sweep.profile,start,frame)?);out.status=Status::Certified;out.reason=None;}
    Ok(out)
}

fn coordinates_in_basis(
    profile: &Curve,
    start: [[f64; 2]; 3],
    frame: authored_frame_certificate::ValuesReport,
) -> Result<Vec<[[f64; 2]; 3]>> {
    let decode = |v: [[f64; 2]; 3]| -> Result<[I; 3]> {
        Ok([
            I::new(v[0][0], v[0][1])?,
            I::new(v[1][0], v[1][1])?,
            I::new(v[2][0], v[2][1])?,
        ])
    };
    let start = decode(start)?;
    let basis = [
        decode(frame.transverse.unwrap())?,
        decode(frame.binormal.unwrap())?,
        decode(frame.longitudinal.unwrap())?,
    ];
    let mut coordinates = Vec::with_capacity(profile.control_points.len());
    for pole in &profile.control_points {
        let offset = interval_sub(
            [I::point(pole[0]), I::point(pole[1]), I::point(pole[2])],
            start,
        )?;
        let q = [
            dot_tight(offset, basis[0])?,
            dot_tight(offset, basis[1])?,
            dot_tight(offset, basis[2])?,
        ];
        coordinates.push(q.map(|x| [x.lo, x.hi]));
    }
    Ok(coordinates)
}

pub(super) fn guided_initial_coordinates(
    sweep: &Sweep<'_>,
    max_cells: usize,
) -> Result<InitialCoordinatesReport> {
    check(
        max_cells <= 100000,
        "Initial coordinate work exceeds100000 cells",
    )?;
    let mut out = InitialCoordinatesReport {
        status: Status::Unresolved,
        cells: 0,
        coordinates: None,
        reason: Some("mode-not-guided"),
    };
    let Some(guide) = sweep.orientation_guide else {
        return Ok(out);
    };
    out.reason = Some("guided-path-start-enclosure-unresolved");
    let start =
        vector_certificate::certify_values_traversal(sweep.path, [0., 0.], max_cells, false)?;
    out.cells += start.cells;
    let Some(start) = start.value else {
        return Ok(out);
    };
    let zero_twist = constant_vector_law([0.; 3])?;
    out.reason = Some("guided-initial-frame-enclosure-unresolved");
    let frame = authored_frame_certificate::certify_path_guide_values(
        sweep.path,
        guide,
        &zero_twist,
        [0., 0.],
        max_cells - out.cells,
    )?;
    out.cells += frame.cells;
    if frame.status != Status::Certified {
        return Ok(out);
    }
    out.coordinates = Some(coordinates_in_basis(sweep.profile, start, frame)?);
    out.status = Status::Certified;
    out.reason = None;
    Ok(out)
}

pub(super) fn control_trajectory(
    sweep: &Sweep<'_>,
    profile_control: usize,
    traversal: [f64; 2],
    max_cells: usize,
) -> Result<authored_frame_certificate::TrajectoryReport> {
    check(
        profile_control < sweep.profile.control_points.len(),
        "Profile control index outside sweep",
    )?;
    check(
        max_cells <= 100000,
        "Control trajectory work exceeds100000 cells",
    )?;
    let unresolved = |cells, reason| authored_frame_certificate::TrajectoryReport {
        traversal,
        status: Status::Unresolved,
        cells,
        jet: None,
        single_span: false,
        reason: Some(reason),
    };
    if sweep.options.spacing != Spacing::Parameter && !original_line(sweep.path) {
        return Ok(unresolved(0, "arc-length-correspondence-unproved"));
    }
    let length_reference=(sweep.options.spacing != Spacing::Parameter).then(|| Curve {
        degree:1,knots:vec![0.,0.,1.,1.],control_points:sweep.path.control_points.clone(),
        weights:vec![1.,1.],periodic:false,
    });
    let reference_path=length_reference.as_ref().unwrap_or(sweep.path);
    let [a, b] = sweep.path.domain();
    let p = sweep.path.evaluate(a)?.point;
    let q = sweep.path.evaluate(b)?.point;
    // Fixed authored transport has no closed holonomy correction. This report
    // encloses the original trajectory; retained closing displacement is a
    // separate obligation handled by the section interpolation owner.
    if closed_extent(sweep.path, [p[0], p[1], p[2]], [q[0], q[1], q[2]])?
        && sweep.options.orientation!=Orientation::Fixed {
        return Ok(unresolved(0, "closed-frame-correction-unproved"));
    }
    let initial = initial_coordinates(sweep, max_cells)?;
    if initial.status != Status::Certified {
        return Ok(unresolved(initial.cells, initial.reason.unwrap()));
    }
    let (axis, normal) = sweep.frame_laws.unwrap();
    let mut report = authored_frame_certificate::certify_control_trajectory(
        reference_path,
        sweep.scale,
        sweep.twist,
        axis,
        normal,
        sweep.affine_laws,
        initial.coordinates.unwrap()[profile_control],
        traversal,
        max_cells - initial.cells,
    )?;
    report.cells += initial.cells;
    Ok(report)
}

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
mod tests {
    use super::*;
    #[test]
    fn owned_dense_profile_patch_error_includes_decomposition_and_product_budget() {
        let profile = Curve {
            degree: 1,
            knots: std::iter::once(0.)
                .chain((0..=32).map(|i| i as f64))
                .chain(std::iter::once(32.))
                .collect(),
            control_points: (0..=32)
                .map(|i| vec![1. + i as f64 / 32., (i % 2) as f64 / 64., 0.])
                .collect(),
            weights: (0..=32).map(|i| if i % 2 == 0 { 1. } else { 2. }).collect(),
            periodic: false,
        };
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let axis = constant_vector_law([0., 0., 1.]).unwrap();
        let normal = constant_vector_law([1., 0., 0.]).unwrap();
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 3,
            max_deviation: 0.01,
        };
        let sweep =
            Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
        let report = sweep.authored_patch_error_bound(3, 10000, 1000).unwrap();
        assert_eq!(report.status, Status::Certified);
        assert_eq!(report.products, 384);
        assert!(report.within_budget && report.error_upper.unwrap() < 1e-9);
        assert!(report.decomposition_error_upper.unwrap() > 0.);
        let endpoint_upper=report.retained_endpoint_error_upper.unwrap();
        assert!(endpoint_upper>=report.original_section_endpoint_error_upper.unwrap());
        assert!(endpoint_upper>=report.decomposition_error_upper.unwrap());
        let patches = report.patches.unwrap();
        assert_eq!(patches.len(), 32);
        let (s, c) = 0.25_f64.sin_cos();
        for (span, patch) in patches.iter().enumerate() {
            let a = patch.knots_u[patch.degree_u];
            let b = patch.knots_u[patch.control_points.len()];
            for f in [0., 0.375, 1.] {
                let p = profile.evaluate(span as f64 + f).unwrap().point;
                for t in [0., 0.375, 1.] {
                    let got = patch.evaluate(a + (b - a) * f, t).unwrap().point;
                    let expected = [p[0] * c - p[1] * s, p[0] * s + p[1] * c, 10. * t];
                    assert!(
                        norm(std::array::from_fn(|k| got[k] - expected[k]))
                            <= report.error_upper.unwrap()
                    );
                    if t==0. || t==1. {
                        assert!(norm(std::array::from_fn(|k|got[k]-expected[k]))<=endpoint_upper);
                    }
                }
            }
        }
        let refused = sweep.authored_patch_error_bound(3, 10000, 383).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.retained_endpoint_error_upper.is_none());
        assert!(
            refused.patches.is_none() && refused.error_upper.is_none() && refused.products <= 383
        );
    }
    #[test]
    fn owned_section_bound_covers_varying_joint_laws_and_refinement() {
        let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        profile.weights = vec![1., 2.];
        let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
        let axis = constant_vector_law([0., 0., 1.]).unwrap();
        let normal = crate::primitives::line([1., 0., 0.], [1., 1., 0.]).unwrap();
        let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
        let twist = crate::primitives::line([0., 0., 0.], [0.25, 0., 0.]).unwrap();
        let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
        let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
        let options = Options {
            normal: [1., 0., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 9,
            max_deviation: 0.01,
        };
        let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
        let coarse = sweep
            .authored_section_interpolation_bound(3, 10000)
            .unwrap();
        let fine = sweep
            .authored_section_interpolation_bound(9, 10000)
            .unwrap();
        assert_eq!(coarse.status, Status::Certified);
        assert_eq!(fine.status, Status::Certified);
        assert!(fine.error_upper.unwrap() < coarse.error_upper.unwrap());
        let sections = sweep.sections(3).unwrap().0;
        for i in 0..=32 {
            let t = i as f64 / 32.;
            let station = (i / 16).min(1);
            let fraction = t * 2. - station as f64;
            for u in [0., 0.375, 1.] {
                let a = sections[station].evaluate(u).unwrap().point;
                let b = sections[station + 1].evaluate(u).unwrap().point;
                let amplitude = (1. + 3. * u) / (1. + u) * (1. + t).powi(2) + 0.5 * t;
                let angle = t.atan() + 0.25 * t;
                let ideal = [amplitude * angle.cos(), amplitude * angle.sin(), 10. * t];
                let delta =
                    std::array::from_fn(|k| (1. - fraction) * a[k] + fraction * b[k] - ideal[k]);
                assert!(norm(delta) <= coarse.error_upper.unwrap());
            }
        }
    }
    #[test]
    fn authored_initial_coordinates_enclose_original_profile_in_independent_domains() {
        let mut profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
        profile.weights = vec![1., 2.];
        let mut path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
        path.knots = vec![17., 17., 19., 19.];
        let mut axis = constant_vector_law([2., 0., 0.]).unwrap();
        axis.knots = vec![2., 2., 5., 5.];
        let mut normal = constant_vector_law([7., 1., 0.]).unwrap();
        normal.knots = vec![31., 31., 41., 41.];
        let scale = constant_vector_law([1., 0., 0.]).unwrap();
        let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
        let options = Options {
            normal: [0., 1., 0.],
            orientation: Orientation::Fixed,
            spacing: Spacing::Parameter,
            initial_sections: 3,
            max_sections: 3,
            max_deviation: 0.01,
        };
        let sweep =
            Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
        let report = sweep.authored_initial_coordinates(100).unwrap();
        assert_eq!(report.status, Status::Certified);
        assert_eq!(report.cells, 10);
        let coordinates = report.coordinates.unwrap();
        for (q, expected) in coordinates.iter().zip([[1., 4., 0.5], [1., 4., 1.5]]) {
            for k in 0..3 {
                assert!(q[k][0] <= expected[k] && expected[k] <= q[k][1]);
            }
        }
        let trajectory = sweep.authored_control_trajectory(0, [0., 1.], 100).unwrap();
        assert_eq!(trajectory.status, Status::Certified);
        assert_eq!(trajectory.cells, 21);
        let jet = trajectory.jet.unwrap();
        let (s, c) = 0.25_f64.sin_cos();
        for i in 0..=16 {
            let t = i as f64 / 16.;
            let expected = [1., 1. + c - 4. * s, -1. + 10. * t + s + 4. * c];
            for k in 0..3 {
                assert!(jet.value[k][0] <= expected[k] && expected[k] <= jet.value[k][1]);
                let derivative = if k == 2 { 10. } else { 0. };
                assert!(jet.first[k][0] <= derivative && derivative <= jet.first[k][1]);
            }
        }
        let limited = sweep.authored_control_trajectory(0, [0., 1.], 20).unwrap();
        assert_eq!(limited.status, Status::Unresolved);
        assert!(limited.jet.is_none() && limited.cells <= 20);
        let bound = sweep.authored_section_interpolation_bound(3, 1000).unwrap();
        assert_eq!(bound.status, Status::Certified);
        // Original source premises are shared across profile controls.
        assert_eq!(bound.cells, 65);
        assert!(bound.error_upper.unwrap() < 1e-10);
        assert!(bound.endpoint_displacement_upper.unwrap() < 1e-10);
        let incomplete = sweep
            .authored_section_interpolation_bound(3, bound.cells - 1)
            .unwrap();
        assert_eq!(incomplete.status, Status::Unresolved);
        assert!(
            incomplete.error_upper.is_none() && incomplete.endpoint_displacement_upper.is_none()
        );
        assert!(incomplete.cells < bound.cells);
        assert!(sweep.authored_control_trajectory(2, [0., 1.], 100).is_err());
        for budget in [0, 9] {
            let refused = sweep.authored_initial_coordinates(budget).unwrap();
            assert_eq!(refused.status, Status::Unresolved);
            assert!(refused.coordinates.is_none() && refused.cells <= budget);
        }
        let plain = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
        assert_eq!(
            plain.authored_initial_coordinates(100).unwrap().reason,
            Some("mode-not-authored")
        );
        let arc_options = Options {
            spacing: Spacing::ArcLength {
                tolerance: 0.001,
                max_cells: 100,
            },
            ..options
        };
        let arc = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, arc_options)
            .unwrap();
        assert_eq!(arc.authored_control_trajectory(0,[0.,1.],100).unwrap().status,Status::Certified);
    }
}

#[test]
fn original_knot_transition_uses_owned_value_bound_and_shared_work() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.knots = vec![2., 2., 3.2, 5., 5.];
    path.control_points = vec![vec![0., 0., 0.], vec![0., 0., 3.], vec![0., 0., 10.]];
    path.weights = vec![1.; 3];
    let constant = |p: [f64; 3]| {
        let mut curve = profile.clone();
        curve.control_points = vec![p.to_vec(); 2];
        curve
    };
    let axis = constant([0., 0., 1.]);
    let normal = constant([1., 0., 0.]);
    let scale = constant([1., 0., 0.]);
    let twist = constant([0.; 3]);
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    let sweep =
        Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options).unwrap();
    let report = sweep
        .authored_section_interpolation_bound(3, 10000)
        .unwrap();
    assert_eq!(report.status, Status::Certified);
    assert!(report.error_upper.unwrap() > 0.5);
    let sections = sweep.sections(3).unwrap().0;
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let expected = path.evaluate(2. + 3. * t).unwrap().point[2];
        let station = if t < 0.5 { 0 } else { 1 };
        let f = 2. * t - station as f64;
        let got = (1. - f) * sections[station].control_points[0][2]
            + f * sections[station + 1].control_points[0][2];
        assert!((got - expected).abs() <= report.error_upper.unwrap());
    }
    let exhausted = sweep
        .authored_section_interpolation_bound(3, report.cells - 1)
        .unwrap();
    assert_eq!(exhausted.status, Status::Unresolved);
    assert!(exhausted.error_upper.is_none());
    assert!(exhausted.cells < report.cells);
}

#[test]
fn knot_fallback_covers_every_authored_transport_law() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let constant = |p: [f64; 3]| {
        let mut c = profile.clone();
        c.control_points = vec![p.to_vec(); 2];
        c
    };
    let piecewise = |a: [f64; 3], b: [f64; 3], c: [f64; 3], domain: [f64; 2]| Curve {
        degree: 1,
        knots: vec![
            domain[0],
            domain[0],
            domain[0] + 0.4 * (domain[1] - domain[0]),
            domain[1],
            domain[1],
        ],
        control_points: vec![a.to_vec(), b.to_vec(), c.to_vec()],
        weights: vec![1., 0.5, 2.],
        periodic: false,
    };
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::Fixed,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    for law_index in 0..7 {
        let mut scale = constant([1., 0., 0.]);
        let mut twist = constant([0.; 3]);
        let mut axis = constant([0., 0., 1.]);
        let mut normal = constant([1., 0., 0.]);
        let mut axes = constant([1.; 3]);
        let mut center = constant([0.; 3]);
        match law_index {
            0 => scale = piecewise([1., 0., 0.], [1.3, 0., 0.], [2., 0., 0.], [2., 5.]),
            1 => twist = piecewise([0.; 3], [0.1, 0., 0.], [0.3, 0., 0.], [7., 9.]),
            2 => axis = piecewise([0., 0., 1.], [0., 0.2, 1.], [0., 0.5, 1.], [17., 19.]),
            3 => normal = piecewise([1., 0., 0.], [1., 0.2, 0.], [1., 1., 0.], [23., 29.]),
            4 => axes = piecewise([1.; 3], [1.3, 1., 1.], [2., 1., 1.], [31., 41.]),
            _ => center = piecewise([0.; 3], [0.1, 0., 0.], [0.5, 0., 0.], [43., 47.]),
        }
        if law_index == 6 {
            scale = piecewise([1., 0., 0.], [1.3, 0., 0.], [2., 0., 0.], [2., 5.]);
            twist = piecewise([0.; 3], [0.1, 0., 0.], [0.3, 0., 0.], [7., 9.]);
            axis = piecewise([0., 0., 1.], [0., 0.2, 1.], [0., 0.5, 1.], [17., 19.]);
            normal = piecewise([1., 0., 0.], [1., 0.2, 0.], [1., 1., 0.], [23., 29.]);
            axes = piecewise([1.; 3], [1.3, 1., 1.], [2., 1., 1.], [31., 41.]);
        }
        let sweep = Sweep::new_authored(&profile, &path, &scale, &twist, &axis, &normal, options)
            .unwrap()
            .with_affine_laws(&axes, &center)
            .unwrap();
        let report = sweep
            .authored_section_interpolation_bound(3, 10000)
            .unwrap();
        assert_eq!(
            report.status,
            Status::Certified,
            "law {law_index}: {:?}",
            report.reason
        );
        assert!(report.error_upper.unwrap().is_finite());
        let retained = sweep.sections(3).unwrap().0;
        let original = sweep.sections(101).unwrap().0;
        for (i, section) in original.iter().enumerate() {
            let t = i as f64 / 100.;
            let station = if t < 0.5 { 0 } else { 1 };
            let f = 2. * t - station as f64;
            for k in 0..2 {
                let delta: [f64; 3] = std::array::from_fn(|a| {
                    section.control_points[k][a]
                        - ((1. - f) * retained[station].control_points[k][a]
                            + f * retained[station + 1].control_points[k][a])
                });
                assert!(
                    norm(delta) <= report.error_upper.unwrap(),
                    "law {law_index}"
                );
            }
        }
        let refused = sweep.authored_section_interpolation_bound(3, 0).unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.error_upper.is_none());
    }
}

#[test]
fn guided_initial_coordinates_own_translated_profile_before_twist_and_affine() {
    let mut profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    profile.weights = vec![1., 2.];
    let mut path = crate::primitives::line([0.5, 1., -1.], [0.5, 1., 9.]).unwrap();
    path.knots = vec![2., 2., 5., 5.];
    let mut guide = crate::primitives::line([0.5, 2., -1.], [0.5, 2., 9.]).unwrap();
    guide.knots = vec![31., 31., 41., 41.];
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let axes = constant_vector_law([1., 2., 3.]).unwrap();
    let center = constant_vector_law([0.5, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let report = sweep.guided_initial_coordinates(100).unwrap();
    assert_eq!(report.status, Status::Certified);
    let coordinates = report.coordinates.unwrap();
    for (q, expected) in coordinates.iter().zip([[1., -0.5, 4.], [1., -1.5, 4.]]) {
        for k in 0..3 {
            assert!(q[k][0] <= expected[k] && expected[k] <= q[k][1]);
            assert!(q[k][1] - q[k][0] < 1e-10);
        }
    }
    let refused = sweep.guided_initial_coordinates(report.cells - 1).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.coordinates.is_none());
}

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

#[test]
fn guided_control_owns_initial_coordinates_and_whole_request_budget() {
    let profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = sweep.guided_control_value(0, [0.25, 0.5], 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    let value = report.value.unwrap();
    for t in [0.25, 0.375, 0.5] {
        let expected = [
            0.25_f64.cos() - 2. * 0.25_f64.sin(),
            0.25_f64.sin() + 2. * 0.25_f64.cos(),
            3. + 10. * t,
        ];
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
        }
    }
    let refused = sweep
        .guided_control_value(0, [0.25, 0.5], report.cells - 1)
        .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.value.is_none());
    assert!(sweep.guided_control_value(2, [0.25, 0.5], 1000).is_err());
}

#[test]
fn guided_control_does_not_promote_contact_or_arc_length_correspondence() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let contact = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_contact_guide(&guide, 0.)
        .unwrap();
    let report = contact.guided_control_value(0, [0., 1.], 1000).unwrap();
    assert_eq!(report.reason, Some("contact-width-law-unproved"));
    assert!(report.value.is_none() && report.cells == 0);
    let arc_options = super::Options {
        spacing: Spacing::ArcLength {
            tolerance: 0.001,
            max_cells: 1000,
        },
        ..options
    };
    let arc = Sweep::new(&profile, &path, &scale, &twist, arc_options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = arc.guided_control_value(0, [0., 1.], 1000).unwrap();
    assert_eq!(report.reason, Some("arc-length-correspondence-unproved"));
    assert!(report.value.is_none() && report.cells == 0);
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

#[test]
fn oblique_corrected_binormal_reference_preserves_constant_world_offset() {
    let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,-1.,0.],vec![2.,0.,-2.],vec![2.,-2.,0.],vec![1.,-1.,0.]],None).unwrap();
    let profile=crate::primitives::line([1.;3],[2.;3]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let options=Options {normal:[1.;3],orientation:Orientation::CorrectedFrenet,spacing:Spacing::Parameter,
        initial_sections:2,max_sections:17,max_deviation:10.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
    for count in [2,3,5,9,17] {
        let proof=corrected_patch_error_with_length(&sweep,count,10000,1000000,None).unwrap();
        assert_eq!(proof.status,Status::Certified,"{proof:?}");
        assert!(proof.within_budget,"{proof:?}");
        let upper=proof.error_upper.unwrap();assert!(upper<10.);
        let sections=sweep.sections_at(count).unwrap();
        for i in 0..count-1 {for f in [0.125,0.5,0.875] {
            let t=(i as f64+f)/(count-1) as f64;let s=1.-t;
            let source=[4.*s*s*s*t+12.*s*s*t*t+8.*s*t*t*t+t*t*t*t,
                -4.*s*s*s*t-8.*s*t*t*t-t*t*t*t,-12.*s*s*t*t];
            for j in 0..2 {
                let expected=source.map(|v|v+(j+1) as f64);
                let retained:V=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]);
                assert!(norm(sub(expected,retained))<=upper,"{count}/{i}/{f}/{j}/{upper}");
            }
        }}
        let short=corrected_patch_error_with_length(&sweep,count,proof.cells-1,1000000,None).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    }
}
#[test]
fn corrected_planar_affine_phase_matches_independent_rational_reference(){
    let scale=crate::primitives::line([1.,0.,0.],[1.25,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axis=Curve {degree:1,knots:vec![7.,7.,9.,9.],control_points:vec![vec![1.,2.,1.],vec![1.1,1.75,1.2]],weights:vec![1.,2.],periodic:false};
    let center=Curve {degree:1,knots:vec![11.,11.,13.,13.],control_points:vec![vec![0.1,-0.2,0.03],vec![0.12,-0.18,0.04]],weights:vec![1.,2.],periodic:false};
    let profile=crate::primitives::line([-1.,1.,1.],[-2.,2.,2.]).unwrap();
    for rational in [false,true] {for oblique in [false,true] {for direction in [1.,-3.] {
        let plane=if oblique {[direction;3]}else{[0.,0.,direction]};
        let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
            control_points:vec![vec![0.,0.,0.],vec![1.,1.,if oblique {-2.}else{0.}],vec![2.,-1.,if oblique {-1.}else{0.}],vec![3.,0.,if oblique {-3.}else{0.}]],
            weights:if rational {vec![1.,2.,2.,1.]}else{vec![1.;4]},periodic:false};
        let original=path.clone();
        let options=Options {orientation:Orientation::CorrectedFrenet,normal:plane,spacing:Spacing::Parameter,
            initial_sections:3,max_sections:129,max_deviation:0.1};
        let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_affine_laws(&axis,&center).unwrap();
        let proof=corrected_patch_error_with_length(&sweep,129,10000,1000000,None).unwrap();
        assert_eq!(proof.status,Status::Certified,"{rational}/{plane:?}/{proof:?}");
        assert!(proof.within_budget,"{proof:?}");
        let upper=proof.error_upper.unwrap();let sections=sweep.sections_at(129).unwrap();
        for i in 0..128 {for f in [0.125,0.5,0.875] {
            let t=(i as f64+f)/128.;
            let (x,y,dx,dy)=if rational {
                let w=1.+3.*t-3.*t*t;let dw=3.-6.*t;
                let x=6.*t-3.*t*t*t;let y=6.*t-18.*t*t+12.*t*t*t;
                (x/w,y/w,((6.-9.*t*t)*w-x*dw)/(w*w),((6.-36.*t+36.*t*t)*w-y*dw)/(w*w))
            }else{(3.*t,3.*t-9.*t*t+6.*t*t*t,3.,3.-18.*t+18.*t*t)};
            // Independent analytic rational quotient, with an exact linear
            // embedding into the oblique plane. No constructor/frame replay.
            let tangent=unit([dx,dy,if oblique {-dx-dy}else{0.}]).unwrap();
            let initial_tangent=unit([1.,1.,if oblique {-2.}else{0.}]).unwrap();
            let binormal=unit(if oblique {[-1.;3]}else{[0.,0.,-1.]}).unwrap();
            let initial_normal=cross(binormal,initial_tangent);let normal=cross(binormal,tangent);
            let (sin,cos)=(0.125*t).sin_cos();
            for j in 0..2 {
                let pole=[-(j as f64+1.),j as f64+1.,j as f64+1.];
                let a=(1.+0.25*t)*(1.+1.2*t)/(1.+t)*dot(pole,initial_normal)+(0.1+0.14*t)/(1.+t);
                let b=(1.+0.25*t)*(2.+1.5*t)/(1.+t)*dot(pole,binormal)+(-0.2-0.16*t)/(1.+t);
                let c=(1.+0.25*t)*(1.+1.4*t)/(1.+t)*dot(pole,initial_tangent)+(0.03+0.05*t)/(1.+t);
                let position=[x,y,if oblique {-x-y}else{0.}];
                let expected:V=std::array::from_fn(|k|position[k]+normal[k]*(a*cos-b*sin)+binormal[k]*(a*sin+b*cos)+c*tangent[k]);
                let retained:V=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]);
                assert!(norm(sub(expected,retained))<=upper,"{rational}/{plane:?}/{i}/{f}/{j}/{upper}");
            }
        }}
        for budget in [0,proof.cells-1]{
            let short=corrected_patch_error_with_length(&sweep,129,budget,1000000,None).unwrap();
            assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
        }
        assert_eq!(path.control_points,original.control_points);assert_eq!(path.weights,original.weights);assert_eq!(path.knots,original.knots);
    }}}
    let straight_start=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.],vec![3.,1.,0.]],None).unwrap();
    let sweep=Sweep::new(&profile,&straight_start,&scale,&twist,Options {normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.1}).unwrap().with_affine_laws(&axis,&center).unwrap();
    let short=corrected_patch_error_with_length(&sweep,129,10000,1000000,None).unwrap();
    assert_eq!(short.status,Status::Certified,"{short:?}");assert!(short.within_budget,"{short:?}");
    let collinear=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.],vec![3.,0.,0.]],None).unwrap();
    let sweep=Sweep::new(&profile,&collinear,&scale,&twist,Options {normal:[0.,0.,1.],orientation:Orientation::CorrectedFrenet,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.1}).unwrap().with_affine_laws(&axis,&center).unwrap();
    let proved=corrected_patch_error_with_length(&sweep,129,10000,1000000,None).unwrap();
    assert_eq!(proved.status,Status::Certified,"{proved:?}");
    assert!(proved.within_budget,"{proved:?}");
}

#[test]
fn collinear_rational_source_bound_covers_independent_parameter_and_length_reference(){
    let profile=crate::primitives::line([0.25,0.125,0.],[0.5,0.25,0.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[1.1,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=constant_vector_law([1.25,0.75,1.]).unwrap();
    let center=constant_vector_law([0.01,-0.02,0.03]).unwrap();
    for rational in [false,true] {for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.0001,max_cells:100000}] {
      for orientation in [Orientation::CorrectedFrenet,Orientation::RotationMinimizing] {
        let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
            control_points:vec![vec![0.;3],vec![0.,0.,1.],vec![0.,0.,7.],vec![0.,0.,10.]],
            weights:if rational {vec![1.,2.,2.,1.]}else{vec![1.;4]},periodic:false};
        let before=path.clone();
        let sweep=Sweep::new(&profile,&path,&scale,&twist,Options {normal:[1.,0.,1.],orientation,spacing,
            initial_sections:3,max_sections:129,max_deviation:0.1}).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=if orientation==Orientation::CorrectedFrenet {corrected_patch_error_with_length(&sweep,129,10000,1000000,None)}
            else {rmf_planar_patch_error_with_length(&sweep,129,10000,1000000,None)}.unwrap();
        assert_eq!(proof.status,Status::Certified,"{rational}/{spacing:?}/{orientation:?}/{proof:?}");
        assert!(proof.within_budget,"{proof:?}");
        let upper=proof.error_upper.unwrap();let retained=sweep.sections_at(129).unwrap();
        for i in 0..128 {for f in [0.125,0.5,0.875] {
            let t=(i as f64+f)/128.;let q=1.-t;
            let z=if spacing==Spacing::Parameter {
                let w=if rational {2.}else{1.};
                (3.*q*q*t*w+21.*q*t*t*w+10.*t*t*t)/(q*q*q+3.*q*q*t*w+3.*q*t*t*w+t*t*t)
            }else{10.*t};
            let (sin,cos)=(0.125*t).sin_cos();
            for (j,pole) in profile.control_points.iter().enumerate(){
                let a=(1.+0.1*t)*1.25*pole[0]+0.01;let b=(1.+0.1*t)*0.75*pole[1]-0.02;
                let expected=[a*cos-b*sin,a*sin+b*cos,z+0.03];
                let actual:V=std::array::from_fn(|k|(1.-f)*retained[i].control_points[j][k]+f*retained[i+1].control_points[j][k]);
                assert!(norm(sub(expected,actual))<=upper,"{rational}/{spacing:?}/{orientation:?}/{i}/{f}/{upper}");
            }
        }}
        assert_eq!(path.control_points,before.control_points);assert_eq!(path.weights,before.weights);assert_eq!(path.knots,before.knots);
      }
    }}
}

#[test]
fn corrected_zero_curvature_leading_phase_covers_rational_original_reference(){
    let scale=crate::primitives::line([1.,0.,0.],[1.1,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=constant_vector_law([1.25,0.75,1.1]).unwrap();let center=constant_vector_law([0.01,-0.02,0.03]).unwrap();
    let profile=crate::primitives::line([0.,0.25,0.125],[0.125,0.5,0.25]).unwrap();
    for degree in [3,4,5] {for rational in [false,true] {for oblique in [false,true] {
        let path=Curve {degree,knots:[vec![2.;degree+1],vec![5.;degree+1]].concat(),
            control_points:(0..=degree).map(|i|vec![i as f64,if i==degree {1.}else{0.},if oblique {-(i as f64)-if i==degree {1.}else{0.}}else{0.}]).collect(),
            weights:(0..=degree).map(|i|if rational&&i>0&&i<degree {2.}else{1.}).collect(),periodic:false};
        let before=path.clone();
        let sweep=Sweep::new(&profile,&path,&scale,&twist,Options {normal:if oblique {[1.;3]}else{[0.,0.,1.]},orientation:Orientation::CorrectedFrenet,
            spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.1}).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=corrected_patch_error_with_length(&sweep,129,10000,1000000,None).unwrap();
        assert_eq!(proof.status,Status::Certified,"{degree}/{rational}/{oblique}/{proof:?}");assert!(proof.within_budget,"{proof:?}");
        let upper=proof.error_upper.unwrap();let retained=sweep.sections_at(129).unwrap();
        let t0=unit([1.,0.,if oblique {-1.}else{0.}]).unwrap();
        let b=unit(if oblique {[1.;3]}else{[0.,0.,1.]}).unwrap();let n0=cross(b,t0);
        for i in 0..128 {for f in [0.125,0.5,0.875] {
            let t=(i as f64+f)/128.;let p=degree as i32;let d=degree as f64;
            let (w,dw,x,dx)=if rational {(2.-(1.-t).powi(p)-t.powi(p),d*((1.-t).powi(p-1)-t.powi(p-1)),d*(2.*t-t.powi(p)),d*(2.-d*t.powi(p-1)))}else{(1.,0.,d*t,d)};
            let y=t.powi(p);let dy=d*t.powi(p-1);let vx=(dx*w-x*dw)/(w*w);let vy=(dy*w-y*dw)/(w*w);
            let tangent=unit([vx,vy,if oblique {-vx-vy}else{0.}]).unwrap();let normal=cross(b,tangent);
            let position=[x/w,y/w,if oblique {-(x+y)/w}else{0.}];let (sin,cos)=(0.125*t).sin_cos();
            for (j,pole) in profile.control_points.iter().enumerate(){
                let pole=[pole[0],pole[1],pole[2]];
                let a=(1.+0.1*t)*1.25*dot(pole,n0)+0.01;let z=(1.+0.1*t)*0.75*dot(pole,b)-0.02;let c=(1.+0.1*t)*1.1*dot(pole,t0)+0.03;
                let expected:V=std::array::from_fn(|k|position[k]+normal[k]*(a*cos-z*sin)+b[k]*(a*sin+z*cos)+tangent[k]*c);
                let actual:V=std::array::from_fn(|k|(1.-f)*retained[i].control_points[j][k]+f*retained[i+1].control_points[j][k]);
                assert!(norm(sub(expected,actual))<=upper,"{degree}/{rational}/{oblique}/{i}/{f}/{j}/{upper}");
            }
        }}
        for budget in [0,proof.cells-1]{let short=corrected_patch_error_with_length(&sweep,129,budget,1000000,None).unwrap();assert!(short.error_upper.is_none());}
        assert_eq!(path.control_points,before.control_points);assert_eq!(path.weights,before.weights);assert_eq!(path.knots,before.knots);
    }}}
}

#[test]
fn corrected_planar_retained_bound_covers_inflection_phase_scale_twist_and_limits(){
    let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,1.,0.],
        vec![2.,-1.,0.],vec![3.,0.,0.]],None).unwrap();
    let profile=crate::primitives::line([-1.,1.,1.],[-2.,2.,2.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[1.25,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let options=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],
        spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
    let proof=corrected_patch_error_with_length(&sweep,129,10000,1000000,None).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    assert!(proof.within_budget,"{proof:?}");
    assert!(proof.original_section_endpoint_error_upper.unwrap()>0.);
    assert!(proof.decomposition_error_upper.unwrap()>=0.);
    let upper=proof.error_upper.unwrap();
    let sections=sweep.sections_at(129).unwrap();
    for i in 0..128 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/128.;let d=1.-6.*t+6.*t*t;let h=(1.+d*d).sqrt();
        let (sin,cos)=(0.125*t).sin_cos();
        for j in 0..2 {
            let p=(j+1) as f64*2_f64.sqrt();let b=(j+1) as f64;
            let n=(1.+0.25*t)*(p*cos-b*sin);
            let z=(1.+0.25*t)*(p*sin+b*cos);
            let expected=[3.*t-d/h*n,3.*t-9.*t*t+6.*t*t*t+n/h,z];
            let retained:V=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]);
            assert!(norm(sub(expected,retained))<=upper,"{i}/{f}/{j}/{upper}");
        }
    }}
    for (cells,products) in [(0,1000000),(proof.cells-1,1000000)] {
        let refused=corrected_patch_error_with_length(&sweep,129,cells,products,None).unwrap();
        assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
    }
    let multispan=Curve{degree:1,knots:std::iter::once(0.).chain((0..=32).map(|i|i as f64))
            .chain(std::iter::once(32.)).collect(),
        control_points:(0..=32).map(|i|vec![-1.-i as f64/32.,1.+i as f64/32.,1.+(i%2) as f64/64.]).collect(),
        weights:(0..=32).map(|i|if i%2==0{1.}else{2.}).collect(),periodic:false};
    let decomposed=Sweep::new(&multispan,&path,&scale,&twist,options).unwrap();
    let full=corrected_patch_error_with_length(&decomposed,33,10000,1000000,None).unwrap();
    assert_eq!(full.status,Status::Certified);assert!(full.products>0);
    for products in [0,full.products-1] {
        let short=corrected_patch_error_with_length(&decomposed,33,10000,products,None).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    }
    let axis=constant_vector_law([1.,2.,1.]).unwrap();let center=constant_vector_law([0.;3]).unwrap();
    let affine=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_affine_laws(&axis,&center).unwrap();
    let affine_proof=corrected_patch_error_with_length(&affine,129,10000,1000000,None).unwrap();
    assert_eq!(affine_proof.status,Status::Certified,"{affine_proof:?}");
    assert!(affine_proof.within_budget,"{affine_proof:?}");
    let mut spatial=path.clone();spatial.control_points[1][2]=f64::from_bits(1);
    let spatial=Sweep::new(&profile,&spatial,&scale,&twist,options).unwrap();
    assert!(corrected_patch_error_with_length(&spatial,3,10000,1000000,None).unwrap().error_upper.is_none());
}

#[test]
fn corrected_original_arc_and_straight_reference_charge_actual_sections_and_shared_work(){
    let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![0.5,0.,0.],vec![1.,1.,0.]],None).unwrap();
    let profile=crate::primitives::line([0.,1.,1.],[0.,2.,2.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[1.25,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let options=Options{orientation:Orientation::CorrectedFrenet,normal:[0.,0.,1.],
        spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
    let proof=corrected_patch_error_with_length(&sweep,17,10000,1000000,None).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");assert!(proof.within_budget,"{proof:?}");
    let upper=proof.error_upper.unwrap();
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=4{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/4.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let (sin,cos)=(0.125*t).sin_cos();let h=(1.+4.*u*u).sqrt();
        let n=(1.+q)*(1.+0.25*t)*(cos-sin);let z=(1.+q)*(1.+0.25*t)*(sin+cos);
        let expected=[u-2.*u*n/h,u*u+n/h,z];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=corrected_patch_error_with_length(&sweep,17,proof.cells-1,1000000,None).unwrap();
    assert!(short.error_upper.is_none());
    let inflection=crate::paths::bezier(vec![vec![0.,0.,0.],vec![1.,1.,0.],vec![2.,-1.,0.],vec![3.,0.,0.]],None).unwrap();
    let crossing=Sweep::new(&profile,&inflection,&scale,&twist,options).unwrap();
    let crossing_proof=corrected_patch_error_with_length(&crossing,17,10000,1000000,None).unwrap();
    assert_eq!(crossing_proof.status,Status::Certified);assert!(crossing_proof.within_budget,"{crossing_proof:?}");
    let mut shifted=inflection.clone();for knot in &mut shifted.knots{*knot=2.+4.**knot;}
    let shifted=Sweep::new(&profile,&shifted,&scale,&twist,options).unwrap();
    let shifted_proof=corrected_patch_error_with_length(&shifted,17,10000,1000000,None).unwrap();
    assert_eq!(shifted_proof.status,Status::Certified);assert!(shifted_proof.within_budget);
    let multi=MultiSweep::new(&[profile.clone(),profile.clone()],&path,&scale,&twist,options).unwrap().preview_at(17).unwrap();
    assert!(multi.report.continuous_bound&&multi.report.accepted);assert!(multi.report.error_certificate_cells<=10000);
    let mut straight=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();straight.weights=vec![1.,2.];
    let ribbon=crate::primitives::line([1.,0.,0.],[2.,0.,1.]).unwrap();
    let axes=crate::primitives::line([1.;3],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    for spacing in [Spacing::Parameter,options.spacing] {
        let opts=Options{normal:[1.,0.,0.],spacing,max_sections:129,max_deviation:0.1,..options};
        let actual=Sweep::new(&ribbon,&straight,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=corrected_patch_error_with_length(&actual,129,10000,1000000,None).unwrap();
        assert_eq!(proof.status,Status::Certified);assert!(proof.within_budget);
        assert!(actual.level(129).unwrap().report.continuous_bound);
    }
    let profiles=vec![profile;64];
    let costly_twist=Curve{degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let partial=MultiSweep::new(&profiles,&path,&scale,&costly_twist,options).unwrap().preview_at(9).unwrap();
    assert!(!partial.report.continuous_bound&&partial.report.continuous_error_upper.is_none());
    assert!(partial.report.error_certificate_cells<=10000);
}

fn guided_section_interpolation_mode(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, mode:OriginalTransport,
) -> Result<SectionInterpolationReport> {
    let contact=mode==OriginalTransport::Contact;
    let corrected=matches!(mode,OriginalTransport::CorrectedStraight|OriginalTransport::CorrectedCollinear|OriginalTransport::CorrectedPlanar);
    let rmf_straight=matches!(mode,OriginalTransport::RmfStraight|OriginalTransport::CorrectedStraight|OriginalTransport::RmfCollinear|OriginalTransport::CorrectedCollinear);
    let rmf_planar=matches!(mode,OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar);
    let collinear=matches!(mode,OriginalTransport::RmfCollinear|OriginalTransport::CorrectedCollinear);
    let fixed=mode==OriginalTransport::Fixed||rmf_straight;
    let fixed_normal=mode==OriginalTransport::FixedNormal||rmf_planar;
    let frenet=mode==OriginalTransport::Frenet;
    check(
        count >= sweep.options.initial_sections && count <= sweep.options.max_sections,
        "Section count outside configured sweep",
    )?;
    check(
        max_cells <= 100000,
        "Guided section work exceeds100000 cells",
    )?;
    let mut out = SectionInterpolationReport {
        status: Status::Unresolved,
        cells: 0,
        error_upper: None,
        endpoint_displacement_upper: None,
        cap_endpoint_displacement_upper: None,
        reason: Some("mode-not-guided"),
    };
    let guide=sweep.orientation_guide;
    if corrected {
        if sweep.options.orientation!=Orientation::CorrectedFrenet||sweep.frame_laws.is_some()||guide.is_some(){return Ok(out);}
        if sweep.path.periodic||(path_is_closed(sweep.path)?
            &&!(mode==OriginalTransport::CorrectedPlanar&&sweep.options.spacing!=Spacing::Parameter
                &&sweep.contact_point.is_none())){
            out.reason=Some("corrected-closed-frame-correspondence-unproved");return Ok(out);
        }
    }
    if fixed {
        out.reason=Some("mode-not-fixed");
        if rmf_straight {
            out.reason=Some("rmf-original-frame-correspondence-unproved");
            if (!corrected&&sweep.options.orientation!=Orientation::RotationMinimizing)||sweep.frame_laws.is_some()||guide.is_some()||(!collinear&&!original_line(sweep.path)){return Ok(out);}
        }else if sweep.options.orientation!=Orientation::Fixed || sweep.frame_laws.is_some() || guide.is_some(){return Ok(out);}
    } else if fixed_normal {
        out.reason=Some("mode-not-fixed-normal");
        if rmf_planar {
            out.reason=Some("rmf-original-frame-correspondence-unproved");
            if (!corrected&&sweep.options.orientation!=Orientation::RotationMinimizing)||sweep.frame_laws.is_some()||guide.is_some(){return Ok(out);}
        } else if sweep.options.orientation!=Orientation::FixedNormal || sweep.frame_laws.is_some() || guide.is_some(){return Ok(out);}
    } else if frenet {
        out.reason=Some("mode-not-frenet");
        if sweep.options.orientation!=Orientation::Frenet || sweep.frame_laws.is_some() || guide.is_some(){return Ok(out);}
    } else if guide.is_none(){return Ok(out);}
    // A positive two-pole rational line has exact normalized arc-length
    // image P0+s(P1-P0), independently of its rational parameter weights.
    // This is the reference field only: sections below still come from the
    // original inverse-length constructor, whose residual/rounding is charged
    // against these reference endpoints. Guides need their own correspondence.
    let arc_line = sweep.options.spacing != Spacing::Parameter
        && (original_line(sweep.path)||collinear)
        && (fixed || mode==OriginalTransport::Guided && guide.is_some_and(original_line));
    if mode==OriginalTransport::Fixed && sweep.contact_point.is_none() && sweep.options.spacing != Spacing::Parameter && !original_line(sweep.path) {
        return original_arc_length_section_interpolation(sweep,count,max_cells,None);
    }
    if mode==OriginalTransport::FixedNormal && sweep.contact_point.is_none() && sweep.options.spacing != Spacing::Parameter {
        return fixed_normal_arc_length_section_interpolation(sweep,count,max_cells,None);
    }
    if rmf_planar&&sweep.contact_point.is_none()&&sweep.options.spacing!=Spacing::Parameter{
        return original_frame_arc_length_section_interpolation(sweep,count,max_cells,None,mode,None);
    }
    if mode==OriginalTransport::Frenet && sweep.contact_point.is_none() && sweep.options.spacing != Spacing::Parameter {
        return original_frame_arc_length_section_interpolation(sweep,count,max_cells,None,OriginalTransport::Frenet,None);
    }
    if mode==OriginalTransport::Guided && sweep.contact_point.is_none() && sweep.options.spacing != Spacing::Parameter && !arc_line {
        return original_frame_arc_length_section_interpolation(sweep,count,max_cells,None,OriginalTransport::Guided,None);
    }
    if mode==OriginalTransport::Contact && sweep.contact_source.is_some() && sweep.options.spacing!=Spacing::Parameter {
        return original_frame_arc_length_section_interpolation(sweep,count,max_cells,None,OriginalTransport::Contact,None);
    }
    if sweep.options.spacing != Spacing::Parameter && !arc_line {
        out.reason = Some("arc-length-correspondence-unproved");
        return Ok(out);
    }
    if rmf_planar&&!original_planar_rmf(sweep.path,sweep.options.normal){
        // Closed nonaxial transport still requires a separate holonomy and
        // actual seam identity proof. This premise admits only open sources.
        if sweep.path.periodic||path_is_closed(sweep.path)?{return Ok(out);}
        let plane=super::source_plane::certify(sweep.path,sweep.options.normal,(max_cells-out.cells) as u64)?;
        out.cells+=plane.exact_work as usize;
        if !plane.proved{out.reason=plane.reason;return Ok(out);}
    }
    let principal_phase=if mode==OriginalTransport::CorrectedPlanar&&sweep.affine_laws.is_some(){
        let (cells,phase)=authored_frame_certificate::certify_initial_planar_principal_phase(sweep.path,sweep.options.normal,max_cells-out.cells)?;
        out.cells+=cells;
        let Some(phase)=phase else{out.reason=Some("corrected-affine-frame-phase-unproved");return Ok(out);};
        Some(phase)
    }else{None};
    let length_reference = (arc_line||collinear).then(|| Curve {
        degree: 1, knots: vec![0.,0.,1.,1.],
        control_points: vec![sweep.path.control_points[0].clone(),sweep.path.control_points.last().unwrap().clone()],
        weights: vec![1.,1.], periodic: false,
    });
    let reference_path = if arc_line {length_reference.as_ref().unwrap()}else{sweep.path};
    // The private collinear mode owns a whole-source line/strict-speed proof.
    // Only its constant tangent uses the endpoint line. Parameter translation
    // continues to use the original rational curve and original domain.
    let frame_reference_path = if collinear {length_reference.as_ref().unwrap()}else{reference_path};
    // The constructor divides the guide by its own length, not by the path's
    // rational parameter. Each original line therefore needs its own exact
    // normalized-length image; actual guide station residuals stay charged in
    // retained section endpoint displacement below.
    let guide_length_reference = if arc_line {guide.map(|g| Curve {
        degree:1,knots:vec![0.,0.,1.,1.],control_points:g.control_points.clone(),
        weights:vec![1.,1.],periodic:false,
    })} else {None};
    let reference_guide = guide_length_reference.as_ref().or(guide);
    // A guide supplies every normal directly: sections apply no RMF closure
    // correction. Endpoint displacement below also encloses the copied seam.
    if sweep.contact_point.is_some() && !contact {
        out.reason = Some("contact-width-law-unproved");
        return Ok(out);
    }
    let anchor = if contact {
        if sweep.contact_source.is_none() {out.reason=Some("mode-not-contact");return Ok(out);}
        let report=sweep.contact_anchor_bound(max_cells)?;
        out.cells+=report.cells;
        if report.status!=Status::Certified {out.reason=report.reason;return Ok(out);}
        Some(report.coordinates.unwrap()[0])
    } else {None};
    let constant_law=|law:&Curve|law.control_points.iter().all(|p|p==&law.control_points[0]);
    let constant_pose=collinear&&constant_law(sweep.scale)&&constant_law(sweep.twist)
        &&sweep.affine_laws.is_none_or(|(a,c)|constant_law(a)&&constant_law(c));
    let pose_cache=std::cell::RefCell::new(None::<Vec<[[f64;2];3]>>);
    let zero_path=constant_vector_law([0.;3])?;
    let certify_values=|qs:&[[[f64;2];3]], interval, budget| {
        if constant_pose {
            let mut charge=0;
            if pose_cache.borrow().is_none() {
                let pose=authored_frame_certificate::certify_fixed_path_control_values_with_frame_path(&zero_path,frame_reference_path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,[0.,1.],budget)?;
                if pose.status!=Status::Certified {return Ok(pose);}
                charge=pose.cells;*pose_cache.borrow_mut()=pose.values;
            }
            let mut values=authored_frame_certificate::translated_constant_values(reference_path,pose_cache.borrow().as_ref().unwrap(),interval,budget-charge)?;
            values.cells+=charge;return Ok(values);
        }
        if fixed {
            authored_frame_certificate::certify_fixed_path_control_values_with_frame_path(reference_path,frame_reference_path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if let Some(phase)=principal_phase {
            authored_frame_certificate::certify_planar_principal_control_values(sweep.path,sweep.options.normal,phase,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if rmf_planar {
            authored_frame_certificate::certify_proved_planar_control_values(sweep.path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if fixed_normal {
            authored_frame_certificate::certify_fixed_normal_control_values(sweep.path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if frenet {
            authored_frame_certificate::certify_frenet_control_values(sweep.path,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if let Some(anchor)=anchor {
            authored_frame_certificate::certify_contact_control_values(sweep.path,guide.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,qs,anchor,interval,budget)
        } else {
            authored_frame_certificate::certify_path_guide_control_values(reference_path,reference_guide.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        }
    };
    let certify_jets=|qs:&[[[f64;2];3]], interval, budget| {
        if constant_pose {
            if let Some(pose)=pose_cache.borrow().as_ref() {
                return authored_frame_certificate::translated_constant_jets(reference_path,pose,interval,budget);
            }
        }
        if fixed {
            authored_frame_certificate::certify_fixed_path_control_trajectories_with_frame_path(reference_path,frame_reference_path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if let Some(phase)=principal_phase {
            authored_frame_certificate::certify_planar_principal_control_trajectories(sweep.path,sweep.options.normal,phase,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if fixed_normal {
            authored_frame_certificate::certify_fixed_normal_control_trajectories(sweep.path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if frenet {
            authored_frame_certificate::certify_frenet_control_trajectories(sweep.path,sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        } else if let Some(anchor)=anchor {
            authored_frame_certificate::certify_contact_control_trajectories(sweep.path,guide.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,qs,anchor,interval,budget)
        } else {
            authored_frame_certificate::certify_path_guide_control_trajectories(reference_path,reference_guide.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,qs,interval,budget)
        }
    };
    let initial = if let Some(phase)=principal_phase {principal_initial_coordinates(sweep,phase,max_cells-out.cells)?} else if corrected {seed_initial_coordinates_as(sweep,max_cells-out.cells,Orientation::RotationMinimizing,Orientation::CorrectedFrenet)?} else if rmf_straight||rmf_planar {seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::RotationMinimizing)?} else if fixed {fixed_initial_coordinates(sweep,max_cells-out.cells)?} else if fixed_normal {fixed_normal_initial_coordinates(sweep,max_cells-out.cells)?} else if frenet {seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::Frenet)?} else {guided_initial_coordinates(sweep, max_cells-out.cells)?};
    out.cells += initial.cells;
    if initial.status != Status::Certified {
        out.reason = initial.reason;
        return Ok(out);
    }
    let coordinates = initial.coordinates.unwrap();
    let (sections,closed,_,closed_identity)=sweep.sections_with_frame_identity(count)?;
    if rmf_planar && closed && !closed_identity {
        out.reason=Some("rmf-closed-correction-correspondence-unproved");
        return Ok(out);
    }
    check(
        sections.iter().all(|c| {
            c.degree == sweep.profile.degree
                && c.knots == sweep.profile.knots
                && c.weights == sweep.profile.weights
                && c.periodic == sweep.profile.periodic
                && c.control_points.len() == coordinates.len()
        }),
        "Retained guided basis correspondence changed",
    )?;
    let mut endpoint_errors = vec![vec![0.; coordinates.len()]; count];
    let mut endpoint_upper = 0_f64;
    for (station, section) in sections.iter().enumerate() {
        let t = station as f64 / (count - 1) as f64;
        let batch=certify_values(&coordinates,[t,t],max_cells-out.cells)?;
        out.cells+=batch.cells;
        let Some(values)=batch.values.filter(|_|batch.status==Status::Certified) else {
            out.reason=batch.reason;
            return Ok(out);
        };
        for (control, value) in values.into_iter().enumerate() {
            let value=authored_frame_certificate::ControlValueReport {status:Status::Certified,cells:0,value:Some(value),reason:None};
            let p = &section.control_points[control];
            let Some(upper) = value.retained_displacement_upper([p[0], p[1], p[2]])? else {
                out.reason = value.reason;
                return Ok(out);
            };
            endpoint_errors[station][control] = upper;
            endpoint_upper = endpoint_upper.max(upper);
        }
    }
    let mut error = 0_f64;
    for station in 0..count - 1 {
        let interval = [
            station as f64 / (count - 1) as f64,
            (station + 1) as f64 / (count - 1) as f64,
        ];
        // A complete original value image can already bound the retained
        // segment. Prefer that sufficient cover before spending derivative
        // work on a closed contact field; no G1/G2 claim follows from it.
        let mut preferred_values=None;
        if contact&&closed {
            let values=certify_values(&coordinates,interval,max_cells-out.cells)?;
            out.cells+=values.cells;
            if values.status==Status::Certified {
                let mut upper=0_f64;
                let mut complete=true;
                for control in 0..coordinates.len() {
                    let value=authored_frame_certificate::ControlValueReport {status:values.status,cells:0,
                        value:values.values.as_ref().map(|v|v[control]),reason:values.reason};
                    let a=&sections[station].control_points[control];
                    let b=&sections[station+1].control_points[control];
                    if let Some(bound)=value.retained_segment_displacement_upper([[a[0],a[1],a[2]],[b[0],b[1],b[2]]])? {
                        upper=upper.max(bound);
                    }else{complete=false;}
                }
                if complete&&upper<=sweep.options.max_deviation {error=error.max(upper);continue;}
            }
            preferred_values=Some(values);
        }
        let batch=certify_jets(&coordinates,interval,max_cells-out.cells)?;
        out.cells+=batch.cells;
        let mut fallback=if batch.status!=Status::Certified || batch.jets.is_none() {
            let values=if let Some(prepared)=preferred_values.take(){prepared}else{
                let values=certify_values(&coordinates,interval,max_cells-out.cells)?;
                out.cells+=values.cells;values
            };
            if rmf_planar && values.status!=Status::Certified {
                out.reason=values.reason;return Ok(out);
            }
            Some(values)
        } else {preferred_values.take()};
        for control in 0..coordinates.len() {
            let jet=authored_frame_certificate::TrajectoryReport {traversal:interval,status:batch.status,cells:0,
                jet:batch.jets.as_ref().map(|v|v[control].clone()),single_span:batch.single_span,reason:batch.reason};
            let a = &sections[station].control_points[control];
            let b = &sections[station + 1].control_points[control];
            let image = jet.retained_segment_displacement_upper([
                [a[0],a[1],a[2]], [b[0],b[1],b[2]],
            ])?;
            let mut remainder = if fixed && !arc_line && batch.status==Status::Certified && fixed_affine_interval(sweep,count,interval) {
                Some(endpoint_errors[station][control].max(endpoint_errors[station+1][control]))
            } else {jet.linear_error_upper([
                endpoint_errors[station][control],
                endpoint_errors[station + 1][control],
            ])?};
            if original_laws_have_no_interior_knots(sweep,interval) {
                // All original time laws are rational-polynomial on the
                // open interval; certified regular frame jets enclose Q''.
                // Original endpoint values already enclose both one-sided
                // limits, including any retained copied seam displacement.
                if let Some(second)=jet.linear_second_error_upper_on_open_span(
                    [endpoint_errors[station][control],endpoint_errors[station+1][control]],
                )? {remainder=Some(remainder.map_or(second,|old|old.min(second)));}
                if let Some(first)=jet.linear_first_error_upper_on_continuous_span(
                    [[a[0],a[1],a[2]],[b[0],b[1],b[2]]],
                    [endpoint_errors[station][control],endpoint_errors[station+1][control]],
                )? {remainder=Some(remainder.map_or(first,|second|second.min(first)));}
            }
            if rmf_planar || contact&&closed {
                let jet_upper=match (image,remainder) {
                    (Some(a),Some(b))=>Some(a.min(b)),(a,b)=>a.or(b),
                };
                // Spend extra value-cover work only where the existing
                // complete jet bound cannot fit the original source budget.
                // Successful tight jets retain their original shared work.
                if jet_upper.is_none_or(|bound|bound>sweep.options.max_deviation) && fallback.is_none() {
                    let values=certify_values(&coordinates,interval,max_cells-out.cells)?;
                    out.cells+=values.cells;
                    if values.status!=Status::Certified {out.reason=values.reason;return Ok(out);}
                    fallback=Some(values);
                }
                // The original-plane value cover preserves the constant B
                // direction even when independent frame jet boxes are wide.
                // Both covers belong to this same source/retained family;
                // their minimum is a continuous bound, never a sampled fit.
                if let Some(values)=fallback.as_ref() {
                    let value=authored_frame_certificate::ControlValueReport {status:values.status,cells:0,
                        value:values.values.as_ref().map(|v|v[control]),reason:values.reason};
                    if let Some(bound)=value.retained_segment_displacement_upper([[a[0],a[1],a[2]],[b[0],b[1],b[2]]])? {
                        remainder=Some(remainder.map_or(bound,|old|old.min(bound)));
                    }
                }
            }
            // Both bounds enclose the same original/retained pointwise error;
            // their minimum is valid, with no extra certificate work.
            let upper = if let Some(upper) = match (image,remainder) {
                (Some(a),Some(b)) => Some(a.min(b)),
                (a,b) => a.or(b),
            } {
                upper
            } else {
                let value=authored_frame_certificate::ControlValueReport {status:fallback.as_ref().map_or(Status::Unresolved,|r|r.status),cells:0,
                    value:fallback.as_ref().and_then(|r|r.values.as_ref()).map(|v|v[control]),reason:fallback.as_ref().and_then(|r|r.reason).or(batch.reason)};
                let a = &sections[station].control_points[control];
                let b = &sections[station + 1].control_points[control];
                let Some(upper) = value.retained_segment_displacement_upper([
                    [a[0], a[1], a[2]],
                    [b[0], b[1], b[2]],
                ])?
                else {
                    out.reason = value.reason;
                    return Ok(out);
                };
                upper
            };
            error = error.max(upper);
        }
    }
    out.endpoint_displacement_upper = Some(endpoint_upper);
    out.status = Status::Certified;
    out.error_upper = Some(error);
    out.reason = None;
    Ok(out)
}

#[test]
fn frenet_retained_bound_covers_original_curvature_scale_and_twist(){
    let profile=crate::primitives::line([0.,1.,1.],[0.,2.,1.]).unwrap();
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:257,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let coarse=sweep.level(3).unwrap();assert!(!coarse.report.accepted);
    let fine=sweep.level(257).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    let upper=fine.report.continuous_error_upper.unwrap();
    let sections=sweep.sections(257).unwrap().0;
    for i in 0..256 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/256.;let h=(1.+4.*t*t).sqrt();let angle=0.25*t;
        for j in 0..2 {
            let x=(j+1) as f64*(1.+t);let y=1.+t;
            let n=x*angle.cos()-y*angle.sin();
            let expected=[t-2.*t/h*n,t*t+n/h,x*angle.sin()+y*angle.cos()];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.frenet_patch_error_bound(257,10000,1000000).unwrap();
    let short=sweep.frenet_patch_error_bound(257,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let arc=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..opts}).unwrap();
    let arc_proof=arc.frenet_patch_error_bound(257,10000,1000000).unwrap();
    assert_eq!(arc_proof.status,Status::Certified,"{arc_proof:?}");
    assert!(arc_proof.error_upper.unwrap().is_finite() && arc_proof.cells<=10000);
    let arc_short=arc.frenet_patch_error_bound(257,1,1000000).unwrap();
    assert_eq!(arc_short.status,Status::Unresolved);assert!(arc_short.error_upper.is_none());
}

#[test]
fn spatial_frenet_retained_bound_encloses_original_cubic_scale_twist(){
    let profile=crate::primitives::line([0.,1.,1.],[0.,2.,1.]).unwrap();
    let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![1./3.,0.,0.],vec![2./3.,1./3.,0.],vec![1.,1.,1.]],weights:vec![1.;4],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:129,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    assert!(!sweep.level(3).unwrap().report.accepted);
    let fine=sweep.level(129).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    assert!(fine.report.error_certificate_cells<=10000);
    let upper=fine.report.continuous_error_upper.unwrap();
    let sections=sweep.sections(129).unwrap().0;
    for i in 0..128 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/128.;
        let gt=(1.+4.*t*t+9.*t.powi(4)).sqrt();let gb=(1.+9.*t*t+9.*t.powi(4)).sqrt();
        let tangent=[1./gt,2.*t/gt,3.*t*t/gt];let binormal=[3.*t*t/gb,-3.*t/gb,1./gb];
        let normal=cross(binormal,tangent);let a=0.25*t;
        for j in 0..2 {
            let n=(1.+t)*((j+1) as f64*a.cos()-a.sin());
            let b=(1.+t)*((j+1) as f64*a.sin()+a.cos());
            let c=[t,t*t,t*t*t];let expected:V=std::array::from_fn(|k|c[k]+n*normal[k]+b*binormal[k]);
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.frenet_patch_error_bound(129,10000,1000000).unwrap();
    let short=sweep.frenet_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn guided_section_value_bound_owns_retained_segments_and_refines() {
    let profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.; 3]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 9,
        max_deviation: 100.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let coarse = sweep.guided_section_interpolation_bound(3, 10000).unwrap();
    let fine = sweep.guided_section_interpolation_bound(9, 10000).unwrap();
    assert_eq!(coarse.status, Status::Certified);
    assert_eq!(fine.status, Status::Certified);
    assert!(coarse.error_upper.unwrap() < 1e-9);
    assert!(fine.error_upper.unwrap() < 1e-9);
    assert!(coarse.endpoint_displacement_upper.unwrap() < 1e-10);
    let refused = sweep
        .guided_section_interpolation_bound(3, coarse.cells - 1)
        .unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.error_upper.is_none());
}

#[test]
fn contact_retained_section_bound_includes_station_rounding_and_joint_fit_remainder() {
    let mut profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    profile.weights=vec![1.,2.];
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let guide=crate::primitives::line([2.,0.,0.],[2.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let axes=crate::primitives::line([1.;3],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.5,0.,0.]).unwrap();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.01};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_contact_guide(&guide,1.).unwrap().with_affine_laws(&axes,&center).unwrap();
    let coarse=sweep.contact_section_interpolation_bound(3,10000).unwrap();
    let fine=sweep.contact_section_interpolation_bound(33,10000).unwrap();
    assert_eq!(coarse.status,Status::Certified);
    assert_eq!(fine.status,Status::Certified);
    assert!(fine.error_upper.unwrap()<coarse.error_upper.unwrap());
    assert!(fine.error_upper.unwrap()<0.01,"{:?}",fine.error_upper);
    assert!(fine.endpoint_displacement_upper.unwrap()<1e-9);
    let patch=sweep.contact_patch_error_bound(33,10000,1000).unwrap();
    assert_eq!(patch.status,Status::Certified);
    assert!(patch.within_budget);
    let level=sweep.level(33).unwrap();
    assert!(level.report.accepted && level.report.continuous_bound);
    assert_eq!(level.report.continuous_error_upper,patch.error_upper);
    assert_eq!(level.report.known_profile_error_upper,patch.error_upper);
    let coarse_level=sweep.level(3).unwrap();
    assert!(coarse_level.report.continuous_bound);
    assert!(!coarse_level.report.accepted);
    let retained=sweep.sections(3).unwrap().0;
    for t in [0.0_f64,0.13,0.375,0.5,0.87,1.] {
        let station=if t<0.5 {0} else {1};
        let f=2.*t-station as f64;
        for control in 0..2 {
            let q=1.+control as f64;
            let ideal=[2.*(q*(1.+t).powi(2)+0.5*t)/(2.*(1.+t).powi(2)+0.5*t),0.,10.*t];
            let delta: [f64;3]=std::array::from_fn(|k|(1.-f)*retained[station].control_points[control][k]+f*retained[station+1].control_points[control][k]-ideal[k]);
            assert!(norm(delta)<=coarse.error_upper.unwrap());
        }
    }
    let short=sweep.contact_section_interpolation_bound(33,fine.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.error_upper.is_none() && short.endpoint_displacement_upper.is_none());
}

#[test]
fn guided_dense_profile_patch_bound_composes_original_decomposition() {
    let profile = Curve {
        degree: 1,
        knots: std::iter::once(0.)
            .chain((0..=32).map(|i| i as f64))
            .chain(std::iter::once(32.))
            .collect(),
        control_points: (0..=32)
            .map(|i| vec![1. + i as f64 / 32., (i % 2) as f64 / 64., 0.])
            .collect(),
        weights: (0..=32).map(|i| if i % 2 == 0 { 1. } else { 2. }).collect(),
        periodic: false,
    };
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = constant_vector_law([0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 100.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    let report = sweep.guided_patch_error_bound(3, 10000, 1000).unwrap();
    assert_eq!(report.status, Status::Certified);
    assert_eq!(report.products, 384);
    assert_eq!(report.patches.as_ref().unwrap().len(), 32);
    assert!(report.decomposition_error_upper.unwrap() > 0.);
    assert!(report.error_upper.unwrap() >= report.section_error_upper.unwrap());
    assert!(report.within_budget);
    let refused = sweep.guided_patch_error_bound(3, 10000, 383).unwrap();
    assert_eq!(refused.status, Status::Unresolved);
    assert!(refused.error_upper.is_none() && refused.patches.is_none());
    assert!(refused.products <= 383);
    let zero_twist = constant_vector_law([0.; 3]).unwrap();
    let contact = Sweep::new(&profile, &path, &scale, &zero_twist, options)
        .unwrap().with_contact_guide(&guide, 0.).unwrap();
    let fitted = contact.contact_patch_error_bound(3, 10000, 1000).unwrap();
    assert_eq!(fitted.status, Status::Certified);
    assert_eq!(fitted.products, 384);
    assert_eq!(fitted.patches.as_ref().unwrap().len(), 32);
    assert!(fitted.decomposition_error_upper.unwrap() > 0.);
    assert!(fitted.error_upper.unwrap() >= fitted.section_error_upper.unwrap());
    assert!(fitted.within_budget);
    let limited = contact.contact_patch_error_bound(3, 10000, 383).unwrap();
    assert_eq!(limited.status, Status::Unresolved);
    assert!(limited.patches.is_none() && limited.error_upper.is_none());
    assert!(limited.products <= 383);
}

#[test]
fn guided_original_rational_stations_have_point_safe_control_enclosures() {
    let profile = crate::primitives::line([1., 2., 3.], [2., 2., 3.]).unwrap();
    let mut path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    path.weights = vec![1., 2.];
    path.knots = vec![2., 2., 5., 5.];
    let guide = crate::primitives::line([1., 0., 0.], [1., 0., 10.]).unwrap();
    let scale = constant_vector_law([1., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 3,
        max_deviation: 1.,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap();
    for t in [0.13_f64, 0.375, 0.5, 0.87] {
        let report = sweep.guided_control_value(0, [t, t], 1000).unwrap();
        assert_eq!(report.status, Status::Certified);
        let expected = [
            (0.25 * t).cos() - 2. * (0.25 * t).sin(),
            (0.25 * t).sin() + 2. * (0.25 * t).cos(),
            3. + 20. * t / (1. + t),
        ];
        let value = report.value.unwrap();
        for k in 0..3 {
            assert!(value[k][0] <= expected[k] && expected[k] <= value[k][1]);
            assert!(value[k][1] - value[k][0] < 1e-9);
        }
        let refused = sweep
            .guided_control_value(0, [t, t], report.cells - 1)
            .unwrap();
        assert_eq!(refused.status, Status::Unresolved);
        assert!(refused.value.is_none());
    }
}

#[test]
fn guided_tight_joint_law_patch_error_refines_to_small_tolerance() {
    let mut profile = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    profile.weights = vec![1., 2.];
    let path = crate::primitives::line([0.; 3], [0., 0., 10.]).unwrap();
    let guide = crate::primitives::line([1., 0., 0.], [1., 1., 10.]).unwrap();
    let scale = crate::primitives::line([1., 0., 0.], [2., 0., 0.]).unwrap();
    let twist = crate::primitives::line([0.; 3], [0.25, 0., 0.]).unwrap();
    let axes = crate::primitives::line([1.; 3], [2., 1., 1.]).unwrap();
    let center = crate::primitives::line([0.; 3], [0.5, 0., 0.]).unwrap();
    let options = super::Options {
        normal: [1., 0., 0.],
        orientation: super::Orientation::RotationMinimizing,
        spacing: Spacing::Parameter,
        initial_sections: 3,
        max_sections: 33,
        max_deviation: 0.01,
    };
    let sweep = Sweep::new(&profile, &path, &scale, &twist, options)
        .unwrap()
        .with_orientation_guide(&guide)
        .unwrap()
        .with_affine_laws(&axes, &center)
        .unwrap();
    let coarse = sweep.guided_patch_error_bound(3, 10000, 1000).unwrap();
    let fine = sweep.guided_patch_error_bound(33, 10000, 1000).unwrap();
    assert_eq!(coarse.status, Status::Certified);
    assert_eq!(fine.status, Status::Certified);
    assert!(fine.error_upper.unwrap() < coarse.error_upper.unwrap());
    assert!(fine.within_budget, "fine bound {:?}", fine.error_upper);
    let coarse_level = sweep.level(3).unwrap();
    assert!(coarse_level.report.continuous_bound);
    assert_eq!(coarse_level.report.continuous_error_upper, coarse.error_upper);
    assert!(!coarse_level.report.accepted);
    let fine_level = sweep.level(33).unwrap();
    assert!(fine_level.report.accepted);
    assert!(fine_level.report.continuous_bound);
    assert_eq!(fine_level.report.continuous_error_upper, fine.error_upper);
    assert_eq!(fine_level.report.known_profile_error_upper, fine.error_upper);
    assert_eq!(fine_level.report.error_certificate_cells, fine.cells);
    assert_eq!(fine_level.report.decomposition_products, fine.products);
    let exhausted = sweep.level_with_error_budget(33, 0, 1000).unwrap();
    assert!(!exhausted.report.continuous_bound);
    assert!(exhausted.report.continuous_error_upper.is_none());
    let mut refining = sweep;
    let mut levels = Vec::new();
    while let Some(level) = refining.next() {
        levels.push(level.unwrap().report);
    }
    assert!(levels.len() > 1);
    assert!(levels.last().unwrap().accepted);
    assert!(levels.iter().all(|report| report.continuous_bound));
    let patch = &coarse.patches.as_ref().unwrap()[0];
    for t in [0.0_f64, 0.13, 0.375, 0.5, 0.87, 1.] {
        for u in [0., 0.375, 1.] {
            let q = (1. + 3. * u) / (1. + u);
            let amplitude = q * (1. + t).powi(2) + 0.5 * t;
            let angle = t.atan() + 0.25 * t;
            let ideal = [amplitude * angle.cos(), amplitude * angle.sin(), 10. * t];
            let got = patch.evaluate(u, t).unwrap().point;
            let delta: [f64; 3] = std::array::from_fn(|k| got[k] - ideal[k]);
            assert!(norm(delta) <= coarse.error_upper.unwrap());
        }
    }
}


#[test]
fn closed_guided_and_contact_retained_bounds_include_copied_seam() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let guide=crate::primitives::circle([0.,0.,1.],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,1.],[5.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:5,max_sections:129,max_deviation:100.};
    for contact in [false,true] {
        let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
        let sweep=if contact {sweep.with_contact_guide(&guide,0.).unwrap()} else {sweep.with_orientation_guide(&guide).unwrap()};
        let sections=sweep.sections(33).unwrap().0;
        assert_eq!(sections.first().unwrap().control_points,sections.last().unwrap().control_points);
        let bound=if contact {sweep.contact_patch_error_bound(33,100000,1000)} else {sweep.guided_patch_error_bound(33,100000,1000)}.unwrap();
        assert_eq!(bound.status,Status::Certified,"contact={contact}: {bound:?}");
        assert!(bound.error_upper.unwrap().is_finite());
        assert!(bound.error_upper.unwrap()<100.,"{bound:?}");
        let fine=if contact {sweep.contact_patch_error_bound(129,100000,1000)} else {sweep.guided_patch_error_bound(129,100000,1000)}.unwrap();
        assert_eq!(fine.status,Status::Certified);
        assert!(fine.error_upper.unwrap()<bound.error_upper.unwrap());
        assert!(fine.error_upper.unwrap()<2.,"Both fine original-law bounds must fit2mm");
        let short_union=if contact {sweep.contact_patch_error_bound(129,10000,1000)} else {sweep.guided_patch_error_bound(129,10000,1000)}.unwrap();
        assert!(fine.cells<=10000,"Both fine certificates must fit the public work budget");
        assert_eq!(short_union.status,Status::Certified);
        assert_eq!(short_union.error_upper,fine.error_upper);
        let fine_short=if contact {sweep.contact_patch_error_bound(129,fine.cells-1,1000)} else {sweep.guided_patch_error_bound(129,fine.cells-1,1000)}.unwrap();
        assert_eq!(fine_short.status,Status::Unresolved);
        assert!(fine_short.error_upper.is_none());
        let tight_options=Options {max_deviation:2.,..opts};
        let tight=Sweep::new(&profile,&path,&scale,&twist,tight_options).unwrap();
        let tight=if contact {tight.with_contact_guide(&guide,0.).unwrap()} else {tight.with_orientation_guide(&guide).unwrap()};
        assert!(!tight.level(33).unwrap().report.accepted);
        let intermediate=tight.level(65).unwrap();
        assert!(intermediate.report.continuous_bound);
        let intermediate_upper=intermediate.report.continuous_error_upper.unwrap();
        assert_eq!(intermediate.report.accepted,intermediate_upper<=2.);
        // A tighter certificate may admit this level earlier. Check the
        // original rational circle independently, including the copied seam.
        let intermediate_sections=tight.sections(65).unwrap().0;
        for i in 0..64 { for fraction in [0.125,0.5,0.875] {
            let t=(i as f64+fraction)/64.;
            let original=path.evaluate(t).unwrap().point;
            for control in 0..2 {
                let expected=[original[0],original[1],1.+control as f64];
                let delta=std::array::from_fn(|k|
                    (1.-fraction)*intermediate_sections[i].control_points[control][k]
                    +fraction*intermediate_sections[i+1].control_points[control][k]-expected[k]);
                assert!(norm(delta)<=intermediate_upper,"contact={contact}, t={t}");
            }
        }}
        let tight_level=tight.level(129).unwrap();
        assert!(tight_level.report.accepted);
        assert!(tight_level.report.continuous_bound);
        assert!(sweep.level(33).unwrap().report.accepted);
        for i in 0..32 {
            for fraction in [0.125,0.5,0.875] {
                let t=(i as f64+fraction)/32.;
                let p=path.evaluate(path.domain()[0]+t*(path.domain()[1]-path.domain()[0])).unwrap().point;
                for control in 0..2 {
                    let expected=[p[0],p[1],1.+control as f64];
                    let delta=std::array::from_fn(|k|(1.-fraction)*sections[i].control_points[control][k]+fraction*sections[i+1].control_points[control][k]-expected[k]);
                    assert!(norm(delta)<=bound.error_upper.unwrap());
                }
            }
        }

        let section=if contact {sweep.contact_section_interpolation_bound(33,100000)} else {sweep.guided_section_interpolation_bound(33,100000)}.unwrap();
        assert!(section.endpoint_displacement_upper.unwrap()<1e-8);
        let short=if contact {sweep.contact_patch_error_bound(33,bound.cells-1,1000)} else {sweep.guided_patch_error_bound(33,bound.cells-1,1000)}.unwrap();
        assert_eq!(short.status,Status::Unresolved);
        assert!(short.error_upper.is_none());
    }
}


#[cfg(feature="transport")]
#[test]
fn closed_guided_contact_transport_preserves_tight_budget_and_refinement() {
    use value_codec::json;
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let guide=crate::primitives::circle([0.,0.,1.],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,1.],[5.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    for contact in [false,true] {
        for count in [33,65,129] {
            let mut request=json!({"op":"surface_progressive_sweep_level","profiles":[profile],
                "path":path,"scale":scale,"twist":twist,"orientation_guide":guide,
                "normal":[0.,0.,1.],"orientation":"rmf","spacing":"parameter",
                "initial_sections":33,"max_sections":129,"max_deviation":2.,"preview_sections":count});
            if contact {request["contact_parameter"]=json!(0.);request["contact_profile"]=json!(0);}
            let result=crate::transport::dispatch(request).unwrap();
            let report=&result["report"];
            assert_eq!(report["continuousBound"],true,"contact={contact}, count={count}: {report:?}");
            let upper=report["continuousErrorUpper"].as_f64().unwrap();
            assert_eq!(report["accepted"],upper<=2.);
            if count==33 {assert!(upper>2.);}
            if count==129 {assert!(upper<=2.);}
            assert_eq!(report["budget"],2.);
            assert!(report["errorCertificateCells"].as_u64().unwrap()<=10000);
            assert_eq!(report["continuousErrorScope"],"retained-patches-relative-to-original-profile-transport");

        }
    }
}


#[test]
fn fixed_retained_bound_owns_coordinates_rounding_and_original_twist(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[std::f64::consts::FRAC_PI_2,0.,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:0.05};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    assert!(!sweep.level(3).unwrap().report.accepted);
    let fine=sweep.level(33).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    let upper=fine.report.continuous_error_upper.unwrap();assert!(upper<=opts.max_deviation);
    let sections=sweep.sections(33).unwrap().0;
    let endpoint_upper=fine.report.original_section_endpoint_error_upper.unwrap();
    for (station,t) in [(0,0_f64),(32,1_f64)] {
        let (sin,cos)=(t*std::f64::consts::FRAC_PI_2).sin_cos();
        for control in 0..2 {
            let radius=(control+1) as f64*(1.+t);
            let expected=[radius*cos,radius*sin,10.*t];
            assert!(norm(std::array::from_fn(|k|sections[station].control_points[control][k]-expected[k]))<=endpoint_upper);
        }
    }
    for i in 0..32 {for fraction in [0.125,0.5,0.875] {
        let t=(i as f64+fraction)/32.;let angle=t*std::f64::consts::FRAC_PI_2;
        for control in 0..2 {
            let r=(control+1) as f64;
            let expected=[r*(1.+t)*angle.cos(),r*(1.+t)*angle.sin(),10.*t];
            let delta=std::array::from_fn(|k|(1.-fraction)*sections[i].control_points[control][k]+fraction*sections[i+1].control_points[control][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.fixed_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);
    let short=sweep.fixed_patch_error_bound(33,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    #[cfg(feature="transport")]
    {
        use value_codec::json;
        let request=json!({"op":"surface_progressive_sweep_level","profiles":[profile],"path":path,
            "scale":scale,"twist":twist,"normal":[1.,0.,0.],"orientation":"fixed","spacing":"parameter",
            "initial_sections":3,"max_sections":33,"max_deviation":0.05,"preview_sections":33});
        let result=crate::transport::dispatch(request).unwrap();
        assert_eq!(result["report"]["continuousBound"],true);
        assert_eq!(result["report"]["accepted"],true);
        assert_eq!(result["report"]["continuousErrorUpper"],upper);
        assert_eq!(result["report"]["originalSectionEndpointErrorUpper"],endpoint_upper);
        let retained_endpoints=result["report"]["endpointContourErrorUpper"].as_array().unwrap();
        assert_eq!(retained_endpoints.len(),2);
        assert!(retained_endpoints.iter().all(|value|value.as_f64().unwrap()>=endpoint_upper));
        assert_eq!(result["report"]["continuousErrorScope"],"retained-patches-relative-to-original-profile-transport");
    }
}


#[test]
fn closed_fixed_rational_path_bound_includes_copied_endpoint(){
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,1.],[5.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,initial_sections:33,max_sections:129,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let coarse=sweep.fixed_patch_error_bound(33,10000,1000000).unwrap();
    let fine=sweep.fixed_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(coarse.status,Status::Certified);assert_eq!(fine.status,Status::Certified);
    assert!(fine.error_upper.unwrap()<coarse.error_upper.unwrap());
    assert!(fine.error_upper.unwrap()<=2.);
    let level=sweep.level(129).unwrap();
    assert!(level.report.closed_path&&level.report.continuous_bound&&level.report.accepted);
    let sections=sweep.sections(129).unwrap().0;
    assert_eq!(sections[0].control_points,sections[128].control_points);
    for i in 0..128 {for f in [0.25,0.5,0.75] {
        let t=(i as f64+f)/128.;
        let p=path.evaluate(t).unwrap().point;
        for c in 0..2 {
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[c][k]+f*sections[i+1].control_points[c][k]-[p[0],p[1],1.+c as f64][k]);
            assert!(norm(delta)<=fine.error_upper.unwrap());
        }
    }}
    let short=sweep.fixed_patch_error_bound(129,fine.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn closed_fixed_normal_rational_path_bound_covers_radial_transport_and_copied_seam(){
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([6.,0.,1.],[6.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::FixedNormal,spacing:Spacing::Parameter,
        initial_sections:33,max_sections:129,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let proof=sweep.fixed_normal_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);assert!(proof.within_budget);
    let level=sweep.level(129).unwrap();
    assert!(level.report.closed_path&&level.report.continuous_bound&&level.report.accepted);
    let upper=proof.error_upper.unwrap();
    let sections=sweep.sections(129).unwrap().0;
    assert_eq!(sections[0].control_points,sections[128].control_points);
    // A planar circle's constant-Z projection has radial binormal C/5.
    for i in 0..128 {for f in [0.25,0.5,0.75] {
        let t=(i as f64+f)/128.;let p=path.evaluate(t).unwrap().point;
        for j in 0..2 {
            let expected=[1.2*p[0],1.2*p[1],(j+1) as f64];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let short=sweep.fixed_normal_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}


#[test]
fn fixed_affine_piece_proof_cannot_hide_rational_speed_or_interior_corner(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let mut rational=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    rational.weights=vec![1.,2.];
    let corner=Curve {degree:1,knots:vec![0.,0.,0.3,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,10.],vec![0.,10.,10.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,initial_sections:3,max_sections:3,max_deviation:1e-12};
    for path in [&rational,&corner] {
        let sweep=Sweep::new(&profile,path,&scale,&twist,opts).unwrap();
        assert!(!fixed_affine_interval(&sweep,3,[0.,0.5]));
        let level=sweep.level(3).unwrap();assert!(!level.report.accepted);
        assert!(level.report.continuous_bound);
        let upper=level.report.continuous_error_upper.unwrap();assert!(upper>0.5);
        let sections=sweep.sections(3).unwrap().0;
        let p=path.evaluate(0.25).unwrap().point;
        let delta=std::array::from_fn(|k|0.5*sections[0].control_points[0][k]+0.5*sections[1].control_points[0][k]-[p[0]+1.,p[1],p[2]][k]);
        assert!(norm(delta)>0.5&&norm(delta)<=upper);
    }
}


#[test]
fn fixed_joint_affine_dense_profile_composes_decomposition_and_original_transport(){
    let profile=Curve {degree:1,knots:std::iter::once(0.).chain((0..=32).map(|i|i as f64)).chain(std::iter::once(32.)).collect(),
        control_points:(0..=32).map(|i|vec![1.+i as f64/32.,(i%2) as f64/64.,0.]).collect(),
        weights:(0..=32).map(|i|if i%2==0 {1.} else {2.}).collect(),periodic:false};
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.25,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,initial_sections:3,max_sections:3,max_deviation:100.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let report=sweep.fixed_patch_error_bound(3,10000,1000).unwrap();
    assert_eq!(report.status,Status::Certified);assert!(report.within_budget);
    assert_eq!(report.products,384);assert!(report.decomposition_error_upper.unwrap()>0.);
    let upper=report.error_upper.unwrap();
    let patches=report.patches.as_ref().unwrap();assert_eq!(patches.len(),32);
    for (i,patch) in patches.iter().enumerate(){
        let a=patch.knots_u[patch.degree_u];let b=patch.knots_u[patch.control_points.len()];
        let f=0.375;let denom=(1.-f)*profile.weights[i]+f*profile.weights[i+1];
        let q:[f64;3]=std::array::from_fn(|k|((1.-f)*profile.weights[i]*profile.control_points[i][k]+f*profile.weights[i+1]*profile.control_points[i+1][k])/denom);
        for t in [0.125,0.375,0.625,0.875] {
            let x=(1.+t)*(1.+t)*q[0]+t/8.;let y=(1.+t)*(2.-t)*q[1]+t/4.;
            let expected=[x*0.25_f64.cos()-y*0.25_f64.sin(),x*0.25_f64.sin()+y*0.25_f64.cos(),10.*t];
            let p=patch.evaluate(a+f*(b-a),t).unwrap().point;
            assert!(norm(std::array::from_fn(|k|p[k]-expected[k]))<=upper);
        }
    }
    let short=sweep.fixed_patch_error_bound(3,10000,report.products-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none()&&short.patches.is_none());
}


#[test]
fn fixed_patch_certificate_proves_line_arc_length_but_refuses_closed_paths_and_other_modes(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:3,max_deviation:1.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let report=sweep.level(3).unwrap().report;
    assert!(report.continuous_bound&&report.accepted);
    assert!(report.continuous_error_upper.unwrap()<=opts.max_deviation);
    let closed=crate::primitives::polyline(&[[0.;3],[0.,0.,1.],[0.;3]],false).unwrap();
    let refused=Sweep::new(&profile,&closed,&scale,&twist,opts).unwrap()
        .fixed_patch_error_bound(3,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
    assert_eq!(refused.reason,Some("arc-length-correspondence-unproved"));
    for orientation in [Orientation::RotationMinimizing,Orientation::FixedNormal] {
        let s=Sweep::new(&profile,&path,&scale,&twist,Options {orientation,spacing:Spacing::Parameter,..opts}).unwrap();
        let proof=s.fixed_patch_error_bound(3,10000,1000000).unwrap();
        assert_eq!(proof.status,Status::Unresolved);assert_eq!(proof.reason,Some("mode-not-fixed"));
        let report=s.level(3).unwrap().report;
        // Each mode uses its own source-frame premise; the Fixed entry point
        // above remains unavailable to both modes.
        assert!(report.continuous_bound);assert!(report.accepted);
    }
}

#[test]
fn fixed_normal_retained_bound_encloses_original_curved_transport(){
    let profile=crate::primitives::line([0.,-1.,1.],[0.,-1.,2.]).unwrap();
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::FixedNormal,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:33,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let fine=sweep.level(33).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    let upper=fine.report.continuous_error_upper.unwrap();
    let sections=sweep.sections(33).unwrap().0;
    for i in 0..32 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/32.;let h=(1.+4.*t*t).sqrt();
        for j in 0..2 {
            let expected=[t+(1.+t)*2.*t/h,t*t-(1.+t)/h,(j+1) as f64*(1.+t)];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.fixed_normal_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);
    let short=sweep.fixed_normal_patch_error_bound(33,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let arc=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..opts}).unwrap();
    assert_eq!(arc.fixed_normal_patch_error_bound(33,0,1000000).unwrap().reason,Some("arc-length-correspondence-unproved"));
    #[cfg(feature="transport")]
    {
        use value_codec::json;
        let request=json!({"op":"surface_progressive_sweep_level","profiles":[profile],"path":path,
            "scale":scale,"twist":twist,"normal":[0.,0.,1.],"orientation":"fixed_normal","spacing":"parameter",
            "initial_sections":3,"max_sections":33,"max_deviation":0.1,"preview_sections":33});
        let result=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(result["report"]["continuousBound"],true);
        assert_eq!(result["report"]["accepted"],true);
        assert_eq!(result["report"]["continuousErrorUpper"],upper);
        assert_eq!(result["report"]["continuousErrorScope"],"retained-patches-relative-to-original-profile-transport");
        let mut strict=request;
        strict["max_deviation"]=json!(1e-30);
        let refused=crate::transport::dispatch(strict).unwrap();
        assert_eq!(refused["report"]["continuousBound"],true);
        assert_eq!(refused["report"]["accepted"],false);
    }
}

#[test]
fn fixed_normal_rational_path_joint_affine_twist_has_original_transport_bound(){
    let profile=crate::primitives::line([0.,-1.,1.],[0.,-1.,2.]).unwrap();
    // Independent rational generator C(t)=(t,t²,0)/(1+t).
    let path=Curve {degree:2,knots:vec![7.,7.,7.,11.,11.,11.],
        control_points:vec![vec![0.;3],vec![1./3.,0.,0.],vec![0.5,0.5,0.]],
        weights:vec![1.,1.5,2.],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    for orientation in [Orientation::FixedNormal,Orientation::RotationMinimizing] {
    let opts=Options {normal:[0.,0.,1.],orientation,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:33,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let certify=|cells|if orientation==Orientation::FixedNormal {sweep.fixed_normal_patch_error_bound(33,cells,1000000)}else{sweep.rmf_planar_patch_error_bound(33,cells,1000000)};
    let proof=certify(10000).unwrap();
    assert_eq!(proof.status,Status::Certified);assert!(proof.within_budget);
    let upper=proof.error_upper.unwrap();
    let sections=sweep.sections(33).unwrap().0;
    for i in 0..32 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/32.;let v=t*(2.+t);let h=(1.+v*v).sqrt();
        let angle=0.25*t;
        for j in 0..2 {
            let x=(1.+t)*(1.+t)*(j+1) as f64+t/8.;
            let y=(1.+t)*(2.-t)+t/4.;
            let side=x*angle.sin()+y*angle.cos();
            let expected=[t/(1.+t)+v/h*side,t*t/(1.+t)-side/h,x*angle.cos()-y*angle.sin()];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let short=certify(proof.cells-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    }
}


#[test]
fn fixed_normal_joint_affine_dense_profile_composes_decomposition_and_original_transport(){
    let profile=Curve {degree:1,knots:std::iter::once(0.).chain((0..=32).map(|i|i as f64)).chain(std::iter::once(32.)).collect(),
        control_points:(0..=32).map(|i|vec![1.+i as f64/32.,(i%2) as f64/64.,0.]).collect(),
        weights:(0..=32).map(|i|if i%2==0 {1.} else {2.}).collect(),periodic:false};
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.25,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::FixedNormal,spacing:Spacing::Parameter,initial_sections:3,max_sections:3,max_deviation:100.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let report=sweep.fixed_normal_patch_error_bound(3,10000,1000).unwrap();
    assert_eq!(report.status,Status::Certified);assert!(report.within_budget);
    assert_eq!(report.products,384);assert!(report.decomposition_error_upper.unwrap()>0.);
    let upper=report.error_upper.unwrap();
    let patches=report.patches.as_ref().unwrap();assert_eq!(patches.len(),32);
    for (i,patch) in patches.iter().enumerate(){
        let a=patch.knots_u[patch.degree_u];let b=patch.knots_u[patch.control_points.len()];
        let f=0.375;let denom=(1.-f)*profile.weights[i]+f*profile.weights[i+1];
        let q:[f64;3]=std::array::from_fn(|k|((1.-f)*profile.weights[i]*profile.control_points[i][k]+f*profile.weights[i+1]*profile.control_points[i+1][k])/denom);
        for t in [0.125,0.375,0.625,0.875] {
            let x=(1.+t)*(1.+t)*q[0]+t/8.;let y=(1.+t)*(2.-t)*q[1]+t/4.;
            let expected=[x*0.25_f64.cos()-y*0.25_f64.sin(),x*0.25_f64.sin()+y*0.25_f64.cos(),10.*t];
            let p=patch.evaluate(a+f*(b-a),t).unwrap().point;
            assert!(norm(std::array::from_fn(|k|p[k]-expected[k]))<=upper);
        }
    }
    let short=sweep.fixed_normal_patch_error_bound(3,10000,report.products-1).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none()&&short.patches.is_none());
}


#[test]
fn closed_frenet_rational_path_bound_covers_radial_transport_and_copied_seam(){
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([6.,0.,1.],[6.,0.,2.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,
        initial_sections:33,max_sections:129,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let proof=sweep.frenet_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);assert!(proof.within_budget);
    let level=sweep.level(129).unwrap();
    assert!(level.report.closed_path&&level.report.continuous_bound&&level.report.accepted);
    let upper=proof.error_upper.unwrap();
    let sections=sweep.sections(129).unwrap().0;
    assert_eq!(sections[0].control_points,sections[128].control_points);
    // A planar circle's Frenet inward normal is -C/5; initial coordinate is -1.
    for i in 0..128 {for f in [0.25,0.5,0.75] {
        let t=(i as f64+f)/128.;let p=path.evaluate(t).unwrap().point;
        for j in 0..2 {
            let expected=[1.2*p[0],1.2*p[1],(j+1) as f64];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let short=sweep.frenet_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}




#[test]
fn spatial_frenet_joint_affine_center_scale_twist_bound_is_original(){
    let profile=crate::primitives::line([0.,1.,1.],[0.,2.,1.]).unwrap();
    let path=Curve {degree:3,knots:vec![2.,2.,2.,2.,5.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![1./3.,0.,0.],vec![2./3.,1./3.,0.],vec![1.,1.,1.]],weights:vec![1.;4],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Frenet,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:129,max_deviation:0.1};
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    assert!(!sweep.level(3).unwrap().report.accepted);
    let fine=sweep.level(129).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    assert!(fine.report.error_certificate_cells<=10000);
    let upper=fine.report.continuous_error_upper.unwrap();
    let sections=sweep.sections(129).unwrap().0;
    for i in 0..128 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/128.;
        let gt=(1.+4.*t*t+9.*t.powi(4)).sqrt();let gb=(1.+9.*t*t+9.*t.powi(4)).sqrt();
        let tangent=[1./gt,2.*t/gt,3.*t*t/gt];let binormal=[3.*t*t/gb,-3.*t/gb,1./gb];
        let normal=cross(binormal,tangent);let a=0.25*t;
        for j in 0..2 {
            let x=(1.+t)*(1.+t)*(j+1) as f64+t/8.;let y=(1.+t)*(2.-t)+t/4.;
            let n=x*a.cos()-y*a.sin();let b=x*a.sin()+y*a.cos();
            let c=[t,t*t,t*t*t];let expected:V=std::array::from_fn(|k|c[k]+n*normal[k]+b*binormal[k]);
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.frenet_patch_error_bound(129,10000,1000000).unwrap();
    let short=sweep.frenet_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}



#[test]
fn original_rational_straight_rmf_bound_includes_affine_twist_and_station_rounding(){
    let profile=Curve {degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![2.,0.,1.]],weights:vec![1.,2.],periodic:false};
    let path=Curve {weights:vec![1.,2.],..crate::primitives::line([0.;3],[0.,0.,10.]).unwrap()};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.01};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.rmf_straight_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<=opts.max_deviation);
    let level=sweep.preview_at(129).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;let qx=(1.+3.*u)/(1.+u);let qz=2.*u/(1.+u);
        let x=(1.+t).powi(2)*qx+t/8.;let y=t/4.;let angle=0.25*t;
        let expected=[x*angle.cos()-y*angle.sin(),x*angle.sin()+y*angle.cos(),20.*t/(1.+t)+(1.+t)*qz];
        let actual=patch.evaluate(u,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.rmf_straight_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let near_line=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![f64::MIN_POSITIVE,0.,5.],vec![0.,0.,10.]],weights:vec![1.;3],periodic:false};
    let refused=Sweep::new(&profile,&near_line,&scale,&twist,opts).unwrap().rmf_straight_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
    assert_eq!(refused.reason,Some("rmf-original-frame-correspondence-unproved"));
}

#[test]
fn original_straight_rmf_accepts_all_axis_directions_with_translated_rational_sources() {
    let scale = Curve { degree: 1, knots: vec![0.,0.,1.,1.], control_points: vec![vec![1.,0.,0.];2], weights: vec![1.;2], periodic: false };
    let twist = Curve { control_points: vec![vec![0.;3];2], ..scale.clone() };
    for axis in 0..3 {
        for direction in [-1., 1.] {
            let start = [3., -2., 7.];
            let mut end = start;
            end[axis] += direction * 10.;
            let path = Curve { weights: vec![2., 3.], ..crate::primitives::line(start, end).unwrap() };
            let mut normal = [0.; 3];
            normal[(axis + 1) % 3] = 1.;
            let p: V = std::array::from_fn(|k| start[k] + normal[k]);
            let q: V = std::array::from_fn(|k| start[k] + 2. * normal[k]);
            let profile = crate::primitives::line(p, q).unwrap();
            let options = Options { normal, orientation: Orientation::RotationMinimizing, spacing: Spacing::Parameter, initial_sections: 3, max_sections: 129, max_deviation: 0.01 };
            let sweep = Sweep::new(&profile, &path, &scale, &twist, options).unwrap();
            let proof = sweep.rmf_straight_patch_error_bound(129, 10000, 1000000).unwrap();
            assert_eq!(proof.status, Status::Certified, "axis={axis}, direction={direction}");
            let upper = proof.error_upper.unwrap();
            for patch in proof.patches.as_ref().unwrap() {
                let a = patch.knots_v[patch.degree_v];
                let b = patch.knots_v[patch.control_points[0].len()];
                for i in 0..=4 {
                    let t = a + (b-a) * i as f64 / 4.;
                    for u in [0., 0.5, 1.] {
                        let expected: V = std::array::from_fn(|k| start[k] + normal[k] * (1.+u) + if k == axis {direction * 30. * t / (2.+t)} else {0.});
                        let actual = patch.evaluate(u, t).unwrap().point;
                        assert!(norm(std::array::from_fn(|k| actual[k]-expected[k])) <= upper);
                    }
                }
            }
        }
    }
}

#[test]
fn original_oblique_rational_rmf_encloses_independent_transport() {
    let scale = Curve { degree:1,knots:vec![0.,0.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![2.,0.,0.]],weights:vec![1.;2],periodic:false };
    let twist = Curve {control_points:vec![vec![0.;3],vec![0.25,0.,0.]],..scale.clone()};
    let start = [3.,-2.,7.];
    let profile = crate::primitives::line([3.,-2.,8.],[3.,-2.,9.]).unwrap();
    for direction in [-1.,1.] {
        let delta = [3.*direction,4.*direction,0.];
        let end = std::array::from_fn(|k|start[k]+delta[k]);
        let path = Curve {weights:vec![2.,3.],..crate::primitives::line(start,end).unwrap()};
        let options = Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:129,max_deviation:0.01};
        let sweep = Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
        let proof = sweep.rmf_straight_patch_error_bound(129,10000,1000000).unwrap();
        assert_eq!(proof.status,Status::Certified);
        let upper = proof.error_upper.unwrap();
        for patch in proof.patches.as_ref().unwrap() {
            let a = patch.knots_v[patch.degree_v];let b = patch.knots_v[patch.control_points[0].len()];
            for i in 0..=8 {let t=a+(b-a)*i as f64/8.;for u in [0.,0.5,1.] {
                let q=(1.+t)*(1.+u);let angle=0.25*t;
                let transverse=[0.8*direction*q*angle.sin(),-0.6*direction*q*angle.sin(),q*angle.cos()];
                let expected:V=std::array::from_fn(|k|start[k]+delta[k]*3.*t/(2.+t)+transverse[k]);
                let actual=patch.evaluate(u,t).unwrap().point;
                assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
            }}
        }
        let short=sweep.rmf_straight_patch_error_bound(129,proof.cells-1,1000000).unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    }
}

#[test]
fn planar_rmf_retained_bound_encloses_original_curved_transport(){
    let profile=crate::primitives::line([0.,-1.,1.],[0.,-1.,2.]).unwrap();
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,
        initial_sections:3,max_sections:33,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let fine=sweep.level(33).unwrap();
    assert!(fine.report.continuous_bound);assert!(fine.report.accepted);
    let upper=fine.report.continuous_error_upper.unwrap();
    let sections=sweep.sections(33).unwrap().0;
    for i in 0..32 {for f in [0.125,0.5,0.875] {
        let t=(i as f64+f)/32.;let h=(1.+4.*t*t).sqrt();
        for j in 0..2 {
            let expected=[t+(1.+t)*2.*t/h,t*t-(1.+t)/h,(j+1) as f64*(1.+t)];
            let delta=std::array::from_fn(|k|(1.-f)*sections[i].control_points[j][k]+f*sections[i+1].control_points[j][k]-expected[k]);
            assert!(norm(delta)<=upper);
        }
    }}
    let proof=sweep.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);
    let short=sweep.rmf_planar_patch_error_bound(33,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let arc=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..opts}).unwrap();
    let arc_proof=arc.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(arc_proof.status,Status::Certified,"{arc_proof:?}");assert!(arc_proof.error_upper.is_some());
    #[cfg(feature="transport")]
    {
        use value_codec::json;
        let request=json!({"op":"surface_progressive_sweep_level","profiles":[profile],"path":path,
            "scale":scale,"twist":twist,"normal":[0.,0.,1.],"orientation":"rmf","spacing":"parameter",
            "initial_sections":3,"max_sections":33,"max_deviation":0.1,"preview_sections":33});
        let result=crate::transport::dispatch(request.clone()).unwrap();
        assert_eq!(result["report"]["continuousBound"],true);
        assert_eq!(result["report"]["accepted"],true);
        assert_eq!(result["report"]["continuousErrorUpper"],upper);
        assert_eq!(result["report"]["continuousErrorScope"],"retained-patches-relative-to-original-profile-transport");
        let mut strict=request;
        strict["max_deviation"]=json!(1e-30);
        let refused=crate::transport::dispatch(strict).unwrap();
        assert_eq!(refused["report"]["continuousBound"],true);
        assert_eq!(refused["report"]["accepted"],false);
    }
}


#[test]
fn planar_rmf_premise_uses_original_coefficients_and_regular_tangent(){
    let profile=crate::primitives::line([0.,-1.,1.],[0.,-1.,2.]).unwrap();
    let path=Curve {degree:2,knots:vec![2.,2.,2.,5.,5.,5.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.,0.75,1.25],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let options=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::Parameter,initial_sections:3,max_sections:33,max_deviation:1.};
    for axis in 0..3 {
        let rotate=|p:&Vec<f64>|(0..3).map(|k|p[(k+axis)%3]+[3.,7.,11.][k]).collect::<Vec<_>>();
        let mut source=path.clone();source.control_points=path.control_points.iter().map(rotate).collect();
        let mut section=profile.clone();section.control_points=profile.control_points.iter().map(rotate).collect();
        let normal=std::array::from_fn(|k|if (k+axis)%3==2 {-3.}else{0.});
        assert!(original_planar_rmf(&source,normal));
        let sweep=Sweep::new(&section,&source,&scale,&twist,Options {normal,..options}).unwrap();
        let proof=sweep.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
        assert_eq!(proof.status,Status::Certified,"{:?}",proof.reason);
    }
    let mut off_plane=path.clone();off_plane.control_points[1][2]=f64::from_bits(1);
    assert!(!original_planar_rmf(&off_plane,options.normal));
    let refused=Sweep::new(&profile,&off_plane,&scale,&twist,options).unwrap().rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);
    assert!(matches!(refused.reason,Some("source-plane-coefficients-nonzero"|"source-plane-exact-work-unproved")));
    assert!(refused.cells>0&&refused.cells<=10000);assert!(refused.error_upper.is_none());
    assert!(!original_planar_rmf(&path,[0.001,0.,1.]));
    let mut singular=path;singular.control_points=vec![vec![0.;3];3];
    let refused=Sweep::new(&profile,&singular,&scale,&twist,options).unwrap().rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
}

#[test]
fn closed_fixed_normal_full_turn_first_defect_refines_without_endpoint_smoothness_claim() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([5.,0.,-1.],[5.,0.,1.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
    let sweep=Sweep::new(&profile,&path,&scale,&twist,Options {
        normal:[0.,0.,1.],orientation:Orientation::FixedNormal,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:257,max_deviation:0.01,
    }).unwrap();
    let mut previous=f64::INFINITY;
    for count in [33,65,129,257] {
        let bound=sweep.fixed_normal_patch_error_bound(count,10000,1000000).unwrap();
        assert_eq!(bound.status,Status::Certified);
        let upper=bound.error_upper.unwrap();
        eprintln!("closed FixedNormal count={count}, upper={upper}, cells={}",bound.cells);
        assert!(upper<previous);
        previous=upper;
    }
    let sections=sweep.sections(257).unwrap().0;
    let qs=[[[-1.,-1.],[0.,0.],[0.,0.]]];
    for station in [0,1,63,64,65,127,128,191,192,254,255] {
        let interval=[station as f64/256.,(station+1) as f64/256.];
        let jets=authored_frame_certificate::certify_fixed_normal_control_trajectories(
            &path,[0.,0.,1.],&scale,&twist,None,&qs,interval,10000).unwrap();
        assert_eq!(jets.status,Status::Certified);
        let jet=authored_frame_certificate::TrajectoryReport {
            traversal:interval,status:jets.status,cells:jets.cells,
            jet:jets.jets.as_ref().map(|v|v[0].clone()),single_span:jets.single_span,reason:jets.reason,
        };
        let endpoints=std::array::from_fn(|i|std::array::from_fn(|k|
            sections[station+i].control_points[0][k]));
        let errors=std::array::from_fn(|i|{
            let value=authored_frame_certificate::certify_fixed_normal_control_values(
                &path,[0.,0.,1.],&scale,&twist,None,&qs,[interval[i];2],10000).unwrap();
            authored_frame_certificate::ControlValueReport {status:value.status,cells:value.cells,
                value:value.values.map(|v|v[0]),reason:value.reason}
                .retained_displacement_upper(endpoints[i]).unwrap().unwrap()
        });
        eprintln!("station={station}, single={}, first={:?}, second={:?}",jet.single_span,
            jet.linear_first_error_upper_on_continuous_span(endpoints,errors).unwrap(),
            jet.linear_second_error_upper_on_open_span(errors).unwrap());
    }
    assert!(previous<0.01);
    assert!(!sweep.level(65).unwrap().report.accepted);
    assert!(sweep.level(129).unwrap().report.accepted);
    // Independent original circle tangent and full-turn transport equation,
    // including the interval terminating at the retained copied seam.
    for station in 0..256 {for fraction in [0.125,0.5,0.875] {
        let t=(station as f64+fraction)/256.;
        let original=path.evaluate(t).unwrap();
        let d=original.d1.unwrap();let length=norm([d[0],d[1],d[2]]);
        let angle=std::f64::consts::TAU*t;
        let normal=[angle.sin()*d[1]/length,-angle.sin()*d[0]/length,angle.cos()];
        for control in 0..2 {
            let sign=if control==0 {-1.}else {1.};
            let error=std::array::from_fn(|k|
                (1.-fraction)*sections[station].control_points[control][k]
                +fraction*sections[station+1].control_points[control][k]
                -original.point[k]-sign*normal[k]);
            assert!(norm(error)<=previous,"station={station}, fraction={fraction}");
        }
    }}
    // No closed RMF holonomy or global embedding claim is implied.
}

#[test]
fn first_defect_gate_preserves_original_scale_kink_error() {
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::line([0.;3],[0.,0.,10.]).unwrap();
    let mut scale=constant_vector_law([1.,0.,0.]).unwrap();
    scale.knots=vec![0.,0.,0.3,1.,1.];
    scale.control_points=vec![vec![1.,0.,0.],vec![1.,0.,0.],vec![2.,0.,0.]];
    scale.weights=vec![1.;3];
    let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=Sweep::new(&profile,&path,&scale,&twist,Options {
        normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::Parameter,initial_sections:3,max_sections:3,max_deviation:0.01,
    }).unwrap();
    assert!(!original_laws_have_no_interior_knots(&sweep,[0.,0.5]));
    let report=sweep.fixed_patch_error_bound(3,10000,1000000).unwrap();
    assert_eq!(report.status,Status::Certified);
    let upper=report.error_upper.unwrap();
    let sections=sweep.sections(3).unwrap().0;
    // The original scale is constant until0.3, then linear. Its
    // second derivative vanishes on both pieces, yet chord error is
    // nonzero. An interior knot prevents assuming one smooth span.
    let t=0.29;
    let fraction=t/0.5;
    let retained=(1.-fraction)*sections[0].control_points[1][0]
        +fraction*sections[1].control_points[1][0];
    assert!((retained-2.).abs()>0.3);
    assert!((retained-2.).abs()<=upper);
    assert!(!report.within_budget);
}

#[test]
fn closed_coordinate_planar_rmf_matches_fixed_normal_full_turn_sections() {
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
    for axis in 0..3 {for offset in [0.,17.] {for sign in [-1.,1.] {
        let mut center=[0.;3];center[axis]=offset;
        let mut seed=[0.;3];seed[axis]=sign*3.;
        let mut axis_normal=[0.;3];axis_normal[axis]=1.;
        let path=crate::primitives::circle(center,axis_normal,5.).unwrap();
        let start=path.evaluate(0.).unwrap().point;
        let mut a=[start[0],start[1],start[2]];let mut b=a;
        a[axis]+=sign*0.5;b[axis]+=sign;
        let profile=crate::primitives::line(a,b).unwrap();
        let options=Options {normal:seed,orientation:Orientation::RotationMinimizing,
            spacing:Spacing::Parameter,initial_sections:5,max_sections:129,max_deviation:0.01};
        let rmf=Sweep::new(&profile,&path,&scale,&twist,options).unwrap();
        let fixed=Sweep::new(&profile,&path,&scale,&twist,Options {
            orientation:Orientation::FixedNormal,..options}).unwrap();
        let mut maximum=0_f64;
        for count in [33,65,129] {
            let actual=rmf.sections(count).unwrap().0;
            let reference=fixed.sections(count).unwrap().0;
            assert_eq!(actual[0].control_points,actual[count-1].control_points);
            for (a,b) in actual.iter().zip(&reference) {
                for (p,q) in a.control_points.iter().zip(&b.control_points) {
                    maximum=maximum.max(norm(std::array::from_fn(|k|p[k]-q[k])));
                }
            }
        }
        eprintln!("closed RMF axis={axis}, offset={offset}, sign={sign}, delta={maximum}");
        if offset==0. {assert_eq!(maximum,0.);}else{assert!(maximum<1e-11);}
        assert!(original_planar_rmf(&path,seed));
        let (_,closed,_,identity)=rmf.sections_with_frame_identity(129).unwrap();
        assert!(closed&&identity);
        let level=rmf.level(129).unwrap();
        assert!(level.report.continuous_bound&&level.report.accepted);
        assert!(level.report.continuous_error_upper.unwrap()<=0.01);
    }}}
}

#[test]
fn closed_planar_rmf_joint_laws_cover_original_transport_and_partial_refusal() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],5.).unwrap();
    let profile=crate::primitives::line([6.,0.,1.],[6.,0.,2.]).unwrap();
    let quadratic=|points:Vec<Vec<f64>>|Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:points,weights:vec![1.,1.5,1.],periodic:false};
    let scale=quadratic(vec![vec![1.,0.,0.],vec![1.1,0.,0.],vec![1.,0.,0.]]);
    let axes=quadratic(vec![vec![1.;3],vec![1.2,0.9,1.],vec![1.;3]]);
    let center=quadratic(vec![vec![0.;3],vec![0.1,-0.05,0.],vec![0.;3]]);
    let twist=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
    let options=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::Parameter,initial_sections:5,max_sections:129,max_deviation:0.05};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap()
        .with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.rmf_planar_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);assert!(proof.within_budget);
    let upper=proof.error_upper.unwrap();
    let sections=sweep.sections(129).unwrap().0;
    for station in 0..128 {for fraction in [0.125,0.5,0.875] {
        let t=(station as f64+fraction)/128.;
        let p=path.evaluate(t).unwrap();let d=p.d1.unwrap();
        let length=norm([d[0],d[1],d[2]]);let angle=std::f64::consts::TAU*t;
        let n=[angle.sin()*d[1]/length,-angle.sin()*d[0]/length,angle.cos()];
        let b=[angle.cos()*d[1]/length,-angle.cos()*d[0]/length,-angle.sin()];
        let s=scale.evaluate(t).unwrap().point[0];
        let a=axes.evaluate(t).unwrap().point;let c=center.evaluate(t).unwrap().point;
        for control in 0..2 {
            let original: [f64;3]=std::array::from_fn(|k|p.point[k]
                +(s*a[0]*(control+1) as f64+c[0])*n[k]+(s*a[1]+c[1])*b[k]);
            let error=std::array::from_fn(|k|
                (1.-fraction)*sections[station].control_points[control][k]
                +fraction*sections[station+1].control_points[control][k]-original[k]);
            assert!(norm(error)<=upper);
        }
    }}
    let short=sweep.rmf_planar_patch_error_bound(129,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let nonaxial=Sweep::new(&profile,&path,&scale,&twist,Options {
        normal:[0.001,0.,1.],..options}).unwrap();
    assert!(!nonaxial.sections_with_frame_identity(129).unwrap().3);
    let refused=nonaxial.rmf_planar_patch_error_bound(129,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
}

#[test]
fn rational_line_arc_length_error_uses_original_sections_and_length_reference() {
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=Curve {weights:vec![1.,4.],..crate::primitives::line([0.;3],[0.,0.,10.]).unwrap()};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.25,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,2.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    for orientation in [Orientation::Fixed,Orientation::RotationMinimizing] {
        let opts=Options {normal:[1.,0.,0.],orientation,
            spacing:Spacing::ArcLength {tolerance:1e-3,max_cells:100000},
            initial_sections:3,max_sections:33,max_deviation:0.1};
        let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
        let proof=if orientation==Orientation::Fixed {sweep.fixed_patch_error_bound(33,10000,1000000)}
            else {sweep.rmf_straight_patch_error_bound(33,10000,1000000)}.unwrap();
        assert_eq!(proof.status,Status::Certified);
        let upper=proof.error_upper.unwrap();assert!(upper<0.1);
        for patch in proof.patches.as_ref().unwrap() {for i in 0..=8 {for j in 0..=8 {
            let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
            let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;let radius=(1.+t).powi(2)*(1.+u)+t/8.;let side=t/4.;
            let expected=[radius*(0.25*t).cos()-side*(0.25*t).sin(),radius*(0.25*t).sin()+side*(0.25*t).cos(),10.*t];
            let actual=patch.evaluate(u,t).unwrap().point;
            assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
        }}}
        let short=if orientation==Orientation::Fixed {sweep.fixed_patch_error_bound(33,proof.cells-1,1000000)}
            else {sweep.rmf_straight_patch_error_bound(33,proof.cells-1,1000000)}.unwrap();
        assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
        let curved=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![0.;3],vec![0.1,0.,5.],vec![0.,0.,10.]],weights:vec![1.;3],periodic:false};
        let closed=crate::primitives::polyline(&[[0.;3],[0.,0.,1.],[0.;3]],false).unwrap();
        let other=Sweep::new(&profile,if orientation==Orientation::Fixed {&closed}else{&curved},&scale,&twist,opts).unwrap();
        let refused=if orientation==Orientation::Fixed {other.fixed_patch_error_bound(33,10000,1000000)}
            else {other.rmf_straight_patch_error_bound(33,10000,1000000)}.unwrap();
        assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
        assert_eq!(refused.reason,Some(if orientation==Orientation::Fixed {"arc-length-correspondence-unproved"}
            else {"rmf-original-frame-correspondence-unproved"}));
    }
}

#[test]
fn guided_arc_length_uses_two_independent_original_line_images(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=Curve {weights:vec![1.,2.],..crate::primitives::line([0.;3],[0.,0.,10.]).unwrap()};
    let guide=Curve {weights:vec![1.,4.],..crate::primitives::line([1.,0.,0.],[2.,1.,10.]).unwrap()};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:9,max_deviation:0.05};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&guide).unwrap();
    let proof=sweep.guided_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.05);
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;let h=((1.+t).powi(2)+t*t).sqrt();
        let expected=[(1.+u)*(1.+t)/h,(1.+u)*t/h,10.*t];
        let actual=patch.evaluate(u,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.guided_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let curved=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![1.,0.,0.],vec![1.,1.,5.],vec![2.,1.,10.]],weights:vec![1.;3],periodic:false};
    let refused=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&curved).unwrap()
        .guided_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Certified,"{refused:?}");
    assert!(refused.error_upper.unwrap().is_finite() && refused.cells<=10000);
    let limited=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&curved).unwrap()
        .guided_patch_error_bound(9,1,1000000).unwrap();
    assert_eq!(limited.status,Status::Unresolved);assert!(limited.error_upper.is_none());
}

#[test]
fn authored_arc_length_preserves_original_moving_frame_and_affine_laws(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=Curve {weights:vec![1.,4.],..crate::primitives::line([0.;3],[0.,0.,10.]).unwrap()};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();
    let normal=crate::primitives::line([1.,0.,0.],[1.,1.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:0.1};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap()
        .with_frame_laws(&axis,&normal).unwrap().with_affine_laws(&axes,&center).unwrap();
    assert_eq!(sweep.authored_control_trajectory(0,[0.,0.125],10000).unwrap().status,Status::Certified);
    let proof=sweep.authored_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.1);
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;let h=(1.+t*t).sqrt();
        let x=(1.+t).powi(2)*(1.+u)+t/8.;let y=t/4.;
        let expected=[(x-t*y)/h,(t*x+y)/h,10.*t];
        let actual=patch.evaluate(u,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.authored_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
    let curved=Curve {degree:1,knots:vec![0.,0.,0.5,0.5,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,4.],vec![1.,0.,6.],vec![0.,0.,10.]],weights:vec![1.;4],periodic:false};
    assert!(Sweep::new_authored(&profile,&curved,&scale,&twist,&axis,&normal,opts).is_err());
    let closed=crate::primitives::polyline(&[[0.;3],[0.,0.,1.],[0.;3]],false).unwrap();
    let one=constant_vector_law([1.,0.,0.]).unwrap();
    let fixed_normal=constant_vector_law([1.,0.,0.]).unwrap();
    let refused=Sweep::new_authored(&profile,&closed,&one,&twist,&axis,&fixed_normal,Options {initial_sections:5,..opts}).unwrap()
        .authored_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(refused.status,Status::Certified,"{refused:?}");
    assert!(refused.error_upper.unwrap()<=opts.max_deviation);
}

#[test]
fn authored_arc_length_c0_path_uses_original_length_and_relative_pose_bounds(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::polyline(&[[0.;3],[0.,0.,1.],[0.,1.,1.]],false).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:9,max_deviation:0.2};
    let sweep=Sweep::new_authored(&profile,&path,&scale,&twist,&axis,&normal,opts).unwrap();
    let proof=sweep.authored_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.2);
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;
        let expected=[1.+u,(2.*t-1.).max(0.),(2.*t).min(1.)];
        let actual=patch.evaluate(u,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.authored_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn authored_arc_length_curved_path_and_joint_pose_match_independent_length_inverse(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    // Exact original polynomial C(u)=(0,u²,u). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();
    let normal=crate::primitives::line([1.,0.,0.],[1.,1.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:9,max_deviation:0.2};
    let sweep=Sweep::new_authored(&profile,&path,&scale,&twist,&axis,&normal,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.authored_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.2);
    let level=sweep.preview_at(9).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let angle=t.atan()+0.125*t;
        let x=(1.+t).powi(2)*(1.+q)+t/8.;let y=t/4.;
        let expected=[x*angle.cos()-y*angle.sin(),u*u+x*angle.sin()+y*angle.cos(),u];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.authored_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn shared_arc_length_proof_keeps_multi_profile_partial_work_unproved(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let profiles=vec![profile;64];
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();let normal=crate::primitives::line([1.,0.,0.],[1.,1.,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:9,max_deviation:1.};
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let level=sweep.preview_at(9).unwrap();
    assert_eq!(level.profile_patch_ranges.len(),64);
    assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());
    assert!(level.report.error_certificate_reason.is_some());assert!(level.report.error_certificate_cells<=10000);
}

#[test]
fn fixed_curved_arc_length_encloses_independent_inverse_with_joint_laws(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    // Exact original polynomial C(u)=(0,u²,u). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:9,max_deviation:0.2};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.fixed_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.2);
    let level=sweep.preview_at(9).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let angle=0.125*t;
        let x=(1.+t).powi(2)*(1.+q)+t/8.;let y=t/4.;
        let expected=[x*angle.cos()-y*angle.sin(),u*u+x*angle.sin()+y*angle.cos(),u];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.fixed_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn fixed_arc_length_c0_path_uses_original_length_and_initial_tangent_frame(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=crate::primitives::polyline(&[[0.;3],[0.,0.,1.],[0.,1.,1.]],false).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:9,max_deviation:0.2};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap();
    let proof=sweep.fixed_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<0.2);
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let u=j as f64/8.;
        let expected=[1.+u,(2.*t-1.).max(0.),(2.*t).min(1.)];
        let actual=patch.evaluate(u,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.fixed_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}


// Position uses original cumulative length; relative pose uses original source
// parameter brackets and independent normalized station-law intervals. The
// interval image remainder needs no unproved inverse-length derivative jets.
fn fixed_normal_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation(sweep,count,max_cells,shared_length,OriginalTransport::FixedNormal,None)
}
fn original_frame_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,mode,shared_guide,512,None)
}
fn original_frame_arc_length_section_interpolation_with_rmf_steps(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mut mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>,rmf_steps:usize,shared_rmf:Option<&super::rmf_transport::OriginalRmfTransportReport>)->Result<SectionInterpolationReport>{
    let mut out=SectionInterpolationReport {status:Status::Unresolved,cells:0,error_upper:None,
        endpoint_displacement_upper:None,cap_endpoint_displacement_upper:None,reason:Some("arc-length-correspondence-unproved")};
    let phase=if mode==OriginalTransport::Contact {
        let Spacing::ArcLength {max_cells:constructor_budget,..}=sweep.options.spacing else{return Ok(out);};
        let phase=super::arc_guide::certify(sweep.path,sweep.orientation_guide.unwrap(),max_cells.min(constructor_budget))?;
        out.cells+=phase.cells;Some(phase)
    }else{None};
    let same_phase=phase.as_ref().is_some_and(|p|p.scale_upper.is_some());
    // Guided frames bypass RMF holonomy correction. The copied closing
    // section is compared below against the original endpoint value image;
    // no endpoint identity or zero displacement is assumed.
    let closed_guided=((mode==OriginalTransport::Guided&&sweep.contact_point.is_none())
        ||(mode==OriginalTransport::Contact&&same_phase))&&sweep.frame_laws.is_none();
    let closed=path_is_closed(sweep.path)?;
    let closed_rmf=closed&&matches!(mode,OriginalTransport::RmfPlanar|OriginalTransport::RmfSpatial)
        &&sweep.options.orientation==Orientation::RotationMinimizing&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none();
    // Fixed-normal transport has no closing holonomy correction. Its copied
    // seam is still charged against the original endpoint image below.
    let closed_fixed_normal=closed&&mode==OriginalTransport::FixedNormal
        &&sweep.options.orientation==Orientation::FixedNormal&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none();
    // Planar corrected transport also has no closing holonomy correction.
    // Actual principal-normal sign choices and copied endpoint remain in the
    // retained-versus-original image displacement bound, not assumed exact.
    let closed_corrected=closed&&mode==OriginalTransport::CorrectedPlanar
        &&sweep.options.orientation==Orientation::CorrectedFrenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none();
    if (sweep.path.periodic&&!(closed_rmf&&mode==OriginalTransport::RmfSpatial))
        ||(closed&&!closed_guided&&!closed_rmf&&!closed_fixed_normal&&!closed_corrected){return Ok(out);}
    let Spacing::ArcLength {tolerance,max_cells:constructor_budget}=sweep.options.spacing else{return Ok(out);};
    if max_cells==0{return Ok(out);}
    // Charge the exact original-coefficient plane premise to the same owner
    // as inverse length and frame bounds. Unproved input never admits RMF.
    if mode==OriginalTransport::CorrectedPlanar&&(sweep.options.orientation!=Orientation::CorrectedFrenet
        ||sweep.frame_laws.is_some()||sweep.orientation_guide.is_some()||sweep.contact_point.is_some()){
        out.reason=Some("corrected-affine-frame-phase-unproved");return Ok(out);
    }
    if matches!(mode,OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar)&&!original_planar_rmf(sweep.path,sweep.options.normal){
        let plane=super::source_plane::certify(sweep.path,sweep.options.normal,max_cells as u64)?;
        out.cells+=plane.exact_work as usize;
        if !plane.proved{if closed && mode==OriginalTransport::RmfPlanar && plane.reason==Some("source-plane-coefficients-nonzero") {mode=OriginalTransport::RmfSpatial;}else{out.reason=plane.reason;return Ok(out);}}
        if out.cells==max_cells{out.reason=Some("rmf-original-frame-work-unproved");return Ok(out);}
    }
    let owned_transport;
    let spatial_transport=if mode==OriginalTransport::RmfSpatial {
        let report=if let Some(shared)=shared_rmf {shared} else {
        let remaining=max_cells-out.cells;
        // Exact coefficient work and interval cells share this owner.
        owned_transport=super::rmf_transport::certify_original_rmf_transport_shared(sweep.path,sweep.options.normal,
            rmf_steps,remaining,closed)?;
        out.cells+=owned_transport.cells+owned_transport.exact_work as usize;
        &owned_transport
        };
        if report.status!=Status::Certified {out.reason=report.reason;return Ok(out);}
        Some(report)
    }else{None};
    let principal_phase=if mode==OriginalTransport::CorrectedPlanar&&sweep.affine_laws.is_some(){
        let (cells,phase)=authored_frame_certificate::certify_initial_planar_principal_phase(sweep.path,sweep.options.normal,max_cells-out.cells)?;
        out.cells+=cells;
        let Some(phase)=phase else{out.reason=Some("corrected-affine-frame-phase-unproved");return Ok(out);};
        Some(phase)
    }else{None};
    let tolerance=phase.as_ref().and_then(|p|p.scale_upper).filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down());
    let owned;
    let division=if let Some(shared)=shared_length{shared}else{
        owned=crate::curve_measure::divide_by_length(sweep.path,count-1,tolerance,(max_cells-out.cells).min(constructor_budget))?;
        out.cells+=owned.cells;&owned
    };
    if !division.within_tolerance||division.points.len()!=count{return Ok(out);}
    let parameters=sweep.parameters(count)?.0;
    if division.points.iter().zip(parameters).any(|(p,u)|p.parameter!=u){return Ok(out);}
    let mut guide_source=Vec::new();
    if (mode==OriginalTransport::Guided||mode==OriginalTransport::Contact)&&!same_phase {
        let Some(guide)=sweep.orientation_guide else{return Ok(out);};
        if guide.periodic||(path_is_closed(guide)?&&!closed_guided){return Ok(out);}
        let owned_guide;
        let guide_division=if let Some(shared)=shared_guide{shared}else{
            if out.cells==max_cells{out.reason=Some("guide-length-work-unproved");return Ok(out);}
            owned_guide=crate::curve_measure::divide_by_length(guide,count-1,tolerance,(max_cells-out.cells).min(constructor_budget))?;
            out.cells+=owned_guide.cells;&owned_guide
        };
        if !guide_division.within_tolerance||guide_division.points.len()!=count{return Ok(out);}
        let guide_parameters=sweep.curve_parameters(guide,count)?.0;
        if guide_division.points.iter().zip(guide_parameters).any(|(p,u)|p.parameter!=u){return Ok(out);}
        let d=guide.domain();let width=I::point(d[1]).sub(I::point(d[0]))?;
        let (speed_cover,speed_lower)=crate::curve_regularity::inspect_speed(guide,max_cells-out.cells)?;
        out.cells+=speed_cover.cells;
        let Some(speed_lower)=speed_lower else {out.reason=Some("guide-arc-speed-lower-unproved");return Ok(out);};
        for p in &guide_division.points{
            let radius=I::point(p.residual_upper).div(I::point(speed_lower))?;
            let bracket=[p.inverse_parameter_bounds[0].max(I::point(p.parameter).sub(radius)?.lo),
                p.inverse_parameter_bounds[1].min(I::point(p.parameter).add(radius)?.hi)];
            if bracket[0]>bracket[1]{out.reason=Some("guide-arc-inverse-bracket-unproved");return Ok(out);}
            let v=I::new(bracket[0],bracket[1])?.sub(I::point(d[0]))?.div(width)?;
            guide_source.push([v.lo.max(0.),v.hi.min(1.)]);
        }
    }
    let initial=match mode {
        OriginalTransport::FixedNormal=>fixed_normal_initial_coordinates(sweep,max_cells-out.cells)?,
        OriginalTransport::RmfPlanar|OriginalTransport::RmfSpatial=>seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::RotationMinimizing)?,
        OriginalTransport::CorrectedPlanar=>if let Some(phase)=principal_phase {principal_initial_coordinates(sweep,phase,max_cells-out.cells)?}else{seed_initial_coordinates_as(sweep,max_cells-out.cells,Orientation::RotationMinimizing,Orientation::CorrectedFrenet)?},
        OriginalTransport::Frenet=>seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::Frenet)?,
        OriginalTransport::Guided|OriginalTransport::Contact=>guided_initial_coordinates(sweep,max_cells-out.cells)?,
        _=>return Ok(out),
    };out.cells+=initial.cells;
    let Some(qs)=initial.coordinates.filter(|_|initial.status==Status::Certified)else{out.reason=initial.reason;return Ok(out);};
    let anchor=if mode==OriginalTransport::Contact {
        let certificate=super::contact_anchor::certify(sweep,max_cells-out.cells)?;
        out.cells+=certificate.cells;
        if certificate.status!=Status::Certified{out.reason=certificate.reason;return Ok(out);}
        Some(certificate.coordinates.unwrap()[0])
    }else{None};
    // Exact source-plane proof above makes the original Bishop normal
    // constant, even in a nonaxial plane. The actual constructor may acquire
    // floating holonomy and apply a correction; every resulting stored pole
    // (including the copied seam) is charged against the original image below.
    // No actual zero-holonomy/bitwise-frame identity premise is reused.
    let (sections,_,_,_)=sweep.sections_with_frame_identity(count)?;
    if sections.iter().any(|c|c.degree!=sweep.profile.degree||c.knots!=sweep.profile.knots||c.weights!=sweep.profile.weights
        ||c.periodic!=sweep.profile.periodic||c.control_points.len()!=qs.len()){
        out.reason=Some("retained-section-basis-correspondence-unproved");return Ok(out);
    }
    let domain=sweep.path.domain();let width=I::point(domain[1]).sub(I::point(domain[0]))?;
    let inverse_speed=if closed_rmf||closed_fixed_normal||matches!(mode,OriginalTransport::RmfSpatial|OriginalTransport::CorrectedPlanar|OriginalTransport::Guided|OriginalTransport::Contact) {
        let (cover,lower)=crate::curve_regularity::inspect_speed(sweep.path,max_cells-out.cells)?;
        out.cells+=cover.cells;
        let Some(lower)=lower else{out.reason=cover.reason.or(Some("corrected-arc-speed-lower-unproved"));return Ok(out);};
        Some(lower)
    }else{None};
    let mut source=Vec::new();
    for p in &division.points{
        let mut bracket=p.inverse_parameter_bounds;
        if let Some(lower)=inverse_speed {
            // |L(u)-s L(1)| <= residual and L' >= lower imply
            // |u-u_exact| <= residual/lower, including a symmetric station
            // whose old bisection bracket remains the entire domain.
            let radius=I::point(p.residual_upper).div(I::point(lower))?;
            bracket[0]=bracket[0].max(I::point(p.parameter).sub(radius)?.lo);
            bracket[1]=bracket[1].min(I::point(p.parameter).add(radius)?.hi);
            if bracket[0]>bracket[1]{out.reason=Some("corrected-arc-inverse-bracket-unproved");return Ok(out);}
        }
        let u=I::new(bracket[0],bracket[1])?.sub(I::point(domain[0]))?.div(width)?;
        source.push([u.lo.max(0.),u.hi.min(1.)]);
    }
    if same_phase {guide_source=source.clone();}
    let mut error=0_f64;let mut endpoint_upper=0_f64;
    for i in 0..count-1{
        let station=[i as f64/(count-1) as f64,(i+1) as f64/(count-1) as f64];
        let path_interval=[source[i][0],source[i+1][1]];
        let values=match mode {
            OriginalTransport::RmfSpatial=>super::rmf_transport::relative_values(spatial_transport.as_ref().unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::CorrectedPlanar if principal_phase.is_some()=>authored_frame_certificate::certify_planar_principal_relative_values(sweep.path,sweep.options.normal,principal_phase.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar=>authored_frame_certificate::certify_proved_planar_relative_values(sweep.path,sweep.options.normal,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::FixedNormal=>authored_frame_certificate::certify_fixed_normal_relative_values(sweep.path,sweep.options.normal,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::Frenet=>authored_frame_certificate::certify_frenet_relative_values(sweep.path,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::Guided=>authored_frame_certificate::certify_path_guide_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,[guide_source[i][0],guide_source[i+1][1]],station,same_phase,max_cells-out.cells)?,
            OriginalTransport::Contact=>authored_frame_certificate::certify_contact_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,anchor.unwrap(),path_interval,[guide_source[i][0],guide_source[i+1][1]],station,same_phase,max_cells-out.cells)?,
            _=>return Ok(out),
        };
        out.cells+=values.cells;
        let Some(values)=values.values.filter(|_|values.status==Status::Certified)else{out.reason=values.reason.or(Some("arc-length-frame-value-unproved"));return Ok(out);};
        let residual=division.points[i].residual_upper.max(division.points[i+1].residual_upper);
        let position=I::point(division.total.bounds[1]).mul(I::point(station[1]).sub(I::point(station[0]))?)?.div(I::point(2.))?.add(I::point(residual))?;
        for (j,value) in values.into_iter().enumerate(){
            let mut delta=[I::point(0.);3];let mut ends=[[I::point(0.);3];2];
            for k in 0..3{
                for e in 0..2{ends[e][k]=I::point(sections[i+e].control_points[j][k])
                    .sub(I::new(division.points[i+e].point_bounds[k][0],division.points[i+e].point_bounds[k][1])?)?;}
                delta[k]=I::new(value[k][0],value[k][1])?.sub(I::new(ends[0][k].lo.min(ends[1][k].lo),ends[0][k].hi.max(ends[1][k].hi))?)?;
            }
            error=error.max(crate::numerics::interval_vec3::norm(delta)?.add(position)?.hi);
            for e in 0..2{
                for k in 0..3{delta[k]=I::new(value[k][0],value[k][1])?.sub(ends[e][k])?;}
                endpoint_upper=endpoint_upper.max(crate::numerics::interval_vec3::norm(delta)?.add(I::point(division.points[i+e].residual_upper))?.hi);
            }
        }
    }
    if mode==OriginalTransport::Contact{
        let mut caps=[0_f64;2];
        for (endpoint,index) in [0,count-1].into_iter().enumerate(){
            // Normalized length maps endpoints exactly to original source
            // endpoints. Enclose original pose and compare with actual stored
            // controls; positive unchanged rational weights extend pole bounds
            // to the entire contour. No interior inverse is reused here.
            let station=[endpoint as f64;2];
            let values=authored_frame_certificate::certify_contact_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,anchor.unwrap(),source[index],guide_source[index],station,same_phase,max_cells-out.cells)?;
            out.cells+=values.cells;
            let Some(values)=values.values.filter(|_|values.status==Status::Certified)else{out.reason=values.reason.or(Some("contact-endpoint-contour-unproved"));return Ok(out);};
            for (j,value) in values.into_iter().enumerate(){
                let mut delta=[I::point(0.);3];
                for k in 0..3{
                    let retained=I::point(sections[index].control_points[j][k]).sub(I::new(division.points[index].point_bounds[k][0],division.points[index].point_bounds[k][1])?)?;
                    delta[k]=I::new(value[k][0],value[k][1])?.sub(retained)?;
                }
                caps[endpoint]=caps[endpoint].max(crate::numerics::interval_vec3::norm(delta)?.add(I::point(division.points[index].residual_upper))?.hi);
            }
        }
        out.cap_endpoint_displacement_upper=Some(caps);
    }
    out.status=Status::Certified;out.error_upper=Some(error);out.endpoint_displacement_upper=Some(endpoint_upper);out.reason=None;Ok(out)
}

#[test]
fn fixed_normal_arc_length_encloses_independent_curved_frame_and_joint_laws(){
    check_original_planar_frame_arc_length(Orientation::FixedNormal);
}
#[test]
fn planar_rmf_arc_length_encloses_independent_curved_frame_and_joint_laws(){
    check_original_planar_frame_arc_length(Orientation::RotationMinimizing);
}
#[test]
fn nonaxial_planar_rmf_arc_length_encloses_independent_original_image(){
    check_nonaxial_planar_rmf_original_image(Spacing::ArcLength {tolerance:0.001,max_cells:100000});
}
#[test]
fn nonaxial_planar_rmf_parameter_encloses_independent_original_image(){
    check_nonaxial_planar_rmf_original_image(Spacing::Parameter);
}
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


#[test]
fn shared_fixed_normal_arc_length_proof_keeps_multi_profile_partial_work_unproved(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let profiles=vec![profile;64];
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();// Same zero twist geometry, but all sixteen authored spans must be
    // charged to each frame restriction under the aggregate proof budget.
    let twist=Curve {degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::FixedNormal,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:9,max_deviation:1.};
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap();
    let level=sweep.preview_at(9).unwrap();
    assert_eq!(level.profile_patch_ranges.len(),64);
    assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());
    assert!(level.report.error_certificate_reason.is_some());assert!(level.report.error_certificate_cells<=10000);
}


#[test]
fn shared_planar_rmf_arc_length_keeps_multi_profile_partial_work_unproved(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let profiles=vec![profile;64];
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();// Same zero twist geometry, but all sixteen authored spans must be
    // charged to each frame restriction under the aggregate proof budget.
    let twist=Curve {degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:9,max_deviation:1.};
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap();
    let level=sweep.preview_at(9).unwrap();
    assert_eq!(level.profile_patch_ranges.len(),64);
    assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());
    assert!(level.report.error_certificate_reason.is_some());assert!(level.report.error_certificate_cells<=10000);
}


#[test]
fn frenet_arc_length_encloses_independent_curved_frame_and_joint_laws(){
    let profile=crate::primitives::line([0.,1.,0.],[0.,2.,0.]).unwrap();
    // Exact original polynomial C(u)=(u,u²,0). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Frenet,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
    let proof=sweep.frenet_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified);let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let angle=0.125*t;
        let x=(1.+t).powi(2)*(1.+q)+t/8.;let y=t/4.;
        let d=(1.+4.*u*u).sqrt();let side=x*angle.cos()-y*angle.sin();
        let expected=[u-2.*u*side/d,u*u+side/d,x*angle.sin()+y*angle.cos()];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let short=sweep.frenet_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}


#[test]
fn shared_frenet_arc_length_proof_keeps_multi_profile_partial_work_unproved(){
    let profile=crate::primitives::line([0.,1.,0.],[0.,2.,0.]).unwrap();
    let profiles=vec![profile;64];
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.5,0.,0.],vec![1.,1.,0.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();// Same zero twist geometry, but all sixteen authored spans must be
    // charged to each frame restriction under the aggregate proof budget.
    let twist=Curve {degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Frenet,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:9,max_deviation:1.};
    let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap();
    let level=sweep.preview_at(9).unwrap();
    assert_eq!(level.profile_patch_ranges.len(),64);
    assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());
    assert!(level.report.error_certificate_reason.is_some());assert!(level.report.error_certificate_cells<=10000);
}


#[test]
fn guided_arc_length_two_original_curved_images_enclose_independent_joint_laws(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    // Exact original polynomial C(u)=(0,u²,u). Its length primitive is
    // F(u)=u*sqrt(1+4u²)/2+asinh(2u)/4; inversion below never calls kernel.
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.125,0.25,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap().with_orientation_guide(&guide).unwrap();
    let proof=sweep.guided_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
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
    let short=sweep.guided_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}


#[test]
fn guided_distinct_curvature_arc_lengths_enclose_independent_frame_oracle(){
    let profile=crate::primitives::line([0.1,0.,0.],[0.2,0.,0.]).unwrap();
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,2.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=crate::primitives::line([0.;3],[0.125,0.,0.]).unwrap();
    let axes=crate::primitives::line([1.,1.,1.],[2.,1.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.01,0.02,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap().with_orientation_guide(&guide).unwrap();
    // Original length partition reuse admits the former exhaustion fixture.
    let exhausted=sweep.guided_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(exhausted.status,Status::Certified,"{exhausted:?}");
    assert!(exhausted.error_upper.unwrap()<2. && exhausted.cells<=10000);
    let limited=sweep.guided_patch_error_bound(17,1,1000000).unwrap();
    assert_eq!(limited.status,Status::Unresolved);assert!(limited.error_upper.is_none());
    let proof=sweep.guided_patch_error_bound(9,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
    // Independent analytic length primitives for C=(0,u²,u), G=(1,2v²,v).
    // No kernel evaluation, division, frame or interval helper is used here.
    let inverse=|s:f64,a:f64|{
        let length=|u:f64|u*(1.+a*a*u*u).sqrt()/2.+(a*u).asinh()/(2.*a);
        let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<s*length(1.){lo=mid;}else{hi=mid;}}
        (lo+hi)/2.
    };
    assert!((inverse(0.5,2.)-inverse(0.5,4.)).abs()>0.02);
    let cross=|a:[f64;3],b:[f64;3]|[a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0]];
    let unit=|a:[f64;3]|{let d=(a.iter().map(|v|v*v).sum::<f64>()).sqrt();a.map(|v|v/d)};
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let u=inverse(t,2.);let v=inverse(t,4.);
        let tangent=unit([0.,2.*u,1.]);let binormal=unit(cross([0.,2.*u,1.],[1.,2.*v*v-u*u,v-u]));
        let normal=cross(binormal,tangent);let angle=0.125*t;
        let x=(1.+t).powi(2)*(0.1+0.1*q)+0.01*t;let y=0.02*t;
        let nx=x*angle.cos()-y*angle.sin();let by=x*angle.sin()+y*angle.cos();
        let expected: [f64;3]=std::array::from_fn(|k|[0.,u*u,u][k]+nx*normal[k]+by*binormal[k]);
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
    }}}
    let level=sweep.preview_at(9).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let short=sweep.guided_patch_error_bound(9,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn contact_curved_arc_lengths_bound_retained_geometry_and_original_anchor(){
    let profile=crate::primitives::line([0.5,0.,0.],[1.,0.,0.]).unwrap();
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let axes=crate::primitives::line([1.;3],[2.,3.,1.]).unwrap();
    let center=crate::primitives::line([0.;3],[0.1,0.,0.]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap().with_contact_guide(&guide,1.).unwrap();
    let proof=sweep.contact_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");let upper=proof.error_upper.unwrap();assert!(upper<2.,"{proof:?}");
    let caps=proof.endpoint_contour_error_upper.unwrap();
    assert!(caps.into_iter().all(|e|e<1e-8),"{caps:?}");
    assert!(proof.original_section_endpoint_error_upper.unwrap()>caps[0].max(caps[1]));
    let length=|u:f64|u*(1.+4.*u*u).sqrt()/2.+(2.*u).asinh()/4.;
    for patch in proof.patches.as_ref().unwrap(){for i in 0..=8{for j in 0..=8{
        let a=patch.knots_v[patch.degree_v];let b=patch.knots_v[patch.control_points[0].len()];
        let t=a+(b-a)*i as f64/8.;let q=j as f64/8.;let mut lo=0.;let mut hi=1.;
        for _ in 0..64{let mid=(lo+hi)/2.;if length(mid)<t*length(1.){lo=mid;}else{hi=mid;}}
        let u=(lo+hi)/2.;let transformed=(1.+t).powi(2)+0.1*t;
        let expected=[((1.+t).powi(2)*(0.5+0.5*q)+0.1*t)/transformed,u*u,u];
        let actual=patch.evaluate(q,t).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=upper);
        if j==8{assert!((expected[0]-1.).abs()<1e-14);}
    }}}
    let level=sweep.preview_at(17).unwrap();assert!(level.report.continuous_bound&&level.report.accepted,"{:?}",level.report);
    let short=sweep.contact_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[test]
fn shared_contact_curved_arc_lengths_accept_two_profiles_and_refuse_partial_64(){
    let profile=crate::primitives::line([0.5,0.,0.],[1.,0.,0.]).unwrap();
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=Curve {degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
    for count in [2,64]{
        let profiles=vec![profile.clone();count];
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap().with_contact_guide(&guide,0,1.).unwrap();
        let level=sweep.preview_at(17).unwrap();assert_eq!(level.profile_patch_ranges.len(),count);
        assert!(level.report.error_certificate_cells<=10000);
        if count==2{assert!(level.report.continuous_bound&&level.report.accepted,"{:?}",level.report);}
        else{assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());assert!(level.report.error_certificate_reason.is_some());}
    }
}

#[test]
fn shared_guided_curved_arc_length_accepts_two_profiles_and_refuses_partial_64(){
    let profile=crate::primitives::line([1.,0.,0.],[2.,0.,0.]).unwrap();
    let path=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.;3],vec![0.,0.,0.5],vec![0.,1.,1.]],weights:vec![1.;3],periodic:false};
    let guide=Curve {degree:2,knots:vec![-3.,-3.,-3.,7.,7.,7.],control_points:vec![vec![1.,0.,0.],vec![1.,0.,0.5],vec![1.,1.,1.]],weights:vec![1.;3],periodic:false};
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=Curve {degree:1,knots:std::iter::once(0.).chain((0..=16).map(|i|i as f64/16.)).chain(std::iter::once(1.)).collect(),
        control_points:vec![vec![0.;3];17],weights:vec![1.;17],periodic:false};
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:3,max_sections:17,max_deviation:2.};
    for count in [2,64]{
        let profiles=vec![profile.clone();count];
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&guide).unwrap();
        let level=sweep.preview_at(17).unwrap();assert_eq!(level.profile_patch_ranges.len(),count);
        assert!(level.report.error_certificate_cells<=10000);
        if count==2{
            assert!(level.report.continuous_bound&&level.report.accepted,"{:?}",level.report);
            assert!(level.report.continuous_error_upper.unwrap()<=2.);
        }else{
            assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());
            assert!(level.report.error_certificate_reason.is_some());
        }
    }
}

#[cfg(test)]
#[test]
fn closed_fixed_authored_original_bound_includes_copied_end_section() {
    let profile=crate::primitives::line([4.1,0.,0.],[4.2,0.,0.]).unwrap();
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let axis=Curve {degree:3,knots:vec![0.,0.,0.,0.,0.25,0.5,0.75,1.,1.,1.,1.],
        control_points:vec![vec![0.,0.,1.],vec![0.125,0.,1.],vec![0.375,0.125,1.],
            vec![0.,0.25,1.],vec![-0.375,0.125,1.],vec![-0.125,0.,1.],vec![0.,0.,1.]],
        weights:vec![1.;7],periodic:false};
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:5,max_sections:33,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let proof=patch_error(&sweep,33,100000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    assert!(proof.within_budget,"{proof:?}");
    assert!(proof.error_upper.unwrap()>0.);
    assert!(!patch_error(&sweep,33,proof.cells-1,1000000).unwrap().within_budget);
    let coarse=sweep.level(17).unwrap();
    assert!(coarse.report.continuous_bound);
    assert!(!coarse.report.accepted);
    assert!(coarse.report.continuous_error_upper.unwrap()>2.);
    let corrected_options=Options {orientation:Orientation::RotationMinimizing,..options};
    assert!(Sweep::new(&profile,&path,&scale,&twist,corrected_options).unwrap().with_frame_laws(&axis,&normal).is_err());
    for count in [2,64] {
        let profiles=vec![profile.clone();count];
        let multi=MultiSweep::new(&profiles,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
        let level=multi.preview_at(33).unwrap();
        assert!(level.report.error_certificate_cells<=10000);
        assert_eq!(level.profile_patch_ranges.len(),count);
        if count==2 {assert!(level.report.continuous_bound&&level.report.accepted,"{:?}",level.report);}
        else {assert!(!level.report.continuous_bound);assert!(level.report.continuous_error_upper.is_none());}
    }
    let law3=|points:Vec<Vec<f64>>|Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],weights:vec![1.;3],control_points:points,periodic:false};
    let variable_scale=law3(vec![vec![1.,0.,0.],vec![1.02,0.,0.],vec![1.,0.,0.]]);
    let variable_twist=law3(vec![vec![0.;3],vec![0.03,0.,0.],vec![0.;3]]);
    let axes=law3(vec![vec![1.;3],vec![1.05,0.95,1.],vec![1.;3]]);
    let center=law3(vec![vec![0.;3],vec![0.01,0.02,0.],vec![0.;3]]);
    let joint=Sweep::new(&profile,&path,&variable_scale,&variable_twist,options).unwrap()
        .with_frame_laws(&axis,&normal).unwrap().with_affine_laws(&axes,&center).unwrap();
    let joint_proof=patch_error(&joint,33,100000,1000000).unwrap();
    assert!(joint_proof.within_budget,"{joint_proof:?}");
    assert!(joint.level(33).unwrap().report.accepted);
    assert!(!patch_error(&joint,33,joint_proof.cells-1,1000000).unwrap().within_budget);
    // Positional correspondence does not promote a non-periodic twist jet to C2.
    assert!(!joint.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let dense=Curve {degree:1,knots:std::iter::once(0.).chain((0..=32).map(|i|i as f64)).chain(std::iter::once(32.)).collect(),
        control_points:(0..=32).map(|i|vec![4.1+i as f64/320.,0.,(i%2) as f64/1000.]).collect(),
        weights:(0..=32).map(|i|if i%2==0 {1.}else {2.}).collect(),periodic:false};
    let dense_sweep=Sweep::new(&dense,&path,&scale,&twist,options).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let dense_proof=patch_error(&dense_sweep,33,100000,1000000).unwrap();
    assert!(dense_proof.within_budget,"{dense_proof:?}");
    assert!(dense_proof.products>0);
    assert!(dense_proof.decomposition_error_upper.is_some());
    let incomplete=patch_error(&dense_sweep,33,100000,dense_proof.products-1).unwrap();
    assert!(!incomplete.within_budget);
    assert!(incomplete.error_upper.is_none());
    let mut near_axis=axis.clone();near_axis.control_points.last_mut().unwrap()[0]=1e-12;
    let near=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_frame_laws(&near_axis,&normal).unwrap();
    let near_proof=patch_error(&near,33,100000,1000000).unwrap();
    assert!(near_proof.within_budget);
    assert!(near_proof.original_section_endpoint_error_upper.unwrap()>1e-13);
    assert!(!near.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let near_sections=near.sections(33).unwrap().0;
    assert_eq!(near_sections.first(),near_sections.last());
    let sections=sweep.sections(33).unwrap().0;
    assert_eq!(sections.first(),sections.last());
    let bound=proof.error_upper.unwrap();
    let [a,b]=path.domain();
    for station in 0..32 {for sample in 0..=16 {
        let f=sample as f64/16.;let t=(station as f64+f)/32.;
        let center=path.evaluate(a+(b-a)*t).unwrap().point;
        let raw=axis.evaluate(t).unwrap().point;
        let length=(raw.iter().map(|v|v*v).sum::<f64>()).sqrt();
        let tangent=[raw[0]/length,raw[1]/length,raw[2]/length];
        let projected=[1.-tangent[0]*tangent[0],-tangent[0]*tangent[1],-tangent[0]*tangent[2]];
        let length=(projected.iter().map(|v|v*v).sum::<f64>()).sqrt();
        for control in 0..2 {
            let radius=profile.control_points[control][0]-4.;
            let expected=std::array::from_fn::<_,3,_>(|k|center[k]+radius*projected[k]/length);
            let delta=std::array::from_fn(|k|(1.-f)*sections[station].control_points[control][k]
                +f*sections[station+1].control_points[control][k]-expected[k]);
            assert!(norm(delta)<=bound,"station={station}, sample={sample}");
        }
    }}
    #[cfg(feature="transport")]
    {
        let result=crate::transport::dispatch(value_codec::json!({"op":"surface_progressive_sweep_level",
            "profiles":[profile],"path":path,"scale":scale,"twist":twist,
            "frame_axis":axis,"frame_normal":normal,"normal":[1.,0.,0.],
            "orientation":"authored","spacing":"parameter","initial_sections":5,"max_sections":33,
            "max_deviation":2.,"preview_sections":33})).unwrap();
        assert_eq!(result["report"]["continuousBound"],true);
        assert_eq!(result["report"]["accepted"],true);
        assert_eq!(result["report"]["continuousErrorScope"],"retained-patches-relative-to-original-profile-transport");
        assert!(result["report"]["continuousErrorUpper"].as_f64().unwrap()<=2.);
    }

}

#[cfg(test)]
#[test]
fn closed_rotating_authored_axis_uses_shared_original_source_work() {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let profile=crate::primitives::circle([4.,0.,0.],[0.,1.,0.],0.2).unwrap();
    let mut axis=path.clone();
    for p in &mut axis.control_points {let x=p[0];p[0]=-p[1]/4.;p[1]=x/4.;}
    let normal=constant_vector_law([0.,0.,1.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[0.,0.,1.],orientation:Orientation::Fixed,spacing:Spacing::Parameter,
        initial_sections:17,max_sections:65,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let original=sweep.authored_control_trajectory(0,[0.,0.03125],10000).unwrap();
    assert_eq!(original.status,Status::Certified,"{original:?}");
    assert!(original.jet.is_some());
    let original_short=sweep.authored_control_trajectory(0,[0.,0.03125],original.cells-1).unwrap();
    assert_eq!(original_short.status,Status::Unresolved);assert!(original_short.jet.is_none());
    let proof=patch_error(&sweep,33,10000,1000000).unwrap();
    assert!(proof.within_budget,"{proof:?}");
    assert!(proof.cells<=10000);
    assert!(!patch_error(&sweep,33,proof.cells-1,1000000).unwrap().within_budget);
    let level=sweep.level(33).unwrap();assert!(level.report.continuous_bound&&level.report.accepted);
    let arc_opts=Options {spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},..opts};
    let arc=Sweep::new(&profile,&path,&scale,&twist,arc_opts).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let arc_proof=patch_error(&arc,33,10000,1000000).unwrap();
    assert!(arc_proof.within_budget,"{arc_proof:?}");
    assert!(arc_proof.cells<=10000);
    let arc_short=patch_error(&arc,33,arc_proof.cells-1,1000000).unwrap();
    assert!(!arc_short.within_budget && arc_short.error_upper.is_none());
    let arc_level=arc.level(33).unwrap();assert!(arc_level.report.continuous_bound&&arc_level.report.accepted);
    let law=|controls:Vec<Vec<f64>>|Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
        control_points:controls,weights:vec![1.;3],periodic:false};
    let joint_scale=law(vec![vec![1.,0.,0.],vec![1.02,0.,0.],vec![1.,0.,0.]]);
    let joint_twist=law(vec![vec![0.;3],vec![0.03,0.,0.],vec![0.;3]]);
    let axes=law(vec![vec![1.;3],vec![1.05,0.95,1.],vec![1.;3]]);
    let center=law(vec![vec![0.;3],vec![0.01,0.02,0.],vec![0.;3]]);
    let joint=Sweep::new(&profile,&path,&joint_scale,&joint_twist,arc_opts).unwrap()
        .with_frame_laws(&axis,&normal).unwrap().with_affine_laws(&axes,&center).unwrap();
    let joint_proof=patch_error(&joint,33,10000,1000000).unwrap();
    assert!(joint_proof.within_budget,"{joint_proof:?}");
    assert!(!patch_error(&joint,33,joint_proof.cells-1,1000000).unwrap().within_budget);
    assert!(!joint.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let retained=arc.sections(33).unwrap().0;
    let upper=arc_proof.error_upper.unwrap();
    for station in 0..32 {for sample in 0..=16 {
        let f=sample as f64/16.;let t=(station as f64+f)/32.;
        let quadrant=((4.*t).floor() as usize).min(3);
        let q=4.*t-quadrant as f64;let w=path.weights[1];
        let den=(1.-q).powi(2)+2.*w*q*(1.-q)+q*q;
        let c=((1.-q).powi(2)+2.*w*q*(1.-q))/den;
        let s=(2.*w*q*(1.-q)+q*q)/den;
        let (x,y)=match quadrant {0=>(c,s),1=>(-s,c),2=>(-c,-s),_=>(s,-c)};
        let length=(x*x+y*y).sqrt();let (x,y)=(x/length,y/length);
        let angle=2.*std::f64::consts::PI*t;
        for (control,p) in profile.control_points.iter().enumerate() {
            // Independent original rational frame and circle arc-length pose.
            let expected=[4.*angle.cos()+(p[0]-4.)*x-p[1]*y,
                4.*angle.sin()+(p[0]-4.)*y+p[1]*x,p[2]];
            let a=&retained[station].control_points[control];
            let b=&retained[station+1].control_points[control];
            assert!(norm(std::array::from_fn(|k|(1.-f)*a[k]+f*b[k]-expected[k]))<=upper);
        }
    }}
}

#[test]
fn closed_fixed_authored_arc_length_charges_the_copied_endpoint() {
    // C0 corners need length-Lipschitz position, not a global path C2 premise.
    let path=Curve {degree:1,knots:vec![0.,0.,0.25,0.5,0.75,1.,1.],
        control_points:vec![vec![4.,0.,0.],vec![4.,4.,0.],vec![0.,4.,0.],vec![0.;3],vec![4.,0.,0.]],
        weights:vec![1.;5],periodic:false};
    let profile=crate::primitives::line([4.1,0.,0.],[4.2,0.,0.]).unwrap();
    let axis=constant_vector_law([0.,0.,1.]).unwrap();
    let normal=constant_vector_law([1.,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let opts=Options {normal:[1.,0.,0.],orientation:Orientation::Fixed,
        spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:5,max_sections:33,max_deviation:2.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let proof=patch_error(&sweep,33,10000,1000000).unwrap();
    assert!(proof.within_budget,"{proof:?}");
    assert!(proof.original_section_endpoint_error_upper.is_some());
    let short=patch_error(&sweep,33,proof.cells-1,1000000).unwrap();
    assert!(!short.within_budget && short.error_upper.is_none());
    let level=sweep.level(33).unwrap();
    assert!(level.report.closed_path && level.report.continuous_bound && level.report.accepted);
    let mut near_axis=axis.clone();
    near_axis.control_points.last_mut().unwrap()[0]=1e-12;
    let near=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_frame_laws(&near_axis,&normal).unwrap();
    let charged=patch_error(&near,33,10000,1000000).unwrap();
    assert!(charged.within_budget,"{charged:?}");
    assert!(charged.original_section_endpoint_error_upper.unwrap()>1e-13);
    assert!(!near.certify_closed_authored_frame_smoothness(2,10000,1000000).unwrap().closed_source_frame_smoothness_certified);
    let circle=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let circular=Sweep::new(&profile,&circle,&scale,&twist,opts).unwrap().with_frame_laws(&axis,&normal).unwrap();
    let circular_proof=patch_error(&circular,33,10000,1000000).unwrap();
    assert!(circular_proof.within_budget,"{circular_proof:?}");
    assert!(circular_proof.cells<=10000);
    let circular_level=circular.level(33).unwrap();
    assert!(circular_level.report.continuous_bound && circular_level.report.accepted);
}

#[cfg(test)]
#[test]
fn closed_guided_arc_length_bound_charges_copied_seam_and_shared_budget(){
 let path=Curve {degree:7,knots:[vec![2.;8],vec![5.;8]].concat(),control_points:vec![vec![0.,0.,0.],vec![1.,0.,0.],vec![2.,1.,0.],vec![3.,3.,0.],vec![-3.,3.,0.],vec![-2.,1.,0.],vec![-1.,0.,0.],vec![0.,0.,0.]],weights:vec![1.;8],periodic:false};
 let mut guide=path.clone();guide.knots=[vec![-3.;8],vec![7.;8]].concat();for p in &mut guide.control_points {p[2]=1.;}
 let profile=crate::primitives::line([0.,0.,0.1],[0.,0.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
 spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
 let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&guide).unwrap();
 let proof=sweep.guided_patch_error_bound(33,10000,1000000).unwrap();
 assert_eq!(proof.status,Status::Certified,"{proof:?}");
 assert!(proof.error_upper.unwrap()>0.&&proof.error_upper.unwrap()<=2.);
 assert!(proof.cells>0&&proof.cells<=10000);
 let short=sweep.guided_patch_error_bound(33,proof.cells-1,1000000).unwrap();
 assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[cfg(test)]
#[test]
fn circular_closed_guided_arc_length_bound_charges_copied_seam_and_shared_budget(){
 let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
 let guide=crate::primitives::circle([0.,0.,1.],[0.,0.,1.],4.).unwrap();
 let profile=crate::primitives::line([4.,0.,0.1],[4.,0.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let opts=Options {normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,
 spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
 let sweep=Sweep::new(&profile,&path,&scale,&twist,opts).unwrap().with_orientation_guide(&guide).unwrap();
 let proof=sweep.guided_patch_error_bound(33,10000,1000000).unwrap();
 assert_eq!(proof.status,Status::Certified,"{proof:?}");
 assert!(proof.error_upper.unwrap()>0.&&proof.error_upper.unwrap()<=2.);
 assert!(proof.cells>0&&proof.cells<=10000);
 let short=sweep.guided_patch_error_bound(33,proof.cells-1,1000000).unwrap();
 assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

#[cfg(test)]
#[test]
fn closed_planar_rmf_arc_length_error_owns_actual_identity_and_copied_seam(){
 let circle=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 let mut conic=circle.clone();for w in &mut conic.weights {if *w!=1. {*w=0.5;}}
 let profile=crate::primitives::line([1.,0.,0.1],[1.,0.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let opts=Options{normal:[0.,0.,1.],orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
 let nonaxial=Sweep::new(&profile,&circle,&scale,&twist,Options{normal:[1.,0.,1.],..opts}).unwrap();
 assert!(!nonaxial.sections_with_frame_identity(33).unwrap().3);
 let refused=nonaxial.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
 assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
 for path in [&circle,&conic] {
  let sweep=Sweep::new(&profile,path,&scale,&twist,opts).unwrap();
  assert!(sweep.sections_with_frame_identity(33).unwrap().3);
  let proof=sweep.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(proof.status,Status::Certified,"{proof:?}");assert!(proof.within_budget);
  assert!(proof.cells>0&&proof.cells<=10000);
  let short=sweep.rmf_planar_patch_error_bound(33,proof.cells-1,1000000).unwrap();
  assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
  let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.125,-0.25,0.]).unwrap();
  let turning=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
  let joint=Sweep::new(&profile,path,&scale,&turning,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
  let law_proof=joint.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(law_proof.status,Status::Certified,"{law_proof:?}");assert!(law_proof.within_budget);
  assert!(law_proof.cells<=10000);

 }
}

#[test]
fn closed_fixed_normal_arc_error_charges_original_images_and_budget(){
 let circle=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 let mut conic=circle.clone();for w in &mut conic.weights {if *w!=1. {*w=0.5;}}
 let profile=crate::primitives::line([1.,0.,0.1],[1.,0.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.125,-0.25,0.]).unwrap();
 let turning=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
 for path in [&circle,&conic] {for normal in [[0.,0.,1.],[1.,0.,1.]] {for joint in [false,true] {
  let opts=Options{normal,orientation:Orientation::FixedNormal,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
  let base=Sweep::new(&profile,path,&scale,if joint {&turning}else{&twist},opts).unwrap();
  let sweep=if joint {base.with_affine_laws(&axes,&center).unwrap()}else{base};
  let proof=sweep.fixed_normal_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(proof.status,Status::Certified,"normal={normal:?} joint={joint}: {proof:?}");
  assert_eq!(proof.within_budget,proof.error_upper.unwrap()<=opts.max_deviation);
  if normal==[0.,0.,1.] {assert!(proof.within_budget);}
  assert!(proof.cells>0&&proof.cells<=10000);
  let strict=Sweep::new(&profile,path,&scale,if joint {&turning}else{&twist},Options{max_deviation:f64::MIN_POSITIVE,..opts}).unwrap();
  let strict=if joint {strict.with_affine_laws(&axes,&center).unwrap()}else{strict};
  let strict_proof=strict.fixed_normal_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(strict_proof.status,Status::Certified);assert!(!strict_proof.within_budget);
  let short=sweep.fixed_normal_patch_error_bound(33,proof.cells-1,1000000).unwrap();
  assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
 }}}
}

#[test]
fn closed_corrected_planar_arc_error_charges_original_phase_and_budget(){
 let circle=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 let mut conic=circle.clone();for w in &mut conic.weights {if *w!=1. {*w=0.5;}}
 let profile=crate::primitives::line([1.,0.,0.1],[1.,0.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();let twist=constant_vector_law([0.;3]).unwrap();
 let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.125,-0.25,0.]).unwrap();
 let turning=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
 for path in [&circle,&conic] {for normal in [[0.,0.,1.]] {for joint in [false,true] {
  let opts=Options{normal,orientation:Orientation::CorrectedFrenet,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
  let base=Sweep::new(&profile,path,&scale,if joint {&turning}else{&twist},opts).unwrap();
  let sweep=if joint {base.with_affine_laws(&axes,&center).unwrap()}else{base};
  let level=sweep.preview_at(33).unwrap();
  assert!(level.report.continuous_bound&&level.report.accepted);
  let proof=corrected_patch_error_with_length(&sweep,33,10000,1000000,None).unwrap();
  assert_eq!(proof.status,Status::Certified,"normal={normal:?} joint={joint}: {proof:?}");
  assert_eq!(proof.within_budget,proof.error_upper.unwrap()<=opts.max_deviation);
  if normal==[0.,0.,1.] {assert!(proof.within_budget);}
  assert!(proof.cells>0&&proof.cells<=10000);
  let strict=Sweep::new(&profile,path,&scale,if joint {&turning}else{&twist},Options{max_deviation:f64::MIN_POSITIVE,..opts}).unwrap();
  let strict=if joint {strict.with_affine_laws(&axes,&center).unwrap()}else{strict};
  let strict_proof=corrected_patch_error_with_length(&strict,33,10000,1000000,None).unwrap();
  assert_eq!(strict_proof.status,Status::Certified);assert!(!strict_proof.within_budget);
  let short=corrected_patch_error_with_length(&sweep,33,proof.cells-1,1000000,None).unwrap();
  assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
 }}}
 let opts=Options{normal:[1.,0.,1.],orientation:Orientation::CorrectedFrenet,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:2.};
 let nonaxial=Sweep::new(&profile,&circle,&scale,&twist,opts).unwrap();
 let refused=corrected_patch_error_with_length(&nonaxial,33,10000,1000000,None).unwrap();
 assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
 let parameter=Sweep::new(&profile,&circle,&scale,&twist,Options{normal:[0.,0.,1.],spacing:Spacing::Parameter,..opts}).unwrap();
 let refused=corrected_patch_error_with_length(&parameter,33,10000,1000000,None).unwrap();
 assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
}

#[test]
fn closed_nonaxial_corrected_arc_owns_exact_original_plane(){
 let mut path=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 for p in &mut path.control_points {let x=p[0];let y=p[1];*p=vec![x,-x,y];}
 let profile=crate::primitives::line([1.,-1.,0.1],[1.,-1.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();
 let twist=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
 let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.125,-0.25,0.]).unwrap();
 let opts=Options{normal:[1.,1.,0.],orientation:Orientation::CorrectedFrenet,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:4.};
 for conic in [false,true] {
  let mut source=path.clone();if conic {for w in &mut source.weights {if *w!=1. {*w=0.5;}}}
  let sweep=Sweep::new(&profile,&source,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
  let proof=corrected_patch_error_with_length(&sweep,33,10000,1000000,None).unwrap();
  assert_eq!(proof.status,Status::Certified,"{proof:?}");assert!(proof.within_budget,"{proof:?}");
  assert!(proof.cells>0&&proof.cells<=10000);
  let short=corrected_patch_error_with_length(&sweep,33,proof.cells-1,1000000,None).unwrap();
  assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
  let mut damaged=source.clone();damaged.control_points[2][0]=damaged.control_points[2][0].next_up();
  let wrong=Sweep::new(&profile,&damaged,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
  let refused=corrected_patch_error_with_length(&wrong,33,10000,1000000,None).unwrap();
  assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
 }
}

#[test]
fn closed_nonaxial_rmf_arc_charges_actual_holonomy_and_original_plane(){
 let mut path=crate::primitives::circle([0.;3],[0.,0.,1.],1.).unwrap();
 for p in &mut path.control_points {let x=p[0];let y=p[1];*p=vec![x,-x,y];}
 let profile=crate::primitives::line([1.,-1.,0.1],[1.,-1.,0.2]).unwrap();
 let scale=constant_vector_law([1.,0.,0.]).unwrap();
 let twist=crate::primitives::line([0.;3],[std::f64::consts::TAU,0.,0.]).unwrap();
 let axes=constant_vector_law([2.,3.,1.]).unwrap();let center=constant_vector_law([0.125,-0.25,0.]).unwrap();
 let opts=Options{normal:[1.,1.,0.],orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength{tolerance:0.001,max_cells:100000},initial_sections:5,max_sections:33,max_deviation:4.};
 for conic in [false,true] {
  let mut source=path.clone();if conic {for w in &mut source.weights {if *w!=1. {*w=0.5;}}}
  let sweep=Sweep::new(&profile,&source,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
  assert!(!sweep.sections_with_frame_identity(33).unwrap().3);
  let proof=sweep.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(proof.status,Status::Certified,"{proof:?}");assert!(proof.within_budget,"{proof:?}");
  assert!(proof.cells>0&&proof.cells<=10000);
  let short=sweep.rmf_planar_patch_error_bound(33,proof.cells-1,1000000).unwrap();
  assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
  let mut damaged=source.clone();damaged.control_points[2][0]=damaged.control_points[2][0].next_up();
  let wrong=Sweep::new(&profile,&damaged,&scale,&twist,opts).unwrap().with_affine_laws(&axes,&center).unwrap();
  let refused=wrong.rmf_planar_patch_error_bound(33,10000,1000000).unwrap();
  assert_eq!(refused.status,Status::Unresolved);assert!(refused.error_upper.is_none());
 }
}

#[test]
fn closed_spatial_rmf_arc_error_owns_source_transport_laws_and_retained_poles(){
    let vertices:[[f64;3];4]=[[1.,0.,0.],[0.,1.,1.],[-1.,0.,0.],[0.,-1.,0.5]];
    let tangents:[[f64;3];4]=[[0.,0.25,0.25],[-0.25,0.,-0.25],[0.,-0.25,0.25],[0.25,0.125,-0.25]];
    let mut poles=vec![vertices[0].to_vec()];
    for i in 0..4 {let j=(i+1)%4;
        poles.push((0..3).map(|k|vertices[i][k]+tangents[i][k]).collect());
        poles.push((0..3).map(|k|vertices[j][k]-tangents[j][k]).collect());poles.push(vertices[j].to_vec());}
    let path=Curve {degree:3,knots:vec![0.,0.,0.,0.,0.25,0.25,0.25,0.5,0.5,0.5,0.75,0.75,0.75,1.,1.,1.,1.],control_points:poles,weights:vec![1.;13],periodic:false};
    let profile=crate::primitives::line([1.1,0.,0.],[1.2,0.,0.]).unwrap();
    let scale=constant_vector_law([1.1,0.,0.]).unwrap();
    let twist=constant_vector_law([0.125,0.,0.]).unwrap();
    let axis=constant_vector_law([1.,1.25,0.75]).unwrap();
    let center=constant_vector_law([0.01,-0.02,0.03]).unwrap();
    let options=Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.01,max_cells:100000},initial_sections:5,max_sections:17,max_deviation:3.};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_affine_laws(&axis,&center).unwrap();
    let length=crate::curve_measure::divide_by_length(&path,16,0.01,100000).unwrap();
    assert!(length.within_tolerance);
    let proof=rmf_planar_patch_error_with_length(&sweep,17,100000,1000000,Some(&length)).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    assert!(proof.error_upper.unwrap().is_finite() && proof.within_budget,"{proof:?}");
    eprintln!("closed spatial retained upper={} work={}",proof.error_upper.unwrap(),proof.cells);
    let dense=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.0001,max_cells:100000},max_sections:129,max_deviation:0.25,..options}).unwrap().with_affine_laws(&axis,&center).unwrap();
    let fine=dense.rmf_spatial_patch_error_bound(129,4096,100000,1000000).unwrap();
    assert_eq!(fine.status,Status::Certified,"{fine:?}");
    assert!(fine.within_budget);
    assert!(fine.error_upper.unwrap()<proof.error_upper.unwrap()*0.5,"{fine:?}");
    eprintln!("dense spatial upper={} cells={}",fine.error_upper.unwrap(),fine.cells);
    let request=value_codec::json!({"op":"surface_progressive_sweep_spatial_rmf_error","profile":profile,"path":path,"scale":scale,"twist":twist,
        "axis_scale":axis,"center_law":center,"normal":[1.,0.,0.],"orientation":"rmf","spacing":"arc_length",
        "length_tolerance":0.0001,"length_max_cells":100000,"initial_sections":5,"max_sections":129,"max_deviation":0.25,
        "preview_sections":129,"transport_steps":4096,"maxCells":100000,"maxProducts":1000000});
    let public=crate::transport::dispatch(request.clone()).unwrap();
    assert_eq!(public["continuousBound"],value_codec::json!(true));
    assert_eq!(public["withinBudget"],value_codec::json!(true));
    assert_eq!(public["errorUpper"].as_f64(),fine.error_upper);
    assert_eq!(public["solidCertified"],value_codec::json!(false));
    let mut short_request=request.clone();short_request["maxCells"]=value_codec::json!(fine.cells-1);
    let refused=crate::transport::dispatch(short_request).unwrap();
    assert_eq!(refused["continuousBound"],value_codec::json!(false));assert!(refused["patches"].is_null());
    let controlled=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.0001,max_cells:100000},max_sections:129,max_deviation:0.25,..options}).unwrap()
        .with_affine_laws(&axis,&center).unwrap().with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let level=controlled.preview_at(129).unwrap();
    assert!(level.report.accepted && level.report.continuous_bound);
    assert_eq!(level.report.continuous_error_upper,fine.error_upper);
    let mut level_request=request.clone();
    level_request["op"]=value_codec::json!("surface_progressive_sweep_level");
    level_request["profiles"]=value_codec::json!([profile]);
    level_request["rmf_transport_steps"]=value_codec::json!(4096);
    level_request["error_max_cells"]=value_codec::json!(100000);
    level_request["error_max_products"]=value_codec::json!(1000000);
    let configured=crate::transport::dispatch(level_request.clone()).unwrap();
    assert_eq!(configured["report"]["continuousBound"],value_codec::json!(true));
    assert_eq!(configured["report"]["accepted"],value_codec::json!(true));
    assert_eq!(configured["report"]["continuousErrorUpper"].as_f64(),fine.error_upper);
    let mut incomplete=level_request.clone();incomplete["error_max_cells"]=value_codec::json!(null);
    assert!(crate::transport::dispatch(incomplete).is_err());
    let mut wrong=level_request;wrong["rmf_transport_steps"]=value_codec::json!(4095);
    assert!(crate::transport::dispatch(wrong).is_err());
    let circle=crate::primitives::circle([1.,0.,0.],[0.,1.,1.],0.05).unwrap();
    let tube=Sweep::new(&circle,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.0001,max_cells:100000},max_sections:129,max_deviation:0.25,..options}).unwrap()
        .with_affine_laws(&axis,&center).unwrap().with_spatial_rmf_error_limits(4096,100000,1000000).unwrap().preview_at(129).unwrap();
    assert!(tube.report.accepted && tube.report.continuous_bound,"{:?}",tube.report);
    let regular=tube.certify_retained_regularity(100000).unwrap();
    assert!(regular.spanwise_regular,"{regular:?}");
    assert!(regular.cells>0);
    let incomplete=tube.certify_retained_regularity(regular.cells-1).unwrap();
    assert!(!incomplete.spanwise_regular && !incomplete.unresolved_patches.is_empty());
    eprintln!("spatial closed-profile patches={} regularity cells={} error={:?}",tube.patches.len(),regular.cells,tube.report.continuous_error_upper);
    let no_work=Sweep::new(&profile,&path,&scale,&twist,Options {spacing:Spacing::ArcLength {tolerance:0.0001,max_cells:100000},max_sections:129,max_deviation:0.25,..options}).unwrap()
        .with_affine_laws(&axis,&center).unwrap().with_spatial_rmf_error_limits(4096,0,1000000).unwrap().preview_at(129).unwrap();
    assert!(!no_work.report.accepted && !no_work.report.continuous_bound);
    let fine_short=dense.rmf_spatial_patch_error_bound(129,4096,fine.cells-1,1000000).unwrap();
    assert_eq!(fine_short.status,Status::Unresolved);assert!(fine_short.error_upper.is_none());
    assert!(dense.rmf_spatial_patch_error_bound(129,4095,100000,1000000).is_err());
    // Independent numerical reference: explicit Bernstein positions plus
    // RK4 Bishop normals; fine polygonal length is validation, never proof.
    let normals=super::rmf_transport::tests::oracle(&path,4096);
    let position=|u:f64| -> [f64;3] {
        let segment=((u*4.).floor() as usize).min(3);let t=u*4.-segment as f64;
        let p=&path.control_points[segment*3..segment*3+4];
        std::array::from_fn(|k|(1.-t).powi(3)*p[0][k]+3.*t*(1.-t).powi(2)*p[1][k]+3.*t*t*(1.-t)*p[2][k]+t.powi(3)*p[3][k])
    };
    let points:Vec<_>=(0..=4096).map(|i|position(i as f64/4096.)).collect();
    let mut cumulative=vec![0.];for i in 0..4096{cumulative.push(cumulative[i]+norm(sub(points[i+1],points[i])));}
    let total=*cumulative.last().unwrap();
    let end=*normals.last().unwrap();let holonomy=((end[2]-end[1])/2f64.sqrt()).atan2(end[0]);
    assert!(holonomy.abs()>1e-3);
    for checked in [&proof,&fine] {for patch in checked.patches.as_ref().unwrap(){for a in 0..=8 {for q in [0.,0.5,1.] {
        let low=patch.knots_v[patch.degree_v];let high=patch.knots_v[patch.control_points[0].len()];
        let station=low+(high-low)*a as f64/8.;let target=station*total;
        let i=cumulative.partition_point(|v|*v<=target).saturating_sub(1).min(4095);
        let f=((target-cumulative[i])/(cumulative[i+1]-cumulative[i])).clamp(0.,1.);
        let u=(i as f64+f)/4096.;let source=position(u);
        let segment=((u*4.).floor() as usize).min(3);let t=u*4.-segment as f64;
        let p=&path.control_points[segment*3..segment*3+4];
        let velocity:V=std::array::from_fn(|k|3.*(1.-t).powi(2)*(p[1][k]-p[0][k])+6.*t*(1.-t)*(p[2][k]-p[1][k])+3.*t*t*(p[3][k]-p[2][k]));
        let tangent=velocity.map(|x|x/norm(velocity));
        let n:V=std::array::from_fn(|k|(1.-f)*normals[i][k]+f*normals[i+1][k]);
        let phase=0.125+holonomy*station;
        let cross=math_core::cross(tangent,n);let dot=math_core::dot(tangent,n);
        let normal:V=std::array::from_fn(|k|n[k]*phase.cos()+cross[k]*phase.sin()+tangent[k]*dot*(1.-phase.cos()));
        let side=math_core::cross(tangent,normal);
        let x=1.1*(0.1+0.1*q)+0.01;
        let expected:V=std::array::from_fn(|k|source[k]+x*normal[k]-0.02*side[k]+0.03*tangent[k]);
        let actual=patch.evaluate(q,station).unwrap().point;
        assert!(norm(std::array::from_fn(|k|actual[k]-expected[k]))<=checked.error_upper.unwrap());
    }}}}
    let preview=sweep.preview_at(17).unwrap();
    assert!(preview.report.continuous_bound && preview.report.accepted,"{:?}",preview.report);
    let short=rmf_planar_patch_error_with_length(&sweep,17,proof.cells-1,1000000,Some(&length)).unwrap();
    assert_eq!(short.status,Status::Unresolved);assert!(short.error_upper.is_none());
}

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


#[test]
fn open_spatial_rmf_original_transport_full_retained_error() {
    let path=crate::paths::bezier(vec![vec![0.,0.,0.],vec![0.,0.,1.],vec![1.,0.,2.],vec![0.,1.,3.],vec![0.,0.,4.],vec![0.,0.,5.]],None).unwrap();
    let profile=crate::paths::bezier(vec![vec![0.05,0.,0.],vec![0.1,0.,0.]],None).unwrap();
    let scale=super::constant_vector_law([1.1,0.,0.]).unwrap();
    let twist=super::constant_vector_law([0.125,0.,0.]).unwrap();
    let axes=super::constant_vector_law([1.,1.25,0.75]).unwrap();
    let center=super::constant_vector_law([0.01,-0.02,0.03]).unwrap();
    let options=super::Options {normal:[1.,0.,0.],orientation:Orientation::RotationMinimizing,
        spacing:Spacing::ArcLength {tolerance:0.0001,max_cells:100000},initial_sections:129,max_sections:129,max_deviation:0.25};
    let sweep=Sweep::new(&profile,&path,&scale,&twist,options).unwrap().with_affine_laws(&axes,&center).unwrap()
        .with_spatial_rmf_error_limits(4096,100000,1000000).unwrap();
    let level=sweep.preview_at(129).unwrap();
    assert!(level.report.accepted && level.report.continuous_bound,"{:?}",level.report);
    assert!(!level.report.closed_path);
    let bound=sweep.rmf_spatial_patch_error_bound(129,4096,100000,1000000).unwrap();
    assert_eq!(bound.status,Status::Certified);
    assert!(bound.error_upper.unwrap()<=0.25);
    let refused=sweep.rmf_spatial_patch_error_bound(129,4096,0,1000000).unwrap();
    assert_eq!(refused.status,Status::Unresolved);
    assert!(refused.error_upper.is_none());
    eprintln!("open spatial RMF error={:?} cells={}",bound.error_upper,bound.cells);
}

#[test]
fn closed_contact_arc_length_requires_owned_original_homothety_and_copied_seam_bound() {
    for (radius,guide_radius) in [(4.,4.25),(3.,3.25)] {
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],radius).unwrap();
    let guide=crate::primitives::circle([0.;3],[0.,0.,1.],guide_radius).unwrap();
    let profile=crate::primitives::line([guide_radius,0.,0.],[guide_radius+0.25,0.,0.]).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    let sweep=Sweep::new(&profile,&path,&scale,&twist,Options {normal:[0.,0.,1.],
        orientation:Orientation::RotationMinimizing,spacing:Spacing::ArcLength {tolerance:0.001,max_cells:100000},
        initial_sections:17,max_sections:65,max_deviation:2.}).unwrap().with_contact_guide(&guide,0.).unwrap();
    let proof=sweep.contact_patch_error_bound(17,10000,1000000).unwrap();
    assert_eq!(proof.status,Status::Certified,"{proof:?}");
    assert!(proof.error_upper.unwrap().is_finite());
    assert!(proof.endpoint_contour_error_upper.is_some());
    let short=sweep.contact_patch_error_bound(17,proof.cells-1,1000000).unwrap();
    assert_eq!(short.status,Status::Unresolved);
    assert!(short.error_upper.is_none());
    let zero=sweep.contact_patch_error_bound(17,0,1000000).unwrap();
    assert_eq!(zero.status,Status::Unresolved);
    assert!(zero.error_upper.is_none());
    }
}

#[test]
fn closed_contact_65_sections_share_original_value_work_for_outer_and_hole() {
    let profiles=[crate::primitives::circle([4.,0.,0.],[0.,1.,0.],0.25).unwrap(),
        crate::primitives::circle([4.,0.,0.],[0.,-1.,0.],0.125).unwrap()];
    let path=crate::primitives::circle([0.;3],[0.,0.,1.],4.).unwrap();
    let guide=crate::primitives::circle([0.;3],[0.,0.,1.],4.25).unwrap();
    let scale=constant_vector_law([1.,0.,0.]).unwrap();
    let twist=constant_vector_law([0.;3]).unwrap();
    for spacing in [Spacing::Parameter,Spacing::ArcLength {tolerance:0.001,max_cells:100000}] {
        let sweep=MultiSweep::new(&profiles,&path,&scale,&twist,Options {normal:[0.,0.,1.],
            orientation:Orientation::RotationMinimizing,spacing,initial_sections:65,max_sections:65,max_deviation:2.})
            .unwrap().with_contact_guide(&guide,0,0.).unwrap();
        let level=sweep.preview_at(65).unwrap();
        assert!(level.report.continuous_bound,"{spacing:?}: {:?}",level.report);
        assert!(level.report.accepted,"{spacing:?}: {:?}",level.report);
        assert!(level.report.error_certificate_cells<=10000);
        assert!(level.report.continuous_error_upper.unwrap()<=2.);
        let individual=sweep.sweeps[0].contact_patch_error_bound(65,10000,1000000).unwrap();
        assert_eq!(individual.status,Status::Certified);
        assert!(individual.error_upper.unwrap()<=2.);
        assert_eq!(sweep.sweeps[0].contact_patch_error_bound(65,individual.cells-1,1000000).unwrap().status,Status::Unresolved);
        assert_eq!(sweep.sweeps[0].contact_patch_error_bound(65,0,1000000).unwrap().status,Status::Unresolved);
    }
}
