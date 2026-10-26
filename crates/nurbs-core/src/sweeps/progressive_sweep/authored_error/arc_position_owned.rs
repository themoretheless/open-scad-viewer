//! Arc position owned: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn section_interpolation_with_length(sweep:&Sweep<'_>,count:usize,max_cells:usize,
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

pub(super) fn original_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_arc_length_section_interpolation_with_chord(sweep,count,max_cells,shared_length,None)
}
pub(super) fn original_arc_length_section_interpolation_with_chord(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,shared_chord:Option<&GlobalChord>)->Result<SectionInterpolationReport>{
    let mut out=SectionInterpolationReport {status:Status::Unresolved,cells:0,error_upper:None,refinement_error_upper:None,
        endpoint_displacement_upper:None,cap_endpoint_displacement_upper:None,reason:Some("arc-length-correspondence-unproved")};
    if shared_chord.is_some_and(|c|c.budget_stopped) {out.reason=Some("arc-chord-work-limit");return Ok(out);}
    let path=sweep.path;
    let domain=path.domain();
    // Interior multiplicity > degree permits a jump, invalidating Lipschitz
    // position in cumulative length. No sampled continuity premise is used.
    // A closed Fixed frame has no holonomy correction. The same
    // length-Lipschitz chord bound applies to its last interval; the endpoint
    // loop below charges the copied closing section against the original pose.
    let closed=path_is_closed(path)?;
    let unsupported_closed=closed&&sweep.options.orientation!=Orientation::Fixed;
    if unsupported_closed || path.knots.iter().any(|k|
        *k>domain[0]&&*k<domain[1]&&path.knots.iter().filter(|x|*x==k).count()>path.degree){return Ok(out);}
    let Spacing::ArcLength {tolerance,max_cells:constructor_budget}=sweep.options.spacing else {return Ok(out);};
    if max_cells==0{return Ok(out);}
    if path.periodic || (closed&&sweep.frame_laws.is_none()) {
        // The inverse-length Lipschitz owner needs the actual one-sided
        // original seam, not merely the periodic flag or copied section.
        if sweep.options.orientation!=Orientation::Fixed{return Ok(out);}
        let mut seam=super::super::endpoint_jets::certify(path,1,max_cells as u64)?;
        out.cells+=seam.exact_work as usize;
        // Fixed arc position is length-Lipschitz, not parameter-C1 dependent.
        // A charged exact original C0/positive tangent-direction seam is also
        // sufficient when endpoint parameter speeds differ. Keep the cheaper
        // existing C1 proof and charge a failed proposal before this fallback.
        // Periodic storage keeps its separate original-basis seam owner.
        if !seam.certified&&!path.periodic&&out.cells<max_cells {
            let direction=super::super::endpoint_jets::certify_direction(path,0,true,false,(max_cells-out.cells) as u64)?;
            out.cells+=direction.exact_work as usize;
            seam.certified=direction.certified;
        }
        if !seam.certified||out.cells>=max_cells{
            out.reason=Some(if sweep.frame_laws.is_some(){"periodic-authored-arc-original-seam-unproved"}
                else if path.periodic{"periodic-fixed-arc-original-seam-unproved"}
                else{"arc-length-correspondence-unproved"});return Ok(out);
        }
    }
    let owned;
    let division=if let Some(shared)=shared_length {shared} else {
        owned=crate::curve_measure::divide_by_length(path,count-1,tolerance,(max_cells-out.cells).min(constructor_budget))?;
        out.cells+=owned.cells;&owned
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
    // Original equal rational poles prove a constant relative Fixed pose.
    // Own that proof once per invocation; endpoint positions and inverse-
    // length residuals remain independent, fully charged source queries.
    let mut constant_relative=frame_laws.is_none();
    if constant_relative {
        let mut laws=[Some(sweep.scale),Some(sweep.twist),None,None];
        if let Some((axis,center))=sweep.affine_laws {laws[2]=Some(axis);laws[3]=Some(center);}
        'laws: for law in laws.into_iter().flatten() {for pole in &law.control_points {
            if out.cells==max_cells {return Ok(out);}
            out.cells+=1;
            if pole!=&law.control_points[0] {constant_relative=false;break 'laws;}
        }}
    }
    let constant_values=if constant_relative {
        let batch=authored_frame_certificate::certify_fixed_relative_values(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,&coordinates,[0.,1.],max_cells-out.cells)?;
        out.cells+=batch.cells;
        if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
        batch.values
    }else {None};
    let mut endpoints=vec![vec![0.;coordinates.len()];count];
    let mut endpoint_upper=0_f64;
    for station in 0..count{
        let t=station as f64/(count-1) as f64;
        let point=&division.points[station];
        let batch=if let Some(values)=&constant_values {Some(values.clone())}
        else if let Some((axis,normal))=frame_laws {
            let batch=authored_frame_certificate::certify_control_values(&zero,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,[t,t],max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            let Some(values)=batch.values else {return Ok(out);};Some(values)
        }else {
            let batch=authored_frame_certificate::certify_fixed_relative_values(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,&coordinates,[t,t],max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            let Some(values)=batch.values else {return Ok(out);};Some(values)
        };
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
    let global_chord=if let Some(shared)=shared_chord {shared.bound}
    else if path.control_points.len()==path.degree+1 {
        let global=shared_global_chord(path,count,division.total.bounds,max_cells-out.cells)?;
        out.cells+=global.cells;
        if global.budget_stopped {out.reason=Some("arc-chord-work-limit");return Ok(out);}
        global.bound
    }else {None};
    // Optional original curvature proof improves the positional chord bound.
    // Keep the valid Lipschitz bound on incomplete regularity/curvature work;
    // every attempted cell still consumes this invocation's unchanged budget.
    let coarse=I::point(division.total.bounds[1]).div(I::point((count-1) as f64))?.div(I::point(2.))?;
    let curvature=if global_chord.is_none() && coarse.hi>sweep.options.max_deviation && max_cells>out.cells {
        let available=max_cells-out.cells;
        // A proved constant relative pose needs no later per-station law work.
        let allowance=if constant_values.is_some(){available}else{available/8};
        let proof=arc_position::certify(path,allowance.min(1024))?;
        out.cells+=proof.cells;proof.curvature_upper
    }else {None};
    let mut error=0_f64;
    for station in 0..count-1{
        let interval=[station as f64/(count-1) as f64,(station+1) as f64/(count-1) as f64];
        let width=I::point(interval[1]).sub(I::point(interval[0]))?;
        let arc=I::point(division.total.bounds[1]).mul(width)?;
        let lipschitz=arc.div(I::point(2.))?;
        let mut best=global_chord.map_or(lipschitz.hi,|c|c.min(lipschitz.hi));
        if let Some(curvature)=curvature {
            let quadratic=I::point(curvature).mul(arc)?.mul(arc)?.div(I::point(8.))?;
            best=best.min(quadratic.hi);
        }
        let position=I::new(0.,best)?;
        let batch=if let Some(values)=&constant_values {
            // Equal original poles make the exact relative derivatives zero;
            // interval uncertainty of the constant value is not a derivative.
            Some(authored_frame_certificate::TrajectoriesReport {traversal:interval,status:Status::Certified,cells:0,
                jets:Some(values.iter().copied().map(|value|authored_frame_certificate::FrameJet {
                    value,first:[[0.,0.];3],second:[[0.,0.];3]}).collect()),single_span:true,reason:None})
        }else if let Some((axis,normal))=frame_laws {
            let batch=authored_frame_certificate::certify_control_trajectories(&zero,sweep.scale,sweep.twist,
                axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            Some(batch)
        }else {
            let batch=authored_frame_certificate::certify_fixed_relative_trajectories(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?;
            out.cells+=batch.cells;
            if batch.status!=Status::Certified {out.reason=batch.reason;return Ok(out);}
            Some(batch)
        };
        let values=if batch.as_ref().is_some_and(|batch|!batch.single_span) {
            let values=if let Some((axis,normal))=frame_laws {
                authored_frame_certificate::certify_control_values(&zero,sweep.scale,sweep.twist,
                    axis,normal,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?
            }else {
                authored_frame_certificate::certify_fixed_relative_values(path,sweep.options.normal,sweep.scale,sweep.twist,sweep.affine_laws,&coordinates,interval,max_cells-out.cells)?
            };
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
