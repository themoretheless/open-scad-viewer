//! Owned progressive body: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn progressive_profile_body_internal(
    loops:&[Vec<Curve>],path:&Curve,scale:&Curve,twist:&Curve,
    affine:Option<(&Curve,&Curve)>,frames:Option<(&Curve,&Curve)>,
    guidance:Option<(&Curve,Option<(usize,f64)>)>,options:nurbs_core::progressive_sweep::Options,
    correction:Option<EndpointCapCorrection>,profile_correction:Option<EndpointCapCorrection>,
    rmf_policy:Option<(usize,usize,usize)>,annular:Option<crate::retained_annular_cap::reference_filling::Limits>,automatic_caps:bool,
)->Result<ProgressiveBodyEvidence>{
    use nurbs_core::progressive_sweep::{Sweep, approximate_profiles};
    if let Some(limits)=annular {annular_caps::validate_limits(limits)?;}
    if rmf_policy.is_some() && (frames.is_some() || guidance.is_some()) {
        return Err(err("Spatial RMF policy excludes authored frames and guides"));
    }
    if (frames.is_some() || guidance.is_some()) && affine.is_none() || frames.is_some() && guidance.is_some() {
        return Err(err("Progressive body frame/guide configuration requires affine laws and one frame source"));
    }
    if options.max_sections > 1025
        || loops.is_empty()
        || loops.len() > 16
        || loops.iter().any(Vec::is_empty)
    {
        return Err(err(
            "Progressive body needs 1..16 nonempty loops and at most1025 sections",
        ));
    }
    let profiles = loops.iter().flatten().cloned().collect::<Vec<_>>();
    let spans = profiles
        .iter()
        .map(|c| c.decompose().map(|s| s.len()))
        .collect::<nurbs_core::Result<Vec<_>>>()?
        .into_iter()
        .sum::<usize>();
    if spans == 0 || spans > 64 {
        return Err(err("Progressive body exceeds64 section spans"));
    }
    let closed=nurbs_core::progressive_sweep::path_is_closed(path)?;
    if annular.is_some() && !automatic_caps && !closed && loops.len()!=2 {
        return Err(err("Annular progressive caps require one outer ring and one hole"));
    }
    let cap_faces = if closed {0} else if annular.is_some() && !automatic_caps {spans} else {2};
    let options = nurbs_core::progressive_sweep::Options {
        max_sections: options
            .max_sections
            .min((MAX_FACES - cap_faces) / spans + 1),
        ..options
    };
    let approximation = if let Some((steps,cells,products)) = rmf_policy {
        nurbs_core::progressive_sweep::approximate_spatial_rmf_profiles(
            &profiles,path,scale,twist,affine,options,steps,cells,products,
        )?
    } else if let Some((guide, anchor)) = guidance {
        let (axes, center) = affine.unwrap();
        if let Some((index, parameter)) = anchor {
            nurbs_core::progressive_sweep::approximate_contact_profiles(
                &profiles, path, scale, twist, guide, index, parameter, axes, center, options,
            )?
        } else {
            nurbs_core::progressive_sweep::approximate_guided_profiles(
                &profiles, path, scale, twist, guide, axes, center, options,
            )?
        }
    } else if let Some((longitudinal, transverse)) = frames {
        let (axes, center) = affine.unwrap();
        nurbs_core::progressive_sweep::approximate_authored_profiles(
            &profiles,
            path,
            scale,
            twist,
            longitudinal,
            transverse,
            axes,
            center,
            options,
        )?
    } else if let Some((axes, center)) = affine {
        nurbs_core::progressive_sweep::approximate_affine_profiles(
            &profiles, path, scale, twist, axes, center, options,
        )?
    } else {
        approximate_profiles(&profiles, path, scale, twist, options)?
    };
    let report = approximation.levels.last().unwrap();
    if !report.accepted {
        if rmf_policy.is_some() {
            return Err(err(&format!("Progressive body continuous retained-patch error is unproved or exceeds budget: {}",
                report.error_certificate_reason.unwrap_or("configured RMF error budget"))));
        }
        if report.sampled_control_deviation <= options.max_deviation {
            return Err(err(&format!("Progressive body continuous retained-patch error is unproved or exceeds budget: {}",
                report.error_certificate_reason.unwrap_or("configured continuous error budget"))));
        }
        return Err(err("Progressive body misses sampled refinement budget"));
    }
    let transported = if let Some((guide, anchor)) = guidance {
        let sweep =
            nurbs_core::progressive_sweep::MultiSweep::new(&profiles, path, scale, twist, options)?;
        let sweep = if let Some((index, parameter)) = anchor {
            sweep.with_contact_guide(guide, index, parameter)?
        } else {
            sweep.with_orientation_guide(guide)?
        };
        let (axes, center) = affine.unwrap();
        sweep
            .with_affine_laws(axes, center)?
            .sections_at(report.sections)?
    } else {
        profiles
            .iter()
            .map(|p| {
                let sweep = Sweep::new(p, path, scale, twist, options)?;
                let sweep = if let Some((axes, center)) = affine {
                    sweep.with_affine_laws(axes, center)?
                } else {
                    sweep
                };
                let sweep = if let Some((longitudinal, transverse)) = frames {
                    sweep.with_frame_laws(longitudinal, transverse)?
                } else {
                    sweep
                };
                sweep.sections_at(report.sections)
            })
            .collect::<nurbs_core::Result<Vec<_>>>()?
    };
    let mut sections = (0..report.sections)
        .map(|station| {
            let mut first = 0;
            loops
                .iter()
                .map(|ring| {
                    let section = transported[first..first + ring.len()]
                        .iter()
                        .map(|p| p[station].clone())
                        .collect::<Vec<_>>();
                    first += ring.len();
                    section
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let profile_correction_error_upper = if let Some(budget) = profile_correction {
        // Flatten only for the native shared-budget repair; preserve loop ownership.
        let flat = sections
            .iter()
            .map(|station| station.iter().flatten().cloned().collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let repaired = nurbs_core::circle_sweep_repair::repair(
            &flat,
            budget.quantum,
            budget.tolerance,
            Some(budget.max_work as f64),
        )?;
        let upper = repaired.wall_displacement_upper.ok_or_else(|| {
            err(&format!(
                "Progressive profile circle correction refused: {}",
                repaired.reason
            ))
        })?;
        let corrected = repaired
            .sections
            .ok_or_else(|| err("Progressive profile circle correction has no geometry"))?;
        for (station, curves) in sections.iter_mut().zip(corrected) {
            let mut at = 0;
            for ring in station {
                for curve in ring {
                    *curve = curves[at].clone();
                    at += 1;
                }
            }
        }
        Some(upper)
    } else {
        None
    };
    let cap_correction_error_upper = if let Some(correction) = correction {
        if report.closed_path {
            return Err(err("Closed progressive body has no caps to correct"));
        }
        let mut work = 0_u64;
        let mut displacement = 0_f64;
        for end in [false, true] {
            let station = if end { sections.len() - 1 } else { 0 };
            let normal = if let Some((axis, _)) = frames {
                let domain = axis.domain();
                axis.evaluate(if end { domain[1] } else { domain[0] })?
                    .point
            } else {
                let domain = path.domain();
                let parameter = if end
                    && options.orientation != nurbs_core::progressive_sweep::Orientation::Fixed
                {
                    domain[1]
                } else {
                    domain[0]
                };
                path.evaluate(parameter)?
                    .d1
                    .ok_or_else(|| err("Cap correction needs an endpoint tangent"))?
            };
            let axis = nurbs_core::progressive_sweep::constant_vector_law([
                normal[0], normal[1], normal[2],
            ])?;
            let curves = sections[station]
                .iter()
                .flatten()
                .cloned()
                .collect::<Vec<_>>();
            let projected = nurbs_core::section_projection::project_authored_axis(
                &curves,
                &axis,
                0.,
                correction.quantum,
                correction.tolerance,
                correction
                    .max_work
                    .checked_sub(work)
                    .ok_or_else(|| err("Cap correction work exhausted"))?,
            )?;
            work += projected.work;
            if !projected.exact_planar {
                return Err(err(&format!(
                    "Progressive cap correction refused: {}",
                    projected.reason
                )));
            }
            let corrected = projected
                .curves
                .ok_or_else(|| err("Progressive cap correction has no geometry"))?;
            displacement = displacement.max(
                projected
                    .displacement_upper
                    .ok_or_else(|| err("Cap correction displacement unproved"))?,
            );
            let mut at = 0;
            for ring in &mut sections[station] {
                for curve in ring {
                    *curve = corrected[at].clone();
                    at += 1;
                }
            }
        }
        Some(displacement)
    } else {
        None
    };
    let correction_error_upper = match (profile_correction_error_upper, cap_correction_error_upper)
    {
        (Some(a), Some(b)) => Some(
            nurbs_core::numerics::error_upper::add(a, b)
                .ok_or_else(|| err("Progressive correction error overflow"))?,
        ),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    };
    let mut model = if report.closed_path {
        periodic_section_loft(&sections)?
    } else {
        rational_section_loft(&sections)?
    };
    let (use_annular, cap_selection_work) = if automatic_caps
        && !report.closed_path
        && loops.len() == 2
    {
        let (exact, work) =
            crate::sweep_miter_owned::cap_boundaries_exact(&model, annular.unwrap().exact_work)?;
        (!exact, work)
    } else {
        (
            annular.is_some() && !automatic_caps && !report.closed_path,
            0,
        )
    };
    // Preserve accepted legacy layouts. A replacement must fit the same face
    // ceiling; no surface/station is dropped to manufacture room for caps.
    if use_annular && model.faces.len() - 2 + spans > MAX_FACES {
        return Err(err(
            "Automatic annular caps exceed the shared body face budget",
        ));
    }
    let (retained_sections, body_decomposition_error_upper, body_decomposition_products) =
        retained_section_partition(&sections, 1000000)?;
    let retained_caps = if report.closed_path || use_annular {
        None
    } else {
        Some(crate::sweep_retained_caps::inspect(
            &model,
            &[
                retained_sections[0].clone(),
                retained_sections.last().unwrap().clone(),
            ],
            crate::sweep_cap_contacts::Budgets {
                max_walls: 1024,
                max_exact_work: annular.map_or(1000000, |l| l.exact_work) - cap_selection_work,
                max_chart_cells: 1000,
                max_trim_pairs: 100000,
                max_trim_cells: 100000,
                max_trim_domain_cells: 1000000,
            },
            1024,
        )?)
    };
    let make_source = || -> Result<nurbs_core::progressive_sweep::MultiSweep<'_>> {
        let mut source =
            nurbs_core::progressive_sweep::MultiSweep::new(&profiles, path, scale, twist, options)?;
        if let Some((axes, center)) = affine {
            source = source.with_affine_laws(axes, center)?;
        }
        if let Some((axis, normal)) = frames {
            source = source.with_frame_laws(axis, normal)?;
        }
        if let Some((guide, anchor)) = guidance {
            source = if let Some((index, parameter)) = anchor {
                source.with_contact_guide(guide, index, parameter)?
            } else {
                source.with_orientation_guide(guide)?
            };
        }
        if let Some((steps, cells, products)) = rmf_policy {
            source = source.with_spatial_rmf_error_limits(steps, cells, products)?;
        }
        Ok(source)
    };
    let cap_projection = if report.closed_path || use_annular {
        None
    } else {
        let source = make_source()?;
        let sizes = loops.iter().map(Vec::len).collect::<Vec<_>>();
        let caps = [
            model.faces[model.faces.len() - 2].surface.clone(),
            model.faces.last().unwrap().surface.clone(),
        ];
        Some(source.certify_endpoint_cap_projection(&sizes, &caps, 1e-9, 10000, 100000, 1000000)?)
    };
    let mut filled_cap_error_upper = cap_projection.as_ref().and_then(|projection| {
        nurbs_core::sweeps::filled_cap_error::filled_caps(
            &nurbs_core::sweeps::filled_cap_error::Premises {
                ideal_domains_certified: projection
                    .original
                    .domains
                    .ideal_endpoint_domains_certified,
                retained_regions_exact: retained_caps.as_ref().is_some_and(|r| r.exact),
                projection_normal_dots: projection.normal_dots,
                endpoint_error: report.endpoint_contour_error_upper,
                correction: Some(correction_error_upper.unwrap_or(0.)),
                decomposition: body_decomposition_error_upper.map(|e| [e; 2]),
                parallel_planes: [false; 2],
            },
        )
    });
    let retained_walls = crate::sweep_retained_walls::inspect(
        &model,
        &retained_sections,
        report.closed_path,
        1024,
        1000000,
    )?;
    // Preview may itself have been decomposed; keeping its full error and adding
    // constructor extraction is conservative even when both include that term.
    let original_wall_error = if retained_walls.certified {
        report
            .continuous_error_upper
            .zip(body_decomposition_error_upper)
            .and_then(|(a, b)| nurbs_core::numerics::error_upper::add(a, b))
    } else {
        None
    };
    let wall_error = if let Some(correction) = correction_error_upper {
        original_wall_error.and_then(|e| nurbs_core::numerics::error_upper::add(e, correction))
    } else {
        original_wall_error
    };
    let annular_caps = if let Some(limits) = annular.filter(|_| use_annular) {
        let source = make_source()?;
        let sizes = loops.iter().map(Vec::len).collect::<Vec<_>>();
        let assembled = annular_caps::assemble(
            &source,
            &model,
            &sizes,
            options.max_deviation,
            1e-9,
            limits,
            cap_selection_work,
        )?;
        filled_cap_error_upper = assembled.evidence.filling().filled_cap_error_upper;
        model = assembled.model;
        Some(assembled.evidence)
    } else {
        None
    };
    let boundary_error_upper = nurbs_core::sweeps::filled_cap_error::boundary(
        wall_error,
        filled_cap_error_upper,
        report.closed_path,
    );
    let boundary_error_within_budget =
        boundary_error_upper.map(|upper| upper <= options.max_deviation);
    let station_boundary = nurbs_core::sweeps::filled_cap_error::compose_boundary(
        wall_error,
        filled_cap_error_upper,
        report.closed_path,
        Some(options.max_deviation),
    );
    let station_closed = report.closed_path;
    Ok(ProgressiveBodyEvidence {
        model,
        approximation,
        retained_caps,
        cap_projection,
        filled_cap_error_upper,
        cap_correction_error_upper,
        profile_correction_error_upper,
        retained_walls,
        boundary_error_upper,
        boundary_error_within_budget,
        body_decomposition_error_upper,
        body_decomposition_products,
        station_sections: sections,
        station_boundary,
        station_closed,
        station_budget: options.max_deviation,
        annular_caps,
        cap_selection_work,
    })
}
