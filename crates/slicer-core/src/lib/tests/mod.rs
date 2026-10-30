use super::*;

/// Pre-AET reference implementation: every row scans every edge.
/// Kept as the equivalence oracle for the active edge table in `hatch`.
fn hatch_naive(source: &Rings, spacing: f64, budget: &mut Budget) -> Result<Vec<Toolpath>> {
    let mut min_y = f64::INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for point in source.iter().flatten() {
        min_y = min_y.min(point[1]);
        max_y = max_y.max(point[1]);
    }
    if source.is_empty() || max_y - min_y < spacing * 0.25 {
        return Ok(Vec::new());
    }
    let start = min_y + spacing * 0.5;
    let end = max_y - spacing * 0.25;
    if start > end {
        return Ok(Vec::new());
    }
    let intervals = ((end - start) / spacing).floor();
    if !intervals.is_finite() || intervals >= MAX_HATCH_LINES as f64 {
        return Err(invalid(
            "TOOLPATH_HATCH_LIMIT",
            "Toolpath hatch exceeds 65536 scan lines",
        ));
    }
    let line_count = intervals as usize + 1;
    let mut paths = Vec::new();
    let mut previous_y = None;
    for row in 0..line_count {
        let y = spacing.mul_add(row as f64, start);
        if !y.is_finite() || previous_y.is_some_and(|previous| y <= previous) {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Hatch spacing does not advance coordinates",
            ));
        }
        previous_y = Some(y);
        if y > end {
            break;
        }
        let mut hits = Vec::new();
        for ring in source {
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                if (a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1]) {
                    let t = (y - a[1]) / (b[1] - a[1]);
                    let x = (b[0] - a[0]).mul_add(t, a[0]);
                    if !valid_coordinate(x) {
                        return Err(invalid(
                            "TOOLPATH_INVALID_GEOMETRY",
                            "Non-finite hatch intersection",
                        ));
                    }
                    hits.push((x, if b[1] > a[1] { -1_i32 } else { 1_i32 }));
                }
            }
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding = 0_i32;
        let mut span_start = 0.0;
        let mut index = 0;
        let row_start = paths.len();
        while index < hits.len() {
            let x = hits[index].0;
            let old_winding = winding;
            while index < hits.len() && hits[index].0 == x {
                winding += hits[index].1;
                index += 1;
            }
            if old_winding == 0 && winding != 0 {
                span_start = x;
            } else if old_winding != 0 && winding == 0 && x > span_start {
                budget.points(2)?;
                paths.push(Toolpath {
                    role: PathRole::Hatch,
                    points: vec![[span_start, y], [x, y]],
                    closed: false,
                });
            }
        }
        if winding != 0 {
            return Err(invalid(
                "TOOLPATH_INVALID_GEOMETRY",
                "Unbalanced hatch boundary",
            ));
        }
        if row % 2 == 1 {
            paths[row_start..].reverse();
            for path in &mut paths[row_start..] {
                path.points.reverse();
            }
        }
    }
    Ok(paths)
}

/// Deterministic xorshift64* so the equivalence test needs no rand crate.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Coarse grid values make scan lines regularly hit exact vertex
    /// heights, stressing the half-open `min_y <= y < max_y` rule.
    fn coordinate(&mut self) -> f64 {
        (self.next() % 41) as f64 * 0.25 - 5.0
    }
}

/// Star-shaped (hence simple) ring: sorted angles, random radii, but with
/// y coordinates snapped to the coarse grid for vertex-height hits.
fn random_ring(rng: &mut Rng, center: [f64; 2], vertices: usize) -> Vec<[f64; 2]> {
    let mut ring = Vec::with_capacity(vertices);
    for i in 0..vertices {
        let angle = 2.0 * std::f64::consts::PI * i as f64 / vertices as f64;
        let radius = 0.5 + (rng.next() % 20) as f64 * 0.25;
        let x = center[0] + radius * angle.cos();
        // Snap y to the grid; keep x raw so crossings are non-degenerate.
        let y = ((center[1] + radius * angle.sin()) * 4.0).round() / 4.0;
        ring.push([x, y]);
    }
    ring
}

