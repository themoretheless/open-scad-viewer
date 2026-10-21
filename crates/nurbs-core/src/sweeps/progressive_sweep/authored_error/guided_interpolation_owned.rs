//! Guided interpolation owned: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn guided_section_interpolation_mode_with_pose(
    sweep: &Sweep<'_>, count: usize, max_cells: usize, mode:OriginalTransport,shared_pose:Option<&mut SharedArcPose>,
) -> Result<SectionInterpolationReport> {
    let contact=mode==OriginalTransport::Contact;
    let corrected=matches!(mode,OriginalTransport::CorrectedStraight|OriginalTransport::CorrectedCollinear|OriginalTransport::CorrectedPlanar);
    let rmf_straight=matches!(mode,OriginalTransport::RmfStraight|OriginalTransport::CorrectedStraight|OriginalTransport::RmfCollinear|OriginalTransport::CorrectedCollinear);
    let rmf_planar=matches!(mode,OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar);
    let collinear=matches!(mode,OriginalTransport::RmfCollinear|OriginalTransport::CorrectedCollinear);
    let fixed=mode==OriginalTransport::Fixed||rmf_straight;
    let fixed_normal=mode==OriginalTransport::FixedNormal||rmf_planar;
    let regular_corrected=mode==OriginalTransport::CorrectedRegular;
    let frenet=mode==OriginalTransport::Frenet||regular_corrected;
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
        refinement_error_upper: None,
        endpoint_displacement_upper: None,
        cap_endpoint_displacement_upper: None,
        reason: Some("mode-not-guided"),
    };
    let guide=sweep.orientation_guide;
    if corrected {
        if sweep.options.orientation!=Orientation::CorrectedFrenet||sweep.frame_laws.is_some()||guide.is_some(){return Ok(out);}
        if sweep.path.periodic||(path_is_closed(sweep.path)?
            &&!(mode==OriginalTransport::CorrectedPlanar&&sweep.contact_point.is_none())){
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
        if sweep.options.orientation!=if regular_corrected {Orientation::CorrectedFrenet}else{Orientation::Frenet} || sweep.frame_laws.is_some() || guide.is_some(){return Ok(out);}
    } else if guide.is_none(){return Ok(out);}
    if regular_corrected {
        out.reason=Some("corrected-regular-phase-correspondence-unproved");
        if sweep.options.spacing!=Spacing::Parameter||sweep.contact_point.is_some(){return Ok(out);}
        let eligibility=corrected_regular::certify(sweep.path,max_cells)?;
        out.cells+=eligibility.cells;
        if !eligibility.certified{out.reason=eligibility.reason;return Ok(out);}
    }
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
        if sweep.path.periodic||path_is_closed(sweep.path)?{
            out.reason=Some(if corrected {"corrected-closed-frame-correspondence-unproved"}else{"rmf-closed-correction-correspondence-unproved"});
            return Ok(out);
        }
        let plane=super::super::source_plane::certify(sweep.path,sweep.options.normal,(max_cells-out.cells) as u64)?;
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
    // Fixed reference frames and constant original law coefficients have one
    // time-independent pose. Literal straight RMF/corrected modes are fixed
    // too; excluding them needlessly exhausted the shared source allowance.
    let constant_pose=fixed&&constant_law(sweep.scale)&&constant_law(sweep.twist)
        &&sweep.affine_laws.is_none_or(|(a,c)|constant_law(a)&&constant_law(c));
    let shared_pose=std::cell::RefCell::new(shared_pose);
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
            let mut values=if let Some(shared)=shared_pose.borrow_mut().as_deref_mut(){
                let source=shared.source_values(interval,||vector_certificate::certify_values_traversal(reference_path,interval,budget-charge,false))?;
                authored_frame_certificate::translated_constant_values_with_source(pose_cache.borrow().as_ref().unwrap(),source)?
            }else{authored_frame_certificate::translated_constant_values(reference_path,pose_cache.borrow().as_ref().unwrap(),interval,budget-charge)?};
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
                return if let Some(shared)=shared_pose.borrow_mut().as_deref_mut(){
                    let source=shared.source_jets(interval,||vector_certificate::certify_traversal(reference_path,interval,budget,false))?;
                    authored_frame_certificate::translated_constant_jets_with_source(reference_path,pose,interval,source)
                }else{authored_frame_certificate::translated_constant_jets(reference_path,pose,interval,budget)};
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
    let initial = if let Some(phase)=principal_phase {principal_initial_coordinates(sweep,phase,max_cells-out.cells)?} else if corrected {seed_initial_coordinates_as(sweep,max_cells-out.cells,Orientation::RotationMinimizing,Orientation::CorrectedFrenet)?} else if rmf_straight||rmf_planar {seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::RotationMinimizing)?} else if fixed {fixed_initial_coordinates(sweep,max_cells-out.cells)?} else if fixed_normal {fixed_normal_initial_coordinates(sweep,max_cells-out.cells)?} else if regular_corrected {seed_initial_coordinates_as(sweep,max_cells-out.cells,Orientation::Frenet,Orientation::CorrectedFrenet)?} else if frenet {seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::Frenet)?} else {guided_initial_coordinates(sweep, max_cells-out.cells)?};
    out.cells += initial.cells;
    if initial.status != Status::Certified {
        out.reason = initial.reason;
        return Ok(out);
    }
    let coordinates = initial.coordinates.unwrap();
    let (sections,closed,_,closed_identity)=sweep.sections_with_frame_identity(count)?;
    // Only RMF applies a closing holonomy correction. Corrected planar
    // sections, including the copied seam, are owned by the original value
    // images and their endpoint displacements below.
    if mode==OriginalTransport::RmfPlanar && closed && !closed_identity {
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
        let open_span=batch.status==Status::Certified
            && original_laws_have_no_interior_knots(sweep,interval,&mut out.cells,max_cells)?;
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
            if open_span {
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
