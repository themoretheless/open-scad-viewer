//! Untrimmed helical patch construction and its authored specification.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    External,
    Internal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hand {
    Right,
    Left,
}
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    pub diameter: f64,
    pub pitch: f64,
    pub clearance: f64,
    pub starts: usize,
    pub turns: usize,
    pub kind: Kind,
    pub hand: Hand,
    pub error_budget: f64,
}
impl Default for Spec {
    fn default() -> Self {
        Self {
            diameter: 6.,
            pitch: 1.,
            clearance: 0.1,
            starts: 1,
            turns: 1,
            kind: Kind::External,
            hand: Hand::Right,
            error_budget: 1e-4,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Patch {
    pub surface: Surface,
    pub start: usize,
    pub turn: usize,
    pub quarter: usize,
    pub profile_segment: usize,
    pub real_arithmetic_error_estimate: f64,
}
#[derive(Clone, Debug)]
pub struct Patches {
    pub patches: Vec<Patch>,
    pub crest_radius: f64,
    pub root_radius: f64,
    pub lead: f64,
    /// Actual untrimmed coverage; differs from a bounded threaded solid's length.
    pub axial_extent: [f64; 2],
    pub rounding_certified: bool,
}
/// Sweep a sharp, truncated 60-degree profile with phase breakpoints
/// [0,1/16,3/8,5/8,15/16,1]. No cutter/root fillet or standard tolerance class.
/// Each integer turn is split into four quarter sweeps, five profile segments.
/// Adjacent starts are shifted by one pitch in Z. End planes are not trimmed.
pub fn patches(spec: Spec) -> Result<Patches> {
    require_finite_f64(spec.diameter, "thread_diameter")?;
    require_finite_f64(spec.pitch, "thread_pitch")?;
    require_finite_f64(spec.clearance, "thread_clearance")?;
    require_finite_f64(spec.error_budget, "thread_error_budget")?;
    check(
        spec.diameter > 0. && spec.pitch > 0. && spec.clearance >= 0. && spec.error_budget > 0.,
        "Thread diameter, pitch and error budget must be positive; clearance nonnegative",
    )?;
    check(
        (1..=8).contains(&spec.starts) && (1..=16).contains(&spec.turns),
        "Thread requires 1..8 starts and 1..16 turns",
    )?;
    let shift = if spec.kind == Kind::External {
        -spec.clearance / 2.
    } else {
        spec.clearance / 2.
    };
    let crest = spec.diameter / 2. + shift;
    let depth = 3_f64.sqrt() * spec.pitch * 5. / 16.;
    let root = crest - depth;
    check(
        root > 0. && root < crest && crest <= 1e9,
        "Thread root must remain positive and radius within coordinate limits",
    )?;
    let lead = spec.pitch * spec.starts as f64;
    let maximum_z = lead * (spec.turns as f64 + 1.);
    check(
        maximum_z.is_finite() && maximum_z <= 1e9,
        "Thread axial size exceeds coordinate limits",
    )?;
    let direction = if spec.hand == Hand::Right { 1. } else { -1. };
    let quarter_angle = direction * std::f64::consts::FRAC_PI_2;
    let phases = [0., 1. / 16., 3. / 8., 5. / 8., 15. / 16., 1.];
    let radii = [crest, crest, root, root, crest, crest];
    let mut out = Vec::with_capacity(20 * spec.starts * spec.turns);
    for start in 0..spec.starts {
        for turn in 0..spec.turns {
            for quarter in 0..4 {
                let z_offset =
                    start as f64 * spec.pitch + (turn as f64 + quarter as f64 / 4.) * lead;
                for segment in 0..5 {
                    let profile = Curve::from_polyline(vec![
                        vec![radii[segment], 0., z_offset + phases[segment] * spec.pitch],
                        vec![
                            radii[segment + 1],
                            0.,
                            z_offset + phases[segment + 1] * spec.pitch,
                        ],
                    ])?;
                    check(
                        profile.control_points[0][2] < profile.control_points[1][2],
                        "Thread pitch phase collapses at coordinate precision",
                    )?;
                    let a = helical_sweep::approximate(
                        &profile,
                        lead / 4.,
                        quarter as f64 * quarter_angle,
                        quarter_angle,
                        spec.error_budget,
                    )?;
                    out.push(Patch {
                        surface: a.surface,
                        start,
                        turn,
                        quarter,
                        profile_segment: segment,
                        real_arithmetic_error_estimate: a.real_arithmetic_error_estimate,
                    });
                }
            }
        }
    }
    Ok(Patches {
        patches: out,
        crest_radius: crest,
        root_radius: root,
        lead,
        axial_extent: [0., maximum_z],
        rounding_certified: false,
    })
}

