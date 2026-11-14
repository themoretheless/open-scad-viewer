use gcode_core::{
    JobProfile, PlannedLayer, PlannedPath, emit_job_variable_width, extrusion_length_mm,
    printer_profile,
};
use slicer_core::{
    InfillPattern, LayerSection, PathRole, ShellConfig, ThinWallConfig, ToolpathLayer,
    ToolpathSettings, plan_layers_with_shells, plan_layers_with_thin_walls,
};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn section(contours: Vec<Vec<[f64; 2]>>) -> Vec<LayerSection> {
    vec![LayerSection {
        z_mm: 0.2,
        contours,
    }]
}

fn planned(layers: &[ToolpathLayer]) -> Vec<PlannedLayer> {
    layers
        .iter()
        .map(|layer| PlannedLayer {
            z_mm: layer.z_mm,
            paths: layer
                .paths
                .iter()
                .map(|path| PlannedPath {
                    points: path.points.clone(),
                    closed: path.closed,
                })
                .collect(),
        })
        .collect()
}

fn max_e(gcode: &str) -> f64 {
    gcode
        .lines()
        .filter(|line| line.starts_with("G1"))
        .flat_map(|line| line.split_whitespace())
        .filter_map(|word| word.strip_prefix('E'))
        .filter_map(|v| v.parse::<f64>().ok())
        .fold(0.0, f64::max)
}

fn nozzle_04() -> ThinWallConfig {
    ThinWallConfig::for_nozzle(0.4)
}

#[test]
fn thin_rib_becomes_one_centre_bead_with_local_width_and_matching_e() {
    let settings = ToolpathSettings::default();
    let sections = section(vec![rect(0.0, 0.0, 10.0, 0.6)]);
    let plan = plan_layers_with_thin_walls(
        &sections,
        &settings,
        InfillPattern::Rectilinear,
        &ShellConfig::default(),
        None,
        &nozzle_04(),
    )
    .unwrap();
    let layer = &plan.layers[0];
    assert_eq!(layer.paths.len(), 1, "{:?}", layer.paths);
    let bead = &layer.paths[0];
    assert_eq!(bead.role, PathRole::ThinWall);
    assert!(!bead.closed);
    for p in &bead.points {
        assert!((p[1] - 0.3).abs() < 1e-6, "off-centre point {p:?}");
    }
    let widths = &plan.widths[0][0];
    assert_eq!(widths.len(), bead.points.len() - 1);
    for &w in widths {
        assert!((w - 0.6).abs() < 1e-6, "width {w}");
    }
    let length: f64 = bead
        .points
        .windows(2)
        .map(|s| ((s[1][0] - s[0][0]).powi(2) + (s[1][1] - s[0][1]).powi(2)).sqrt())
        .sum();
    // Bead ends sit half a width inside the rib ends.
    assert!((length - 9.4).abs() < 1e-6, "length {length}");

    let job = JobProfile::default();
    let gcode = emit_job_variable_width(&planned(&plan.layers), &plan.widths, &job).unwrap();
    let m = &job.machine;
    let expected = extrusion_length_mm(0.6, m.layer_height_mm, length, m.filament_diameter_mm);
    let e = max_e(&gcode);
    assert!(
        (e - expected).abs() < 1e-4 + expected * 1e-4,
        "E {e} expected {expected}"
    );
    // Nominal width would under-extrude by a third.
    let nominal = extrusion_length_mm(0.4, m.layer_height_mm, length, m.filament_diameter_mm);
    assert!(e > nominal * 1.4);
}

#[test]
fn rib_width_is_clamped_to_profile_range() {
    let settings = ToolpathSettings::default();
    let config = ThinWallConfig {
        min_width_mm: 0.35,
        max_width_mm: 0.5,
    };
    let plan = plan_layers_with_thin_walls(
        &section(vec![rect(0.0, 0.0, 10.0, 0.6)]),
        &settings,
        InfillPattern::Rectilinear,
        &ShellConfig::default(),
        None,
        &config,
    )
    .unwrap();
    assert!(plan.widths[0][0].iter().all(|&w| (w - 0.5).abs() < 1e-9));
}

#[test]
fn thick_wall_is_unchanged_and_gets_nominal_widths() {
    let settings = ToolpathSettings::default();
    let sections = section(vec![rect(0.0, 0.0, 20.0, 20.0)]);
    let shells = ShellConfig::default();
    let reference =
        plan_layers_with_shells(&sections, &settings, InfillPattern::Grid, &shells, None).unwrap();
    let plan = plan_layers_with_thin_walls(
        &sections,
        &settings,
        InfillPattern::Grid,
        &shells,
        None,
        &nozzle_04(),
    )
    .unwrap();
    assert_eq!(plan.layers, reference);
    assert!(
        plan.layers[0]
            .paths
            .iter()
            .all(|p| p.role != PathRole::ThinWall)
    );
    for (path, widths) in plan.layers[0].paths.iter().zip(&plan.widths[0]) {
        assert!(widths.iter().all(|&w| w == settings.line_width_mm));
        let segments = path.points.len() - 1 + usize::from(path.closed);
        assert_eq!(widths.len(), segments);
    }
}

#[test]
fn block_with_attached_rib_keeps_block_perimeters_and_adds_one_bead() {
    let settings = ToolpathSettings::default();
    // 10x10 block with a 0.6 mm rib running 8 mm out of its right side.
    let outline = vec![
        [0.0, 0.0],
        [10.0, 0.0],
        [10.0, 4.7],
        [18.0, 4.7],
        [18.0, 5.3],
        [10.0, 5.3],
        [10.0, 10.0],
        [0.0, 10.0],
    ];
    let plan = plan_layers_with_thin_walls(
        &section(vec![outline]),
        &settings,
        InfillPattern::Rectilinear,
        &ShellConfig::default(),
        None,
        &nozzle_04(),
    )
    .unwrap();
    let layer = &plan.layers[0];
    let beads: Vec<_> = layer
        .paths
        .iter()
        .enumerate()
        .filter(|(_, p)| p.role == PathRole::ThinWall)
        .collect();
    assert_eq!(beads.len(), 1);
    let (index, bead) = beads[0];
    assert!(
        bead.points
            .iter()
            .all(|p| p[0] > 9.5 && (p[1] - 5.0).abs() < 0.05)
    );
    assert!(bead.points.iter().any(|p| p[0] > 17.0));
    let widths = &plan.widths[0][index];
    assert!(widths.iter().all(|&w| (w - 0.6).abs() < 0.05), "{widths:?}");
    // Block perimeters stay inside the block.
    for path in layer.paths.iter().filter(|p| p.role == PathRole::Outline) {
        assert!(path.points.iter().all(|p| p[0] < 10.5));
    }
    let profile = printer_profile("prusa-mk3s").unwrap();
    let (lo, hi) = profile.extrusion_width_range_mm();
    assert!(lo < 0.6 && 0.6 < hi);
}
