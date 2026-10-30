use super::*;

#[test]
fn arc_fit_welds_discretized_circle_into_g2_or_g3_arcs() {
    let mut pts = Vec::new();
    let r = 10.0;
    for k in 0..=16 {
        let theta = std::f64::consts::FRAC_PI_2 * (k as f64 / 16.0);
        pts.push([r * theta.cos(), r * theta.sin()]);
    }
    let fitted = arc_fit_polyline(&pts, false, &ArcWelderConfig::default());
    assert_eq!(fitted.len(), 1);
    match fitted[0] {
        FittedSegment::Arc {
            start,
            end,
            ij_offset,
            radius,
            clockwise,
        } => {
            assert!(!clockwise); // CCW quarter circle -> G3
            assert!((radius - 10.0).abs() < 1e-3);
            assert!((start[0] - 10.0).abs() < 1e-6);
            assert!((end[1] - 10.0).abs() < 1e-6);
            assert!((ij_offset[0] + 10.0).abs() < 1e-3);
            assert!(ij_offset[1].abs() < 1e-3);
        }
        _ => panic!("expected FittedSegment::Arc"),
    }
}

#[test]
fn emit_arc_gcode_writes_g3_and_zhop_retracts() {
    let mut arc_pts = Vec::new();
    for k in 0..=12 {
        let theta = std::f64::consts::FRAC_PI_2 * (k as f64 / 12.0);
        arc_pts.push([10.0 * theta.cos(), 10.0 * theta.sin()]);
    }
    let layers = vec![PlannedLayer {
        z_mm: 0.2,
        paths: vec![
            PlannedPath {
                points: arc_pts,
                closed: false,
            },
            PlannedPath {
                points: vec![[30.0, 30.0], [40.0, 30.0]],
                closed: false,
            },
        ],
    }];
    let machine = MachineProfile {
        layer_height_mm: 0.2,
        line_width_mm: 0.4,
        print_feedrate_mm_s: 50.0,
        travel_feedrate_mm_s: 120.0,
        filament_diameter_mm: 1.75,
    };
    let gcode = emit_arc_gcode(
        &layers,
        &machine,
        &ArcWelderConfig::default(),
        Some(&RetractZHopConfig::default()),
    )
    .unwrap();
    assert!(gcode.contains("G3 X0.000 Y10.000 I-10.000 J0.000"));
    assert!(gcode.contains("G0 Z0.400")); // Z-hop lift of 0.2mm above z=0.2mm
}

