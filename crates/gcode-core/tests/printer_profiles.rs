use gcode_core::{
    Flavor, PlannedLayer, PlannedPath, PrinterProfile, builtin_printer_ids,
    builtin_printer_profiles, emit_job_variable_width, emit_printer_job, extrusion_length_mm,
    printer_profile,
};

fn square_layers(origin: [f64; 2], size: f64, layers: usize, height: f64) -> Vec<PlannedLayer> {
    let [x, y] = origin;
    (0..layers)
        .map(|i| PlannedLayer {
            z_mm: height * (i + 1) as f64,
            paths: vec![PlannedPath {
                points: vec![[x, y], [x + size, y], [x + size, y + size], [x, y + size]],
                closed: true,
            }],
        })
        .collect()
}

fn moves(gcode: &str) -> impl Iterator<Item = (Option<f64>, Option<f64>, Option<f64>)> + '_ {
    gcode.lines().filter_map(|line| {
        let command = line.split(';').next()?.trim();
        let mut words = command.split_whitespace();
        let head = words.next()?;
        if head != "G0" && head != "G1" {
            return None;
        }
        let (mut x, mut y, mut e) = (None, None, None);
        for word in words {
            let value = word[1..].parse::<f64>().ok();
            match &word[..1] {
                "X" => x = value,
                "Y" => y = value,
                "E" => e = value,
                _ => {}
            }
        }
        Some((x, y, e))
    })
}

fn final_e(gcode: &str) -> f64 {
    moves(gcode).filter_map(|(_, _, e)| e).fold(0.0, f64::max)
}

#[test]
fn at_least_three_builtin_profiles_validate_and_are_selectable() {
    let profiles = builtin_printer_profiles().unwrap();
    assert!(profiles.len() >= 3);
    let ids = builtin_printer_ids();
    assert_eq!(ids.len(), profiles.len());
    for id in &ids {
        let profile = printer_profile(id).unwrap();
        assert_eq!(&profile.id, id);
        profile.validate().unwrap();
    }
    let flavors: Vec<Flavor> = profiles.iter().map(|p| p.job.flavor).collect();
    for flavor in Flavor::ALL {
        assert!(
            flavors.contains(&flavor),
            "no built-in profile for {flavor:?}"
        );
    }
    assert!(printer_profile("no-such-printer").is_err());
}

#[test]
fn each_profile_respects_its_bed_and_firmware_dialect() {
    for profile in builtin_printer_profiles().unwrap() {
        let [bx, by, bz] = profile.bed_size_mm;
        // A part spanning nearly the whole bed fits.
        let layers = square_layers(
            [1.0, 1.0],
            bx.min(by) - 2.0,
            3,
            profile.machine().layer_height_mm,
        );
        let gcode = emit_printer_job(&layers, &[], &profile).unwrap();
        assert!(gcode.contains(&format!(";PRINTER:{}", profile.id)));
        assert!(gcode.contains(&format!(";FLAVOR:{}", profile.job.flavor.header_label())));
        for (x, y, _) in moves(&gcode) {
            if let Some(x) = x {
                assert!((0.0..=bx).contains(&x), "{}: X {x} off bed", profile.id);
            }
            if let Some(y) = y {
                assert!((0.0..=by).contains(&y), "{}: Y {y} off bed", profile.id);
            }
        }
        for custom in profile.start_gcode.iter().chain(&profile.end_gcode) {
            assert!(
                gcode.lines().any(|l| l == custom),
                "{} lacks {custom}",
                profile.id
            );
        }
        let start = gcode.find(";START_GCODE").unwrap();
        let first_layer = gcode.find(";LAYER:0").unwrap();
        let end = gcode.find(";END_GCODE").unwrap();
        assert!(start < first_layer && first_layer < end);
        match profile.job.flavor {
            Flavor::Marlin => {
                assert!(gcode.contains("M109 S") && gcode.contains("M200 D0"));
                assert!(!gcode.contains("SET_HEATER_TEMPERATURE"));
            }
            Flavor::Klipper => {
                assert!(gcode.contains("SET_HEATER_TEMPERATURE HEATER=extruder"));
                assert!(gcode.contains("TURN_OFF_HEATERS"));
                assert!(!gcode.contains("M200"));
            }
            Flavor::RepRapFirmware => {
                assert!(gcode.contains("G10 P0 S") && gcode.contains("M116"));
            }
        }
        // Retraction from the profile is used on long travels.
        let two_islands = vec![PlannedLayer {
            z_mm: profile.machine().layer_height_mm,
            paths: vec![
                square_layers([5.0, 5.0], 60.0, 1, 0.2)
                    .remove(0)
                    .paths
                    .remove(0),
                square_layers([100.0, 100.0], 60.0, 1, 0.2)
                    .remove(0)
                    .paths
                    .remove(0),
            ],
        }];
        let gcode = emit_printer_job(&two_islands, &[], &profile).unwrap();
        let retract_f = format!("F{:.3}", profile.job.retract_feedrate_mm_s * 60.0);
        assert!(
            gcode
                .lines()
                .any(|l| l.starts_with("G1 E") && l.ends_with(&retract_f))
        );

        // Off-bed in X, Y or Z is refused.
        let off_x = square_layers([bx - 1.0, 1.0], 5.0, 1, 0.2);
        let off_y = square_layers([1.0, by - 1.0], 5.0, 1, 0.2);
        let mut off_z = square_layers([1.0, 1.0], 5.0, 1, 0.2);
        off_z[0].z_mm = bz + 1.0;
        let off_neg = square_layers([-1.0, 1.0], 5.0, 1, 0.2);
        for bad in [off_x, off_y, off_z, off_neg] {
            let err = emit_printer_job(&bad, &[], &profile).unwrap_err();
            assert_eq!(err.code, "GCODE_BED_BOUNDS", "{}", profile.id);
        }
    }
}

