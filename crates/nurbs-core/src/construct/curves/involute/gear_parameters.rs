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
    pub half_low_angle: [f64; 2],
    pub half_high_angle: [f64; 2],
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
pub(crate) fn atan_positive(mut x: I) -> Result<I> {
    check(
        x.lo >= 0. && x.hi <= 8.,
        "Certified gear roll arctangent must lie in 0..8",
    )?;
    if x.lo == 0. && x.hi == 0. {
        return Ok(I::point(0.));
    }
    let mut factor = 1.;
    for _ in 0..5 {
        if x.hi <= 0.5 {
            return atan_small(x)?.mul(I::point(factor));
        }
        let divisor = I::point(1.).add(sqrt_positive(I::point(1.).add(x.mul(x)?)?)?)?;
        x = x.div(divisor)?;
        factor *= 2.;
    }
    Err(crate::input(
        "Gear roll arctangent reduction exceeded its fixed budget",
    ))
}
pub(crate) fn pi() -> Result<I> {
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
    let half_base = half.add(inv_pitch)?;
    let involute_angle = |t: I| -> Result<I> {
        if t.lo == 0. && t.hi == 0. {
            Ok(I::point(0.))
        } else {
            t.sub(atan_positive(t)?)
        }
    };
    let half_low = half_base.sub(involute_angle(start)?)?;
    let half_high = half_base.sub(involute_angle(end)?)?;
    Ok(GearParameterBounds {
        pitch_radius: pair(pitch),
        base_radius: pair(base),
        tip_radius: pair(tip),
        root_radius: pair(root),
        roll_start: pair(start),
        roll_end: pair(end),
        half_base_angle: pair(half_base),
        half_low_angle: pair(half_low),
        half_high_angle: pair(half_high),
        tooth_step: pair(pi.mul(I::point(2.))?.div(I::point(teeth as f64))?),
    })
}

/// Outward total twist height*tan(beta)/pitch_radius, where beta is given in
/// degrees. Arguments are exact stored real data; no libm tangent is trusted.
pub fn derive_gear_twist_bounds(
    module: f64,
    teeth: usize,
    height: f64,
    helix_degrees: f64,
) -> Result<[f64; 2]> {
    check(
        module.is_finite()
            && (0.05..=100.).contains(&module)
            && (3..=256).contains(&teeth)
            && height.is_finite()
            && (0.05..=1000.).contains(&height)
            && helix_degrees.is_finite()
            && helix_degrees.abs() < 80.,
        "Gear twist requires supported finite module, tooth count, height and helix angle",
    )?;
    if helix_degrees == 0. {
        return Ok([0.; 2]);
    }
    let alpha = pi()?.mul(I::point(helix_degrees))?.div(I::point(180.))?;
    let (sin, cos) = trig(alpha)?;
    let pitch = I::point(module)
        .mul(I::point(teeth as f64))?
        .div(I::point(2.))?;
    Ok(pair(I::point(height).mul(sin.div(cos)?)?.div(pitch)?))
}

/// Transverse contact ratio for two nominal external spur gears at standard
/// centre distance, zero backlash and standard quarter-module clearance.
/// Uses the active involute overlap only. Root interference and manufacturing
/// undercut require separate checks.
pub fn external_spur_contact_ratio_bounds(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
) -> Result<[f64; 2]> {
    spur_contact_ratio(module, teeth, pressure_degrees, false, None)
}

/// Active transverse involute overlap for a pinion inside a larger spur ring.
/// Uses standard centre distance, zero backlash and quarter-module clearance.
/// This is a flank calculation; it does not establish trochoid clearance.
pub fn internal_spur_contact_ratio_bounds(
    module: f64,
    pinion_teeth: usize,
    ring_teeth: usize,
    pressure_degrees: f64,
) -> Result<[f64; 2]> {
    check(
        ring_teeth > pinion_teeth,
        "Ring must have more teeth than pinion",
    )?;
    spur_contact_ratio(
        module,
        [pinion_teeth, ring_teeth],
        pressure_degrees,
        true,
        None,
    )
}

