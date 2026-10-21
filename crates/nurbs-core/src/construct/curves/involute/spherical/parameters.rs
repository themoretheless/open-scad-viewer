//! Outward nominal straight-bevel cone geometry. Angles are exact real
//! binary64 radians. No Gleason profile shift, cutter/root fillet or undercut.
use super::super::{
    gear_parameters::{atan_positive, pi},
    quintic_certificate::trig,
};
use super::*;
use crate::distance_bounds::Interval as I;

#[derive(Clone, Copy, Debug)]
pub struct BevelGearSpec {
    pub module: f64,
    pub teeth: usize,
    pub pitch_cone_angle: f64,
    pub pressure_angle: f64,
    /// Radial cone-distance width, not axial thickness.
    pub face_width: f64,
    pub addendum: f64,
    pub dedendum: f64,
    /// Pitch-circle arc-length reduction of tooth thickness.
    pub backlash: f64,
}
impl Default for BevelGearSpec {
    fn default() -> Self {
        Self {
            module: 1.,
            teeth: 20,
            pitch_cone_angle: std::f64::consts::FRAC_PI_4,
            pressure_angle: 20_f64.to_radians(),
            face_width: 3.,
            addendum: 1.,
            dedendum: 1.25,
            backlash: 0.,
        }
    }
}
#[derive(Clone, Debug)]
pub struct BevelGearParameterBounds {
    pub cone_distances: [[f64; 2]; 2],
    pub base_cone_angle: [f64; 2],
    pub tip_cone_angle: [f64; 2],
    pub root_cone_angle: [f64; 2],
    pub roll: [[f64; 2]; 2],
    pub pitch_roll: [f64; 2],
    pub half_pitch_tooth_angle: [f64; 2],
    pub pitch_involute_azimuth: [f64; 2],
    pub tooth_step: [f64; 2],
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
pub(super) fn atan(x: I) -> Result<I> {
    check(x.lo >= 0., "Bevel arctangent needs a nonnegative argument")?;
    if x.lo > 1. {
        pi()?
            .div(I::point(2.))?
            .sub(atan_positive(I::point(1.).div(x)?)?)
    } else {
        atan_positive(x)
    }
}
/// Proven monotone sine bracketing. Overlap stops refinement but never
/// narrows the bracket without a strict outward comparison.
fn asin(x: I) -> Result<I> {
    check(
        x.lo >= 0. && x.hi < 1.,
        "Bevel inverse sine must stay in [0,1)",
    )?;
    if x.hi == 0. {
        return Ok(I::point(0.));
    }
    let mut lo = 0.;
    let mut hi = pi()?.lo * 0.5;
    check(
        trig(I::point(hi))?.0.lo > x.hi,
        "Bevel inverse sine upper bracket unproved",
    )?;
    for _ in 0..80 {
        let mid = lo + (hi - lo) * 0.5;
        if mid <= lo || mid >= hi {
            break;
        }
        let value = trig(I::point(mid))?.0;
        if value.hi < x.lo {
            lo = mid;
        } else if value.lo > x.hi {
            hi = mid;
        } else {
            break;
        }
    }
    I::new(lo, hi)
}
fn acos(x: I) -> Result<I> {
    pi()?.div(I::point(2.))?.sub(asin(x)?)
}
/// Derive complete nominal pitch/base/tip/root cone and flank roll boxes.
/// The addendum/dedendum cone increments are atan(height/outer cone distance).
/// Face width reduces cone distance towards the apex. This describes sharp
/// theoretical cones, not a manufacturing or conjugate-meshing certificate.
pub fn derive_bevel_gear_parameters(spec: BevelGearSpec) -> Result<BevelGearParameterBounds> {
    check(
        [
            spec.module,
            spec.pitch_cone_angle,
            spec.pressure_angle,
            spec.face_width,
            spec.addendum,
            spec.dedendum,
            spec.backlash,
        ]
        .iter()
        .all(|x| x.is_finite())
            && (0.05..=100.).contains(&spec.module)
            && (6..=256).contains(&spec.teeth)
            && (0.1..=1.4).contains(&spec.pitch_cone_angle)
            && (0.15..=0.65).contains(&spec.pressure_angle)
            && spec.face_width > 0.
            && spec.addendum > 0.
            && spec.addendum <= spec.module * 2.
            && spec.dedendum >= spec.addendum
            && spec.dedendum <= spec.module * 3.
            && spec.backlash >= 0.
            && spec.backlash < spec.module,
        "Bevel spec has invalid module, teeth, cone/pressure angle, width or tooth allowances",
    )?;
    let pitch = I::point(spec.pitch_cone_angle);
    let (sp, cp) = trig(pitch)?;
    let (_, ca) = trig(I::point(spec.pressure_angle))?;
    let radius = I::point(spec.module)
        .mul(I::point(spec.teeth as f64))?
        .div(I::point(2.))?;
    let outer = radius.div(sp)?;
    let inner = outer.sub(I::point(spec.face_width))?;
    check(inner.lo > 0., "Bevel face width reaches or passes the apex")?;
    let base = asin(sp.mul(ca)?)?;
    let tip = pitch.add(atan(I::point(spec.addendum).div(outer)?)?)?;
    let root = pitch.sub(atan(I::point(spec.dedendum).div(outer)?)?)?;
    check(
        root.lo > 0. && tip.hi < pi()?.lo * 0.5 && base.hi < tip.lo,
        "Bevel root/tip cones reach axis/equator or do not separate from the base cone",
    )?;
    let (sb, cb) = trig(base)?;
    let roll_at = |angle: I| -> Result<I> {
        let (_, c) = trig(angle)?;
        acos(c.div(cb)?)?.div(sb)
    };
    let start = if root.hi < base.lo {
        I::point(0.)
    } else {
        check(
            root.lo > base.hi,
            "Bevel root/base cone ordering is unresolved",
        )?;
        roll_at(root)?
    };
    let end = roll_at(tip)?;
    let pitch_roll = acos(cp.div(cb)?)?.div(sb)?;
    check(
        start.hi < end.lo && end.hi <= 8.,
        "Bevel flank roll interval is unproved or exceeds 8",
    )?;
    let w = sb.mul(pitch_roll)?;
    let (sw, cw) = trig(w)?;
    let azimuth = pitch_roll.sub(atan(sw.div(cw.mul(sb)?)?)?)?;
    let half = pi()?
        .div(I::point(2. * spec.teeth as f64))?
        .sub(I::point(spec.backlash).div(radius.mul(I::point(2.))?)?)?;
    check(
        half.lo > 0.,
        "Bevel backlash consumes the entire pitch tooth thickness",
    )?;
    Ok(BevelGearParameterBounds {
        cone_distances: [pair(inner), pair(outer)],
        base_cone_angle: pair(base),
        tip_cone_angle: pair(tip),
        root_cone_angle: pair(root),
        roll: [pair(start), pair(end)],
        pitch_roll: pair(pitch_roll),
        half_pitch_tooth_angle: pair(half),
        pitch_involute_azimuth: pair(azimuth),
        tooth_step: pair(pi()?.mul(I::point(2.))?.div(I::point(spec.teeth as f64))?),
    })
}

fn midpoint(x: [f64; 2]) -> f64 {
    x[0] + (x[1] - x[0]) * 0.5
}
fn difference(value: f64, bounds: [f64; 2]) -> Result<f64> {
    let d = I::point(value).sub(I::new(bounds[0], bounds[1])?)?;
    Ok(d.lo.abs().max(d.hi.abs()))
}
fn nominal_error(patches: &QualifiedBevelPatches, p: &BevelGearParameterBounds) -> Result<f64> {
    let source = patches.source();
    let distances = patches.cone_distances();
    let angle = difference(source.base_cone_angle, p.base_cone_angle)?;
    let roll = difference(source.roll[0], p.roll[0])?.max(difference(source.roll[1], p.roll[1])?);
    let radial = difference(distances[0], p.cone_distances[0])?
        .max(difference(distances[1], p.cone_distances[1])?);
    let tmax = source.roll[1].max(p.roll[1][1]);
    let rmax = distances[1].max(p.cone_distances[1][1]);
    // ||dq/d(base angle)|| <= 1+|roll|, ||dq/droll|| <= 1,
    // ||q||=1. Convex interpolation bounds both endpoint roll/radius errors.
    let direction = I::point(1.)
        .add(I::point(tmax))?
        .mul(I::point(angle))?
        .add(I::point(roll))?;
    I::point(radial)
        .add(I::point(rmax).mul(direction)?)?
        .add(I::point(patches.certificate().error_upper))
        .map(|x| x.hi)
}
/// Whole working flank bound to original nominal module/tooth/cone inputs.
/// This still does not contain left/right tooth placement or a closed gear.
pub struct QualifiedNominalBevelFlank {
    spec: BevelGearSpec,
    parameters: BevelGearParameterBounds,
    patches: QualifiedBevelPatches,
    tolerance: f64,
    error_upper: f64,
}
impl QualifiedNominalBevelFlank {
    pub fn specification(&self) -> BevelGearSpec {
        self.spec
    }
    pub fn parameters(&self) -> &BevelGearParameterBounds {
        &self.parameters
    }
    pub fn patches(&self) -> &[Surface] {
        self.patches.patches()
    }
    pub fn error_upper(&self) -> f64 {
        self.error_upper
    }
    pub fn reverify(&self) -> Result<()> {
        self.patches.reverify()?;
        let fresh = derive_bevel_gear_parameters(self.spec)?;
        let error = nominal_error(&self.patches, &fresh)?;
        check(
            error <= self.tolerance && error == self.error_upper,
            "Nominal bevel source or inclusive error receipt changed",
        )
    }
}
pub fn qualified_nominal_bevel_flank(
    spec: BevelGearSpec,
    tolerance: f64,
    max_patches: usize,
) -> Result<QualifiedNominalBevelFlank> {
    check(
        tolerance.is_finite() && tolerance > 0.,
        "Nominal bevel tolerance must be finite positive",
    )?;
    let parameters = derive_bevel_gear_parameters(spec)?;
    let source = SphericalInvoluteSpec {
        base_cone_angle: midpoint(parameters.base_cone_angle),
        roll: parameters.roll.map(midpoint),
    };
    let patches = qualified_straight_bevel_patches(
        source,
        parameters.cone_distances.map(midpoint),
        tolerance * 0.5,
        max_patches,
    )?;
    let error_upper = nominal_error(&patches, &parameters)?;
    check(
        error_upper <= tolerance,
        "Nominal bevel parameter and surface error exceeds requested tolerance",
    )?;
    Ok(QualifiedNominalBevelFlank {
        spec,
        parameters,
        patches,
        tolerance,
        error_upper,
    })
}