#[test]
fn hatch_active_edge_table_matches_naive_oracle() {
    let mut rng = Rng(0x1234_5678_9ABC_DEF1);
    for case in 0..200 {
        let mut rings: Rings = Vec::new();
        let ring_count = 1 + (rng.next() % 3) as usize;
        for _ in 0..ring_count {
            let center = [rng.coordinate(), rng.coordinate()];
            let vertices = 3 + (rng.next() % 10) as usize;
            rings.push(random_ring(&mut rng, center, vertices));
        }
        // Spacings that do and do not land on the 0.25 y grid.
        for spacing in [0.25, 0.5, 0.75, 1.0, 0.3, 2.0] {
            let expected = hatch_naive(&rings, spacing, &mut Budget::default());
            let actual = hatch(&rings, spacing, &mut Budget::default());
            match (expected, actual) {
                (Ok(expected), Ok(actual)) => assert_eq!(
                    actual, expected,
                    "case {case} spacing {spacing}: AET hatch diverged from naive scan"
                ),
                (expected, actual) => assert_eq!(
                    actual.is_err(),
                    expected.is_err(),
                    "case {case} spacing {spacing}: error mismatch"
                ),
            }
        }
    }
}

#[test]
fn hatch_handles_hole_and_touching_rows() {
    // Square with a square hole, spacing lands exactly on vertex heights.
    let rings: Rings = vec![
        vec![[0., 0.], [4., 0.], [4., 4.], [0., 4.]],
        vec![[1., 1.], [1., 3.], [3., 3.], [3., 1.]],
    ];
    let expected = hatch_naive(&rings, 1.0, &mut Budget::default()).unwrap();
    let actual = hatch(&rings, 1.0, &mut Budget::default()).unwrap();
    assert_eq!(actual, expected);
    assert!(!actual.is_empty());
    // Degenerate inputs keep working: empty rings and flat rings.
    assert!(hatch(&Rings::new(), 1.0, &mut Budget::default())
        .unwrap()
        .is_empty());
    let flat: Rings = vec![vec![[0., 1.], [2., 1.], [1., 1.5]]];
    assert_eq!(
        hatch(&flat, 1.0, &mut Budget::default()).unwrap(),
        hatch_naive(&flat, 1.0, &mut Budget::default()).unwrap()
    );
}

#[test]
fn infill_patterns_and_solid_skin_generate_valid_toolpaths() {
    let section = LayerSection {
        z_mm: 0.2,
        contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
    };
    let settings = ToolpathSettings::default();
    let rect = plan_layer_with_pattern(&section, &settings, InfillPattern::Rectilinear, false)
        .unwrap();
    let grid = plan_layer_with_pattern(&section, &settings, InfillPattern::Grid, false).unwrap();
    let tri =
        plan_layer_with_pattern(&section, &settings, InfillPattern::Triangles, false).unwrap();
    let conc =
        plan_layer_with_pattern(&section, &settings, InfillPattern::Concentric, false).unwrap();
    let hex =
        plan_layer_with_pattern(&section, &settings, InfillPattern::Honeycomb, false).unwrap();
    let gyroid =
        plan_layer_with_pattern(&section, &settings, InfillPattern::Gyroid, false).unwrap();
    let hilbert =
        plan_layer_with_pattern(&section, &settings, InfillPattern::HilbertCurve, false)
            .unwrap();
    let arch =
        plan_layer_with_pattern(&section, &settings, InfillPattern::ArchimedeanChords, false)
            .unwrap();
    let skin =
        plan_layer_with_pattern(&section, &settings, InfillPattern::Rectilinear, true).unwrap();

    assert!(grid.paths.len() > rect.paths.len());
    assert!(tri.paths.len() > grid.paths.len());
    assert!(!hex.paths.is_empty());
    assert!(!gyroid.paths.is_empty());
    assert!(!hilbert.paths.is_empty());
    assert!(!arch.paths.is_empty());
    let gyroid_lines_z0 = generate_gyroid_infill_lines(([0.0, 0.0], [20.0, 20.0]), 0.2, 2.0);
    let gyroid_lines_z1 = generate_gyroid_infill_lines(([0.0, 0.0], [20.0, 20.0]), 0.8, 2.0);
    assert!(!gyroid_lines_z0.is_empty());
    assert_ne!(gyroid_lines_z0, gyroid_lines_z1);
    assert!(conc
        .paths
        .iter()
        .filter(|p| p.role == PathRole::Hatch)
        .all(|p| p.closed));
    assert!(skin
        .paths
        .iter()
        .any(|p| p.role == PathRole::SolidSkin && !p.closed));
}