/// Which common base-circle tangent is used for the mounted pair.
#[derive(Clone, Copy, Debug)]
pub enum SpurMesh {
    External,
    Internal,
}

/// Active flank overlap at a specified positive mounting distance. Both gears
/// retain their nominal tooth geometry. For internal meshes the ring is second.
/// An error indicates an invalid input, unresolved tangent or no proven overlap.
/// Backlash, undercut and material interference require separate checks.
pub fn mounted_spur_contact_ratio_bounds(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    mesh: SpurMesh,
    centre_distance: f64,
) -> Result<[f64; 2]> {
    spur_contact_ratio_over_mounting_range(
        module,
        teeth,
        pressure_degrees,
        mesh,
        [centre_distance; 2],
    )
}

/// Enclose the active contact ratio for every centre distance in the interval.
/// The overlap of two translated active flank intervals can peak strictly
/// inside the tolerance. Bounds account for both cross distances and both
/// interval widths, including changes of limiting flank within the tolerance.
pub fn spur_contact_ratio_over_mounting_range(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    mesh: SpurMesh,
    centre_distance: [f64; 2],
) -> Result<[f64; 2]> {
    let centres = I::new(centre_distance[0], centre_distance[1])?;
    check(centres.lo > 0., "Expected positive finite centre distance")?;
    let internal = matches!(mesh, SpurMesh::Internal);
    check(
        !internal || teeth[1] > teeth[0],
        "Ring must have more teeth than pinion",
    )?;
    spur_contact_ratio(module, teeth, pressure_degrees, internal, Some(centres))
}

fn contact_flank(
    module: f64,
    count: usize,
    pressure_degrees: f64,
    ring: bool,
) -> Result<(I, I, I)> {
    let gear =
        derive_gear_parameter_bounds(module, count, pressure_degrees, module / 4., 0., ring)?;
    check(
        gear.half_high_angle[0] > 0.,
        "Active flank angular extent must stay positive",
    )?;
    let base = I::new(gear.base_radius[0], gear.base_radius[1])?;
    let distance = |roll: [f64; 2]| base.mul(I::new(roll[0], roll[1])?);
    Ok((distance(gear.roll_start)?, distance(gear.roll_end)?, base))
}

fn spur_contact_ratio(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    internal: bool,
    mounted: Option<I>,
) -> Result<[f64; 2]> {
    let (low, high, base) = contact_flank(module, teeth[0], pressure_degrees, false)?;
    let (peer_low, peer_high, peer_base) =
        contact_flank(module, teeth[1], pressure_degrees, internal)?;
    let span = if let Some(centres) = mounted {
        let offset = if internal {
            peer_base.sub(base)?
        } else {
            peer_base.add(base)?
        };
        let gap = centres.sub(offset)?;
        check(
            gap.lo > 0.,
            "Common tangent requires positive base-circle separation margin",
        )?;
        sqrt_positive(gap.mul(centres.add(offset)?)?)?
    } else {
        let alpha = pi()?.mul(I::point(pressure_degrees))?.div(I::point(180.))?;
        let separation = if internal {
            teeth[1] - teeth[0]
        } else {
            teeth[0] + teeth[1]
        };
        let centres = I::point(module).mul(I::point(separation as f64 / 2.))?;
        centres.mul(trig(alpha)?.0)?
    };
    // Ring and pinion roll in the same tangent coordinate direction.
    let (peer_start, peer_end) = if internal {
        (peer_low.sub(span)?, peer_high.sub(span)?)
    } else {
        (span.sub(peer_high)?, span.sub(peer_low)?)
    };
    // min(endpoints) - max(starts) equals the smallest of the four
    // pairwise differences. Keep each flank's translation-invariant width.
    let limits = [
        high.sub(low)?,
        peer_high.sub(peer_low)?,
        high.sub(peer_start)?,
        peer_end.sub(low)?,
    ];
    let length = limits
        .into_iter()
        .reduce(|a, b| I {
            lo: a.lo.min(b.lo),
            hi: a.hi.min(b.hi),
        })
        .unwrap();
    check(length.lo > 0., "No certified positive flank overlap")?;
    let base_pitch = base
        .mul(pi()?)?
        .mul(I::point(2.))?
        .div(I::point(teeth[0] as f64))?;
    Ok(pair(length.div(base_pitch)?))
}

