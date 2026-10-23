//! Adaptive layer height scheduling, skin/bridge/support region planning,
//! bed adhesion (brim, skirt, raft), tree supports, and fuzzy skin.

use planar_geometry::{
    LayerSection, MAX_LAYERS,
    path::BezierPath,
    rings::{self, Rings},
    stroke::{self, StrokeOptions},
};
use crate::{
    Budget, InfillPattern, PathRole, Result, Toolpath, ToolpathLayer, ToolpathSettings,
    infill::{generate_cubic_infill, generate_infill, hatch_angled},
    inset_rings, invalid, path_from_ring, point_count, require_section, require_settings,
    valid_coordinate,
};

/// Configuration for curvature / surface-normal adaptive layer thickness.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VariableLayerConfig {
    pub min_layer_height_mm: f64,
    pub max_layer_height_mm: f64,
}

impl Default for VariableLayerConfig {
    fn default() -> Self {
        Self {
            min_layer_height_mm: 0.08,
            max_layer_height_mm: 0.28,
        }
    }
}

/// Computes an adaptive sequence of Z sample heights in `[z_min, z_max)` driven by
/// `max_abs_normal_z(z)` ($\max |n_z| \in [0, 1]$ on the slice contour):
/// vertical walls ($|n_z| \approx 0$) use `max_layer_height_mm`, while shallow / horizontal
/// slopes ($|n_z| \to 1$) smoothly step down to `min_layer_height_mm`.
pub fn schedule_adaptive_heights<F>(
    z_min: f64,
    z_max: f64,
    config: &VariableLayerConfig,
    mut max_abs_normal_z: F,
) -> Result<Vec<f64>>
where
    F: FnMut(f64) -> f64,
{
    if !valid_coordinate(z_min)
        || !valid_coordinate(z_max)
        || z_max <= z_min
        || !config.min_layer_height_mm.is_finite()
        || !config.max_layer_height_mm.is_finite()
        || config.min_layer_height_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || config.max_layer_height_mm < config.min_layer_height_mm
    {
        return Err(invalid(
            "TOOLPATH_INVALID_RANGE",
            "Invalid adaptive layer height range or settings",
        ));
    }
    let mut heights = Vec::with_capacity(
        (((z_max - z_min) / config.min_layer_height_mm).ceil() as usize + 1).min(MAX_LAYERS),
    );
    let mut z = z_min;
    for _ in 0..MAX_LAYERS {
        if z >= z_max {
            break;
        }
        heights.push(z);
        let nz = max_abs_normal_z(z).abs().clamp(0.0, 1.0);
        let dz = config.max_layer_height_mm
            - nz * (config.max_layer_height_mm - config.min_layer_height_mm);
        let next_z = z + dz.max(config.min_layer_height_mm);
        if !next_z.is_finite() || next_z <= z {
            return Err(invalid(
                "TOOLPATH_INVALID_RANGE",
                "Adaptive layer height did not advance Z",
            ));
        }
        z = next_z;
    }
    if z < z_max && heights.len() >= MAX_LAYERS {
        return Err(invalid(
            "TOOLPATH_LAYER_LIMIT",
            "Adaptive layer schedule exceeded 2048 layers",
        ));
    }
    Ok(heights)
}

/// Configuration for automatic overhang and support toolpath generation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportConfig {
    /// Horizontal expansion threshold beyond which upper-layer material is considered an overhang.
    pub overhang_offset_mm: f64,
    /// Line spacing for support hatching.
    pub support_spacing_mm: f64,
}

impl Default for SupportConfig {
    fn default() -> Self {
        Self {
            overhang_offset_mm: 0.4,
            support_spacing_mm: 2.5,
        }
    }
}