#[test]
fn adaptive_heights_and_skin_support_detection_work() {
    let cfg = VariableLayerConfig::default();
    // Steep vertical wall (|n_z| = 0) -> max_layer_height_mm, shallow dome (|n_z| -> 1) -> min_layer_height_mm
    let heights = schedule_adaptive_heights(0.0, 2.0, &cfg, |z| if z < 1.0 { 0.0 } else { 0.95 })
        .unwrap();
    assert!(heights.len() >= 8);
    let step_steep = heights[1] - heights[0];
    let step_shallow = heights[heights.len() - 1] - heights[heights.len() - 2];
    assert!(step_steep > step_shallow);

    // Two-layer overhanging L-shape: layer 0 is [0..10]x[0..10], layer 1 is [0..20]x[0..10]
    let sections = vec![
        LayerSection {
            z_mm: 0.2,
            contours: vec![vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.]]],
        },
        LayerSection {
            z_mm: 0.4,
            contours: vec![vec![[0., 0.], [20., 0.], [20., 10.], [0., 10.]]],
        },
    ];
    let settings = ToolpathSettings::default();
    let support_cfg = SupportConfig {
        overhang_offset_mm: 1.0,
        support_spacing_mm: 2.0,
    };
    let planned = plan_layers_with_skins_and_supports(
        &sections,
        &settings,
        InfillPattern::Grid,
        Some(&support_cfg),
    )
    .unwrap();
    assert_eq!(planned.len(), 2);
    // Layer 0 should contain Support paths under the [10..20]x[0..10] overhang of layer 1
    assert!(planned[0].paths.iter().any(|p| p.role == PathRole::Support));
    // Layer 1 has an unsupported bridge region [10..20]x[0..10]
    assert!(planned[1].paths.iter().any(|p| p.role == PathRole::Bridge));
}

