use gcode_core::{
    JobOutputOptions, JobProfile, PlannedLayer, PlannedPath, emit_configured_job, parse_foreign,
};
#[test]
fn mode_matrix_preserves_endpoints_and_filament() {
    let layers = [PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[10., 0.], [20., 0.]],
            closed: false,
        }],
    }];
    let mut expected: Option<f64> = None;
    for inches in [false, true] {
        for relative_xyz in [false, true] {
            for relative_e in [false, true] {
                let options = JobOutputOptions {
                    inches,
                    relative_xyz,
                    relative_e,
                    start_template: "G91\nG20\nM83\n".into(),
                    z_hop_mm: 0.5,
                    ..Default::default()
                };
                let code = emit_configured_job(&layers, &JobProfile::default(), &options).unwrap();
                let preview = parse_foreign(&code).unwrap();
                if let Some(value) = expected {
                    assert!((preview.extrusion_mm - value).abs() < 1e-5)
                } else {
                    expected = Some(preview.extrusion_mm)
                }
                assert!(code.contains("configured-job 1"));
                assert!(preview.print_distance_mm > 9.999 && preview.print_distance_mm < 10.001);
            }
        }
    }
}

#[test]
fn distant_paths_lift_and_recover_in_every_mode() {
    let layers = [PlannedLayer {
        z_mm: 2.0,
        paths: vec![
            PlannedPath {
                points: vec![[10., 0.], [50., 0.]],
                closed: false,
            },
            PlannedPath {
                points: vec![[100., 20.], [140., 20.]],
                closed: false,
            },
        ],
    }];
    for inches in [false, true] {
        for relative_xyz in [false, true] {
            for relative_e in [false, true] {
                let options = JobOutputOptions {
                    inches,
                    relative_xyz,
                    relative_e,
                    z_hop_mm: 0.5,
                    ..Default::default()
                };
                let code = emit_configured_job(&layers, &JobProfile::default(), &options).unwrap();
                let preview = parse_foreign(&code).unwrap();
                assert!((preview.print_distance_mm - 80.0).abs() < 1e-4);
                assert!(preview.moves.iter().any(|m| (m.z - 2.5).abs() < 1e-5));
                assert!(
                    preview
                        .moves
                        .iter()
                        .filter(|m| m.extruded)
                        .all(|m| (m.z - 2.0).abs() < 1e-5)
                );
            }
        }
    }
}

#[test]
fn firmware_specific_options_refuse_incompatible_targets() {
    let layers = [PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[0., 0.], [10., 0.]],
            closed: false,
        }],
    }];
    let job = JobProfile {
        flavor: gcode_core::Flavor::Klipper,
        ..Default::default()
    };
    assert!(
        emit_configured_job(
            &layers,
            &job,
            &JobOutputOptions {
                inches: true,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        emit_configured_job(
            &layers,
            &job,
            &JobOutputOptions {
                chamber_temp_c: Some(50.0),
                ..Default::default()
            }
        )
        .is_err()
    );
    let code = emit_configured_job(
        &layers,
        &JobProfile::default(),
        &JobOutputOptions {
            chamber_temp_c: Some(50.0),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(code.contains("M141 S50\nM191 S50"));
}

#[test]
fn configured_package_reopens_the_exact_emitted_program() {
    let layers = [PlannedLayer {
        z_mm: 0.2,
        paths: vec![PlannedPath {
            points: vec![[10., 0.], [50., 0.]],
            closed: false,
        }],
    }];
    let job = JobProfile::default();
    let code = emit_configured_job(
        &layers,
        &job,
        &JobOutputOptions {
            inches: true,
            relative_xyz: true,
            relative_e: true,
            ..Default::default()
        },
    )
    .unwrap();
    let package = gcode_core::package_job_3mf(&code, &job, None).unwrap();
    assert_eq!(gcode_core::extract_gcode_3mf(&package).unwrap(), code);
    let preview = gcode_core::parse_3mf(&package).unwrap();
    assert!((preview.print_distance_mm - 40.).abs() < 1e-5);
}
