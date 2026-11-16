//! Stages of the owned progressive body over plain data.
//! Each stage keeps the original arithmetic, proof budgets and refusal texts.
use super::*;
use nurbs_core::progressive_sweep::{
    self as ps, LevelReport, MultiApproximation, MultiSweep, Options,
};

/// Loops per station, curves per loop.
pub(super) type Sections = Vec<Vec<Vec<Curve>>>;

/// Most stations a progressive body may request.
const MAX_SECTIONS: usize = 1025;
/// Most profile loops (outer ring plus holes).
const MAX_LOOPS: usize = 16;
/// Most Bezier spans over all profile curves.
const MAX_SECTION_SPANS: usize = 64;

/// Request-level refusals, in the original order.
pub(super) fn validate_request(
    laws: &ProgressiveLaws<'_>,
    options: &Options,
    rmf_policy: Option<(usize, usize, usize)>,
    annular: Option<crate::retained_annular_cap::reference_filling::Limits>,
) -> Result<()> {
    let (frames, guidance) = (laws.frames.is_some(), laws.guidance.is_some());
    if let Some(limits) = annular {
        annular_caps::validate_limits(limits)?;
    }
    if rmf_policy.is_some() && (frames || guidance) {
        return Err(err(
            "Spatial RMF policy excludes authored frames and guides",
        ));
    }
    if (frames || guidance) && laws.affine.is_none() || frames && guidance {
        return Err(err(
            "Progressive body frame/guide configuration requires affine laws and one frame source",
        ));
    }
    let loops = laws.loops;
    if options.max_sections > MAX_SECTIONS
        || loops.is_empty()
        || loops.len() > MAX_LOOPS
        || loops.iter().any(Vec::is_empty)
    {
        return Err(err(
            "Progressive body needs 1..16 nonempty loops and at most1025 sections",
        ));
    }
    Ok(())
}

pub(super) fn section_spans(profiles: &[Curve]) -> Result<usize> {
    let spans = profiles
        .iter()
        .map(|c| c.decompose().map(|s| s.len()))
        .collect::<nurbs_core::Result<Vec<_>>>()?
        .into_iter()
        .sum::<usize>();
    if spans == 0 || spans > MAX_SECTION_SPANS {
        return Err(err("Progressive body exceeds64 section spans"));
    }
    Ok(spans)
}

/// Affine laws required by authored frames and guides (checked by `validate_request`).
fn affine_laws<'a>(laws: &ProgressiveLaws<'a>) -> Result<(&'a Curve, &'a Curve)> {
    laws.affine.ok_or_else(|| {
        err("Progressive body frame/guide configuration requires affine laws and one frame source")
    })
}

pub(super) fn approximate(
    profiles: &[Curve],
    laws: &ProgressiveLaws<'_>,
    options: Options,
    rmf_policy: Option<(usize, usize, usize)>,
) -> Result<MultiApproximation> {
    let ProgressiveLaws {
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
        ..
    } = *laws;
    Ok(if let Some((steps, cells, products)) = rmf_policy {
        ps::approximate_spatial_rmf_profiles(
            profiles,
            ps::SweepLaws {
                path,
                scale,
                twist,
                affine,
            },
            options,
            ps::RmfTransport {
                steps,
                max_cells: cells,
                max_products: products,
            },
        )?
    } else if let Some((guide, anchor)) = guidance {
        let (axes, center) = affine_laws(laws)?;
        let laws = ps::ContactSweepLaws {
            path,
            scale,
            twist,
            guide,
            axis_scale: axes,
            center,
        };
        if let Some((index, parameter)) = anchor {
            ps::approximate_contact_profiles(
                profiles,
                laws,
                ps::ContactAnchor {
                    profile_index: index,
                    parameter,
                },
                options,
            )?
        } else {
            ps::approximate_guided_profiles(profiles, laws, options)?
        }
    } else if let Some((longitudinal, transverse)) = frames {
        let (axes, center) = affine_laws(laws)?;
        ps::approximate_authored_profiles(
            profiles,
            ps::AuthoredSweepLaws {
                path,
                scale,
                twist,
                longitudinal,
                transverse,
                axis_scale: axes,
                center,
            },
            options,
        )?
    } else if let Some((axes, center)) = affine {
        ps::approximate_affine_profiles(profiles, path, scale, twist, axes, center, options)?
    } else {
        ps::approximate_profiles(profiles, path, scale, twist, options)?
    })
}