#[test]
fn crosshatch_trihexagon_cubic_and_brim_generation_work() {
    let sec_z0 = LayerSection {
        z_mm: 0.2,
        contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
    };
    let sec_z1 = LayerSection {
        z_mm: 1.4,
        contours: vec![vec![[0., 0.], [20., 0.], [20., 20.], [0., 20.]]],
    };
    let settings = ToolpathSettings::default();
    let ch =
        plan_layer_with_pattern(&sec_z0, &settings, InfillPattern::CrossHatch, false).unwrap();
    let th =
        plan_layer_with_pattern(&sec_z0, &settings, InfillPattern::TriHexagon, false).unwrap();
    let cub0 = plan_layer_with_pattern(
        &sec_z0,
        &settings,
        InfillPattern::CubicSubdivision,
        false,
    )
    .unwrap();
    let cub1 = plan_layer_with_pattern(
        &sec_z1,
        &settings,
        InfillPattern::CubicSubdivision,
        false,
    )
    .unwrap();
    assert!(!ch.paths.is_empty());
    assert!(!th.paths.is_empty());
    assert!(!cub0.paths.is_empty());
    // CubicSubdivision shifts phase with Z so layer Z=0.2 and Z=1.4 have distinct infill coordinates
    assert_ne!(cub0.paths, cub1.paths);

    let brim = generate_brim(&sec_z0.contours, 2.0, 0.5).unwrap();
    assert_eq!(brim.len(), 4);
    // Outermost brim loop extends outside [-0.2..20.2]
    assert!(brim.last().unwrap().iter().any(|p| p[0] > 21.0));

    let skirt = generate_skirt(&sec_z0.contours, 3.0, 2, 0.5).unwrap();
    assert_eq!(skirt.len(), 2);
    // First skirt loop starts 3.0 mm outside [0..20] -> beyond 23.0 mm
    assert!(skirt[0].iter().any(|p| p[0] >= 22.9));
    // Second skirt loop is spaced further out
    assert!(skirt[1].iter().any(|p| p[0] >= 23.4));

    // Tree support trunk coalescing from 4 overhang tips at z=20.0 down to bed_z=0.0
    let tips = vec![
        [10.0, 10.0, 20.0],
        [12.0, 10.0, 20.0],
        [30.0, 30.0, 20.0],
        [31.5, 30.0, 20.0],
    ];
    let trunks = plan_tree_support_trunks(&tips, 0.0, 45.0, 5.0);
    assert!(!trunks.is_empty());
    // Nearby tips ([10,10] & [12,10]) and ([30,30] & [31.5,30]) coalesce so fewer trunks hit bed_z=0.0 than 4
    let bed_contacts = trunks
        .iter()
        .filter(|(_, end, _)| (end[2] - 0.0).abs() < 1e-6)
        .count();
    assert!(bed_contacts >= 1 && bed_contacts <= 2, "bed_contacts={bed_contacts}");
    // Trunk radii increase toward the bed
    let max_radius = trunks.iter().map(|(_, _, r)| *r).fold(0.0_f64, f64::max);
    let min_radius = trunks.iter().map(|(_, _, r)| *r).fold(f64::INFINITY, f64::min);
    assert!(max_radius > min_radius);

    // Two-layer raft generation with orthogonal base + denser interface
    let raft_cfg = RaftConfig::default();
    let (raft_base, raft_interface) = generate_raft(
        &sec_z0.contours,
        raft_cfg.margin_mm,
        raft_cfg.base_spacing_mm,
        raft_cfg.interface_spacing_mm,
        raft_cfg.base_z_mm,
        raft_cfg.interface_z_mm,
    )
    .unwrap();
    assert!(!raft_base.is_empty());
    assert!(raft_interface.len() > raft_base.len() * 2);
    assert!(raft_base.iter().all(|s| (s[0][2] - raft_cfg.base_z_mm).abs() < 1e-9));
    assert!(raft_interface.iter().all(|s| (s[0][2] - raft_cfg.interface_z_mm).abs() < 1e-9));

    // Fuzzy skin outer contour subdivision and deterministic normal jitter
    let f_cfg = FuzzySkinConfig::default();
    let fuzzy = apply_fuzzy_skin_to_contour(
        &sec_z0.contours[0],
        f_cfg.thickness_mm,
        f_cfg.point_dist_mm,
        f_cfg.seed,
    );
    // 80 mm perimeter at 0.8 mm point distance -> ~100 subdivided vertices
    assert!(fuzzy.len() >= 80, "expected >= 80 vertices, got {}", fuzzy.len());
    // Deterministic: calling with same seed yields identical contour
    let fuzzy_same = apply_fuzzy_skin_to_contour(
        &sec_z0.contours[0],
        f_cfg.thickness_mm,
        f_cfg.point_dist_mm,
        f_cfg.seed,
    );
    assert_eq!(fuzzy, fuzzy_same);
    // Outward displacement: some vertices lie strictly outside [0..20]x[0..20] up to thickness_mm
    assert!(fuzzy.iter().any(|p| p[0] > 20.05 || p[1] > 20.05 || p[0] < -0.05 || p[1] < -0.05));
    assert!(fuzzy.iter().all(|p| p[0] >= -f_cfg.thickness_mm - 1e-6 && p[0] <= 20.0 + f_cfg.thickness_mm + 1e-6));
}