/// Plans a stack of layers with automatic Top/Bottom Solid Skin detection ($Z_i \setminus Z_{i\pm 1}$),
/// Bridge infill on unsupported regions ($Z_i \setminus Z_{i-1}$), and optional Support hatching
/// projected down from upper overhangs onto lower layers.
pub fn plan_layers_with_skins_and_supports(
    sections: &[LayerSection],
    settings: &ToolpathSettings,
    pattern: InfillPattern,
    support: Option<&SupportConfig>,
) -> Result<Vec<ToolpathLayer>> {
    require_settings(settings)?;
    if sections.len() > MAX_LAYERS {
        return Err(invalid(
            "TOOLPATH_LAYER_LIMIT",
            "Toolpath plan exceeded 2048 layers",
        ));
    }
    let mut budget = Budget::default();
    let mut normalized = Vec::with_capacity(sections.len());
    for sec in sections {
        require_section(sec, &mut budget)?;
        budget.arrangement(point_count(&sec.contours))?;
        let norm = rings::nonzero(&sec.contours)?;
        budget.points(point_count(&norm))?;
        normalized.push(norm);
    }

    // Downward projection of overhanging regions for support generation
    let mut support_regions: Vec<Rings> = vec![Vec::new(); sections.len()];
    if let Some(sup) = support {
        if !sup.overhang_offset_mm.is_finite()
            || sup.overhang_offset_mm < 0.0
            || !sup.support_spacing_mm.is_finite()
            || sup.support_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Invalid support configuration",
            ));
        }
        let mut accumulated_overhang: Rings = Vec::new();
        for i in (0..sections.len()).rev() {
            if !accumulated_overhang.is_empty() {
                let free_support =
                    rings::planar(&accumulated_overhang, &normalized[i], "difference")?;
                support_regions[i] = free_support.clone();
                accumulated_overhang = free_support;
            }
            if i > 0 && !normalized[i].is_empty() {
                let unsupported = rings::planar(&normalized[i], &normalized[i - 1], "difference")?;
                let significant = if sup.overhang_offset_mm > 1e-6 {
                    inset_rings(&unsupported, sup.overhang_offset_mm * 0.5, &mut budget)?
                } else {
                    unsupported
                };
                if !significant.is_empty() {
                    accumulated_overhang = if accumulated_overhang.is_empty() {
                        significant
                    } else {
                        rings::planar(&accumulated_overhang, &significant, "union")?
                    };
                }
            }
        }
    }

    let mut out_layers = Vec::with_capacity(sections.len());
    for i in 0..sections.len() {
        let source = &normalized[i];
        let mut paths = Vec::with_capacity(settings.wall_count * source.len() + 8);
        for wall in 0..settings.wall_count {
            let inset = (wall as f64 + 0.5) * settings.line_width_mm;
            let offset = inset_rings(source, inset, &mut budget)?;
            let role = if wall == 0 {
                PathRole::Outline
            } else {
                PathRole::Inset
            };
            for ring in &offset {
                budget.points(ring.len())?;
                paths.push(path_from_ring(role, ring, true));
            }
            if offset.is_empty() {
                break;
            }
        }
        let interior = inset_rings(
            source,
            settings.wall_count as f64 * settings.line_width_mm,
            &mut budget,
        )?;
        if !interior.is_empty() {
            let bridge_region = if i > 0 && !normalized[i - 1].is_empty() {
                rings::planar(&interior, &normalized[i - 1], "difference")?
            } else {
                Vec::new()
            };
            if !bridge_region.is_empty() {
                paths.extend(hatch_angled(
                    &bridge_region,
                    settings.line_width_mm,
                    0.0,
                    PathRole::Bridge,
                    &mut budget,
                )?);
            }
            let supported_interior = if bridge_region.is_empty() {
                interior
            } else {
                rings::planar(&interior, &bridge_region, "difference")?
            };
            if !supported_interior.is_empty() {
                let is_bottom = i == 0 || normalized[i - 1].is_empty();
                let is_top = i + 1 == sections.len() || normalized[i + 1].is_empty();
                if is_bottom || is_top {
                    paths.extend(hatch_angled(
                        &supported_interior,
                        settings.line_width_mm,
                        std::f64::consts::FRAC_PI_4,
                        PathRole::SolidSkin,
                        &mut budget,
                    )?);
                } else {
                    let top_skin_region =
                        rings::planar(&supported_interior, &normalized[i + 1], "difference")?;
                    let sparse_region =
                        rings::planar(&supported_interior, &top_skin_region, "difference")?;
                    if !top_skin_region.is_empty() {
                        paths.extend(hatch_angled(
                            &top_skin_region,
                            settings.line_width_mm,
                            std::f64::consts::FRAC_PI_4,
                            PathRole::SolidSkin,
                            &mut budget,
                        )?);
                    }
                    if !sparse_region.is_empty() {
                        if pattern == InfillPattern::CubicSubdivision {
                            paths.extend(generate_cubic_infill(
                                &sparse_region,
                                settings.infill_spacing_mm,
                                sections[i].z_mm,
                                PathRole::Hatch,
                                &mut budget,
                            )?);
                        } else {
                            paths.extend(generate_infill(
                                &sparse_region,
                                settings.infill_spacing_mm,
                                pattern,
                                PathRole::Hatch,
                                &mut budget,
                            )?);
                        }
                    }
                }
            }
        }
        if let Some(sup) = support
            && !support_regions[i].is_empty()
        {
            paths.extend(hatch_angled(
                &support_regions[i],
                sup.support_spacing_mm,
                0.0,
                PathRole::Support,
                &mut budget,
            )?);
        }
        out_layers.push(ToolpathLayer {
            z_mm: sections[i].z_mm,
            paths,
        });
    }
    Ok(out_layers)
}