/// The finest refinement level, refused unless it met every budget.
pub(super) fn accepted_level<'a>(
    approximation: &'a MultiApproximation,
    rmf_policy: Option<(usize, usize, usize)>,
    options: &Options,
) -> Result<&'a LevelReport> {
    let report = approximation
        .levels
        .last()
        .ok_or_else(|| err("Progressive body approximation has no refinement level"))?;
    if report.accepted {
        return Ok(report);
    }
    let unproved = |budget: &str| {
        err(format!(
            "Progressive body continuous retained-patch error is unproved or exceeds budget: {}",
            report.error_certificate_reason.unwrap_or(budget)
        ))
    };
    if rmf_policy.is_some() {
        return Err(unproved("configured RMF error budget"));
    }
    if report.sampled_control_deviation <= options.max_deviation {
        return Err(unproved("configured continuous error budget"));
    }
    Err(err("Progressive body misses sampled refinement budget"))
}

/// Profile sections at every accepted station, grouped back into loops.
pub(super) fn transported_sections(
    profiles: &[Curve],
    laws: &ProgressiveLaws<'_>,
    options: Options,
    count: usize,
) -> Result<Sections> {
    let ProgressiveLaws {
        loops,
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
    } = *laws;
    let transported = if let Some((guide, anchor)) = guidance {
        let sweep = MultiSweep::new(profiles, path, scale, twist, options)?;
        let sweep = if let Some((index, parameter)) = anchor {
            sweep.with_contact_guide(guide, index, parameter)?
        } else {
            sweep.with_orientation_guide(guide)?
        };
        let (axes, center) = affine_laws(laws)?;
        sweep.with_affine_laws(axes, center)?.sections_at(count)?
    } else {
        profiles
            .iter()
            .map(|p| {
                let sweep = ps::Sweep::new(p, path, scale, twist, options)?;
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
                sweep.sections_at(count)
            })
            .collect::<nurbs_core::Result<Vec<_>>>()?
    };
    Ok((0..count)
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
        .collect())
}

/// Replaces one station's curves in loop order, refusing a changed layout.
fn overwrite_station(station: &mut [Vec<Curve>], curves: &[Curve], refusal: &str) -> Result<()> {
    if station.iter().map(Vec::len).sum::<usize>() != curves.len() {
        return Err(err(refusal));
    }
    for (curve, corrected) in station.iter_mut().flatten().zip(curves) {
        *curve = corrected.clone();
    }
    Ok(())
}

/// Native shared-budget circle repair of every station; returns its displacement.
pub(super) fn correct_profiles(
    sections: &mut Sections,
    budget: EndpointCapCorrection,
) -> Result<f64> {
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
        err(format!(
            "Progressive profile circle correction refused: {}",
            repaired.reason
        ))
    })?;
    let corrected = repaired
        .sections
        .ok_or_else(|| err("Progressive profile circle correction has no geometry"))?;
    for (station, curves) in sections.iter_mut().zip(corrected) {
        overwrite_station(
            station,
            &curves,
            "Progressive profile circle correction changed the section layout",
        )?;
    }
    Ok(upper)
}

