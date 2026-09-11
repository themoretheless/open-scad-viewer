//! Sampled involute and phase-aligned thread generation, bounded before emission.
use super::emit::append_number;
use crate::{Error, Result};
use std::collections::HashMap;
use std::f64::consts::{PI, TAU};
use std::fmt::Write;
use value_codec::{Value, json};
type Point = [f64; 2];
pub struct Generated {
    pub source: String,
    pub report: Value,
    pub parts: Vec<Value>,
}
fn err(path: &str, message: impl Into<String>) -> Error {
    Error::new("invalid_mechanical_geometry", path, message)
}
fn n(o: &Value, key: &str) -> f64 {
    o[key].as_f64().unwrap()
}
fn flag(o: &Value, key: &str) -> bool {
    o[key].as_bool().unwrap_or(false)
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
fn append_points<const N: usize>(out: &mut String, points: impl Iterator<Item = [f64; N]>) {
    out.push('[');
    for (i, p) in points.enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push('[');
        for (j, value) in p.into_iter().enumerate() {
            if j != 0 {
                out.push(',');
            }
            append_number(out, value);
        }
        out.push(']');
    }
    out.push(']');
}
pub fn extrude(loops: &[Vec<Point>], thickness: f64) -> String {
    let vertices: usize = loops.iter().map(Vec::len).sum();
    let mut out = String::with_capacity(64 + vertices * 30);
    out.push_str("linear_extrude(height=");
    append_number(&mut out, thickness);
    out.push_str(")polygon(points=");
    append_points(&mut out, loops.iter().flatten().copied());
    out.push_str(",paths=[");
    let mut offset = 0;
    for (i, points) in loops.iter().enumerate() {
        if i != 0 {
            out.push(',');
        }
        out.push('[');
        for j in 0..points.len() {
            if j != 0 {
                out.push(',');
            }
            write!(&mut out, "{}", offset + j).unwrap();
        }
        offset += points.len();
        out.push(']');
    }
    out.push_str("]);");
    out
}
fn gear_profile(o: &Value, path: &str) -> Result<(Vec<Vec<Point>>, Value)> {
    let e = |m: &str| err(path, m);
    let teeth = n(o, "teeth");
    let flank = n(o, "flank_segments");
    let module = n(o, "module");
    let pressure = n(o, "pressure_angle");
    let thickness = n(o, "thickness");
    let backlash = n(o, "backlash");
    let clearance = n(o, "clearance");
    let bore = n(o, "bore");
    let rim = n(o, "rim_width");
    let internal = flag(o, "internal");
    if teeth.fract() != 0. || !(8.0..=128.).contains(&teeth) {
        return Err(e("Gear teeth must be an integer from 8 to 128."));
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
    let minimum = (2. / alpha.sin().powi(2) - 1e-12).ceil();
    if !internal && teeth < minimum {
        return Err(err(
            path,
            format!(
                "Gear requires at least {} teeth at this pressure angle; undercut and profile shift are not implemented.",
                super::emit::number(minimum)
            ),
        ));
    }
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
    let report = json!({"kind":if internal{"internal_spur_gear"}else{"external_spur_gear"},"teeth":teeth,"module_mm":module,"pressure_angle_deg":pressure,"pitch_radius_mm":pitch,"base_radius_mm":base,"tip_radius_mm":tip,"root_radius_mm":root,"outside_radius_mm":outside,"pitch_diameter_mm":2.*pitch,"outside_diameter_mm":2.*outside,"thickness_mm":thickness,"bore_diameter_mm":bore,"tooth_thickness_at_pitch_mm":PI*module/2.-backlash,"backlash_per_gear_mm":backlash,"clearance_mm":clearance,"tooth_center_angle_deg":0,"profile_vertices":vertices,"profile_area_mm2":profile_area,"expected_volume_mm3":profile_area*thickness,"minimum_external_teeth_without_undercut":minimum,"root_transition":if low<base{"radial_below_base_circle"}else{"involute_to_root_circle"},"warnings":["Flanks and circles are sampled; tooth roots have no generated trochoidal fillet. Mating interference, strength and printer tolerances require separate validation."]});
    Ok((loops, report))
}
pub fn gear(o: &Value, path: &str) -> Result<Generated> {
    let (loops, report) = gear_profile(o, path)?;
    Ok(Generated {
        source: extrude(&loops, n(o, "thickness")),
        report,
        parts: vec![],
    })
}
fn rounded10(v: f64) -> f64 {
    if v.abs() < 1e-10 {
        0.
    } else {
        format!("{v:.10}").parse().unwrap()
    }
}
fn angle(v: f64) -> f64 {
    ((v % 360.) + 540.) % 360. - 180.
}
fn mechanical_document(options: &Value) -> Value {
    let mut parameters = Vec::new();
    let mut node = json!({"id":"part","op":"gear"});
    for key in [
        "teeth",
        "module",
        "pressure_angle",
        "thickness",
        "bore",
        "backlash",
        "clearance",
        "internal",
        "rim_width",
        "flank_segments",
    ] {
        let value = &options[key];
        if value.is_boolean() {
            node[key] = value.clone();
            continue;
        }
        let mut p = json!({"id":key,"value":value});
        if [
            "module",
            "thickness",
            "bore",
            "backlash",
            "clearance",
            "rim_width",
        ]
        .contains(&key)
        {
            p["unit"] = json!("mm");
        } else if key == "pressure_angle" {
            p["unit"] = json!("deg");
        }
        if ["teeth", "flank_segments"].contains(&key) {
            p["integer"] = json!(true);
        }
        parameters.push(p);
        node[key] = json!({"param":key});
    }
    json!({"language":"modelgraph/1","units":"mm","type_policy":"strict","parameters":parameters,"nodes":[node],"root":"part","segments":48})
}
pub fn planetary(o: &Value, path: &str) -> Result<Generated> {
    let e = |m: &str| err(path, m);
    let sun_teeth = n(o, "sun_teeth");
    let planet_teeth = n(o, "planet_teeth");
    let count = n(o, "planet_count");
    let carrier = n(o, "carrier_angle");
    let module = n(o, "module");
    let pressure = n(o, "pressure_angle");
    let thickness = n(o, "thickness");
    if count.fract() != 0. || !(2.0..=6.).contains(&count) {
        return Err(e("Planetary planet_count must be an integer from 2 to 6."));
    }
    if sun_teeth.fract() != 0. || planet_teeth.fract() != 0. || sun_teeth < 1. || planet_teeth < 1.
    {
        return Err(e("Planetary tooth counts must be positive integers."));
    }
    if carrier.abs() > 360000. {
        return Err(e("Planetary carrier_angle must be within ±360000 degrees."));
    }
    let ring_teeth = sun_teeth + 2. * planet_teeth;
    if (sun_teeth + ring_teeth) % count != 0. {
        return Err(e(
            "Equally spaced planets require (sun_teeth + ring_teeth) / planet_count to be an integer.",
        ));
    }
    let mut common = json!({});
    for k in [
        "module",
        "pressure_angle",
        "thickness",
        "bore",
        "backlash",
        "clearance",
        "rim_width",
        "flank_segments",
    ] {
        common[k] = o[k].clone();
    }
    let mut sun_o = common.clone();
    sun_o["teeth"] = json!(sun_teeth);
    sun_o["internal"] = json!(false);
    let mut planet_o = common.clone();
    planet_o["teeth"] = json!(planet_teeth);
    planet_o["internal"] = json!(false);
    let mut ring_o = common;
    ring_o["teeth"] = json!(ring_teeth);
    ring_o["internal"] = json!(true);
    ring_o["bore"] = json!(0);
    let (sun, sun_report) = gear_profile(&sun_o, path)?;
    let (planet, planet_report) = gear_profile(&planet_o, path)?;
    let (ring, ring_report) = gear_profile(&ring_o, path)?;
    let _ = sun_report;
    let orbit = module * (sun_teeth + planet_teeth) / 2.;
    let adjacent = 2. * orbit * (PI / count).sin() - 2. * n(&planet_report, "tip_radius_mm");
    if adjacent <= module * 1e-8 {
        return Err(e(
            "Adjacent planet addendum circles overlap or touch; reduce planet_count or increase sun_teeth.",
        ));
    }
    let margin = (n(&ring_report, "tip_radius_mm").powi(2)
        - n(&ring_report, "base_radius_mm").powi(2))
    .max(0.)
    .sqrt()
        - orbit * (pressure * PI / 180.).sin();
    if margin <= module * 1e-8 {
        return Err(e(
            "Internal involute interference: ring tooth tips reach below the planet base circle; increase planet_teeth.",
        ));
    }
    let sun_angle = (1. + ring_teeth / sun_teeth) * carrier;
    let ring_angle = (planet_teeth % 2.) * 180. / ring_teeth;
    let mut sources = Vec::new();
    let mut parts = Vec::new();
    let mut report_parts = Vec::new();
    let mut place = |id: String,
                     role: &str,
                     loops: &[Vec<Point>],
                     options: &Value,
                     x: f64,
                     y: f64,
                     rotation: f64| {
        let rotation_angle = angle(rotation);
        let origin = [rounded10(x), rounded10(y), 0.];
        let c = rounded10((rotation_angle * PI / 180.).cos());
        let s = rounded10((rotation_angle * PI / 180.).sin());
        let transformed: Vec<Vec<Point>> = loops
            .iter()
            .map(|l| {
                l.iter()
                    .map(|[x, y]| {
                        [
                            rounded10(c * x - s * y + origin[0]),
                            rounded10(s * x + c * y + origin[1]),
                        ]
                    })
                    .collect()
            })
            .collect();
        sources.push(extrude(&transformed, thickness));
        let pose = json!({"origin":origin,"rotation":[0,0,rounded10(rotation_angle)]});
        report_parts.push(json!({"id":id,"role":role,"pose":pose}));
        parts
            .push(json!({"id":id,"role":role,"pose":pose,"document":mechanical_document(options)}));
    };
    place("sun".into(), "sun", &sun, &sun_o, 0., 0., sun_angle);
    place("ring".into(), "ring", &ring, &ring_o, 0., 0., ring_angle);
    let mut planet_angles = Vec::new();
    for i in 0..count as usize {
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
            &planet,
            &planet_o,
            orbit * polar.cos(),
            orbit * polar.sin(),
            planet_angle,
        );
    }
    let source = sources.join("\n");
    if source.len() > 240000 {
        return Err(e(
            "Planetary generated source exceeds the geometry budget; reduce tooth counts, planet_count or flank_segments.",
        ));
    }
    let report = json!({"generator":"planetary_gears","construction":"unshifted_involute_spur_gearset","sun_teeth":sun_teeth,"planet_teeth":planet_teeth,"ring_teeth":ring_teeth,"planet_count":count,"module_mm":module,"pressure_angle_deg":pressure,"thickness_mm":thickness,"orbit_radius_mm":orbit,"adjacent_planet_tip_gap_mm":adjacent,"internal_involute_contact_margin_mm":margin,"equal_spacing_assembly_index":(sun_teeth+ring_teeth)/count,"fixed_member":"ring","input_member":"sun","output_member":"carrier","sun_to_carrier_ratio":1.+ring_teeth/sun_teeth,"carrier_angle_deg":carrier,"sun_angle_deg":sun_angle,"ring_angle_deg":ring_angle,"planet_angles_deg":planet_angles,"backlash_per_gear_mm":o["backlash"],"pair_circumferential_backlash_mm":2.*n(o,"backlash"),"generated_parts":report_parts,"omitted_components":["carrier_plate","axles","bearings","housing","fasteners"],"load_capacity":"not_evaluated","manufacturing_tolerance_class":"not_assigned","profile_note":"Sampled involute flanks with radial root transitions; no cutter-generated trochoid or root fillet."});
    Ok(Generated {
        source,
        report,
        parts,
    })
}
const BREAKS: [f64; 4] = [1. / 16., 3. / 8., 5. / 8., 15. / 16.];
fn clip(poly: &[Point], slope: f64, bound: f64, lower: bool) -> Vec<Point> {
    let mut out = Vec::with_capacity(poly.len() + 2);
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        let sign = if lower { 1. } else { -1. };
        let da = (a[1] - slope * a[0] - bound) * sign;
        let db = (b[1] - slope * b[0] - bound) * sign;
        let ia = da >= -1e-11;
        let ib = db >= -1e-11;
        if ia {
            out.push(a);
        }
        if ia != ib {
            let f = da / (da - db);
            out.push([a[0] + f * (b[0] - a[0]), a[1] + f * (b[1] - a[1])]);
        }
    }
    out
}
// Coordinates are nonnegative and bounded by 64. Compute the exact rounded
// decimal key used by JS toFixed(11), without allocating decimal strings. The
// 53-bit significand times 5^11 fits u128; this avoids floating-point rounding
// before the final decimal rounding (including exact halfway cases).
fn fixed11_key(value: f64) -> u64 {
    debug_assert!((0.0..=64.0).contains(&value));
    let bits = value.to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    if exponent == 0 {
        return 0;
    }
    let significand = ((bits & ((1u64 << 52) - 1)) | (1u64 << 52)) as u128;
    let scaled = significand * 48_828_125u128; // 5^11
    let shift = (1064 - exponent) as u32;
    if shift >= 128 {
        return 0;
    }
    let whole = scaled >> shift;
    let remainder = scaled & ((1u128 << shift) - 1);
    (whole + u128::from(remainder >= (1u128 << (shift - 1)))) as u64
}

