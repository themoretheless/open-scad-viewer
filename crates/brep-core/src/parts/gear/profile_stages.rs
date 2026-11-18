//! Stages of `profile_with_report`: spec validation, the derived tooth frame,
//! per-tooth outline assembly with flank certification, and the final report.
use super::*;
use nurbs_core::involute::GearParameterBounds;
use std::ops::RangeInclusive;

const TEETH_RANGE: RangeInclusive<usize> = 3..=256;
const MODULE_RANGE: RangeInclusive<f64> = 0.05..=100.;
const PRESSURE_ANGLE_RANGE: RangeInclusive<f64> = 10.0..=35.;
const HEIGHT_RANGE: RangeInclusive<f64> = 0.05..=1000.;
/// Exclusive bound on the absolute helix angle, in degrees.
const HELIX_ANGLE_LIMIT: f64 = 80.;
const OUTSIDE_RADIUS_MAX: f64 = 10000.;
/// Margin by which an internal tip must clear the base circle.
const INTERNAL_TIP_BASE_MARGIN: f64 = 1e-9;
/// Radius gap above which the flank starts above the low circle.
const ROOT_LEAD_EPS: f64 = 1e-12;
/// Angular margin against a collapsing or overlapping tooth.
const COLLAPSE_MARGIN: f64 = 1e-6;
/// Smallest root fillet angle that is authored as a fillet.
const FILLET_ANGLE_EPS: f64 = 1e-9;
/// Smallest half-angle kept for a rounded arc.
const MIN_ARC_HALF: f64 = 1e-5;
/// Endpoint join tolerance passed to the profile closer.
const JOIN_TOLERANCE: f64 = 1e-7;
const EXTERNAL_ROOT_FILLET_RADIAL: f64 = 0.72;
const EXTERNAL_ROOT_FILLET_GAP: f64 = 0.44;
const INTERNAL_ROOT_FILLET_MODULE: f64 = 0.18;
const INTERNAL_ROOT_FILLET_GAP: f64 = 0.35;
const TIP_FILLET_MODULE: f64 = 0.18;
const TIP_FILLET_HALF: f64 = 0.35;
const FLANK_JOIN_MODULE: f64 = 0.14;
const FLANK_JOIN_SPAN: f64 = 0.25;

pub(super) fn validate_spec(spec: &GearSpec) -> Result<()> {
    if ![
        spec.module,
        spec.pressure_angle_deg,
        spec.height,
        spec.helix_angle_deg,
        spec.bore,
        spec.rim_width,
        spec.clearance,
        spec.backlash,
    ]
    .iter()
    .all(|x| x.is_finite())
    {
        return Err(invalid(
            "Gear dimensions, angles and allowances must be finite",
        ));
    }
    if !TEETH_RANGE.contains(&spec.teeth) {
        return Err(invalid("Gear teeth must be 3..256"));
    }
    if !MODULE_RANGE.contains(&spec.module) {
        return Err(invalid("Gear module must be 0.05..100 mm"));
    }
    if !PRESSURE_ANGLE_RANGE.contains(&spec.pressure_angle_deg) {
        return Err(invalid("Gear pressure angle must be 10..35 degrees"));
    }
    if !HEIGHT_RANGE.contains(&spec.height) {
        return Err(invalid("Gear height must be 0.05..1000 mm"));
    }
    if !spec.helix_angle_deg.is_finite() || spec.helix_angle_deg.abs() >= HELIX_ANGLE_LIMIT {
        return Err(invalid("Gear helix angle must be within -80..80 degrees"));
    }
    if spec.backlash < 0.
        || spec.backlash > spec.module / 2.
        || spec.clearance < 0.
        || spec.clearance > spec.module
    {
        return Err(invalid(
            "Gear backlash must be 0..module/2 and clearance 0..module",
        ));
    }
    Ok(())
}

/// Radii and angles derived from a validated spec.
pub(super) struct ToothFrame {
    internal: bool,
    module: f64,
    pitch: f64,
    base: f64,
    tip: f64,
    root: f64,
    outside: f64,
    low: f64,
    high: f64,
    start: f64,
    inv_pitch: f64,
    half_pitch: f64,
    low_half: f64,
    high_half: f64,
    step: f64,
    round_corners: bool,
    root_fillet_angle: f64,
    tip_fillet_angle: f64,
    t0: f64,
    t1: f64,
    t0_flank: f64,
    flank_start_half: f64,
}