/// Signed radial tip-to-root margins on the line of centres for standard
/// full-depth spur profiles. The first interval is the first gear's tip
/// against the second root, and the second interval is the reverse pairing.
/// The two clearance inputs are root-depth allowances in model units.
/// Negative margins denote radial overlap. This does not test flank collisions.
pub fn spur_tip_root_clearance_bounds(
    module: f64,
    teeth: [usize; 2],
    root_allowances: [f64; 2],
    mesh: SpurMesh,
    centre_distance: f64,
) -> Result<[[f64; 2]; 2]> {
    check(
        centre_distance.is_finite() && centre_distance > 0.,
        "Expected positive finite centre distance",
    )?;
    let ring = matches!(mesh, SpurMesh::Internal);
    check(
        !ring || teeth[1] > teeth[0],
        "Ring must have more teeth than pinion",
    )?;
    // Pressure angle does not enter the nominal root and tip radii.
    let radii = |index: usize, internal| -> Result<(I, I)> {
        let p = derive_gear_parameter_bounds(
            module,
            teeth[index],
            20.,
            root_allowances[index],
            0.,
            internal,
        )?;
        Ok((
            I::new(p.tip_radius[0], p.tip_radius[1])?,
            I::new(p.root_radius[0], p.root_radius[1])?,
        ))
    };
    let (tip, root) = radii(0, false)?;
    let (peer_tip, peer_root) = radii(1, ring)?;
    let distance = I::point(centre_distance);
    let (first, second) = if ring {
        (
            peer_root.sub(distance)?.sub(tip)?,
            peer_tip.sub(distance)?.sub(root)?,
        )
    } else {
        (
            distance.sub(tip)?.sub(peer_root)?,
            distance.sub(peer_tip)?.sub(root)?,
        )
    };
    Ok([pair(first), pair(second)])
}

/// Nominal external spur-tooth caliper dimensions in model units.
#[derive(Clone, Copy, Debug)]
pub struct ToothChordBounds {
    pub chord_length: [f64; 2],
    /// Radial measuring depth from the tip circle to the midpoint of the chord.
    pub depth_from_tip: [f64; 2],
}

/// Measure between symmetric involute flanks where they cross a given circle.
/// Requires a radius provably within the active involute and a positive tooth
/// thickness there. Root transitions and manufacturing modifications are excluded.
pub fn spur_tooth_chord_bounds(
    module: f64,
    teeth: usize,
    pressure_degrees: f64,
    clearance: f64,
    backlash: f64,
    radius: f64,
) -> Result<ToothChordBounds> {
    let gear =
        derive_gear_parameter_bounds(module, teeth, pressure_degrees, clearance, backlash, false)?;
    let angle = flank_half_angle(&gear, I::new(radius, radius)?)?;
    let r = I::point(radius);
    let (sin, cos) = trig(angle)?;
    let tip = I::new(gear.tip_radius[0], gear.tip_radius[1])?;
    Ok(ToothChordBounds {
        chord_length: pair(r.mul(sin)?.mul(I::point(2.))?),
        depth_from_tip: pair(tip.sub(r.mul(cos)?)?),
    })
}

fn flank_half_angle(gear: &GearParameterBounds, r: I) -> Result<I> {
    check(
        r.lo >= gear.root_radius[1]
            .min(gear.tip_radius[1])
            .max(gear.base_radius[1])
            && r.hi <= gear.tip_radius[0].max(gear.root_radius[0]),
        "Measurement circle must lie on the active involute",
    )?;
    let base = I::new(gear.base_radius[0], gear.base_radius[1])?;
    let ratio = r.div(base)?;
    let roll = sqrt_positive(ratio.mul(ratio)?.sub(I::point(1.))?)?;
    let half_base = I::new(gear.half_base_angle[0], gear.half_base_angle[1])?;
    let angle = half_base.sub(roll.sub(atan_positive(roll)?)?)?;
    check(angle.lo > 0., "Measurement circle crosses a pointed tooth")?;
    Ok(angle)
}