#[test]
fn pressure_advance_input_shaper_and_klipper_print_time() {
    let gcode = "G21\nG90\nG1 X10 Y0 E0.5 F3000\nG1 X10 Y10 E1.0 F3000\n";
    let klipper_pa = inject_pressure_advance(gcode, Flavor::Klipper, 0.045, 0.04).unwrap();
    assert!(klipper_pa.contains("SET_PRESSURE_ADVANCE ADVANCE=0.04500 SMOOTH_TIME=0.04000"));

    let marlin_pa = inject_pressure_advance(gcode, Flavor::Marlin, 0.06, 0.04).unwrap();
    assert!(marlin_pa.contains("M900 K0.06000"));

    let shaper = format_input_shaper_command(Flavor::Klipper, "mzv", 54.2, 48.6).unwrap();
    assert_eq!(
        shaper,
        "SET_INPUT_SHAPER SHAPER_TYPE=MZV SHAPER_FREQ_X=54.20 SHAPER_FREQ_Y=48.60"
    );

    let moves = vec![
        GcodeMove {
            x: 0.0,
            y: 0.0,
            z: 0.2,
            e: 0.0,
            feedrate_mm_s: 100.0,
            layer_index: 0,
            extruded: false,
        },
        GcodeMove {
            x: 50.0,
            y: 0.0,
            z: 0.2,
            e: 1.5,
            feedrate_mm_s: 100.0,
            layer_index: 0,
            extruded: true,
        },
        GcodeMove {
            x: 50.0,
            y: 50.0,
            z: 0.2,
            e: 3.0,
            feedrate_mm_s: 100.0,
            layer_index: 0,
            extruded: true,
        },
    ];
    // Ideal time at 100 mm/s for 100 mm is 1.0 s; with finite acceleration (3000 mm/s^2) and 90 deg corner (scv=5 mm/s), time > 1.0 s
    let time_s = estimate_klipper_print_time(&moves, 3000.0, 5.0);
    assert!(time_s > 1.05 && time_s < 1.5, "got {time_s}");

    let fan_sched = inject_ramping_fan_control(5, 64, 255, 3);
    assert_eq!(fan_sched.len(), 5);
    assert_eq!(fan_sched[0], (0, "M107".to_string()));
    assert!(fan_sched[1].1.starts_with("M106 S"));
    assert_eq!(fan_sched[3], (3, "M106 S255".to_string()));
    assert_eq!(fan_sched[4], (4, "M106 S255".to_string()));

    // Straight collinear move -> full cruise velocity
    let v_straight = optimize_corner_velocity_limits([10.0, 0.0, 0.0], [10.0, 0.0, 0.0], 3000.0, 0.02, 120.0);
    assert!((v_straight - 120.0).abs() < 1e-6);
    // 90-degree corner with JD = 0.02 mm and accel = 3000 mm/s^2 -> v = sqrt(3000 * 0.02 * sin(45) / (1 - sin(45))) ≈ 12.04 mm/s
    let v_90 = optimize_corner_velocity_limits([10.0, 0.0, 0.0], [0.0, 10.0, 0.0], 3000.0, 0.02, 120.0);
    assert!(v_90 > 11.5 && v_90 < 12.5, "got {v_90}");
    // 180-degree reversal -> 0.0 mm/s
    let v_rev = optimize_corner_velocity_limits([10.0, 0.0, 0.0], [-10.0, 0.0, 0.0], 3000.0, 0.02, 120.0);
    assert!(v_rev < 1e-3);

    // Spiral vase mode continuous helical Z interpolation
    let sq = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
    let vase = plan_spiral_vase_mode(&[(0.2, sq.clone()), (0.4, sq.clone()), (0.6, sq)], 0.2);
    assert_eq!(vase.len(), 13); // 5 on first layer + 4 + 4 on next two layers (seam deduplicated)
    for w in vase.windows(2) {
        assert!(w[1][2] > w[0][2], "Z must strictly increase along spiral vase path: {:?} -> {:?}", w[0], w[1]);
    }
    assert!((vase.first().unwrap()[2] - 0.2).abs() < 1e-9);
    assert!((vase.last().unwrap()[2] - 0.8).abs() < 1e-9);

    // Nozzle wipe retraction back-tracing along extrusion path before travel
    let path_moves = vec![
        [0.0, 0.0, 0.2, 0.0],
        [10.0, 0.0, 0.2, 0.5],
        [10.0, 10.0, 0.2, 1.0],
        [30.0, 30.0, 0.2, 1.0], // non-extruding travel move
    ];
    let wiped = inject_wiper_retraction(&path_moves, 0.8, 3.0);
    // Should insert a wipe move back from [10.0, 10.0] toward [10.0, 0.0] at [10.0, 7.0, 0.2, 0.2]
    // and restore E=1.0 at the travel destination
    assert!(wiped.len() > path_moves.len());
    assert!(wiped.iter().any(|m| (m[0] - 10.0).abs() < 1e-6 && (m[1] - 7.0).abs() < 1e-6 && (m[3] - 0.2).abs() < 1e-6));

    // Direct circular arc fit detection & collinear extrusion move compression
    let mut ccw_arc = Vec::new();
    for k in 0..=8 {
        let theta = std::f64::consts::FRAC_PI_2 * (k as f64 / 8.0);
        ccw_arc.push([5.0 + 12.0 * theta.cos(), -3.0 + 12.0 * theta.sin()]);
    }
    let (center, radius, is_cw) = detect_circular_arc_fit(&ccw_arc, 0.02).unwrap();
    assert!(!is_cw);
    assert!((radius - 12.0).abs() < 1e-3);
    assert!((center[0] - 5.0).abs() < 1e-3 && (center[1] + 3.0).abs() < 1e-3);

    let collinear_moves = vec![
        [0.0, 0.0, 0.2, 0.0],
        [2.0, 0.0, 0.2, 0.1],
        [5.0, 0.001, 0.2, 0.25],
        [10.0, 0.0, 0.2, 0.5],
        [10.0, 10.0, 0.2, 1.0],
    ];
    let compressed = compress_collinear_extrusion_moves(&collinear_moves, 0.01);
    assert_eq!(compressed.len(), 3);
    assert_eq!(compressed[0], [0.0, 0.0, 0.2, 0.0]);
    assert_eq!(compressed[1], [10.0, 0.0, 0.2, 0.5]);
    assert_eq!(compressed[2], [10.0, 10.0, 0.2, 1.0]);
}