struct ThreadMesh<'a> {
    o: &'a Value,
    turns: f64,
    points: Vec<[f64; 3]>,
    faces: Vec<[usize; 3]>,
    ids: HashMap<(bool, u64, u64), usize>,
    path: &'a str,
}
impl ThreadMesh<'_> {
    fn point(&mut self, mut t: f64, mut y: f64, outer: bool) -> usize {
        if (t - 1.).abs() < 1e-10 || t.abs() < 1e-10 {
            t = 0.;
        }
        if y.abs() < 1e-10 {
            y = 0.;
        }
        if (y - self.turns).abs() < 1e-10 {
            y = self.turns;
        }
        let key = (outer, fixed11_key(t), fixed11_key(y));
        if let Some(id) = self.ids.get(&key) {
            return *id;
        }
        let angle = TAU * t;
        let radius = if outer {
            n(self.o, "diameter") / 2. + n(self.o, "clearance") / 2. + n(self.o, "wall")
        } else {
            let phase = y
                - (if flag(self.o, "left_handed") { -1. } else { 1. })
                    * n(self.o, "starts")
                    * angle
                    / TAU;
            let wrapped = phase - phase.floor();
            let distance = wrapped.min(1. - wrapped);
            let depth =
                3f64.sqrt() * n(self.o, "pitch") * (distance - 1. / 16.).clamp(0., 5. / 16.);
            n(self.o, "diameter") / 2. - depth
                + (if flag(self.o, "internal") { 1. } else { -1. }) * n(self.o, "clearance") / 2.
        };
        let round = |v: f64| format!("{v:.12e}").parse::<f64>().unwrap();
        let id = self.points.len();
        self.points.push([
            round(radius * angle.cos()),
            round(radius * angle.sin()),
            round(y * n(self.o, "pitch")),
        ]);
        self.ids.insert(key, id);
        id
    }
    fn triangle(&mut self, a: usize, b: usize, c: usize, reverse: bool) -> Result<()> {
        if a == b || b == c || c == a {
            return Ok(());
        }
        self.faces.push(if reverse { [a, c, b] } else { [a, b, c] });
        if self.faces.len() > 3500 {
            return Err(err(
                self.path,
                "Thread mesh exceeds 3500 triangles; shorten length, increase pitch, or reduce segments_per_turn.",
            ));
        }
        Ok(())
    }
}
pub fn thread(o: &Value, path: &str) -> Result<Generated> {
    let e = |m: &str| err(path, m);
    for name in ["diameter", "pitch", "length", "wall"] {
        let v = n(o, name);
        if !(0.01..=10000.).contains(&v) {
            return Err(err(
                path,
                format!("Thread {name} must be between 0.01 and 10000 mm."),
            ));
        }
    }
    let diameter = n(o, "diameter");
    let pitch = n(o, "pitch");
    let length = n(o, "length");
    let clearance = n(o, "clearance");
    let starts = n(o, "starts");
    let segments = n(o, "segments_per_turn");
    let internal = flag(o, "internal");
    if diameter < 2. * pitch {
        return Err(e("Thread diameter must be at least twice the pitch."));
    }
    if length < pitch / 4. || length > pitch * 64. {
        return Err(e("Thread length must be between 0.25 and 64 pitches."));
    }
    if clearance < 0. || clearance >= 5. * 3f64.sqrt() * pitch / 16. {
        return Err(e(
            "Thread radial clearance must be nonnegative and smaller than the thread depth.",
        ));
    }
    if starts.fract() != 0. || !(1.0..=4.).contains(&starts) {
        return Err(e("Thread starts must be an integer from 1 to 4."));
    }
    if segments.fract() != 0. || !(16.0..=96.).contains(&segments) || segments < 8. * starts {
        return Err(e(
            "Thread segments_per_turn must be an integer from 16 to 96, and at least eight times starts.",
        ));
    }
    let turns = length / pitch;
    let slope = (if flag(o, "left_handed") { -1. } else { 1. }) * starts;
    let mut angles: Vec<f64> = (0..=segments as usize)
        .map(|i| i as f64 / segments)
        .collect();
    for y in [0., turns] {
        for k in (y.floor() as i32 - starts as i32 - 1)..=(y.ceil() as i32 + starts as i32 + 1) {
            for corner in BREAKS {
                let t = (y - k as f64 - corner) / slope;
                if t > 1e-10 && t < 1. - 1e-10 {
                    angles.push(t);
                }
            }
        }
    }
    angles.sort_by(f64::total_cmp);
    let columns: Vec<f64> = angles
        .iter()
        .enumerate()
        .filter(|(i, t)| *i == 0 || **t - angles[i - 1] > 1e-10)
        .map(|(_, t)| *t)
        .collect();
    let mut mesh = ThreadMesh {
        o,
        turns,
        points: Vec::new(),
        faces: Vec::new(),
        ids: HashMap::new(),
        path,
    };
    for col in 0..columns.len() - 1 {
        let (a, b) = (columns[col], columns[col + 1]);
        let min = (-slope * a).min(-slope * b);
        let max = (turns - slope * a).max(turns - slope * b);
        let mut breaks = Vec::new();
        for k in min.floor() as i32 - 1..=max.ceil() as i32 {
            for corner in BREAKS {
                breaks.push(k as f64 + corner);
            }
        }
        for j in 0..breaks.len() - 1 {
            let (lo, hi) = (breaks[j], breaks[j + 1]);
            if hi <= min + 1e-11 || lo >= max - 1e-11 {
                continue;
            }
            let poly = clip(
                &clip(&[[a, 0.], [b, 0.], [b, turns], [a, turns]], slope, lo, true),
                slope,
                hi,
                false,
            );
            let all: Vec<usize> = poly
                .iter()
                .map(|[t, y]| {
                    mesh.point(
                        if (*t - a).abs() < 1e-10 {
                            a
                        } else if (*t - b).abs() < 1e-10 {
                            b
                        } else {
                            *t
                        },
                        *y,
                        false,
                    )
                })
                .collect();
            let ids: Vec<usize> = all
                .iter()
                .enumerate()
                .filter(|(i, id)| **id != all[(i + all.len() - 1) % all.len()])
                .map(|(_, id)| *id)
                .collect();
            for k in 1..ids.len().saturating_sub(1) {
                mesh.triangle(ids[0], ids[k], ids[k + 1], !internal)?;
            }
        }
    }
    let bottom = mesh.points.len();
    let top = bottom + 1;
    if !internal {
        mesh.points.push([0., 0., 0.]);
        mesh.points.push([0., 0., length]);
    }
    for i in 0..columns.len() - 1 {
        let (a, b) = (columns[i], columns[i + 1]);
        let a0 = mesh.point(a, 0., false);
        let b0 = mesh.point(b, 0., false);
        let a1 = mesh.point(a, turns, false);
        let b1 = mesh.point(b, turns, false);
        if !internal {
            mesh.triangle(bottom, a0, b0, false)?;
            mesh.triangle(top, b1, a1, false)?;
        } else {
            let ao = mesh.point(a, 0., true);
            let bo = mesh.point(b, 0., true);
            let at = mesh.point(a, turns, true);
            let bt = mesh.point(b, turns, true);
            for [a, b, c] in [
                [ao, b0, a0],
                [ao, bo, b0],
                [at, a1, b1],
                [at, b1, bt],
                [ao, bt, bo],
                [ao, at, bt],
            ] {
                mesh.triangle(a, b, c, false)?;
            }
        }
    }
    let mut source = String::with_capacity(48 + mesh.points.len() * 48 + mesh.faces.len() * 18);
    source.push_str("polyhedron(points=");
    append_points(&mut source, mesh.points.iter().copied());
    source.push_str(",faces=[");
    for (i, [a, b, c]) in mesh.faces.iter().enumerate() {
        if i != 0 {
            source.push(',');
        }
        write!(&mut source, "[{a},{b},{c}]").unwrap();
    }
    source.push_str("],convexity=10);");
    if source.len() > 200000 {
        return Err(e(
            "Thread generated source exceeds 200000 characters; reduce segments_per_turn or length.",
        ));
    }
    let depth = 5. * 3f64.sqrt() * pitch / 16.;
    let offset = (if internal { 1. } else { -1. }) * clearance / 2.;
    let report = json!({"generator":"own_helical_thread","profile":"metric_60_degree_basic_faceted","units":"mm","nominal_diameter_mm":diameter,"pitch_mm":pitch,"lead_mm":starts*pitch,"length_mm":length,"starts":starts,"handedness":if flag(o,"left_handed"){"left"}else{"right"},"internal":internal,"radial_clearance_mm":clearance,"clearance_convention":"External radius decreases by clearance/2; internal cavity radius increases by clearance/2. Matching settings give nominal radial clearance.","major_diameter_mm":diameter+2.*offset,"minor_diameter_mm":diameter-2.*depth+2.*offset,"outer_diameter_mm":if internal{diameter+clearance+2.*n(o,"wall")}else{diameter-clearance},"minimum_wall_mm":if internal{json!(n(o,"wall"))}else{Value::Null},"profile_depth_mm":depth,"flank_included_angle_degrees":60,"segments_per_turn":segments,"angular_columns":columns.len()-1,"vertex_count":mesh.points.len(),"triangle_count":mesh.faces.len(),"tolerance_class":null,"limitations":["Faceted basic profile, without root rounding, lead-in chamfers, runout or a tolerance class.","Matching pitch, starts, handedness and angular phase are required; printing fit is not certified."]});
    Ok(Generated {
        source,
        report,
        parts: vec![],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: &Value, b: &Value, path: &str) {
        if let (Some(a), Some(b)) = (a.as_f64(), b.as_f64()) {
            assert!(
                (a - b).abs() <= 1e-10 * a.abs().max(1.0),
                "{path}: {a} != {b}"
            );
        } else if let (Some(a), Some(b)) = (a.as_object(), b.as_object()) {
            assert_eq!(a.len(), b.len(), "{path}: key count");
            for (key, value) in a {
                close(value, &b[key], &format!("{path}/{key}"));
            }
        } else if let (Some(a), Some(b)) = (a.as_array(), b.as_array()) {
            assert_eq!(a.len(), b.len(), "{path}: array length");
            for (i, (a, b)) in a.iter().zip(b).enumerate() {
                close(a, b, &format!("{path}/{i}"));
            }
        } else {
            assert_eq!(a, b, "{path}");
        }
    }
    fn source_arrays(source: &str) -> Vec<Value> {
        let mut arrays = Vec::new();
        let mut tail = source;
        while let Some((at, kind)) = ["points", "paths", "faces"]
            .iter()
            .filter_map(|kind| tail.find(&format!("{kind}=")).map(|i| (i, *kind)))
            .min_by_key(|(i, _)| *i)
        {
            let start = at + kind.len() + 1;
            let mut depth = 0;
            let end = tail[start..]
                .bytes()
                .position(|ch| {
                    if ch == b'[' {
                        depth += 1;
                    }
                    if ch == b']' {
                        depth -= 1;
                    }
                    depth == 0
                })
                .unwrap()
                + start
                + 1;
            let value: Value = value_codec::from_str(&tail[start..end]).unwrap();
            arrays.push(json!({"kind":kind,"value":value}));
            tail = &tail[end..];
        }
        arrays
    }
    #[test]
    fn mechanical_generators_preserve_reference_reports_and_geometry() {
        let corpus: Value =
            value_codec::from_str(include_str!("../tests/fixtures/mechanical-parity.json"))
                .unwrap();
        for case in corpus["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let actual = match case["kind"].as_str().unwrap() {
                "gear" => gear(&case["options"], "/test"),
                "thread" => thread(&case["options"], "/test"),
                "planetary" => planetary(&case["options"], "/test"),
                _ => unreachable!(),
            };
            if let Some(message) = case["error"].as_str() {
                assert_eq!(actual.err().unwrap().message, message, "{name}");
                continue;
            }
            let actual = actual.unwrap_or_else(|e| panic!("{name}: {:?}", e));
            close(&actual.report, &case["report"], name);
            let arrays = source_arrays(&actual.source);
            assert_eq!(
                arrays.len(),
                case["arrays"].as_array().unwrap().len(),
                "{name}"
            );
            for (array, expected) in arrays.iter().zip(case["arrays"].as_array().unwrap()) {
                assert_eq!(array["kind"], expected["kind"], "{name}");
                assert_eq!(
                    array["value"].as_array().unwrap().len(),
                    expected["length"].as_u64().unwrap() as usize,
                    "{name}"
                );
                for sample in expected["samples"].as_array().unwrap() {
                    close(
                        &array["value"][sample["index"].as_u64().unwrap() as usize],
                        &sample["value"],
                        name,
                    );
                }
            }
        }
    }
    #[test]
    fn decimal_vertex_keys_round_exact_halfway_like_javascript() {
        assert_eq!(fixed11_key(0.0), 0);
        assert_eq!(fixed11_key(1.0), 100_000_000_000);
        assert_eq!(fixed11_key(64.0), 6_400_000_000_000);
        assert_eq!(fixed11_key(0.000244140625), 24_414_063);
        assert_eq!(fixed11_key(1.0 / 3.0), 33_333_333_333);
        assert_eq!(fixed11_key(2.0 / 3.0), 66_666_666_667);
    }
}