/// Infer the single external tooth's pitch-circle thinning from an interval of
/// measured chord widths on a known circle. The returned enclosure is clipped
/// to the supported zero-to-half-module allowance. Assembly backlash also
/// depends on the mate and the mounting distance.
pub fn spur_tooth_thinning_bounds(
    module: f64,
    teeth: usize,
    pressure_degrees: f64,
    clearance: f64,
    radius: f64,
    measured_chord: [f64; 2],
) -> Result<[f64; 2]> {
    let gear = derive_gear_parameter_bounds(module, teeth, pressure_degrees, clearance, 0., false)?;
    let nominal = flank_half_angle(&gear, I::new(radius, radius)?)?;
    let chord = I::new(measured_chord[0], measured_chord[1])?;
    check(
        chord.lo > 0. && chord.hi < 2. * radius,
        "Expected positive chord widths below the diameter",
    )?;
    let half_angle = |width| -> Result<I> {
        let sine = I::point(width).div(I::point(radius).mul(I::point(2.))?)?;
        let cosine = sqrt_positive(I::point(1.).sub(sine.mul(sine)?)?)?;
        atan_positive(sine.div(cosine)?)
    };
    let scale = I::new(gear.pitch_radius[0], gear.pitch_radius[1])?.mul(I::point(2.))?;
    // Width decreases with thinning. Clip the measured range to the
    // unthinned upper bound before evaluating the inverse near the diameter.
    let nominal_width = I::point(radius).mul(I::point(2.))?.mul(trig(nominal)?.0)?;
    let feasible = I::new(chord.lo, chord.hi.min(nominal_width.hi))?;
    let observed = I::new(half_angle(feasible.lo)?.lo, half_angle(feasible.hi)?.hi)?;
    let allowance = nominal.sub(observed)?.mul(scale)?;
    Ok(pair(I::new(
        allowance.lo.max(0.),
        allowance.hi.min(module / 2.),
    )?))
}

/// Signed transverse circular backlash of a nominal external spur pair at its
/// working pitch point. Each thinning is specified on its reference pitch
/// circle. Negative values indicate angular overlap at the pitch point.
/// Root clearance is a quarter module; off-pitch interference is separate.
pub fn mounted_spur_backlash_bounds(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    thinning: [f64; 2],
    centre_distance: f64,
) -> Result<[f64; 2]> {
    spur_backlash(
        module,
        teeth,
        pressure_degrees,
        thinning,
        centre_distance,
        false,
    )
}

/// Signed ring space width minus pinion tooth thickness at their working
/// pitch point. The pinion is first. Positive allowances narrow the pinion
/// tooth and widen the ring space on their respective reference circles.
pub fn mounted_internal_spur_backlash_bounds(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    thinning: [f64; 2],
    centre_distance: f64,
) -> Result<[f64; 2]> {
    check(teeth[1] > teeth[0], "Ring must have more teeth than pinion")?;
    spur_backlash(
        module,
        teeth,
        pressure_degrees,
        thinning,
        centre_distance,
        true,
    )
}

