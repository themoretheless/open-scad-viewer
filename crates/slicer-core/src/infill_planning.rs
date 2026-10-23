//! Infill planning: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn generate_infill(
    remaining: &Rings,
    spacing: f64,
    pattern: InfillPattern,
    role: PathRole,
    budget: &mut Budget,
) -> Result<Vec<Toolpath>> {
    match pattern {
        InfillPattern::Rectilinear => hatch_angled(remaining, spacing, 0.0, role, budget),
        InfillPattern::Grid => {
            let mut paths = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_2,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::Triangles => {
            let mut paths = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            paths.extend(hatch_angled(
                remaining,
                spacing,
                -std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::Concentric => {
            let mut paths = Vec::new();
            let mut step = 1usize;
            while step <= 64 {
                let offset = inset_rings(remaining, step as f64 * spacing, budget)?;
                if offset.is_empty() {
                    break;
                }
                for ring in &offset {
                    budget.points(ring.len())?;
                    paths.push(path_from_ring(role, ring, true));
                }
                step += 1;
            }
            Ok(paths)
        }
        InfillPattern::Honeycomb => {
            let mut paths = Vec::new();
            for angle in [
                0.0,
                std::f64::consts::FRAC_PI_3,
                -std::f64::consts::FRAC_PI_3,
            ] {
                let spans = hatch_angled(remaining, spacing * 1.5, angle, role, budget)?;
                paths.reserve(spans.len());
                for span in spans {
                    if span.points.len() == 2 {
                        let [a, b] = [span.points[0], span.points[1]];
                        let dx = b[0] - a[0];
                        let dy = b[1] - a[1];
                        let len = dx.hypot(dy);
                        let step_len = (spacing * 0.5).max(0.2);
                        let steps = ((len / step_len).ceil() as usize).clamp(1, 512);
                        let (nx, ny) = if len > 1e-12 {
                            (-dy / len, dx / len)
                        } else {
                            (0.0, 0.0)
                        };
                        let amp = spacing * 0.25;
                        let mut pts = Vec::with_capacity(steps + 1);
                        for i in 0..=steps {
                            let t = i as f64 / steps as f64;
                            let offset = if i % 2 == 0 { -amp } else { amp };
                            let px = a[0] + t * dx + offset * nx;
                            let py = a[1] + t * dy + offset * ny;
                            if point_in_rings(remaining, [px, py]) {
                                pts.push([px, py]);
                            }
                        }
                        if pts.len() >= 2 {
                            budget.points(pts.len())?;
                            paths.push(Toolpath {
                                role,
                                points: pts,
                                closed: false,
                            });
                        }
                    }
                }
            }
            Ok(paths)
        }
        InfillPattern::Gyroid => {
            let spans = hatch_angled(remaining, spacing, 0.0, role, budget)?;
            let mut paths = Vec::with_capacity(spans.len());
            let k = std::f64::consts::TAU / (spacing * 2.0).max(0.5);
            let amp = spacing * 0.35;
            for (row_idx, span) in spans.into_iter().enumerate() {
                if span.points.len() == 2 {
                    let [a, b] = [span.points[0], span.points[1]];
                    let dx = b[0] - a[0];
                    let dy = b[1] - a[1];
                    let len = dx.hypot(dy);
                    let steps = ((len / (spacing * 0.25).max(0.2)).ceil() as usize).clamp(2, 512);
                    let phase = if row_idx % 2 == 0 {
                        0.0
                    } else {
                        std::f64::consts::PI
                    };
                    let mut pts = Vec::with_capacity(steps + 1);
                    for i in 0..=steps {
                        let t = i as f64 / steps as f64;
                        let x = a[0] + t * dx;
                        let y = a[1] + t * dy + amp * (x * k + phase).sin();
                        if point_in_rings(remaining, [x, y]) {
                            pts.push([x, y]);
                        }
                    }
                    if pts.len() >= 2 {
                        budget.points(pts.len())?;
                        paths.push(Toolpath {
                            role,
                            points: pts,
                            closed: false,
                        });
                    }
                }
            }
            Ok(paths)
        }
        InfillPattern::HilbertCurve => generate_hilbert_infill(remaining, spacing, role, budget),
        InfillPattern::ArchimedeanChords => {
            generate_archimedean_infill(remaining, spacing, role, budget)
        }
        InfillPattern::CrossHatch => {
            let mut paths = hatch_angled(
                remaining,
                spacing,
                std::f64::consts::FRAC_PI_4,
                role,
                budget,
            )?;
            paths.extend(hatch_angled(
                remaining,
                spacing,
                -std::f64::consts::FRAC_PI_4,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::TriHexagon => {
            let mut paths = hatch_angled(remaining, spacing * 1.25, 0.0, role, budget)?;
            paths.extend(hatch_angled(
                remaining,
                spacing * 1.25,
                std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            paths.extend(hatch_angled(
                remaining,
                spacing * 1.25,
                -std::f64::consts::FRAC_PI_3,
                role,
                budget,
            )?);
            Ok(paths)
        }
        InfillPattern::CubicSubdivision => {
            generate_cubic_infill(remaining, spacing, 0.0, role, budget)
        }
    }
}

pub(super) fn hatch(source: &Rings, spacing: f64, budget: &mut Budget) -> Result<Vec<Toolpath>> {
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
    let edges = point_count(source);
    let row_work = edges
        .checked_mul(1 + edges.max(1).ilog2() as usize)
        .ok_or_else(budget_error)?;
    budget.work(line_count.checked_mul(row_work).ok_or_else(budget_error)?)?;
    // Scanline active edge table: index every non-horizontal edge once by its
    // lower endpoint, then rows only visit edges that can actually cross them.
    // The eligibility rule `(a[1] <= y && y < b[1]) || (b[1] <= y && y < a[1])`
    // is exactly `min_y <= y < max_y`, and the intersection formula, winding
    // sign and grouping below are unchanged, so spans stay bit-identical to
    // the naive per-row scan over all edges.
    struct HatchEdge {
        a: [f64; 2],
        b: [f64; 2],
        min_y: f64,
        max_y: f64,
        winding: i32,
    }
    let mut sorted_edges = Vec::with_capacity(edges);
    for ring in source {
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            if a[1] == b[1] {
                continue;
            }
            sorted_edges.push(HatchEdge {
                a,
                b,
                min_y: a[1].min(b[1]),
                max_y: a[1].max(b[1]),
                winding: if b[1] > a[1] { -1_i32 } else { 1_i32 },
            });
        }
    }
    sorted_edges.sort_by(|x, y| x.min_y.total_cmp(&y.min_y));
    let mut active: Vec<usize> = Vec::with_capacity(sorted_edges.len().min(64));
    let mut paths = Vec::with_capacity(line_count);
    let mut hits = Vec::with_capacity(sorted_edges.len().min(64));
    let mut previous_y = None;
    let mut next_edge = 0;
    for row in 0..line_count {
        // Index-based placement avoids accumulated roundoff and non-advancing
        // floating-point additions. The budget counts every attempted row.
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
        while next_edge < sorted_edges.len() && sorted_edges[next_edge].min_y <= y {
            active.push(next_edge);
            next_edge += 1;
        }
        active.retain(|&index| y < sorted_edges[index].max_y);
        hits.clear();
        for &index in &active {
            let edge = &sorted_edges[index];
            let (a, b) = (edge.a, edge.b);
            let t = (y - a[1]) / (b[1] - a[1]);
            let x = (b[0] - a[0]).mul_add(t, a[0]);
            if !valid_coordinate(x) {
                return Err(invalid(
                    "TOOLPATH_INVALID_GEOMETRY",
                    "Non-finite hatch intersection",
                ));
            }
            hits.push((x, edge.winding));
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        // Sweep signed crossings so holes, disconnected islands and touching
        // boundaries share the same nonzero fill rule as the wall geometry.
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