fn roll(r: f64, base: f64) -> f64 {
    ((r / base).powi(2) - 1.).max(0.).sqrt()
}

impl ToothFrame {
    /// Tooth half-angle at radius `r`.
    fn half(&self, r: f64) -> f64 {
        let t = roll(r, self.base);
        self.half_pitch + self.inv_pitch - (t - t.atan())
    }

    fn has_root_lead(&self) -> bool {
        self.start > self.low + ROOT_LEAD_EPS
    }

    fn tip_arc_half(&self) -> f64 {
        (self.high_half - self.tip_fillet_angle).max(MIN_ARC_HALF)
    }

    pub(super) fn new(spec: &GearSpec) -> Result<Self> {
        let teeth = spec.teeth as f64;
        let alpha = spec.pressure_angle_deg.to_radians();
        let pitch = spec.module * teeth / 2.;
        let base = pitch * alpha.cos();
        let (tip, root) = if spec.internal {
            (pitch - spec.module, pitch + spec.module + spec.clearance)
        } else {
            (pitch + spec.module, pitch - spec.module - spec.clearance)
        };
        let outside = if spec.internal {
            root + spec.rim_width
        } else {
            tip
        };
        Self::check_radii(spec, base, tip, root, outside)?;
        let low = if spec.internal { tip } else { root };
        let high = if spec.internal { root } else { tip };
        let start = base.max(low);
        let inv_pitch = alpha.tan() - alpha;
        let half_pitch = PI / (2. * teeth)
            + if spec.internal {
                spec.backlash / (2. * pitch)
            } else {
                -spec.backlash / (2. * pitch)
            };
        let mut frame = Self {
            internal: spec.internal,
            module: spec.module,
            pitch,
            base,
            tip,
            root,
            outside,
            low,
            high,
            start,
            inv_pitch,
            half_pitch,
            low_half: 0.,
            high_half: 0.,
            step: TAU / teeth,
            round_corners: spec.backlash > 0.,
            root_fillet_angle: 0.,
            tip_fillet_angle: 0.,
            t0: 0.,
            t1: 0.,
            t0_flank: 0.,
            flank_start_half: 0.,
        };
        frame.low_half = frame.half(start);
        frame.high_half = frame.half(high);
        if frame.high_half <= COLLAPSE_MARGIN || frame.low_half >= frame.step / 2. - COLLAPSE_MARGIN
        {
            return Err(invalid(
                "Gear tooth profile collapses or overlaps; change teeth, pressure angle, backlash or clearance",
            ));
        }
        frame.derive_fillets();
        Ok(frame)
    }

    fn check_radii(spec: &GearSpec, base: f64, tip: f64, root: f64, outside: f64) -> Result<()> {
        if root <= 0. || outside > OUTSIDE_RADIUS_MAX {
            return Err(invalid(
                "Gear root radius must be positive and outside radius at most 10000 mm",
            ));
        }
        if spec.internal {
            if tip <= base + INTERNAL_TIP_BASE_MARGIN {
                return Err(invalid(
                    "Internal gear tip must stay outside the base circle",
                ));
            }
            if spec.bore != 0. {
                return Err(invalid(
                    "Internal gear has a toothed opening; bore must be zero",
                ));
            }
            if spec.rim_width < spec.module / 4. {
                return Err(invalid(
                    "Internal gear rim must be at least a quarter module",
                ));
            }
        } else if spec.bore < 0. || (spec.bore > 0. && spec.bore > 2. * root - spec.module / 2.) {
            return Err(invalid(
                "Gear bore must leave a quarter module inside the root circle",
            ));
        }
        Ok(())
    }