fn spur_backlash(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    thinning: [f64; 2],
    centre_distance: f64,
    internal: bool,
) -> Result<[f64; 2]> {
    check(
        centre_distance.is_finite() && centre_distance > 0.,
        "Expected positive finite centre distance",
    )?;
    let mut widths = [I::point(0.); 2];
    let first = I::point(teeth[0] as f64);
    let second = I::point(teeth[1] as f64);
    let count = if internal {
        second.sub(first)?
    } else {
        second.add(first)?
    };
    for i in 0..2 {
        let gear = derive_gear_parameter_bounds(
            module,
            teeth[i],
            pressure_degrees,
            module / 4.,
            thinning[i],
            internal && i == 1,
        )?;
        let radius = I::point(centre_distance)
            .mul(I::point(teeth[i] as f64))?
            .div(count)?;
        let half_angle = flank_half_angle(&gear, radius)?;
        widths[i] = radius.mul(half_angle)?.mul(I::point(2.))?;
    }
    if internal {
        return Ok(pair(widths[1].sub(widths[0])?));
    }
    let pitch = pi()?
        .mul(I::point(2.))?
        .mul(I::point(centre_distance))?
        .div(count)?;
    Ok(pair(pitch.sub(widths[0].add(widths[1])?)?))
}

/// Enclose signed transverse circular backlash throughout a continuous centre
/// distance tolerance. Both endpoint working circles must be on active flanks.
/// Dividing backlash by distance removes the working-radius scale: the
/// remaining variable term is +/- twice the working involute function, which
/// increases with pressure angle. Thus the two endpoints enclose the normalized
/// backlash even when the dimensional backlash has an interior extremum.
pub fn spur_backlash_over_mounting_range(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    thinning: [f64; 2],
    mesh: SpurMesh,
    centre_distance: [f64; 2],
) -> Result<[f64; 2]> {
    let distance = I::new(centre_distance[0], centre_distance[1])?;
    let evaluate = match mesh {
        SpurMesh::External => mounted_spur_backlash_bounds,
        SpurMesh::Internal => mounted_internal_spur_backlash_bounds,
    };
    let normalized = |d: f64| -> Result<I> {
        let b = evaluate(module, teeth, pressure_degrees, thinning, d)?;
        I::new(b[0], b[1])?.div(I::point(d))
    };
    let low = normalized(distance.lo)?;
    let high = normalized(distance.hi)?;
    Ok(pair(
        distance.mul(I::new(low.lo.min(high.lo), low.hi.max(high.hi))?)?,
    ))
}

/// Inward-rounded centre-distance window guaranteeing at least the requested
/// active transverse contact ratio for nominal external or internal spur gears.
/// Solves the four flank-overlap inequalities in common-tangent coordinates.
/// This window only certifies active contact length; clearance is separate.
pub fn mounting_window_for_contact_ratio(
    module: f64,
    teeth: [usize; 2],
    pressure_degrees: f64,
    mesh: SpurMesh,
    minimum_ratio: f64,
) -> Result<[f64; 2]> {
    check(
        minimum_ratio.is_finite() && minimum_ratio > 0.,
        "Expected positive finite contact ratio",
    )?;
    let ring = matches!(mesh, SpurMesh::Internal);
    check(
        !ring || teeth[1] > teeth[0],
        "Ring must have more teeth than pinion",
    )?;
    let (low, high, base) = contact_flank(module, teeth[0], pressure_degrees, false)?;
    let (peer_low, peer_high, peer_base) = contact_flank(module, teeth[1], pressure_degrees, ring)?;
    let needed = base
        .mul(pi()?)?
        .mul(I::point(2.))?
        .div(I::point(teeth[0] as f64))?
        .mul(I::point(minimum_ratio))?;
    check(
        high.sub(low)?.lo.min(peer_high.sub(peer_low)?.lo) >= needed.hi,
        "Required overlap exceeds certified active flank width",
    )?;
    let (offset, lower, upper) = if ring {
        (
            peer_base.sub(base)?,
            peer_low.sub(high)?.add(needed)?,
            peer_high.sub(low)?.sub(needed)?,
        )
    } else {
        (
            peer_base.add(base)?,
            low.add(peer_low)?.add(needed)?,
            high.add(peer_high)?.sub(needed)?,
        )
    };
    check(
        upper.lo > 0.,
        "No positive tangent span for requested overlap",
    )?;
    let squared_offset = offset.mul(offset)?;
    let distance = |span: f64| {
        let s = I::point(span);
        sqrt_positive(squared_offset.add(s.mul(s)?)?)
    };
    let first = distance(lower.hi.max(0.))?.hi.max(offset.hi.next_up());
    let last = distance(upper.lo)?.lo;
    Ok(pair(I::new(first, last)?))
}

