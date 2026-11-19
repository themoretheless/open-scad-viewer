//! Adaptive layer height scheduling, skin/bridge/support region planning,
//! bed adhesion (brim, skirt, raft), tree supports, and fuzzy skin.

use planar_geometry::{
    LayerSection, MAX_LAYERS,
    path::BezierPath,
    rings::{self, Rings},
    stroke::{self, StrokeOptions},
};
use crate::{
    Budget, InfillPattern, PathRole, Result, ToolpathLayer, ToolpathSettings,
    infill::{generate_cubic_infill, generate_infill, hatch_angled},
    inset_rings, invalid, path_from_ring, point_count, require_section, require_settings,
    valid_coordinate,
    thin_walls::{ThinWallConfig, split_thin_walls},
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
    /// Parallel lines or a crossed grid beneath overhangs.
    pub pattern: SupportPattern,
}

/// Support infill layout.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SupportPattern {
    #[default]
    Lines,
    Grid,
}

impl Default for SupportConfig {
    fn default() -> Self {
        Self {
            overhang_offset_mm: 0.4,
            support_spacing_mm: 2.5,
            pattern: SupportPattern::Lines,
        }
    }
}

impl SupportConfig {
    /// Support for faces steeper than `threshold_deg` from vertical: a layer may
    /// step out by `layer_height * tan(threshold)` before it needs support.
    pub fn from_overhang_angle(
        threshold_deg: f64,
        layer_height_mm: f64,
        support_spacing_mm: f64,
        pattern: SupportPattern,
    ) -> Result<Self> {
        if !(threshold_deg.is_finite() && (0.0..90.0).contains(&threshold_deg))
            || !(layer_height_mm.is_finite() && layer_height_mm > 0.0)
        {
            return Err(invalid(
                "TOOLPATH_INVALID_SETTINGS",
                "Overhang angle must be 0..90 degrees and layer height positive",
            ));
        }
        Ok(Self {
            overhang_offset_mm: layer_height_mm * threshold_deg.to_radians().tan(),
            support_spacing_mm,
            pattern,
        })
    }
}

/// Number of solid shell layers at the top and bottom of every surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShellConfig {
    pub top_layers: usize,
    pub bottom_layers: usize,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            top_layers: 1,
            bottom_layers: 1,
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
    plan_layers_with_shells(
        sections,
        settings,
        pattern,
        &ShellConfig::default(),
        support,
    )
}

/// Like [`plan_layers_with_skins_and_supports`] with configurable solid shells:
/// region of layer `i` becomes solid skin unless it is covered by material on
/// each of the `top_layers` layers above and `bottom_layers` layers below.
/// Perimeter count comes from `settings.wall_count`.
pub fn plan_layers_with_shells(
    sections: &[LayerSection],
    settings: &ToolpathSettings,
    pattern: InfillPattern,
    shells: &ShellConfig,
    support: Option<&SupportConfig>,
) -> Result<Vec<ToolpathLayer>> {
    plan_layers_core(sections, settings, pattern, shells, support, None).map(|(layers, _)| layers)
}

/// Layers plus the per-segment extrusion width table that
/// `gcode_core::emit_job_variable_width` consumes (`widths[layer][path][segment]`).
#[derive(Clone, Debug, PartialEq)]
pub struct WidthPlan {
    pub layers: Vec<ToolpathLayer>,
    pub widths: Vec<Vec<Vec<f64>>>,
}

/// Like [`plan_layers_with_shells`], but strips narrower than two nominal line
/// widths are printed as one [`PathRole::ThinWall`] centre-line bead whose
/// segment widths follow the local wall thickness, clamped to `thin`. Every
/// other path gets the nominal line width. A layer without thin strips plans
/// exactly as [`plan_layers_with_shells`] does.
pub fn plan_layers_with_thin_walls(
    sections: &[LayerSection],
    settings: &ToolpathSettings,
    pattern: InfillPattern,
    shells: &ShellConfig,
    support: Option<&SupportConfig>,
    thin: &ThinWallConfig,
) -> Result<WidthPlan> {
    require_settings(settings)?;
    thin.validate(settings.line_width_mm)?;
    let (layers, widths) =
        plan_layers_core(sections, settings, pattern, shells, support, Some(thin))?;
    Ok(WidthPlan { layers, widths })
}