/// Closed 2D contour loop (`Vec<[f64; 2]>`) in millimeter coordinates.
pub type Contour = Vec<[f64; 2]>;

/// Bed-adhesion brim configuration for the first print layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrimConfig {
    pub width_mm: f64,
    pub line_spacing_mm: f64,
}

impl Default for BrimConfig {
    fn default() -> Self {
        Self {
            width_mm: 4.0,
            line_spacing_mm: 0.4,
        }
    }
}

/// Generates concentric outward-offset bed-adhesion brim loops around `outer_loops`
/// spanning `width_mm` at pitch `line_spacing_mm`.
pub fn generate_brim(
    outer_loops: &[Contour],
    width_mm: f64,
    line_spacing_mm: f64,
) -> Result<Vec<Contour>> {
    if !width_mm.is_finite()
        || width_mm <= 0.0
        || !line_spacing_mm.is_finite()
        || line_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Brim width_mm and line_spacing_mm must be finite and positive",
        ));
    }
    if outer_loops.is_empty() {
        return Ok(Vec::new());
    }
    let mut budget = Budget::default();
    let norm = rings::nonzero(&outer_loops.to_vec())?;
    let ring_count = ((width_mm / line_spacing_mm).round() as usize).clamp(1, 64);
    let mut brim_loops = Vec::with_capacity(ring_count * norm.len());
    for step in 1..=ring_count {
        let distance = (step as f64 - 0.5) * line_spacing_mm;
        let expanded = outset_rings(&norm, distance, &mut budget)?;
        for ring in expanded {
            let mut area2 = 0.0;
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                area2 += a[0] * b[1] - b[0] * a[1];
            }
            if area2 > 0.0 {
                brim_loops.push(ring);
            }
        }
    }
    Ok(brim_loops)
}

fn outset_rings(source: &Rings, distance: f64, budget: &mut Budget) -> Result<Rings> {
    if source.is_empty() {
        return Ok(Vec::new());
    }
    let mut bands = Vec::with_capacity(source.len());
    let tolerance = (distance * 0.01).min(0.01);
    let options = StrokeOptions {
        width: 2.0 * distance,
        ..Default::default()
    };
    for ring in source {
        budget.arrangement(ring.len().saturating_mul(6))?;
        let path = BezierPath::from_polyline(ring, true)?;
        for contour in stroke::outline_stroke_tol(&path, &options, tolerance)? {
            let band = contour.to_ring(tolerance)?;
            budget.points(band.len())?;
            bands.push(band);
        }
    }
    budget.arrangement(point_count(source).saturating_add(point_count(&bands)))?;
    let result = rings::planar(source, &bands, "union")?;
    budget.points(point_count(&result))?;
    Ok(result)
}

