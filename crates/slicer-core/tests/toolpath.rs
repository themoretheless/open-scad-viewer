use polygon_core::solid::{
    primitives as cad,
    section::{MeshSection, MeshSectionIndex, SectionContour},
};
use slicer_core::{
    deposited_volume_mm3, emit_gcode, parse_gcode_preview, plan_layer, schedule_layers,
    LayerSection, PathRole, Result, ToolpathSettings,
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
    assert!(layer
        .paths
        .iter()
        .any(|path| path.role == PathRole::Outline && path.closed));
    assert!(layer
        .paths
        .iter()
        .any(|path| path.role == PathRole::Inset && path.closed));
    assert!(layer
        .paths
        .iter()
        .any(|path| path.role == PathRole::Hatch && path.points.len() == 2));
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
    for path in layer.paths.iter().filter(|path| path.role == PathRole::Hatch) {
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
