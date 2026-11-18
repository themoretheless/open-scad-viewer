use gcode_core::{PlannedLayer, PlannedPath, emit_printer_job, printer_profile};
use slicer_core::{
    InfillPattern, LayerSection, PathRole, ShellConfig, SupportConfig, SupportPattern,
    ToolpathLayer, ToolpathSettings, plan_layers_with_shells,
};

fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Vec<[f64; 2]> {
    vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]
}

fn stack(layers: usize, h: f64, ring: impl Fn(usize) -> Vec<[f64; 2]>) -> Vec<LayerSection> {
    (0..layers)
        .map(|i| LayerSection {
            z_mm: h * (i + 1) as f64,
            contours: vec![ring(i)],
        })
        .collect()
}

fn has(layer: &ToolpathLayer, role: PathRole) -> bool {
    layer.paths.iter().any(|p| p.role == role)
}

fn count(layer: &ToolpathLayer, role: PathRole) -> usize {
    layer.paths.iter().filter(|p| p.role == role).count()
}

#[test]
fn cube_honours_perimeter_count_and_top_bottom_solid_layers() {
    let sections = stack(20, 0.2, |_| rect(10.0, 10.0, 30.0, 30.0));
    for (walls, top, bottom) in [(1, 1, 1), (3, 3, 4), (2, 5, 2)] {
        let settings = ToolpathSettings {
            wall_count: walls,
            ..ToolpathSettings::default()
        };
        let shells = ShellConfig {
            top_layers: top,
            bottom_layers: bottom,
        };
        let planned =
            plan_layers_with_shells(&sections, &settings, InfillPattern::Grid, &shells, None)
                .unwrap();
        assert_eq!(planned.len(), 20);
        for layer in &planned {
            let perimeters = count(layer, PathRole::Outline) + count(layer, PathRole::Inset);
            assert_eq!(perimeters, walls);
            assert_eq!(count(layer, PathRole::Outline), 1);
        }
        let solid: Vec<usize> = (0..20)
            .filter(|&i| has(&planned[i], PathRole::SolidSkin))
            .collect();
        let expected: Vec<usize> = (0..bottom).chain(20 - top..20).collect();
        assert_eq!(solid, expected, "walls={walls} top={top} bottom={bottom}");
        for i in bottom..20 - top {
            assert!(has(&planned[i], PathRole::Hatch));
            assert!(!has(&planned[i], PathRole::SolidSkin));
        }
        // Solid layers are fully skinned, not mixed with sparse infill.
        for &i in &expected {
            assert!(!has(&planned[i], PathRole::Hatch));
        }
    }
}

#[test]
fn default_shells_match_legacy_skin_planner() {
    let sections = stack(6, 0.2, |_| rect(0.0, 0.0, 20.0, 20.0));
    let settings = ToolpathSettings::default();
    let legacy = slicer_core::plan_layers_with_skins_and_supports(
        &sections,
        &settings,
        InfillPattern::Grid,
        None,
    )
    .unwrap();
    let shells = plan_layers_with_shells(
        &sections,
        &settings,
        InfillPattern::Grid,
        &ShellConfig::default(),
        None,
    )
    .unwrap();
    assert_eq!(legacy, shells);
}

/// T-shape: a 4 mm stem under a 20 mm bar. The bar's wings overhang at 90 degrees.
fn t_shape() -> Vec<LayerSection> {
    stack(15, 0.2, |i| {
        if i < 10 {
            rect(18.0, 10.0, 22.0, 30.0)
        } else {
            rect(10.0, 10.0, 30.0, 30.0)
        }
    })
}

fn support_bounds(layer: &ToolpathLayer) -> Option<[f64; 4]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut any = false;
    for p in layer.paths.iter().filter(|p| p.role == PathRole::Support) {
        for q in &p.points {
            any = true;
            b = [
                b[0].min(q[0]),
                b[1].min(q[1]),
                b[2].max(q[0]),
                b[3].max(q[1]),
            ];
        }
    }
    any.then_some(b)
}

#[test]
fn t_shape_overhang_gets_supports_beneath_the_wings_only() {
    let settings = ToolpathSettings::default();
    let support =
        SupportConfig::from_overhang_angle(45.0, 0.2, 2.0, SupportPattern::Lines).unwrap();
    assert!((support.overhang_offset_mm - 0.2).abs() < 1e-12);
    let planned = plan_layers_with_shells(
        &t_shape(),
        &settings,
        InfillPattern::Grid,
        &ShellConfig::default(),
        Some(&support),
    )
    .unwrap();
    for (i, layer) in planned.iter().enumerate() {
        if i < 10 {
            let b = support_bounds(layer).unwrap_or_else(|| panic!("layer {i} lacks support"));
            assert!(b[0] >= 10.0 - 1e-6 && b[2] <= 30.0 + 1e-6);
            // Support never enters the stem.
            for p in layer.paths.iter().filter(|p| p.role == PathRole::Support) {
                for q in &p.points {
                    assert!(
                        q[0] <= 18.0 + 1e-6 || q[0] >= 22.0 - 1e-6,
                        "support in stem: {q:?}"
                    );
                }
            }
            // Both wings are supported.
            assert!(b[0] < 12.0 && b[2] > 28.0);
        } else {
            assert!(
                support_bounds(layer).is_none(),
                "layer {i} has support above the overhang"
            );
        }
    }
    // The first bar layer bridges the wings.
    assert!(has(&planned[10], PathRole::Bridge));
}