    fn derive_fillets(&mut self) {
        let (low, high, start, module) = (self.low, self.high, self.start, self.module);
        let external_lead = !self.internal && self.round_corners && self.has_root_lead();
        self.root_fillet_angle = if external_lead {
            let gap_half = (self.step / 2. - self.low_half).max(0.);
            ((start - low) * EXTERNAL_ROOT_FILLET_RADIAL / low)
                .min(gap_half * EXTERNAL_ROOT_FILLET_GAP)
        } else if self.internal && self.round_corners {
            let gap_half = (self.step / 2. - self.low_half).max(0.);
            (INTERNAL_ROOT_FILLET_MODULE * module / low).min(gap_half * INTERNAL_ROOT_FILLET_GAP)
        } else {
            0.
        };
        self.tip_fillet_angle = if self.round_corners {
            (TIP_FILLET_MODULE * module / high).min(self.high_half * TIP_FILLET_HALF)
        } else {
            0.
        };
        self.t0 = roll(start, self.base);
        self.t1 = roll(high, self.base);
        self.t0_flank = if external_lead {
            let r_join =
                (start + FLANK_JOIN_MODULE * module).min(start + FLANK_JOIN_SPAN * (high - start));
            roll(r_join, self.base)
        } else {
            self.t0
        };
        self.flank_start_half = self.half(self.base * self.t0_flank.hypot(1.));
    }
}

/// Accumulates the tooth outline and flank certification across teeth.
pub(super) struct ToothBuilder<'a> {
    spec: &'a GearSpec,
    frame: &'a ToothFrame,
    ideal: Option<&'a GearParameterBounds>,
    tooth: Vec<Curve>,
    flank_deviation: f64,
    full_flank_certified: bool,
    full_flank_deviation: f64,
}

fn xy(p: &[f64]) -> [f64; 2] {
    [p[0], p[1]]
}

fn missing_poles() -> Error {
    invalid("Gear flank must keep at least two control points")
}

impl<'a> ToothBuilder<'a> {
    pub(super) fn new(
        spec: &'a GearSpec,
        frame: &'a ToothFrame,
        ideal: Option<&'a GearParameterBounds>,
    ) -> Self {
        Self {
            spec,
            frame,
            ideal,
            tooth: Vec::new(),
            flank_deviation: 0.,
            full_flank_certified: ideal.is_some(),
            full_flank_deviation: 0.,
        }
    }

    /// Appends tooth `i`, counter-clockwise, starting at the low radius on
    /// the leading side.
    pub(super) fn push_tooth(&mut self, i: usize) -> Result<()> {
        let f = self.frame;
        let center = i as f64 * f.step + if f.internal { f.step / 2. } else { 0. };
        let half = |r: f64| f.half(r);
        let (mut rising, d1) = involute(f.base, f.t0_flank, f.t1, center, -1., &half)?;
        let (mut falling, d2) = involute(f.base, f.t1, f.t0_flank, center, 1., &half)?;
        self.flank_deviation = self.flank_deviation.max(d1).max(d2);
        if f.internal && !f.round_corners {
            self.check_reversed_flanks(&rising, &falling, center)?;
        }
        if let Some(ideal) = (!f.round_corners).then_some(self.ideal).flatten() {
            self.certify_ideal_flanks(ideal, i, &rising, &falling)?;
        }
        if f.round_corners {
            self.round_corners(&mut rising, &mut falling, center);
        }
        if f.has_root_lead() {
            self.push_leading_root(&rising, center)?;
        }
        let falling_last = xy(falling.control_points.last().ok_or_else(missing_poles)?);
        let falling_prev = xy(falling
            .control_points
            .len()
            .checked_sub(2)
            .and_then(|k| falling.control_points.get(k))
            .ok_or_else(missing_poles)?);
        self.tooth.push(rising);
        let tip_arc_half = f.tip_arc_half();
        self.tooth
            .push(arc2(f.high, center - tip_arc_half, center + tip_arc_half)?);
        self.tooth.push(falling);
        if f.has_root_lead() {
            self.push_trailing_root(falling_last, falling_prev, center);
        }
        self.tooth.push(arc2(
            f.low,
            center + f.low_half + f.root_fillet_angle,
            center + f.step - f.low_half - f.root_fillet_angle,
        )?);
        Ok(())
    }

