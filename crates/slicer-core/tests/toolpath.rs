use polygon_core::solid::{
    primitives as cad,
    section::{MeshSection, MeshSectionIndex, SectionContour},
};
use slicer_core::{
    LayerSection, PathRole, Result, ToolpathSettings, deposited_volume_mm3, emit_gcode,
    parse_gcode_preview, plan_layer, schedule_layers,
};

fn settings() -> ToolpathSettings {
    ToolpathSettings {
        layer_height_mm: 0.2,
        line_width_mm: 0.4,
        wall_count: 2,
        infill_spacing_mm: 2.0,
        ..ToolpathSettings::default()
    }
}

fn layer_section(section: MeshSection) -> LayerSection {
    LayerSection {
        z_mm: section.z_mm,
        contours: section
            .contours
            .into_iter()
            .map(|contour| contour.points)
            .collect(),
    }
}

fn section_at(index: &MeshSectionIndex, z: f64) -> Result<LayerSection> {
    index
        .section(z)
        .map(layer_section)
        .map_err(|error| slicer_core::Error {
            code: error.code,
            message: error.message,
        })
}

#[test]
fn box_section_has_outline_and_hatch() {
    let mesh = cad::cube([20.0, 30.0, 10.0], false).unwrap();
    let index = MeshSectionIndex::new(&mesh).unwrap();
    let section = section_at(&index, 0.1).unwrap();
    assert!(!section.contours.is_empty());
    let layer = plan_layer(&section, &settings()).unwrap();
    assert!(
        layer
            .paths
            .iter()
            .any(|path| path.role == PathRole::Outline && path.closed)
    );
    assert!(
        layer
            .paths
            .iter()
            .any(|path| path.role == PathRole::Inset && path.closed)
    );
    assert!(
        layer
            .paths
            .iter()
            .any(|path| path.role == PathRole::Hatch && path.points.len() == 2)
    );
}

#[test]
fn annulus_keeps_a_hole_out_of_hatch() {
    let outer = vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]];
    let hole = vec![[8.0, 8.0], [8.0, 12.0], [12.0, 12.0], [12.0, 8.0]];
    let section = layer_section(MeshSection {
        z_mm: 0.1,
        contours: vec![
            SectionContour {
                points: outer,
                source_triangles: vec![0, 0, 0, 0],
            },
            SectionContour {
                points: hole,
                source_triangles: vec![1, 1, 1, 1],
            },
        ],
        candidate_triangles: 2,
    });
    let layer = plan_layer(&section, &settings()).unwrap();
    for path in layer
        .paths
        .iter()
        .filter(|path| path.role == PathRole::Hatch)
    {
        let mid = [
            (path.points[0][0] + path.points[1][0]) * 0.5,
            (path.points[0][1] + path.points[1][1]) * 0.5,
        ];
        assert!(
            mid[0] < 8.0 || mid[0] > 12.0 || mid[1] < 8.0 || mid[1] > 12.0,
            "hatch crossed the hole at {mid:?}"
        );
    }
}

#[test]
fn gcode_is_one_encoding_of_the_plan() {
    let mesh = cad::cube([20.0, 30.0, 10.0], false).unwrap();
    let index = MeshSectionIndex::new(&mesh).unwrap();
    let settings = settings();
    let layers = schedule_layers(|z| section_at(&index, z), 0.0, 10.0, &settings).unwrap();
    assert!(!layers.is_empty());
    let gcode = emit_gcode(&layers, &settings).unwrap();
    let preview = parse_gcode_preview(&gcode).unwrap();
    assert_eq!(preview.layers, layers.len());
    assert!(preview.extrusion_mm > 0.0);
    let volume = deposited_volume_mm3(&layers, &settings);
    assert!(volume > 0.0);
    let filament_area = std::f64::consts::PI * (settings.filament_diameter_mm * 0.5).powi(2);
    let from_e = preview.extrusion_mm * filament_area;
    assert!((from_e - volume).abs() / volume < 0.15);
}

#[test]
fn optimized_job_pipeline_emits_heat_and_package() {
    use slicer_core::{
        JobProfile, MeshBody, OptimizeSettings, emit_job_gcode, emit_job_gcode_3mf,
        emit_optimized_gcode, parse_gcode_job, job_profile,
    };
    let mesh = cad::cube([20.0, 30.0, 10.0], false).unwrap();
    let index = MeshSectionIndex::new(&mesh).unwrap();
    let settings = settings();
    let layers = schedule_layers(|z| section_at(&index, z), 0.0, 4.0, &settings).unwrap();
    let optimize = OptimizeSettings::default();
    let preview = emit_optimized_gcode(&layers, &settings, &optimize).unwrap();
    assert!(preview.starts_with("; open-scad-viewer/print-preview 2\n"));
    let job = job_profile(&settings, JobProfile::default()).unwrap();
    let gcode = emit_job_gcode(&layers, &job, &optimize).unwrap();
    assert!(gcode.contains("M109"));
    assert_eq!(parse_gcode_job(&gcode).unwrap().layers, layers.len());
    let body = MeshBody {
        positions: mesh.positions.clone(),
        indices: mesh.indices.clone(),
    };
    let packaged = emit_job_gcode_3mf(&layers, &job, &optimize, Some(&body)).unwrap();
    assert!(packaged.starts_with(b"PK"));
    assert_eq!(gcode_core::extract_gcode_3mf(&packaged).unwrap(), gcode);
}