/// Bed-priming skirt configuration around the first print layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkirtConfig {
    pub offset_mm: f64,
    pub loops: usize,
    pub line_spacing_mm: f64,
}

impl Default for SkirtConfig {
    fn default() -> Self {
        Self {
            offset_mm: 3.0,
            loops: 2,
            line_spacing_mm: 0.4,
        }
    }
}

/// Generates concentric bed-priming skirt loops around the outer contour offset of the first layer,
/// starting at `offset_mm` from `outer_loops` and stepping outward `loops` times by `line_spacing_mm`.
pub fn generate_skirt(
    outer_loops: &[Contour],
    offset_mm: f64,
    loops: usize,
    line_spacing_mm: f64,
) -> Result<Vec<Contour>> {
    if !offset_mm.is_finite()
        || offset_mm < 0.0
        || !line_spacing_mm.is_finite()
        || line_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || loops > 64
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Skirt offset_mm, line_spacing_mm, and loops (<= 64) must be valid",
        ));
    }
    if outer_loops.is_empty() || loops == 0 {
        return Ok(Vec::new());
    }
    let mut budget = Budget::default();
    let norm = rings::nonzero(&outer_loops.to_vec())?;
    let mut skirt_loops = Vec::with_capacity(loops * norm.len());
    for step in 0..loops {
        let distance = offset_mm + (step as f64) * line_spacing_mm;
        if distance > 1e-6 {
            let expanded = outset_rings(&norm, distance, &mut budget)?;
            for ring in expanded {
                let mut area2 = 0.0;
                for i in 0..ring.len() {
                    let a = ring[i];
                    let b = ring[(i + 1) % ring.len()];
                    area2 += a[0] * b[1] - b[0] * a[1];
                }
                if area2 > 0.0 {
                    skirt_loops.push(ring);
                }
            }
        } else {
            for ring in &norm {
                let mut area2 = 0.0;
                for i in 0..ring.len() {
                    let a = ring[i];
                    let b = ring[(i + 1) % ring.len()];
                    area2 += a[0] * b[1] - b[0] * a[1];
                }
                if area2 > 0.0 {
                    skirt_loops.push(ring.clone());
                }
            }
        }
    }
    Ok(skirt_loops)
}

/// A node in a hierarchical organic/tree support structure.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeSupportNode {
    pub position: [f64; 3],
    pub radius_mm: f64,
    pub parent_index: Option<usize>,
}

