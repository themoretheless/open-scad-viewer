//! Legacy SCAD/Value compatibility adapters.
use super::*;
use std::f64::consts::{PI, TAU};
use std::fmt::Write;
use value_codec::{Value, json};

/// ECMAScript-compatible spelling for finite generated coordinates. Keep the
/// decimal interval used by JSON.stringify, including a canonical positive zero.
/// The same spelling as a standalone string.
pub fn number(v: f64) -> String {
    let mut out = String::with_capacity(24);
    append_number(&mut out, v);
    out
}
pub fn append_number(out: &mut String, v: f64) {
    if v == 0. {
        out.push('0');
    } else if v.abs() < 1e-6 || v.abs() >= 1e21 {
        let scientific = format!("{v:e}");
        let (mantissa, exponent) = scientific.split_once('e').unwrap();
        let exponent: i32 = exponent.parse().unwrap();
        write!(
            out,
            "{mantissa}e{}{exponent}",
            if exponent >= 0 { "+" } else { "" }
        )
        .unwrap();
    } else {
        write!(out, "{v}").unwrap();
    }
}
// Sampled involute and phase-aligned thread generation, bounded before emission.
pub struct Generated {
    pub source: String,
    pub report: Value,
    pub parts: Vec<Value>,
}
fn n(o: &Value, key: &str) -> f64 {
    o[key].as_f64().unwrap()
}
fn flag(o: &Value, key: &str) -> bool {
    o[key].as_bool().unwrap_or(false)
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
fn gear_options(o: &Value, path: &str) -> Result<GearOptions> {
    let count = |key: &str, message: &str| -> Result<usize> {
        let n = o
            .get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite() && *v >= 0. && v.fract() == 0. && *v <= 256.)
            .ok_or_else(|| err(path, message))?;
        Ok(n as usize)
    };
    let scalar = |key: &str| {
        o.get(key)
            .and_then(Value::as_f64)
            .filter(|v| v.is_finite())
            .ok_or_else(|| err(path, format!("Gear {key} must be finite.")))
    };
    Ok(GearOptions {
        teeth: count("teeth", "Gear teeth must be an integer from 3 to 256.")?,
        flank_segments: count(
            "flank_segments",
            "Gear flank_segments must be an integer from 3 to 12.",
        )?,
        module: scalar("module")?,
        pressure_angle: scalar("pressure_angle")?,
        thickness: scalar("thickness")?,
        backlash: scalar("backlash")?,
        clearance: scalar("clearance")?,
        bore: scalar("bore")?,
        rim_width: scalar("rim_width")?,
        internal: flag(o, "internal"),
    })
}
fn gear_report(r: &GearReport) -> Value {
    let internal = r.internal;
    let teeth = r.teeth as f64;
    let module = r.module_mm;
    let pressure = r.pressure_angle_deg;
    let pitch = r.pitch_radius_mm;
    let base = r.base_radius_mm;
    let tip = r.tip_radius_mm;
    let root = r.root_radius_mm;
    let outside = r.outside_radius_mm;
    let thickness = r.thickness_mm;
    let bore = r.bore_diameter_mm;
    let backlash = r.backlash_per_gear_mm;
    let clearance = r.clearance_mm;
    let vertices = r.profile_vertices;
    let profile_area = r.profile_area_mm2;
    let minimum = r.minimum_external_teeth_without_undercut;

    let report = json!({"kind":if internal{"internal_spur_gear"}else{"external_spur_gear"},"teeth":teeth,"module_mm":module,"pressure_angle_deg":pressure,"pitch_radius_mm":pitch,"base_radius_mm":base,"tip_radius_mm":tip,"root_radius_mm":root,"outside_radius_mm":outside,"pitch_diameter_mm":2.*pitch,"outside_diameter_mm":2.*outside,"thickness_mm":thickness,"bore_diameter_mm":bore,"tooth_thickness_at_pitch_mm":PI*module/2.-backlash,"backlash_per_gear_mm":backlash,"clearance_mm":clearance,"tooth_center_angle_deg":0,"profile_vertices":vertices,"profile_area_mm2":profile_area,"expected_volume_mm3":profile_area*thickness,"minimum_external_teeth_without_undercut":minimum,"root_transition":if r.radial_root_transition{"radial_below_base_circle"}else{"involute_to_root_circle"},"warnings":["Flanks and circles are sampled; tooth roots have no generated trochoidal fillet. Mating interference, strength and printer tolerances require separate validation."]});
    report
}
fn gear_profile(o: &Value, path: &str) -> Result<(Vec<Vec<Point>>, Value)> {
    let profile = crate::gear_profile(&gear_options(o, path)?).map_err(|e| err(path, e.message))?;
    Ok((profile.loops, gear_report(&profile.report)))
}
/// The gear as the `brep_gear(...)` builtin: an exact involute NURBS solid
/// in both workspaces (the Mesh track tessellates it). The sampled profile
/// is still computed for validation and for the report.
fn brep_gear_source(o: &Value, prefix: &str) -> String {
    let mut out = String::from(prefix);
    out.push_str("brep_gear(module=");
    append_number(&mut out, n(o, "module"));
    out.push_str(",teeth=");
    append_number(&mut out, n(o, "teeth"));
    out.push_str(",height=");
    append_number(&mut out, n(o, "thickness"));
    out.push_str(",pressure_angle=");
    append_number(&mut out, n(o, "pressure_angle"));
    out.push_str(",bore=");
    append_number(&mut out, n(o, "bore"));
    out.push_str(",internal=");
    out.push_str(if flag(o, "internal") { "true" } else { "false" });
    out.push_str(",rim_width=");
    append_number(&mut out, n(o, "rim_width"));
    out.push_str(",clearance=");
    append_number(&mut out, n(o, "clearance"));
    out.push_str(",backlash=");
    append_number(&mut out, n(o, "backlash"));
    out.push_str(",helix=");
    append_number(
        &mut out,
        o.get("helix_angle").and_then(Value::as_f64).unwrap_or(0.),
    );
    out.push_str(",herringbone=");
    out.push_str(if flag(o, "herringbone") {
        "true"
    } else {
        "false"
    });
    out.push_str(");");
    out
}
pub fn gear(o: &Value, path: &str) -> Result<Generated> {
    let (_loops, report) = gear_profile(o, path)?;
    Ok(Generated {
        source: brep_gear_source(o, ""),
        report,
        parts: vec![],
    })
}
use crate::profiles::rounded10;
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
    if sun_teeth.fract() != 0.
        || planet_teeth.fract() != 0.
        || sun_teeth < 1.
        || planet_teeth < 1.
        || !sun_teeth.is_finite()
        || !planet_teeth.is_finite()
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
        "helix_angle",
        "herringbone",
    ] {
        common[k] = o[k].clone();
    }
    let mut sun_o = common.clone();
    sun_o["teeth"] = json!(sun_teeth);
    sun_o["internal"] = json!(false);
    // Mating external gears take opposite hands; the planet and the
    // internal ring share one.
    if let Some(helix) = o.get("helix_angle").and_then(Value::as_f64) {
        sun_o["helix_angle"] = json!(-helix);
    }
    let mut planet_o = common.clone();
    planet_o["teeth"] = json!(planet_teeth);
    planet_o["internal"] = json!(false);
    let mut ring_o = common;
    ring_o["teeth"] = json!(ring_teeth);
    ring_o["internal"] = json!(true);
    ring_o["bore"] = json!(0);
    let native = crate::planetary_profiles(&PlanetaryOptions {
        gear: gear_options(&sun_o, path)?,
        sun_teeth: sun_teeth as usize,
        planet_teeth: planet_teeth as usize,
        planet_count: count as usize,
        carrier_angle: carrier,
    })
    .map_err(|e| err(path, e.message))?;
    let orbit = native.orbit_radius_mm;
    let adjacent = native.adjacent_planet_tip_gap_mm;
    let margin = native.internal_involute_contact_margin_mm;
    let sun_angle = native.sun_angle_deg;
    let ring_angle = native.ring_angle_deg;
    let planet_angles = &native.planet_angles_deg;
    let mut sources = Vec::new();
    let mut parts = Vec::new();
    let mut report_parts = Vec::new();
    for part in &native.parts {
        let options = match part.role {
            "sun" => &sun_o,
            "ring" => &ring_o,
            _ => &planet_o,
        };
        let rotation_angle = part.rotation_deg;
        let origin = part.origin;
        let c = rounded10((rotation_angle * PI / 180.).cos());
        let sine = rounded10((rotation_angle * PI / 180.).sin());
        let mut prefix = String::from("multmatrix([[");
        for (i, value) in [c, -sine, 0., origin[0], sine, c, 0., origin[1]]
            .iter()
            .enumerate()
        {
            if i == 4 {
                prefix.push_str("],[");
            } else if i != 0 {
                prefix.push(',');
            }
            append_number(&mut prefix, *value);
        }
        prefix.push_str("],[0,0,1,0],[0,0,0,1]])");
        sources.push(brep_gear_source(options, &prefix));
        let pose = json!({"origin":origin,"rotation":[0,0,rounded10(rotation_angle)]});
        report_parts.push(json!({"id":part.id,"role":part.role,"pose":pose}));
        parts.push(json!({"id":part.id,"role":part.role,"pose":pose,"document":mechanical_document(options)}));
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
fn admit_thread_fields(o: &Value, path: &str) -> Result<()> {
    for key in [
        "diameter",
        "pitch",
        "length",
        "wall",
        "clearance",
        "starts",
        "segments_per_turn",
    ] {
        if !o
            .get(key)
            .and_then(Value::as_f64)
            .is_some_and(f64::is_finite)
        {
            return Err(err(path, format!("Thread {key} must be finite.")));
        }
    }
    for key in ["internal", "left_handed"] {
        if o.get(key).and_then(Value::as_bool).is_none() {
            return Err(err(path, format!("Thread {key} must be boolean.")));
        }
    }
    Ok(())
}
pub fn thread(o: &Value, path: &str) -> Result<Generated> {
    thread_with_mesh(o, path).map(|(generated, _)| generated)
}
pub fn thread_geometry(o: &Value) -> Result<Value> {
    let (generated, mesh) = thread_with_mesh(o, "thread")?;
    Ok(json!({"source":generated.source,"mesh":mesh,"report":generated.report}))
}
pub fn thread_radius(o: &Value, angle: f64, z: f64) -> Result<f64> {
    admit_thread_fields(o, "thread")?;
    if !angle.is_finite() || !z.is_finite() || n(o, "pitch") <= 0. {
        return Err(err("thread", "Invalid thread radius query."));
    }
    let phase = z / n(o, "pitch")
        - (if flag(o, "left_handed") { -1. } else { 1. }) * n(o, "starts") * angle / TAU;
    let wrapped = phase - phase.floor();
    let distance = wrapped.min(1. - wrapped);
    let depth = 3f64.sqrt() * n(o, "pitch") * (distance - 1. / 16.).clamp(0., 5. / 16.);
    let radius = n(o, "diameter") / 2. - depth
        + (if flag(o, "internal") { 1. } else { -1. }) * n(o, "clearance") / 2.;
    if !phase.is_finite() || !radius.is_finite() {
        return Err(err("thread", "Thread radius exceeds finite numeric range."));
    }
    Ok(radius)
}
fn thread_options(o: &Value, path: &str) -> Result<ThreadOptions> {
    admit_thread_fields(o, path)?;
    let integer = |key: &str, limit: f64, message: &str| -> Result<usize> {
        let v = n(o, key);
        if v.fract() != 0. || v < 0. || v > limit {
            return Err(err(path, message));
        }
        Ok(v as usize)
    };
    Ok(ThreadOptions {
        diameter: n(o, "diameter"),
        pitch: n(o, "pitch"),
        length: n(o, "length"),
        wall: n(o, "wall"),
        clearance: n(o, "clearance"),
        starts: integer(
            "starts",
            4.,
            "Thread starts must be an integer from 1 to 4.",
        )?,
        segments_per_turn: integer(
            "segments_per_turn",
            96.,
            "Thread segments_per_turn must be an integer from 16 to 96, and at least eight times starts.",
        )?,
        internal: flag(o, "internal"),
        left_handed: flag(o, "left_handed"),
    })
}
fn thread_with_mesh(o: &Value, path: &str) -> Result<(Generated, Value)> {
    let options = thread_options(o, path)?;
    let mesh = crate::thread_mesh(&options).map_err(|e| err(path, e.message))?;
    let e = |message| err(path, message);
    let diameter = options.diameter;
    let pitch = options.pitch;
    let length = options.length;
    let clearance = options.clearance;
    let starts = options.starts as f64;
    let segments = options.segments_per_turn as f64;
    let internal = options.internal;
    let faces: Vec<[usize; 3]> = mesh
        .indices
        .as_chunks::<3>()
        .0
        .iter()
        .map(|t| [t[2], t[1], t[0]])
        .collect();
    let columns_len = mesh.angular_columns + 1;
    let mut source = String::with_capacity(48 + mesh.positions.len() * 48 + faces.len() * 18);
    source.push_str("polyhedron(points=");
    append_points(&mut source, mesh.positions.iter().copied());
    source.push_str(",faces=[");
    for (i, [a, b, c]) in faces.iter().enumerate() {
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
    let report = json!({"generator":"own_helical_thread","profile":"metric_60_degree_basic_faceted","units":"mm","nominal_diameter_mm":diameter,"pitch_mm":pitch,"lead_mm":starts*pitch,"length_mm":length,"starts":starts,"handedness":if flag(o,"left_handed"){"left"}else{"right"},"internal":internal,"radial_clearance_mm":clearance,"clearance_convention":"External radius decreases by clearance/2; internal cavity radius increases by clearance/2. Matching settings give nominal radial clearance.","major_diameter_mm":diameter+2.*offset,"minor_diameter_mm":diameter-2.*depth+2.*offset,"outer_diameter_mm":if internal{diameter+clearance+2.*n(o,"wall")}else{diameter-clearance},"minimum_wall_mm":if internal{json!(n(o,"wall"))}else{Value::Null},"profile_depth_mm":depth,"flank_included_angle_degrees":60,"segments_per_turn":segments,"angular_columns":columns_len-1,"vertex_count":mesh.positions.len(),"triangle_count":faces.len(),"tolerance_class":null,"limitations":["Faceted basic profile, without root rounding, lead-in chamfers, runout or a tolerance class.","Matching pitch, starts, handedness and angular phase are required; printing fit is not certified."]});
    let geometry = json!({"positions":mesh.positions.iter().flatten().copied().collect::<Vec<_>>(),"indices":mesh.indices.to_vec()});
    Ok((
        Generated {
            source,
            report,
            parts: vec![],
        },
        geometry,
    ))
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
            if matches!(case["kind"].as_str().unwrap(), "gear" | "planetary") {
                // Gears are emitted as the exact `brep_gear(...)` builtin now;
                // the sampled profile survives only in the report.
                assert!(actual.source.contains("brep_gear("), "{name}");
                assert!(!actual.source.contains("polygon("), "{name}");
                continue;
            }
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
}