fn rectangle(min: [f64; 2], max: [f64; 2]) -> Vec<[f64; 2]> {
    vec![min, [max[0], min[1]], max, [min[0], max[1]]]
}

fn empty_section(z_mm: f64) -> Result<LayerSection> {
    Ok(LayerSection {
        z_mm,
        contours: Vec::new(),
    })
}

#[test]
fn schedule_counts_empty_samples_and_rejects_excess_before_calling_host() {
    let settings = ToolpathSettings {
        layer_height_mm: 1.0,
        ..settings()
    };
    let mut calls = 0;
    let error = schedule_layers(
        |z| {
            calls += 1;
            empty_section(z)
        },
        0.0,
        slicer_core::MAX_LAYERS as f64 + 1.0,
        &settings,
    )
    .unwrap_err();
    assert_eq!(error.code, "TOOLPATH_LAYER_LIMIT");
    assert_eq!(calls, 0);
    let layers = schedule_layers(
        |z| {
            calls += 1;
            empty_section(z)
        },
        0.0,
        slicer_core::MAX_LAYERS as f64,
        &settings,
    )
    .unwrap();
    assert!(layers.is_empty());
    assert_eq!(calls, slicer_core::MAX_LAYERS);
}

#[test]
fn schedule_samples_half_open_range_without_roundoff_extra_layer() {
    let mut samples = Vec::new();
    schedule_layers(
        |z| {
            samples.push(z);
            empty_section(z)
        },
        0.0,
        10.0,
        &settings(),
    )
    .unwrap();
    assert_eq!(samples.len(), 50);
    assert_eq!(samples[0], 0.0);
    assert_eq!(samples[49], 9.8);
    assert!(samples.windows(2).all(|pair| pair[0] < pair[1]));

    samples.clear();
    schedule_layers(
        |z| {
            samples.push(z);
            empty_section(z)
        },
        -0.2,
        0.11,
        &settings(),
    )
    .unwrap();
    assert_eq!(samples, [-0.2, 0.0]);
}

#[test]
fn schedule_rejects_invalid_range_and_mismatched_callback_height() {
    for (min, max) in [
        (f64::NEG_INFINITY, 1.0),
        (0.0, f64::NAN),
        (1.0, 1.0),
        (-1e308, 1e308),
    ] {
        assert_eq!(
            schedule_layers(empty_section, min, max, &settings())
                .unwrap_err()
                .code,
            "TOOLPATH_INVALID_RANGE"
        );
    }
    for returned_z in [f64::NAN, 0.2] {
        let error =
            schedule_layers(|_| empty_section(returned_z), 0.0, 0.1, &settings()).unwrap_err();
        assert_eq!(error.code, "TOOLPATH_INVALID_HEIGHT");
    }
}

#[test]
fn settings_reject_underflow_overflow_and_unrepresentable_spacing() {
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![rectangle([0.0, 0.0], [10.0, 10.0])],
    };
    for invalid in [
        f64::NAN,
        f64::INFINITY,
        f64::MAX,
        f64::MIN_POSITIVE,
        0.0,
        -1.0,
    ] {
        for field in 0..6 {
            let mut settings = settings();
            match field {
                0 => settings.layer_height_mm = invalid,
                1 => settings.line_width_mm = invalid,
                2 => settings.infill_spacing_mm = invalid,
                3 => settings.feedrate_mm_s = invalid,
                4 => settings.travel_feedrate_mm_s = invalid,
                _ => settings.filament_diameter_mm = invalid,
            }
            assert_eq!(
                plan_layer(&section, &settings).unwrap_err().code,
                "TOOLPATH_INVALID_SETTINGS",
                "field {field}, value {invalid}"
            );
        }
    }
}

#[test]
fn invalid_contours_are_rejected_before_offsetting() {
    for contours in [
        vec![vec![]],
        vec![vec![[0.0, 0.0], [1.0, 1.0]]],
        vec![rectangle([0.0, 0.0], [f64::NAN, 1.0])],
        vec![rectangle([0.0, 0.0], [f64::INFINITY, 1.0])],
        vec![rectangle([-1e308, -1e308], [1e308, 1e308])],
    ] {
        let section = LayerSection {
            z_mm: 0.0,
            contours,
        };
        assert_eq!(
            plan_layer(&section, &settings()).unwrap_err().code,
            "TOOLPATH_INVALID_GEOMETRY"
        );
    }
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![vec![[0.0, 0.0]; slicer_core::MAX_SECTION_POINTS + 1]],
    };
    assert_eq!(
        plan_layer(&section, &settings()).unwrap_err().code,
        "TOOLPATH_GEOMETRY_LIMIT"
    );
}