/// Projects both end stations onto their authored cap planes; returns the displacement.
pub(super) fn correct_caps(
    sections: &mut Sections,
    correction: EndpointCapCorrection,
    closed_path: bool,
    laws: &ProgressiveLaws<'_>,
    options: &Options,
) -> Result<f64> {
    if closed_path {
        return Err(err("Closed progressive body has no caps to correct"));
    }
    let mut work = 0_u64;
    let mut displacement = 0_f64;
    for end in [false, true] {
        let station = if end { sections.len() - 1 } else { 0 };
        let normal = cap_normal(laws, options, end)?;
        let axis = ps::constant_vector_law([normal[0], normal[1], normal[2]])?;
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
            return Err(err(format!(
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
        overwrite_station(
            &mut sections[station],
            &corrected,
            "Progressive cap correction changed the section layout",
        )?;
    }
    Ok(displacement)
}

fn cap_normal(laws: &ProgressiveLaws<'_>, options: &Options, end: bool) -> Result<Vec<f64>> {
    if let Some((axis, _)) = laws.frames {
        let domain = axis.domain();
        return Ok(axis
            .evaluate(if end { domain[1] } else { domain[0] })?
            .point);
    }
    let domain = laws.path.domain();
    let parameter = if end && options.orientation != ps::Orientation::Fixed {
        domain[1]
    } else {
        domain[0]
    };
    laws.path
        .evaluate(parameter)?
        .d1
        .ok_or_else(|| err("Cap correction needs an endpoint tangent"))
}

pub(super) fn combined_correction(profile: Option<f64>, cap: Option<f64>) -> Result<Option<f64>> {
    Ok(match (profile, cap) {
        (Some(a), Some(b)) => Some(
            nurbs_core::numerics::error_upper::add(a, b)
                .ok_or_else(|| err("Progressive correction error overflow"))?,
        ),
        (Some(a), None) | (None, Some(a)) => Some(a),
        (None, None) => None,
    })
}

/// The original multi-profile sweep with every authored law attached.
pub(super) fn sweep_source<'a>(
    profiles: &'a [Curve],
    laws: &ProgressiveLaws<'a>,
    options: Options,
    rmf_policy: Option<(usize, usize, usize)>,
) -> Result<MultiSweep<'a>> {
    let ProgressiveLaws {
        path,
        scale,
        twist,
        affine,
        frames,
        guidance,
        ..
    } = *laws;
    let mut source = MultiSweep::new(profiles, path, scale, twist, options)?;
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
}

/// Whether annular caps replace the planar pair, and the exact work spent deciding.
pub(super) fn annular_cap_selection(
    model: &Model,
    annular: Option<crate::retained_annular_cap::reference_filling::Limits>,
    automatic_caps: bool,
    closed_path: bool,
    [loop_count, spans]: [usize; 2],
) -> Result<(bool, u64)> {
    let (use_annular, work) = if automatic_caps && !closed_path && loop_count == 2 {
        let limits =
            annular.ok_or_else(|| err("Automatic annular caps require annular filling limits"))?;
        let (exact, work) =
            crate::sweep_miter_owned::cap_boundaries_exact(model, limits.exact_work)?;
        (!exact, work)
    } else {
        (annular.is_some() && !automatic_caps && !closed_path, 0)
    };
    // Preserve accepted legacy layouts. A replacement must fit the same face
    // ceiling; no surface/station is dropped to manufacture room for caps.
    if use_annular && model.faces.len() - 2 + spans > MAX_FACES {
        return Err(err(
            "Automatic annular caps exceed the shared body face budget",
        ));
    }
    Ok((use_annular, work))
}

/// Retained planar end-cap regions of an open body.
pub(super) fn retained_caps(
    model: &Model,
    retained_sections: &[Vec<Vec<Curve>>],
    max_exact_work: u64,
) -> Result<crate::sweep_retained_caps::Report> {
    let (Some(first), Some(last)) = (retained_sections.first(), retained_sections.last()) else {
        return Err(err("Progressive body has no retained sections"));
    };
    crate::sweep_retained_caps::inspect(
        model,
        &[first.clone(), last.clone()],
        crate::sweep_cap_contacts::Budgets {
            max_walls: 1024,
            max_exact_work,
            max_chart_cells: 1000,
            max_trim_pairs: 100000,
            max_trim_cells: 100000,
            max_trim_domain_cells: 1000000,
        },
        1024,
    )
}

/// The two planar end caps, which the open loft appends last.
pub(super) fn end_cap_surfaces(model: &Model) -> Result<[Surface; 2]> {
    match model.faces.as_slice() {
        [.., start, end] => Ok([start.surface.clone(), end.surface.clone()]),
        _ => Err(err("Progressive body has no end caps")),
    }
}

/// Caps the station count so walls and caps fit the shared face ceiling.
pub(super) fn face_bounded_options(
    laws: &ProgressiveLaws<'_>,
    options: Options,
    annular: Option<crate::retained_annular_cap::reference_filling::Limits>,
    automatic_caps: bool,
    spans: usize,
) -> Result<Options> {
    let closed = ps::path_is_closed(laws.path)?;
    let authored_annular_caps = annular.is_some() && !automatic_caps;
    if authored_annular_caps && !closed && laws.loops.len() != 2 {
        return Err(err(
            "Annular progressive caps require one outer ring and one hole",
        ));
    }
    let cap_faces = if closed {
        0
    } else if authored_annular_caps {
        spans
    } else {
        2
    };
    Ok(Options {
        max_sections: options
            .max_sections
            .min((MAX_FACES - cap_faces) / spans + 1),
        ..options
    })
}

/// Filled planar cap error from the certified endpoint projection.
pub(super) fn filled_cap_error(
    projection: &ps::EndpointCapProjectionReport,
    retained_regions_exact: bool,
    report: &LevelReport,
    correction: Option<f64>,
    decomposition: Option<f64>,
) -> Option<[f64; 2]> {
    nurbs_core::sweeps::filled_cap_error::filled_caps(
        &nurbs_core::sweeps::filled_cap_error::Premises {
            ideal_domains_certified: projection.original.domains.ideal_endpoint_domains_certified,
            retained_regions_exact,
            projection_normal_dots: projection.normal_dots,
            endpoint_error: report.endpoint_contour_error_upper,
            correction: Some(correction.unwrap_or(0.)),
            decomposition: decomposition.map(|e| [e; 2]),
            parallel_planes: [false; 2],
        },
    )
}

/// Wall error against the original source, including any correction.
pub(super) fn wall_error(
    report: &LevelReport,
    decomposition: Option<f64>,
    walls_certified: bool,
    correction: Option<f64>,
) -> Option<f64> {
    // Preview may itself have been decomposed; keeping its full error and adding
    // constructor extraction is conservative even when both include that term.
    let original = report
        .continuous_error_upper
        .zip(decomposition)
        .filter(|_| walls_certified)
        .and_then(|(a, b)| nurbs_core::numerics::error_upper::add(a, b));
    match correction {
        Some(correction) => {
            original.and_then(|e| nurbs_core::numerics::error_upper::add(e, correction))
        }
        None => original,
    }
}

/// Normal-dot tolerance of the endpoint cap projection certificate.
pub(super) const CAP_PROJECTION_TOLERANCE: f64 = 1e-9;

/// Profile then cap corrections; returns (profile, cap, combined) error bounds.
pub(super) fn apply_corrections(
    sections: &mut Sections,
    policy: ProgressiveBodyPolicy,
    closed_path: bool,
    laws: &ProgressiveLaws<'_>,
    options: &Options,
) -> Result<(Option<f64>, Option<f64>, Option<f64>)> {
    let profile = policy
        .profile_correction
        .map(|budget| correct_profiles(sections, budget))
        .transpose()?;
    let cap = policy
        .correction
        .map(|correction| correct_caps(sections, correction, closed_path, laws, options))
        .transpose()?;
    Ok((profile, cap, combined_correction(profile, cap)?))
}

/// Certified projection of the two planar end caps onto the ideal source.
pub(super) fn cap_projection(
    source: &MultiSweep<'_>,
    sizes: &[usize],
    model: &Model,
) -> Result<ps::EndpointCapProjectionReport> {
    let caps = end_cap_surfaces(model)?;
    source.certify_endpoint_cap_projection(
        sizes,
        &caps,
        CAP_PROJECTION_TOLERANCE,
        10000,
        100000,
        1000000,
    )
}

/// Retained ruled-wall certificate of the lofted body.
pub(super) fn retained_walls(
    model: &Model,
    retained_sections: &[Vec<Vec<Curve>>],
    closed_path: bool,
) -> Result<crate::sweep_retained_walls::Report> {
    crate::sweep_retained_walls::inspect(model, retained_sections, closed_path, 1024, 1000000)
}