type WidthTable = Vec<Vec<Vec<f64>>>;

fn plan_layers_core(
    sections: &[LayerSection],
    settings: &ToolpathSettings,
    pattern: InfillPattern,
    shells: &ShellConfig,
    support: Option<&SupportConfig>,
    thin: Option<&ThinWallConfig>,
) -> Result<(Vec<ToolpathLayer>, WidthTable)> {
    require_settings(settings)?;
    if shells.top_layers > 64 || shells.bottom_layers > 64 {
        return Err(invalid(
            "TOOLPATH_INVALID_SETTINGS",
            "Shell layer counts must be at most 64",
        ));
    }
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
    let mut out_widths = Vec::with_capacity(if thin.is_some() { sections.len() } else { 0 });
    for i in 0..sections.len() {
        let (split_source, beads) = match thin {
            Some(config) => {
                let (walls, beads) =
                    split_thin_walls(&normalized[i], settings.line_width_mm, config, &mut budget)?;
                if beads.is_empty() {
                    (None, beads)
                } else {
                    (Some(walls), beads)
                }
            }
            None => (None, Vec::new()),
        };
        let source = split_source.as_ref().unwrap_or(&normalized[i]);
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
                // Material present on every required neighbour layer; a missing
                // neighbour (outside the stack) leaves nothing covered.
                let mut covered: Option<Rings> = None;
                let neighbours = (1..=shells.top_layers)
                    .map(|k| i.checked_add(k).filter(|&j| j < sections.len()))
                    .chain((1..=shells.bottom_layers).map(|k| i.checked_sub(k)));
                for j in neighbours {
                    let Some(j) = j else {
                        covered = Some(Vec::new());
                        break;
                    };
                    let next = match covered {
                        None => normalized[j].clone(),
                        Some(ref acc) => rings::planar(acc, &normalized[j], "intersection")?,
                    };
                    let empty = next.is_empty();
                    covered = Some(next);
                    if empty {
                        break;
                    }
                }
                let covered = covered.unwrap_or_else(|| supported_interior.clone());
                let (skin_region, sparse_region) = if covered.is_empty() {
                    (supported_interior.clone(), Vec::new())
                } else {
                    (
                        rings::planar(&supported_interior, &covered, "difference")?,
                        rings::planar(&supported_interior, &covered, "intersection")?,
                    )
                };
                {
                    if !skin_region.is_empty() {
                        paths.extend(hatch_angled(
                            &skin_region,
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
            if sup.pattern == SupportPattern::Grid {
                paths.extend(hatch_angled(
                    &support_regions[i],
                    sup.support_spacing_mm,
                    std::f64::consts::FRAC_PI_2,
                    PathRole::Support,
                    &mut budget,
                )?);
            }
        }
        if thin.is_some() {
            let mut layer_widths: Vec<Vec<f64>> = paths
                .iter()
                .map(|path| {
                    let segments = path.points.len().saturating_sub(1)
                        + usize::from(path.closed && path.points.len() >= 3);
                    vec![settings.line_width_mm; segments]
                })
                .collect();
            for bead in beads {
                budget.points(bead.path.points.len())?;
                paths.push(bead.path);
                layer_widths.push(bead.widths);
            }
            out_widths.push(layer_widths);
        }
        out_layers.push(ToolpathLayer {
            z_mm: sections[i].z_mm,
            paths,
        });
    }
    Ok((out_layers, out_widths))
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

mod raft;
mod fuzzy_skin;
pub use raft::{RaftConfig, generate_raft};
pub use fuzzy_skin::{FuzzySkinConfig, apply_fuzzy_skin_to_contour};
