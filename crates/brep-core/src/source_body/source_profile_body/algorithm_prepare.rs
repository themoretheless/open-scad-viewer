//! source profile body algorithm: prepare.
use super::*;
/// Re-isolate both original crossing roots and requalify shared carrier/tangency.
/// Returned seams borrow only the proposal, never temporary decoded bounds.

/// Restore an authored section end by fresh original-pcurve enclosure.

/// Reconstruct an authored two-section Hermite expression on this proposal.
/// The recipe binds source profiles, section axes and exact binary64 scales;
/// regularity, sewing, embedding and wall admission remain fresh later gates.

/// Rebuild exact retained profiles against a freshly admitted original body.
/// Serialized diagnostic fields never replace native preparation or budgets.

pub fn prepare<'a>(body: &'a Body, specs: &[FaceSpec], limits: Limits) -> Result<Report<'a>> {
    let regions = body
        .geometry()
        .shell()
        .regions()
        .ok_or_else(|| Error::new("BREP_PROFILE_BODY", "Require original retained regions"))?;
    validate(specs, &limits, regions.len())?;
    let mut out = Report {
        proposal: None,
        controls: 0,
        fragments: 0,
        reason: "profile-body-work-limit",
    };
    let mut faces = Vec::new();
    let mut deviation = [0_f64; 3];
    for spec in specs {
        let region = &regions[spec.face];
        let source = region.loops()[0][0].surface();
        let (profile, used) = nurbs_core::normal_alignment::qualify_weight_profile(
            source,
            spec.anchor,
            limits.controls - out.controls,
        )?;
        out.controls += used;
        let Some(profile) = profile else {
            return Ok(out);
        };
        if !profile.within_position_tolerance(limits.position_tolerance)? {
            out.reason = "profile-body-position-tolerance-unproven";
            return Ok(out);
        }
        if !boundaries_preserved(&profile, &limits, &mut out)?
            || !contours_on_boundary(region.loops(), source, &limits, &mut out)
        {
            return Ok(out);
        }
        for (k, error) in profile.axis_deviation_upper().into_iter().enumerate() {
            deviation[k] = deviation[k].max(error);
        }
        faces.push(Face {
            spec: *spec,
            profile,
        });
    }
    out.reason = "profile-body-boundaries-preserved";
    out.proposal = Some(Proposal {
        body,
        faces,
        axis_deviation_upper: deviation,
        position_tolerance: limits.position_tolerance,
    });
    Ok(out)
}

/// Inclusive upper bound on the control-point budget of one preparation.
const MAX_CONTROLS: usize = 1_000_000;

fn validate(specs: &[FaceSpec], limits: &Limits, regions: usize) -> Result<()> {
    if specs.is_empty()
        || specs.len() > regions
        || limits.controls > MAX_CONTROLS
        || limits.fragments > span_work::MAX_WORK
        || !limits.position_tolerance.is_finite()
        || limits.position_tolerance < 0.
        || specs.iter().any(|f| f.face >= regions)
        || specs
            .iter()
            .enumerate()
            .any(|(i, f)| specs[..i].iter().any(|a| a.face == f.face))
    {
        return Err(Error::new(
            "BREP_PROFILE_BODY",
            "Choose distinct owned faces and finite bounded proposal limits",
        ));
    }
    Ok(())
}

/// Check all natural boundaries, including collapsed rails absent from the
/// retained wire. This preserves original pole and edge positions.
fn boundaries_preserved(
    profile: &WeightProfile<'_>,
    limits: &Limits,
    out: &mut Report<'_>,
) -> Result<bool> {
    for axis in 0..2 {
        for high in [false, true] {
            let (boundary, used) =
                profile.preserved_boundary(axis, high, limits.controls - out.controls)?;
            out.controls += used;
            if boundary.is_none() {
                out.reason = "profile-body-boundary-identity-unproven";
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// Every retained trim fragment must stay on its source and touch at least
/// one natural boundary line with all of its control points.
fn contours_on_boundary(
    loops: &[Vec<crate::source_boundary_fragment::Fragment>],
    source: &Surface,
    limits: &Limits,
    out: &mut Report<'_>,
) -> bool {
    let domains = [
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    ];
    for fragment in loops.iter().flatten() {
        if out.fragments == limits.fragments {
            return false;
        }
        out.fragments += 1;
        if !same(fragment.surface(), source) || fragment.curve().periodic {
            out.reason = "profile-body-contour-source-unproven";
            return false;
        }
        let mut fixed = [true; 4];
        for point in &fragment.curve().control_points {
            if out.controls == limits.controls {
                return false;
            }
            out.controls += 1;
            if point.len() != 2 || (0..2).any(|a| point[a] < domains[a][0] || point[a] > domains[a][1])
            {
                out.reason = "profile-body-contour-boundary-unproven";
                return false;
            }
            for axis in 0..2 {
                for high in 0..2 {
                    fixed[2 * axis + high] &= point[axis] == domains[axis][high];
                }
            }
        }
        if !fixed.iter().any(|x| *x) {
            out.reason = "profile-body-interior-trim-unproven";
            return false;
        }
    }
    true
}
