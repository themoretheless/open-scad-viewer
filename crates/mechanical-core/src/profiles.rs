//! Sampled gear profiles and planetary placement; no code emission or value transport.
use crate::{Point, Result, err};
use std::f64::consts::{PI, TAU};
#[derive(Debug, Clone)]
pub struct GearOptions {
    pub teeth: usize,
    pub flank_segments: usize,
    pub module: f64,
    pub pressure_angle: f64,
    pub thickness: f64,
    pub backlash: f64,
    pub clearance: f64,
    pub bore: f64,
    pub rim_width: f64,
    pub internal: bool,
}
impl Default for GearOptions {
    fn default() -> Self {
        Self {
            teeth: 24,
            flank_segments: 6,
            module: 1.,
            pressure_angle: 20.,
            thickness: 4.,
            backlash: 0.1,
            clearance: 0.25,
            bore: 0.,
            rim_width: 2.,
            internal: false,
        }
    }
}
impl GearOptions {
    fn validate_finite(&self) -> Result<()> {
        if [
            self.module,
            self.pressure_angle,
            self.thickness,
            self.backlash,
            self.clearance,
            self.bore,
            self.rim_width,
        ]
        .into_iter()
        .all(f64::is_finite)
        {
            Ok(())
        } else {
            Err(err("gear", "Gear dimensions must be finite."))
        }
    }
}
#[derive(Debug, Clone)]
pub struct GearReport {
    pub internal: bool,
    pub teeth: usize,
    pub module_mm: f64,
    pub pressure_angle_deg: f64,
    pub pitch_radius_mm: f64,
    pub base_radius_mm: f64,
    pub tip_radius_mm: f64,
    pub root_radius_mm: f64,
    pub outside_radius_mm: f64,
    pub thickness_mm: f64,
    pub bore_diameter_mm: f64,
    pub tooth_thickness_at_pitch_mm: f64,
    pub backlash_per_gear_mm: f64,
    pub clearance_mm: f64,
    pub profile_vertices: usize,
    pub profile_area_mm2: f64,
    pub minimum_external_teeth_without_undercut: f64,
    pub radial_root_transition: bool,
}
#[derive(Debug, Clone)]
pub struct GearProfile {
    pub loops: Vec<Vec<Point>>,
    pub report: GearReport,
}
fn round7(v: f64) -> f64 {
    (v * 1e7 + 0.5).floor() / 1e7
}
fn polar(r: f64, a: f64) -> Point {
    [round7(r * a.cos()), round7(r * a.sin())]
}
fn area(points: &[Point]) -> f64 {
    points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let q = points[(i + 1) % points.len()];
            p[0] * q[1] - p[1] * q[0]
        })
        .sum::<f64>()
        .abs()
        / 2.
}
fn add(profile: &mut Vec<Point>, r: f64, a: f64) {
    let p = polar(r, a);
    if profile.last() != Some(&p) {
        profile.push(p);
    }
}
fn arc(profile: &mut Vec<Point>, r: f64, from: f64, to: f64, step: f64) {
    let count = 2usize.max(((to - from) / step * 8.).ceil() as usize);
    for i in 1..=count {
        add(profile, r, from + (to - from) * i as f64 / count as f64);
    }
}
pub fn gear_profile(o: &GearOptions) -> Result<GearProfile> {
    let path = "gear";
    o.validate_finite()?;
    let e = |m: &str| err(path, m);
    let teeth = o.teeth as f64;
    let flank = o.flank_segments as f64;
    let module = o.module;
    let pressure = o.pressure_angle;
    let thickness = o.thickness;
    let backlash = o.backlash;
    let clearance = o.clearance;
    let bore = o.bore;
    let rim = o.rim_width;
    let internal = o.internal;
    if teeth.fract() != 0. || !(3.0..=256.).contains(&teeth) {
        return Err(e("Gear teeth must be an integer from 3 to 256."));
    }
    if flank.fract() != 0. || !(3.0..=12.).contains(&flank) {
        return Err(e("Gear flank_segments must be an integer from 3 to 12."));
    }
    if !(0.1..=100.).contains(&module) {
        return Err(e("Gear module must be from 0.1 to 100 mm."));
    }
    if !(14.5..=30.).contains(&pressure) {
        return Err(e("Gear pressure_angle must be from 14.5 to 30 degrees."));
    }
    if !(0.1..=1000.).contains(&thickness) {
        return Err(e("Gear thickness must be from 0.1 to 1000 mm."));
    }
    if backlash < 0. || backlash > module / 2. {
        return Err(e(
            "Gear backlash must be from zero to half the module; it reduces each gear tooth thickness.",
        ));
    }
    if clearance < 0. || clearance > module {
        return Err(e("Gear clearance must be from zero to one module."));
    }
    if bore < 0. || !(0.0..=10000.0).contains(&rim) {
        return Err(e(
            "Gear bore and rim_width must be nonnegative, with rim_width at most 10000 mm.",
        ));
    }
    let alpha = pressure * PI / 180.;
    // Reported only: brep_gear draws a radial flank below the base circle, so
    // small tooth counts build without undercut or profile shift.
    let minimum = (2. / alpha.sin().powi(2) - 1e-12).ceil();
    let pitch = module * teeth / 2.;
    let base = pitch * alpha.cos();
    let tip = pitch + if internal { -module } else { module };
    let root = pitch
        + if internal {
            module + clearance
        } else {
            -(module + clearance)
        };
    let outside = if internal { root + rim } else { tip };
    if root <= 0. || outside > 10000. {
        return Err(e(
            "Gear root radius must be positive and outside radius at most 10000 mm.",
        ));
    }
    if internal && tip <= base + 1e-9 {
        return Err(e(
            "Internal gear tip must remain outside the base circle; increase teeth or pressure_angle.",
        ));
    }
    if internal && bore != 0. {
        return Err(e(
            "Internal gear uses a toothed central opening; bore must be zero.",
        ));
    }
    if internal && rim < module / 4. {
        return Err(e(
            "Internal gear rim_width must be at least one quarter module.",
        ));
    }
    if !internal && bore > 2. * root - module / 2. {
        return Err(e(
            "Gear bore must leave at least one quarter module of material inside the root circle.",
        ));
    }
    let low = if internal { tip } else { root };
    let high = if internal { root } else { tip };
    let start = base.max(low);
    let inv_pitch = alpha.tan() - alpha;
    let half_pitch = PI / (2. * teeth)
        + if internal {
            backlash / (2. * pitch)
        } else {
            -backlash / (2. * pitch)
        };
    let half = |r: f64| {
        let t = ((r / base).powi(2) - 1.).max(0.).sqrt();
        half_pitch + inv_pitch - (t - t.atan())
    };
    let low_half = half(start);
    let high_half = half(high);
    let tooth_step = TAU / teeth;
    if high_half <= 1e-6 || low_half >= tooth_step / 2. - 1e-6 {
        return Err(e(
            "Gear tooth profile collapses or overlaps; reduce backlash/clearance or change teeth/pressure_angle.",
        ));
    }
    let t0 = ((start / base).powi(2) - 1.).max(0.).sqrt();
    let t1 = ((high / base).powi(2) - 1.).max(0.).sqrt();
    let mut samples: Vec<f64> = (0..=flank as usize)
        .map(|i| t0 + (t1 - t0) * i as f64 / flank)
        .collect();
    samples.push(alpha.tan());
    samples.sort_by(f64::total_cmp);
    let radii: Vec<f64> = samples
        .iter()
        .enumerate()
        .filter(|(i, t)| *i == 0 || **t - samples[i - 1] > 1e-10)
        .map(|(_, t)| base * (1. + t * t).sqrt())
        .collect();
    let mut profile = Vec::with_capacity(teeth as usize * (radii.len() * 2 + 10));
    for tooth in 0..teeth as usize {
        let center = tooth as f64 * tooth_step + if internal { tooth_step / 2. } else { 0. };
        add(&mut profile, low, center - low_half);
        for r in &radii {
            add(&mut profile, *r, center - half(*r));
        }
        arc(
            &mut profile,
            high,
            center - high_half,
            center + high_half,
            tooth_step,
        );
        for r in radii[..radii.len() - 1].iter().rev() {
            add(&mut profile, *r, center + half(*r));
        }
        add(&mut profile, low, center + low_half);
        arc(
            &mut profile,
            low,
            center + low_half,
            center + tooth_step - low_half,
            tooth_step,
        );
    }
    if profile.first() == profile.last() {
        profile.pop();
    }
    let circle = |r: f64| {
        let count = (teeth as usize * 8).max(64);
        (0..count)
            .map(|i| polar(r, i as f64 * TAU / count as f64))
            .collect::<Vec<Point>>()
    };
    let loops = if internal {
        profile.reverse();
        vec![circle(outside), profile]
    } else if bore > 0. {
        let mut hole = circle(bore / 2.);
        hole.reverse();
        vec![profile, hole]
    } else {
        vec![profile]
    };
    let profile_area = area(&loops[0]) - loops.iter().skip(1).map(|l| area(l)).sum::<f64>();
    let vertices: usize = loops.iter().map(|l| l.len()).sum();
    if vertices > 6000 || !profile_area.is_finite() || profile_area <= 0. {
        return Err(e(
            "Gear exceeds the 6000 profile vertex budget or has no material area.",
        ));
    }
    Ok(GearProfile {
        loops,
        report: GearReport {
            internal,
            teeth: o.teeth,
            module_mm: module,
            pressure_angle_deg: pressure,
            pitch_radius_mm: pitch,
            base_radius_mm: base,
            tip_radius_mm: tip,
            root_radius_mm: root,
            outside_radius_mm: outside,
            thickness_mm: thickness,
            bore_diameter_mm: bore,
            tooth_thickness_at_pitch_mm: PI * module / 2. - backlash,
            backlash_per_gear_mm: backlash,
            clearance_mm: clearance,
            profile_vertices: vertices,
            profile_area_mm2: profile_area,
            minimum_external_teeth_without_undercut: minimum,
            radial_root_transition: low < base,
        },
    })
}