#[test]
fn consumed_wall_offsets_do_not_reappear_as_phantom_islands() {
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![rectangle([0.0, 0.0], [0.1, 0.1])],
    };
    assert!(plan_layer(&section, &settings()).unwrap().paths.is_empty());

    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![rectangle([0.0, 0.0], [1.0, 1.0])],
    };
    let layer = plan_layer(&section, &settings()).unwrap();
    assert_eq!(layer.paths.len(), 1);
    assert_eq!(layer.paths[0].role, PathRole::Outline);
    for point in &layer.paths[0].points {
        assert!((0.2 - 1e-10..=0.8 + 1e-10).contains(&point[0]));
        assert!((0.2 - 1e-10..=0.8 + 1e-10).contains(&point[1]));
    }
}

#[test]
fn thin_neck_splits_without_out_of_material_paths() {
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![vec![
            [0.0, 0.0],
            [4.0, 0.0],
            [4.0, 1.9],
            [8.0, 1.9],
            [8.0, 0.0],
            [12.0, 0.0],
            [12.0, 4.0],
            [8.0, 4.0],
            [8.0, 2.1],
            [4.0, 2.1],
            [4.0, 4.0],
            [0.0, 4.0],
        ]],
    };
    let layer = plan_layer(&section, &settings()).unwrap();
    assert_eq!(
        layer
            .paths
            .iter()
            .filter(|path| path.role == PathRole::Outline)
            .count(),
        2
    );
    for path in &layer.paths {
        assert!(path.points.iter().all(|p| p[0] < 4.0 || p[0] > 8.0));
    }
}

#[test]
fn complete_hatch_segments_and_bead_radius_avoid_holes() {
    let mut hole = rectangle([8.0, 8.0], [12.0, 12.0]);
    hole.reverse();
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![rectangle([0.0, 0.0], [20.0, 20.0]), hole],
    };
    let settings = ToolpathSettings {
        infill_spacing_mm: 0.4,
        ..settings()
    };
    let layer = plan_layer(&section, &settings).unwrap();
    let radius = settings.line_width_mm * 0.5;
    let hatches: Vec<_> = layer
        .paths
        .iter()
        .filter(|path| path.role == PathRole::Hatch)
        .collect();
    assert!(!hatches.is_empty());
    for path in hatches {
        let y = path.points[0][1];
        let min_x = path.points[0][0].min(path.points[1][0]);
        let max_x = path.points[0][0].max(path.points[1][0]);
        assert!(min_x >= radius && max_x <= 20.0 - radius && y >= radius && y <= 20.0 - radius);
        assert!(
            y + radius <= 8.0
                || y - radius >= 12.0
                || max_x + radius <= 8.0
                || min_x - radius >= 12.0,
            "bead crosses hole: {:?}",
            path.points
        );
    }
}

#[test]
fn reversed_source_winding_preserves_material() {
    let mut contour = rectangle([0.0, 0.0], [10.0, 10.0]);
    let forward = plan_layer(
        &LayerSection {
            z_mm: 0.0,
            contours: vec![contour.clone()],
        },
        &settings(),
    )
    .unwrap();
    contour.reverse();
    let reversed = plan_layer(
        &LayerSection {
            z_mm: 0.0,
            contours: vec![contour],
        },
        &settings(),
    )
    .unwrap();
    assert!(!forward.paths.is_empty());
    assert_eq!(
        deposited_volume_mm3(&[forward], &settings()),
        deposited_volume_mm3(&[reversed], &settings())
    );
}

#[test]
fn dense_hatch_is_rejected_with_a_bounded_error() {
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![rectangle([0.0, 0.0], [10.0, 10.0])],
    };
    let settings = ToolpathSettings {
        infill_spacing_mm: 1e-5,
        ..settings()
    };
    assert_eq!(
        plan_layer(&section, &settings).unwrap_err().code,
        "TOOLPATH_HATCH_LIMIT"
    );
}

#[test]
fn schedule_shares_one_geometry_work_budget_across_layers() {
    let contour: Vec<_> = (0..128)
        .map(|i| {
            let angle = std::f64::consts::TAU * i as f64 / 128.0;
            [10.0 * angle.cos(), 10.0 * angle.sin()]
        })
        .collect();
    let settings = ToolpathSettings {
        wall_count: 1,
        infill_spacing_mm: 100.0,
        ..settings()
    };
    let section = LayerSection {
        z_mm: 0.0,
        contours: vec![contour.clone()],
    };
    assert!(!plan_layer(&section, &settings).unwrap().paths.is_empty());
    let mut calls = 0;
    let error = schedule_layers(
        |z_mm| {
            calls += 1;
            Ok(LayerSection {
                z_mm,
                contours: vec![contour.clone()],
            })
        },
        0.0,
        20.0,
        &settings,
    )
    .unwrap_err();
    assert_eq!(error.code, "TOOLPATH_WORK_LIMIT");
    assert!(
        calls > 1 && calls < 100,
        "budget did not accumulate: {calls} calls"
    );
}
