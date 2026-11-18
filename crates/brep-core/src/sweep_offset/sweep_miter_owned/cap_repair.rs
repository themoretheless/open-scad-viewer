use super::*;

/// Repair retained endpoint caps using the same bounded native policy as
/// original-request construction. The returned displacement is relative to
/// these retained sections; callers must add any earlier correction bound.
pub fn automatic_cap_correction(
    model: &Model,
    retained: &[Vec<Curve>],
    points: &[[f64; 3]],
    frame_axis: Option<&Curve>,
    max_deviation: f64,
    max_work: u64,
) -> Result<Option<nurbs_core::sweep_section_projection::Report>> {
    let (needed, work) = nonplanar_caps(model, max_work);
    if !needed {
        return Ok(None);
    }
    let profile = periodic_profile_lattice(
        retained,
        2_f64.powi(-40),
        max_deviation.min(1e-9),
        max_work - work,
    )?;
    let (profile_error, profile_work) = profile.as_ref().map_or((0., 0), |(_, e, w)| (*e, *w));
    let projected_source = profile
        .as_ref()
        .map_or(retained, |(curves, _, _)| curves.as_slice());
    let mut repair = miter_section_correction::correct(
        projected_source,
        points,
        false,
        None,
        Some(CapCorrection {
            budget: Budget {
                quantum: 2_f64.powi(-40),
                tolerance: max_deviation.min(1e-9),
                max_work: Some((max_work - work - profile_work) as f64),
            },
            authored_frame: frame_axis.is_some(),
        }),
        frame_axis,
    )?
    .ok_or_else(|| invalid("Automatic cap correction result missing"))?;
    repair.wall_displacement_upper = repair
        .wall_displacement_upper
        .and_then(|x| error_upper::add(x, profile_error));
    if repair.wall_displacement_upper.is_none() {
        return Err(invalid(
            "Automatic cap correction displacement bound unproved",
        ));
    }
    repair.work += work + profile_work;
    repair.reason = "automatic-bounded-cap-planarity";
    Ok(Some(repair))
}