/// Signed separation between the tip-circle envelopes of two external planets.
/// Each pin has its own orbit-radius tolerance, measured from the carrier axis.
/// Supply the smaller included angle in degrees. A positive lower bound proves
/// separation for any tooth phase; overlapping envelopes need a profile check.
pub fn planet_tip_gap_bounds(
    module: f64,
    teeth: [usize; 2],
    orbit_radii: [[f64; 2]; 2],
    separation_degrees: [f64; 2],
) -> Result<[f64; 2]> {
    let orbit = |bounds: [f64; 2]| -> Result<I> {
        let r = I::new(bounds[0], bounds[1])?;
        check(r.lo > 0., "Expected positive carrier orbit radii")?;
        Ok(r)
    };
    let a = orbit(orbit_radii[0])?;
    let b = orbit(orbit_radii[1])?;
    let angle = I::new(separation_degrees[0], separation_degrees[1])?;
    check(
        angle.lo >= 0. && angle.hi <= 180.,
        "Included angle must be between zero and 180 degrees",
    )?;
    let half_sine =
        |deg| -> Result<I> { Ok(trig(pi()?.mul(I::point(deg))?.div(I::point(360.))?)?.0) };
    let sine = I::new(
        half_sine(angle.lo)?.lo.max(0.),
        half_sine(angle.hi)?.hi.min(1.),
    )?;
    // Chord squared = (a-b)^2 + 4ab sin^2(theta/2), stable even near coincident pins.
    let delta = a.sub(b)?;
    let min_abs = delta.lo.max(-delta.hi).max(0.);
    let abs = I::new(min_abs, delta.lo.abs().max(delta.hi.abs()))?;
    let chord = sqrt_positive(
        abs.mul(abs)?
            .add(a.mul(b)?.mul(I::point(4.))?.mul(sine)?.mul(sine)?)?,
    )?;
    let mut gap = chord;
    for count in teeth {
        let gear = derive_gear_parameter_bounds(module, count, 20., module / 4., 0., false)?;
        gap = gap.sub(I::new(gear.tip_radius[0], gear.tip_radius[1])?)?;
    }
    Ok(pair(gap))
}

/// Equally spaced planet counts for a concentric, unshifted spur train.
/// Each returned count satisfies the integer assembly-phase condition and
/// has certified adjacent tip-circle clearance at least the requested value.
/// Geometry is nominal; this is a layout filter, not a flank-contact proof.
pub fn planetary_layout_counts(
    module: f64,
    sun_teeth: usize,
    planet_teeth: usize,
    minimum_gap: f64,
) -> Result<Vec<usize>> {
    check(
        minimum_gap.is_finite() && minimum_gap >= 0.,
        "Expected finite nonnegative minimum planet gap",
    )?;
    check(
        (3..=256).contains(&sun_teeth) && (3..=256).contains(&planet_teeth),
        "Unsupported tooth count",
    )?;
    let ring = sun_teeth + 2 * planet_teeth;
    check(ring <= 256, "Derived ring exceeds supported tooth count")?;
    let planet = derive_gear_parameter_bounds(module, planet_teeth, 20., module / 4., 0., false)?;
    let diameter = I::new(planet.tip_radius[0], planet.tip_radius[1])?.mul(I::point(2.))?;
    let orbit_diameter = I::point(module).mul(I::point((sun_teeth + planet_teeth) as f64))?;
    let phase = sun_teeth + ring;
    let pi = pi()?;
    let mut layouts = Vec::new();
    for count in 2..=phase {
        if phase.is_multiple_of(count) {
            let sine = trig(pi.div(I::point(count as f64))?)?.0;
            let gap = orbit_diameter.mul(sine)?.sub(diameter)?;
            if gap.lo >= minimum_gap {
                layouts.push(count);
            }
        }
    }
    Ok(layouts)
}
