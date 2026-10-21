//! Cylindrical ZA worm working patches from an axial trapezoidal rack.
//! Original-spec inclusive bounds; no finite end trim, caps or solid claim.
pub use crate::thread::Hand;
use crate::{
    Result, check,
    curve::Curve,
    distance_bounds::{Interval as I, box_distance},
    surface::Surface,
};
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub axial_module: f64,
    pub diameter_quotient: f64,
    /// Axial pressure angle in radians, not normal pressure angle.
    pub pressure_angle: f64,
    pub dedendum_factor: f64,
    pub backlash: f64,
    pub starts: usize,
    pub turns: usize,
    pub hand: Hand,
    pub error_budget: f64,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            axial_module: 1.,
            diameter_quotient: 10.,
            pressure_angle: 20_f64.to_radians(),
            dedendum_factor: 1.2,
            backlash: 0.,
            starts: 1,
            turns: 1,
            hand: Hand::Right,
            error_budget: 1e-4,
        }
    }
}
#[derive(Clone, Debug)]
pub struct ParameterBounds {
    pub pitch_radius: [f64; 2],
    pub crest_radius: [f64; 2],
    pub root_radius: [f64; 2],
    pub pitch: [f64; 2],
    pub lead: [f64; 2],
    pub lead_angle: [f64; 2],
    pub profile_z: [[f64; 2]; 6],
    pub crest_width: [f64; 2],
    pub root_width: [f64; 2],
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
fn iv(x: [f64; 2]) -> Result<I> {
    I::new(x[0], x[1])
}
fn mid(x: I) -> f64 {
    x.lo + (x.hi - x.lo) * 0.5
}
/// Exact real binary64 spec data; outward Taylor trig and Machin pi bound all
/// nominal pitch, lead, rack flank placement and radius arithmetic.
pub fn derive_parameters(spec: Spec) -> Result<ParameterBounds> {
    check(
        [
            spec.axial_module,
            spec.diameter_quotient,
            spec.pressure_angle,
            spec.dedendum_factor,
            spec.backlash,
            spec.error_budget,
        ]
        .iter()
        .all(|x| x.is_finite())
            && (0.05..=100.).contains(&spec.axial_module)
            && (6. ..=80.).contains(&spec.diameter_quotient)
            && (0.15..=0.65).contains(&spec.pressure_angle)
            && (1. ..=2.).contains(&spec.dedendum_factor)
            && spec.backlash >= 0.
            && spec.backlash < spec.axial_module / 2.
            && (1..=8).contains(&spec.starts)
            && (1..=16).contains(&spec.turns)
            && spec.error_budget > 0.,
        "ZA worm spec has invalid module, diameter quotient, axial pressure angle, allowances, starts or turns",
    )?;
    let m = I::point(spec.axial_module);
    let pitch = crate::involute::certified_pi()?.mul(m)?;
    let lead = pitch.mul(I::point(spec.starts as f64))?;
    let rp = m.mul(I::point(spec.diameter_quotient))?.div(I::point(2.))?;
    let crest = rp.add(m)?;
    let dedendum = m.mul(I::point(spec.dedendum_factor))?;
    let root = rp.sub(dedendum)?;
    let (s, c) = crate::involute::certified_trig(I::point(spec.pressure_angle))?;
    let tangent = s.div(c)?;
    let thickness = pitch.div(I::point(2.))?.sub(I::point(spec.backlash))?;
    let crest_width = thickness.sub(m.mul(tangent)?.mul(I::point(2.))?)?;
    let root_width = thickness.add(dedendum.mul(tangent)?.mul(I::point(2.))?)?;
    check(
        root.lo > 0. && crest_width.lo > 0. && root_width.hi < pitch.lo,
        "ZA worm crest collapses, root reaches the axis or adjacent root teeth overlap",
    )?;
    let half = I::point(2.);
    let z = [
        I::point(0.),
        crest_width.div(half)?,
        root_width.div(half)?,
        pitch.sub(root_width.div(half)?)?,
        pitch.sub(crest_width.div(half)?)?,
        pitch,
    ];
    check(
        z.windows(2).all(|p| p[0].hi < p[1].lo),
        "ZA worm profile phases are not provably ordered",
    )?;
    Ok(ParameterBounds {
        pitch_radius: pair(rp),
        crest_radius: pair(crest),
        root_radius: pair(root),
        pitch: pair(pitch),
        lead: pair(lead),
        lead_angle: pair(crate::involute::certified_atan_positive(
            I::point(spec.starts as f64).div(I::point(spec.diameter_quotient))?,
        )?),
        profile_z: z.map(pair),
        crest_width: pair(crest_width),
        root_width: pair(root_width),
    })
}
#[derive(Clone, Debug)]
pub struct Patch {
    pub surface: Surface,
    pub start: usize,
    pub turn: usize,
    pub quarter: usize,
    pub segment: usize,
}
#[derive(Clone, Debug)]
pub struct Certificate {
    pub error_upper: f64,
    pub sweep_error_upper: f64,
    pub axial_profile_error_upper: f64,
    pub patches: usize,
}
fn bounds(
    p: &ParameterBounds,
    start: usize,
    turn: usize,
    quarter: usize,
    segment: usize,
) -> Result<([[I; 3]; 2], I)> {
    let lead = iv(p.lead)?;
    let offset = iv(p.pitch)?
        .mul(I::point(start as f64))?
        .add(lead.mul(I::point(turn as f64 + quarter as f64 / 4.))?)?;
    let radii = [
        p.crest_radius,
        p.crest_radius,
        p.root_radius,
        p.root_radius,
        p.crest_radius,
        p.crest_radius,
    ];
    let point = |i: usize| -> Result<[I; 3]> {
        Ok([
            iv(radii[i])?,
            I::point(0.),
            offset.add(iv(p.profile_z[i])?)?,
        ])
    };
    Ok((
        [point(segment)?, point(segment + 1)?],
        lead.div(I::point(4.))?,
    ))
}
fn profile(p: [[I; 3]; 2]) -> Result<Curve> {
    let points = p.map(|r| r.map(mid).to_vec());
    check(
        points[0][2] < points[1][2],
        "ZA worm axial profile phase collapses at stored precision",
    )?;
    Curve::from_polyline(points.to_vec())
}
fn angles(hand: Hand, quarter: usize) -> Result<(I, I)> {
    let sign = if hand == Hand::Right { 1. } else { -1. };
    let twist = crate::involute::certified_pi()?
        .div(I::point(2.))?
        .mul(I::point(sign))?;
    Ok((twist.mul(I::point(quarter as f64))?, twist))
}
/// Independently certify every actual saved patch and its complete original
/// start/turn/quarter/profile census. No cached flag or sampled residual is used.
pub fn certify_patches(spec: Spec, patches: &[Patch], max_patches: usize) -> Result<Certificate> {
    let parameters = derive_parameters(spec)?;
    let expected = 20 * spec.starts * spec.turns;
    check(
        (1..=2560).contains(&max_patches) && expected <= max_patches && patches.len() == expected,
        "ZA worm patch inventory or budget is incomplete",
    )?;
    let mut sweep_error = 0_f64;
    let mut profile_error = 0_f64;
    let mut error = 0_f64;
    for (i, patch) in patches.iter().enumerate() {
        let segment = i % 5;
        let quarter = (i / 5) % 4;
        let turn = (i / 20) % spec.turns;
        let start = i / (20 * spec.turns);
        check(
            (patch.start, patch.turn, patch.quarter, patch.segment)
                == (start, turn, quarter, segment),
            "ZA worm original patch roles are missing, reordered or changed",
        )?;
        let (nominal, height) = bounds(&parameters, start, turn, quarter, segment)?;
        let original = profile(nominal)?;
        let mut delta = 0_f64;
        for (stored, target) in original.control_points.iter().zip(&nominal) {
            let stored = [I::point(stored[0]), I::point(stored[1]), I::point(stored[2])];
            delta = delta.max(box_distance(&stored, target)?.1);
        }
        let (angle, twist) = angles(spec.hand, quarter)?;
        let proof = crate::helical_sweep::certify_stored(
            &original,
            &patch.surface,
            pair(height),
            pair(angle),
            pair(twist),
        )?;
        sweep_error = sweep_error.max(proof.error_upper);
        profile_error = profile_error.max(delta);
        error = error.max(I::point(proof.error_upper).add(I::point(delta))?.hi);
    }
    Ok(Certificate {
        error_upper: error,
        sweep_error_upper: sweep_error,
        axial_profile_error_upper: profile_error,
        patches: expected,
    })
}
pub struct QualifiedPatches {
    spec: Spec,
    parameters: ParameterBounds,
    patches: Vec<Patch>,
    certificate: Certificate,
}
impl QualifiedPatches {
    pub fn specification(&self) -> Spec {
        self.spec
    }
    pub fn parameters(&self) -> &ParameterBounds {
        &self.parameters
    }
    pub fn patches(&self) -> &[Patch] {
        &self.patches
    }
    pub fn certificate(&self) -> &Certificate {
        &self.certificate
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = certify_patches(self.spec, &self.patches, self.patches.len())?;
        check(
            fresh.error_upper <= self.spec.error_budget
                && fresh.error_upper == self.certificate.error_upper,
            "Owned ZA worm original source/error receipt changed",
        )
    }
}
/// Construct and independently admit the untrimmed ZA worm working surface.
/// The requested error covers all stored patches against the original nominal
/// axial profile and lead. End clipping, shell sewing and caps remain separate.
pub fn patches_certified(spec: Spec, max_patches: usize) -> Result<QualifiedPatches> {
    let parameters = derive_parameters(spec)?;
    let count = 20 * spec.starts * spec.turns;
    check(
        (1..=2560).contains(&max_patches) && count <= max_patches,
        "ZA worm patch budget is insufficient",
    )?;
    let mut patches = Vec::with_capacity(count);
    for start in 0..spec.starts {
        for turn in 0..spec.turns {
            for quarter in 0..4 {
                let (angle, twist) = angles(spec.hand, quarter)?;
                let mid_angle = mid(angle);
                let mid_twist = mid(twist);
                for segment in 0..5 {
                    let (p, h) = bounds(&parameters, start, turn, quarter, segment)?;
                    let original = profile(p)?;
                    let result = crate::helical_sweep::approximate(
                        &original,
                        mid(h),
                        mid_angle,
                        mid_twist,
                        spec.error_budget * 0.25,
                    )?;
                    patches.push(Patch {
                        surface: result.surface,
                        start,
                        turn,
                        quarter,
                        segment,
                    });
                }
            }
        }
    }
    let certificate = certify_patches(spec, &patches, max_patches)?;
    check(
        certificate.error_upper <= spec.error_budget,
        "ZA worm nominal stored-surface error exceeds requested tolerance",
    )?;
    Ok(QualifiedPatches {
        spec,
        parameters,
        patches,
        certificate,
    })
}

fn screw_source(source: &QualifiedPatches) -> Vec<crate::thread::Patch> {
    source
        .patches
        .iter()
        .map(|p| crate::thread::Patch {
            surface: p.surface.clone(),
            start: p.start,
            turn: p.turn,
            quarter: p.quarter,
            profile_segment: p.segment,
            real_arithmetic_error_estimate: source.certificate.error_upper,
        })
        .collect()
}
/// Construct source-bound finite-slab candidates using the shared affine
/// clipping engine. These are candidates until independent coverage replay.
pub fn trim_candidates(
    source: &QualifiedPatches,
    z_bounds: [f64; 2],
    axial_tolerance: f64,
) -> Result<crate::thread::TrimCandidates> {
    source.reverify()?;
    crate::thread::trim_patch_candidates(
        screw_source(source),
        mid(iv(source.parameters.lead)?),
        z_bounds,
        axial_tolerance,
    )
}
/// Replay full original ZA source accuracy and complete retained/excluded slab
/// inventory. Thread containers carry geometry only; no 60-degree spec is used.
pub fn certify_trim_candidates(
    source: &QualifiedPatches,
    trims: &crate::thread::TrimCandidates,
    max_uv_error: f64,
    axial_tolerance: f64,
    max_patches: usize,
) -> Result<crate::thread::TrimCoverageCertificate> {
    source.reverify()?;
    crate::thread::certify_patch_trim_coverage(
        &screw_source(source),
        source.spec.starts,
        source.spec.turns,
        mid(iv(source.parameters.lead)?),
        trims,
        max_uv_error,
        axial_tolerance,
        max_patches,
    )
}
pub struct QualifiedTrimmedPatches {
    source: QualifiedPatches,
    trims: crate::thread::TrimCandidates,
    coverage: crate::thread::TrimCoverageCertificate,
    max_uv_error: f64,
    axial_tolerance: f64,
}
impl QualifiedTrimmedPatches {
    pub fn source(&self) -> &QualifiedPatches {
        &self.source
    }
    pub fn trims(&self) -> &crate::thread::TrimCandidates {
        &self.trims
    }
    pub fn coverage(&self) -> &crate::thread::TrimCoverageCertificate {
        &self.coverage
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = certify_trim_candidates(
            &self.source,
            &self.trims,
            self.max_uv_error,
            self.axial_tolerance,
            self.source.patches.len(),
        )?;
        check(
            fresh.axial_error_band_upper == self.coverage.axial_error_band_upper
                && fresh.retained_patches == self.coverage.retained_patches,
            "Owned ZA worm trim source/coverage receipt changed",
        )
    }
}
pub fn trimmed_patches_certified(
    spec: Spec,
    z_bounds: [f64; 2],
    max_uv_error: f64,
    axial_tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedTrimmedPatches> {
    let source = patches_certified(spec, max_patches)?;
    let trims = trim_candidates(&source, z_bounds, axial_tolerance)?;
    let coverage =
        certify_trim_candidates(&source, &trims, max_uv_error, axial_tolerance, max_patches)?;
    Ok(QualifiedTrimmedPatches {
        source,
        trims,
        coverage,
        max_uv_error,
        axial_tolerance,
    })
}
