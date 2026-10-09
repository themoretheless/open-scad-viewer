//! Original parameter and arc-length transport enclosures.
use super::*;
use super::super::rmf_transport;

pub(super) fn guided_section_interpolation_mode(
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

// Position uses original cumulative length; relative pose uses original source
// parameter brackets and independent normalized station-law intervals. The
// interval image remainder needs no unproved inverse-length derivative jets.
pub(super) fn fixed_normal_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation(sweep,count,max_cells,shared_length,OriginalTransport::FixedNormal,None)
}
pub(super) fn original_frame_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation_with_rmf_steps(sweep,count,max_cells,shared_length,mode,shared_guide,512,None)
}
pub(super) fn original_frame_arc_length_section_interpolation_with_rmf_steps(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mut mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>,rmf_steps:usize,shared_rmf:Option<&rmf_transport::OriginalRmfTransportReport>)->Result<SectionInterpolationReport>{
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
        owned_transport=rmf_transport::certify_original_rmf_transport_shared(sweep.path,sweep.options.normal,
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
            OriginalTransport::RmfSpatial=>rmf_transport::relative_values(spatial_transport.as_ref().unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
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