#[test]
fn profile_data_is_validated_on_load() {
    let good = include_str!("../profiles/prusa-mk3s.printer");
    assert!(PrinterProfile::parse(good).is_ok());
    let cases = [
        good.replace("bed_x_mm = 250", "bed_x_mm = 0"),
        good.replace("flavor = marlin", "flavor = smoothie"),
        good.replace("line_width_mm = 0.45", "line_width_mm = 1.2"),
        good.replace("retract_length_mm = 0.8\n", ""),
        good.replace("start_gcode = G80", "start_gcode = G92 E0"),
        good.replace("start_gcode = G80", "start_gcode = G1 X5 E3"),
        good.replace("nozzle_temp_c = 215", "nozzle_temp_c = 900"),
        format!("{good}\nbogus_key = 1\n"),
        good.replace("id = prusa-mk3s", "id = has space"),
    ];
    for case in cases {
        let err = PrinterProfile::parse(&case).unwrap_err();
        assert!(
            err.code == "GCODE_PRINTER_PROFILE" || err.code.starts_with("GCODE_"),
            "{}",
            err.code
        );
    }
}

#[test]
fn variable_width_extrusion_matches_bead_volume() {
    let profile = printer_profile("prusa-mk3s").unwrap();
    let machine = profile.machine().clone();
    let h = machine.layer_height_mm;
    let d = machine.filament_diameter_mm;
    // Thin wall: one open 10 mm segment at 0.25 mm and one 20 mm segment at 0.6 mm.
    let layers = vec![PlannedLayer {
        z_mm: h,
        paths: vec![PlannedPath {
            points: vec![[10.0, 10.0], [20.0, 10.0], [20.0, 30.0]],
            closed: false,
        }],
    }];
    let widths = vec![vec![vec![0.25, 0.6]]];
    let gcode = emit_job_variable_width(&layers, &widths, &profile.job).unwrap();
    let es: Vec<f64> = moves(&gcode).filter_map(|(x, _, e)| x.and(e)).collect();
    let area = std::f64::consts::PI * (d * 0.5).powi(2);
    let first = 0.25 * h * 10.0 / area;
    let second = 0.6 * h * 20.0 / area;
    assert!((es[0] - first).abs() < 1e-6, "{} vs {first}", es[0]);
    assert!((es[1] - es[0] - second).abs() < 1e-6);
    assert!((extrusion_length_mm(0.25, h, 10.0, d) - first).abs() < 1e-12);
    // Volume check: filament volume equals deposited bead volume.
    let bead_volume = 0.25 * h * 10.0 + 0.6 * h * 20.0;
    assert!((final_e(&gcode) * area - bead_volume).abs() < 1e-5);

    // Empty table falls back to the nominal line width.
    let nominal = emit_job_variable_width(&layers, &[], &profile.job).unwrap();
    let expected = machine.line_width_mm * h * 30.0 / area;
    assert!((final_e(&nominal) - expected).abs() < 1e-6);

    // Through the printer envelope as well.
    let printer = emit_printer_job(&layers, &widths, &profile).unwrap();
    assert!((final_e(&printer) - (first + second)).abs() < 1e-6);

    // Malformed width tables are refused.
    for bad in [
        vec![vec![vec![0.25]]],
        vec![vec![vec![0.25, -1.0]]],
        vec![vec![vec![0.25, 5.0]]],
        vec![],
    ] {
        if bad.is_empty() {
            continue;
        }
        let err = emit_job_variable_width(&layers, &bad, &profile.job).unwrap_err();
        assert_eq!(err.code, "GCODE_INVALID_WIDTH");
    }
}