    /// The hole reverses each flank a second time. Recheck the actual saved
    /// reverse knots rather than assuming subtraction is exact.
    fn check_reversed_flanks(
        &mut self,
        rising: &Curve,
        falling: &Curve,
        center: f64,
    ) -> Result<()> {
        let f = self.frame;
        for (curve, from, to, sign) in [(rising, f.t1, f.t0, -1.), (falling, f.t0, f.t1, 1.)] {
            let saved = reverse2(curve);
            let proof = nurbs_core::involute::certify_quintic_placed(
                &saved,
                [0.; 3],
                f.base,
                from,
                to,
                center + sign * f.half(f.base),
                -sign,
            )?;
            if proof.error_upper > FLANK_FIT {
                return Err(invalid(
                    "Reversed internal flank exceeds the inclusive allowance",
                ));
            }
            self.flank_deviation = self.flank_deviation.max(proof.error_upper);
        }
        Ok(())
    }

    fn certify_ideal_flanks(
        &mut self,
        ideal: &GearParameterBounds,
        i: usize,
        rising: &Curve,
        falling: &Curve,
    ) -> Result<()> {
        use nurbs_core::interval_eval::Interval as I;
        let internal = self.spec.internal;
        let step = I::new(ideal.tooth_step[0], ideal.tooth_step[1])?;
        let nominal_center = step.mul(I::point(i as f64 + if internal { 0.5 } else { 0. }))?;
        let half = I::new(ideal.half_base_angle[0], ideal.half_base_angle[1])?;
        for (curve, from, to, sign) in [
            (rising, ideal.roll_start, ideal.roll_end, -1.),
            (falling, ideal.roll_end, ideal.roll_start, 1.),
        ] {
            let angle = nominal_center.add(half.mul(I::point(sign))?)?;
            let reversed = internal.then(|| reverse2(curve));
            let curve = reversed.as_ref().unwrap_or(curve);
            let (from, to) = if internal { (to, from) } else { (from, to) };
            match nurbs_core::involute::certify_quintic_parameter_box(
                curve,
                [0.; 3],
                ideal.base_radius,
                from,
                to,
                [angle.lo, angle.hi],
                -sign,
            ) {
                Ok(certificate) => {
                    self.full_flank_deviation =
                        self.full_flank_deviation.max(certificate.error_upper);
                    self.full_flank_certified &= certificate.error_upper <= FLANK_FIT;
                }
                Err(_) => self.full_flank_certified = false,
            }
        }
        Ok(())
    }

    fn round_corners(&mut self, rising: &mut Curve, falling: &mut Curve, center: f64) {
        let f = self.frame;
        self.full_flank_certified = false;
        if f.internal && !f.has_root_lead() {
            let low_arc_half = f.low_half + f.root_fillet_angle;
            let high_arc_half = f.tip_arc_half();
            round_internal_flank_corners(
                rising,
                f.low,
                f.high,
                center,
                low_arc_half,
                high_arc_half,
                true,
            );
            round_internal_flank_corners(
                falling,
                f.low,
                f.high,
                center,
                low_arc_half,
                high_arc_half,
                false,
            );
        } else {
            let tip_arc_half = f.tip_arc_half();
            round_flank_tip_corner(rising, f.high, center, tip_arc_half, true);
            round_flank_tip_corner(falling, f.high, center, tip_arc_half, false);
        }
    }

    fn push_leading_root(&mut self, rising: &Curve, center: f64) -> Result<()> {
        let f = self.frame;
        if f.root_fillet_angle > FILLET_ANGLE_EPS {
            let a0 = center - f.low_half - f.root_fillet_angle;
            let p0 = polar(f.low, a0);
            let t0 = [-a0.sin(), a0.cos()];
            let p2 = xy(rising.control_points.first().ok_or_else(missing_poles)?);
            let p1_pole = rising.control_points.get(1).ok_or_else(missing_poles)?;
            let t2 = [p1_pole[0] - p2[0], p1_pole[1] - p2[1]];
            let p1 = tangent_fillet_control_point(
                p0,
                t0,
                p2,
                t2,
                polar(f.low, center - f.flank_start_half),
            );
            self.tooth.push(corner_fillet2(p0, p1, p2));
        } else {
            self.tooth.push(line2(
                polar(f.low, center - f.low_half),
                polar(f.start, center - f.low_half),
            ));
        }
        Ok(())
    }

