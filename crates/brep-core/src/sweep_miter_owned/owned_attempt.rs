//! Owned attempt: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn construct_attempt(
    request: &Request<'_>,
    limits: &Limits,
    direct: bool,
    cap_endpoints: Option<[bool; 2]>,
    annular_caps: bool,
    automatic_caps: bool,
) -> Result<ConstructionAttempt> {
    // This private mode defers only the old terminal planar fillings. Wall
    // correction, source correspondence and regularity remain mandatory. The
    // owning annular constructor must derive both new filled-cap bounds before
    // it can expose a complete boundary certificate.
    let mut defer_annular_caps =
        annular_caps && !request.options.closed && request.loops.len() == 2;
    let mut cap_probe_work = 0;
    let planner = if annular_caps {
        sweep_miter_layout::plan_annular
    } else {
        sweep_miter_layout::plan
    };
    let plan = planner(
        request.loops,
        request.points.len(),
        request.options.closed,
        request.options.initial_steps,
        request.options.max_steps,
    )?;
    let total_profiles: usize = request.loops.iter().map(Vec::len).sum();
    let mut profiles = Vec::with_capacity(total_profiles);
    for l in request.loops {
        profiles.extend_from_slice(l);
    }
    let rings = request.loops.iter().map(Vec::len).collect::<Vec<_>>();
    let mut options = request.options;
    options.max_steps = plan.max_steps;
    let constructor = if direct {
        Sweep::new_direct
    } else {
        Sweep::new
    };
    let mut sweep = constructor(
        &profiles,
        request.points,
        request.scale,
        request.twist,
        options,
    )?;
    if let Some((axes, center)) = request.affine {
        sweep = sweep.with_affine_laws(axes, center)?;
    }
    if let Some((axis, normal)) = request.frames {
        sweep = sweep.with_frame_laws(axis, normal)?;
    }
    if let Some(guide) = request.guide {
        sweep = sweep.with_orientation_guide(guide)?;
    }
    let mut levels = Vec::with_capacity(4);
    let mut accepted = None;
    if direct {
        use nurbs_core::construct::curves::paths::{closed_miter_sections, miter_sections};
        let sections = if options.closed {
            closed_miter_sections(
                &profiles,
                request.points,
                options.normal,
                options.miter_limit,
            )
        } else {
            miter_sections(
                &profiles,
                request.points,
                options.normal,
                options.miter_limit,
            )
        }?;
        let level = sweep.preview_retained_at(1, sections)?;
        if level.report.accepted {
            accepted = Some(level.sections);
        }
        levels.push(level.report);
    } else {
        for level in sweep.by_ref() {
            let level = level?;
            if level.report.accepted {
                accepted = Some(level.sections);
            }
            levels.push(level.report);
        }
    }
    let report = levels
        .last()
        .ok_or_else(|| invalid("Progressive miter produced no level"))?;
    if !report.frame_transport_certified {
        return Err(invalid(
            "Progressive miter frame transport could not be proved; review the path, normal and miter limit",
        ));
    }
    if !report.profile_regularity_certified {
        return Err(invalid(
            "Progressive miter profile tangent regularity could not be proved",
        ));
    }
    if report.wall_regularity_certified == Some(false) {
        return Err(invalid(
            "Progressive miter retained wall Jacobian regularity could not be proved",
        ));
    }
    let Some(source_error) = report.certified_error_upper else {
        return Err(invalid(format!(
            "Progressive miter wall error bound could not be proved: {}",
            report
                .error_certificate_reason
                .unwrap_or("unresolved certificate")
        )));
    };
    let original = accepted.ok_or_else(|| {
        invalid("Progressive miter refinement/phase budget not met within the body face budget")
    })?;
    // Explicit caps share their caller-owned correction/work allowance with
    // periodic profile preservation. Circle repair retains its existing order.
    let explicit_profile = if !direct && !options.closed && request.circle.is_none() {
        if let Some(cap) = request.caps.as_ref().filter(|c| c.budget.tolerance > 0.) {
            if let Some(work) = cap
                .budget
                .max_work
                .filter(|w| w.is_finite() && *w >= 1. && *w <= 1000000. && w.fract() == 0.)
            {
                periodic_profile_lattice(
                    &original,
                    2_f64.powi(-40),
                    cap.budget.tolerance.min(options.max_deviation).min(1e-9),
                    work as u64,
                )?
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };
    let (profile_error, profile_work) = explicit_profile
        .as_ref()
        .map_or((0., 0), |(_, e, w)| (*e, *w));
    let correction_source = explicit_profile
        .as_ref()
        .map_or(original.as_slice(), |(s, _, _)| s.as_slice());
    let mut correction = miter_section_correction::correct_selected(
        correction_source,
        request.points,
        options.closed,
        request.circle,
        request.caps.as_ref().map(|c| CapCorrection {
            budget: Budget {
                max_work: c.budget.max_work.map(|w| w - profile_work as f64),
                ..c.budget
            },
            authored_frame: c.authored_frame,
        }),
        request.frames.map(|(axis, _)| axis),
        cap_endpoints,
    )?;
    if explicit_profile.is_some() {
        let repair = correction
            .as_mut()
            .ok_or_else(|| invalid("Explicit periodic cap correction missing"))?;
        let upper = repair
            .wall_displacement_upper
            .and_then(|d| error_upper::add(d, profile_error))
            .ok_or_else(|| invalid("Explicit periodic cap displacement bound unproved"))?;
        if upper > request.caps.as_ref().unwrap().budget.tolerance {
            return Err(invalid(
                "Explicit periodic cap displacement exceeds shared budget",
            ));
        }
        repair.wall_displacement_upper = Some(upper);
        repair.work += profile_work;
    }
    if !direct && options.closed && request.circle.is_none() && request.caps.is_none() {
        if let Some((sections, displacement, work)) = periodic_profile_lattice(
            &original,
            2_f64.powi(-40),
            options.max_deviation.min(1e-9),
            limits.projection_exact_work,
        )? {
            correction = Some(nurbs_core::sweep_section_projection::Report {
                sections: Some(sections),
                wall_displacement_upper: Some(displacement),
                exact_planar_sections: Vec::new(),
                work,
                reason: "bounded-periodic-profile-interpolation",
            });
        }
    }
    if !direct && request.circle.is_none() && request.caps.is_none() && correction.is_none() {
        if let Some((sections, displacement, work)) = rational_periodic_lattice(
            &original,
            2_f64.powi(-40),
            options.max_deviation.min(1e-9),
            limits.projection_exact_work,
        )? {
            correction = Some(nurbs_core::sweep_section_projection::Report {
                sections: Some(sections),
                wall_displacement_upper: Some(displacement),
                exact_planar_sections: Vec::new(),
                work,
                reason: "bounded-rational-periodic-profile-interpolation",
            });
        }
    }
    let retained = correction
        .as_ref()
        .and_then(|c| c.sections.as_deref())
        .unwrap_or(&original);
    let mut sections = sweep_miter_layout::partition(retained, &rings)?;
    let mut model = if options.closed {
        crate::periodic_section_loft(&sections)?
    } else {
        crate::rational_section_loft(&sections)?
    };
    if !direct && !options.closed && request.caps.is_none() {
        if let Some(mut repair) = automatic_cap_correction(
            &model, retained, request.points, request.frames.map(|(a, _)| a),
            options.max_deviation, limits.cap_regions.max_exact_work,
        )? {
            let before = correction.as_ref().and_then(|c| c.wall_displacement_upper).unwrap_or(0.);
            repair.wall_displacement_upper = repair.wall_displacement_upper
                .and_then(|x| error_upper::add(before, x));
            if repair.wall_displacement_upper.is_none() {
                return Err(invalid("Automatic cap correction displacement bound unproved"));
            }
            repair.work += correction.as_ref().map_or(0, |c| c.work);
            sections = sweep_miter_layout::partition(repair.sections.as_deref().unwrap(), &rings)?;
            model = crate::rational_section_loft(&sections)?;
            correction = Some(repair);
        }
    }
    let displacement = correction
        .as_ref()
        .and_then(|c| c.wall_displacement_upper)
        .unwrap_or(0.);
    if automatic_caps && defer_annular_caps {
        let (exact, used) = cap_boundaries_exact(&model, limits.cap_regions.max_exact_work)?;
        cap_probe_work = used;
        defer_annular_caps = !exact;
    }
    let correspondence = sweep_retained_walls::inspect_correspondence(
        &model,
        &sections,
        options.closed,
        limits.correspondence_faces,
        limits.exact_work,
    )?;
    let decomposition = if correspondence.certified {
        Some(0.)
    } else {
        sweep_retained_decomposition::inspect(
            &model,
            &sections,
            options.closed,
            limits.max_products,
            limits.max_faces,
            limits.exact_work,
        )?
        .wall_error_upper
    };
    if correction.is_some() && decomposition.is_none() {
        return Err(invalid(
            "Corrected progressive miter retained wall correspondence unproved",
        ));
    }
    let wall = error_upper::add(source_error, displacement)
        .zip(decomposition)
        .and_then(|(base, extra)| error_upper::add(base, extra));
    if correction.is_some() && !wall.is_some_and(|x| x <= options.max_deviation) {
        let reason =
            "Corrected progressive miter retained wall error exceeds max_deviation or is unproved";
        if wall.is_some() {
            let steps = report.steps;
            return Ok(ConstructionAttempt::Refine {
                levels,
                steps,
                work: correction.as_ref().map_or(0, |c| c.work),
                reason,
            });
        }
        return Err(invalid(reason));
    }
    let caps = if options.closed || defer_annular_caps {
        None
    } else {
        let mut cap_regions = limits.cap_regions;
        cap_regions.max_exact_work -= cap_probe_work;
        let endpoints = [sections[0].clone(), sections.last().unwrap().clone()];
        let exact =
            sweep_retained_caps::inspect(&model, &endpoints, cap_regions, limits.cap_max_edges)?;
        let (regions, decomposition) = if exact.exact {
            (true, Some([0., 0.]))
        } else {
            // An inconclusive exact audit has already spent part of the cap
            // allowance. Approximate decomposition does not receive a new one.
            let mut remaining = cap_regions;
            remaining.max_exact_work = remaining
                .max_exact_work
                .checked_sub(exact.exact_work)
                .ok_or_else(|| invalid("Retained cap audit exceeded its exact-work allowance"))?;
            let r = sweep_retained_decomposition::inspect_caps(
                &model,
                &endpoints,
                remaining,
                limits.max_products,
                limits.cap_max_edges,
            )?;
            (r.certified, r.cap_error_upper)
        };
        if correction.is_some() && !regions {
            return Err(invalid(
                "Corrected progressive miter filled retained cap regions unproved",
            ));
        }
        let domains = sweep.certify_ideal_cap_domains(
            &rings,
            limits.domain_tolerance,
            limits.domain_pairs,
            limits.domain_cells,
            limits.domain_exact_work,
        )?;
        let planes = [
            &model.faces[model.faces.len() - 2].surface,
            &model.faces[model.faces.len() - 1].surface,
        ];
        let projection = sweep.certify_endpoint_cap_projection(
            planes,
            limits.projection_cells,
            limits.projection_exact_work,
        )?;
        let parallel = sweep.certify_endpoint_cap_parallelism(
            planes,
            limits.projection_cells,
            limits.projection_exact_work,
        )?;
        filled_cap_error::filled_caps(&Premises {
            ideal_domains_certified: domains.ideal_cap_domains_certified,
            retained_regions_exact: regions,
            projection_normal_dots: projection.normal_dots,
            endpoint_error: report.endpoint_contour_error_upper,
            correction: Some(displacement),
            decomposition,
            parallel_planes: parallel.parallel.unwrap_or([false; 2]),
        })
    };
    let boundary =
        filled_cap_error::compose_boundary(wall, caps, options.closed, Some(options.max_deviation));
    if boundary.within_budget == Some(false) {
        let steps = report.steps;
        return Ok(ConstructionAttempt::Refine {
            levels,
            steps,
            work: correction.as_ref().map_or(0, |c| c.work),
            reason: "Progressive miter complete boundary error exceeds max_deviation",
        });
    }
    let cap_faces = if options.closed {
        Vec::new()
    } else {
        vec![model.faces.len() - 2, model.faces.len() - 1]
    };
    let wall_charts = crate::sweep_retained_charts::inspect(&model, &cap_faces, limits.wall_cells)?;
    if correction.is_some() && !wall_charts.certified {
        #[cfg(all(test, feature = "codec"))]
        if let Ok(path) = std::env::var("OSV_MITER_UNPROVED_MODEL") {
            std::fs::write(path, value_codec::to_string(&model).unwrap()).unwrap();
        }
        #[cfg(test)]
        eprintln!(
            "Unproved corrected wall charts: cells={} unresolved={:?} proofs={:?}",
            wall_charts.cells,
            wall_charts.unresolved_faces,
            wall_charts
                .charts
                .iter()
                .filter(|(_, r)| r.cells > 256 || !r.certified)
                .collect::<Vec<_>>()
        );
        return Err(invalid(
            "Corrected progressive miter retained wall regularity unproved",
        ));
    }
    let sharp = sweep_miter_layout::sharp_stations(
        sections.len(),
        plan.edges,
        report.steps,
        options.closed,
    )?;
    Ok(ConstructionAttempt::Complete(BoundaryBody {
        model,
        sections,
        source_sections: original,
        correction,
        wall_charts,
        edges: plan.edges,
        max_steps: plan.max_steps,
        levels,
        boundary,
        sharp,
        closed: options.closed,
        budget: options.max_deviation,
        caps_deferred: defer_annular_caps,
        cap_probe_work,
    }))
}
