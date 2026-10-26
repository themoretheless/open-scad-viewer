//! Arc frame owned: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn original_frame_arc_length_section_interpolation_with_rmf_steps(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>,mode:OriginalTransport,shared_guide:Option<&crate::curve_measure::DivisionReport>,rmf_steps:usize,shared_rmf:Option<&super::super::rmf_transport::OriginalRmfTransportReport>,mut shared_pose:Option<&mut SharedArcPose>)->Result<SectionInterpolationReport>{
    let mut out=SectionInterpolationReport {status:Status::Unresolved,cells:0,error_upper:None,refinement_error_upper:None,
        endpoint_displacement_upper:None,cap_endpoint_displacement_upper:None,reason:Some("arc-length-correspondence-unproved")};
    let phase=if matches!(mode,OriginalTransport::Contact|OriginalTransport::Guided) {
        let Spacing::ArcLength {max_cells:constructor_budget,..}=sweep.options.spacing else{return Ok(out);};
        let phase=super::super::arc_guide::certify(sweep.path,sweep.orientation_guide.unwrap(),max_cells.min(constructor_budget))?;
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
    // Corrected transport has no closing holonomy correction. Regular
    // spatial sources additionally prove original C2 endpoint identity below.
    // Actual principal-normal sign choices and copied endpoint remain in the
    // retained-versus-original image displacement bound, not assumed exact.
    // Strict Frenet has no holonomy correction. Prove the original closed
    // positional/tangent seam independently; copied-section displacement is
    // still enclosed by the original full frame images below.
    let closed_frenet=closed&&mode==OriginalTransport::Frenet
        &&sweep.options.orientation==Orientation::Frenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none();
    let closed_corrected=closed&&matches!(mode,OriginalTransport::CorrectedPlanar|OriginalTransport::CorrectedRegular)
        &&sweep.options.orientation==Orientation::CorrectedFrenet&&sweep.frame_laws.is_none()
        &&sweep.orientation_guide.is_none()&&sweep.contact_point.is_none();
    // Periodic corrected-regular sources use the same original active-basis
    // C2 seam/C4 interior certificate below as parameter spacing. Arc length
    // remains independently enclosed, including inverse station brackets.
    if (sweep.path.periodic&&!((closed_rmf&&mode==OriginalTransport::RmfSpatial)
        ||(closed_corrected&&mode==OriginalTransport::CorrectedRegular)||closed_frenet
        ||(sweep.path.periodic&&closed_fixed_normal)))
        ||(closed&&!closed_guided&&!closed_rmf&&!closed_fixed_normal&&!closed_corrected&&!closed_frenet){
        if closed&&mode==OriginalTransport::CorrectedPlanar{out.reason=Some("corrected-closed-frame-correspondence-unproved");}
        return Ok(out);
    }
    let Spacing::ArcLength {tolerance,max_cells:constructor_budget}=sweep.options.spacing else{return Ok(out);};
    if max_cells==0{return Ok(out);}
    if sweep.path.periodic&&closed_fixed_normal {
        // Periodicity alone does not prove the original endpoint frame seam.
        // Charge exact active-basis C1 identity before enclosing inverse length.
        let seam=super::super::endpoint_jets::certify(sweep.path,1,(max_cells-out.cells) as u64)?;
        out.cells+=seam.exact_work as usize;
        if !seam.certified||out.cells>=max_cells {
            out.reason=Some("periodic-fixed-normal-arc-original-seam-unproved");return Ok(out);
        }
    }
    if closed_frenet {
        // Arc position needs original C0, not an unneeded parametric-C1
        // expansion. Clamped endpoints are their original Cartesian poles;
        // exact binary equality is sufficient and every comparison is charged.
        let mut seam=closed_frenet_position_seam(sweep.path,max_cells-out.cells)?;
        out.cells+=seam.exact_work as usize;
        if !seam.certified&&out.cells<max_cells {
            let stronger=super::super::endpoint_jets::certify(sweep.path,1,(max_cells-out.cells) as u64)?;
            out.cells+=stronger.exact_work as usize;seam.certified=stronger.certified;
        }
        if !seam.certified&&!sweep.path.periodic&&out.cells<max_cells {
            let direction=super::super::endpoint_jets::certify_direction(sweep.path,0,true,false,(max_cells-out.cells) as u64)?;
            out.cells+=direction.exact_work as usize;
            seam.certified=direction.certified;
        }
        if !seam.certified||out.cells>=max_cells {
            out.reason=Some("closed-frenet-arc-original-seam-unproved");return Ok(out);
        }
    }
    // Charge the exact original-coefficient plane premise to the same owner
    // as inverse length and frame bounds. Unproved input never admits RMF.
    if mode==OriginalTransport::CorrectedPlanar&&(sweep.options.orientation!=Orientation::CorrectedFrenet
        ||sweep.frame_laws.is_some()||sweep.orientation_guide.is_some()||sweep.contact_point.is_some()){
        out.reason=Some("corrected-affine-frame-phase-unproved");return Ok(out);
    }
    if matches!(mode,OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar)&&!original_planar_rmf(sweep.path,sweep.options.normal){
        let plane=super::super::source_plane::certify(sweep.path,sweep.options.normal,max_cells as u64)?;
        out.cells+=plane.exact_work as usize;
        // Explicit planar certificates retain their original plane premise.
        // The default RMF dispatcher may try its independently charged
        // spatial owner after this refusal; this owner never changes mode.
        if !plane.proved{out.reason=plane.reason;return Ok(out);}
        if out.cells==max_cells{out.reason=Some("rmf-original-frame-work-unproved");return Ok(out);}
    }
    if mode==OriginalTransport::CorrectedRegular {
        if sweep.options.orientation!=Orientation::CorrectedFrenet||sweep.frame_laws.is_some()
            ||sweep.orientation_guide.is_some()||sweep.contact_point.is_some(){return Ok(out);}
        let eligibility=if let Some(pose)=shared_pose.as_deref_mut(){pose.regular(sweep.path,max_cells-out.cells)?}else{corrected_regular::certify(sweep.path,max_cells-out.cells)?};
        out.cells+=eligibility.cells;
        if !eligibility.certified{out.reason=eligibility.reason;return Ok(out);}
    }
    let owned_transport;
    let spatial_transport=if mode==OriginalTransport::RmfSpatial {
        let report=if let Some(shared)=shared_rmf {shared} else {
        let remaining=max_cells-out.cells;
        // Exact coefficient work and interval cells share this owner.
        owned_transport=super::super::rmf_transport::certify_original_rmf_transport_shared_for_spacing(sweep.path,sweep.options.normal,
            rmf_steps,remaining,closed,sweep.options.spacing)?;
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
    let tolerance=if mode==OriginalTransport::Contact {
        phase.as_ref().and_then(|p|p.scale_upper).filter(|scale|*scale>1.).map_or(tolerance,|scale|(tolerance/scale).next_down())
    }else{tolerance};
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
        guide_source.reserve(guide_division.points.len());
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
        OriginalTransport::CorrectedRegular=>seed_initial_coordinates_as(sweep,max_cells-out.cells,Orientation::Frenet,Orientation::CorrectedFrenet)?,
        OriginalTransport::Frenet=>seed_initial_coordinates(sweep,max_cells-out.cells,Orientation::Frenet)?,
        OriginalTransport::Guided|OriginalTransport::Contact=>guided_initial_coordinates(sweep,max_cells-out.cells)?,
        _=>return Ok(out),
    };out.cells+=initial.cells;
    let Some(qs)=initial.coordinates.filter(|_|initial.status==Status::Certified)else{out.reason=initial.reason;return Ok(out);};
    let anchor=if mode==OriginalTransport::Contact {
        let certificate=super::super::contact_anchor::certify(sweep,max_cells-out.cells)?;
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
    let inverse_speed=if closed_rmf||closed_fixed_normal||closed_frenet||matches!(mode,OriginalTransport::RmfSpatial|OriginalTransport::CorrectedPlanar|OriginalTransport::CorrectedRegular|OriginalTransport::Guided|OriginalTransport::Contact) {
        let (cells,lower,reason)=if mode==OriginalTransport::CorrectedRegular&&shared_pose.is_some(){let (cells,lower)=shared_pose.as_deref_mut().unwrap().speed(sweep.path,max_cells-out.cells)?;(cells,lower,None)}
            else{let (cover,lower)=crate::curve_regularity::inspect_speed(sweep.path,max_cells-out.cells)?;(cover.cells,lower,cover.reason)};
        out.cells+=cells;
        let Some(lower)=lower else{out.reason=reason.or(Some("corrected-arc-speed-lower-unproved"));return Ok(out);};
        Some(lower)
    }else{None};
    let mut source=Vec::with_capacity(division.points.len());
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
    let mut position_curvature:Option<Option<f64>>=None;
    let mut original_refinement_error=0_f64;
    for i in 0..count-1{
        let station=[i as f64/(count-1) as f64,(i+1) as f64/(count-1) as f64];
        let path_interval=[source[i][0],source[i+1][1]];
        let values=match mode {
            OriginalTransport::RmfSpatial if shared_pose.is_some()=>shared_pose.as_deref_mut().unwrap().rmf_values(spatial_transport.as_ref().unwrap(),sweep,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::RmfSpatial=>super::super::rmf_transport::relative_values(spatial_transport.as_ref().unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::CorrectedPlanar if principal_phase.is_some()=>authored_frame_certificate::certify_planar_principal_relative_values(sweep.path,sweep.options.normal,principal_phase.unwrap(),sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::RmfPlanar|OriginalTransport::CorrectedPlanar=>authored_frame_certificate::certify_proved_planar_relative_values(sweep.path,sweep.options.normal,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::FixedNormal=>authored_frame_certificate::certify_fixed_normal_relative_values(sweep.path,sweep.options.normal,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::CorrectedRegular if shared_pose.is_some()=>shared_pose.as_deref_mut().unwrap().values(sweep,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::Frenet if closed_frenet=>authored_frame_certificate::certify_frenet_relative_values_with_binormal_identity(sweep.path,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::Frenet|OriginalTransport::CorrectedRegular=>authored_frame_certificate::certify_frenet_relative_values(sweep.path,sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,station,max_cells-out.cells)?,
            OriginalTransport::Guided=>authored_frame_certificate::certify_path_guide_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,path_interval,[guide_source[i][0],guide_source[i+1][1]],station,same_phase,max_cells-out.cells)?,
            OriginalTransport::Contact=>authored_frame_certificate::certify_contact_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                sweep.twist,sweep.affine_laws,&qs,anchor.unwrap(),path_interval,[guide_source[i][0],guide_source[i+1][1]],station,same_phase,max_cells-out.cells)?,
            _=>return Ok(out),
        };
        out.cells+=values.cells;
        let Some(values)=values.values.filter(|_|values.status==Status::Certified)else{
            #[cfg(test)] eprintln!("arc original frame refusal: path={path_interval:?}, station={station:?}, cells={}, reason={:?}",out.cells,values.reason);
            out.reason=values.reason.or(Some("arc-length-frame-value-unproved"));return Ok(out);};
        // Complete original C4 eligibility above makes Q(s) twice
        // differentiable through source knots. Compare actual endpoints
        // separately, then apply the vector linear-interpolation remainder.
        let residual=division.points[i].residual_upper.max(division.points[i+1].residual_upper);
        let h=I::point(station[1]).sub(I::point(station[0]))?;
        let mut coarse_position=I::point(division.total.bounds[1]).mul(h)?.div(I::point(2.))?.add(I::point(residual))?;
        let mut relative_errors=Vec::with_capacity(qs.len());
        let mut coarse_errors=Vec::with_capacity(qs.len());
        for (j,value) in values.iter().enumerate(){
            let mut delta=[I::point(0.);3];
            for k in 0..3 {
                let a=I::point(sections[i].control_points[j][k]).sub(I::new(division.points[i].point_bounds[k][0],division.points[i].point_bounds[k][1])?)?;
                let b=I::point(sections[i+1].control_points[j][k]).sub(I::new(division.points[i+1].point_bounds[k][0],division.points[i+1].point_bounds[k][1])?)?;
                delta[k]=I::new(value[k][0],value[k][1])?.sub(I::new(a.lo.min(b.lo),a.hi.max(b.hi))?)?;
            }
            let relative=crate::numerics::interval_vec3::norm(delta)?;
            relative_errors.push(relative);
            coarse_errors.push(relative.add(coarse_position)?.hi);
        }
        for upper in &coarse_errors {original_refinement_error=original_refinement_error.max(*upper);}
        // Keep the strongest proved source enclosure even when the source
        // alone fits: later retained-wall reconstruction must add its own
        // displacement to this bound under the same caller tolerance.
        // CorrectedRegular already owns a shared arc-jet/curvature proof
        // below; avoid duplicating its original-source work across profiles.
        // New geometric arc joins use an original arc-station chord cover.
        // The policy depends on the fresh source proof, never on the requested
        // tolerance: tightening admission cannot change the geometric bound.
        let use_arc_chord=spatial_transport.is_some_and(|r|r.used_geometric_arc_join_premises)||mode==OriginalTransport::Guided&&same_phase;
        if use_arc_chord {
            if out.cells==max_cells {out.reason=Some("arc-chord-work-limit");return Ok(out);}
            let (cells,chord)=arc_chord::upper(sweep.path,path_interval,station,division.total.bounds,residual,spatial_transport.is_some(),max_cells-out.cells)?;
            out.cells+=cells;
            if chord.is_none() && out.cells==max_cells {out.reason=Some("arc-chord-work-limit");return Ok(out);}
            if let Some(chord)=chord {
                coarse_position=I::new(0.,coarse_position.hi.min(chord))?;
                for (upper,relative) in coarse_errors.iter_mut().zip(&relative_errors) {
                    *upper=relative.add(coarse_position)?.hi;
                }
            }
        } else if mode!=OriginalTransport::CorrectedRegular {
            if position_curvature.is_none() {
                // A fixed local ceiling keeps the proof route independent of
                // the caller's total budget; clipping a fractional allowance
                // could skip this work on a one-short replay and certify via
                // a cheaper, weaker fallback instead.
                let allowance=(max_cells-out.cells).min(1024);
                let proof=arc_position::certify(sweep.path,allowance)?;
                out.cells+=proof.cells;
                if proof.curvature_upper.is_none() && proof.cells==allowance && allowance<1024 {
                    out.reason=Some("arc-position-curvature-work-exhausted");
                    return Ok(out);
                }
                position_curvature=Some(proof.curvature_upper);
            }
            if let Some(Some(curvature))=position_curvature {
                let arc=I::point(division.total.bounds[1]).mul(h)?;
                let refined=I::point(curvature).mul(arc)?.mul(arc)?.div(I::point(8.))?.add(I::point(residual))?;
                coarse_position=I::new(0.,coarse_position.hi.min(refined.hi))?;
                for (upper,relative) in coarse_errors.iter_mut().zip(&relative_errors) {
                    *upper=relative.add(coarse_position)?.hi;
                }
            }
        }
        // A complete image cover already inside the requested tolerance needs
        // no additional derivative work. The decomposition still has its own
        // independently charged final error and admission check.
        let needs_second=coarse_errors.iter().any(|upper|*upper>sweep.options.max_deviation);
        if mode==OriginalTransport::CorrectedRegular&&needs_second&&out.cells<max_cells {
            let speed=inverse_speed.unwrap();
            let relative=if let Some(pose)=shared_pose.as_deref_mut(){pose.arc_jets(sweep,&qs,path_interval,station,division.total.bounds,speed,max_cells-out.cells)?}
                else{authored_frame_certificate::certify_c4_frenet_arc_relative_trajectories(
                sweep.path,sweep.scale,sweep.twist,sweep.affine_laws,&qs,path_interval,station,
                division.total.bounds,speed,max_cells-out.cells)?};
            out.cells+=relative.cells;
            if relative.status==Status::Certified&&relative.single_span {
                let (cells,curvature)=if let Some(pose)=shared_pose.as_deref_mut(){pose.curvature(sweep.path,path_interval,speed,max_cells-out.cells)?}
                    else{let p=vector_certificate::certify_traversal(sweep.path,path_interval,max_cells-out.cells,false)?;
                        let upper=if p.status==Status::Certified{let a=p.second.unwrap().map(|v|I{lo:v[0],hi:v[1]});Some(crate::numerics::interval_vec3::norm(a)?.div(I::point(speed).mul(I::point(speed))?)?.hi)}else{None};(p.cells,upper)};
                out.cells+=cells;
                if let Some(curvature)=curvature {
                    let curvature=I::point(curvature);
                    let h=I::point(station[1]).sub(I::point(station[0]))?;
                    let arc=I::new(division.total.bounds[0],division.total.bounds[1])?.mul(h)?;
                    let position=curvature.mul(arc.mul(arc)?)?.div(I::point(8.))?;
                    let mut endpoints=Vec::with_capacity(2);
                    for e in 0..2 {
                        let endpoint=if let Some(pose)=shared_pose.as_deref_mut(){pose.values(sweep,&qs,source[i+e],[station[e];2],max_cells-out.cells)?}
                            else{authored_frame_certificate::certify_frenet_relative_values(
                            sweep.path,sweep.scale,sweep.twist,sweep.affine_laws,&qs,source[i+e],
                            [station[e];2],max_cells-out.cells)?};
                        out.cells+=endpoint.cells;
                        if endpoint.status!=Status::Certified{break;}
                        endpoints.push(endpoint.values.unwrap());
                    }
                    if endpoints.len()==2 {
                        for (j,jet) in relative.jets.unwrap().into_iter().enumerate(){
                            let mut displacement=0_f64;
                            for e in 0..2 {
                                let mut delta=[I::point(0.);3];
                                for k in 0..3 {
                                    let retained=I::point(sections[i+e].control_points[j][k]).sub(I::new(division.points[i+e].point_bounds[k][0],division.points[i+e].point_bounds[k][1])?)?;
                                    delta[k]=I::new(endpoints[e][j][k][0],endpoints[e][j][k][1])?.sub(retained)?;
                                }
                                displacement=displacement.max(crate::numerics::interval_vec3::norm(delta)?.add(I::point(division.points[i+e].residual_upper))?.hi);
                            }
                            endpoint_upper=endpoint_upper.max(displacement);
                            let dd=[
                                I::new(jet.second[0][0],jet.second[0][1])?,
                                I::new(jet.second[1][0],jet.second[1][1])?,
                                I::new(jet.second[2][0],jet.second[2][1])?,
                            ];
                            let remainder=crate::numerics::interval_vec3::norm(dd)?.mul(h.mul(h)?)?.div(I::point(8.))?;
                            let refined=remainder.add(position)?.add(I::point(displacement))?.hi;
                            // Both covers enclose the same original field. A
                            // numerically broad second jet cannot worsen the
                            // previously valid whole-image certificate.
                            error=error.max(refined.min(coarse_errors[j]));
                        }
                        continue;
                    }
                }
            }
        }
        if mode==OriginalTransport::CorrectedRegular&&needs_second&&out.cells==max_cells {
            out.reason=Some("arc-second-remainder-work-unproved");return Ok(out);
        }
        let _residual=division.points[i].residual_upper.max(division.points[i+1].residual_upper);
        let position=coarse_position;
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
    if matches!(mode,OriginalTransport::Guided|OriginalTransport::Contact){
        let mut caps=[0_f64;2];
        for (endpoint,index) in [0,count-1].into_iter().enumerate(){
            // Normalized length maps endpoints exactly to original source
            // endpoints. Enclose original pose and compare with actual stored
            // controls; positive unchanged rational weights extend pole bounds
            // to the entire contour. No interior inverse is reused here.
            let station=[endpoint as f64;2];
            let values=if mode==OriginalTransport::Contact {
                authored_frame_certificate::certify_contact_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                    sweep.twist,sweep.affine_laws,&qs,anchor.unwrap(),source[index],guide_source[index],station,same_phase,max_cells-out.cells)?
            }else{
                authored_frame_certificate::certify_path_guide_relative_values(sweep.path,sweep.orientation_guide.unwrap(),sweep.scale,
                    sweep.twist,sweep.affine_laws,&qs,source[index],guide_source[index],station,same_phase,max_cells-out.cells)?
            };
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
    out.status=Status::Certified;out.error_upper=Some(error);out.endpoint_displacement_upper=Some(endpoint_upper);
    out.refinement_error_upper=(mode!=OriginalTransport::CorrectedRegular).then_some(original_refinement_error);
    out.reason=None;Ok(out)
}

pub(super) fn closed_frenet_position_seam(path:&Curve,max_cells:usize)->Result<super::super::endpoint_jets::Report> {
    path.validate()?;
    let mut out=super::super::endpoint_jets::Report {certified:false,exact_work:0};
    if path.periodic {return Ok(out);}
    let [a,b]=path.domain();
    for (actual,expected) in path.knots[..path.degree+1].iter().map(|&u|(u,a))
        .chain(path.knots[path.control_points.len()..].iter().map(|&u|(u,b)))
        .chain(path.control_points[0].iter().zip(path.control_points.last().unwrap()).map(|(&p,&q)|(p,q))) {
        if out.exact_work==max_cells as u64 {return Ok(out);}
        out.exact_work+=1;
        if actual!=expected {return Ok(out);}
    }
    out.certified=true;Ok(out)
}

pub(super) fn fixed_normal_arc_length_section_interpolation(sweep:&Sweep<'_>,count:usize,max_cells:usize,shared_length:Option<&crate::curve_measure::DivisionReport>)->Result<SectionInterpolationReport>{
    original_frame_arc_length_section_interpolation(sweep,count,max_cells,shared_length,OriginalTransport::FixedNormal,None)
}
