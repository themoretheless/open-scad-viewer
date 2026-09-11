use mechanics_core::{analyze_layers, analyze_section, LayerSection, LoadCase};

fn rect(z: f64, min: [f64; 2], max: [f64; 2]) -> LayerSection {
    LayerSection {
        z_mm: z,
        contours: vec![vec![
            min,
            [max[0], min[1]],
            max,
            [min[0], max[1]],
        ]],
    }
}

fn load(mx: f64, my: f64) -> LoadCase {
    LoadCase {
        moment_x_nmm: mx,
        moment_y_nmm: my,
        axial_n: 0.0,
        allowable_mpa: 30.0,
    }
}

#[test]
fn rectangle_matches_beam_formulas() {
    let section = rect(0.1, [0.0, 0.0], [20.0, 10.0]);
    let report = analyze_section(&section, &load(0.0, 0.0)).unwrap();
    assert!((report.area_mm2 - 200.0).abs() < 1e-9);
    assert!((report.centroid[0] - 10.0).abs() < 1e-9);
    assert!((report.centroid[1] - 5.0).abs() < 1e-9);
    assert!((report.ixx_mm4 - 20.0 * 10.0_f64.powi(3) / 12.0).abs() < 1e-6);
    assert!((report.iyy_mm4 - 10.0 * 20.0_f64.powi(3) / 12.0).abs() < 1e-6);
    assert!(report.min_wall_mm > 9.0);
}

#[test]
fn hole_reduces_area_and_is_kept_out_of_wall_probe() {
    let section = LayerSection {
        z_mm: 0.1,
        contours: vec![
            vec![[0.0, 0.0], [20.0, 0.0], [20.0, 20.0], [0.0, 20.0]],
            vec![[8.0, 8.0], [8.0, 12.0], [12.0, 12.0], [12.0, 8.0]],
        ],
    };
    let report = analyze_section(&section, &load(0.0, 0.0)).unwrap();
    assert!((report.area_mm2 - 384.0).abs() < 1e-9);
    assert!(report.min_wall_mm > 3.5);
    assert!(report.min_wall_mm < 8.5);
}

#[test]
fn bending_stress_is_mc_over_i() {
    let section = rect(0.0, [-10.0, -5.0], [10.0, 5.0]);
    let report = analyze_section(&section, &load(20.0 * 10.0_f64.powi(3) / 12.0 / 5.0, 0.0)).unwrap();
    assert!((report.bending_mpa - 1.0).abs() < 1e-6);
    assert!((report.factor_of_safety - 30.0).abs() < 1e-6);
}

#[test]
fn weakest_layer_is_the_smaller_section() {
    let layers = [
        rect(0.0, [0.0, 0.0], [20.0, 10.0]),
        rect(1.0, [0.0, 0.0], [8.0, 4.0]),
    ];
    let report = analyze_layers(&layers, &load(100.0, 0.0)).unwrap();
    assert_eq!(report.weakest_index, 1);
    assert!(report.layers[1].factor_of_safety < report.layers[0].factor_of_safety);
}

#[test]
fn empty_and_bad_allowable_fail_closed() {
    assert_eq!(
        analyze_section(
            &LayerSection {
                z_mm: 0.0,
                contours: vec![],
            },
            &load(0.0, 0.0)
        )
        .unwrap_err()
        .code,
        "MECHANICS_EMPTY_SECTION"
    );
    let mut bad = load(0.0, 0.0);
    bad.allowable_mpa = 0.0;
    assert_eq!(
        analyze_section(&rect(0.0, [0.0, 0.0], [2.0, 2.0]), &bad)
            .unwrap_err()
            .code,
        "MECHANICS_INVALID_MATERIAL"
    );
}
