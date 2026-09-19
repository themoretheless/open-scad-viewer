use gcode_core::{
    JOB_DIALECT, JobProfile, MeshBody, PLATE_JSON_PATH, PlannedLayer, PlannedPath,
    emit_gcode_3mf_job, emit_job, extract_gcode_3mf, extract_member_3mf, parse_job,
};

fn two_islands() -> PlannedLayer {
    PlannedLayer {
        z_mm: 0.2,
        paths: vec![
            PlannedPath {
                points: vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
                closed: true,
            },
            PlannedPath {
                points: vec![[20.0, 0.0], [30.0, 0.0], [30.0, 10.0], [20.0, 10.0]],
                closed: true,
            },
        ],
    }
}

fn cube() -> MeshBody {
    MeshBody {
        positions: vec![
            0., 0., 0., 1., 0., 0., 1., 1., 0., 0., 1., 0., 0., 0., 1., 1., 0., 1., 1., 1., 1., 0.,
            1., 1.,
        ],
        indices: vec![
            0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6, 0, 4, 5, 0, 5, 1, 1, 5, 6, 1, 6, 2, 2, 6, 7, 2, 7,
            3, 3, 7, 4, 3, 4, 0,
        ],
    }
}

#[test]
fn job_gcode_includes_heat_retract_and_round_trips() {
    let job = JobProfile::default();
    let gcode = emit_job(&[two_islands()], &job).unwrap();
    assert!(gcode.starts_with(&format!("; {JOB_DIALECT}\n")));
    assert!(gcode.contains("M109 S210"));
    assert!(gcode.contains("M190 S60"));
    assert!(gcode.contains("G28\n"));
    assert!(gcode.contains("M104 S0"));
    // Absolute-E retract / unretract between distant islands.
    assert!(
        gcode
            .lines()
            .filter(|line| line.starts_with("G1 E"))
            .count()
            >= 2
    );
    let preview = parse_job(&gcode).unwrap();
    assert_eq!(preview.layers, 1);
    assert!(preview.extrusion_mm > 0.0);
    assert!(preview.print_distance_mm > 70.0);
}

#[test]
fn thick_gcode_3mf_carries_job_mesh_and_plate_json() {
    let job = JobProfile::default();
    let packaged = emit_gcode_3mf_job(&[two_islands()], &job, Some(&cube())).unwrap();
    let gcode = extract_gcode_3mf(&packaged).unwrap();
    assert!(gcode.contains("M109"));
    let model = extract_member_3mf(&packaged, "3D/3dmodel.model").unwrap();
    assert!(model.contains("<triangle"));
    let json = extract_member_3mf(&packaged, PLATE_JSON_PATH).unwrap();
    assert!(json.contains("\"plate\":1"));
    assert!(json.contains("\"gcode_md5\""));
    assert_eq!(parse_job(&gcode).unwrap().layers, 1);
}
