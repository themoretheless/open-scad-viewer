use gcode_core::{MachineProfile, PlannedLayer, PlannedPath};
use gcode_optimize::{
    emit_optimized, from_gcode, from_planned, from_toolpaths, optimize, travel_mm,
    OptimizeSettings, SeamPrefer,
};

fn machine() -> MachineProfile {
    MachineProfile::default()
}

fn open(points: Vec<[f64; 2]>) -> PlannedPath {
    PlannedPath {
        points,
        closed: false,
    }
}

fn closed(points: Vec<[f64; 2]>) -> PlannedPath {
    PlannedPath {
        points,
        closed: true,
    }
}

fn layer(z_mm: f64, paths: Vec<PlannedPath>) -> PlannedLayer {
    PlannedLayer { z_mm, paths }
}

#[test]
fn simplify_keeps_endpoints_and_drops_colinear() {
    let input = from_planned(&[layer(
        0.2,
        vec![open(vec![[0.0, 0.0], [5.0, 0.0], [10.0, 0.0]])],
    )]);
    let settings = OptimizeSettings {
        simplify_tolerance_mm: 0.1,
        ..OptimizeSettings::default()
    };
    let (layers, report) = optimize(input, &settings).unwrap();
    assert_eq!(layers[0].paths[0].points, vec![[0.0, 0.0], [10.0, 0.0]]);
    assert_eq!(report.simplified_vertices, 1);
}

#[test]
fn seam_min_x_rotates_closed_loop() {
    let input = from_planned(&[layer(
        0.2,
        vec![closed(vec![[10.0, 0.0], [10.0, 10.0], [0.0, 10.0], [0.0, 0.0]])],
    )]);
    let settings = OptimizeSettings {
        seam: SeamPrefer::MinX,
        ..OptimizeSettings::default()
    };
    let (layers, report) = optimize(input, &settings).unwrap();
    assert_eq!(layers[0].paths[0].points[0], [0.0, 0.0]);
    assert_eq!(report.seam_rotations, 1);
}

#[test]
fn order_reduces_travel_on_scattered_segments() {
    let bad = layer(
        0.2,
        vec![
            open(vec![[0.0, 0.0], [1.0, 0.0]]),
            open(vec![[100.0, 0.0], [101.0, 0.0]]),
            open(vec![[2.0, 0.0], [3.0, 0.0]]),
        ],
    );
    let before = travel_mm(&bad.paths);
    let (layers, _) = optimize(from_planned(&[bad]), &OptimizeSettings::default()).unwrap();
    let after = travel_mm(&layers[0].paths);
    assert!(after < before, "{after} !< {before}");
}

#[test]
fn combing_walks_around_a_hole() {
    let hole = vec![
        [8.0, 8.0],
        [12.0, 8.0],
        [12.0, 12.0],
        [8.0, 12.0],
    ];
    let input = from_planned(&[layer(
        0.2,
        vec![
            open(vec![[1.0, 10.0], [2.0, 10.0]]),
            open(vec![[18.0, 10.0], [19.0, 10.0]]),
        ],
    )])
    .with_avoid(0, vec![hole]);
    let (layers, report) = optimize(input, &OptimizeSettings::default()).unwrap();
    assert!(report.comb_waypoints > 0);
    assert!(layers[0].paths.iter().any(|path| path.points.len() == 1));
}

#[test]
fn long_gap_counts_as_virtual_retract() {
    let input = from_planned(&[layer(
        0.2,
        vec![
            open(vec![[0.0, 0.0], [1.0, 0.0]]),
            open(vec![[20.0, 0.0], [21.0, 0.0]]),
        ],
    )]);
    let (_, report) = optimize(input, &OptimizeSettings::default()).unwrap();
    assert_eq!(report.virtual_retracts, 1);
}