#[test]
fn gentle_slope_stays_below_angle_threshold() {
    // Each layer steps out 0.1 mm: about 27 degrees from vertical at 0.2 mm layers.
    let sections = stack(12, 0.2, |i| {
        let grow = 0.1 * i as f64;
        rect(10.0 - grow, 10.0, 20.0 + grow, 20.0)
    });
    let settings = ToolpathSettings::default();
    let strict = SupportConfig::from_overhang_angle(20.0, 0.2, 2.0, SupportPattern::Lines).unwrap();
    let lenient =
        SupportConfig::from_overhang_angle(45.0, 0.2, 2.0, SupportPattern::Lines).unwrap();
    let plan = |cfg: &SupportConfig| {
        plan_layers_with_shells(
            &sections,
            &settings,
            InfillPattern::Grid,
            &ShellConfig::default(),
            Some(cfg),
        )
        .unwrap()
    };
    assert!(plan(&lenient).iter().all(|l| !has(l, PathRole::Support)));
    // A 20 degree threshold (0.073 mm per layer) flags a 0.6 mm per-layer step.
    let steep = stack(12, 0.2, |i| {
        let grow = 0.6 * i as f64;
        rect(20.0 - grow, 10.0, 30.0 + grow, 20.0)
    });
    let strict_plan = plan_layers_with_shells(
        &steep,
        &settings,
        InfillPattern::Grid,
        &ShellConfig::default(),
        Some(&strict),
    )
    .unwrap();
    assert!(strict_plan.iter().any(|l| has(l, PathRole::Support)));
    assert!(SupportConfig::from_overhang_angle(90.0, 0.2, 2.0, SupportPattern::Lines).is_err());
}

#[test]
fn grid_support_crosses_lines() {
    let settings = ToolpathSettings::default();
    let lines = SupportConfig::from_overhang_angle(45.0, 0.2, 2.0, SupportPattern::Lines).unwrap();
    let grid = SupportConfig::from_overhang_angle(45.0, 0.2, 2.0, SupportPattern::Grid).unwrap();
    let plan = |cfg: &SupportConfig| {
        plan_layers_with_shells(
            &t_shape(),
            &settings,
            InfillPattern::Grid,
            &ShellConfig::default(),
            Some(cfg),
        )
        .unwrap()
    };
    let l = plan(&lines);
    let g = plan(&grid);
    let direction = |layer: &ToolpathLayer, vertical: bool| {
        layer
            .paths
            .iter()
            .filter(|p| p.role == PathRole::Support)
            .any(|p| {
                let (a, b) = (p.points[0], p.points[p.points.len() - 1]);
                let (dx, dy) = ((b[0] - a[0]).abs(), (b[1] - a[1]).abs());
                if vertical {
                    dx < 1e-6 && dy > 1.0
                } else {
                    dy < 1e-6 && dx > 1.0
                }
            })
    };
    assert!(direction(&l[0], false) && !direction(&l[0], true));
    assert!(direction(&g[0], false) && direction(&g[0], true));
    assert!(count(&g[0], PathRole::Support) > count(&l[0], PathRole::Support));
}

#[test]
fn sliced_cube_emits_for_a_concrete_printer() {
    let sections = stack(10, 0.2, |_| rect(10.0, 10.0, 30.0, 30.0));
    let settings = ToolpathSettings::default();
    let planned = plan_layers_with_shells(
        &sections,
        &settings,
        InfillPattern::Grid,
        &ShellConfig {
            top_layers: 3,
            bottom_layers: 3,
        },
        None,
    )
    .unwrap();
    let layers: Vec<PlannedLayer> = planned
        .iter()
        .map(|l| PlannedLayer {
            z_mm: l.z_mm,
            paths: l
                .paths
                .iter()
                .map(|p| PlannedPath {
                    points: p.points.clone(),
                    closed: p.closed,
                })
                .collect(),
        })
        .collect();
    for id in ["creality-ender3-v2", "voron-2.4-350"] {
        let profile = printer_profile(id).unwrap();
        let gcode = emit_printer_job(&layers, &[], &profile).unwrap();
        assert_eq!(gcode.matches(";LAYER:").count(), 10);
    }
}