    fn push_trailing_root(&mut self, falling_last: [f64; 2], falling_prev: [f64; 2], center: f64) {
        let f = self.frame;
        if f.root_fillet_angle > FILLET_ANGLE_EPS {
            let a2 = center + f.low_half + f.root_fillet_angle;
            let (p0, p2) = (falling_last, polar(f.low, a2));
            let t0 = [p0[0] - falling_prev[0], p0[1] - falling_prev[1]];
            let t2 = [-a2.sin(), a2.cos()];
            let p1 = tangent_fillet_control_point(
                p0,
                t0,
                p2,
                t2,
                polar(f.low, center + f.flank_start_half),
            );
            self.tooth.push(corner_fillet2(p0, p1, p2));
        } else {
            self.tooth.push(line2(
                polar(f.start, center + f.low_half),
                polar(f.low, center + f.low_half),
            ));
        }
    }

    pub(super) fn finish(self) -> (Vec<Curve>, FlankTally) {
        (
            self.tooth,
            FlankTally {
                flank_deviation: self.flank_deviation,
                full_flank_certified: self.full_flank_certified,
                full_flank_deviation: self.full_flank_deviation,
            },
        )
    }
}

pub(super) struct FlankTally {
    flank_deviation: f64,
    full_flank_certified: bool,
    full_flank_deviation: f64,
}

/// Outer boundary and holes of the outline from the assembled tooth chain.
pub(super) fn outline_loops(
    spec: &GearSpec,
    frame: &ToothFrame,
    tooth: Vec<Curve>,
) -> Result<Vec<Vec<Curve>>> {
    let mut loops = Vec::new();
    if spec.internal {
        loops.push(circle2(
            frame.outside,
            arcs_clearing(frame.outside, frame.root).max(circle_arcs(spec.teeth)),
        )?);
        loops.push(tooth.iter().rev().map(reverse2).collect());
    } else {
        loops.push(tooth);
        if spec.bore > 0. {
            let hole = circle2(spec.bore / 2., circle_arcs(spec.teeth))?;
            loops.push(hole.iter().rev().map(reverse2).collect());
        }
    }
    Ok(loops)
}

/// Closes the loops and certifies joins, arcs and transitions into the report.
pub(super) fn certify_outline(
    spec: &GearSpec,
    frame: &ToothFrame,
    ideal: Option<&GearParameterBounds>,
    loops: &mut [Vec<Curve>],
    tally: FlankTally,
) -> Result<GearGeometry> {
    let profile_join_adjustment_upper = profile_join::close(loops, JOIN_TOLERANCE)?;
    let lead = frame.has_root_lead();
    let arc_radius_error_upper =
        ideal.and_then(|bounds| arc_radius::certify(spec, loops, bounds, lead).ok());
    let arc_radius_rounding_certified = arc_radius_error_upper.is_some_and(|e| e <= FLANK_FIT);
    let transition_error_upper =
        ideal.and_then(|bounds| profile_parameter::certify(spec, loops, bounds, lead).ok());
    let transition_rounding_certified = transition_error_upper.is_some_and(|e| e <= FLANK_FIT);
    let mut flank_deviation = tally.flank_deviation;
    if tally.full_flank_certified {
        flank_deviation = flank_deviation.max(tally.full_flank_deviation);
    }
    let profile_error_upper = if tally.full_flank_certified && transition_rounding_certified {
        transition_error_upper.map(|e| e.max(flank_deviation))
    } else {
        None
    };
    Ok(GearGeometry {
        pitch_radius: frame.pitch,
        base_radius: frame.base,
        tip_radius: frame.tip,
        root_radius: frame.root,
        outside_radius: frame.outside,
        flank_deviation,
        flank_rounding_certified: tally.full_flank_certified,
        flank_stored_parameters_rounding_certified: true,
        sweep_deviation: 0.,
        sweep_rounding_certified: false,
        face_count: 0,
        profile_join_adjustment_upper,
        arc_radius_error_upper,
        arc_radius_rounding_certified,
        transition_error_upper,
        transition_rounding_certified,
        profile_error_upper,
        ideal_wall_error_upper: None,
    })
}
