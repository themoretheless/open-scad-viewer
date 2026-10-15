use super::*;
use value_codec::json;

fn cube(s: f64) -> Value {
    let p = [[0., 0., 0.], [s, 0., 0.], [s, s, 0.], [0., s, 0.], [0., 0., s], [s, 0., s], [s, s, s], [0., s, s]];
    let positions: Vec<f64> = p.iter().flatten().copied().collect();
    let indices: Vec<usize> = vec![0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6, 3, 0, 4, 3, 4, 7];
    json!({"positions": positions, "indices": indices})
}

fn settings() -> Value {
    json!({"nozzle": 0.4, "layer": 0.2, "lines": 3, "skinLayers": 3, "maxBridge": 10., "openTop": true})
}

fn spatial(pattern: &str) -> Value {
    json!({"pattern": pattern, "axis": "z", "cell": 10., "rib": 3.2, "rim": 0., "bottom": 0., "top": 0.,
        "seed": 42, "jitter": 0., "lineWidth": 0.45, "perimeters": 3, "skin": 0., "openTop": true, "diagonals": false})
}

fn channel(pattern: &str, axis: &str, cell: f64) -> Value {
    json!({"pattern": pattern, "axis": axis, "cell": cell, "rib": 1.35, "rim": 2., "bottom": 0.6, "top": 0.,
        "seed": 42, "jitter": 0., "lineWidth": 0.45, "perimeters": 3})
}

fn strength_of(options: Value, material: Value, case: &str, force: f64) -> Value {
    strength(json!({"mesh": cube(30.), "options": options, "material": material,
        "load": {"case": case, "forceN": force, "safety": 2.}})).unwrap()
}