#[test]
fn emit_optimized_round_trips_volume_without_simplify() {
    let square = layer(
        0.2,
        vec![closed(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]])],
    );
    let gcode = emit_optimized(
        from_planned(std::slice::from_ref(&square)),
        &machine(),
        &OptimizeSettings::default(),
    )
    .unwrap();
    let preview = gcode_core::parse(&gcode).unwrap();
    let rebuilt = from_gcode(&gcode).unwrap();
    let again = emit_optimized(from_planned(&rebuilt), &machine(), &OptimizeSettings::default()).unwrap();
    let second = gcode_core::parse(&again).unwrap();
    let volume = gcode_core::deposited_volume_mm3(std::slice::from_ref(&square), &machine());
    assert!((preview.deposited_volume_mm3 - volume).abs() / volume < 1e-6);
    assert!((second.deposited_volume_mm3 - preview.deposited_volume_mm3).abs() / volume < 1e-6);
}

#[test]
fn from_toolpaths_drops_roles_and_emits() {
    let input = from_toolpaths([(
        0.2,
        vec![(vec![[0.0, 0.0], [4.0, 0.0], [4.0, 4.0], [0.0, 4.0]], true)],
    )]);
    let gcode = emit_optimized(input, &machine(), &OptimizeSettings::default()).unwrap();
    assert!(gcode.contains("G1"));
}

#[test]
fn islands_travel_does_not_increase() {
    let islands = layer(
        0.2,
        (0..8)
            .map(|i| {
                let x = if i % 2 == 0 {
                    i as f64 * 10.0
                } else {
                    (7 - i) as f64 * 10.0
                };
                open(vec![[x, 0.0], [x + 1.0, 0.0]])
            })
            .collect(),
    );
    let before = travel_mm(&islands.paths);
    let (layers, _) = optimize(from_planned(&[islands]), &OptimizeSettings::default()).unwrap();
    assert!(travel_mm(&layers[0].paths) <= before + 1e-9);
}

#[test]
fn simplify_volume_stays_within_tolerance_on_a_square() {
    let square = layer(
        0.2,
        vec![closed(vec![
            [0.0, 0.0],
            [5.0, 0.001],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ])],
    );
    let settings = OptimizeSettings {
        simplify_tolerance_mm: 0.05,
        ..OptimizeSettings::default()
    };
    let before = gcode_core::deposited_volume_mm3(std::slice::from_ref(&square), &machine());
    let (layers, _) = optimize(from_planned(std::slice::from_ref(&square)), &settings).unwrap();
    let after = gcode_core::deposited_volume_mm3(&layers, &machine());
    assert!((after - before).abs() / before < 0.05);
}

#[test]
fn emit_optimized_job_uses_print_job_dialect() {
    use gcode_core::{parse_job, JobProfile, JOB_DIALECT};
    use gcode_optimize::{emit_optimized_gcode_3mf_job, emit_optimized_job, MeshBody};

    let input = from_planned(&[layer(
        0.2,
        vec![
            closed(vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]]),
            closed(vec![[20.0, 0.0], [30.0, 0.0], [30.0, 10.0], [20.0, 10.0]]),
        ],
    )]);
    let job = JobProfile::default();
    let gcode = emit_optimized_job(input.clone(), &job, &OptimizeSettings::default()).unwrap();
    assert!(gcode.starts_with(&format!("; {JOB_DIALECT}\n")));
    assert!(gcode.contains("M109"));
    assert_eq!(parse_job(&gcode).unwrap().layers, 1);

    let mesh = MeshBody {
        positions: vec![
            0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 1., 1., 0., 1., 1., 1., 1., 0.,
            1., 1.,
        ],
        indices: vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7,
            3, 3, 7, 4, 3, 4, 0,
        ],
    };
    let packaged =
        emit_optimized_gcode_3mf_job(input, &job, &OptimizeSettings::default(), Some(&mesh))
            .unwrap();
    assert!(packaged.starts_with(b"PK"));
    assert!(gcode_core::extract_member_3mf(&packaged, "3D/3dmodel.model")
        .unwrap()
        .contains("<triangle"));
}