#[derive(Debug, Clone)]
pub struct PlanetaryOptions {
    pub gear: GearOptions,
    pub sun_teeth: usize,
    pub planet_teeth: usize,
    pub planet_count: usize,
    pub carrier_angle: f64,
}
#[derive(Debug, Clone)]
pub struct GearInstance {
    pub id: String,
    pub role: &'static str,
    pub profile: GearProfile,
    pub origin: [f64; 3],
    pub rotation_deg: f64,
}
#[derive(Debug, Clone)]
pub struct PlanetaryProfiles {
    pub parts: Vec<GearInstance>,
    pub ring_teeth: usize,
    pub orbit_radius_mm: f64,
    pub adjacent_planet_tip_gap_mm: f64,
    pub internal_involute_contact_margin_mm: f64,
    pub sun_to_carrier_ratio: f64,
    pub sun_angle_deg: f64,
    pub ring_angle_deg: f64,
    pub planet_angles_deg: Vec<f64>,
}
pub(crate) fn rounded10(v: f64) -> f64 {
    if v.abs() < 1e-10 {
        0.
    } else {
        format!("{v:.10}").parse().unwrap()
    }
}
pub(crate) fn angle(v: f64) -> f64 {
    ((v % 360.) + 540.) % 360. - 180.
}
pub fn planetary_profiles(o: &PlanetaryOptions) -> Result<PlanetaryProfiles> {
    let e = |message| err("planetary", message);
    if !(2..=6).contains(&o.planet_count) {
        return Err(e("Planetary planet_count must be an integer from 2 to 6."));
    }
    if o.sun_teeth == 0 || o.planet_teeth == 0 {
        return Err(e("Planetary tooth counts must be positive integers."));
    }
    if !o.carrier_angle.is_finite() || o.carrier_angle.abs() > 360000. {
        return Err(e("Planetary carrier_angle must be within ±360000 degrees."));
    }
    let ring_teeth = o
        .planet_teeth
        .checked_mul(2)
        .and_then(|v| v.checked_add(o.sun_teeth))
        .ok_or_else(|| e("Planetary tooth count overflow."))?;
    // Gear admission bounds every member before further arithmetic/allocation.
    let mut gear = o.gear.clone();
    gear.teeth = o.sun_teeth;
    gear.internal = false;
    let sun = gear_profile(&gear)?;
    gear.teeth = o.planet_teeth;
    let planet = gear_profile(&gear)?;
    gear.teeth = ring_teeth;
    gear.internal = true;
    gear.bore = 0.;
    let ring = gear_profile(&gear)?;
    if !(o.sun_teeth + ring_teeth).is_multiple_of(o.planet_count) {
        return Err(e(
            "Equally spaced planets require (sun_teeth + ring_teeth) / planet_count to be an integer.",
        ));
    }
    let sun_teeth = o.sun_teeth as f64;
    let planet_teeth = o.planet_teeth as f64;
    let ring_count = ring_teeth as f64;
    let count = o.planet_count as f64;
    let module = o.gear.module;
    let pressure = o.gear.pressure_angle;
    let carrier = o.carrier_angle;
    let orbit = module * (sun_teeth + planet_teeth) / 2.;
    let adjacent = 2. * orbit * (PI / count).sin() - 2. * planet.report.tip_radius_mm;
    if adjacent <= module * 1e-8 {
        return Err(e(
            "Adjacent planet addendum circles overlap or touch; reduce planet_count or increase sun_teeth.",
        ));
    }
    let margin = (ring.report.tip_radius_mm.powi(2) - ring.report.base_radius_mm.powi(2))
        .max(0.)
        .sqrt()
        - orbit * (pressure * PI / 180.).sin();
    if margin <= module * 1e-8 {
        return Err(e(
            "Internal involute interference: ring tooth tips reach below the planet base circle; increase planet_teeth.",
        ));
    }
    let sun_angle = (1. + ring_count / sun_teeth) * carrier;
    let ring_angle = (planet_teeth % 2.) * 180. / ring_count;
    let mut parts = Vec::new();
    let mut place = |id: String, role, profile: GearProfile, x: f64, y: f64, rotation: f64| {
        parts.push(GearInstance {
            id,
            role,
            profile,
            origin: [rounded10(x), rounded10(y), 0.],
            rotation_deg: angle(rotation),
        });
    };
    place("sun".into(), "sun", sun, 0., 0., sun_angle);
    place("ring".into(), "ring", ring, 0., 0., ring_angle);
    let mut planet_angles = Vec::new();
    for i in 0..o.planet_count {
        let orbit_angle = carrier + 360. * i as f64 / count;
        let planet_angle = (1. + sun_teeth / planet_teeth) * orbit_angle
            - sun_teeth / planet_teeth * sun_angle
            + 180.
            - 180. / planet_teeth;
        planet_angles.push(planet_angle);
        let polar = angle(orbit_angle) * PI / 180.;
        place(
            format!("planet_{}", i + 1),
            "planet",
            planet.clone(),
            orbit * polar.cos(),
            orbit * polar.sin(),
            planet_angle,
        );
    }
    Ok(PlanetaryProfiles {
        parts,
        ring_teeth,
        orbit_radius_mm: orbit,
        adjacent_planet_tip_gap_mm: adjacent,
        internal_involute_contact_margin_mm: margin,
        sun_to_carrier_ratio: 1. + ring_count / sun_teeth,
        sun_angle_deg: sun_angle,
        ring_angle_deg: ring_angle,
        planet_angles_deg: planet_angles,
    })
}