/// Plans hierarchical organic/tree support trunk segments `(start_xyz, end_xyz, radius_mm)` coalescing
/// downward from `overhang_points` to `bed_z` within `branch_angle_deg` and horizontal `merge_distance_mm`.
pub fn plan_tree_support_trunks(
    overhang_points: &[[f64; 3]],
    bed_z: f64,
    branch_angle_deg: f64,
    merge_distance_mm: f64,
) -> Vec<([f64; 3], [f64; 3], f64)> {
    if overhang_points.is_empty()
        || !bed_z.is_finite()
        || !branch_angle_deg.is_finite()
        || !merge_distance_mm.is_finite()
        || merge_distance_mm <= 0.0
    {
        return Vec::new();
    }
    let tan_angle = branch_angle_deg.clamp(5.0, 80.0).to_radians().tan();
    let mut active: Vec<([f64; 3], f64)> = Vec::with_capacity(overhang_points.len());
    for &p in overhang_points {
        if p[0].is_finite() && p[1].is_finite() && p[2].is_finite() && p[2] > bed_z + 1e-6 {
            active.push((p, 0.8_f64));
        }
    }
    if active.is_empty() {
        return Vec::new();
    }
    // Sort highest Z first, then deterministically by X, Y
    active.sort_by(|a, b| {
        b.0[2]
            .total_cmp(&a.0[2])
            .then_with(|| a.0[0].total_cmp(&b.0[0]))
            .then_with(|| a.0[1].total_cmp(&b.0[1]))
    });

    let max_z = active[0].0[2];
    let step_z = (merge_distance_mm / (2.0 * tan_angle)).clamp(0.5, ((max_z - bed_z) * 0.5).max(0.5));
    let z_steps_est = (((max_z - bed_z) / step_z).ceil() as usize).max(1);
    let mut segments = Vec::with_capacity(active.len().saturating_mul(z_steps_est));
    let mut next_active = Vec::with_capacity(active.len());
    let mut used = Vec::with_capacity(active.len());
    let mut current_z = max_z;

    while current_z > bed_z + 1e-6 && !active.is_empty() {
        let next_z = (current_z - step_z).max(bed_z);
        let dz = current_z - next_z;
        let max_horiz_shift = dz * tan_angle;
        let effective_merge = merge_distance_mm.min(2.0 * max_horiz_shift);

        used.clear();
        used.resize(active.len(), false);
        next_active.clear();

        for i in 0..active.len() {
            if used[i] {
                continue;
            }
            let (pi, ri) = active[i];
            if pi[2] < current_z - 1e-6 {
                next_active.push((pi, ri));
                used[i] = true;
                continue;
            }
            // Look for nearest unmerged neighbor at this Z level within effective_merge
            let mut best_j = None;
            let mut best_dist = effective_merge;
            for j in (i + 1)..active.len() {
                if used[j] || active[j].0[2] < current_z - 1e-6 {
                    continue;
                }
                let pj = active[j].0;
                let dist = (pi[0] - pj[0]).hypot(pi[1] - pj[1]);
                if dist <= best_dist {
                    best_dist = dist;
                    best_j = Some(j);
                }
            }
            if let Some(j) = best_j {
                used[i] = true;
                used[j] = true;
                let (pj, rj) = active[j];
                let mid = [0.5 * (pi[0] + pj[0]), 0.5 * (pi[1] + pj[1]), next_z];
                let merged_r = (ri.hypot(rj) + 0.05 * dz).min(6.0);
                segments.push((pi, mid, ri));
                segments.push((pj, mid, rj));
                next_active.push((mid, merged_r));
            } else {
                used[i] = true;
                let down = [pi[0], pi[1], next_z];
                let next_r = (ri + 0.05 * dz).min(6.0);
                segments.push((pi, down, ri));
                next_active.push((down, next_r));
            }
        }
        std::mem::swap(&mut active, &mut next_active);
        current_z = next_z;
    }
    segments
}

/// Bed-adhesion raft configuration with coarse base layer and fine orthogonal interface layer.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RaftConfig {
    pub margin_mm: f64,
    pub base_spacing_mm: f64,
    pub interface_spacing_mm: f64,
    pub base_z_mm: f64,
    pub interface_z_mm: f64,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            margin_mm: 5.0,
            base_spacing_mm: 2.0,
            interface_spacing_mm: 0.6,
            base_z_mm: 0.3,
            interface_z_mm: 0.5,
        }
    }
}

