//! Constructor-owned full boundary error for moving frames and authored caps.
//! Walls come from the original MultiSweep law proof; both filled cap interiors
//! are affine endpoint images of original rational carriers. No planar shortcut.
use crate::{Error, LoftCap, Model, Result, boundary_embedding::regularity, volume_validity};
use nurbs_core::{
    curve::Curve,
    numerics::error_upper,
    progressive_sweep::{LevelReport, MultiSweep},
    retained_wall_coefficients, surface_correspondence,
};

#[derive(Clone, Copy)]
pub struct Limits {
    pub source_cells: usize,
    pub source_products: usize,
    pub source_exact_work: u64,
    pub regularity_cells: usize,
    pub linear_cells: usize,
    pub volume: volume_validity::Limits,
}
pub struct Report {
    pub certified: bool,
    pub continuous_bound: bool,
    pub within_budget: bool,
    pub boundary_error_upper: Option<f64>,
    pub wall: LevelReport,
    pub caps: Vec<surface_correspondence::Report>,
    pub cap_errors: Option<[f64; 2]>,
    pub source_cells: usize,
    pub source_products: usize,
    pub source_exact_work: u64,
    pub regularity: Option<regularity::Report>,
    pub volume: Option<volume_validity::Report>,
    pub reason: Option<&'static str>,
    pub ideal_volume: Option<nurbs_core::progressive_sweep::IdealVolumeReport>,
}
pub struct CertifiedBody {
    model: Model,
    report: Report,
}
impl CertifiedBody {
    pub fn model(&self) -> &Model {
        &self.model
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn into_model(self) -> Model {
        self.model
    }
}
pub enum Attempt {
    Certified(Box<CertifiedBody>),
    Refused(Box<Report>),
}
fn fail(message: &'static str) -> Error {
    Error::new("BREP_SWEEP_BOUNDARY_INVALID", message)
}

/// One initial authored filled carrier for each endpoint. Their trims must
/// bound the original profile loops exactly and match the retained cap trims.
/// Source-law approximation and retained-solid validity remain distinct fields;
/// this does not prove ideal-body embedding or G1/G2 at seams.
pub fn construct(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    source_caps: [&LoftCap; 2],
    retained_caps: [&LoftCap; 2],
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
) -> Result<Attempt> {
    if !error_budget.is_finite()
        || error_budget <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || limits.source_cells > 100000
        || limits.source_products > 1000000
        || limits.source_exact_work > 1000000
        || limits.regularity_cells > 100000
        || limits.linear_cells > 100000
    {
        return Err(fail("Invalid full sweep certificate limits"));
    }
    for cap in source_caps.iter().chain(retained_caps.iter()) {
        cap.surface.validate()?;
        for curve in cap.trims.iter().flatten() {
            curve.validate()?;
            if curve.control_points.iter().any(|p| p.len() != 2) {
                return Err(fail("Cap trims require UV controls"));
            }
        }
    }
    let profiles = sweep.source_profiles().collect::<Vec<_>>();
    if loop_sizes.is_empty()
        || loop_sizes.len() > 16
        || loop_sizes.contains(&0)
        || loop_sizes.iter().any(|&n| n > profiles.len())
        || loop_sizes.iter().sum::<usize>() != profiles.len()
        || sweep.source_is_closed()?
    {
        return Err(fail(
            "Open capped sweep requires complete authored loop coverage",
        ));
    }
    let level = sweep.preview_at_with_error_limits(
        sections,
        limits.source_cells,
        limits.source_products,
    )?;
    let mut out = Report {
        certified: false,
        continuous_bound: false,
        within_budget: false,
        boundary_error_upper: None,
        source_cells: level.report.error_certificate_cells,
        source_products: level.report.decomposition_products,
        source_exact_work: 0,
        wall: level.report,
        caps: Vec::new(),
        cap_errors: None,
        regularity: None,
        volume: None,
        reason: Some("source-wall-bound-unproved"),
        ideal_volume: None,
    };
    if !out.wall.continuous_bound || out.wall.continuous_error_upper.is_none() {
        return Ok(Attempt::Refused(Box::new(out)));
    }
    // Bind both complete original contours, including every hole, to the
    // authored cap carriers. Exact algebraic identity also checks UV inclusion.
    for endpoint in 0..2 {
        source_caps[endpoint].surface.validate()?;
        retained_caps[endpoint].surface.validate()?;
        if source_caps[endpoint].trims != retained_caps[endpoint].trims
            || source_caps[endpoint].trims.len() != loop_sizes.len()
        {
            out.reason = Some("cap-uv-correspondence-mismatch");
            return Ok(Attempt::Refused(Box::new(out)));
        }
        let mut offset = 0;
        for (ring, &size) in loop_sizes.iter().enumerate() {
            let mut boundary = Vec::new();
            for curve in &profiles[offset..offset + size] {
                let rows = (curve.degree..curve.control_points.len())
                    .filter(|&i| curve.knots[i] < curve.knots[i + 1])
                    .count()
                    * (curve.degree + 1);
                if rows > limits.source_products - out.source_products {
                    out.reason = Some("source-cap-decomposition-work-limit");
                    return Ok(Attempt::Refused(Box::new(out)));
                }
                out.source_products += rows;
                let Some(parts) =
                    retained_wall_coefficients::exact_bezier_controls(curve, rows.min(4096))
                else {
                    out.reason = Some("source-cap-decomposition-unproved");
                    return Ok(Attempt::Refused(Box::new(out)));
                };
                boundary.extend(parts);
            }
            offset += size;
            if boundary.len() != source_caps[endpoint].trims[ring].len() {
                out.reason = Some("source-cap-contour-count-mismatch");
                return Ok(Attempt::Refused(Box::new(out)));
            }
            for (world, uv) in boundary.iter().zip(&source_caps[endpoint].trims[ring]) {
                let remaining = limits.source_exact_work - out.source_exact_work;
                let proof = nurbs_core::curve_surface_agreement::verify_exact(
                    world,
                    uv,
                    &source_caps[endpoint].surface,
                    false,
                    remaining,
                )?;
                let Some(proof) = proof else {
                    out.source_exact_work = limits.source_exact_work;
                    out.reason = Some("source-cap-boundary-unproved");
                    return Ok(Attempt::Refused(Box::new(out)));
                };
                out.source_exact_work += proof.work_used;
                if proof.outcome != cad_predicates::BezierIdentity::Equal {
                    out.reason = Some("source-cap-boundary-unproved");
                    return Ok(Attempt::Refused(Box::new(out)));
                }
            }
        }
    }
    let mut caps = [0_f64; 2];
    for endpoint in 0..2 {
        let image = sweep.endpoint_surface_image(
            &source_caps[endpoint].surface,
            endpoint,
            limits.source_cells - out.source_cells,
            limits.source_products - out.source_products,
        )?;
        out.source_cells += image.cells;
        out.source_products += image.products;
        let (Some(surface), Some(rounding)) = (image.surface, image.error_upper) else {
            out.reason = image.reason;
            return Ok(Attempt::Refused(Box::new(out)));
        };
        let bound = surface_correspondence::inspect(
            &surface,
            &retained_caps[endpoint].surface,
            error_budget,
            surface_correspondence::Limits {
                cells: limits.source_cells - out.source_cells,
                products: limits.source_products - out.source_products,
            },
        )?;
        out.source_cells += bound.cells;
        out.source_products += bound.products;
        caps[endpoint] = match bound
            .error_upper
            .and_then(|e| error_upper::add(rounding, e))
        {
            Some(value) => value,
            None => {
                out.reason = Some("filled-cap-bound-unproved");
                out.caps.push(bound);
                return Ok(Attempt::Refused(Box::new(out)));
            }
        };
        out.caps.push(bound);
    }
    out.cap_errors = Some(caps);
    let bound = out
        .wall
        .continuous_error_upper
        .unwrap()
        .max(caps[0])
        .max(caps[1]);
    out.boundary_error_upper = Some(bound);
    out.continuous_bound = true;
    out.within_budget = bound <= error_budget;
    if !out.within_budget {
        out.reason = Some("boundary-error-budget-exceeded");
        return Ok(Attempt::Refused(Box::new(out)));
    }
    // Build from this exact level, not from a caller-supplied model or station report.
    let rows = sweep.sections_at(sections)?;
    let mut start = Vec::new();
    let mut end = Vec::new();
    let mut sides = Vec::new();
    let mut offset = 0;
    for &size in loop_sizes {
        start.push(
            rows[offset..offset + size]
                .iter()
                .map(|p| p[0].clone())
                .collect::<Vec<Curve>>(),
        );
        end.push(
            rows[offset..offset + size]
                .iter()
                .map(|p| p.last().unwrap().clone())
                .collect::<Vec<Curve>>(),
        );
        let range = [
            level.profile_patch_ranges[offset][0],
            level.profile_patch_ranges[offset + size - 1][1],
        ];
        sides.push(level.patches[range[0]..range[1]].to_vec());
        offset += size;
    }
    let model = crate::capped_loft_with_caps(&start, &end, &sides, retained_caps)?;
    let regular = regularity::inspect(&model, limits.regularity_cells)?;
    let regular_proven = regular.all_faces_regular_modulo_poles;
    out.regularity = Some(regular);
    if !regular_proven {
        out.reason = Some("whole-boundary-regularity-unproved");
        return Ok(Attempt::Refused(Box::new(out)));
    }
    let volume = volume_validity::inspect_with_linear(
        &model,
        tolerance_uv,
        limits.volume,
        limits.linear_cells,
    )?;
    out.certified = regular_proven && volume.proven;
    out.volume = Some(volume);
    out.reason = if !regular_proven {
        Some("whole-boundary-regularity-unproved")
    } else if !out.certified {
        Some("solid-geometry-unproved")
    } else {
        None
    };
    Ok(if out.certified {
        Attempt::Certified(Box::new(CertifiedBody { model, report: out }))
    } else {
        Attempt::Refused(Box::new(out))
    })
}

#[derive(Clone, Copy)]
pub struct IdealLimits {
    /// One fixed projection, used for the complete original source volume.
    pub projection: [[f64; 3]; 3],
    pub cells: usize,
    pub exact_work: u64,
}
/// Stronger constructor: no output model unless BOTH the retained solid and
/// the original filled-source embedding are proved. The UV topology comes from
/// the same native cap/contour/trim and retained-volume proof as `construct`;
/// a caller cannot attach an unrelated source proof to an existing model.
pub fn construct_with_ideal(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    source_caps: [&LoftCap; 2],
    retained_caps: [&LoftCap; 2],
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
    ideal: IdealLimits,
) -> Result<Attempt> {
    construct_with_ideal_impl(
        sweep,
        loop_sizes,
        sections,
        source_caps,
        retained_caps,
        tolerance_uv,
        error_budget,
        limits,
        ideal,
        false,
    )
}
/// Same constructor-owned admission, using the exhaustive ORIGINAL parameter
/// pair cover instead of the sufficient global strong-monotonicity metric.
pub fn construct_with_ideal_atlas(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    source_caps: [&LoftCap; 2],
    retained_caps: [&LoftCap; 2],
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
    ideal: IdealLimits,
) -> Result<Attempt> {
    construct_with_ideal_impl(
        sweep,
        loop_sizes,
        sections,
        source_caps,
        retained_caps,
        tolerance_uv,
        error_budget,
        limits,
        ideal,
        true,
    )
}
fn construct_with_ideal_impl(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    source_caps: [&LoftCap; 2],
    retained_caps: [&LoftCap; 2],
    tolerance_uv: f64,
    error_budget: f64,
    limits: Limits,
    ideal: IdealLimits,
    atlas: bool,
) -> Result<Attempt> {
    // Validate stronger limits even when an earlier geometric stage refuses.
    if ideal.cells > 100000
        || ideal.exact_work > 1000000
        || ideal.projection.iter().flatten().any(|x| !x.is_finite())
    {
        return Err(fail("Invalid original ideal-body proof limits"));
    }
    let attempt = construct(
        sweep,
        loop_sizes,
        sections,
        source_caps,
        retained_caps,
        tolerance_uv,
        error_budget,
        limits,
    )?;
    let Attempt::Certified(mut body) = attempt else {
        return Ok(attempt);
    };
    let domain = |s: &nurbs_core::surface::Surface| {
        [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ]
    };
    if source_caps[0].trims != source_caps[1].trims
        || domain(&source_caps[0].surface) != domain(&source_caps[1].surface)
    {
        body.report.certified = false;
        body.report.reason = Some("ideal-body-common-trim-region-unproved");
        return Ok(Attempt::Refused(Box::new(body.report)));
    }
    let mut profile_cells = 0;
    for profile in sweep.source_profiles() {
        let regular = nurbs_core::curve_regularity::inspect(profile, ideal.cells - profile_cells)?;
        profile_cells += regular.cells;
        if !regular.spanwise_regular {
            body.report.certified = false;
            body.report.reason = Some("ideal-body-original-profile-regularity-unproved");
            return Ok(Attempt::Refused(Box::new(body.report)));
        }
    }
    let certify = if atlas {
        MultiSweep::certify_ideal_volume_atlas
    } else {
        MultiSweep::certify_ideal_volume
    };
    let proof = certify(
        sweep,
        [&source_caps[0].surface, &source_caps[1].surface],
        ideal.projection,
        ideal.cells - profile_cells,
        ideal.exact_work,
    )?;
    let mut proof = proof;
    proof.cells += profile_cells;
    let proven = proof.embedding_certified
        && proof.regularity_certified
        && proof.parameter_topology_preserved;
    body.report.reason = if proven { None } else { proof.reason };
    body.report.ideal_volume = Some(proof);
    body.report.certified = proven;
    Ok(if proven {
        Attempt::Certified(body)
    } else {
        Attempt::Refused(Box::new(body.report))
    })
}

mod closed;
pub use closed::{ClosedAttempt, ClosedBody, ClosedLimits, ClosedReport, MaterialLimits, MaterialReport, construct_closed_with_ideal_atlas};
pub use closed::{SmoothClosedAttempt,SmoothClosedBody,SmoothClosedLimits,SmoothClosedReport,construct_smooth_closed_with_ideal_atlas};
pub use closed::OriginalClosedLimits;
pub use closed::{OriginalClosedReport,AffineWallBindingReport};
pub use closed::verify_smooth_closed_with_ideal_atlas;