#[test]
fn print_report_ranks_z_for_channels_and_flags_bridges() {
    let r = print_report(json!({"options": channel("grid", "x", 20.), "settings": settings()})).unwrap();
    assert_eq!(r["preferredAxis"].as_str(), Some("z"));
    assert_eq!(r["bridgeSpanMm"].as_f64(), Some(18.65));
    let kinds: Vec<&str> = r["issues"].as_array().unwrap().iter().map(|i| i["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"bridge") && kinds.contains(&"orientation") && kinds.contains(&"overhang"));
    assert_eq!(r["slicerHints"]["support_enable"].as_bool(), Some(true));
    assert_eq!(r["slicerHints"]["top_shell_layers"].as_f64(), Some(0.));
    assert_eq!(r["slicerHints"]["recommended_print_axis"].as_str(), Some("z"));
    // Overhang risk follows the current channel axis, not the recommended one.
    assert_eq!(r["overhangRisk"].as_f64(), Some(0.95));
}

#[test]
fn print_optimize_reorients_fits_and_bounds_bridge() {
    let r = print_optimize(json!({"options": channel("grid", "x", 20.), "settings": settings()})).unwrap();
    let o = &r["options"];
    let changed: Vec<&str> = r["changed"].as_array().unwrap().iter().map(|c| c.as_str().unwrap()).collect();
    assert_eq!(changed[0], "reorient_channels_z");
    assert!(changed.contains(&"fit_lines_layers") && changed.contains(&"shrink_cell_for_bridge"));
    assert_eq!(o["axis"].as_str(), Some("z"));
    assert!(o["cell"].as_f64().unwrap() - o["rib"].as_f64().unwrap() <= 10. + 1e-9);
    assert!(!r["report"]["issues"].as_array().unwrap().iter().any(|i| i["kind"].as_str() == Some("bridge")));
}

#[test]
fn print_report_refuses_bad_settings() {
    let mut s = settings();
    s["layer"] = json!(0.6);
    assert!(print_report(json!({"options": channel("grid", "z", 8.), "settings": s})).is_err());
    assert!(print_report(json!({"options": {"pattern": "nope", "cell": 1., "rib": 0.1}, "settings": settings()})).is_err());
}

#[test]
fn materials_and_hints_are_native() {
    let m = materials(json!({})).unwrap();
    assert_eq!(m.as_array().unwrap().len(), 6);
    assert_eq!(m[0]["id"].as_str(), Some("pla"));
    let h = structure_hint_op(json!({"pattern": "octet"})).unwrap();
    assert_eq!(h["load"].as_str(), Some("stretch"));
    assert_eq!(h["rank"].as_f64(), Some(1.));
    assert!(structure_hint_op(json!({"pattern": "x"})).is_err());
}

#[test]
fn spatial_strength_reports_ashby_euler_and_weak_spots() {
    let r = strength_of(spatial("spatial"), json!("pla"), "compression", 100.);
    assert_eq!(r["loadClass"].as_str(), Some("mixed"));
    assert_eq!(r["ashbyExponent"].as_f64(), Some(1.8));
    let rho = r["relativeDensity"].as_f64().unwrap();
    assert!(rho > 0.02 && rho < 0.95);
    assert!(r["eulerBucklingN"].as_f64().unwrap() > 0.);
    assert_eq!(r["meanStrutLengthMm"].as_f64(), Some(10.));
    assert!(r["connectivity"].as_f64().unwrap() < 6.);
    let spots = r["weakSpots"].as_array().unwrap();
    assert!(!spots.is_empty() && spots.len() <= 12);
    let scores: Vec<f64> = spots.iter().map(|s| s["score"].as_f64().unwrap()).collect();
    assert!(scores.windows(2).all(|w| w[0] >= w[1]));
    assert!(spots.iter().any(|s| s["kind"].as_str() == Some("body_corner")));
    for s in spots {
        assert_eq!(s["color"].as_array().unwrap().len(), 4);
        assert_eq!(s["at"].as_array().unwrap().len(), 3);
    }
}

#[test]
fn material_changes_weak_spot_bias_and_fdm_orthotropy() {
    let mut octet = spatial("octet");
    octet["cell"] = json!(15.);
    let pla = strength_of(octet.clone(), json!("pla"), "compression", 100.);
    let steel = strength_of(octet.clone(), json!("steel"), "compression", 100.);
    // Load along -Z, printed along Z: across-layer modulus governs for FDM.
    assert_eq!(pla["materialEffective"]["layerAlignment"].as_f64(), Some(1.));
    assert!((pla["materialEffective"]["E"].as_f64().unwrap() - 1800.).abs() < 1e-6);
    assert_eq!(steel["materialEffective"]["layerAlignment"].as_f64(), Some(0.));
    assert!(steel["bucklingRatio"].as_f64().unwrap() < pla["bucklingRatio"].as_f64().unwrap());
    let bias = |r: &Value| r["weakSpots"][0]["materialBias"].as_f64().unwrap();
    assert!(bias(&pla) != bias(&steel));
    assert_ne!(pla["weakSpots"][0]["color"], steel["weakSpots"][0]["color"]);
    let mut sideways = json!({"mesh": cube(30.), "options": octet, "material": "pla",
        "load": {"case": "compression", "forceN": 100., "safety": 2., "direction": [1., 0., 0.]}});
    let side = strength(sideways.clone()).unwrap();
    assert_eq!(side["materialEffective"]["layerAlignment"].as_f64(), Some(0.));
    sideways["load"]["direction"] = json!([0., 0., 0.]);
    assert!(strength(sideways).is_err());
}

#[test]
fn channel_strength_finds_acute_corners_and_frame_corners() {
    let r = strength_of(channel("triangles", "z", 8.), json!("petg"), "bending", 50.);
    assert_eq!(r["loadClass"].as_str(), Some("stretch"));
    assert!(r["eulerBucklingN"].is_null());
    let kinds: Vec<&str> = r["weakSpots"].as_array().unwrap().iter().map(|s| s["kind"].as_str().unwrap()).collect();
    assert!(kinds.contains(&"sharp_corner") && kinds.contains(&"body_corner"));
    assert!(!r["advice"].as_array().unwrap().is_empty());
}

#[test]
fn measured_volumes_override_density_estimate_and_bad_loads_fail() {
    let r = strength(json!({"mesh": cube(30.), "options": spatial("bcc"), "material": "pla",
        "load": {"case": "tension", "forceN": 10., "safety": 1.5},
        "volumes": {"beforeMm3": 27000., "afterMm3": 8100.}})).unwrap();
    assert_eq!(r["relativeDensity"].as_f64(), Some(0.3));
    assert_eq!(r["volumeReductionPct"].as_f64(), Some(70.));
    for load in [json!({"case": "compression", "forceN": 0., "safety": 2.}),
                 json!({"case": "compression", "forceN": 1., "safety": 0.5}),
                 json!({"case": "shear", "forceN": 1., "safety": 2.})] {
        assert!(strength(json!({"mesh": cube(30.), "options": spatial("bcc"), "material": "pla", "load": load})).is_err());
    }
    let custom = json!({"E": 1000., "sigmaAllow": 20., "process": "fdm"});
    let r = strength(json!({"mesh": cube(30.), "options": spatial("bcc"), "material": custom,
        "load": {"case": "compression", "forceN": 10., "safety": 2.}})).unwrap();
    assert_eq!(r["material"]["id"].as_str(), Some("custom"));
}

#[test]
fn markers_are_octahedra_scaled_by_score() {
    let m = weak_spot_markers(json!({"markerMm": 1., "spots": [{"at": [0., 0., 0.], "score": 1.}, {"at": [5., 5., 5.], "score": 0.}]})).unwrap();
    let a = m[0].as_array().unwrap();
    assert_eq!(a.len(), 72);
    let max = a.iter().map(|v| v.as_f64().unwrap()).fold(f64::MIN, f64::max);
    assert!((max - 1.25).abs() < 1e-12);
    assert_eq!(m[1].as_array().unwrap().len(), 72);
    assert!(weak_spot_markers(json!({"markerMm": 0., "spots": []})).is_err());
}