/// Generates a two-layer orthogonal adhesion raft beneath `outer_loops` expanded by `margin_mm`.
/// Returns `(base_segments_3d, interface_segments_3d)`, where `base_segments_3d` are horizontal
/// ($0^\circ$) hatch segments at `base_z` with pitch `base_spacing_mm`, and `interface_segments_3d`
/// are orthogonal ($90^\circ$) hatch segments at `interface_z` with pitch `interface_spacing_mm`.
pub fn generate_raft(
    outer_loops: &[Contour],
    margin_mm: f64,
    base_spacing_mm: f64,
    interface_spacing_mm: f64,
    base_z: f64,
    interface_z: f64,
) -> Result<(Vec<[[f64; 3]; 2]>, Vec<[[f64; 3]; 2]>)> {
    if !margin_mm.is_finite()
        || margin_mm < 0.0
        || !base_spacing_mm.is_finite()
        || base_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || !interface_spacing_mm.is_finite()
        || interface_spacing_mm < gcode_core::COORDINATE_RESOLUTION_MM
        || !valid_coordinate(base_z)
        || !valid_coordinate(interface_z)
        || interface_z <= base_z
    {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Raft margin, spacings, and Z heights (interface_z > base_z) must be valid",
        ));
    }
    if outer_loops.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut budget = Budget::default();
    let norm = rings::nonzero(&outer_loops.to_vec())?;
    let footprint = if margin_mm > 1e-6 {
        outset_rings(&norm, margin_mm, &mut budget)?
    } else {
        norm
    };
    let base_paths = hatch_angled(&footprint, base_spacing_mm, 0.0, PathRole::Support, &mut budget)?;
    let interface_paths = hatch_angled(
        &footprint,
        interface_spacing_mm,
        std::f64::consts::FRAC_PI_2,
        PathRole::Support,
        &mut budget,
    )?;
    let to_3d = |paths: Vec<Toolpath>, z: f64| -> Vec<[[f64; 3]; 2]> {
        let mut segs = Vec::with_capacity(paths.len());
        for p in paths {
            if let (Some(&a), Some(&b)) = (p.points.first(), p.points.last())
                && p.points.len() >= 2
            {
                segs.push([[a[0], a[1], z], [b[0], b[1], z]]);
            }
        }
        segs
    };
    Ok((to_3d(base_paths, base_z), to_3d(interface_paths, interface_z)))
}

/// Configuration for textured "Fuzzy Skin" outer perimeter wall displacement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FuzzySkinConfig {
    /// Maximum outward normal jitter amplitude in millimeters (typical `0.3` mm).
    pub thickness_mm: f64,
    /// Target subdivision spacing along outer contour segments in millimeters (typical `0.8` mm).
    pub point_dist_mm: f64,
    /// Deterministic PRNG seed for reproducible surface texture.
    pub seed: u64,
}

impl Default for FuzzySkinConfig {
    fn default() -> Self {
        Self {
            thickness_mm: 0.3,
            point_dist_mm: 0.8,
            seed: 0x517C_C1B7_2722_0A95,
        }
    }
}

/// Subdivides a closed outer contour at `point_dist_mm` intervals and displaces vertices
/// outward along the 2D segment normal by deterministic pseudo-random offsets in `[0, thickness_mm]`
/// to produce a matte / textured 3D-printed exterior ("Fuzzy Skin").
pub fn apply_fuzzy_skin_to_contour(
    contour: &Contour,
    thickness_mm: f64,
    point_dist_mm: f64,
    seed: u64,
) -> Contour {
    if contour.len() < 3
        || !thickness_mm.is_finite()
        || thickness_mm <= 0.0
        || !point_dist_mm.is_finite()
        || point_dist_mm <= 1e-4
    {
        return contour.clone();
    }
    let n = contour.len();
    // Determine contour signed area so outward normal points away from interior regardless of winding
    let mut area2 = 0.0;
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        area2 += a[0] * b[1] - b[0] * a[1];
    }
    // For CCW contour (area2 > 0), outward unit normal of (dx, dy) is (dy, -dx) / len
    let out_sign = if area2 >= 0.0 { 1.0 } else { -1.0 };

    let mut state = seed ^ 0x9E37_79B9_7F4A_7C15;
    let mut next_u01 = || -> f64 {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^= z >> 31;
        ((z >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    };

    let mut out = Vec::with_capacity(n * 4);
    for i in 0..n {
        let a = contour[i];
        let b = contour[(i + 1) % n];
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let seg_len = dx.hypot(dy);
        if seg_len <= 1e-9 {
            continue;
        }
        let nx = out_sign * (dy / seg_len);
        let ny = out_sign * (-dx / seg_len);
        let steps = ((seg_len / point_dist_mm).round() as usize).clamp(1, 2048);
        let inv_steps = 1.0 / steps as f64;
        for s in 0..steps {
            let t = (s as f64) * inv_steps;
            let disp = next_u01() * thickness_mm;
            out.push([
                a[0] + t * dx + disp * nx,
                a[1] + t * dy + disp * ny,
            ]);
        }
    }
    out
}



