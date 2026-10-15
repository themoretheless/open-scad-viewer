//! Outward parameter derivation for the nominal involute gear family.
use super::quintic_certificate::trig;
use crate::{Result, check, distance_bounds::Interval as I};
#[derive(Clone, Debug)]
pub struct GearParameterBounds {
    pub pitch_radius: [f64; 2],
    pub base_radius: [f64; 2],
    pub tip_radius: [f64; 2],
    pub root_radius: [f64; 2],
    pub roll_start: [f64; 2],
    pub roll_end: [f64; 2],
    pub half_base_angle: [f64; 2],
    pub tooth_step: [f64; 2],
}
fn pair(x: I) -> [f64; 2] {
    [x.lo, x.hi]
}
fn sqrt_positive(x: I) -> Result<I> {
    I::new(
        x.lo.max(0.).sqrt().next_down().max(0.),
        x.hi.max(0.).sqrt().next_up(),
    )
}
/// Alternating arctangent series, with a symmetric bound for the first omitted
/// term. Endpoint interval uncertainty is propagated through every operation.
fn atan_small(x: I) -> Result<I> {
    check(
        x.lo >= -0.5 && x.hi <= 0.5,
        "Arctangent reduced argument exceeds certified range",
    )?;
    let xx = x.mul(x)?;
    let mut power = x;
    let mut sum = I::point(0.);
    for k in 0..32 {
        let term = power.div(I::point((2 * k + 1) as f64))?;
        sum = if k % 2 == 0 {
            sum.add(term)?
        } else {
            sum.sub(term)?
        };
        power = power.mul(xx)?;
    }
    let error = power.lo.abs().max(power.hi.abs()) / 65.;
    sum.add(I::new(-error.next_up(), error.next_up())?)
}
fn pi() -> Result<I> {
    // Machin's identity pi = 16 atan(1/5) - 4 atan(1/239).
    // No f64 PI, libm atan or sampled identity is trusted as evidence.
    atan_small(I::point(1.).div(I::point(5.))?)?
        .mul(I::point(16.))?
        .sub(atan_small(I::point(1.).div(I::point(239.))?)?.mul(I::point(4.))?)
}
/// Exact nominal module/tooth/degree-pressure/backlash/clearance derivation.
/// Binary64 arguments denote exact real input data. Machin/Taylor bounds,
/// outward arithmetic and bracketed square roots cover all rounding. This
/// describes a sharp involute profile, not undercut/root fillets or strength.
pub fn derive_gear_parameter_bounds(
    module: f64,
    teeth: usize,
    pressure_degrees: f64,
    clearance: f64,
    backlash: f64,
    internal: bool,
) -> Result<GearParameterBounds> {
    check(
        [module, pressure_degrees, clearance, backlash]
            .iter()
            .all(|x| x.is_finite())
            && (0.05..=100.).contains(&module)
            && (3..=256).contains(&teeth)
            && (10. ..=35.).contains(&pressure_degrees)
            && clearance >= 0.
            && clearance <= module
            && backlash >= 0.
            && backlash <= module / 2.,
        "Gear parameter bounds require finite supported module, tooth count, pressure and allowances",
    )?;
    let m = I::point(module);
    let pitch = m.mul(I::point(teeth as f64))?.div(I::point(2.))?;
    let pi = pi()?;
    let alpha = pi.mul(I::point(pressure_degrees))?.div(I::point(180.))?;
    let (sin, cos) = trig(alpha)?;
    let base = pitch.mul(cos)?;
    let (tip, root) = if internal {
        (pitch.sub(m)?, pitch.add(m)?.add(I::point(clearance))?)
    } else {
        (pitch.add(m)?, pitch.sub(m)?.sub(I::point(clearance))?)
    };
    check(
        root.lo > 0. && tip.lo > 0.,
        "Gear radius enclosure must stay positive",
    )?;
    let (low, high) = if internal { (tip, root) } else { (root, tip) };
    let roll = |r: I| -> Result<I> {
        let ratio = r.div(base)?;
        sqrt_positive(ratio.mul(ratio)?.sub(I::point(1.))?)
    };
    let start = if low.hi < base.lo {
        I::point(0.)
    } else if low.lo > base.hi {
        roll(low)?
    } else {
        // Do not lose correlation when the lower radius equals the base:
        // max(base,low)/base is >=1 and the possible roll starts at zero.
        let upper = roll(I::point(low.hi))?.hi;
        I::new(0., upper)?
    };
    let end = roll(high)?;
    let inv_pitch = sin.div(cos)?.sub(alpha)?;
    let allowance = I::point(backlash).div(pitch.mul(I::point(2.))?)?;
    let half = pi.div(I::point(2. * teeth as f64))?;
    let half = if internal {
        half.add(allowance)?
    } else {
        half.sub(allowance)?
    };
    Ok(GearParameterBounds {
        pitch_radius: pair(pitch),
        base_radius: pair(base),
        tip_radius: pair(tip),
        root_radius: pair(root),
        roll_start: pair(start),
        roll_end: pair(end),
        half_base_angle: pair(half.add(inv_pitch)?),
        tooth_step: pair(pi.mul(I::point(2.))?.div(I::point(teeth as f64))?),
    })
}
